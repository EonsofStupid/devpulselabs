//! Pack schema 1 — the declarative per-app unit (seam ADR §3, §7.4,
//! §7.7): tolerant TOML in (unknown keys warn, never fail, and are
//! preserved), canonical content-hashed TOML out, promote-only in types.

use egui_pilot_ai_protocol::seam::{AppId, CellKey, PackId, TransportRung};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fmt::Write as _;
use toml::{Table, Value};

/// The pack schema this build writes and the newest it will load.
pub const PACK_SCHEMA: i64 = 1;
/// The seam version this build speaks (ADR §7.4 `min_seam` gate).
pub const CURRENT_SEAM: i64 = 1;

// The transport rung's one home is `egui_pilot_ai_protocol::seam` —
// S2 rehomed it there as this file's own l.16 comment promised. The
// TOML wire values (`"r0"`/`"r1"`) are byte-identical to the S3a packs
// on disk. (`dpl-fabric`'s private copy is label-equality-tested in
// the daemon until its own follow-up micro-slice retires it.)

/// The only rungs a pack may GRANT. Demotion to `r2` and `r3` (vision
/// never enters the lesson hot path) are unrepresentable — the
/// promote-only law lives in this type, not in a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromotedRung {
    R0Native,
    R1Semantic,
}

impl PromotedRung {
    pub const fn rung(self) -> TransportRung {
        match self {
            Self::R0Native => TransportRung::R0Native,
            Self::R1Semantic => TransportRung::R1Semantic,
        }
    }

    pub const fn label(self) -> &'static str {
        self.rung().label()
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "r0" => Some(Self::R0Native),
            "r1" => Some(Self::R1Semantic),
            _ => None,
        }
    }
}

/// Every verb's default ladder starts semantic; injection is the
/// fallback rung the runtime downgrades to (S1 records the downgrade).
pub const DEFAULT_RUNG: TransportRung = TransportRung::R1Semantic;

/// A grant only wins when STRICTLY better than the default — a pack can
/// promote a verb up the ladder, never push it down.
pub fn effective_rung(default: TransportRung, grant: Option<PromotedRung>) -> TransportRung {
    match grant {
        Some(g) if g.rung() < default => g.rung(),
        _ => default,
    }
}

/// The `[identity]` table — what the evidence resolver scores bundles
/// against. Names are weighted signals, never the anchor (locale law).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub app_id: AppId,
    pub display_name: String,
    pub desktop_ids: Vec<String>,
    pub wm_classes: Vec<String>,
    pub atspi_names: Vec<String>,
    pub exe_basenames: Vec<String>,
    pub aliases: Vec<String>,
}

/// One `[[verb]]` capability row. Unknown keys ride `extras`, preserved.
#[derive(Debug, Clone, PartialEq)]
pub struct VerbGrant {
    pub verb: String,
    pub rung: PromotedRung,
    /// Effect rate from the last battery run (Foundry writes it, S3b).
    pub reliability: Option<f64>,
    pub oracle: Option<String>,
    pub extras: Table,
}

/// One pack — the declarative unit the Foundry builds and the registry
/// distributes. `extras` preserves every unknown top-level key AND the
/// unknown keys of known sections (re-nested under the section name).
#[derive(Debug, Clone, PartialEq)]
pub struct Pack {
    pub schema: i64,
    pub min_seam: i64,
    pub identity: Identity,
    pub cell: CellKey,
    pub ladder: Vec<(String, PromotedRung)>,
    pub verbs: Vec<VerbGrant>,
    pub teaching_landmarks: Vec<String>,
    pub extras: Table,
}

#[derive(Debug)]
pub enum PackError {
    Toml(String),
    SchemaTooNew {
        found: i64,
        supported: i64,
    },
    Missing(&'static str),
    BadId(String),
    /// The registry persists resolved identities only — promote first.
    UnresolvedWrite(AppId),
    Io(String),
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Toml(e) => write!(f, "pack toml: {e}"),
            Self::SchemaTooNew { found, supported } => write!(
                f,
                "pack schema {found} is newer than supported {supported} — update the seam"
            ),
            Self::Missing(k) => write!(f, "pack is missing required key `{k}`"),
            Self::BadId(e) => write!(f, "pack identity: {e}"),
            Self::UnresolvedWrite(id) => write!(
                f,
                "refusing to persist unresolved identity {id} — mint a real AppId (promotion), then write"
            ),
            Self::Io(e) => write!(f, "pack io: {e}"),
        }
    }
}

impl std::error::Error for PackError {}

/// A parsed pack plus everything honesty requires beside it.
#[derive(Debug)]
pub struct ParsedPack {
    pub pack: Pack,
    /// Unknown keys, refused grants, tolerated oddities — never fatal.
    pub warnings: Vec<String>,
    /// `min_seam` ≤ [`CURRENT_SEAM`]. False still loads — degrade, never crash.
    pub compatible: bool,
    /// `Some(ok)` when `[signing].pack_hash` was present to check.
    pub hash_verified: Option<bool>,
}

impl Pack {
    pub fn pack_id(&self) -> PackId {
        PackId::new(&self.identity.app_id, &self.cell)
    }

    /// Parse tolerant TOML. Fails ONLY on: invalid TOML, a newer
    /// `schema`, or a missing/invalid identity. Everything else warns.
    pub fn parse(text: &str) -> Result<ParsedPack, PackError> {
        let mut doc: Table = text.parse().map_err(|e| PackError::Toml(format!("{e}")))?;
        let mut warnings = Vec::new();

        // Verify the content hash BEFORE any extraction mutates the doc.
        let declared_hash = doc.remove("signing").and_then(|v| match v {
            Value::Table(t) => t
                .get("pack_hash")
                .and_then(Value::as_str)
                .map(str::to_string),
            _ => None,
        });
        let hash_verified = match declared_hash {
            Some(declared) => {
                let body = toml::to_string_pretty(&doc)
                    .map_err(|e| PackError::Toml(format!("canonicalize: {e}")))?;
                let ok = content_hash_of(&body) == declared;
                if !ok {
                    warnings
                        .push("pack_hash mismatch — the body was edited after signing".to_string());
                }
                Some(ok)
            }
            None => None,
        };

        let schema =
            take_int(&mut doc, "schema", &mut warnings).ok_or(PackError::Missing("schema"))?;
        if schema > PACK_SCHEMA {
            return Err(PackError::SchemaTooNew {
                found: schema,
                supported: PACK_SCHEMA,
            });
        }
        let min_seam = take_int(&mut doc, "min_seam", &mut warnings).unwrap_or(1);
        let compatible = min_seam <= CURRENT_SEAM;
        if !compatible {
            warnings.push(format!(
                "pack needs seam {min_seam}, this build speaks {CURRENT_SEAM} — loaded, degraded"
            ));
        }

        // [identity] — the one section that must be present and valid.
        let mut id_tbl = match doc.remove("identity") {
            Some(Value::Table(t)) => t,
            Some(_) | None => return Err(PackError::Missing("identity")),
        };
        let app_id_raw = take_str(&mut id_tbl, "identity.app_id", "app_id", &mut warnings)
            .ok_or(PackError::Missing("identity.app_id"))?;
        let app_id = AppId::parse(&app_id_raw).map_err(|e| PackError::BadId(e.to_string()))?;
        let identity = Identity {
            app_id,
            display_name: take_str(
                &mut id_tbl,
                "identity.display_name",
                "display_name",
                &mut warnings,
            )
            .unwrap_or_default(),
            desktop_ids: take_str_array(
                &mut id_tbl,
                "identity.desktop_ids",
                "desktop_ids",
                &mut warnings,
            ),
            wm_classes: take_str_array(
                &mut id_tbl,
                "identity.wm_classes",
                "wm_classes",
                &mut warnings,
            ),
            atspi_names: take_str_array(
                &mut id_tbl,
                "identity.atspi_names",
                "atspi_names",
                &mut warnings,
            ),
            exe_basenames: take_str_array(
                &mut id_tbl,
                "identity.exe_basenames",
                "exe_basenames",
                &mut warnings,
            ),
            aliases: take_str_array(&mut id_tbl, "identity.aliases", "aliases", &mut warnings),
        };
        renest_leftovers(&mut doc, "identity", id_tbl, &mut warnings);

        // [cell] — tolerated missing; "unknown" is honest, a guess is not.
        let cell = match doc.remove("cell") {
            Some(Value::Table(mut t)) => {
                let cell = CellKey::new(
                    take_str(&mut t, "cell.app_version", "app_version", &mut warnings)
                        .unwrap_or_else(|| "unknown".to_string()),
                    take_str(&mut t, "cell.os", "os", &mut warnings)
                        .unwrap_or_else(|| "unknown".to_string()),
                    take_str(&mut t, "cell.locale", "locale", &mut warnings)
                        .unwrap_or_else(|| "unknown".to_string()),
                );
                renest_leftovers(&mut doc, "cell", t, &mut warnings);
                cell
            }
            Some(other) => {
                warnings.push(format!(
                    "[cell] expected a table, got {} — defaulted",
                    other.type_str()
                ));
                CellKey::new("unknown", "unknown", "unknown")
            }
            None => {
                warnings.push("[cell] missing — defaulted to unknown/unknown/unknown".to_string());
                CellKey::new("unknown", "unknown", "unknown")
            }
        };

        // [ladder] — verb → promoted rung; demotions refused by name.
        let mut ladder = Vec::new();
        match doc.remove("ladder") {
            Some(Value::Table(t)) => {
                for (verb, v) in t {
                    match v {
                        Value::String(s) => match PromotedRung::parse(&s) {
                            Some(r) => ladder.push((verb, r)),
                            None => warnings.push(format!(
                                "[ladder] {verb} = {s:?} refused — packs promote only (r0|r1)"
                            )),
                        },
                        other => warnings.push(format!(
                            "[ladder] {verb}: expected a rung string, got {}",
                            other.type_str()
                        )),
                    }
                }
            }
            Some(other) => warnings.push(format!(
                "[ladder] expected a table, got {}",
                other.type_str()
            )),
            None => {}
        }

        // [[verb]] — capability matrix rows.
        let mut verbs = Vec::new();
        match doc.remove("verb") {
            Some(Value::Array(rows)) => {
                for (i, row) in rows.into_iter().enumerate() {
                    let Value::Table(mut t) = row else {
                        warnings.push(format!("[[verb]] #{i} is not a table — dropped"));
                        continue;
                    };
                    let Some(verb) = take_str(&mut t, "verb.verb", "verb", &mut warnings) else {
                        warnings.push(format!("[[verb]] #{i} missing `verb` — dropped"));
                        continue;
                    };
                    let rung = match take_str(&mut t, "verb.rung", "rung", &mut warnings) {
                        Some(s) => match PromotedRung::parse(&s) {
                            Some(r) => r,
                            None => {
                                warnings.push(format!(
                                    "[[verb]] {verb}: rung {s:?} refused — packs promote only (r0|r1); row dropped"
                                ));
                                continue;
                            }
                        },
                        None => PromotedRung::R1Semantic,
                    };
                    let reliability =
                        take_float(&mut t, "verb.reliability", "reliability", &mut warnings);
                    let oracle = take_str(&mut t, "verb.oracle", "oracle", &mut warnings);
                    for k in t.keys() {
                        warnings.push(format!("unknown key `verb.{k}` preserved"));
                    }
                    verbs.push(VerbGrant {
                        verb,
                        rung,
                        reliability,
                        oracle,
                        extras: t,
                    });
                }
            }
            Some(other) => warnings.push(format!(
                "[[verb]] expected an array of tables, got {}",
                other.type_str()
            )),
            None => {}
        }

        // [teaching]
        let mut teaching_landmarks = Vec::new();
        if let Some(v) = doc.remove("teaching") {
            match v {
                Value::Table(mut t) => {
                    teaching_landmarks =
                        take_str_array(&mut t, "teaching.landmarks", "landmarks", &mut warnings);
                    renest_leftovers(&mut doc, "teaching", t, &mut warnings);
                }
                other => warnings.push(format!(
                    "[teaching] expected a table, got {}",
                    other.type_str()
                )),
            }
        }

        // Whatever remains at the top level is unknown — warned, kept.
        for k in doc.keys() {
            if !matches!(k.as_str(), "identity" | "cell" | "teaching") {
                warnings.push(format!("unknown key `{k}` preserved"));
            }
        }

        Ok(ParsedPack {
            pack: Pack {
                schema,
                min_seam,
                identity,
                cell,
                ladder,
                verbs,
                teaching_landmarks,
                extras: doc,
            },
            warnings,
            compatible,
            hash_verified,
        })
    }

    /// The canonical body table — every known field re-inserted over the
    /// preserved extras, `[signing]` excluded (it signs this).
    pub fn body_table(&self) -> Table {
        let mut doc = self.extras.clone();
        doc.remove("signing"); // writer-owned, always regenerated
        doc.insert("schema".into(), Value::Integer(self.schema));
        doc.insert("min_seam".into(), Value::Integer(self.min_seam));

        let mut id_tbl = remove_table(&mut doc, "identity");
        id_tbl.insert(
            "app_id".into(),
            Value::String(self.identity.app_id.as_str().to_string()),
        );
        id_tbl.insert(
            "display_name".into(),
            Value::String(self.identity.display_name.clone()),
        );
        insert_str_array(&mut id_tbl, "desktop_ids", &self.identity.desktop_ids);
        insert_str_array(&mut id_tbl, "wm_classes", &self.identity.wm_classes);
        insert_str_array(&mut id_tbl, "atspi_names", &self.identity.atspi_names);
        insert_str_array(&mut id_tbl, "exe_basenames", &self.identity.exe_basenames);
        insert_str_array(&mut id_tbl, "aliases", &self.identity.aliases);
        doc.insert("identity".into(), Value::Table(id_tbl));

        let mut cell_tbl = remove_table(&mut doc, "cell");
        cell_tbl.insert(
            "app_version".into(),
            Value::String(self.cell.app_version.clone()),
        );
        cell_tbl.insert("os".into(), Value::String(self.cell.os.clone()));
        cell_tbl.insert("locale".into(), Value::String(self.cell.locale.clone()));
        doc.insert("cell".into(), Value::Table(cell_tbl));

        if !self.ladder.is_empty() {
            let mut t = Table::new();
            for (verb, rung) in &self.ladder {
                t.insert(verb.clone(), Value::String(rung.label().to_string()));
            }
            doc.insert("ladder".into(), Value::Table(t));
        }

        if !self.verbs.is_empty() {
            let rows = self
                .verbs
                .iter()
                .map(|g| {
                    let mut t = g.extras.clone();
                    t.insert("verb".into(), Value::String(g.verb.clone()));
                    t.insert("rung".into(), Value::String(g.rung.label().to_string()));
                    if let Some(r) = g.reliability {
                        t.insert("reliability".into(), Value::Float(r));
                    }
                    if let Some(o) = &g.oracle {
                        t.insert("oracle".into(), Value::String(o.clone()));
                    }
                    Value::Table(t)
                })
                .collect();
            doc.insert("verb".into(), Value::Array(rows));
        }

        if !self.teaching_landmarks.is_empty() || doc.contains_key("teaching") {
            let mut t = remove_table(&mut doc, "teaching");
            insert_str_array(&mut t, "landmarks", &self.teaching_landmarks);
            doc.insert("teaching".into(), Value::Table(t));
        }

        doc
    }

    /// Canonical body TOML — what the content hash covers (ADR §7.7).
    pub fn canonical_body(&self) -> Result<String, PackError> {
        toml::to_string_pretty(&self.body_table())
            .map_err(|e| PackError::Toml(format!("serialize: {e}")))
    }

    pub fn content_hash(&self) -> Result<String, PackError> {
        Ok(content_hash_of(&self.canonical_body()?))
    }

    /// Body + `[signing]` with the hash of everything above it.
    pub fn to_signed_toml(&self) -> Result<String, PackError> {
        let body = self.canonical_body()?;
        let hash = content_hash_of(&body);
        Ok(format!("{body}\n[signing]\npack_hash = \"{hash}\"\n"))
    }
}

pub(crate) fn content_hash_of(body: &str) -> String {
    let digest = Sha256::digest(body.as_bytes());
    let mut out = String::with_capacity(7 + digest.len() * 2);
    out.push_str("sha256:");
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Put a known section's UNKNOWN leftovers back under its name so the
/// top-level extras carry them (warned key-by-key, preserved verbatim).
fn renest_leftovers(doc: &mut Table, section: &str, leftovers: Table, warnings: &mut Vec<String>) {
    if leftovers.is_empty() {
        return;
    }
    for k in leftovers.keys() {
        warnings.push(format!("unknown key `{section}.{k}` preserved"));
    }
    doc.insert(section.to_string(), Value::Table(leftovers));
}

fn remove_table(doc: &mut Table, key: &str) -> Table {
    match doc.remove(key) {
        Some(Value::Table(t)) => t,
        _ => Table::new(),
    }
}

fn take_int(t: &mut Table, key: &str, warnings: &mut Vec<String>) -> Option<i64> {
    match t.remove(key) {
        Some(Value::Integer(i)) => Some(i),
        Some(other) => {
            warnings.push(format!(
                "`{key}` expected an integer, got {} — ignored",
                other.type_str()
            ));
            None
        }
        None => None,
    }
}

fn take_str(t: &mut Table, path: &str, key: &str, warnings: &mut Vec<String>) -> Option<String> {
    match t.remove(key) {
        Some(Value::String(s)) => Some(s),
        Some(other) => {
            warnings.push(format!(
                "`{path}` expected a string, got {} — ignored",
                other.type_str()
            ));
            None
        }
        None => None,
    }
}

fn take_float(t: &mut Table, path: &str, key: &str, warnings: &mut Vec<String>) -> Option<f64> {
    match t.remove(key) {
        Some(Value::Float(f)) => Some(f),
        Some(Value::Integer(i)) => Some(i as f64),
        Some(other) => {
            warnings.push(format!(
                "`{path}` expected a number, got {} — ignored",
                other.type_str()
            ));
            None
        }
        None => None,
    }
}

fn take_str_array(t: &mut Table, path: &str, key: &str, warnings: &mut Vec<String>) -> Vec<String> {
    match t.remove(key) {
        Some(Value::Array(items)) => items
            .into_iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(s),
                other => {
                    warnings.push(format!(
                        "`{path}` element expected a string, got {} — skipped",
                        other.type_str()
                    ));
                    None
                }
            })
            .collect(),
        Some(other) => {
            warnings.push(format!(
                "`{path}` expected an array of strings, got {} — ignored",
                other.type_str()
            ));
            Vec::new()
        }
        None => Vec::new(),
    }
}

fn insert_str_array(t: &mut Table, key: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    t.insert(
        key.to_string(),
        Value::Array(values.iter().cloned().map(Value::String).collect()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blender_pack() -> Pack {
        Pack {
            schema: PACK_SCHEMA,
            min_seam: CURRENT_SEAM,
            identity: Identity {
                app_id: AppId::parse("org.blender.Blender").unwrap(),
                display_name: "Blender".into(),
                desktop_ids: vec![
                    "blender.desktop".into(),
                    "org.blender.Blender.desktop".into(),
                ],
                wm_classes: vec!["Blender".into()],
                atspi_names: vec!["Blender".into()],
                exe_basenames: vec!["blender".into()],
                aliases: vec![],
            },
            cell: CellKey::new("4.2", "linux", "en"),
            ladder: vec![("open".into(), PromotedRung::R0Native)],
            verbs: vec![VerbGrant {
                verb: "open".into(),
                rung: PromotedRung::R1Semantic,
                reliability: Some(0.97),
                oracle: Some("window-appears".into()),
                extras: Table::new(),
            }],
            teaching_landmarks: vec!["3D Viewport".into()],
            extras: Table::new(),
        }
    }

    #[test]
    fn signed_round_trip_verifies_hash_and_equality() {
        let pack = blender_pack();
        let text = pack.to_signed_toml().unwrap();
        let parsed = Pack::parse(&text).unwrap();
        assert_eq!(parsed.hash_verified, Some(true));
        assert!(parsed.compatible);
        assert_eq!(parsed.pack, pack);
        assert!(
            parsed.warnings.is_empty(),
            "warnings: {:?}",
            parsed.warnings
        );
        assert_eq!(
            parsed.pack.pack_id().as_str(),
            "org.blender.Blender@4.2/linux/en"
        );
    }

    #[test]
    fn tampered_body_warns_but_loads() {
        let pack = blender_pack();
        let text = pack
            .to_signed_toml()
            .unwrap()
            .replace("display_name = \"Blender\"", "display_name = \"Evil\"");
        let parsed = Pack::parse(&text).unwrap();
        assert_eq!(parsed.hash_verified, Some(false));
        assert!(parsed
            .warnings
            .iter()
            .any(|w| w.contains("pack_hash mismatch")));
        assert_eq!(parsed.pack.identity.display_name, "Evil");
    }

    #[test]
    fn unknown_keys_warn_and_survive_a_rewrite() {
        let text = "\
schema = 1
min_seam = 1
future_top = \"kept\"

[identity]
app_id = \"org.gnome.Calculator\"
display_name = \"Calculator\"
future_field = 42

[cell]
app_version = \"48\"
os = \"linux\"
locale = \"en\"

[future_section]
key = \"kept too\"
";
        let parsed = Pack::parse(text).unwrap();
        assert!(parsed
            .warnings
            .iter()
            .any(|w| w.contains("`identity.future_field` preserved")));
        assert!(parsed
            .warnings
            .iter()
            .any(|w| w.contains("`future_top` preserved")));
        assert!(parsed
            .warnings
            .iter()
            .any(|w| w.contains("`future_section` preserved")));
        // The rewrite carries them verbatim.
        let rewritten = parsed.pack.to_signed_toml().unwrap();
        assert!(rewritten.contains("future_top = \"kept\""));
        assert!(rewritten.contains("future_field = 42"));
        assert!(rewritten.contains("[future_section]"));
        // And the reparse agrees with itself, hash included.
        let again = Pack::parse(&rewritten).unwrap();
        assert_eq!(again.hash_verified, Some(true));
        assert_eq!(again.pack, parsed.pack);
    }

    #[test]
    fn newer_schema_is_refused_newer_min_seam_degrades() {
        let newer_schema = "schema = 2\n[identity]\napp_id = \"org.x.Y\"\n";
        assert!(matches!(
            Pack::parse(newer_schema),
            Err(PackError::SchemaTooNew { found: 2, .. })
        ));
        let newer_seam = "schema = 1\nmin_seam = 99\n[identity]\napp_id = \"org.x.Y\"\n[cell]\napp_version = \"1\"\nos = \"linux\"\nlocale = \"en\"\n";
        let parsed = Pack::parse(newer_seam).unwrap();
        assert!(!parsed.compatible);
        assert!(parsed.warnings.iter().any(|w| w.contains("degraded")));
    }

    #[test]
    fn promote_only_demotions_are_refused_by_the_loader() {
        let text = "\
schema = 1

[identity]
app_id = \"org.x.Y\"

[cell]
app_version = \"1\"
os = \"linux\"
locale = \"en\"

[ladder]
open = \"r0\"
close = \"r2\"
observe = \"r3\"

[[verb]]
verb = \"scroll\"
rung = \"r3\"
";
        let parsed = Pack::parse(text).unwrap();
        assert_eq!(
            parsed.pack.ladder,
            vec![("open".to_string(), PromotedRung::R0Native)]
        );
        assert!(parsed.pack.verbs.is_empty());
        let refused = parsed
            .warnings
            .iter()
            .filter(|w| w.contains("packs promote only"))
            .count();
        assert_eq!(refused, 3);
    }

    #[test]
    fn effective_rung_promotes_and_never_demotes() {
        assert_eq!(
            effective_rung(DEFAULT_RUNG, Some(PromotedRung::R0Native)),
            TransportRung::R0Native
        );
        assert_eq!(
            effective_rung(DEFAULT_RUNG, Some(PromotedRung::R1Semantic)),
            TransportRung::R1Semantic
        );
        assert_eq!(
            effective_rung(DEFAULT_RUNG, None),
            TransportRung::R1Semantic
        );
        // A grant equal to a BETTER default never demotes it.
        assert_eq!(
            effective_rung(TransportRung::R0Native, Some(PromotedRung::R1Semantic)),
            TransportRung::R0Native
        );
    }

    /// S2 proof 6: a pack file in the exact shape the S3a drill wrote
    /// to disk parses unchanged through the post-move loader — the
    /// protocol-homed rung speaks the same `"r0"`/`"r1"` TOML values.
    #[test]
    fn s3a_pack_files_on_disk_parse_unchanged_after_the_move() {
        let s3a_shape = "\
schema = 1
min_seam = 1

[identity]
app_id = \"org.blender.Blender\"
display_name = \"Blender\"
desktop_ids = [\"blender.desktop\", \"org.blender.Blender.desktop\"]
wm_classes = [\"Blender\", \"blender\"]
atspi_names = [\"Blender\"]
exe_basenames = [\"blender\"]

[cell]
app_version = \"4.0.2\"
os = \"linux\"
locale = \"en\"

[ladder]
open = \"r0\"
select = \"r1\"
";
        let parsed = Pack::parse(s3a_shape).unwrap();
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        assert_eq!(
            parsed.pack.ladder,
            vec![
                ("open".to_string(), PromotedRung::R0Native),
                ("select".to_string(), PromotedRung::R1Semantic),
            ]
        );
        // The rewrite emits the same byte values the S3a files carry.
        let rewritten = parsed.pack.to_signed_toml().unwrap();
        assert!(rewritten.contains("open = \"r0\""), "{rewritten}");
        assert!(rewritten.contains("select = \"r1\""), "{rewritten}");
    }

    #[test]
    fn missing_identity_is_the_one_hard_failure() {
        assert!(matches!(
            Pack::parse("schema = 1\n"),
            Err(PackError::Missing("identity"))
        ));
        assert!(matches!(
            Pack::parse("schema = 1\n[identity]\ndisplay_name = \"X\"\n"),
            Err(PackError::Missing("identity.app_id"))
        ));
        assert!(matches!(
            Pack::parse("schema = 1\n[identity]\napp_id = \"nodots\"\n"),
            Err(PackError::BadId(_))
        ));
    }
}
