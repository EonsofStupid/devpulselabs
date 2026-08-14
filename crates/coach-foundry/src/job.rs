//! The Foundry job runner — staged, checkpointed, resumable, purgeable.
//!
//! A job builds ONE pack cell for one live app. Census and Resolve
//! re-observe live truth every run; Fabric/Probe/Induct/Battery
//! checkpoint to `packs/<app-id>/foundry/` and are skipped on resume;
//! Publish is an idempotent signed registry write. `purge` moves the
//! whole pack directory into the registry's `.trash` (content is never
//! destroyed — document law), and `remake` is purge + run.

use crate::artifacts::{
    battery_from_table, battery_to_table, probes_from_table, probes_to_table, read_table,
    write_table, AnchorNode, Anchors, BatteryRow, ProbeRow,
};
use crate::probe::{probe_verbs, run_battery, run_probes, SpanSink};
use crate::stage::FoundryStage;
use dpl_coach_fabric::{find_inventory_item, mint, MintOutcome, MintRequest};
use dpl_coach_pack::{
    FingerprintBundle, Pack, PackRegistry, PromotedRung, Resolution, VerbGrant, CURRENT_SEAM,
    PACK_SCHEMA,
};
use dpl_coach_runtime::probe_presence;
use egui_pilot_ai_protocol::seam::{AppId, CellKey};
use std::path::{Path, PathBuf};
use toml::{Table, Value};

/// What to build and where. Roots are explicit — the hub hands its
/// userdata roots, drills hand temp dirs (root-agnostic law).
#[derive(Debug, Clone)]
pub struct FoundryRequest {
    /// Inventory selector: desktop id, display name, or live AT-SPI name.
    pub selector: String,
    /// Pack registry root (the directory that contains `packs/`).
    pub registry_root: PathBuf,
    /// Seam-span Parquet root — the same layout the app and daemon write.
    pub analytics_root: PathBuf,
    /// Probe repetitions per verb (reliability denominator). Default 3.
    pub runs_per_verb: u32,
    /// Test hook: stop cleanly after this stage (simulates a kill).
    pub halt_after: Option<FoundryStage>,
}

impl FoundryRequest {
    pub fn new(
        selector: impl Into<String>,
        registry_root: impl Into<PathBuf>,
        analytics_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            selector: selector.into(),
            registry_root: registry_root.into(),
            analytics_root: analytics_root.into(),
            runs_per_verb: 3,
            halt_after: None,
        }
    }
}

/// One progress fact for the hub's console.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageReport {
    pub stage: FoundryStage,
    pub status: StageStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageStatus {
    Started,
    /// Checkpoint artifact survived a previous run — stage not repeated.
    Skipped,
    Done,
}

/// How the job ended. Every arm is honest — `Unresolved` and `Refused`
/// are outcomes, not errors to hide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoundryOutcome {
    Published {
        app_id: AppId,
        pack_path: PathBuf,
        warnings: Vec<String>,
    },
    /// The `halt_after` test hook fired.
    Halted {
        after: FoundryStage,
    },
    /// Evidence named no installed pack and minted no identity — the
    /// operator (or a future pack install) must name this app first.
    Unresolved {
        display_name: String,
        tip: String,
    },
    Refused {
        stage: FoundryStage,
        reason: String,
    },
}

/// The per-app Foundry checkpoint directory, beside the pack.
pub fn foundry_dir(registry: &PackRegistry, id: &AppId) -> PathBuf {
    registry.pack_dir(id).join("foundry")
}

/// A stage is checkpointed when its artifact survives on disk.
pub fn is_checkpointed(dir: &Path, stage: FoundryStage) -> bool {
    stage
        .checkpoint_artifact()
        .is_some_and(|name| dir.join(name).is_file())
}

/// Mint an [`AppId`] from the evidence itself: a reverse-DNS desktop id
/// (`org.gnome.Calculator.desktop`) IS an identity claim; a bare one
/// (`blender.desktop`) is not — those stay unresolved until a real pack
/// names them (promotion law: never guess an identity).
pub fn mint_app_id_from_evidence(bundle: &FingerprintBundle) -> Option<AppId> {
    let desktop_id = bundle.desktop_id.as_deref()?;
    let stem = desktop_id.trim_end_matches(".desktop");
    if stem.split('.').filter(|s| !s.is_empty()).count() < 3 {
        return None;
    }
    AppId::parse(stem).ok()
}

/// Elect the seed control from minted anchors: first named, actionable
/// node in walk order (seam ADR §6.1). Returns `None` for fabrics with
/// no actionable nodes — honest, the pack then carries no seed.
pub fn elect_seed(nodes: &[AnchorNode]) -> Option<String> {
    nodes
        .iter()
        .find(|n| !n.actions.is_empty() && !n.name.is_empty())
        .map(|n| n.name.clone())
}

/// Fold anchors + probe stats into the pack draft. An existing
/// registry pack is the base (identity, ladder, extras, landmarks are
/// PRESERVED); a fresh app starts from the evidence identity.
pub fn induct_pack(
    base: Option<Pack>,
    evidence_identity: dpl_coach_pack::Identity,
    version: Option<String>,
    seed: Option<String>,
    probes: &[ProbeRow],
) -> Pack {
    let mut pack = base.unwrap_or_else(|| Pack {
        schema: PACK_SCHEMA,
        min_seam: CURRENT_SEAM,
        identity: evidence_identity,
        cell: CellKey::new("unknown", "unknown", "unknown"),
        ladder: Vec::new(),
        verbs: Vec::new(),
        teaching_landmarks: Vec::new(),
        teaching_seed_control: None,
        extras: Table::new(),
    });
    pack.schema = PACK_SCHEMA;
    pack.min_seam = CURRENT_SEAM;
    pack.cell = CellKey::this_machine(version.unwrap_or_else(|| "unknown".to_string()));
    if seed.is_some() {
        pack.teaching_seed_control = seed;
    }
    // Grants are what the probes MEASURED — never asserted. A verb that
    // never showed its effect earns no grant.
    pack.verbs = probes
        .iter()
        .filter(|row| row.effect_ok > 0)
        .map(|row| VerbGrant {
            verb: row.verb.clone(),
            rung: PromotedRung::R1Semantic,
            reliability: Some(row.reliability()),
            oracle: Some(row.oracle.clone()),
            extras: Table::new(),
        })
        .collect();
    pack
}

/// Job-cursor sidecar (`job.toml`) — advisory; the artifacts are the
/// real checkpoints. Records what ran and when for the hub's console.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobState {
    pub selector: String,
    pub app_id: Option<String>,
    pub completed: Vec<String>,
}

impl JobState {
    pub fn load(dir: &Path) -> Self {
        let Ok(doc) = read_table(&dir.join("job.toml")) else {
            return Self::default();
        };
        Self {
            selector: doc
                .get("selector")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            app_id: doc
                .get("app_id")
                .and_then(Value::as_str)
                .map(str::to_string),
            completed: match doc.get("completed") {
                Some(Value::Array(items)) => items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
                _ => Vec::new(),
            },
        }
    }

    pub fn store(&self, dir: &Path) -> Result<(), String> {
        let mut doc = Table::new();
        doc.insert("schema".into(), Value::Integer(1));
        doc.insert("selector".into(), Value::String(self.selector.clone()));
        if let Some(id) = &self.app_id {
            doc.insert("app_id".into(), Value::String(id.clone()));
        }
        doc.insert(
            "completed".into(),
            Value::Array(self.completed.iter().cloned().map(Value::String).collect()),
        );
        doc.insert(
            "updated_ms".into(),
            Value::Integer(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0),
            ),
        );
        write_table(&dir.join("job.toml"), &doc)
    }
}

/// Purge one app's pack directory into `packs/.trash/<id>-<ms>` —
/// content moved aside, never destroyed. Returns the trash path.
pub fn purge(registry_root: &Path, id: &AppId) -> Result<PathBuf, String> {
    let registry = PackRegistry::open(registry_root);
    let dir = registry.pack_dir(id);
    if !dir.exists() {
        return Err(format!("nothing to purge — {} has no pack cell", id));
    }
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let trash_dir = registry.packs_dir().join(".trash");
    std::fs::create_dir_all(&trash_dir)
        .map_err(|e| format!("create {}: {e}", trash_dir.display()))?;
    let dest = trash_dir.join(format!("{id}-{ms}"));
    std::fs::rename(&dir, &dest)
        .map_err(|e| format!("trash {} → {}: {e}", dir.display(), dest.display()))?;
    Ok(dest)
}

/// Purge + run — the iteration loop's one-click "start this cell over".
pub fn remake(
    request: &FoundryRequest,
    id: &AppId,
    on_progress: impl FnMut(StageReport),
) -> FoundryOutcome {
    if let Err(reason) = purge(&request.registry_root, id) {
        // A missing cell is fine for remake — it just means "make".
        if !reason.contains("nothing to purge") {
            return FoundryOutcome::Refused {
                stage: FoundryStage::Census,
                reason,
            };
        }
    }
    run(request, on_progress)
}

/// Run (or resume) the pipeline. Blocking — callers put it on a worker
/// thread; progress flows through `on_progress`.
pub fn run(request: &FoundryRequest, mut on_progress: impl FnMut(StageReport)) -> FoundryOutcome {
    let mut report = |stage: FoundryStage, status: StageStatus, detail: String| {
        on_progress(StageReport {
            stage,
            status,
            detail,
        });
    };
    let registry = PackRegistry::open(&request.registry_root);

    // -- census: live truth, always fresh --------------------------------
    report(
        FoundryStage::Census,
        StageStatus::Started,
        format!("observing {:?}", request.selector),
    );
    let item = match find_inventory_item(&request.selector) {
        Ok(item) => item,
        Err(r) => {
            return FoundryOutcome::Refused {
                stage: FoundryStage::Census,
                reason: format!("{} — {}", r.reason, r.tip),
            }
        }
    };
    let Some(live) = item.live.clone() else {
        return FoundryOutcome::Refused {
            stage: FoundryStage::Census,
            reason: format!(
                "{} is not running — the Foundry probes a LIVE app; open it, then build",
                item.display_name
            ),
        };
    };
    let presence = probe_presence(&live.atspi_name);
    if !presence.is_real() {
        return FoundryOutcome::Refused {
            stage: FoundryStage::Census,
            reason: format!(
                "{} is on the a11y bus but has no real window — open it fully, then build",
                item.display_name
            ),
        };
    }
    let bundle = FingerprintBundle::from_inventory(&item);
    report(
        FoundryStage::Census,
        StageStatus::Done,
        format!(
            "live · atspi={:?} pid={:?} window={}×{}",
            live.atspi_name, presence.pid, presence.outer_w, presence.outer_h
        ),
    );
    if request.halt_after == Some(FoundryStage::Census) {
        return FoundryOutcome::Halted {
            after: FoundryStage::Census,
        };
    }

    // -- resolve: evidence → identity, always fresh -----------------------
    report(
        FoundryStage::Resolve,
        StageStatus::Started,
        "scoring evidence against installed packs".into(),
    );
    let (app_id, resolve_detail, evidence_identity) = match registry.resolve(&bundle) {
        Resolution::Resolved {
            app_id,
            score,
            matched,
        } => {
            let signals: Vec<&str> = matched.iter().map(|m| m.signal).collect();
            let detail = format!("resolved score={score} via {}", signals.join("+"));
            let identity = registry
                .load(&app_id)
                .map(|p| p.pack.identity)
                .unwrap_or_else(|_| synthesized_identity(&bundle, &app_id));
            (app_id, detail, identity)
        }
        Resolution::Unresolved { synthesized } => match mint_app_id_from_evidence(&bundle) {
            Some(minted) => {
                let detail = format!("minted {minted} from reverse-DNS desktop id");
                let identity = synthesized_identity_from(&synthesized, &minted);
                (minted, detail, identity)
            }
            None => {
                return FoundryOutcome::Unresolved {
                    display_name: synthesized.identity.display_name.clone(),
                    tip: "No pack claims this app and its desktop id names no identity. \
                          Install a pack for it, or author one with a real AppId first."
                        .into(),
                };
            }
        },
    };
    let dir = foundry_dir(&registry, &app_id);
    let mut job = JobState::load(&dir);
    job.selector = request.selector.clone();
    job.app_id = Some(app_id.to_string());
    if let Err(e) = write_census(&dir, &bundle, &live.atspi_name) {
        return refused(FoundryStage::Resolve, e);
    }
    if let Err(e) = write_resolve(&dir, &app_id, &resolve_detail) {
        return refused(FoundryStage::Resolve, e);
    }
    mark(&mut job, &dir, FoundryStage::Census);
    mark(&mut job, &dir, FoundryStage::Resolve);
    report(FoundryStage::Resolve, StageStatus::Done, resolve_detail);
    if request.halt_after == Some(FoundryStage::Resolve) {
        return FoundryOutcome::Halted {
            after: FoundryStage::Resolve,
        };
    }

    // -- fabric: mint anchors (checkpointed) ------------------------------
    let anchors = if is_checkpointed(&dir, FoundryStage::Fabric) {
        let loaded = read_table(&dir.join("anchors.toml"))
            .ok()
            .and_then(|t| Anchors::from_table(&t));
        match loaded {
            Some(a) => {
                report(
                    FoundryStage::Fabric,
                    StageStatus::Skipped,
                    format!("checkpoint holds {} anchors", a.nodes.len()),
                );
                a
            }
            None => {
                return refused(
                    FoundryStage::Fabric,
                    "anchors.toml checkpoint unreadable — purge and remake this cell".into(),
                )
            }
        }
    } else {
        report(
            FoundryStage::Fabric,
            StageStatus::Started,
            "walking the a11y tree".into(),
        );
        let mint_request = MintRequest {
            target_id: item.id.clone(),
            atspi_name: live.atspi_name.clone(),
            complexity: item.complexity,
            seed_name: None,
            budget: None,
        };
        let outcome = mint(&mint_request, None, |p| {
            report(
                FoundryStage::Fabric,
                StageStatus::Started,
                format!("{} · {} nodes", p.phase.as_str(), p.visited),
            );
        });
        let fabric = match outcome {
            MintOutcome::Ready(f) => f,
            MintOutcome::Refused(r) => {
                return refused(FoundryStage::Fabric, format!("{} — {}", r.reason, r.tip))
            }
            MintOutcome::Cancelled => {
                return refused(FoundryStage::Fabric, "mint cancelled".into())
            }
        };
        let nodes: Vec<AnchorNode> = fabric
            .nodes
            .iter()
            .map(|n| AnchorNode {
                path: n.path.clone(),
                role: n.role.clone(),
                name: n.name.clone(),
                actions: n.actions.clone(),
            })
            .collect();
        let anchors = Anchors {
            atspi_name: fabric.atspi_name.clone(),
            minted_ms: fabric.minted_ms,
            seed_control: elect_seed(&nodes),
            nodes,
        };
        if let Err(e) = write_table(&dir.join("anchors.toml"), &anchors.to_table()) {
            return refused(FoundryStage::Fabric, e);
        }
        report(
            FoundryStage::Fabric,
            StageStatus::Done,
            format!(
                "{} anchors in {}ms · seed={:?}",
                anchors.nodes.len(),
                anchors.minted_ms,
                anchors.seed_control
            ),
        );
        anchors
    };
    mark(&mut job, &dir, FoundryStage::Fabric);
    if request.halt_after == Some(FoundryStage::Fabric) {
        return FoundryOutcome::Halted {
            after: FoundryStage::Fabric,
        };
    }

    // -- probe: measured reliability (checkpointed) ------------------------
    let mut sink = SpanSink::open(&request.analytics_root);
    if let Some(e) = &sink.ledger_error {
        report(
            FoundryStage::Probe,
            StageStatus::Started,
            format!("WARNING: spans not recorded — {e}"),
        );
    }
    let probes = if is_checkpointed(&dir, FoundryStage::Probe) {
        let rows = read_table(&dir.join("probes.toml"))
            .map(|t| probes_from_table(&t))
            .unwrap_or_default();
        report(
            FoundryStage::Probe,
            StageStatus::Skipped,
            format!("checkpoint holds {} probed verbs", rows.len()),
        );
        rows
    } else {
        report(
            FoundryStage::Probe,
            StageStatus::Started,
            format!(
                "{} verbs × {} runs",
                probe_verbs().len(),
                request.runs_per_verb
            ),
        );
        let rows = run_probes(
            app_id.as_str(),
            &live.atspi_name,
            request.runs_per_verb,
            &mut sink,
            |verb, i, n| {
                report(
                    FoundryStage::Probe,
                    StageStatus::Started,
                    format!("{verb} {i}/{n}"),
                );
            },
        );
        if let Err(e) = write_table(&dir.join("probes.toml"), &probes_to_table(&rows)) {
            sink.flush();
            return refused(FoundryStage::Probe, e);
        }
        let granted = rows.iter().filter(|r| r.effect_ok > 0).count();
        report(
            FoundryStage::Probe,
            StageStatus::Done,
            format!("{granted}/{} verbs showed their effect", rows.len()),
        );
        rows
    };
    mark(&mut job, &dir, FoundryStage::Probe);
    if request.halt_after == Some(FoundryStage::Probe) {
        sink.flush();
        return FoundryOutcome::Halted {
            after: FoundryStage::Probe,
        };
    }

    // -- induct: fold the draft (checkpointed) -----------------------------
    let pack = {
        let base = registry.load(&app_id).ok().map(|p| p.pack);
        let pack = induct_pack(
            base,
            evidence_identity,
            bundle.version.clone(),
            anchors.seed_control.clone(),
            &probes,
        );
        match pack.to_signed_toml() {
            Ok(text) => {
                if let Err(e) = std::fs::write(dir.join("pack.draft.toml"), text) {
                    sink.flush();
                    return refused(FoundryStage::Induct, format!("write draft: {e}"));
                }
            }
            Err(e) => {
                sink.flush();
                return refused(FoundryStage::Induct, e.to_string());
            }
        }
        report(
            FoundryStage::Induct,
            StageStatus::Done,
            format!(
                "draft: {} grants · cell {}",
                pack.verbs.len(),
                pack.cell.label()
            ),
        );
        pack
    };
    mark(&mut job, &dir, FoundryStage::Induct);
    if request.halt_after == Some(FoundryStage::Induct) {
        sink.flush();
        return FoundryOutcome::Halted {
            after: FoundryStage::Induct,
        };
    }

    // -- battery: every grant must hold (checkpointed) ---------------------
    let battery = if is_checkpointed(&dir, FoundryStage::Battery) {
        let rows = read_table(&dir.join("battery.toml"))
            .map(|t| battery_from_table(&t))
            .unwrap_or_default();
        report(
            FoundryStage::Battery,
            StageStatus::Skipped,
            format!("checkpoint holds {} checks", rows.len()),
        );
        rows
    } else {
        report(
            FoundryStage::Battery,
            StageStatus::Started,
            format!("re-verifying {} grants", pack.verbs.len()),
        );
        let verbs: Vec<String> = pack.verbs.iter().map(|g| g.verb.clone()).collect();
        let rows = run_battery(app_id.as_str(), &live.atspi_name, &verbs, &mut sink);
        if let Err(e) = write_table(&dir.join("battery.toml"), &battery_to_table(&rows)) {
            sink.flush();
            return refused(FoundryStage::Battery, e);
        }
        rows
    };
    sink.flush();
    let failed: Vec<&BatteryRow> = battery.iter().filter(|r| !r.ok).collect();
    if !failed.is_empty() {
        let names: Vec<&str> = failed.iter().map(|r| r.verb.as_str()).collect();
        return refused(
            FoundryStage::Battery,
            format!(
                "battery red on {} — the pack does NOT publish; purge and remake, or \
                 investigate with the inspector",
                names.join(", ")
            ),
        );
    }
    report(
        FoundryStage::Battery,
        StageStatus::Done,
        format!("{} checks green", battery.len()),
    );
    mark(&mut job, &dir, FoundryStage::Battery);
    if request.halt_after == Some(FoundryStage::Battery) {
        return FoundryOutcome::Halted {
            after: FoundryStage::Battery,
        };
    }

    // -- publish: signed registry write ------------------------------------
    let pack_path = match registry.write(&pack) {
        Ok(path) => path,
        Err(e) => return refused(FoundryStage::Publish, e.to_string()),
    };
    mark(&mut job, &dir, FoundryStage::Publish);
    let warnings = match sinkless_verify(&registry, &app_id) {
        Ok(w) => w,
        Err(e) => vec![format!("post-publish verify: {e}")],
    };
    report(
        FoundryStage::Publish,
        StageStatus::Done,
        format!("{}", pack_path.display()),
    );
    FoundryOutcome::Published {
        app_id,
        pack_path,
        warnings,
    }
}

/// Re-load what was just published — the loader's warnings are the
/// honest post-publish report the hub shows on the Profile card.
fn sinkless_verify(registry: &PackRegistry, id: &AppId) -> Result<Vec<String>, String> {
    registry
        .load(id)
        .map(|p| p.warnings)
        .map_err(|e| e.to_string())
}

fn refused(stage: FoundryStage, reason: String) -> FoundryOutcome {
    FoundryOutcome::Refused { stage, reason }
}

fn mark(job: &mut JobState, dir: &Path, stage: FoundryStage) {
    let slug = stage.slug().to_string();
    if !job.completed.contains(&slug) {
        job.completed.push(slug);
    }
    let _ = job.store(dir);
}

fn write_census(dir: &Path, bundle: &FingerprintBundle, atspi_name: &str) -> Result<(), String> {
    let mut doc = Table::new();
    doc.insert("schema".into(), Value::Integer(1));
    doc.insert("atspi_name".into(), Value::String(atspi_name.into()));
    let mut put = |key: &str, v: &Option<String>| {
        if let Some(s) = v {
            doc.insert(key.into(), Value::String(s.clone()));
        }
    };
    put("desktop_id", &bundle.desktop_id);
    put("startup_wm_class", &bundle.startup_wm_class);
    put("wm_class", &bundle.wm_class);
    put("atspi_live_name", &bundle.atspi_name);
    put("exe_basename", &bundle.exe_basename);
    put("exe_path", &bundle.exe_path);
    put("version", &bundle.version);
    write_table(&dir.join("census.toml"), &doc)
}

fn write_resolve(dir: &Path, id: &AppId, detail: &str) -> Result<(), String> {
    let mut doc = Table::new();
    doc.insert("schema".into(), Value::Integer(1));
    doc.insert("app_id".into(), Value::String(id.to_string()));
    doc.insert("detail".into(), Value::String(detail.into()));
    write_table(&dir.join("resolve.toml"), &doc)
}

fn synthesized_identity(bundle: &FingerprintBundle, id: &AppId) -> dpl_coach_pack::Identity {
    let synthesized = dpl_coach_pack::synthesize_generic(bundle);
    synthesized_identity_from(&synthesized, id)
}

fn synthesized_identity_from(synthesized: &Pack, id: &AppId) -> dpl_coach_pack::Identity {
    let mut identity = synthesized.identity.clone();
    identity.app_id = id.clone();
    identity
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_pilot_ai_protocol::seam::SeamVerb;

    fn temp_root(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dpl-foundry-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn minted_identity_needs_reverse_dns_evidence() {
        let claim = FingerprintBundle {
            desktop_id: Some("org.gnome.Calculator.desktop".into()),
            ..Default::default()
        };
        assert_eq!(
            mint_app_id_from_evidence(&claim).unwrap().as_str(),
            "org.gnome.Calculator"
        );
        let bare = FingerprintBundle {
            desktop_id: Some("blender.desktop".into()),
            ..Default::default()
        };
        assert_eq!(mint_app_id_from_evidence(&bare), None);
        assert_eq!(
            mint_app_id_from_evidence(&FingerprintBundle::default()),
            None
        );
    }

    #[test]
    fn seed_election_prefers_first_named_actionable() {
        let nodes = vec![
            AnchorNode {
                path: "0/0".into(),
                role: "label".into(),
                name: "display".into(),
                actions: vec![],
            },
            AnchorNode {
                path: "0/1".into(),
                role: "push button".into(),
                name: String::new(),
                actions: vec!["click".into()],
            },
            AnchorNode {
                path: "0/2".into(),
                role: "push button".into(),
                name: "7".into(),
                actions: vec!["click".into()],
            },
        ];
        assert_eq!(elect_seed(&nodes).as_deref(), Some("7"));
        assert_eq!(elect_seed(&nodes[..2]), None);
    }

    #[test]
    fn every_probe_verb_speaks_the_protocol_catalog() {
        for verb in probe_verbs() {
            let parsed = SeamVerb::parse(verb)
                .unwrap_or_else(|e| panic!("probe verb {verb:?} not in catalog: {e:?}"));
            assert_eq!(parsed.wire_name(), verb);
        }
    }

    #[test]
    fn induct_grants_only_what_showed_effect_and_keeps_the_base() {
        let base = Pack {
            schema: PACK_SCHEMA,
            min_seam: CURRENT_SEAM,
            identity: dpl_coach_pack::Identity {
                app_id: AppId::parse("org.gnome.Calculator").unwrap(),
                display_name: "Calculator".into(),
                desktop_ids: vec!["org.gnome.Calculator.desktop".into()],
                wm_classes: vec![],
                atspi_names: vec!["gnome-calculator".into()],
                exe_basenames: vec![],
                aliases: vec!["calc".into()],
            },
            cell: CellKey::new("47", "linux", "en"),
            ladder: vec![("open".into(), PromotedRung::R0Native)],
            verbs: Vec::new(),
            teaching_landmarks: vec!["keypad".into()],
            teaching_seed_control: None,
            extras: Table::new(),
        };
        let probes = vec![
            ProbeRow {
                verb: "geom".into(),
                rung: "r1".into(),
                oracle: "geometry-read".into(),
                runs: 3,
                effect_ok: 3,
                total_ms: 30,
            },
            ProbeRow {
                verb: "resize".into(),
                rung: "r1".into(),
                oracle: "geometry-delta".into(),
                runs: 3,
                effect_ok: 0,
                total_ms: 900,
            },
        ];
        let pack = induct_pack(
            Some(base),
            dpl_coach_pack::synthesize_generic(&FingerprintBundle::default()).identity,
            None,
            Some("7".into()),
            &probes,
        );
        // The base identity, ladder and landmarks survive the re-induct.
        assert_eq!(pack.identity.aliases, vec!["calc".to_string()]);
        assert_eq!(pack.ladder.len(), 1);
        assert_eq!(pack.teaching_landmarks, vec!["keypad".to_string()]);
        assert_eq!(pack.teaching_seed_control.as_deref(), Some("7"));
        // Only geom earned a grant; its reliability is the measured rate.
        assert_eq!(pack.verbs.len(), 1);
        assert_eq!(pack.verbs[0].verb, "geom");
        assert_eq!(pack.verbs[0].reliability, Some(1.0));
        // The cell is THIS machine's, not the stale base cell.
        assert_eq!(pack.cell.os, std::env::consts::OS);
    }

    #[test]
    fn checkpoint_detection_reads_the_artifact_law() {
        let root = temp_root("ckpt");
        std::fs::create_dir_all(&root).unwrap();
        assert!(!is_checkpointed(&root, FoundryStage::Fabric));
        std::fs::write(root.join("anchors.toml"), "schema = 1\n").unwrap();
        assert!(is_checkpointed(&root, FoundryStage::Fabric));
        // Live-truth stages never checkpoint.
        assert!(!is_checkpointed(&root, FoundryStage::Census));
        assert!(!is_checkpointed(&root, FoundryStage::Publish));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn job_state_round_trips() {
        let root = temp_root("job");
        std::fs::create_dir_all(&root).unwrap();
        let mut job = JobState {
            selector: "Calculator".into(),
            app_id: Some("org.gnome.Calculator".into()),
            completed: vec!["census".into(), "resolve".into()],
        };
        job.store(&root).unwrap();
        assert_eq!(JobState::load(&root), job);
        job.completed.push("fabric".into());
        job.store(&root).unwrap();
        assert_eq!(JobState::load(&root).completed.len(), 3);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn purge_moves_the_cell_to_trash_never_destroys() {
        let root = temp_root("purge");
        let id = AppId::parse("org.gnome.Calculator").unwrap();
        let registry = PackRegistry::open(&root);
        let dir = registry.pack_dir(&id);
        std::fs::create_dir_all(dir.join("foundry")).unwrap();
        std::fs::write(dir.join("pack.toml"), "schema = 1\n").unwrap();
        std::fs::write(dir.join("foundry/anchors.toml"), "schema = 1\n").unwrap();

        let trash = purge(&root, &id).unwrap();
        assert!(!dir.exists());
        assert!(trash.join("pack.toml").is_file());
        assert!(trash.join("foundry/anchors.toml").is_file());
        // A second purge has nothing to move — honest error, no panic.
        assert!(purge(&root, &id).unwrap_err().contains("nothing to purge"));
        let _ = std::fs::remove_dir_all(root);
    }
}
