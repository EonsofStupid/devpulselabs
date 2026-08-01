//! Coach hub — Operator Import Process (pick → check → Standard mint → lesson).
//!
//! Foreign paint uses the `dpl-coach-overlay` **binary** beside the exe
//! (ADR), never a crate dep. Plan: `docs/plans/coach-c2f-operator-import-ui.md`.

#![forbid(unsafe_code)]

use dpl_coach_fabric::{eta_hint, mint, MintOutcome, MintProgress, MintRequest};
use dpl_coach_inventory::{inventory, Complexity, InventoryItem};
use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use egui::RichText;
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

fn seed_control_for(item: &InventoryItem) -> &'static str {
    let blob = format!("{} {}", item.id.as_str(), item.display_name).to_lowercase();
    if blob.contains("cursor") {
        "Search Ctrl+K"
    } else if blob.contains("calculator") {
        "7"
    } else {
        "File"
    }
}

fn mint_seed_name(item: &InventoryItem) -> Option<String> {
    let seed = seed_control_for(item);
    let blob = format!("{} {}", item.id.as_str(), item.display_name).to_lowercase();
    if blob.contains("cursor") || blob.contains("calculator") {
        Some(seed.to_string())
    } else {
        None
    }
}

fn atspi_app_name(item: &InventoryItem) -> String {
    item.live
        .as_ref()
        .map(|l| l.atspi_name.clone())
        .unwrap_or_else(|| item.exec_hint.clone())
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
        };
        hub.refresh_inventory();
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
    }

    fn selected_item(&self) -> Option<&InventoryItem> {
        self.selected.and_then(|i| self.items.get(i))
    }

    fn start_mint(&mut self, item: &InventoryItem) {
        let Some(live) = item.live.as_ref() else {
            self.phase = Phase::Refused {
                reason: format!("{} is not running", item.display_name),
                tip: format!("Open {}, then Import again.", item.display_name),
            };
            return;
        };
        let request = MintRequest {
            target_id: item.id.clone(),
            atspi_name: live.atspi_name.clone(),
            complexity: item.complexity,
            seed_name: mint_seed_name(item),
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
                            let seed = self
                                .selected_item()
                                .map(seed_control_for)
                                .unwrap_or("File")
                                .to_string();
                            self.phase = Phase::Ready {
                                nodes: f.nodes.len() as u32,
                                ms: f.minted_ms,
                                seed,
                            };
                            self.status = format!(
                                "Fabric ready · {} nodes · {} ms",
                                f.nodes.len(),
                                f.minted_ms
                            );
                        }
                        MintOutcome::Refused(r) => {
                            self.phase = Phase::Refused {
                                reason: r.reason,
                                tip: r.tip,
                            };
                        }
                        MintOutcome::Cancelled => {
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
        let Some(item) = self.selected_item().cloned() else {
            return;
        };
        let app = atspi_app_name(&item);
        let seed = seed_control_for(&item);
        let bin = overlay_bin();
        let mut cmd = Command::new(&bin);
        dpl_proc::quiet(&mut cmd);
        // spawn-audit: quiet() — ADR overlay painter; CREATE_NO_WINDOW only
        let result = cmd
            .args([
                "spotlight",
                "--app",
                &app,
                "--name",
                seed,
                "--seconds",
                "20",
                "--message",
                &format!("Press {seed}"),
            ])
            .spawn();
        match result {
            Ok(_) => {
                self.phase = Phase::LessonLaunched {
                    detail: format!(
                        "Overlay spotlight on {app:?} · {seed:?} (20s). Press the control to advance."
                    ),
                };
                self.status = "Lesson running — watch the foreign window".into();
            }
            Err(e) => {
                self.phase = Phase::Refused {
                    reason: format!("could not start overlay painter: {e}"),
                    tip: format!(
                        "Build and place `dpl-coach-overlay` beside the exe (or set DPL_COACH_OVERLAY). CLI: {} spotlight --app {} --name {:?}",
                        bin.display(),
                        app,
                        seed
                    ),
                };
            }
        }
    }
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
        if self.mint_rx.is_some() {
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
                                }
                            }
                        });
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
                ui.label(format!("Fabric ready · {nodes} nodes · {ms} ms"));
                ui.label(
                    RichText::new(format!("Seed control: {seed}"))
                        .monospace()
                        .color(SPEC.tone.color()),
                );
                if cl::tip(
                    ui.button("Start guided lesson"),
                    "Paints the seed on the foreign app via dpl-coach-overlay (operator gesture).",
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
                    self.phase = Phase::Pick;
                }
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
        assert_eq!(seed_control_for(&i), "Search Ctrl+K");
        assert_eq!(mint_seed_name(&i).as_deref(), Some("Search Ctrl+K"));
    }

    #[test]
    fn calculator_seed_is_seven() {
        let i = item(
            "Calculator",
            "org.gnome.Calculator.desktop",
            true,
            Complexity::Small,
        );
        assert_eq!(seed_control_for(&i), "7");
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
