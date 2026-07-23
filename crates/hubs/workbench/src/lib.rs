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
pub const EPISODES: SubId = SubId("episodes");
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
            id: EPISODES,
            label: "Episodes",
            blurb: "The shows — an ordered chapter→slide sequence over slide folders, rendered live.",
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
    /// The episode-wall's live slide docs, loaded once per episode open.
    episode_view: Option<EpisodeView>,
    /// The "copy to my shows" form (X3) — open on the episodes picker.
    clone_form: Option<CloneForm>,
    test_lab: egui_pilot_ai_lab::TestLab,
}

/// The clone-a-seed form: names a show category + episode (kebab law),
/// runs `dpl_doc::clone_episode`. Seeds are read, never written.
struct CloneForm {
    seed_id: String,
    seed_dir: PathBuf,
    show: String,
    episode: String,
    error: Option<String>,
}

/// The operator's ratified stream categories (2026-07-23) — offered as
/// suggestions while the content tree is empty; any folder name works.
const SHOW_SUGGESTIONS: [&str; 3] = ["devpulse-news", "eons-of-stupidity", "effingai"];

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
        } else if route.sub == EPISODES {
            self.episodes_surface(ui);
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

/// An opened episode's live slides, in the authored chapter→slide order.
struct EpisodeView {
    id: String,
    title: String,
    /// (chapter title on the first slide of the chapter, slide name, doc).
    rows: Vec<(Option<String>, String, DocSlide)>,
    issues: Vec<dpl_doc::Issue>,
}

impl EpisodeView {
    fn open(dir: &std::path::Path, id: String) -> Result<Self, String> {
        let ep = dpl_doc::load_episode(dir).map_err(|e| e.to_string())?;
        let issues = dpl_doc::validate_episode(dir, &ep);
        // The show's authored look (W3b) — every slide resolves through it.
        let look = dpl_doc::load_look(dir)
            .ok()
            .flatten()
            .unwrap_or_else(dpl_doc::Look::fallback);
        let mut rows = Vec::new();
        for chapter in &ep.chapters {
            for (i, name) in chapter.slides.iter().enumerate() {
                let head = (i == 0).then(|| chapter.title.clone());
                let doc = dpl_doc::load(&dir.join(name))
                    .map(|s| DocSlide::with_look(s, look.clone()))
                    .map_err(|e| e.to_string())?;
                rows.push((head, name.clone(), doc));
            }
        }
        Ok(Self {
            id,
            title: ep.title,
            rows,
            issues,
        })
    }
}

impl WorkbenchHub {
    /// The EPISODES subroute: an episode picker, then the show's slides in
    /// order (chapter headers + each slide live). Read-only gallery gate.
    fn episodes_surface(&mut self, ui: &mut egui::Ui) {
        if let Some(view) = self.episode_view.as_mut() {
            let back = ui
                .horizontal(|ui| {
                    let b = ui
                        .label(RichText::new("‹ all shows").color(SPEC.tone.color()))
                        .interact(egui::Sense::click());
                    ui.label(RichText::new(&view.title).strong().color(cl::ink::STRONG));
                    ui.label(
                        RichText::new(&view.id)
                            .monospace()
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                    b
                })
                .inner;
            if back.clicked() {
                self.episode_view = None;
                return;
            }
            for issue in &view.issues {
                ui.label(
                    RichText::new(format!("⚠ {} — {}", issue.path, issue.message))
                        .small()
                        .color(SPEC.tone.color()),
                );
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let stage_w = (ui.available_width() - cl::space::S4 * 2.0).min(720.0);
                    let stage_h = stage_w * 9.0 / 16.0;
                    for (n, (head, name, doc)) in view.rows.iter_mut().enumerate() {
                        if let Some(title) = head {
                            ui.add_space(cl::space::S3);
                            ui.label(
                                RichText::new(title.to_uppercase())
                                    .small()
                                    .color(cl::ink::SUBTLE),
                            );
                        }
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{}", n + 1))
                                    .monospace()
                                    .color(SPEC.tone.color()),
                            );
                            ui.label(RichText::new(name.as_str()).color(cl::ink::MUTED));
                        });
                        let (stage, _) = ui.allocate_exact_size(
                            egui::vec2(stage_w, stage_h),
                            egui::Sense::hover(),
                        );
                        ui.painter()
                            .rect_filled(stage, cl::RADIUS, cl::surface::WELL);
                        doc.ui(ui, stage.shrink(2.0));
                        ui.painter().rect_stroke(
                            stage,
                            cl::RADIUS,
                            egui::Stroke::new(1.0, cl::surface::HAIRLINE),
                            egui::StrokeKind::Inside,
                        );
                        ui.add_space(cl::space::S4);
                    }
                });
            return;
        }

        ui.label(
            RichText::new("The shows — tap one to see its slides in order")
                .small()
                .color(cl::ink::SUBTLE),
        );
        ui.add_space(cl::space::S2);

        // The "copy to my shows" form (X3): name the show category + the
        // episode, then clone_episode copies the seed into content.
        if let Some(done) = self.clone_form_ui(ui) {
            if done {
                self.clone_form = None;
            }
        }

        let content_root = dpl_engine_capture::content_root();
        let mut open: Option<(String, PathBuf)> = None;
        let mut clone_from: Option<(String, PathBuf)> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // The union (X3): content shows first, templates after.
                for entry in dpl_doc::all_episodes(&content_root) {
                    let title = dpl_doc::load_episode(&entry.dir)
                        .map(|e| e.title)
                        .unwrap_or_else(|_| entry.id.clone());
                    let badge = match &entry.show {
                        Some(show) => format!("{show}/{}", entry.id),
                        None => format!("template · {}", entry.id),
                    };
                    let is_template = entry.show.is_none();
                    let mut copy_clicked = false;
                    let resp = cl::card_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width().min(560.0));
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&title).strong().color(cl::ink::STRONG));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new("open ›").small().color(SPEC.tone.color()),
                                    );
                                    ui.label(
                                        RichText::new(&badge)
                                            .monospace()
                                            .small()
                                            .color(cl::ink::SUBTLE),
                                    );
                                    if is_template {
                                        let chip = ui
                                            .label(
                                                RichText::new("copy to my shows")
                                                    .small()
                                                    .color(cl::Tone::Amber.color()),
                                            )
                                            .interact(egui::Sense::click());
                                        if chip.clicked() {
                                            copy_clicked = true;
                                        }
                                    }
                                },
                            );
                        });
                    });
                    if copy_clicked {
                        clone_from = Some((entry.id.clone(), entry.dir.clone()));
                    } else if resp.response.interact(egui::Sense::click()).clicked() {
                        open = Some((entry.id.clone(), entry.dir.clone()));
                    }
                    ui.add_space(cl::space::S2);
                }
            });
        if let Some((id, dir)) = clone_from {
            self.clone_form = Some(CloneForm {
                seed_id: id,
                seed_dir: dir,
                show: String::new(),
                episode: String::new(),
                error: None,
            });
            return;
        }
        if let Some((id, dir)) = open {
            match EpisodeView::open(&dir, id) {
                Ok(v) => self.episode_view = Some(v),
                Err(e) => {
                    ui.label(RichText::new(format!("could not open: {e}")).color(cl::ink::MUTED));
                }
            }
        }
    }

    /// The clone form, when open. Returns Some(true) when it finished
    /// (created or cancelled) and should close.
    fn clone_form_ui(&mut self, ui: &mut egui::Ui) -> Option<bool> {
        let form = self.clone_form.as_mut()?;
        let content_root = dpl_engine_capture::content_root();
        let mut done = false;
        cl::card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width().min(560.0));
            ui.label(
                RichText::new(format!("Copy \"{}\" into my shows", form.seed_id))
                    .strong()
                    .color(cl::ink::STRONG),
            );
            ui.add_space(cl::space::S1);
            // Show suggestions: the existing shows, or the operator's
            // three ratified categories while content is still empty.
            let existing: Vec<String> = dpl_doc::list_shows(&content_root)
                .into_iter()
                .map(|(s, _)| s)
                .collect();
            let suggestions: Vec<String> = if existing.is_empty() {
                SHOW_SUGGESTIONS.iter().map(|s| s.to_string()).collect()
            } else {
                existing
            };
            ui.horizontal_wrapped(|ui| {
                for s in &suggestions {
                    let chip = ui
                        .label(RichText::new(s).small().color(SPEC.tone.color()))
                        .interact(egui::Sense::click());
                    if chip.clicked() {
                        form.show = s.clone();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("show").small().color(cl::ink::SUBTLE));
                ui.add(egui::TextEdit::singleline(&mut form.show).desired_width(200.0));
                ui.label(RichText::new("episode").small().color(cl::ink::SUBTLE));
                ui.add(egui::TextEdit::singleline(&mut form.episode).desired_width(200.0));
            });
            if let Some(e) = &form.error {
                ui.label(RichText::new(e).small().color(cl::Tone::Coral.color()));
            }
            ui.horizontal(|ui| {
                let create = ui
                    .label(RichText::new("create").color(SPEC.tone.color()))
                    .interact(egui::Sense::click());
                let cancel = ui
                    .label(RichText::new("cancel").color(cl::ink::MUTED))
                    .interact(egui::Sense::click());
                if create.clicked() {
                    match dpl_doc::clone_episode(
                        &form.seed_dir,
                        &content_root,
                        form.show.trim(),
                        form.episode.trim(),
                    ) {
                        Ok(_) => done = true,
                        Err(e) => form.error = Some(e),
                    }
                }
                if cancel.clicked() {
                    done = true;
                }
            });
        });
        ui.add_space(cl::space::S2);
        Some(done)
    }

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
        let picker_card =
            |ui: &mut egui::Ui, label: &str, open: &mut Option<PathBuf>, dir: PathBuf| {
                let resp = cl::card_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width().min(560.0));
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(label).strong().color(cl::ink::STRONG));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("edit ›").small().color(SPEC.tone.color()));
                        });
                    });
                });
                if resp.response.interact(egui::Sense::click()).clicked() {
                    *open = Some(dir);
                }
                ui.add_space(cl::space::S2);
            };
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (id, dir) in dpl_doc::list_seeds() {
                    picker_card(ui, &id, &mut open, dir);
                }
                // The shows (W3b/X3): the seeds+content UNION — episode
                // slides edit against their show's look (the editor
                // detects it from the parent folder), content first.
                for entry in dpl_doc::all_episodes(&dpl_engine_capture::content_root()) {
                    let Ok(ep) = dpl_doc::load_episode(&entry.dir) else {
                        continue;
                    };
                    let badge = match &entry.show {
                        Some(show) => format!("{show}/{}", entry.id),
                        None => format!("template · {}", entry.id),
                    };
                    ui.add_space(cl::space::S3);
                    ui.label(
                        RichText::new(ep.title.to_uppercase())
                            .small()
                            .color(cl::ink::SUBTLE),
                    )
                    .on_hover_text(badge);
                    for r in dpl_doc::episode_slides(&entry.dir, &ep) {
                        picker_card(ui, &format!("{} · {}", r.chapter, r.name), &mut open, r.dir);
                    }
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
