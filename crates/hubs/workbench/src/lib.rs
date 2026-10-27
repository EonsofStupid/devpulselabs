//! Workbench — the BUILD house (doctrine §1.1): the ingredient LIBRARY
//! leads, then the editor tailors, episodes compose, the studio polishes.
//!
//! LIBRARY is the hub's default route: ALL the raw ingredients on one
//! sectioned surface — seed slides by class folder, every registered
//! template live, and the vocabulary walls (colors · presets · fonts ·
//! moods). The editor (W4) opens exactly these documents.

mod author;
mod backgrounds;
mod containers;
mod controls;
mod experiments;
mod lessons;
mod library;
mod pictures;
mod studio;
mod words;

use author::AuthorSurface;
use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use dpl_slides::document::DocSlide;
use dpl_slides::SlideTemplate;
use egui::RichText;
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};
use library::LibSection;
use std::path::PathBuf;
use studio::StudioState;

pub const HUB: HubId = HubId("workbench");
pub const LIBRARY: SubId = SubId("library");
pub const AUTHOR: SubId = SubId("author");
pub const EPISODES: SubId = SubId("episodes");
pub const EDITOR: SubId = SubId("editor");
pub const LESSONS: SubId = SubId("lessons");
pub const STUDIO: SubId = SubId("studio");
pub const TEST_LAB: SubId = SubId("test-lab");
pub const EXPERIMENTS: SubId = SubId("experiments");

pub(crate) static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Workbench",
    icon: cl::icon::WORKBENCH,
    tone: Tone::Amber,
    subroutes: &[
        SubRoute {
            id: LIBRARY,
            label: "Library",
            blurb: "ALL the raw ingredients: seed slides by class, live templates, colors, presets, fonts, moods.",
        },
        SubRoute {
            id: AUTHOR,
            label: "Author",
            blurb: "Say what you want and seed a card — the intent-first authoring surface (charter pass 1).",
        },
        SubRoute {
            id: EDITOR,
            label: "Editor",
            blurb: "Tap a slide to change it — font, colours, motion; edits write the colocated file.",
        },
        SubRoute {
            id: LESSONS,
            label: "Lessons",
            blurb: "Your lessons — runs of cards from your library, each keeping the version it chose until you update it.",
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
            id: EXPERIMENTS,
            label: "Experiments",
            blurb: "Each experiment its own domain — hypothesis, minimal requirements, and its arms' runs grouped from the evidence on disk.",
        },
        SubRoute {
            id: TEST_LAB,
            label: "Test Lab",
            blurb: "Passive agent snapshots, full semantics, run bundles, and trace previews. Live control and approval are not implemented.",
        },
    ],
};

/// One loaded seed: its id, the colocated folder, the live document (or the
/// load error, honestly), and the validator's findings.
struct SeedEntry {
    /// The seed's class folder (doctrine §1.3) — the wall sections by it.
    class: &'static str,
    id: String,
    dir: PathBuf,
    doc: Result<DocSlide, String>,
    issues: Vec<dpl_doc::Issue>,
}

#[derive(Default)]
pub struct WorkbenchHub {
    /// The NAVIGATE seam: tapped placeholder cards publish through here.
    fabric: Option<dpl_fabric::Fabric>,
    /// Loaded on first visit; the reload chip clears it (edits to the
    /// colocated files show up without restarting the app).
    seeds: Option<Vec<SeedEntry>>,
    /// The seed-tree doctor's findings, refreshed with `seeds` — an
    /// unclassed/unknown folder under seeds/slides is NAMED, never hidden.
    tree_issues: Vec<dpl_doc::Issue>,
    /// The Library surface's open section (chip row).
    lib_section: LibSection,
    /// The looks the PRESETS section shows — the built-in ones plus the
    /// operator's own (LK2). Loaded when the section opens, dropped when
    /// it closes, so a look saved meanwhile shows on the next visit.
    looks: Option<dpl_doc::Looks>,
    /// One live instance per registered template — the TEMPLATES section's
    /// stock, lazy so Default stays derivable.
    demos: Option<Vec<Box<dyn SlideTemplate>>>,
    /// TEMPLATES solo view: one template full-stage (tap to enter, « to
    /// leave). Also the harness's per-template proof hook (`--focus <id>`).
    focus: Option<String>,
    /// The open editor (EDITOR subroute) — None shows the seed picker.
    editor: Option<dpl_editor::Editor>,
    /// The editor picker's cards, read when the page opens and after each
    /// edit — never per frame (each episode is walked card by card).
    picker: Option<Vec<PickerGroup>>,
    /// The episode built from lessons the open card was picked from, its
    /// title, and the card's revision when it opened: on the way back the
    /// episode takes the change (LS3b).
    editing_from: Option<(PathBuf, String, u32)>,
    /// What the last edit made from an episode did, in words.
    picker_note: Option<String>,
    /// The LESSONS subroute (LS3a) — your lessons and the one open.
    lessons: lessons::Lessons,
    /// The episode-wall's live slide docs, loaded once per episode open.
    episode_view: Option<EpisodeView>,
    /// The "copy to my shows" form (X3) — open on the episodes picker.
    clone_form: Option<CloneForm>,
    /// "Move it into the library" (LS3b2): a show still in the old shape,
    /// and what moving it would do — shown before anything moves.
    move_sheet: Option<MoveSheet>,
    /// W10a Studio — place a finished slide into a content episode.
    studio: StudioState,
    /// The authoring surface (charter pass 1) — intent bar + draft mint,
    /// signal-first.
    author: AuthorSurface,
    test_lab: egui_pilot_ai_lab::TestLab,
    experiments: experiments::Experiments,
}

/// The clone-a-seed form: names a show category + episode (kebab law),
/// runs `dpl_doc::clone_episode`. Seeds are read, never written.
/// The plan for moving one old-shape show into the library, as the sheet
/// shows it.
struct MoveSheet {
    id: String,
    plan: dpl_doc::legacy::MovePlan,
    error: Option<String>,
}

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
    pub fn wire_fabric(&mut self, fabric: &dpl_fabric::Fabric) {
        self.fabric = Some(fabric.clone());
    }

    /// Solo one template on the Library's TEMPLATES section (unknown ids
    /// degrade to the wall). The `--focus <template-id>` proof hook.
    pub fn set_focus(&mut self, id: impl Into<String>) {
        self.focus = Some(id.into());
        self.lib_section = LibSection::Templates;
    }

    /// Mint a card and open its overlay editor — the `--author-editor` proof
    /// seam, so the overlay is provable on the real exe.
    pub fn open_author_editor(&mut self) {
        self.author.open_editor_drill();
    }

    /// Open the card editor on one tab by slug — the `--card-tab <slug>`
    /// proof seam, so a tab other than Words is provable on the real exe.
    pub fn open_card_tab(&mut self, slug: &str) {
        self.author.card_tab_drill(slug);
    }

    /// Start `n` cards on the authoring surface WITHOUT opening the editor —
    /// the `--author-draft <n>` proof seam. The filmstrip is only interesting
    /// with something in it, and the overlay would cover it.
    pub fn author_draft(&mut self, n: usize) {
        self.author.draft_drill(n);
    }

    /// Point the authoring surface at one container kind — the
    /// `--author-container <kind>` proof seam, so the honest-empty line of a
    /// kind with no storage can be photographed alongside a real one.
    ///
    /// Unknown slugs are ignored rather than guessed at.
    pub fn author_container(&mut self, slug: &str) {
        self.author.container_drill(slug);
    }

    /// Open the Library on one SECTION by its chip slug — the
    /// `--lib-section <slug>` proof hook. Every wall other than templates was
    /// unreachable from the harness before DS3, so no shelf but that one could
    /// be proven on the real exe.
    ///
    /// Unknown slugs degrade to the default section (law: degrade, never
    /// crash) and report it, so a typo in a proof script is visible rather
    /// than silently shooting the wrong wall.
    pub fn set_lib_section(&mut self, slug: &str) -> bool {
        match LibSection::ALL.iter().find(|s| s.slug() == slug) {
            Some(&section) => {
                self.lib_section = section;
                true
            }
            None => false,
        }
    }

    /// Open one episode's wall — the same path a picker tap takes (also
    /// the `--open-episode` harness seam). Accepts the picker's badge
    /// vocabulary: `<show>/<id>`, `template/<id>` (a seed), or a bare id
    /// (content wins collisions — the union's order). Unknown ids
    /// degrade to the picker.
    pub fn open_episode(&mut self, id: &str) {
        let union = dpl_doc::all_episodes(&dpl_roots::content_root());
        let hit = match id.split_once('/') {
            Some(("template", ep)) => union.into_iter().find(|e| e.show.is_none() && e.id == ep),
            Some((show, ep)) => union
                .into_iter()
                .find(|e| e.show.as_deref() == Some(show) && e.id == ep),
            None => union.into_iter().find(|e| e.id == id),
        };
        if let Some(entry) = hit {
            if let Ok(v) = EpisodeView::open(&entry.dir, entry.id) {
                self.episode_view = Some(v);
            }
        }
    }

    /// The ingredient LIBRARY (doctrine §1.1): the section chip row over
    /// the walls — seeds by class, live templates, colors, presets,
    /// fonts, moods.
    fn library_surface(&mut self, ui: &mut egui::Ui) {
        let was = self.lib_section;
        library::chip_row(ui, &mut self.lib_section);
        if self.lib_section != was {
            self.looks = None;
        }
        match self.lib_section {
            LibSection::Seeds => self.seeds_wall(ui),
            LibSection::Templates => {
                let demos = self.demos.get_or_insert_with(dpl_slides::demos);
                library::templates_gallery(ui, demos, &mut self.focus);
            }
            LibSection::Blocks => library::blocks_wall(ui),
            LibSection::Elements => library::elements_wall(ui),
            LibSection::Themes => library::traits_wall(ui),
            LibSection::Colors => library::pool_wall(ui),
            LibSection::Presets => {
                let looks = self
                    .looks
                    .get_or_insert_with(|| dpl_doc::Looks::load(&dpl_roots::content_root()));
                library::presets_wall(ui, looks);
            }
            LibSection::Fonts => library::fonts_wall(ui),
            LibSection::Icons => library::icons_wall(ui),
            LibSection::Moods => library::moods_wall(ui),
        }
    }

    fn load_seeds() -> Vec<SeedEntry> {
        let looks = dpl_doc::Looks::load(&dpl_roots::content_root());
        dpl_doc::list_seeds()
            .into_iter()
            .map(|s| match dpl_doc::load(&s.dir) {
                Ok(slide) => {
                    let issues = dpl_doc::validate(&slide);
                    SeedEntry {
                        class: s.class,
                        id: s.id,
                        dir: s.dir,
                        doc: Ok(DocSlide::new(slide, looks.clone())),
                        issues,
                    }
                }
                Err(e) => SeedEntry {
                    class: s.class,
                    id: s.id,
                    dir: s.dir,
                    doc: Err(e.to_string()),
                    issues: Vec::new(),
                },
            })
            .collect()
    }

    fn seeds_wall(&mut self, ui: &mut egui::Ui) {
        if self.seeds.is_none() {
            self.tree_issues = dpl_doc::seed_tree_issues();
        }
        let seeds = self.seeds.get_or_insert_with(Self::load_seeds);

        let mut reload = false;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "SEED SLIDES — {} raw ingredients · one folder per seed, sectioned by class; the editor edits the colocated slide.json",
                    seeds.len()
                ))
                .small()
                .color(cl::ink::SUBTLE),
            );
            let chip = ui
                .label(RichText::new("reload").monospace().small().color(SPEC.tone.color()))
                .interact(egui::Sense::click());
            ui.pilot_track(
                &chip,
                NodeMetadata::new(
                    PilotId::new("workbench.library.seeds").child(&PilotKey::new("reload")),
                    egui_pilot_ai::protocol::Role::Button,
                    "reload",
                ),
            );
            if chip.clicked() {
                reload = true;
            }
        });
        ui.add_space(cl::space::S2);

        for issue in &self.tree_issues {
            ui.label(
                RichText::new(format!("⚠ seeds/slides/{} — {}", issue.path, issue.message))
                    .small()
                    .color(SPEC.tone.color()),
            );
        }
        if seeds.is_empty() {
            ui.label(
                RichText::new("no seeds found — expected seeds/slides/<class>/<id>/slide.json")
                    .color(cl::ink::MUTED),
            );
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let avail_w = ui.available_width();
                let stage_w = avail_w - cl::space::S4 * 2.0;
                let stage_h = stage_w * 9.0 / 16.0;
                // Always emit every seed class header (doctrine §1.3) — empty
                // classes stay honest instead of vanishing from the wall.
                for class in dpl_doc::SEED_CLASSES {
                    ui.add_space(cl::space::S2);
                    ui.label(
                        RichText::new(format!("{} —", class.to_uppercase()))
                            .small()
                            .strong()
                            .color(SPEC.tone.color()),
                    );
                    ui.add_space(cl::space::S2);
                    let mut any = false;
                    for entry in seeds.iter_mut().filter(|e| e.class == class) {
                        any = true;
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
                        let (stage, _) = ui.allocate_exact_size(
                            egui::vec2(stage_w, stage_h),
                            egui::Sense::hover(),
                        );
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
                    if !any {
                        ui.label(
                            RichText::new(format!(
                                "no {class} seeds yet — folder stays ready for slide.json"
                            ))
                            .small()
                            .color(cl::ink::MUTED),
                        );
                    }
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

    fn library_back(&mut self) {
        self.author.save_again();
        self.lessons.save_again(&dpl_roots::content_root());
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        if route.sub == LIBRARY {
            self.library_surface(ui);
        } else if route.sub == AUTHOR {
            self.author.ui(ui, &self.fabric);
        } else if route.sub == EDITOR {
            self.editor_surface(ui);
        } else if route.sub == LESSONS {
            self.lessons.ui(ui, &self.fabric);
        } else if route.sub == EPISODES {
            self.episodes_surface(ui);
        } else if route.sub == STUDIO {
            self.studio.ui(ui, &self.fabric);
        } else if route.sub == EXPERIMENTS {
            self.experiments.ui(ui);
        } else if route.sub == TEST_LAB {
            self.test_lab.ui(ui);
        } else if let Some(go) = dpl_hub::placeholder::surface(
            ui,
            &SPEC,
            route,
            "The episode factory. Everything captured or generated ends up on a slide here.",
        ) {
            if let Some(fabric) = &self.fabric {
                fabric.publish_shell(dpl_fabric::ShellEvent::NavigateRequest { route: go });
            }
        }
    }
}

/// An opened episode's live slides, in the authored chapter→slide order.
/// One wall row: the chapter header (first slide of a chapter), the
/// slide, and its script chip (D5) — "script · N beats" when a clean
/// script.mdx is colocated; broken scripts surface as doctor issues.
struct EpisodeRow {
    head: Option<String>,
    name: String,
    /// The card, live — or why it will not load (the row says so; the rest
    /// of the episode still shows).
    doc: Result<DocSlide, String>,
    script: Option<String>,
}

struct EpisodeView {
    id: String,
    /// The episode's folder — what Run runs (RL).
    dir: std::path::PathBuf,
    title: String,
    rows: Vec<EpisodeRow>,
    issues: Vec<dpl_doc::Issue>,
}

impl EpisodeView {
    /// Read through the one episode walk (LS3b): an episode built from
    /// lessons and a seed template show alike, every card in the episode's
    /// own theme.
    fn open(dir: &std::path::Path, id: String) -> Result<Self, String> {
        let root = dpl_roots::content_root();
        let shown = dpl_doc::episode::shows(dir, &root).map_err(|e| e.to_string())?;
        let issues = dpl_doc::episode::doctor(dir, &root);
        let looks = dpl_doc::Looks::load(&root);
        let mut rows = Vec::new();
        for chapter in &shown.chapters {
            for (i, card) in chapter.cards.iter().enumerate() {
                rows.push(EpisodeRow {
                    head: (i == 0).then(|| chapter.title.clone()),
                    name: card.label.clone(),
                    doc: card.slide.clone().map(|slide| {
                        DocSlide::styled(
                            slide,
                            dpl_doc::Styling {
                                theme: shown.theme.clone(),
                                looks: looks.clone(),
                            },
                        )
                    }),
                    script: card
                        .script
                        .as_ref()
                        .map(|s| format!("script · {} beats", s.beats.len())),
                });
            }
        }
        Ok(Self {
            id,
            dir: dir.to_path_buf(),
            title: shown.title,
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
            let (back, run) = ui
                .horizontal(|ui| {
                    let b = ui
                        .label(RichText::new("‹ all shows").color(SPEC.tone.color()))
                        .interact(egui::Sense::click());
                    ui.pilot_track(
                        &b,
                        NodeMetadata::new(
                            PilotId::new("workbench.episodes").child(&PilotKey::new("back")),
                            egui_pilot_ai::protocol::Role::Button,
                            "all shows",
                        ),
                    );
                    ui.label(RichText::new(&view.title).strong().color(cl::ink::STRONG));
                    ui.label(
                        RichText::new(&view.id)
                            .monospace()
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                    // Run (RL): the Present hub shows it full screen.
                    let run = ui.add(
                        egui::Button::new(RichText::new("▶ Run").strong().color(SPEC.tone.color()))
                            .min_size(egui::vec2(96.0, 36.0)),
                    );
                    ui.pilot_track(
                        &run,
                        NodeMetadata::new(
                            PilotId::new("workbench.episodes").child(&PilotKey::new("run")),
                            egui_pilot_ai::protocol::Role::Button,
                            "Run this episode",
                        ),
                    );
                    let run = cl::tip(
                        run,
                        "Show this episode full screen, with your script on the other screen",
                    );
                    (b, run.clicked())
                })
                .inner;
            if back.clicked() {
                self.episode_view = None;
                return;
            }
            if run {
                if let Some(fabric) = &self.fabric {
                    fabric.publish_deck_command(dpl_fabric::DeckCommand::Run {
                        target: dpl_fabric::RunTarget::Episode(view.dir.clone()),
                        start: 0,
                    });
                    fabric.publish_shell(dpl_fabric::ShellEvent::NavigateRequest {
                        route: Route {
                            hub: dpl_hub::HubId("present"),
                            sub: dpl_hub::SubId("shows"),
                        },
                    });
                }
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
                    for (n, row) in view.rows.iter_mut().enumerate() {
                        if let Some(title) = &row.head {
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
                            ui.label(RichText::new(row.name.as_str()).color(cl::ink::MUTED));
                            // The script chip (D5): the teleprompter's
                            // data, visible before D4 consumes it.
                            if let Some(chip) = &row.script {
                                ui.label(
                                    RichText::new(chip.as_str())
                                        .monospace()
                                        .small()
                                        .color(SPEC.tone.color()),
                                );
                            }
                        });
                        let (stage, _) = ui.allocate_exact_size(
                            egui::vec2(stage_w, stage_h),
                            egui::Sense::hover(),
                        );
                        ui.painter()
                            .rect_filled(stage, cl::RADIUS, cl::surface::WELL);
                        match &mut row.doc {
                            Ok(doc) => doc.ui(ui, stage.shrink(2.0)),
                            Err(e) => {
                                ui.painter().text(
                                    stage.center(),
                                    egui::Align2::CENTER_CENTER,
                                    format!("this card will not load — {e}"),
                                    egui::FontId::proportional(cl::Fluid::of(stage).fs_body_sm()),
                                    cl::ink::MUTED,
                                );
                            }
                        }
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
        if self.move_sheet_ui(ui) {
            return;
        }

        let content_root = dpl_roots::content_root();
        let mut open: Option<(String, PathBuf)> = None;
        let mut clone_from: Option<(String, PathBuf)> = None;
        let mut move_from: Option<(String, PathBuf)> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // The union (X3): content shows first, templates after.
                for entry in dpl_doc::all_episodes(&content_root) {
                    let title = dpl_doc::episode::summary(&entry.dir, &content_root)
                        .map_or_else(|| entry.id.clone(), |e| e.title);
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
                                },
                            );
                        });
                    });
                    let card = resp.response.interact(egui::Sense::click());
                    ui.pilot_track(
                        &card,
                        NodeMetadata::new(
                            PilotId::new("workbench.episodes.open")
                                .child(&PilotKey::new(&entry.id)),
                            egui_pilot_ai::protocol::Role::Button,
                            title.clone(),
                        )
                        .value(if is_template {
                            "template"
                        } else {
                            "show"
                        }),
                    );
                    // Its own control under the card: inside it, the card's
                    // own tap target sat on top and swallowed every tap.
                    if is_template {
                        let chip = cl::tip(
                            controls::control(
                                ui,
                                "Copy into my shows",
                                false,
                                cl::Tone::Amber,
                                controls::Weight::Chrome,
                            ),
                            "Make your own episode from this template: each chapter becomes a \
                             lesson, each slide your own card",
                        );
                        ui.pilot_track(
                            &chip,
                            NodeMetadata::new(
                                PilotId::new("workbench.episodes.copy")
                                    .child(&PilotKey::new(&entry.id)),
                                egui_pilot_ai::protocol::Role::Button,
                                "Copy into my shows",
                            ),
                        );
                        copy_clicked = chip.clicked();
                    }
                    // A show of the operator's still in the old shape: it
                    // moves into the library only when asked (LS3b2).
                    let old_shape = !is_template
                        && matches!(
                            dpl_doc::episode::open(&entry.dir),
                            Ok(dpl_doc::episode::Opened::Template(_))
                        );
                    if old_shape {
                        let chip = cl::tip(
                            controls::control(
                                ui,
                                "Move into the library…",
                                false,
                                cl::Tone::Cyan,
                                controls::Weight::Chrome,
                            ),
                            "See what moving this show into your library would do — nothing \
                             moves until you say so",
                        );
                        ui.pilot_track(
                            &chip,
                            NodeMetadata::new(
                                PilotId::new("workbench.episodes.move")
                                    .child(&PilotKey::new(&entry.id)),
                                egui_pilot_ai::protocol::Role::Button,
                                "Move into the library…",
                            ),
                        );
                        if chip.clicked() {
                            move_from = Some((entry.id.clone(), entry.dir.clone()));
                        }
                    }
                    if copy_clicked {
                        clone_from = Some((entry.id.clone(), entry.dir.clone()));
                    } else if card.clicked() {
                        open = Some((entry.id.clone(), entry.dir.clone()));
                    }
                    ui.add_space(cl::space::S2);
                }
            });
        if let Some((id, dir)) = move_from {
            match dpl_doc::legacy::plan_move(&dir, &content_root) {
                Ok(plan) => {
                    self.move_sheet = Some(MoveSheet {
                        id,
                        plan,
                        error: None,
                    })
                }
                Err(e) => {
                    ui.label(
                        RichText::new(format!("could not plan the move: {e}"))
                            .color(cl::ink::MUTED),
                    );
                }
            }
            return;
        }
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

    /// The move sheet, when open: exactly what moving the show would do,
    /// then Move it or Not now. Returns true while it is open (the list
    /// waits under it).
    fn move_sheet_ui(&mut self, ui: &mut egui::Ui) -> bool {
        let Some(sheet) = self.move_sheet.as_mut() else {
            return false;
        };
        let fluid = cl::Fluid::of(ui.max_rect());
        let node =
            |key: &str| PilotId::new("workbench.episodes.move-sheet").child(&PilotKey::new(key));
        let plan = &sheet.plan;
        let (mut go, mut close) = (false, false);
        cl::card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width().min(fluid.panel_width() * 1.4));
            ui.label(
                RichText::new(format!("Move “{}” into your library", plan.title))
                    .size(fluid.fs_h4())
                    .strong()
                    .color(cl::ink::STRONG),
            );
            ui.label(
                RichText::new(
                    "Its slides become cards in your library and each chapter becomes a \
                     lesson, so the same card can be used in more than one lesson and a change \
                     to a card is offered to every lesson that uses it.",
                )
                .size(fluid.fs_body_sm())
                .color(cl::ink::MUTED),
            );
            ui.add_space(cl::space::S2);
            ui.label(RichText::new("Each chapter becomes a lesson:").color(cl::ink::BASE));
            for lesson in &plan.lessons {
                ui.label(
                    RichText::new(format!(
                        "  • {} — {}",
                        lesson.title,
                        match lesson.cards.len() {
                            1 => "1 card".to_owned(),
                            n => format!("{n} cards"),
                        }
                    ))
                    .size(fluid.fs_body_sm())
                    .color(cl::ink::BASE),
                );
            }
            let moving = plan.slides.len() + plan.orphans.len();
            let scripts = plan
                .slides
                .iter()
                .chain(&plan.orphans)
                .filter(|s| s.script)
                .count();
            let pictures: usize = plan
                .slides
                .iter()
                .chain(&plan.orphans)
                .map(|s| s.media.len())
                .sum();
            let copies = plan
                .slides
                .iter()
                .chain(&plan.orphans)
                .filter(|s| s.forked)
                .count();
            let mut facts = vec![format!(
                "{moving} slide{} move into your library{}.",
                if moving == 1 { "" } else { "s" },
                match plan.orphans.len() {
                    0 => String::new(),
                    n => format!(" ({n} of them in no chapter, kept as cards)"),
                }
            )];
            if scripts > 0 {
                facts.push(match scripts {
                    1 => "1 script goes with its slide.".to_owned(),
                    n => format!("{n} scripts go with their slides."),
                });
            }
            if pictures > 0 {
                facts.push(match pictures {
                    1 => "1 picture goes with its slide.".to_owned(),
                    n => format!("{n} pictures go with their slides."),
                });
            }
            if copies > 0 {
                facts.push(match copies {
                    1 => "1 slide is already in your library, so it moves as a copy — the one \
                          there stays."
                        .to_owned(),
                    n => format!(
                        "{n} slides are already in your library, so they move as copies — the \
                         ones there stay."
                    ),
                });
            }
            facts.push(
                "The show's old file is kept beside it (episode.v1.json). Nothing is deleted."
                    .into(),
            );
            ui.add_space(cl::space::S2);
            for fact in &facts {
                ui.label(
                    RichText::new(fact)
                        .size(fluid.fs_body_sm())
                        .color(cl::ink::BASE),
                );
            }
            if !plan.refused.is_empty() {
                ui.add_space(cl::space::S2);
                ui.label(
                    RichText::new("It can't move yet:")
                        .strong()
                        .color(cl::Tone::Coral.color()),
                );
                for why in &plan.refused {
                    ui.label(
                        RichText::new(format!("  • {why}"))
                            .size(fluid.fs_body_sm())
                            .color(cl::Tone::Coral.color()),
                    );
                }
            }
            if let Some(e) = &sheet.error {
                ui.label(RichText::new(e).small().color(cl::Tone::Coral.color()));
            }
            ui.add_space(cl::space::S3);
            ui.horizontal_wrapped(|ui| {
                if plan.refused.is_empty() {
                    let move_it = cl::tip(
                        controls::control(ui, "Move it", false, SPEC.tone, controls::Weight::Hero),
                        "Move this show into your library now — its old file is kept",
                    );
                    ui.pilot_track(
                        &move_it,
                        NodeMetadata::new(
                            node("move"),
                            egui_pilot_ai::protocol::Role::Button,
                            "Move it",
                        ),
                    );
                    go = move_it.clicked();
                }
                let not_now = cl::tip(
                    controls::control(ui, "Not now", false, SPEC.tone, controls::Weight::Chrome),
                    "Leave the show as it is",
                );
                ui.pilot_track(
                    &not_now,
                    NodeMetadata::new(
                        node("not-now"),
                        egui_pilot_ai::protocol::Role::Button,
                        "Not now",
                    ),
                );
                close = not_now.clicked();
            });
        });
        ui.add_space(cl::space::S2);
        if close {
            self.move_sheet = None;
        } else if go {
            let root = dpl_roots::content_root();
            let dir = sheet.plan.dir.clone();
            match dpl_doc::legacy::move_episode(&dir, &root) {
                Ok(_) => {
                    let id = sheet.id.clone();
                    self.move_sheet = None;
                    if let Ok(view) = EpisodeView::open(&dir, id) {
                        self.episode_view = Some(view);
                    }
                }
                Err(e) => sheet.error = Some(e),
            }
        }
        true
    }

    /// The clone form, when open. Returns Some(true) when it finished
    /// (created or cancelled) and should close.
    fn clone_form_ui(&mut self, ui: &mut egui::Ui) -> Option<bool> {
        let form = self.clone_form.as_mut()?;
        let content_root = dpl_roots::content_root();
        let mut done = false;
        // The episode just made, opened straight away (LS3b).
        let mut made: Option<(PathBuf, String)> = None;
        let node = |key: &str| PilotId::new("workbench.episodes.clone").child(&PilotKey::new(key));
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
                let show = ui.add(egui::TextEdit::singleline(&mut form.show).desired_width(200.0));
                ui.pilot_track(
                    &show,
                    NodeMetadata::new(
                        node("show"),
                        egui_pilot_ai::protocol::Role::TextInput,
                        "show",
                    ),
                );
                cl::tip(
                    show,
                    "Which show it goes under — lower-case words joined by dashes",
                );
                ui.label(RichText::new("episode").small().color(cl::ink::SUBTLE));
                let episode =
                    ui.add(egui::TextEdit::singleline(&mut form.episode).desired_width(200.0));
                ui.pilot_track(
                    &episode,
                    NodeMetadata::new(
                        node("episode"),
                        egui_pilot_ai::protocol::Role::TextInput,
                        "episode",
                    ),
                );
                cl::tip(
                    episode,
                    "The episode's folder name — lower-case words joined by dashes",
                );
            });
            if let Some(e) = &form.error {
                ui.label(RichText::new(e).small().color(cl::Tone::Coral.color()));
            }
            ui.horizontal(|ui| {
                let create = ui
                    .label(RichText::new("create").color(SPEC.tone.color()))
                    .interact(egui::Sense::click());
                ui.pilot_track(
                    &create,
                    NodeMetadata::new(
                        node("create"),
                        egui_pilot_ai::protocol::Role::Button,
                        "create",
                    ),
                );
                let create = cl::tip(
                    create,
                    "Make the episode: each chapter becomes a lesson, each slide your own card",
                );
                let cancel = ui
                    .label(RichText::new("cancel").color(cl::ink::MUTED))
                    .interact(egui::Sense::click());
                ui.pilot_track(
                    &cancel,
                    NodeMetadata::new(
                        node("cancel"),
                        egui_pilot_ai::protocol::Role::Button,
                        "cancel",
                    ),
                );
                let cancel = cl::tip(cancel, "Don't copy it");
                if create.clicked() {
                    match dpl_doc::clone_episode(
                        &form.seed_dir,
                        &content_root,
                        form.show.trim(),
                        form.episode.trim(),
                    ) {
                        Ok(dest) => {
                            made = Some((dest, form.episode.trim().to_owned()));
                            done = true;
                        }
                        Err(e) => form.error = Some(e),
                    }
                }
                if cancel.clicked() {
                    done = true;
                }
            });
        });
        ui.add_space(cl::space::S2);
        if let Some((dir, id)) = made {
            match EpisodeView::open(&dir, id) {
                Ok(view) => self.episode_view = Some(view),
                Err(e) => {
                    ui.label(
                        RichText::new(format!("made it, but it could not open: {e}"))
                            .small()
                            .color(cl::ink::MUTED),
                    );
                }
            }
        }
        Some(done)
    }

    /// The EDITOR subroute: a picker until a card is opened, then the
    /// tap-to-edit editor. A back chip returns to the picker.
    fn editor_surface(&mut self, ui: &mut egui::Ui) {
        let root = dpl_roots::content_root();
        if self.editor.is_some() {
            let back = ui
                .horizontal(|ui| {
                    ui.label(RichText::new("‹ pick another slide").color(SPEC.tone.color()))
                        .interact(egui::Sense::click())
                })
                .inner;
            ui.pilot_track(
                &back,
                NodeMetadata::new(
                    PilotId::new("workbench.editor").child(&PilotKey::new("back")),
                    egui_pilot_ai::protocol::Role::Button,
                    "pick another slide",
                ),
            );
            if cl::tip(
                back,
                "Back to the slides — a show you picked from shows your change",
            )
            .clicked()
            {
                self.close_editor(&root);
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
        if let Some(note) = &self.picker_note {
            let line = ui.label(RichText::new(note).small().color(cl::ink::BASE));
            ui.pilot_track(
                &line,
                NodeMetadata::new(
                    PilotId::new("workbench.editor").child(&PilotKey::new("note")),
                    egui_pilot_ai::protocol::Role::Status,
                    note.clone(),
                ),
            );
        }
        ui.add_space(cl::space::S2);
        let groups = self.picker.get_or_insert_with(|| picker_groups(&root));
        let mut open: Option<PickerCard> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (g, group) in groups.iter().enumerate() {
                    if let Some(title) = &group.title {
                        ui.add_space(cl::space::S3);
                        ui.label(
                            RichText::new(title.to_uppercase())
                                .small()
                                .color(cl::ink::SUBTLE),
                        )
                        .on_hover_text(&group.badge);
                    }
                    for (c, card) in group.cards.iter().enumerate() {
                        let resp = cl::card_frame().show(ui, |ui| {
                            ui.set_width(ui.available_width().min(560.0));
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(&card.label).strong().color(cl::ink::STRONG),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let (words, tone) = if card.waiting {
                                            ("newer version waiting", cl::ink::SUBTLE)
                                        } else {
                                            ("edit ›", SPEC.tone.color())
                                        };
                                        ui.label(RichText::new(words).small().color(tone));
                                    },
                                );
                            });
                        });
                        let resp = resp.response.interact(egui::Sense::click());
                        ui.pilot_track(
                            &resp,
                            NodeMetadata::new(
                                PilotId::new("workbench.editor.pick")
                                    .child(&PilotKey::new(format!("{g}/{c}"))),
                                egui_pilot_ai::protocol::Role::Button,
                                card.label.clone(),
                            ),
                        );
                        let resp = cl::tip(
                            resp,
                            if card.waiting {
                                "This show still uses an older version of this card — compare \
                                 and update it on the Lessons page first"
                            } else {
                                "Edit this card"
                            },
                        );
                        if resp.clicked() && !card.waiting {
                            open = Some(card.clone());
                        }
                        ui.add_space(cl::space::S2);
                    }
                }
            });
        if let Some(card) = open {
            self.open_picked(card, &root, ui);
        }
    }

    /// Open a picked card in the editor, in the theme of the show it was
    /// picked from. Seeds are read-only starting points (LS2, ruled
    /// 2026-10-21): opening one makes the operator's own copy in the
    /// library first.
    fn open_picked(&mut self, card: PickerCard, root: &std::path::Path, ui: &mut egui::Ui) {
        let dir = if dpl_doc::is_seed_dir(&card.dir) {
            match dpl_doc::fork_into_library(&card.dir, root) {
                Ok(copy) => copy,
                Err(e) => {
                    ui.label(
                        RichText::new(format!("could not copy the starter: {e}"))
                            .color(cl::ink::MUTED),
                    );
                    return;
                }
            }
        } else {
            card.dir
        };
        match dpl_editor::Editor::open(&dir, root) {
            Ok(ed) => {
                let ed = ed.with_theme(card.theme);
                self.editing_from = card
                    .from
                    .map(|(episode, title)| (episode, title, ed.slide().rev));
                self.editor = Some(ed);
                self.picker_note = None;
            }
            Err(e) => {
                ui.label(RichText::new(format!("could not open: {e}")).color(cl::ink::MUTED));
            }
        }
    }

    /// Back to the picker. A card changed from an episode built from
    /// lessons is that episode's own change (ruled 2026-10-24, "Show it
    /// straight away"): its lessons take it; every other lesson asks.
    fn close_editor(&mut self, root: &std::path::Path) {
        let Some(ed) = self.editor.take() else {
            return;
        };
        if let Some((episode, title, was)) = self.editing_from.take() {
            let slide = ed.slide();
            if slide.rev != was {
                self.picker_note = Some(
                    match dpl_doc::episode::take_edit(&episode, root, &slide.id, slide.rev) {
                        Ok(_) => {
                            let ours: Vec<PathBuf> = dpl_doc::episode::lessons_of(&episode, root)
                                .into_iter()
                                .map(|(dir, _)| dir)
                                .collect();
                            let others = dpl_doc::lesson::lessons_using(root, &slide.id)
                                .into_iter()
                                .filter(|dir| !ours.contains(dir))
                                .count();
                            match others {
                                0 => format!("Your change shows in “{title}”."),
                                1 => format!(
                                    "Your change shows in “{title}”. 1 other lesson using this \
                                 card will ask first."
                                ),
                                n => format!(
                                    "Your change shows in “{title}”. {n} other lessons using \
                                 this card will ask first."
                                ),
                            }
                        }
                        Err(e) => {
                            format!("Your change is saved, but “{title}” could not take it: {e}")
                        }
                    },
                );
            }
        }
        self.picker = None;
    }
}

/// One card the editor picker offers.
#[derive(Clone)]
struct PickerCard {
    label: String,
    dir: PathBuf,
    /// The theme of the show it is edited from.
    theme: dpl_doc::Theme,
    /// The episode built from lessons it is edited from, and its title.
    from: Option<(PathBuf, String)>,
    /// The show still uses an older version than the card's newest — it is
    /// compared and updated on the Lessons page first, so an edit never
    /// carries a version the operator put off (the LS3a rule).
    waiting: bool,
}

/// The picker's sections: the seeds, then every episode (content first).
struct PickerGroup {
    title: Option<String>,
    badge: String,
    cards: Vec<PickerCard>,
}

/// Read the picker: each seed, then each episode through the one walk —
/// its cards in its own theme (LS3b; never inferred from a folder).
fn picker_groups(root: &std::path::Path) -> Vec<PickerGroup> {
    let mut groups = vec![PickerGroup {
        title: None,
        badge: "seeds".into(),
        cards: dpl_doc::list_seeds()
            .into_iter()
            .map(|s| PickerCard {
                label: format!("{} · {}", s.class, s.id),
                dir: s.dir,
                theme: dpl_doc::Theme::fallback(),
                from: None,
                waiting: false,
            })
            .collect(),
    }];
    for entry in dpl_doc::all_episodes(root) {
        let Ok(shown) = dpl_doc::episode::shows(&entry.dir, root) else {
            continue;
        };
        let built = matches!(
            dpl_doc::episode::open(&entry.dir),
            Ok(dpl_doc::episode::Opened::Lessons(_))
        );
        let badge = match &entry.show {
            Some(show) => format!("{show}/{}", entry.id),
            None => format!("template · {}", entry.id),
        };
        let mut cards = Vec::new();
        for chapter in &shown.chapters {
            for card in &chapter.cards {
                let Ok(slide) = &card.slide else {
                    continue;
                };
                let waiting = built
                    && dpl_doc::lesson::newest_rev(root, &slide.id)
                        .is_some_and(|newest| newest > slide.rev);
                cards.push(PickerCard {
                    label: format!("{} · {}", chapter.title, card.label),
                    dir: card.dir.clone(),
                    theme: shown.theme.clone(),
                    from: built.then(|| (entry.dir.clone(), shown.title.clone())),
                    waiting,
                });
            }
        }
        groups.push(PickerGroup {
            title: Some(shown.title.clone()),
            badge,
            cards,
        });
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A content root with an episode cloned from a scratch template that
    /// has its own theme.
    fn cloned(tag: &str) -> (PathBuf, PathBuf, dpl_doc::Theme) {
        let base = std::env::temp_dir().join(format!("dpl-wb-{tag}-{}", dpl_doc::new_id()));
        let seed = base.join("seed");
        let mut slide = dpl_doc::Slide::new("title@1");
        slide.blocks = vec![dpl_doc::Block::text("title", "Hello")];
        dpl_doc::save(&seed.join("a"), &slide).unwrap();
        dpl_doc::save_episode(
            &seed,
            &dpl_doc::Episode {
                v: dpl_doc::SCHEMA_V,
                id: "s".into(),
                title: "Seeded".into(),
                chapters: vec![dpl_doc::Chapter {
                    id: "one".into(),
                    title: "One".into(),
                    slides: vec!["a".into()],
                }],
            },
        )
        .unwrap();
        let mut theme = dpl_doc::Theme::fallback();
        theme.font = "michroma".into();
        dpl_doc::save_theme(&seed, &theme).unwrap();
        let root = base.join("content");
        let dir = dpl_doc::clone_episode(&seed, &root, "show", "ep").unwrap();
        (root, dir, theme)
    }

    /// The picker hands each card the theme of the show it is edited from,
    /// and remembers an episode built from lessons so it takes the edit.
    #[test]
    fn the_picker_carries_the_shows_theme_and_where_it_came_from() {
        let (root, dir, theme) = cloned("picker");
        let groups = picker_groups(&root);
        let card = groups
            .iter()
            .flat_map(|g| g.cards.iter())
            .find(|c| c.from.as_ref().is_some_and(|(ep, _)| ep == &dir))
            .expect("the cloned episode's card");
        assert_eq!(card.theme, theme);
        assert_eq!(card.label, "Seeded — One · Hello");
        assert!(!card.waiting);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    /// An edit made from an episode is that episode's own change (ruled
    /// 2026-10-24): its lesson takes it, and the note says another lesson
    /// using the card will ask first.
    #[test]
    fn an_edit_from_an_episode_shows_in_it_and_others_ask() {
        let (root, dir, _) = cloned("edit");
        let card = picker_groups(&root)
            .into_iter()
            .flat_map(|g| g.cards)
            .find(|c| c.from.is_some())
            .unwrap();
        let slide = dpl_doc::load(&card.dir).unwrap();
        let (other_dir, mut other) = dpl_doc::lesson::create_lesson(&root, "Elsewhere").unwrap();
        other.add(&slide);
        dpl_doc::lesson::save_lesson(&other_dir, &other).unwrap();

        let mut hub = WorkbenchHub::default();
        let mut editor = dpl_editor::Editor::open(&card.dir, &root)
            .unwrap()
            .with_theme(card.theme.clone());
        editor.set_enter(Some("none".into()));
        hub.editing_from = Some((dir.clone(), "Seeded".into(), 1));
        hub.editor = Some(editor);
        hub.close_editor(&root);

        assert_eq!(
            hub.picker_note.as_deref(),
            Some("Your change shows in “Seeded”. 1 other lesson using this card will ask first.")
        );
        let shown = dpl_doc::episode::shows(&dir, &root).unwrap();
        assert_eq!(shown.cards().next().unwrap().slide.as_ref().unwrap().rev, 2);
        let other = dpl_doc::lesson::load_lesson(&other_dir).unwrap();
        assert_eq!(
            dpl_doc::lesson::use_state(&root, &other.uses[0]),
            dpl_doc::lesson::UseState::UpdateAvailable { newest: 2 }
        );
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }
}
