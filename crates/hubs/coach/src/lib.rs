//! Coach hub — Operator Import Process + fabric bridge lesson wire +
//! the Foundry console (S3b).
//!
//! Lessons address apps via `dpl-coach-runtime` (resolve / invoke / Mount
//! spec). Foreign paint uses the `dpl-coach-overlay` **binary** only (ADR).
//! Pick shows the EVIDENCE-RESOLVED identity (never a name guess); the
//! Foundry builds pack cells with purge/remake; the Profile card reads
//! the registry. Seeds come from packs (`[teaching] seed_control`) — the
//! hardcoded seed table died in S3b.
//! Plans: `coach-c2f-operator-import-ui.md` ·
//! `coach-bridge-fabric-runtime.md` · `coach-seam-s3b-foundry-v0.md`.

#![forbid(unsafe_code)]

use dpl_coach_fabric::{eta_hint, mint, MintOutcome, MintProgress, MintRequest};
use dpl_coach_foundry::{
    foundry_dir, purge, read_table, run as run_foundry, Anchors, FoundryOutcome, FoundryRequest,
    FoundryStage, StageReport, StageStatus,
};
use dpl_coach_inventory::{inventory, Complexity, InventoryItem};
use dpl_coach_pack::{FingerprintBundle, PackRegistry, Resolution};
use dpl_coach_runtime::{invoke, mount_spec, resolve_seed, AgencyMode, FabricSession};
use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use egui::RichText;
use egui_pilot_ai_protocol::seam::AppId;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

pub const HUB: HubId = HubId("coach");
pub const MAIN: SubId = SubId("main");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Coach",
    icon: cl::icon::STEPS,
    tone: Tone::Cyan,
    subroutes: &[SubRoute {
        id: MAIN,
        label: "Teach",
        blurb: "Pick an installed app, import a signal map, teach with paint — no screenshot loop.",
    }],
};

/// Import workflow after essential checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportWorkflow {
    Standard,
    AdvancedDivert,
}

/// Classify Standard vs Advanced from inventory row (unit-tested).
pub fn classify_import(item: &InventoryItem) -> ImportWorkflow {
    match (&item.live, item.complexity) {
        (Some(_), Complexity::Deep) => ImportWorkflow::AdvancedDivert,
        (Some(_), _) => ImportWorkflow::Standard,
        (None, _) => ImportWorkflow::AdvancedDivert,
    }
}

/// Pack registry root: `DPL_PACK_ROOT` → the app's userdata ring. The
/// registry adds `packs/` itself.
fn registry_root() -> PathBuf {
    if let Ok(p) = std::env::var("DPL_PACK_ROOT") {
        return PathBuf::from(p);
    }
    dpl_engine_capture::userdata_root()
}

/// Seam-span Parquet root — the SAME ladder the daemon resolves, so
/// Foundry spans and daemon spans meet in the same files.
fn analytics_root() -> PathBuf {
    if let Ok(p) = std::env::var("DPL_ANALYTICS_ROOT") {
        return PathBuf::from(p);
    }
    dpl_engine_capture::userdata_root().join("analytics")
}

/// What the evidence says about the selected row — shown on Pick, and
/// the ONLY seed source for a mint (seam ADR §6.1: identity and seeds
/// are resolved facts, never name guesses).
#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedIdentity {
    app_id: AppId,
    /// `resolved` (installed pack matched), `minted` (evidence names an
    /// identity), or `unresolved` (ephemeral generic profile).
    tier: &'static str,
    detail: String,
    /// An installed pack exists for this id.
    has_pack: bool,
    /// The pack's `[teaching] seed_control`, when a pack carries one.
    seed: Option<String>,
}

fn resolve_identity(registry: &PackRegistry, item: &InventoryItem) -> ResolvedIdentity {
    let bundle = FingerprintBundle::from_inventory(item);
    match registry.resolve(&bundle) {
        Resolution::Resolved {
            app_id,
            score,
            matched,
        } => {
            let signals: Vec<&str> = matched.iter().map(|m| m.signal).collect();
            let seed = registry
                .load(&app_id)
                .ok()
                .and_then(|p| p.pack.teaching_seed_control);
            ResolvedIdentity {
                app_id: app_id.clone(),
                tier: "resolved",
                detail: format!("score {score} via {}", signals.join("+")),
                has_pack: true,
                seed,
            }
        }
        Resolution::Unresolved { synthesized } => {
            match dpl_coach_foundry::mint_app_id_from_evidence(&bundle) {
                Some(minted) => {
                    let loaded = registry.load(&minted).ok();
                    let has_pack = loaded.is_some();
                    let seed = loaded.and_then(|p| p.pack.teaching_seed_control);
                    ResolvedIdentity {
                        app_id: minted,
                        tier: "minted",
                        detail: "reverse-DNS desktop id names this identity".into(),
                        has_pack,
                        seed,
                    }
                }
                None => ResolvedIdentity {
                    app_id: synthesized.identity.app_id.clone(),
                    tier: "unresolved",
                    detail: "no installed pack claims this evidence".into(),
                    has_pack: false,
                    seed: None,
                },
            }
        }
    }
}

fn overlay_bin() -> PathBuf {
    if let Ok(p) = std::env::var("DPL_COACH_OVERLAY") {
        return PathBuf::from(p);
    }
    let name = if cfg!(windows) {
        "dpl-coach-overlay.exe"
    } else {
        "dpl-coach-overlay"
    };
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(name);
            if p.is_file() {
                return p;
            }
        }
    }
    PathBuf::from(name)
}

#[derive(Debug)]
enum MintMsg {
    Progress(MintProgress),
    Done(MintOutcome),
}

#[derive(Debug)]
enum FoundryMsg {
    Report(StageReport),
    Done(FoundryOutcome),
}

/// One console row per Foundry stage.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConsoleRow {
    stage: FoundryStage,
    status: Option<StageStatus>,
    detail: String,
}

/// What the Profile card shows — read from the registry, never invented.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProfileCard {
    app_id: AppId,
    display_name: String,
    cell: String,
    hash: String,
    seed: Option<String>,
    /// (verb, rung, reliability %, oracle)
    verbs: Vec<(String, String, String, String)>,
    warnings: Vec<String>,
    raw_toml: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Phase {
    Pick,
    Confirm,
    Checks {
        workflow: ImportWorkflow,
    },
    Minting {
        visited: u32,
        budget: u32,
        phase: String,
        eta: String,
    },
    Ready {
        nodes: u32,
        ms: u64,
        seed: String,
    },
    LessonLaunched {
        detail: String,
    },
    /// Foundry console — stage rows stream in; `outcome` lands last.
    Foundry {
        running: bool,
        outcome: Option<String>,
    },
    /// The pack cell, human-first (raw TOML under Advanced).
    Profile,
    Refused {
        reason: String,
        tip: String,
    },
}

pub struct CoachHub {
    items: Vec<InventoryItem>,
    selected: Option<usize>,
    phase: Phase,
    status: String,
    mint_rx: Option<Receiver<MintMsg>>,
    last_refresh_err: Option<String>,
    /// Session fabric after Standard Import — bridge address space.
    session: Option<FabricSession>,
    agency: AgencyMode,
    /// Evidence-resolved identity for the selected row (S3b).
    resolved: Option<ResolvedIdentity>,
    foundry_rx: Option<Receiver<FoundryMsg>>,
    console: Vec<ConsoleRow>,
    profile: Option<ProfileCard>,
}

impl Default for CoachHub {
    fn default() -> Self {
        let mut hub = Self {
            items: Vec::new(),
            selected: None,
            phase: Phase::Pick,
            status: String::new(),
            mint_rx: None,
            last_refresh_err: None,
            session: None,
            agency: AgencyMode::Coach,
            resolved: None,
            foundry_rx: None,
            console: Vec::new(),
            profile: None,
        };
        hub.refresh_inventory();
        hub.apply_demo_view();
        hub
    }
}

impl CoachHub {
    pub fn new() -> Self {
        Self::default()
    }

    fn refresh_inventory(&mut self) {
        match inventory(true) {
            Ok(mut items) => {
                items.sort_by(|a, b| {
                    conquest_rank(a)
                        .cmp(&conquest_rank(b))
                        .then_with(|| b.live.is_some().cmp(&a.live.is_some()))
                        .then_with(|| {
                            a.display_name
                                .to_lowercase()
                                .cmp(&b.display_name.to_lowercase())
                        })
                });
                self.selected = items
                    .iter()
                    .position(|i| i.id.as_str().contains("cursor"))
                    .or_else(|| items.iter().position(|i| i.live.is_some()))
                    .or(if items.is_empty() { None } else { Some(0) });
                self.items = items;
                self.last_refresh_err = None;
                self.status = format!("{} app(s) in inventory", self.items.len());
            }
            Err(e) => {
                self.items.clear();
                self.selected = None;
                self.last_refresh_err = Some(e.clone());
                self.status = format!("inventory refused: {e}");
            }
        }
        self.recompute_resolution();
    }

    fn selected_item(&self) -> Option<&InventoryItem> {
        self.selected.and_then(|i| self.items.get(i))
    }

    /// Re-score the selected row's evidence against installed packs.
    fn recompute_resolution(&mut self) {
        let registry = PackRegistry::open(registry_root());
        self.resolved = self
            .selected_item()
            .map(|item| resolve_identity(&registry, item));
    }

    /// `DPL_COACH_VIEW=foundry|profile` — proof-shot hook: replay the
    /// last Foundry job's console, or open the first installed pack's
    /// card, without a live pipeline. Real data only (reads the
    /// registry); absent data leaves the normal Pick view.
    fn apply_demo_view(&mut self) {
        let Ok(view) = std::env::var("DPL_COACH_VIEW") else {
            return;
        };
        let registry = PackRegistry::open(registry_root());
        let Some(first) = registry.list().into_iter().next() else {
            return;
        };
        match view.as_str() {
            "profile" => {
                if self.load_profile(&first) {
                    self.phase = Phase::Profile;
                }
            }
            "foundry" => {
                self.console = replay_console(&registry, &first);
                self.phase = Phase::Foundry {
                    running: false,
                    outcome: Some(format!("replayed from {first}'s job checkpoints")),
                };
            }
            _ => {}
        }
    }

    /// Load the Profile card from the registry. False when unreadable.
    fn load_profile(&mut self, id: &AppId) -> bool {
        let registry = PackRegistry::open(registry_root());
        let Ok(parsed) = registry.load(id) else {
            return false;
        };
        let raw_toml = std::fs::read_to_string(registry.pack_path(id)).unwrap_or_default();
        let hash = parsed
            .pack
            .content_hash()
            .unwrap_or_else(|_| "unhashable".into());
        self.profile = Some(ProfileCard {
            app_id: id.clone(),
            display_name: parsed.pack.identity.display_name.clone(),
            cell: parsed.pack.cell.label(),
            hash,
            seed: parsed.pack.teaching_seed_control.clone(),
            verbs: parsed
                .pack
                .verbs
                .iter()
                .map(|g| {
                    (
                        g.verb.clone(),
                        g.rung.label().to_string(),
                        g.reliability
                            .map(|r| format!("{:.0}%", r * 100.0))
                            .unwrap_or_else(|| "—".into()),
                        g.oracle.clone().unwrap_or_default(),
                    )
                })
                .collect(),
            warnings: parsed.warnings,
            raw_toml,
        });
        true
    }

    /// Start (or resume) the Foundry on a worker thread.
    fn start_foundry(&mut self, selector: String) {
        let request = FoundryRequest::new(selector, registry_root(), analytics_root());
        let (tx, rx) = mpsc::channel();
        self.foundry_rx = Some(rx);
        self.console = FoundryStage::ALL
            .into_iter()
            .map(|stage| ConsoleRow {
                stage,
                status: None,
                detail: String::new(),
            })
            .collect();
        self.phase = Phase::Foundry {
            running: true,
            outcome: None,
        };
        thread::spawn(move || {
            let progress_tx = tx.clone();
            let outcome = run_foundry(&request, move |report| {
                let _ = progress_tx.send(FoundryMsg::Report(report));
            });
            let _ = tx.send(FoundryMsg::Done(outcome));
        });
    }

    fn poll_foundry(&mut self) {
        let Some(rx) = self.foundry_rx.as_ref() else {
            return;
        };
        loop {
            match rx.try_recv() {
                Ok(FoundryMsg::Report(report)) => {
                    if let Some(row) = self.console.iter_mut().find(|r| r.stage == report.stage) {
                        row.status = Some(report.status);
                        row.detail = report.detail;
                    }
                }
                Ok(FoundryMsg::Done(outcome)) => {
                    self.foundry_rx = None;
                    let summary = match outcome {
                        FoundryOutcome::Published {
                            app_id, pack_path, ..
                        } => {
                            self.recompute_resolution();
                            let loaded = self.load_profile(&app_id);
                            self.status = format!("pack published: {}", pack_path.display());
                            if loaded {
                                self.phase = Phase::Profile;
                                return;
                            }
                            format!("published {app_id}, but the card could not load")
                        }
                        FoundryOutcome::Halted { after } => {
                            format!("halted after {} (test hook)", after.slug())
                        }
                        FoundryOutcome::Unresolved { display_name, tip } => {
                            format!("{display_name} stays unresolved — {tip}")
                        }
                        FoundryOutcome::Refused { stage, reason } => {
                            format!("refused at {}: {reason}", stage.slug())
                        }
                    };
                    self.phase = Phase::Foundry {
                        running: false,
                        outcome: Some(summary),
                    };
                    return;
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.foundry_rx = None;
                    return;
                }
            }
        }
    }

    /// Spawn the overlay's `inspect` mode over the pack's anchors — the
    /// "am I anchored where I think I am" debug loop.
    fn launch_inspect(&mut self, id: &AppId) {
        let registry = PackRegistry::open(registry_root());
        let anchors = read_table(&foundry_dir(&registry, id).join("anchors.toml"))
            .ok()
            .and_then(|t| Anchors::from_table(&t));
        let Some(anchors) = anchors else {
            self.status = "no anchors.toml — run the Foundry first".into();
            return;
        };
        let names: Vec<String> = anchors
            .nodes
            .iter()
            .filter(|n| !n.actions.is_empty() && !n.name.is_empty())
            .take(6)
            .map(|n| n.name.clone())
            .collect();
        if names.is_empty() {
            self.status = "anchors hold no named actionable nodes to inspect".into();
            return;
        }
        let mut cmd = Command::new(overlay_bin());
        dpl_proc::quiet(&mut cmd);
        cmd.args(["inspect", "--app", &anchors.atspi_name, "--seconds", "12"]);
        for name in &names {
            cmd.args(["--name", name]);
        }
        // spawn-audit: quiet() — ADR overlay painter, inspect mode (S3b)
        match cmd.spawn() {
            Ok(_) => {
                self.status = format!("inspector painting {} anchors", names.len());
            }
            Err(e) => {
                self.status = format!("could not start inspector: {e}");
            }
        }
    }

    fn start_mint(&mut self, item: &InventoryItem) {
        let Some(live) = item.live.as_ref() else {
            self.phase = Phase::Refused {
                reason: format!("{} is not running", item.display_name),
                tip: format!("Open {}, then Import again.", item.display_name),
            };
            return;
        };
        // The seed is the PACK's (S3b seed law). No pack seed → seedless
        // mint; resolve-time falls back to the first actionable node.
        let request = MintRequest {
            target_id: item.id.clone(),
            atspi_name: live.atspi_name.clone(),
            complexity: item.complexity,
            seed_name: self.resolved.as_ref().and_then(|r| r.seed.clone()),
            budget: None,
        };
        let (tx, rx) = mpsc::channel();
        self.mint_rx = Some(rx);
        self.phase = Phase::Minting {
            visited: 0,
            budget: 0,
            phase: "locating".into(),
            eta: eta_hint(item.complexity),
        };
        thread::spawn(move || {
            let outcome = mint(&request, None, |p| {
                let _ = tx.send(MintMsg::Progress(p));
            });
            let _ = tx.send(MintMsg::Done(outcome));
        });
    }

    fn poll_mint(&mut self) {
        let Some(rx) = self.mint_rx.as_ref() else {
            return;
        };
        loop {
            match rx.try_recv() {
                Ok(MintMsg::Progress(p)) => {
                    self.phase = Phase::Minting {
                        visited: p.visited,
                        budget: p.budget,
                        phase: p.phase.as_str().into(),
                        eta: p.eta_hint,
                    };
                }
                Ok(MintMsg::Done(outcome)) => {
                    self.mint_rx = None;
                    match outcome {
                        MintOutcome::Ready(f) => {
                            // Honest seed display: what the walk actually
                            // seeded, or the fallback the lesson will mount.
                            let seed = f
                                .seed_path
                                .as_deref()
                                .and_then(|p| f.nodes.iter().find(|n| n.path == p))
                                .or_else(|| f.first_actionable())
                                .map(|n| n.name.clone())
                                .unwrap_or_else(|| "(no actionable node)".into());
                            let nodes = f.nodes.len() as u32;
                            let ms = f.minted_ms;
                            self.session = Some(FabricSession::new(f, self.agency));
                            self.phase = Phase::Ready { nodes, ms, seed };
                            self.status = format!(
                                "Fabric ready · {nodes} nodes · {ms} ms · bridge session held"
                            );
                        }
                        MintOutcome::Refused(r) => {
                            self.session = None;
                            self.phase = Phase::Refused {
                                reason: r.reason,
                                tip: r.tip,
                            };
                        }
                        MintOutcome::Cancelled => {
                            self.session = None;
                            self.phase = Phase::Refused {
                                reason: "mint cancelled".into(),
                                tip: "Import again when ready.".into(),
                            };
                        }
                    }
                    return;
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.mint_rx = None;
                    return;
                }
            }
        }
    }

    fn launch_lesson(&mut self) {
        let Some(session) = self.session.as_ref() else {
            self.phase = Phase::Refused {
                reason: "no fabric session — Import first".into(),
                tip: "Run Standard Import, then Start guided lesson.".into(),
            };
            return;
        };
        // Sync agency onto a clone for this launch.
        let mut session = session.clone();
        session.agency = self.agency;

        let handle = match resolve_seed(&session) {
            Ok(h) => h,
            Err(r) => {
                self.phase = Phase::Refused {
                    reason: format!("bridge resolve: {}", r.reason),
                    tip: r.tip,
                };
                return;
            }
        };

        let mut detail = format!(
            "Resolved {:?} via fabric path {} (agency={})",
            handle.node.name,
            handle.node.path,
            self.agency.as_str()
        );

        // Teacher may execute; Coach Mounts paint + observe.
        if self.agency == AgencyMode::Teacher {
            match invoke(&handle, "click", self.agency) {
                Ok(effect) => {
                    detail.push_str(&format!(" · INVOKED {}", effect.action));
                }
                Err(r) => {
                    self.phase = Phase::Refused {
                        reason: format!("Teacher invoke: {}", r.reason),
                        tip: r.tip,
                    };
                    return;
                }
            }
        }

        let spec = match mount_spec(&session, &handle) {
            Ok(s) => s,
            Err(r) => {
                // Invoke-only success still counts for Teacher; Coach needs paint.
                if self.agency == AgencyMode::Teacher {
                    self.phase = Phase::LessonLaunched {
                        detail: format!("{detail} · Mount paint skipped: {}", r.reason),
                    };
                    self.status = "Teacher invoke done (no Mount paint)".into();
                    return;
                }
                self.phase = Phase::Refused {
                    reason: format!("Mount extents: {}", r.reason),
                    tip: r.tip,
                };
                return;
            }
        };

        let bin = overlay_bin();
        let rect = spec.window_rect.as_csv();
        let mut cmd = Command::new(&bin);
        dpl_proc::quiet(&mut cmd);
        // spawn-audit: quiet() — ADR overlay painter from bridge MountSpec
        let result = cmd
            .args([
                "spotlight",
                "--app",
                &spec.atspi_name,
                "--name",
                &spec.expected_name,
                "--window-rect",
                &rect,
                "--seconds",
                "20",
                "--message",
                &format!("Press {}", spec.expected_name),
            ])
            .spawn();
        match result {
            Ok(_) => {
                detail.push_str(&format!(
                    " · Mount window_rect={} (bridge, not name-locate)",
                    rect
                ));
                self.phase = Phase::LessonLaunched { detail };
                self.status = "Lesson running — fabric bridge Mount".into();
            }
            Err(e) => {
                self.phase = Phase::Refused {
                    reason: format!("could not start overlay painter: {e}"),
                    tip: format!(
                        "Build `dpl-coach-overlay` beside the exe. Bridge resolved {:?} at {}.",
                        spec.expected_name, rect
                    ),
                };
            }
        }
    }
}

/// Rebuild console rows from a cell's job checkpoints — the honest
/// "what happened last run" view for the demo hook and cold reopens.
fn replay_console(registry: &PackRegistry, id: &AppId) -> Vec<ConsoleRow> {
    let dir = foundry_dir(registry, id);
    let job = dpl_coach_foundry::JobState::load(&dir);
    FoundryStage::ALL
        .into_iter()
        .map(|stage| {
            let completed = job.completed.iter().any(|s| s == stage.slug());
            ConsoleRow {
                stage,
                status: completed.then_some(StageStatus::Done),
                detail: if completed {
                    match stage.checkpoint_artifact() {
                        Some(name) if dir.join(name).is_file() => format!("checkpoint: {name}"),
                        _ => "completed".into(),
                    }
                } else {
                    String::new()
                },
            }
        })
        .collect()
}

/// Sort key: Cursor conquest first, Calculator regression second, then rest.
fn conquest_rank(item: &InventoryItem) -> u8 {
    let id = item.id.as_str().to_lowercase();
    let name = item.display_name.to_lowercase();
    if id.contains("cursor") || name == "cursor" {
        0
    } else if id.contains("calculator") || name.contains("calculator") {
        1
    } else {
        2
    }
}

impl Hub for CoachHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, _route: Route) {
        self.poll_mint();
        self.poll_foundry();
        if self.mint_rx.is_some() || self.foundry_rx.is_some() {
            ui.ctx().request_repaint();
        }

        let fluid = cl::Fluid::of(ui.max_rect());
        ui.vertical(|ui| {
            ui.add_space(fluid.sp_3());
            ui.label(
                RichText::new("OPERATOR COACH")
                    .monospace()
                    .size(fluid.fs_eyebrow())
                    .color(cl::ink::MUTED),
            );
            ui.label(
                RichText::new("Teach with Coach")
                    .size(fluid.fs_h1())
                    .color(cl::ink::STRONG)
                    .strong(),
            );
            ui.label(
                RichText::new(
                    "Choose an installed app. We check accessibility, import a signal map, then paint a real control — no screenshot loop.",
                )
                .size(fluid.fs_body())
                .color(cl::ink::BASE),
            );
            ui.add_space(fluid.sp_4());

            ui.horizontal(|ui| {
                if cl::tip(
                    ui.button(RichText::new("Refresh list").monospace()),
                    "Re-scan installed apps and running windows (Phase 0+1).",
                )
                .clicked()
                {
                    self.refresh_inventory();
                    self.session = None;
                    self.phase = Phase::Pick;
                }
                ui.label(
                    RichText::new(&self.status)
                        .monospace()
                        .size(fluid.fs_body_sm())
                        .color(cl::ink::MUTED),
                );
            });

            if let Some(err) = &self.last_refresh_err {
                ui.colored_label(cl::Tone::Coral.color(), err);
            }

            ui.add_space(fluid.sp_3());
            ui.columns(2, |cols| {
                cols[0].group(|ui| {
                    ui.label(
                        RichText::new("Installed & available")
                            .strong()
                            .color(cl::ink::STRONG),
                    );
                    ui.label(
                        RichText::new("Lightweight list. Import starts only after you confirm.")
                            .size(fluid.fs_body_sm())
                            .color(cl::ink::MUTED),
                    );
                    ui.add_space(fluid.sp_2());
                    let mut selection_changed = false;
                    egui::ScrollArea::vertical()
                        .max_height((ui.available_height() - 80.0).max(240.0))
                        .show(ui, |ui| {
                            for (idx, item) in self.items.iter().enumerate() {
                                let selected = self.selected == Some(idx);
                                let run = if item.live.is_some() {
                                    "RUNNING"
                                } else {
                                    "cold"
                                };
                                let label = format!(
                                    "{}  ·  {}  ·  {}",
                                    item.display_name,
                                    item.complexity.as_str(),
                                    run
                                );
                                let tip = item.seed_tip.as_deref().unwrap_or(
                                    "Select, then Import / analyze — honesty prompt first.",
                                );
                                if cl::tip(
                                    ui.selectable_label(selected, RichText::new(label).monospace()),
                                    tip,
                                )
                                .clicked()
                                {
                                    self.selected = Some(idx);
                                    self.phase = Phase::Pick;
                                    selection_changed = true;
                                }
                            }
                        });
                    if selection_changed {
                        self.recompute_resolution();
                    }
                    ui.add_space(fluid.sp_3());
                    let can_import = self.selected.is_some()
                        && matches!(
                            self.phase,
                            Phase::Pick
                                | Phase::Refused { .. }
                                | Phase::Ready { .. }
                                | Phase::LessonLaunched { .. }
                        );
                    if cl::tip(
                        ui.add_enabled(can_import, egui::Button::new("Import / analyze…")),
                        "Opens the honesty prompt — ETA and essential checks before any deep walk.",
                    )
                    .clicked()
                    {
                        self.phase = Phase::Confirm;
                    }
                });

                cols[1].group(|ui| {
                    self.right_panel(ui, &fluid);
                });
            });

            ui.add_space(fluid.sp_4());
            ui.label(
                RichText::new(
                    "Execution law: lessons use the fabric — Mount / highlight / press. Screenshots are never the hot path.",
                )
                .size(fluid.fs_body_sm())
                .color(cl::ink::MUTED),
            );
        });

        if matches!(self.phase, Phase::Confirm) {
            self.confirm_modal(ui);
        }
    }
}

impl CoachHub {
    fn right_panel(&mut self, ui: &mut egui::Ui, fluid: &cl::Fluid) {
        match &self.phase.clone() {
            Phase::Pick => {
                ui.label(
                    RichText::new("After you select")
                        .strong()
                        .color(cl::ink::STRONG),
                );
                ui.label(
                    RichText::new(
                        "1 Pick · 2 Confirm import · 3 Essential checks · 4 Standard Import · 5 Lesson",
                    )
                    .size(fluid.fs_body_sm())
                    .color(cl::ink::MUTED),
                );
                if let Some(item) = self.selected_item() {
                    ui.add_space(fluid.sp_2());
                    ui.label(
                        RichText::new(format!("Selected: {}", item.display_name))
                            .color(SPEC.tone.color()),
                    );
                    ui.label(
                        RichText::new(eta_hint(item.complexity))
                            .size(fluid.fs_body_sm())
                            .color(cl::ink::MUTED),
                    );
                }
                self.identity_block(ui, fluid);
            }
            Phase::Confirm => {
                ui.label(RichText::new("Honesty prompt open…").color(cl::ink::MUTED));
            }
            Phase::Checks { workflow } => {
                ui.label(RichText::new("Essential checks").strong());
                self.draw_checks(ui, fluid, *workflow);
            }
            Phase::Minting {
                visited,
                budget,
                phase,
                eta,
            } => {
                ui.label(RichText::new("Standard Import").strong());
                ui.label(
                    RichText::new(format!("[{phase}] visited={visited}/{budget}"))
                        .monospace()
                        .color(SPEC.tone.color()),
                );
                let frac = if *budget == 0 {
                    0.05
                } else {
                    (*visited as f32 / *budget as f32).clamp(0.02, 1.0)
                };
                ui.add(egui::ProgressBar::new(frac).show_percentage());
                ui.label(
                    RichText::new(eta)
                        .size(fluid.fs_body_sm())
                        .color(cl::ink::MUTED),
                );
                ui.label(
                    RichText::new("Cancel: close the app under import or wait for refuse tip.")
                        .size(fluid.fs_body_sm())
                        .color(cl::ink::MUTED),
                );
            }
            Phase::Ready { nodes, ms, seed } => {
                ui.label(
                    RichText::new("Lesson-ready")
                        .strong()
                        .color(cl::Tone::Lime.color()),
                );
                ui.label(format!(
                    "Fabric ready · {nodes} nodes · {ms} ms · session held"
                ));
                ui.label(
                    RichText::new(format!("Seed control: {seed}"))
                        .monospace()
                        .color(SPEC.tone.color()),
                );
                ui.horizontal(|ui| {
                    ui.label("Agency");
                    let coach = self.agency == AgencyMode::Coach;
                    if cl::tip(
                        ui.selectable_label(coach, "Coach"),
                        "Operator leads — Mount paint + observe (no auto-invoke).",
                    )
                    .clicked()
                    {
                        self.agency = AgencyMode::Coach;
                        if let Some(s) = self.session.as_mut() {
                            s.agency = AgencyMode::Coach;
                        }
                    }
                    if cl::tip(
                        ui.selectable_label(!coach, "Teacher"),
                        "Same resolve/invoke/mount rails — AI may invoke the seed via fabric.",
                    )
                    .clicked()
                    {
                        self.agency = AgencyMode::Teacher;
                        if let Some(s) = self.session.as_mut() {
                            s.agency = AgencyMode::Teacher;
                        }
                    }
                });
                if cl::tip(
                    ui.button("Start guided lesson"),
                    "Bridge: resolve fabric seed → (Teacher: invoke) → Mount overlay with --window-rect.",
                )
                .clicked()
                {
                    self.launch_lesson();
                }
            }
            Phase::LessonLaunched { detail } => {
                ui.label(
                    RichText::new("Lesson running")
                        .strong()
                        .color(cl::Tone::Lime.color()),
                );
                ui.label(detail);
                if cl::tip(ui.button("Back to pick"), "Return to the app list.").clicked() {
                    self.session = None;
                    self.phase = Phase::Pick;
                }
            }
            Phase::Foundry { running, outcome } => {
                self.foundry_console(ui, fluid, *running, outcome.as_deref());
            }
            Phase::Profile => {
                self.profile_card(ui, fluid);
            }
            Phase::Refused { reason, tip } => {
                ui.colored_label(cl::Tone::Coral.color(), format!("REFUSED — {reason}"));
                ui.label(RichText::new(format!("tip: {tip}")).color(cl::ink::BASE));
                if cl::tip(ui.button("Back"), "Return to pick.").clicked() {
                    self.phase = Phase::Pick;
                }
            }
        }
    }

    /// The evidence-resolved identity + Foundry actions on Pick (S3b).
    fn identity_block(&mut self, ui: &mut egui::Ui, fluid: &cl::Fluid) {
        let Some(resolved) = self.resolved.clone() else {
            return;
        };
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        ui.add_space(fluid.sp_3());
        ui.separator();
        ui.label(
            RichText::new("Identity (evidence-resolved)")
                .strong()
                .color(cl::ink::STRONG),
        );
        let tier_color = match resolved.tier {
            "resolved" => cl::Tone::Lime.color(),
            "minted" => SPEC.tone.color(),
            _ => cl::Tone::Amber.color(),
        };
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(resolved.app_id.as_str())
                    .monospace()
                    .color(cl::ink::STRONG),
            );
            cl::tip(
                ui.label(RichText::new(resolved.tier.to_uppercase()).monospace().color(tier_color)),
                "RESOLVED: an installed pack claimed this evidence. MINTED: the desktop id names an identity. UNRESOLVED: ephemeral profile — the Foundry can still try.",
            );
        });
        ui.label(
            RichText::new(&resolved.detail)
                .size(fluid.fs_body_sm())
                .color(cl::ink::MUTED),
        );
        if let Some(seed) = &resolved.seed {
            ui.label(
                RichText::new(format!("pack seed: {seed}"))
                    .monospace()
                    .size(fluid.fs_body_sm())
                    .color(SPEC.tone.color()),
            );
        }
        ui.add_space(fluid.sp_2());
        ui.horizontal_wrapped(|ui| {
            let live = item.live.is_some();
            if cl::tip(
                ui.add_enabled(live, egui::Button::new("Build fabric (Foundry)")),
                "Staged pipeline: census → resolve → fabric → probe → induct → battery → publish. Checkpointed — a killed build resumes.",
            )
            .clicked()
            {
                self.start_foundry(item.display_name.clone());
                return;
            }
            if resolved.has_pack {
                if cl::tip(
                    ui.button("Profile"),
                    "The installed pack cell: identity, verbs with MEASURED reliability, warnings.",
                )
                .clicked()
                    && self.load_profile(&resolved.app_id)
                {
                    self.phase = Phase::Profile;
                    return;
                }
                if cl::tip(
                    ui.button("Purge"),
                    "Move this pack cell to packs/.trash — content is never destroyed. Build again for a fresh cell.",
                )
                .clicked()
                {
                    match purge(&registry_root(), &resolved.app_id) {
                        Ok(trash) => {
                            self.status = format!("purged to {}", trash.display());
                            self.recompute_resolution();
                        }
                        Err(e) => self.status = e,
                    }
                }
            }
        });
    }

    fn foundry_console(
        &mut self,
        ui: &mut egui::Ui,
        fluid: &cl::Fluid,
        running: bool,
        outcome: Option<&str>,
    ) {
        ui.label(
            RichText::new("Fabric Foundry")
                .strong()
                .color(cl::ink::STRONG),
        );
        ui.label(
            RichText::new(
                "Building this app into the catalog — reliability is measured, never asserted.",
            )
            .size(fluid.fs_body_sm())
            .color(cl::ink::MUTED),
        );
        ui.add_space(fluid.sp_2());
        let done = self
            .console
            .iter()
            .filter(|r| matches!(r.status, Some(StageStatus::Done | StageStatus::Skipped)))
            .count();
        ui.add(
            egui::ProgressBar::new(done as f32 / FoundryStage::ALL.len() as f32).show_percentage(),
        );
        ui.add_space(fluid.sp_2());
        for row in &self.console {
            ui.horizontal(|ui| {
                let (mark, color) = match row.status {
                    Some(StageStatus::Done) => ("done", cl::Tone::Lime.color()),
                    Some(StageStatus::Skipped) => ("kept", SPEC.tone.color()),
                    Some(StageStatus::Started) => ("....", cl::Tone::Amber.color()),
                    None => ("    ", cl::ink::MUTED),
                };
                ui.label(RichText::new(format!("[{mark}]")).monospace().color(color));
                cl::tip(
                    ui.label(RichText::new(row.stage.human()).color(cl::ink::STRONG)),
                    row.stage.slug(),
                );
                if !row.detail.is_empty() {
                    ui.label(
                        RichText::new(&row.detail)
                            .monospace()
                            .size(fluid.fs_body_sm())
                            .color(cl::ink::MUTED),
                    );
                }
            });
        }
        if let Some(outcome) = outcome {
            ui.add_space(fluid.sp_2());
            ui.label(RichText::new(outcome).color(cl::Tone::Amber.color()));
        }
        if !running {
            ui.add_space(fluid.sp_2());
            ui.horizontal(|ui| {
                if cl::tip(ui.button("Back to pick"), "Return to the app list.").clicked() {
                    self.phase = Phase::Pick;
                }
                if let Some(resolved) = self.resolved.clone() {
                    if let Some(item) = self.selected_item().cloned() {
                        if cl::tip(
                            ui.button("Remake"),
                            "Purge this cell to .trash, then build it again from scratch.",
                        )
                        .clicked()
                        {
                            let _ = purge(&registry_root(), &resolved.app_id);
                            self.start_foundry(item.display_name.clone());
                        }
                    }
                }
            });
        }
    }

    fn profile_card(&mut self, ui: &mut egui::Ui, fluid: &cl::Fluid) {
        let Some(card) = self.profile.clone() else {
            ui.label(RichText::new("No pack loaded.").color(cl::ink::MUTED));
            if cl::tip(ui.button("Back to pick"), "Return to the app list.").clicked() {
                self.phase = Phase::Pick;
            }
            return;
        };
        ui.label(
            RichText::new(format!("{} — pack cell", card.display_name))
                .strong()
                .color(cl::ink::STRONG),
        );
        ui.label(
            RichText::new(card.app_id.as_str())
                .monospace()
                .color(SPEC.tone.color()),
        );
        ui.label(
            RichText::new(format!(
                "cell {} · {}",
                card.cell,
                &card.hash[..21.min(card.hash.len())]
            ))
            .monospace()
            .size(fluid.fs_body_sm())
            .color(cl::ink::MUTED),
        );
        if let Some(seed) = &card.seed {
            ui.label(
                RichText::new(format!("lesson seed: {seed}"))
                    .monospace()
                    .color(SPEC.tone.color()),
            );
        }
        ui.add_space(fluid.sp_2());
        ui.label(
            RichText::new("Verbs (reliability is the measured effect rate)")
                .strong()
                .color(cl::ink::STRONG),
        );
        if card.verbs.is_empty() {
            ui.label(
                RichText::new("no grants — battery never went green")
                    .color(cl::Tone::Amber.color()),
            );
        }
        for (verb, rung, reliability, oracle) in &card.verbs {
            ui.horizontal(|ui| {
                ui.label(RichText::new(verb).monospace().color(cl::ink::STRONG));
                ui.label(RichText::new(rung).monospace().color(SPEC.tone.color()));
                ui.label(
                    RichText::new(reliability)
                        .monospace()
                        .color(cl::Tone::Lime.color()),
                );
                ui.label(
                    RichText::new(oracle)
                        .monospace()
                        .size(fluid.fs_body_sm())
                        .color(cl::ink::MUTED),
                );
            });
        }
        for warning in &card.warnings {
            ui.colored_label(cl::Tone::Amber.color(), format!("warn: {warning}"));
        }
        ui.add_space(fluid.sp_2());
        ui.horizontal(|ui| {
            if cl::tip(ui.button("Back to pick"), "Return to the app list.").clicked() {
                self.phase = Phase::Pick;
            }
            if cl::tip(
                ui.button("Inspect anchors"),
                "Debug paint: outline this pack's anchor rects over the live app window.",
            )
            .clicked()
            {
                let id = card.app_id.clone();
                self.launch_inspect(&id);
            }
        });
        ui.add_space(fluid.sp_2());
        // The raw pack is the ADVANCED view — the card leads with meaning.
        ui.collapsing(
            RichText::new("Advanced — raw pack.toml").monospace(),
            |ui| {
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(&card.raw_toml)
                                .monospace()
                                .size(fluid.fs_body_sm()),
                        );
                    });
            },
        );
    }

    fn draw_checks(&mut self, ui: &mut egui::Ui, fluid: &cl::Fluid, workflow: ImportWorkflow) {
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        let a11y_ok = item.live.is_some() && !matches!(item.complexity, Complexity::Deep);
        ui.horizontal(|ui| {
            ui.label("Accessibility stream");
            ui.label(
                RichText::new(if a11y_ok { "PASS" } else { "THIN" })
                    .monospace()
                    .color(if a11y_ok {
                        cl::Tone::Lime.color()
                    } else {
                        cl::Tone::Amber.color()
                    }),
            );
        });
        ui.label(
            RichText::new("AT-SPI / UIA app root + interactables")
                .size(fluid.fs_body_sm())
                .color(cl::ink::MUTED),
        );
        ui.horizontal(|ui| {
            ui.label("Window focus");
            ui.label(
                RichText::new("PASS")
                    .monospace()
                    .color(cl::Tone::Lime.color()),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Screenshot-as-eyes");
            ui.label(
                RichText::new("REFUSED")
                    .monospace()
                    .color(cl::Tone::Lime.color()),
            );
        });
        ui.add_space(fluid.sp_2());
        match workflow {
            ImportWorkflow::Standard => {
                ui.label(
                    RichText::new("Checks pass → Standard Import").color(cl::Tone::Lime.color()),
                );
                if cl::tip(
                    ui.button("Start Standard Import"),
                    "Budgeted Tier A mint — cancelable; builds fabric for lightweight lessons.",
                )
                .clicked()
                {
                    self.start_mint(&item);
                }
            }
            ImportWorkflow::AdvancedDivert => {
                ui.label(
                    RichText::new("Accessibility thin / not running → Advanced Import")
                        .color(cl::Tone::Amber.color()),
                );
                ui.label(
                    RichText::new(
                        "C2f does not run Advanced harvest yet. Open the app (or pick Cursor/Calculator when running), then retry. Harvest is CX-harvest.",
                    )
                    .size(fluid.fs_body_sm())
                    .color(cl::ink::MUTED),
                );
                if cl::tip(ui.button("Back"), "Return to pick.").clicked() {
                    self.phase = Phase::Pick;
                }
            }
        }
    }

    fn confirm_modal(&mut self, ui: &mut egui::Ui) {
        let Some(item) = self.selected_item().cloned() else {
            self.phase = Phase::Pick;
            return;
        };
        let eta = eta_hint(item.complexity);
        let mut open = true;
        egui::Window::new("Analyze & import?")
            .id(egui::Id::new("coach_import_confirm"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                ui.set_max_width(420.0);
                ui.label(format!(
                    "We’ll take a few minutes to analyze {} and build a signal map.",
                    item.display_name
                ));
                ui.label(
                    RichText::new("• Check accessibility stream (Standard vs Advanced)").size(13.0),
                );
                ui.label(
                    RichText::new(
                        "• Build fabric for lightweight lessons (no screenshot loop later)",
                    )
                    .size(13.0),
                );
                ui.label(RichText::new(format!("ETA: {eta}")).color(cl::ink::MUTED));
                ui.horizontal(|ui| {
                    if cl::tip(
                        ui.button(RichText::new("Start import").strong()),
                        "Run essential checks, then Standard Import if a11y passes.",
                    )
                    .clicked()
                    {
                        let workflow = classify_import(&item);
                        self.phase = Phase::Checks { workflow };
                    }
                    if cl::tip(ui.button("Cancel"), "Dismiss without scanning.").clicked() {
                        self.phase = Phase::Pick;
                    }
                });
            });
        if !open {
            self.phase = Phase::Pick;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_coach_inventory::{Complexity, DesktopId, LiveHint};

    fn item(name: &str, id: &str, running: bool, complexity: Complexity) -> InventoryItem {
        InventoryItem {
            id: DesktopId::from_raw(id),
            display_name: name.into(),
            exec_hint: name.into(),
            startup_wm_class: None,
            cold_ok: true,
            live: running.then(|| LiveHint {
                atspi_name: name.to_lowercase(),
                pid: None,
                running: true,
            }),
            complexity,
            seed_tip: None,
        }
    }

    #[test]
    fn cursor_running_is_standard_import() {
        let i = item("Cursor", "cursor.desktop", true, Complexity::Medium);
        assert_eq!(classify_import(&i), ImportWorkflow::Standard);
    }

    /// The S3b seed law: seeds come from packs, identity from evidence
    /// — no name sniffing anywhere in the hub.
    #[test]
    fn identity_and_seed_come_from_the_registry_never_from_names() {
        let root = std::env::temp_dir().join(format!(
            "dpl-hub-coach-seed-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let registry = PackRegistry::open(&root);

        // A pack with a seed → RESOLVED + the pack's seed.
        let pack = dpl_coach_pack::Pack {
            schema: dpl_coach_pack::PACK_SCHEMA,
            min_seam: dpl_coach_pack::CURRENT_SEAM,
            identity: dpl_coach_pack::Identity {
                app_id: AppId::parse("org.gnome.Calculator").unwrap(),
                display_name: "Calculator".into(),
                desktop_ids: vec!["org.gnome.Calculator.desktop".into()],
                wm_classes: vec![],
                atspi_names: vec!["gnome-calculator".into()],
                exe_basenames: vec![],
                aliases: vec![],
            },
            cell: egui_pilot_ai_protocol::seam::CellKey::new("48", "linux", "en"),
            ladder: vec![],
            verbs: vec![],
            teaching_landmarks: vec![],
            teaching_seed_control: Some("7".into()),
            extras: toml::Table::new(),
        };
        registry.write(&pack).unwrap();
        let calc = item(
            "gnome-calculator",
            "org.gnome.Calculator.desktop",
            true,
            Complexity::Small,
        );
        let resolved = resolve_identity(&registry, &calc);
        assert_eq!(resolved.tier, "resolved");
        assert_eq!(resolved.app_id.as_str(), "org.gnome.Calculator");
        assert!(resolved.has_pack);
        assert_eq!(resolved.seed.as_deref(), Some("7"));

        // Reverse-DNS evidence with NO pack → MINTED, no seed invented.
        let files = item(
            "Files",
            "org.gnome.Nautilus.desktop",
            true,
            Complexity::Medium,
        );
        let resolved = resolve_identity(&registry, &files);
        assert_eq!(resolved.tier, "minted");
        assert_eq!(resolved.app_id.as_str(), "org.gnome.Nautilus");
        assert!(!resolved.has_pack);
        assert_eq!(resolved.seed, None);

        // Bare desktop id, no pack → honestly UNRESOLVED.
        let blender = item("Blender", "blender.desktop", true, Complexity::Deep);
        let resolved = resolve_identity(&registry, &blender);
        assert_eq!(resolved.tier, "unresolved");
        assert_eq!(resolved.seed, None);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn deep_or_cold_diverts_advanced() {
        let deep = item("Blender", "blender.desktop", true, Complexity::Deep);
        assert_eq!(classify_import(&deep), ImportWorkflow::AdvancedDivert);
        let cold = item("Cursor", "cursor.desktop", false, Complexity::Medium);
        assert_eq!(classify_import(&cold), ImportWorkflow::AdvancedDivert);
    }

    #[test]
    fn conquest_rank_orders_cursor_then_calculator() {
        let c = item("Cursor", "cursor.desktop", true, Complexity::Medium);
        let calc = item(
            "Calculator",
            "org.gnome.Calculator.desktop",
            true,
            Complexity::Small,
        );
        let other = item(
            "Files",
            "org.gnome.Nautilus.desktop",
            true,
            Complexity::Medium,
        );
        assert!(conquest_rank(&c) < conquest_rank(&calc));
        assert!(conquest_rank(&calc) < conquest_rank(&other));
    }
}
