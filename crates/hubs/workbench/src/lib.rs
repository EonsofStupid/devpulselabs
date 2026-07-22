//! Workbench — the internal slide + full-episode builder: blocks, elements,
//! templates, then effects (the operator's stated stage order).
//!
//! SEEDS leads (the hub's default route): the default styling slides — real
//! `dpl_doc` documents from `seeds/slides/`, rendered LIVE through the one
//! document renderer. The editor (W4) opens exactly these.

use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use dpl_slides::document::DocSlide;
use egui::RichText;
use std::path::PathBuf;

pub const HUB: HubId = HubId("workbench");
pub const SEEDS: SubId = SubId("seeds");
pub const EDITOR: SubId = SubId("editor");
pub const STUDIO: SubId = SubId("studio");
pub const LIBRARY: SubId = SubId("library");
pub const TOKENS: SubId = SubId("tokens");
pub const TEST_LAB: SubId = SubId("test-lab");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Workbench",
    icon: cl::icon::WORKBENCH,
    tone: Tone::Amber,
    subroutes: &[
        SubRoute {
            id: SEEDS,
            label: "Seed slides",
            blurb: "The default styling slides — documents from seeds/slides, rendered live.",
        },
        SubRoute {
            id: EDITOR,
            label: "Editor",
            blurb: "Tap a slide to change it — font, colours, motion; edits write the colocated file.",
        },
        SubRoute {
            id: STUDIO,
            label: "Studio",
            blurb: "Compose episodes: blocks → elements → templates → effects, onto sequenced slides.",
        },
        SubRoute {
            id: LIBRARY,
            label: "Library",
            blurb: "Every registered building block, rendered live with fixture props.",
        },
        SubRoute {
            id: TOKENS,
            label: "Tokens",
            blurb: "ColdLight's tones, spacing, and paint vocabulary — the authored source, visualized.",
        },
        SubRoute {
            id: TEST_LAB,
            label: "Test Lab",
            blurb: "Agent snapshots, full semantics, isolated runs, traces, and visual approvals.",
        },
    ],
};

/// One loaded seed: its id, the colocated folder, the live document (or the
/// load error, honestly), and the validator's findings.
struct SeedEntry {
    id: String,
    dir: PathBuf,
    doc: Result<DocSlide, String>,
    issues: Vec<dpl_doc::Issue>,
}

#[derive(Default)]
pub struct WorkbenchHub {
    /// Loaded on first visit; the reload chip clears it (edits to the
    /// colocated files show up without restarting the app).
    seeds: Option<Vec<SeedEntry>>,
    /// The open editor (EDITOR subroute) — None shows the seed picker.
    editor: Option<dpl_editor::Editor>,
    test_lab: egui_pilot_ai_lab::TestLab,
}

impl WorkbenchHub {
    fn load_seeds() -> Vec<SeedEntry> {
        dpl_doc::list_seeds()
            .into_iter()
            .map(|(id, dir)| match dpl_doc::load(&dir) {
                Ok(slide) => {
                    let issues = dpl_doc::validate(&slide);
                    SeedEntry {
                        id,
                        dir,
                        doc: Ok(DocSlide::new(slide)),
                        issues,
                    }
                }
                Err(e) => SeedEntry {
                    id,
                    dir,
                    doc: Err(e.to_string()),
                    issues: Vec::new(),
                },
            })
            .collect()
    }

    fn seeds_wall(&mut self, ui: &mut egui::Ui) {
        let seeds = self.seeds.get_or_insert_with(Self::load_seeds);

        let mut reload = false;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "SEED SLIDES — {} documents · each is its own folder; the editor edits the colocated slide.json",
                    seeds.len()
                ))
                .small()
                .color(cl::ink::SUBTLE),
            );
            let chip = ui
                .label(RichText::new("reload").monospace().small().color(SPEC.tone.color()))
                .interact(egui::Sense::click());
            if chip.clicked() {
                reload = true;
            }
        });
        ui.add_space(cl::space::S2);

        if seeds.is_empty() {
            ui.label(
                RichText::new("no seeds found — expected seeds/slides/<id>/slide.json")
                    .color(cl::ink::MUTED),
            );
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let avail_w = ui.available_width();
                let stage_w = avail_w - cl::space::S4 * 2.0;
                let stage_h = stage_w * 9.0 / 16.0;
                for entry in seeds.iter_mut() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&entry.id).strong().color(cl::ink::STRONG));
                        ui.label(
                            RichText::new(entry.dir.join("slide.json").to_string_lossy())
                                .monospace()
                                .small()
                                .color(cl::ink::SUBTLE),
                        );
                    });
                    for issue in &entry.issues {
                        ui.label(
                            RichText::new(format!("⚠ {} — {}", issue.path, issue.message))
                                .small()
                                .color(SPEC.tone.color()),
                        );
                    }
                    let (stage, _) =
                        ui.allocate_exact_size(egui::vec2(stage_w, stage_h), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(stage, cl::RADIUS, cl::surface::WELL);
                    match &mut entry.doc {
                        Ok(doc) => doc.ui(ui, stage.shrink(2.0)),
                        Err(e) => {
                            ui.painter().text(
                                stage.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("failed to load: {e}"),
                                egui::FontId::monospace(12.0),
                                cl::ink::SUBTLE,
                            );
                        }
                    }
                    ui.painter().rect_stroke(
                        stage,
                        cl::RADIUS,
                        egui::Stroke::new(1.0, cl::surface::HAIRLINE),
                        egui::StrokeKind::Inside,
                    );
                    ui.add_space(cl::space::S5);
                }
            });

        if reload {
            self.seeds = None;
        }
    }
}

impl Hub for WorkbenchHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        if route.sub == SEEDS {
            self.seeds_wall(ui);
        } else if route.sub == EDITOR {
            self.editor_surface(ui);
        } else if route.sub == TEST_LAB {
            self.test_lab.ui(ui);
        } else {
            dpl_hub::placeholder::surface(
                ui,
                &SPEC,
                route,
                "The episode factory. Everything captured or generated ends up on a slide here.",
            );
        }
    }
}

impl WorkbenchHub {
    /// The EDITOR subroute: a seed picker until one is opened, then the
    /// tap-to-edit editor. A back chip returns to the picker.
    fn editor_surface(&mut self, ui: &mut egui::Ui) {
        if self.editor.is_some() {
            let back = ui
                .horizontal(|ui| {
                    ui.label(RichText::new("‹ pick another slide").color(SPEC.tone.color()))
                        .interact(egui::Sense::click())
                })
                .inner;
            if back.clicked() {
                self.editor = None;
                return;
            }
            if let Some(ed) = self.editor.as_mut() {
                ed.ui(ui);
            }
            return;
        }

        ui.label(
            RichText::new("Pick a slide to edit — tap it, then tap what you want to change")
                .small()
                .color(cl::ink::SUBTLE),
        );
        ui.add_space(cl::space::S2);
        let mut open: Option<PathBuf> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (id, dir) in dpl_doc::list_seeds() {
                    let resp = cl::card_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width().min(560.0));
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&id).strong().color(cl::ink::STRONG));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new("edit ›").small().color(SPEC.tone.color()),
                                    );
                                },
                            );
                        });
                    });
                    if resp.response.interact(egui::Sense::click()).clicked() {
                        open = Some(dir);
                    }
                    ui.add_space(cl::space::S2);
                }
            });
        if let Some(dir) = open {
            match dpl_editor::Editor::open(&dir) {
                Ok(ed) => self.editor = Some(ed),
                Err(e) => {
                    ui.label(RichText::new(format!("could not open: {e}")).color(cl::ink::MUTED));
                }
            }
        }
    }
}
