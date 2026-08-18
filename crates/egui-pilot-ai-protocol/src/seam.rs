//! The foreign-app seam vocabulary (coach seam ADR §2.1–2.2, executed by
//! S2: `docs/plans/coach-seam-s2-protocol-rehoming.md`).
//!
//! ONE typed vocabulary with two capability-gated consumers: the
//! EguiPilot testing/demo lane (which does NOT execute foreign verbs —
//! its `seam.*` capability entries say so honestly) and the coaching
//! lane (`dpl-coach-runtime` + `dpl-coach-daemon`, devtools today).
//!
//! Identity types (`AppId`, `CellKey`, `PackId`) moved here verbatim
//! from `dpl-coach-pack` (S3a), parse laws intact — serde goes THROUGH
//! `parse`, so the wire cannot smuggle an invalid id. The verb catalog
//! absorbed `dpl-coach-runtime`'s stringly-typed `verbs.rs` whole
//! (superset test below), plus `geom` — shipped and battery-proven
//! since 2026-08-01 but never cataloged until now.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ── identity (moved from dpl-coach-pack, S3a laws intact) ──────────────

/// The `x.` extension namespace for scan-observed apps no pack claims
/// yet (seam ADR §7.2).
pub const UNRESOLVED_PREFIX: &str = "x.unresolved:";

/// Stable application identity — minted once per pack, never derived at
/// runtime. Exactly two forms parse; everything else is refused:
///
/// - resolved: reverse-DNS with ≥ 2 segments (`org.blender.Blender`);
///   the `x.` first segment is reserved for extensions
/// - extension: `x.unresolved:<slug>` with a `[a-z0-9-]` slug
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AppId(String);

/// Why an id string was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdError {
    pub raw: String,
    pub reason: &'static str,
}

impl fmt::Display for IdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "app id {:?} refused: {}", self.raw, self.reason)
    }
}

impl std::error::Error for IdError {}

impl AppId {
    pub fn parse(raw: &str) -> Result<Self, IdError> {
        if let Some(slug) = raw.strip_prefix(UNRESOLVED_PREFIX) {
            if !slug.is_empty()
                && slug
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                return Ok(Self(raw.to_string()));
            }
            return Err(IdError {
                raw: raw.to_string(),
                reason: "unresolved slug must be non-empty [a-z0-9-]",
            });
        }
        if raw.starts_with("x.") {
            return Err(IdError {
                raw: raw.to_string(),
                reason: "the x. namespace is reserved for extensions (x.unresolved:<slug>)",
            });
        }
        let segments: Vec<&str> = raw.split('.').collect();
        if segments.len() < 2 {
            return Err(IdError {
                raw: raw.to_string(),
                reason: "a resolved app id needs at least two dot segments",
            });
        }
        for segment in &segments {
            if segment.is_empty()
                || !segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-')
            {
                return Err(IdError {
                    raw: raw.to_string(),
                    reason: "segments must be non-empty [A-Za-z0-9-]",
                });
            }
        }
        Ok(Self(raw.to_string()))
    }

    /// Sanitize a scan-observed name into the extension namespace — the
    /// honest identity for an app no installed pack claims.
    pub fn unresolved(observed: &str) -> Self {
        Self(format!("{UNRESOLVED_PREFIX}{}", slugify(observed)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_unresolved(&self) -> bool {
        self.0.starts_with(UNRESOLVED_PREFIX)
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for AppId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

/// Deserialize goes THROUGH `parse` — a derived transparent impl would
/// let the wire smuggle an id that never passed the law (S2 plan §4.1;
/// the ADR's "via `string_id!`" line was refined for exactly this).
impl<'de> Deserialize<'de> for AppId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).map_err(serde::de::Error::custom)
    }
}

/// Lowercase, `[a-z0-9-]`, dashes collapsed and trimmed.
fn slugify(raw: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;
    for c in raw.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c);
        } else {
            pending_dash = true;
        }
    }
    if out.is_empty() {
        "unknown-app".to_string()
    } else {
        out
    }
}

/// The pack cell key — app version × OS × locale, first-class from day
/// one (seam ADR §7.4) so multi-cell is never a retrofit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellKey {
    pub app_version: String,
    pub os: String,
    pub locale: String,
}

impl CellKey {
    pub fn new(
        app_version: impl Into<String>,
        os: impl Into<String>,
        locale: impl Into<String>,
    ) -> Self {
        Self {
            app_version: app_version.into(),
            os: os.into(),
            locale: locale.into(),
        }
    }

    /// The cell this process runs in: compile-target OS + env locale.
    pub fn this_machine(app_version: impl Into<String>) -> Self {
        Self::new(app_version, std::env::consts::OS, current_locale())
    }

    pub fn label(&self) -> String {
        format!("{}/{}/{}", self.app_version, self.os, self.locale)
    }
}

/// `LC_ALL` → `LC_MESSAGES` → `LANG`, language part only (`en_US.UTF-8`
/// → `en`). Absent locale is reported honestly, never guessed.
fn current_locale() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .map(|v| {
            v.split(['_', '.', '@'])
                .next()
                .unwrap_or("unknown")
                .to_lowercase()
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// Pack identity: the app id plus its cell — `<app-id>@<version>/<os>/<locale>`.
/// Derived, never parsed — no wire `Deserialize` until a format law exists.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackId(String);

impl PackId {
    pub fn new(app: &AppId, cell: &CellKey) -> Self {
        Self(format!("{}@{}", app.as_str(), cell.label()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

string_id!(AnchorId);

/// A `x.`-namespaced id for incubator verbs (seam ADR §7.2): extensions
/// live outside the ratified catalog until promoted, and the namespace
/// is enforced by construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SeamExtensionId(String);

impl SeamExtensionId {
    pub fn parse(raw: &str) -> Result<Self, IdError> {
        let Some(rest) = raw.strip_prefix("x.") else {
            return Err(IdError {
                raw: raw.to_string(),
                reason: "extension verbs live in the x. namespace",
            });
        };
        if rest.is_empty()
            || !rest
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
        {
            return Err(IdError {
                raw: raw.to_string(),
                reason: "extension id after x. must be non-empty [a-z0-9.-]",
            });
        }
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SeamExtensionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

// ── the transport ladder ────────────────────────────────────────────────

/// Which rung of the seam transport ladder executes a verb (seam ADR §3:
/// R0 app-native API, R1 accessibility semantic action, R2 targeted
/// input injection, R3 vision-grounded — import/harvest fuel only, never
/// the lesson hot path). Ordering is quality: `R0Native` is the best
/// rung, `R3Vision` the last. Wire labels are `r0`..`r3`, identical to
/// the S1 Parquet rows and the S3a pack TOML values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TransportRung {
    #[serde(rename = "r0")]
    R0Native,
    #[serde(rename = "r1")]
    R1Semantic,
    #[serde(rename = "r2")]
    R2Injection,
    #[serde(rename = "r3")]
    R3Vision,
}

impl TransportRung {
    pub const fn label(self) -> &'static str {
        match self {
            Self::R0Native => "r0",
            Self::R1Semantic => "r1",
            Self::R2Injection => "r2",
            Self::R3Vision => "r3",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "r0" => Some(Self::R0Native),
            "r1" => Some(Self::R1Semantic),
            "r2" => Some(Self::R2Injection),
            "r3" => Some(Self::R3Vision),
            _ => None,
        }
    }
}

// ── the verb catalog, typed ─────────────────────────────────────────────

/// What a verb acts on — the ratified catalog's target column, typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SeamTarget {
    Desktop,
    App,
    Window,
    Fabric,
    Predicate,
}

impl SeamTarget {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::App => "app",
            Self::Window => "window",
            Self::Fabric => "fabric",
            Self::Predicate => "predicate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerbStatus {
    Shipped,
    Planned,
}

impl VerbStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shipped => "shipped",
            Self::Planned => "planned",
        }
    }
}

/// The seam verbs — the ratified operator catalog
/// (`docs/plans/coach-operator-verb-catalog-adr.md`), typed. Wire names
/// are the exact `snake_case` strings the catalog ratified; `parse` /
/// `wire_name` are the wire contract and unknown names refuse BY NAME.
/// `Extension` is the §7.2 incubator arm: `x.`-namespaced verbs ride
/// the same rails without touching the ratified set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeamVerb {
    Inventory,
    Presence,
    Import,
    Resolve,
    Invoke,
    Click,
    Mount,
    Open,
    Close,
    Focus,
    Move,
    Resize,
    /// Shipped in code and battery-proven since 2026-08-01, cataloged
    /// here for the first time (S2 plan §1 correction 3). `window_geom`
    /// is accepted as a parse alias, never emitted.
    Geom,
    TypeText,
    PressKey,
    Select,
    Scroll,
    Drag,
    WaitUntil,
    AssertState,
    Observe,
    Extension {
        id: SeamExtensionId,
        params: HashMap<String, String>,
    },
}

/// An unknown verb name, refused by name (never a silent fallthrough).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerbParseError {
    pub raw: String,
}

impl fmt::Display for VerbParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "verb not implemented: {}", self.raw)
    }
}

impl std::error::Error for VerbParseError {}

impl SeamVerb {
    pub fn parse(raw: &str) -> Result<Self, VerbParseError> {
        Ok(match raw {
            "inventory" => Self::Inventory,
            "presence" => Self::Presence,
            "import" => Self::Import,
            "resolve" => Self::Resolve,
            "invoke" => Self::Invoke,
            "click" => Self::Click,
            "mount" => Self::Mount,
            "open" => Self::Open,
            "close" => Self::Close,
            "focus" => Self::Focus,
            "move" => Self::Move,
            "resize" => Self::Resize,
            // `window_geom` shipped as an accepted spelling in the
            // daemon's string match — carried as a parse alias.
            "geom" | "window_geom" => Self::Geom,
            "type_text" => Self::TypeText,
            "press_key" => Self::PressKey,
            "select" => Self::Select,
            "scroll" => Self::Scroll,
            "drag" => Self::Drag,
            "wait_until" => Self::WaitUntil,
            "assert_state" => Self::AssertState,
            "observe" => Self::Observe,
            other => match SeamExtensionId::parse(other) {
                Ok(id) => Self::Extension {
                    id,
                    params: HashMap::new(),
                },
                Err(_) => {
                    return Err(VerbParseError {
                        raw: other.to_string(),
                    })
                }
            },
        })
    }

    pub fn wire_name(&self) -> &str {
        match self {
            Self::Inventory => "inventory",
            Self::Presence => "presence",
            Self::Import => "import",
            Self::Resolve => "resolve",
            Self::Invoke => "invoke",
            Self::Click => "click",
            Self::Mount => "mount",
            Self::Open => "open",
            Self::Close => "close",
            Self::Focus => "focus",
            Self::Move => "move",
            Self::Resize => "resize",
            Self::Geom => "geom",
            Self::TypeText => "type_text",
            Self::PressKey => "press_key",
            Self::Select => "select",
            Self::Scroll => "scroll",
            Self::Drag => "drag",
            Self::WaitUntil => "wait_until",
            Self::AssertState => "assert_state",
            Self::Observe => "observe",
            Self::Extension { id, .. } => id.as_str(),
        }
    }
}

impl fmt::Display for SeamVerb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.wire_name())
    }
}

/// One catalog entry — absorbed whole from `dpl-coach-runtime`'s retired
/// `verbs.rs` (`VerbInfo { name, status, target }`), every string now a
/// type (S2 plan §4.5 disposition).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeamVerbDescriptor {
    pub verb: &'static str,
    pub status: VerbStatus,
    pub target: SeamTarget,
}

/// The stable catalog the control plane lists (`GET /verbs` derives from
/// this — one source of truth). The 20 rows ratified 2026-08-01 carried
/// verbatim, plus `geom` (shipped since CX-events-01, battery-proven,
/// honest entry at last).
pub const fn seam_verb_catalog() -> &'static [SeamVerbDescriptor] {
    SEAM_VERB_CATALOG
}

const fn row(verb: &'static str, status: VerbStatus, target: SeamTarget) -> SeamVerbDescriptor {
    SeamVerbDescriptor {
        verb,
        status,
        target,
    }
}

const SEAM_VERB_CATALOG: &[SeamVerbDescriptor] = {
    use SeamTarget as T;
    use VerbStatus as S;
    &[
        row("inventory", S::Shipped, T::Desktop),
        row("presence", S::Shipped, T::App),
        row("import", S::Shipped, T::App),
        row("resolve", S::Shipped, T::Fabric),
        row("invoke", S::Shipped, T::Fabric),
        row("click", S::Shipped, T::Fabric),
        row("mount", S::Shipped, T::Fabric),
        row("move", S::Shipped, T::Window),
        row("resize", S::Shipped, T::Window),
        row("focus", S::Shipped, T::Window),
        row("geom", S::Shipped, T::Window),
        row("observe", S::Shipped, T::App),
        row("open", S::Shipped, T::App),
        row("close", S::Shipped, T::App),
        row("type_text", S::Shipped, T::App),
        row("press_key", S::Shipped, T::App),
        row("select", S::Shipped, T::App),
        row("scroll", S::Shipped, T::App),
        row("drag", S::Planned, T::Fabric),
        row("wait_until", S::Shipped, T::Predicate),
        row("assert_state", S::Shipped, T::App),
    ]
};

// ── the effect contract ─────────────────────────────────────────────────

/// Outer window geometry in root coordinates — the wire shape of the
/// runtime's `OuterGeom`, nothing dropped (`window_id` included).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowGeom {
    pub window_id: u32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl WindowGeom {
    pub fn as_csv(self) -> String {
        format!("{},{},{},{}", self.x, self.y, self.w, self.h)
    }
}

/// The typed observables one side of an effect contract may carry
/// (seam ADR §2.1). Every field optional: a verb reports what its
/// oracle actually observed, nothing invented.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Observables {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rect: Option<WindowGeom>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub running: Option<bool>,
}

/// What every mutating verb returns (seam ADR §2.1 + §7.3): did the
/// world observably change, what was before/after, on which rung. The
/// `extra` map is the §7 additive lane — pack-declared observables ride
/// there before they earn a typed field, and readers that do not know a
/// key must never fail on it (compat test below).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectContract {
    pub ok: bool,
    /// The ratified wire name (`SeamVerb::wire_name`).
    pub verb: String,
    /// The app as addressed (AT-SPI name today; resolves to `AppId`
    /// when packs attach, S3b).
    pub app: String,
    pub rung: TransportRung,
    /// Set when the ladder fell through from a higher rung — downgrades
    /// are first-class facts on the wire, same as in the S1 spans.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downgraded_from: Option<TransportRung>,
    pub effect_observed: bool,
    #[serde(default, skip_serializing_if = "Observables::is_empty")]
    pub before: Observables,
    #[serde(default, skip_serializing_if = "Observables::is_empty")]
    pub after: Observables,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, String>,
}

impl Observables {
    pub fn is_empty(&self) -> bool {
        self.text.is_none()
            && self.rect.is_none()
            && self.focus.is_none()
            && self.selection.is_none()
            && self.running.is_none()
    }
}

// ── the normalized fact stream ──────────────────────────────────────────

/// Facts the seam observes on a foreign target (seam ADR §2.2) — the
/// typed shape of the daemon's `/events` stream. The name collides with
/// `dpl_fabric::SeamEvent` (the in-process ledger fact) by accepted
/// precedent (two `Fabric`s already coexist); consumers qualify.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeamEvent {
    ObserveStart {
        app: String,
        seconds: u64,
    },
    /// First sighting of the window this observation window.
    Geom {
        app: String,
        geom: WindowGeom,
    },
    Moved {
        app: String,
        before: WindowGeom,
        after: WindowGeom,
        dx: i32,
        dy: i32,
    },
    Resized {
        app: String,
        before: WindowGeom,
        after: WindowGeom,
        dw: i32,
        dh: i32,
    },
    Focused {
        app: String,
    },
    TextChanged {
        app: String,
        before: String,
        after: String,
    },
    StateChanged {
        app: String,
        detail: String,
    },
    LayoutChanged {
        app: String,
    },
    ObserveEnd {
        app: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── S3a law tables, moved with the types (S2 plan §4.5) ────────────

    #[test]
    fn app_id_parse_law_table() {
        // Accepted.
        assert!(AppId::parse("org.blender.Blender").is_ok());
        assert!(AppId::parse("org.gnome.Calculator").is_ok());
        assert!(AppId::parse("com.example.app-2").is_ok());
        assert!(AppId::parse("x.unresolved:mystery-app").is_ok());
        // Refused.
        assert!(AppId::parse("blender").is_err()); // one segment
        assert!(AppId::parse("").is_err());
        assert!(AppId::parse("org..blender").is_err()); // empty segment
        assert!(AppId::parse("org.blender/Blender").is_err()); // charset
        assert!(AppId::parse("x.blender.set-mode").is_err()); // reserved x.
        assert!(AppId::parse("x.unresolved:").is_err()); // empty slug
        assert!(AppId::parse("x.unresolved:Has Space").is_err()); // slug charset
    }

    #[test]
    fn unresolved_mints_a_valid_slugged_id() {
        let id = AppId::unresolved("Mystery App 3.1!");
        assert_eq!(id.as_str(), "x.unresolved:mystery-app-3-1");
        assert!(id.is_unresolved());
        // Whatever unresolved() mints must itself pass parse.
        assert_eq!(AppId::parse(id.as_str()).unwrap(), id);
        assert_eq!(
            AppId::unresolved("!!!").as_str(),
            "x.unresolved:unknown-app"
        );
    }

    #[test]
    fn resolved_ids_are_not_unresolved() {
        assert!(!AppId::parse("org.blender.Blender").unwrap().is_unresolved());
    }

    #[test]
    fn pack_id_carries_the_cell() {
        let app = AppId::parse("org.blender.Blender").unwrap();
        let cell = CellKey::new("4.2", "linux", "en");
        assert_eq!(
            PackId::new(&app, &cell).as_str(),
            "org.blender.Blender@4.2/linux/en"
        );
    }

    // ── wire honesty for validated ids (S2 plan proof 3) ───────────────

    #[test]
    fn serde_cannot_smuggle_an_invalid_app_id() {
        let good: AppId = serde_json::from_str("\"org.blender.Blender\"").unwrap();
        assert_eq!(good.as_str(), "org.blender.Blender");
        assert_eq!(
            serde_json::to_string(&good).unwrap(),
            "\"org.blender.Blender\""
        );
        // The law holds on the wire: one segment, reserved x., bad slug.
        assert!(serde_json::from_str::<AppId>("\"blender\"").is_err());
        assert!(serde_json::from_str::<AppId>("\"x.blender.set-mode\"").is_err());
        assert!(serde_json::from_str::<AppId>("\"x.unresolved:Has Space\"").is_err());
    }

    // ── the verb catalog (S2 plan proofs 1, 2, 7) ──────────────────────

    #[test]
    fn every_catalog_name_parses_and_wire_names_round_trip() {
        for row in seam_verb_catalog() {
            let verb = SeamVerb::parse(row.verb)
                .unwrap_or_else(|e| panic!("catalog name must parse: {e}"));
            assert_eq!(verb.wire_name(), row.verb, "round trip {}", row.verb);
        }
        // The shipped alias parses to Geom but is never emitted.
        assert_eq!(SeamVerb::parse("window_geom").unwrap(), SeamVerb::Geom);
        assert_eq!(SeamVerb::parse("window_geom").unwrap().wire_name(), "geom");
        // Garbage refuses by name.
        let err = SeamVerb::parse("teleport").unwrap_err();
        assert_eq!(err.raw, "teleport");
    }

    #[test]
    fn extension_verbs_require_the_x_namespace() {
        let ext = SeamVerb::parse("x.blender.set-mode").unwrap();
        assert_eq!(ext.wire_name(), "x.blender.set-mode");
        assert!(matches!(ext, SeamVerb::Extension { .. }));
        assert!(SeamExtensionId::parse("blender.set-mode").is_err());
        assert!(SeamExtensionId::parse("x.").is_err());
        assert!(SeamExtensionId::parse("x.Has Space").is_err());
    }

    /// No-silent-drop census (S2 plan §4.5 enforced in code): the new
    /// catalog is a superset of the 20 rows `coach-runtime/src/verbs.rs`
    /// carried on 2026-08-01, with identical status/target per name.
    /// This literal copy of the retired file's data IS the disposition
    /// table's proof.
    #[test]
    fn the_catalog_absorbs_the_retired_verbs_rs_whole() {
        let retired: &[(&str, &str, &str)] = &[
            ("inventory", "shipped", "desktop"),
            ("presence", "shipped", "app"),
            ("import", "shipped", "app"),
            ("resolve", "shipped", "fabric"),
            ("invoke", "shipped", "fabric"),
            ("click", "shipped", "fabric"),
            ("mount", "shipped", "fabric"),
            ("move", "shipped", "window"),
            ("resize", "shipped", "window"),
            ("focus", "shipped", "window"),
            ("observe", "shipped", "app"),
            ("open", "shipped", "app"),
            ("close", "shipped", "app"),
            ("type_text", "shipped", "app"),
            ("press_key", "shipped", "app"),
            ("select", "shipped", "app"),
            ("scroll", "shipped", "app"),
            ("drag", "planned", "fabric"),
            ("wait_until", "shipped", "predicate"),
            ("assert_state", "shipped", "app"),
        ];
        for (name, status, target) in retired {
            let row = seam_verb_catalog()
                .iter()
                .find(|r| r.verb == *name)
                .unwrap_or_else(|| panic!("retired verb {name} was silently dropped"));
            assert_eq!(row.status.as_str(), *status, "{name} status");
            assert_eq!(row.target.as_str(), *target, "{name} target");
        }
        // Exactly one addition beyond the retired rows: geom (shipped in
        // code + battery since 2026-08-01, cataloged honestly by S2).
        assert_eq!(seam_verb_catalog().len(), retired.len() + 1);
        let geom = seam_verb_catalog()
            .iter()
            .find(|r| r.verb == "geom")
            .unwrap();
        assert_eq!(geom.status, VerbStatus::Shipped);
        assert_eq!(geom.target, SeamTarget::Window);
    }

    // ── the ladder + contract (S2 plan proofs 4, 5) ────────────────────

    #[test]
    fn rung_labels_are_the_s1_parquet_vocabulary() {
        assert_eq!(TransportRung::R0Native.label(), "r0");
        assert_eq!(TransportRung::R1Semantic.label(), "r1");
        assert_eq!(TransportRung::R2Injection.label(), "r2");
        assert_eq!(TransportRung::R3Vision.label(), "r3");
        for rung in [
            TransportRung::R0Native,
            TransportRung::R1Semantic,
            TransportRung::R2Injection,
            TransportRung::R3Vision,
        ] {
            assert_eq!(TransportRung::parse(rung.label()), Some(rung));
            // Serde speaks the same labels as `label()` — one vocabulary.
            assert_eq!(
                serde_json::to_string(&rung).unwrap(),
                format!("\"{}\"", rung.label())
            );
        }
        // Ordering is quality: promotion moves toward R0.
        assert!(TransportRung::R0Native < TransportRung::R1Semantic);
        assert!(TransportRung::R2Injection < TransportRung::R3Vision);
    }

    #[test]
    fn an_effect_contract_with_unknown_keys_still_reads() {
        // The §7.1 additive law: a v2.7+ writer may add fields this
        // reader does not know; reading must not fail.
        let future = r#"{
            "ok": true, "verb": "move", "app": "gnome-calculator",
            "rung": "r2", "effect_observed": true,
            "before": {"rect": {"window_id": 7, "x": 0, "y": 0, "w": 400, "h": 300},
                       "future_observable": "ignored"},
            "after": {"rect": {"window_id": 7, "x": 60, "y": 40, "w": 400, "h": 300}},
            "extra": {"pack_declared": "value"},
            "future_field": {"nested": true}
        }"#;
        let contract: EffectContract = serde_json::from_str(future).expect("additive law");
        assert!(contract.ok && contract.effect_observed);
        assert_eq!(contract.rung, TransportRung::R2Injection);
        assert_eq!(contract.before.rect.unwrap().w, 400);
        assert_eq!(contract.extra["pack_declared"], "value");
        // And a minimal contract (no observables, no extra) also reads.
        let minimal = r#"{"ok":true,"verb":"focus","app":"x","rung":"r2","effect_observed":false}"#;
        let contract: EffectContract = serde_json::from_str(minimal).expect("defaults");
        assert!(contract.before.is_empty() && contract.extra.is_empty());
    }

    #[test]
    fn seam_events_speak_the_streams_established_kind_names() {
        let event = SeamEvent::Moved {
            app: "gnome-calculator".into(),
            before: WindowGeom {
                window_id: 7,
                x: 0,
                y: 0,
                w: 400,
                h: 300,
            },
            after: WindowGeom {
                window_id: 7,
                x: 60,
                y: 0,
                w: 400,
                h: 300,
            },
            dx: 60,
            dy: 0,
        };
        let encoded = serde_json::to_string(&event).unwrap();
        assert!(encoded.contains("\"kind\":\"moved\""), "{encoded}");
        assert_eq!(serde_json::from_str::<SeamEvent>(&encoded).unwrap(), event);
        // The SSE framing events keep their wire spellings.
        let start = SeamEvent::ObserveStart {
            app: "x".into(),
            seconds: 20,
        };
        assert!(serde_json::to_string(&start)
            .unwrap()
            .contains("\"kind\":\"observe_start\""));
    }
}
