//! The LESSONS subroute (LS3a, R285): your lessons, each a run of cards
//! from your library.
//!
//! A lesson never copies a card. Each place a card is shown is a USE that
//! points at the library card and keeps the version you chose; when the
//! card changes somewhere else, the lesson shows **Newer version** and asks
//! — Compare, then Update or Not now (a "Not now" is remembered for that
//! version). Editing a card from inside a lesson is this lesson's own
//! change, so the lesson takes it straight away; every other lesson using
//! the card asks first. Plan: docs/plans/lessons-ls3.md §4.3–4.4.

use crate::controls::{control, Weight};
use crate::SPEC;
use dpl_coldlight::{self as cl, Tone};
use dpl_doc::lesson::{self, Lesson, UseState};
use egui::RichText;
use egui_pilot_ai::protocol::Role;
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, PilotPath, UiPilotExt};
use std::path::{Path, PathBuf};

/// A use's job, in plain words — one per [`dpl_doc::roles`], in teaching
/// order (a test pins the two lists together).
const ROLE_WORDS: [(&str, &str); 9] = [
    ("attention", "Get attention"),
    ("objective", "Say what they'll learn"),
    ("recall", "Bring back what they know"),
    ("present", "Teach it"),
    ("guide", "Show how"),
    ("practice", "Let them try"),
    ("feedback", "Say how they did"),
    ("assess", "Check they've got it"),
    ("retain", "Make it stick"),
];

/// What "no job" reads as.
const NO_ROLE: &str = "No job yet";

/// How many library cards the add shelf shows at once.
const SHELF_MAX: usize = 8;

/// How long ago a miniature's entrance started — long enough that every
/// entrance has finished.
const STILL_AFTER: f64 = 10.0;

/// The most cards across a row on the lesson page, and on the add shelf.
const CARDS_ACROSS: f32 = 3.0;
const SHELF_ACROSS: f32 = 5.0;

fn role_words(role: Option<&str>) -> &'static str {
    role.and_then(|r| ROLE_WORDS.iter().find(|(id, _)| *id == r))
        .map_or(NO_ROLE, |(_, words)| words)
}

/// This page's pilot node `key`.
fn node(key: &str) -> PilotPath {
    PilotId::new("workbench.lessons").child(&PilotKey::new(key))
}

/// One lesson on the list page.
struct Listed {
    dir: PathBuf,
    title: String,
    count: usize,
    waiting: usize,
    broken: Option<String>,
}

/// One use, ready to draw.
struct Card {
    key: String,
    state: UseState,
    pinned: u32,
    role: Option<String>,
    label: String,
    /// The pinned version, drawn as a still (this lesson's variant applied).
    slide: Option<dpl_doc::Slide>,
    /// This lesson lays the card out its own way (LS4a).
    own_layout: bool,
    /// How many taps the card has (its reveals) — RL.
    taps: usize,
    /// Its teleprompter script: none yet, or how many taps still have no
    /// words.
    script: Option<usize>,
}

/// What a card's taps and script come to, in a few words: "2 taps ·
/// script", "1 tap · script, 1 to write", "no script yet".
fn taps_words(taps: usize, script: Option<usize>) -> String {
    let taps = match taps {
        0 => "no taps".to_owned(),
        1 => "1 tap".to_owned(),
        n => format!("{n} taps"),
    };
    match script {
        None => format!("{taps} · no script yet"),
        Some(0) => format!("{taps} · script"),
        Some(n) => format!("{taps} · script, {n} to write"),
    }
}

/// The side-by-side sheet before an update.
struct Compare {
    key: String,
    label: String,
    pinned: u32,
    newest: u32,
    before: Option<dpl_doc::Slide>,
    after: Option<dpl_doc::Slide>,
}

/// One library card the add shelf offers.
struct ShelfCard {
    slide: dpl_doc::Slide,
    label: String,
}

/// An open lesson.
struct LessonView {
    dir: PathBuf,
    lesson: Lesson,
    /// The last save was refused; the lesson waits in memory (ST2b).
    unsaved: bool,
    /// The name field's text.
    title: String,
    cards: Vec<Card>,
    compare: Option<Compare>,
    /// The library cards on offer, while the add shelf is open.
    shelf: Option<(Vec<ShelfCard>, usize)>,
    /// The use whose job is being chosen.
    picking: Option<String>,
    /// The use whose layout is being chosen for this lesson only (LS4a).
    shaping: Option<String>,
    /// A card being changed from inside the lesson: the revision it had,
    /// and the editor.
    editing: Option<(u32, dpl_editor::Editor)>,
    /// Run asked for this frame: the card to start at (RL).
    run: Option<usize>,
    /// The card being changed shows its On tap panel (RL).
    taps_open: bool,
    /// What every card here paints against: the looks, and the fallback
    /// theme — a lesson has no show of its own; an episode's theme reaches
    /// its slides in LS3b.
    styling: dpl_doc::Styling,
}

#[derive(Default)]
pub(crate) struct Lessons {
    view: Option<LessonView>,
    /// The list, read when the page opens and after each change.
    listed: Option<Vec<Listed>>,
    /// The name being typed for a new lesson.
    naming: Option<String>,
    status: String,
}

impl LessonView {
    fn open(dir: &Path, root: &Path) -> Result<Self, String> {
        let lesson = lesson::load_lesson(dir).map_err(|e| e.to_string())?;
        let mut view = Self {
            dir: dir.to_path_buf(),
            unsaved: false,
            title: lesson.title.clone(),
            lesson,
            cards: Vec::new(),
            compare: None,
            shelf: None,
            picking: None,
            shaping: None,
            editing: None,
            run: None,
            taps_open: false,
            styling: dpl_doc::Styling {
                theme: dpl_doc::Theme::fallback(),
                looks: dpl_doc::Looks::load(root),
            },
        };
        view.refresh(root);
        Ok(view)
    }

    /// Re-read every use's state and pinned card.
    fn refresh(&mut self, root: &Path) {
        self.cards = self
            .lesson
            .uses
            .iter()
            .map(|use_| {
                let slide = lesson::load_use(root, use_).ok();
                let script =
                    dpl_doc::script::load_script(&dpl_doc::library_slide_dir(root, &use_.slide.id))
                        .ok()
                        .flatten()
                        .map(|s| s.beats.iter().filter(|b| b.cue.trim().is_empty()).count());
                Card {
                    taps: slide.as_ref().map_or(0, |s| s.reveals.len()),
                    script,
                    key: use_.key.clone(),
                    state: lesson::use_state(root, use_),
                    pinned: use_.slide.rev,
                    role: use_.role.clone(),
                    label: slide
                        .as_ref()
                        .map_or_else(|| "A missing card".to_owned(), dpl_doc::Slide::label),
                    slide,
                    own_layout: !use_.variant.is_empty(),
                }
            })
            .collect();
    }

    /// Save the lesson after a change and redraw it. Returns what went
    /// wrong, if anything.
    fn commit(&mut self, root: &Path) -> Option<String> {
        let saved = lesson::save_lesson(&self.dir, &self.lesson).err();
        self.unsaved = saved.is_some();
        self.refresh(root);
        saved.map(|e| format!("Could not save the lesson: {e}"))
    }

    fn open_shelf(&mut self, root: &Path) {
        let all = dpl_doc::library_slides(root);
        let more = all.len().saturating_sub(SHELF_MAX);
        let cards = all
            .into_iter()
            .take(SHELF_MAX)
            .filter_map(|dir| dpl_doc::load(&dir).ok())
            .map(|slide| ShelfCard {
                label: slide.label(),
                slide,
            })
            .collect();
        self.shelf = Some((cards, more));
    }

    fn open_compare(&mut self, root: &Path, key: &str, newest: u32) {
        let Some(use_) = self.lesson.get(key) else {
            return;
        };
        let dir = dpl_doc::library_slide_dir(root, &use_.slide.id);
        let before = dpl_doc::load_rev(&dir, use_.slide.rev).ok();
        let after = dpl_doc::load_rev(&dir, newest).ok();
        self.compare = Some(Compare {
            key: key.to_owned(),
            label: before
                .as_ref()
                .or(after.as_ref())
                .map_or_else(|| "This card".to_owned(), dpl_doc::Slide::label),
            pinned: use_.slide.rev,
            newest,
            before,
            after,
        });
    }
}

/// Read the list page's lessons.
fn list(root: &Path) -> Vec<Listed> {
    lesson::list_lessons(root)
        .into_iter()
        .map(|(dir, found)| match found {
            Ok(l) => Listed {
                waiting: lesson::updates(root, &l).len(),
                count: l.uses.len(),
                title: l.title,
                broken: None,
                dir,
            },
            Err(why) => Listed {
                title: dir
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                count: 0,
                waiting: 0,
                broken: Some(why),
                dir,
            },
        })
        .collect()
}

/// What the last change did, in words — under the page's header, where it
/// is seen without scrolling. Shown from the frame after the change.
fn status_line(ui: &mut egui::Ui, status: &str) {
    if status.is_empty() {
        return;
    }
    let fluid = cl::Fluid::of(ui.max_rect());
    let line = ui.label(
        RichText::new(status)
            .size(fluid.fs_body_sm())
            .color(cl::ink::BASE),
    );
    ui.pilot_track(
        &line,
        NodeMetadata::new(node("status"), Role::Status, status),
    );
}

/// A tracked control with its tooltip.
fn button(
    ui: &mut egui::Ui,
    key: &str,
    label: &str,
    selected: bool,
    tone: Tone,
    weight: Weight,
    tip: &str,
) -> bool {
    let resp = cl::tip(control(ui, label, selected, tone, weight), tip);
    ui.pilot_track(&resp, NodeMetadata::new(node(key), Role::Button, label));
    resp.clicked()
}

/// A card drawn as a miniature in a fixed-size well — a still frame, the
/// entrance long played (the editor's before/after sheet does the same), so
/// a change to the lesson never flashes every card blank while it replays.
fn miniature(
    ui: &mut egui::Ui,
    w: f32,
    slide: Option<&dpl_doc::Slide>,
    styling: &dpl_doc::Styling,
) -> egui::Response {
    let h = w * 9.0 / 16.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    ui.painter()
        .rect_filled(rect, cl::RADIUS_SM, cl::surface::WELL);
    if let Some(slide) = slide {
        let now = ui.input(|i| i.time);
        let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        inner.set_clip_rect(rect.intersect(ui.clip_rect()));
        let _ = dpl_slides::document::render_thumbnail(
            &mut inner,
            rect,
            slide,
            styling,
            now,
            now - STILL_AFTER,
        );
    }
    resp
}

/// How wide a card is when `most` fit across the room, never narrower than
/// a thumbnail and a half nor wider than a panel. The frame's own margin
/// and edge come off first, so a row of them fills the width exactly.
fn tile_width(ui: &egui::Ui, most: f32) -> f32 {
    let fluid = cl::Fluid::of(ui.max_rect());
    let frame = cl::card_frame();
    let chrome = frame.inner_margin.sum().x
        + frame.stroke.width * 2.0
        + ui.spacing().item_spacing.x
        + cl::space::S2;
    let least = fluid.ctl_thumb_w() * 1.6;
    let room = ui.available_width();
    let across = (room / (least + chrome)).floor().clamp(1.0, most);
    (room / across - chrome).clamp(least, fluid.panel_width().max(least))
}

impl Lessons {
    /// The library is back (ST2b): the open lesson and the card being
    /// changed in it save again if their last save was refused.
    pub(crate) fn save_again(&mut self, root: &Path) {
        let Some(view) = self.view.as_mut() else {
            return;
        };
        if let Some((_, editor)) = view.editing.as_mut() {
            editor.save_again();
        }
        if view.unsaved {
            self.status = view.commit(root).unwrap_or_default();
        }
    }

    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, fabric: &Option<dpl_fabric::Fabric>) {
        let root = dpl_roots::content_root();
        self.ui_in(ui, &root);
        // Run (RL): the Present hub runs the lesson; the shell goes there.
        if let (Some((id, start)), Some(fabric)) = (self.take_run(), fabric) {
            fabric.publish_deck_command(dpl_fabric::DeckCommand::Run {
                target: dpl_fabric::RunTarget::Lesson(id),
                start,
            });
            fabric.publish_shell(dpl_fabric::ShellEvent::NavigateRequest {
                route: dpl_hub::Route {
                    hub: dpl_hub::HubId("present"),
                    sub: dpl_hub::SubId("shows"),
                },
            });
        }
    }

    /// A run asked for on this frame: the lesson's id and the card to start
    /// at. Asking twice before it is taken keeps the latest.
    fn take_run(&mut self) -> Option<(String, usize)> {
        let view = self.view.as_mut()?;
        let start = view.run.take()?;
        Some((view.lesson.id.clone(), start))
    }

    /// The page against an explicit content root (tests point it at a
    /// scratch folder).
    fn ui_in(&mut self, ui: &mut egui::Ui, root: &Path) {
        if self.view.as_ref().is_some_and(|v| v.editing.is_some()) {
            self.editing_ui(ui, root);
            return;
        }
        if self.view.is_some() {
            self.lesson_ui(ui, root);
        } else {
            self.list_ui(ui, root);
        }
    }

    /// Open a lesson — the list's tap.
    pub(crate) fn open(&mut self, dir: &Path, root: &Path) {
        match LessonView::open(dir, root) {
            Ok(view) => {
                self.view = Some(view);
                self.status.clear();
            }
            Err(e) => self.status = format!("Could not open that lesson: {e}"),
        }
    }

    fn make_lesson(&mut self, name: &str, root: &Path) {
        match lesson::create_lesson(root, name) {
            Ok((dir, _)) => {
                self.naming = None;
                self.listed = None;
                self.open(&dir, root);
            }
            Err(e) => self.status = format!("Could not make the lesson: {e}"),
        }
    }

    // ── the list ────────────────────────────────────────────────────────

    fn list_ui(&mut self, ui: &mut egui::Ui, root: &Path) {
        let fluid = cl::Fluid::of(ui.max_rect());
        let listed = self.listed.get_or_insert_with(|| list(root));
        let mut open: Option<PathBuf> = None;

        ui.label(
            RichText::new("WORKBENCH · LESSONS")
                .size(fluid.fs_eyebrow())
                .color(cl::ink::MUTED),
        );
        ui.label(
            RichText::new("Your lessons")
                .heading()
                .color(cl::ink::STRONG),
        );
        ui.label(
            RichText::new(
                "A lesson is a run of cards from your library. It keeps the version of each \
                 card you chose — when a card changes, the lesson asks before it shows the \
                 change.",
            )
            .small()
            .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S3);

        let mut make: Option<String> = None;
        match self.naming.as_mut() {
            None => {
                if button(
                    ui,
                    "new",
                    "+ New lesson",
                    false,
                    SPEC.tone,
                    Weight::Hero,
                    "Make a new, empty lesson and name it",
                ) {
                    self.naming = Some(String::new());
                }
            }
            Some(name) => {
                let mut cancel = false;
                ui.horizontal_wrapped(|ui| {
                    let field = ui.add(
                        egui::TextEdit::singleline(name)
                            .hint_text("Name the lesson")
                            .desired_width(fluid.ctl_thumb_w() * 2.5),
                    );
                    ui.pilot_track(
                        &field,
                        NodeMetadata::new(node("name"), Role::TextInput, "Name the lesson"),
                    );
                    let field = cl::tip(field, "What the lesson is called");
                    if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        make = Some(name.clone());
                    }
                    if button(
                        ui,
                        "create",
                        "Make it",
                        false,
                        SPEC.tone,
                        Weight::Hero,
                        "Make the lesson and open it",
                    ) {
                        make = Some(name.clone());
                    }
                    if button(
                        ui,
                        "cancel",
                        "Cancel",
                        false,
                        SPEC.tone,
                        Weight::Chrome,
                        "Don't make a lesson",
                    ) {
                        cancel = true;
                    }
                });
                if cancel {
                    self.naming = None;
                }
            }
        }
        status_line(ui, &self.status);
        ui.add_space(cl::space::S3);

        if listed.is_empty() {
            ui.label(
                RichText::new("No lessons yet — make one and add cards from your library.")
                    .size(fluid.fs_body())
                    .color(cl::ink::MUTED),
            );
        }
        egui::ScrollArea::vertical()
            .id_salt("workbench.lessons.list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for item in listed.iter() {
                    let resp = cl::card_frame()
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width().min(fluid.panel_width()));
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(&item.title)
                                        .size(fluid.fs_body_lg())
                                        .strong()
                                        .color(cl::ink::STRONG),
                                );
                                match &item.broken {
                                    Some(why) => {
                                        ui.label(
                                            RichText::new(format!("can't be read: {why}"))
                                                .size(fluid.fs_body_sm())
                                                .color(Tone::Coral.color()),
                                        );
                                    }
                                    None => {
                                        ui.label(
                                            RichText::new(match item.count {
                                                1 => "1 card".to_owned(),
                                                n => format!("{n} cards"),
                                            })
                                            .size(fluid.fs_body_sm())
                                            .color(cl::ink::MUTED),
                                        );
                                        if item.waiting > 0 {
                                            ui.label(
                                                RichText::new(match item.waiting {
                                                    1 => "· 1 card has a newer version".to_owned(),
                                                    n => {
                                                        format!("· {n} cards have a newer version")
                                                    }
                                                })
                                                .size(fluid.fs_body_sm())
                                                .color(Tone::Amber.color()),
                                            );
                                        }
                                    }
                                }
                            });
                        })
                        .response
                        .interact(egui::Sense::click());
                    ui.pilot_track(
                        &resp,
                        NodeMetadata::new(
                            PilotId::new("workbench.lessons.lesson")
                                .child(&PilotKey::new(&item.title)),
                            Role::Button,
                            &item.title,
                        ),
                    );
                    if item.broken.is_none() && cl::tip(resp, "Open this lesson").clicked() {
                        open = Some(item.dir.clone());
                    }
                    ui.add_space(cl::space::S2);
                }
            });

        if let Some(name) = make {
            self.make_lesson(&name, root);
        } else if let Some(dir) = open {
            self.open(&dir, root);
        }
    }

    // ── one lesson ──────────────────────────────────────────────────────

    fn lesson_ui(&mut self, ui: &mut egui::Ui, root: &Path) {
        let fluid = cl::Fluid::of(ui.max_rect());
        let Some(view) = self.view.as_mut() else {
            return;
        };
        let mut back = false;
        let mut note: Option<String> = None;

        ui.horizontal_wrapped(|ui| {
            if button(
                ui,
                "back",
                "‹ All lessons",
                false,
                SPEC.tone,
                Weight::Chrome,
                "Back to your lessons",
            ) {
                back = true;
            }
            ui.add_space(cl::space::S2);
            let field = ui.add(
                egui::TextEdit::singleline(&mut view.title)
                    .font(egui::FontId::proportional(fluid.fs_h4()))
                    .desired_width(fluid.ctl_thumb_w() * 3.0),
            );
            ui.pilot_track(
                &field,
                NodeMetadata::new(node("title"), Role::TextInput, "The lesson's name"),
            );
            let field = cl::tip(field, "The lesson's name — type to rename it");
            if field.lost_focus() {
                if view.lesson.rename(&view.title) {
                    note = view.commit(root);
                } else {
                    view.title.clone_from(&view.lesson.title);
                }
            }
            ui.label(
                RichText::new(match view.lesson.uses.len() {
                    1 => "1 card".to_owned(),
                    n => format!("{n} cards"),
                })
                .size(fluid.fs_body_sm())
                .color(cl::ink::MUTED),
            );
            if !view.lesson.uses.is_empty() {
                ui.add_space(cl::space::S3);
                if button(
                    ui,
                    "run",
                    "▶ Run this lesson",
                    false,
                    SPEC.tone,
                    Weight::Hero,
                    "Show this lesson full screen, with your script on the other screen",
                ) {
                    view.run = Some(0);
                }
            }
        });
        status_line(ui, &self.status);
        ui.add_space(cl::space::S2);

        if view.compare.is_some() {
            note = Self::compare_ui(view, ui, root).or(note);
        } else {
            note = Self::cards_ui(view, ui, root).or(note);
        }

        if let Some(note) = note {
            self.status = note;
            ui.ctx().request_repaint();
        }
        if back {
            self.view = None;
            self.listed = None;
            self.status.clear();
        }
    }

    /// The lesson's cards, the add shelf and Update all.
    fn cards_ui(view: &mut LessonView, ui: &mut egui::Ui, root: &Path) -> Option<String> {
        let fluid = cl::Fluid::of(ui.max_rect());
        let waiting = view
            .cards
            .iter()
            .filter(|c| matches!(c.state, UseState::UpdateAvailable { .. }))
            .count();
        let mut note = None;
        let mut act: Option<Act> = None;

        ui.horizontal_wrapped(|ui| {
            let adding = view.shelf.is_some();
            if button(
                ui,
                "add",
                if adding {
                    "Done adding"
                } else {
                    "+ Add a card"
                },
                adding,
                SPEC.tone,
                if waiting > 0 {
                    Weight::Chrome
                } else {
                    Weight::Hero
                },
                if adding {
                    "Close your library"
                } else {
                    "Add a card from your library — the lesson points at it, nothing is copied"
                },
            ) {
                act = Some(Act::ToggleShelf);
            }
            if waiting > 0 {
                ui.add_space(cl::space::S3);
                let words = match waiting {
                    1 => "1 card has a newer version".to_owned(),
                    n => format!("{n} cards have a newer version"),
                };
                ui.label(
                    RichText::new(&words)
                        .size(fluid.fs_body_sm())
                        .color(Tone::Amber.color()),
                );
                if button(
                    ui,
                    "update-all",
                    &format!("Update all ({waiting})"),
                    false,
                    Tone::Amber,
                    Weight::Hero,
                    "Show the newest version of every card that changed",
                ) {
                    act = Some(Act::UpdateAll);
                }
            }
        });
        ui.add_space(cl::space::S2);

        egui::ScrollArea::vertical()
            .id_salt("workbench.lessons.view")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(key) = &view.picking {
                    let card = view.cards.iter().find(|c| &c.key == key);
                    Self::role_picker_ui(ui, key, card, &mut act);
                    ui.add_space(cl::space::S3);
                }
                if let Some(key) = &view.shaping {
                    let card = view.cards.iter().find(|c| &c.key == key);
                    let own = view
                        .lesson
                        .get(key)
                        .map(|u| u.variant.clone())
                        .unwrap_or_default();
                    Self::layout_picker_ui(ui, key, card, &own, &mut act);
                    ui.add_space(cl::space::S3);
                }
                if let Some((shelf, more)) = &view.shelf {
                    Self::shelf_ui(ui, shelf, *more, &view.styling, &mut act);
                    ui.add_space(cl::space::S3);
                }
                if view.cards.is_empty() {
                    ui.label(
                        RichText::new("Nothing in this lesson yet — add a card from your library.")
                            .size(fluid.fs_body())
                            .color(cl::ink::MUTED),
                    );
                }
                let last = view.cards.len().saturating_sub(1);
                let tile_w = tile_width(ui, CARDS_ACROSS);
                ui.horizontal_wrapped(|ui| {
                    for (ix, card) in view.cards.iter().enumerate() {
                        Self::card_ui(ui, card, ix, last, tile_w, &view.styling, &mut act);
                        ui.add_space(cl::space::S2);
                    }
                });
            });

        match act {
            None => {}
            Some(Act::Run(ix)) => view.run = Some(ix),
            Some(Act::CancelRole) => view.picking = None,
            Some(Act::ToggleShelf) => {
                if view.shelf.is_some() {
                    view.shelf = None;
                } else {
                    view.open_shelf(root);
                }
            }
            Some(Act::Add(ix)) => {
                let picked = view
                    .shelf
                    .as_ref()
                    .and_then(|(shelf, _)| shelf.get(ix))
                    .map(|c| (c.slide.clone(), c.label.clone()));
                if let Some((slide, label)) = picked {
                    view.lesson.add(&slide);
                    note = view
                        .commit(root)
                        .or_else(|| Some(format!("Added “{label}”.")));
                }
            }
            Some(Act::UpdateAll) => {
                let n = lesson::update_all(root, &mut view.lesson);
                note = view.commit(root).or_else(|| {
                    Some(match n {
                        1 => "1 card shows its newest version now.".to_owned(),
                        n => format!("{n} cards show their newest version now."),
                    })
                });
            }
            Some(Act::Compare(key, newest)) => view.open_compare(root, &key, newest),
            Some(Act::Move(key, to)) => {
                view.lesson.move_to(&key, to);
                note = view.commit(root);
            }
            Some(Act::Remove(key)) => {
                if view.lesson.remove(&key) {
                    note = view
                        .commit(root)
                        .or_else(|| Some("Taken out — the card is still in your library.".into()));
                }
            }
            Some(Act::PickRole(key)) => view.picking = Some(key),
            Some(Act::PickLayout(key)) => view.shaping = Some(key),
            Some(Act::DoneLayout) => view.shaping = None,
            Some(Act::Variant(key, axis, option)) => {
                view.lesson.set_variant(&key, &axis, option.as_deref());
                note = view.commit(root);
            }
            Some(Act::Role(key, role)) => {
                view.picking = None;
                view.lesson.set_role(&key, role.as_deref());
                note = view.commit(root);
            }
            Some(Act::Edit(key)) => {
                let found = view.lesson.get(&key).map(|u| u.slide.clone());
                if let Some(pin) = found {
                    let dir = dpl_doc::library_slide_dir(root, &pin.id);
                    match dpl_editor::Editor::open(&dir, root) {
                        Ok(editor) => view.editing = Some((editor.slide().rev, editor)),
                        Err(e) => note = Some(format!("Could not open that card: {e}")),
                    }
                }
            }
        }
        note
    }

    /// How a card is laid out in this lesson only (LS4a, ruled 2026-10-24:
    /// "a one off variant for that specific lesson"): each of its
    /// component's variant axes as chips, the lesson's own choice marked,
    /// and "Same as the card" to give a choice back. The card itself never
    /// changes.
    fn layout_picker_ui(
        ui: &mut egui::Ui,
        key: &str,
        card: Option<&Card>,
        own: &std::collections::BTreeMap<String, String>,
        act: &mut Option<Act>,
    ) {
        let fluid = cl::Fluid::of(ui.max_rect());
        let label = card.map_or("this card", |c| c.label.as_str());
        let Some((slide, contract)) = card
            .and_then(|c| c.slide.as_ref())
            .and_then(|slide| Some((slide, dpl_slides::contract_of(slide.component_id())?)))
        else {
            return;
        };
        ui.label(
            RichText::new(format!("How “{label}” is laid out in this lesson"))
                .size(fluid.fs_body_lg())
                .strong()
                .color(cl::ink::STRONG),
        );
        ui.label(
            RichText::new("Only this lesson changes — the card stays as it is everywhere else.")
                .size(fluid.fs_body_sm())
                .color(cl::ink::MUTED),
        );
        for axis in &contract.variant_axes {
            let current = contract.axis_value(&slide.variant, &axis.id).unwrap_or("");
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{}{}",
                        axis.label,
                        if own.contains_key(&axis.id) {
                            " (this lesson)"
                        } else {
                            ""
                        }
                    ))
                    .size(fluid.fs_body_sm())
                    .color(cl::ink::BASE),
                );
                for option in &axis.options {
                    let on = current == option.id;
                    if button(
                        ui,
                        &format!("layout/{}/{}", axis.id, option.id),
                        &option.label,
                        on,
                        SPEC.tone,
                        Weight::Chrome,
                        &format!(
                            "{} in this lesson: {}",
                            axis.label,
                            option.label.to_lowercase()
                        ),
                    ) && !on
                    {
                        *act = Some(Act::Variant(
                            key.to_owned(),
                            axis.id.clone(),
                            Some(option.id.clone()),
                        ));
                    }
                }
                if own.contains_key(&axis.id)
                    && button(
                        ui,
                        &format!("layout/{}/same", axis.id),
                        "Same as the card",
                        false,
                        SPEC.tone,
                        Weight::Chrome,
                        "Give this choice back to the card",
                    )
                {
                    *act = Some(Act::Variant(key.to_owned(), axis.id.clone(), None));
                }
            });
        }
        if button(
            ui,
            "layout/done",
            "Done",
            false,
            SPEC.tone,
            Weight::Chrome,
            "Close the layout choices",
        ) {
            *act = Some(Act::DoneLayout);
        }
    }

    /// The nine jobs (and none) as chips: what does this card do here?
    fn role_picker_ui(ui: &mut egui::Ui, key: &str, card: Option<&Card>, act: &mut Option<Act>) {
        let fluid = cl::Fluid::of(ui.max_rect());
        let label = card.map_or("this card", |c| c.label.as_str());
        let current = card.and_then(|c| c.role.as_deref());
        ui.label(
            RichText::new(format!("What does “{label}” do in this lesson?"))
                .size(fluid.fs_body_lg())
                .strong()
                .color(cl::ink::STRONG),
        );
        ui.horizontal_wrapped(|ui| {
            let choices = ROLE_WORDS
                .iter()
                .map(|(id, words)| (Some(*id), *words))
                .chain(std::iter::once((None, NO_ROLE)));
            for (id, words) in choices {
                let on = current == id;
                if button(
                    ui,
                    &format!("role/{}", id.unwrap_or("none")),
                    words,
                    on,
                    SPEC.tone,
                    Weight::Chrome,
                    &format!("Give “{label}” this job: {}", words.to_lowercase()),
                ) {
                    *act = Some(if on {
                        Act::CancelRole
                    } else {
                        Act::Role(key.to_owned(), id.map(str::to_owned))
                    });
                }
            }
            if button(
                ui,
                "role/cancel",
                "Cancel",
                false,
                SPEC.tone,
                Weight::Chrome,
                "Leave the job as it is",
            ) {
                *act = Some(Act::CancelRole);
            }
        });
    }

    /// The library cards on offer; tap one to add it.
    fn shelf_ui(
        ui: &mut egui::Ui,
        shelf: &[ShelfCard],
        more: usize,
        styling: &dpl_doc::Styling,
        act: &mut Option<Act>,
    ) {
        let fluid = cl::Fluid::of(ui.max_rect());
        ui.label(
            RichText::new("YOUR LIBRARY — tap a card to add it")
                .size(fluid.fs_eyebrow())
                .color(cl::ink::MUTED),
        );
        if shelf.is_empty() {
            ui.label(
                RichText::new("Your library is empty. Start a card in Author and it lands here.")
                    .size(fluid.fs_body_sm())
                    .color(cl::ink::MUTED),
            );
            return;
        }
        let tile_w = tile_width(ui, SHELF_ACROSS);
        ui.horizontal_wrapped(|ui| {
            for (ix, card) in shelf.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.set_width(tile_w);
                    let resp = miniature(ui, tile_w, Some(&card.slide), styling);
                    ui.pilot_track(
                        &resp,
                        NodeMetadata::new(
                            PilotId::new("workbench.lessons.shelf")
                                .child(&PilotKey::new(ix.to_string())),
                            Role::Button,
                            &card.label,
                        ),
                    );
                    if cl::tip(resp, "Add this card to the lesson").clicked() {
                        *act = Some(Act::Add(ix));
                    }
                    ui.label(
                        RichText::new(&card.label)
                            .size(fluid.fs_body_sm())
                            .color(cl::ink::BASE),
                    );
                });
                ui.add_space(cl::space::S2);
            }
        });
        if more > 0 {
            ui.label(
                RichText::new(format!("…and {more} older cards"))
                    .size(fluid.fs_body_sm())
                    .color(cl::ink::SUBTLE),
            );
        }
    }

    /// One use: the card as the lesson shows it, its job, its version and
    /// what you can do with it.
    fn card_ui(
        ui: &mut egui::Ui,
        card: &Card,
        ix: usize,
        last: usize,
        tile_w: f32,
        styling: &dpl_doc::Styling,
        act: &mut Option<Act>,
    ) {
        let fluid = cl::Fluid::of(ui.max_rect());
        let key = card.key.clone();
        let k = |what: &str| format!("use/{key}/{what}");
        cl::card_frame().show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(tile_w);
                let resp = miniature(ui, tile_w, card.slide.as_ref(), styling);
                ui.pilot_track(
                    &resp,
                    NodeMetadata::new(node(&format!("use/{key}")), Role::Image, &card.label)
                        .value(format!("version {}", card.pinned)),
                );
                cl::tip(resp, &format!("“{}” as this lesson shows it", card.label));
                ui.label(
                    RichText::new(format!("{}. {}", ix + 1, card.label))
                        .size(fluid.fs_body_sm())
                        .strong()
                        .color(cl::ink::STRONG),
                );
                // What happens on its taps, and whether its words are
                // written (RL) — set on its On tap panel.
                let taps = ui.label(
                    RichText::new(taps_words(card.taps, card.script))
                        .size(fluid.fs_body_sm())
                        // Amber only for words still to write; a card with
                        // no taps is not unfinished.
                        .color(if card.script.is_some_and(|missing| missing > 0) {
                            Tone::Amber.color()
                        } else {
                            cl::ink::MUTED
                        }),
                );
                cl::tip(
                    taps,
                    "How many taps this card has, and whether its teleprompter words are written — Edit › On tap",
                );

                // Where it stands against the card's newest version.
                match card.state {
                    UseState::Current => {}
                    UseState::UpdateAvailable { newest } => {
                        if button(
                            ui,
                            &k("compare"),
                            "Newer version",
                            true,
                            Tone::Amber,
                            Weight::Chrome,
                            "This card changed since you added it — compare the two and choose",
                        ) {
                            *act = Some(Act::Compare(key.clone(), newest));
                        }
                    }
                    UseState::NotNow { newest } => {
                        ui.label(
                            RichText::new(format!("Kept yours — version {newest} waits"))
                                .size(fluid.fs_body_sm())
                                .color(cl::ink::SUBTLE),
                        );
                        if button(
                            ui,
                            &k("compare"),
                            "Compare",
                            false,
                            SPEC.tone,
                            Weight::Chrome,
                            "Look at the newer version again",
                        ) {
                            *act = Some(Act::Compare(key.clone(), newest));
                        }
                    }
                    UseState::Missing => {
                        ui.label(
                            RichText::new("This card isn't in your library any more")
                                .size(fluid.fs_body_sm())
                                .color(Tone::Coral.color()),
                        );
                    }
                }

                // Its job in the lesson — the picker opens above the cards —
                // and Run from here beside it (RL), so the card's bottom row
                // keeps its four controls on the narrowest window.
                ui.horizontal(|ui| {
                    if button(
                        ui,
                        &k("role"),
                        &format!("Job: {}", role_words(card.role.as_deref())),
                        card.role.is_some(),
                        SPEC.tone,
                        Weight::Chrome,
                        "Choose the job this card does in the lesson",
                    ) {
                        *act = Some(Act::PickRole(key.clone()));
                    }
                    if button(
                        ui,
                        &k("run"),
                        "▶ Run",
                        false,
                        SPEC.tone,
                        Weight::Chrome,
                        "Run the lesson starting with this card",
                    ) {
                        *act = Some(Act::Run(ix));
                    }
                });
                // How it is laid out here, when its component offers a choice.
                let offers = card
                    .slide
                    .as_ref()
                    .and_then(|s| dpl_slides::contract_of(s.component_id()))
                    .is_some_and(|c| !c.variant_axes.is_empty());
                if offers
                    && button(
                        ui,
                        &k("layout"),
                        if card.own_layout {
                            "Layout: this lesson's"
                        } else {
                            "Layout"
                        },
                        card.own_layout,
                        SPEC.tone,
                        Weight::Chrome,
                        "Change how this card is laid out in this lesson only",
                    )
                {
                    *act = Some(Act::PickLayout(key.clone()));
                }

                ui.horizontal_wrapped(|ui| {
                    // Four controls side by side on the narrowest card: the
                    // gap between them is the token's smallest.
                    ui.spacing_mut().item_spacing.x = cl::space::S1;
                    if ix > 0
                        && button(
                            ui,
                            &k("earlier"),
                            "◀",
                            false,
                            SPEC.tone,
                            Weight::Chrome,
                            "Show this card one place earlier",
                        )
                    {
                        *act = Some(Act::Move(key.clone(), ix - 1));
                    }
                    if ix < last
                        && button(
                            ui,
                            &k("later"),
                            "▶",
                            false,
                            SPEC.tone,
                            Weight::Chrome,
                            "Show this card one place later",
                        )
                    {
                        *act = Some(Act::Move(key.clone(), ix + 1));
                    }
                    if card.state == UseState::Current
                        && button(
                            ui,
                            &k("edit"),
                            "Edit",
                            false,
                            SPEC.tone,
                            Weight::Chrome,
                            "Change this card — this lesson shows your change straight away",
                        )
                    {
                        *act = Some(Act::Edit(key.clone()));
                    }
                    if button(
                        ui,
                        &k("remove"),
                        "Take out",
                        false,
                        SPEC.tone,
                        Weight::Danger,
                        "Take this card out of the lesson — it stays in your library",
                    ) {
                        *act = Some(Act::Remove(key.clone()));
                    }
                });
            });
        });
    }

    /// The version this lesson shows beside the newest, then Update, Not
    /// now or Back.
    fn compare_ui(view: &mut LessonView, ui: &mut egui::Ui, root: &Path) -> Option<String> {
        let fluid = cl::Fluid::of(ui.max_rect());
        let compare = view.compare.as_ref()?;
        let mut decision: Option<Option<bool>> = None;

        ui.label(
            RichText::new(format!("“{}” changed", compare.label))
                .size(fluid.fs_h4())
                .strong()
                .color(cl::ink::STRONG),
        );
        ui.label(
            RichText::new(
                "This lesson still shows the version you added. Look at the two, then choose.",
            )
            .small()
            .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S2);
        // Two side by side, and short enough that the choice under them
        // stays on screen.
        let w = ((ui.available_width() - cl::space::S4) / 2.0)
            .min(ui.available_height() * 0.45 * 16.0 / 9.0)
            .min(fluid.panel_width())
            .max(fluid.ctl_thumb_w());
        let styling = &view.styling;
        ui.horizontal_wrapped(|ui| {
            for (caption, rev, slide, key) in [
                (
                    "In this lesson now",
                    compare.pinned,
                    compare.before.as_ref(),
                    "compare/before",
                ),
                (
                    "Newest",
                    compare.newest,
                    compare.after.as_ref(),
                    "compare/after",
                ),
            ] {
                ui.vertical(|ui| {
                    ui.set_width(w);
                    ui.label(
                        RichText::new(format!("{caption} — version {rev}"))
                            .size(fluid.fs_body_sm())
                            .color(cl::ink::MUTED),
                    );
                    let resp = miniature(ui, w, slide, styling);
                    ui.pilot_track(
                        &resp,
                        NodeMetadata::new(node(key), Role::Image, caption)
                            .value(format!("version {rev}")),
                    );
                    cl::tip(resp, &format!("{caption} — version {rev}"));
                });
                ui.add_space(cl::space::S4);
            }
        });
        ui.add_space(cl::space::S3);
        ui.horizontal_wrapped(|ui| {
            if button(
                ui,
                "compare/update",
                "Update",
                false,
                SPEC.tone,
                Weight::Hero,
                "Show the newest version in this lesson",
            ) {
                decision = Some(Some(true));
            }
            if button(
                ui,
                "compare/not-now",
                "Not now",
                false,
                SPEC.tone,
                Weight::Chrome,
                "Keep the version this lesson shows, and stop asking about this one",
            ) {
                decision = Some(Some(false));
            }
            if button(
                ui,
                "compare/back",
                "Back",
                false,
                SPEC.tone,
                Weight::Chrome,
                "Decide later",
            ) {
                decision = Some(None);
            }
        });

        let decision = decision?;
        let (key, newest, label) = (compare.key.clone(), compare.newest, compare.label.clone());
        view.compare = None;
        match decision {
            Some(true) => {
                view.lesson.update(&key, newest);
                view.commit(root)
                    .or_else(|| Some(format!("“{label}” shows version {newest} now.")))
            }
            Some(false) => {
                view.lesson.not_now(&key, newest);
                view.commit(root).or_else(|| {
                    Some(format!(
                        "Kept your version of “{label}”. It asks again if the card changes."
                    ))
                })
            }
            None => None,
        }
    }

    /// A card being changed from inside the lesson.
    fn editing_ui(&mut self, ui: &mut egui::Ui, root: &Path) {
        let Some(view) = self.view.as_mut() else {
            return;
        };
        let mut done = false;
        ui.horizontal(|ui| {
            if button(
                ui,
                "edit-back",
                "‹ Back to the lesson",
                false,
                SPEC.tone,
                Weight::Chrome,
                "Finish changing this card",
            ) {
                done = true;
            }
            // RL: what each tap shows and what you'll say.
            if button(
                ui,
                "edit-taps",
                if view.taps_open {
                    "Back to the words"
                } else {
                    "On tap"
                },
                view.taps_open,
                SPEC.tone,
                Weight::Chrome,
                "What each tap of the screen shows, and your teleprompter words",
            ) {
                view.taps_open = !view.taps_open;
            }
        });
        if let Some((_, editor)) = view.editing.as_mut() {
            if view.taps_open {
                egui::ScrollArea::vertical()
                    .id_salt("workbench.lessons.taps")
                    .auto_shrink([false, false])
                    .show(ui, |ui| editor.taps_ui(ui));
            } else {
                editor.ui(ui);
            }
        }
        if done {
            self.finish_edit(root);
        }
    }

    /// "Back to the lesson". A card changed from inside a lesson is this
    /// lesson's own change, so the lesson takes it; every other lesson
    /// using the card asks first.
    fn finish_edit(&mut self, root: &Path) {
        let Some(view) = self.view.as_mut() else {
            return;
        };
        let Some((was, editor)) = view.editing.take() else {
            return;
        };
        let slide = editor.slide();
        if slide.rev == was {
            view.refresh(root);
            return;
        }
        // Every place this lesson shows the card — it may be used twice.
        let keys: Vec<String> = view
            .lesson
            .uses
            .iter()
            .filter(|u| u.slide.id == slide.id)
            .map(|u| u.key.clone())
            .collect();
        for key in &keys {
            view.lesson.update(key, slide.rev);
        }
        let others = lesson::lessons_using(root, &slide.id)
            .into_iter()
            .filter(|dir| dir != &view.dir)
            .count();
        self.status = view.commit(root).unwrap_or_else(|| match others {
            0 => "Your change shows in this lesson.".to_owned(),
            1 => "Your change shows in this lesson. 1 other lesson using this card will \
                  ask first."
                .to_owned(),
            n => format!(
                "Your change shows in this lesson. {n} other lessons using this card will \
                 ask first."
            ),
        });
    }
}

/// What a tap on the lesson page asked for, applied after drawing.
enum Act {
    ToggleShelf,
    Add(usize),
    UpdateAll,
    Compare(String, u32),
    Move(String, usize),
    Remove(String),
    PickRole(String),
    CancelRole,
    PickLayout(String),
    DoneLayout,
    /// Axis set for this lesson only; `None` gives it back to the card.
    Variant(String, String, Option<String>),
    Role(String, Option<String>),
    Edit(String),
    /// Run the lesson from the card at this position (RL).
    Run(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dpl-lessons-{tag}-{}", dpl_doc::new_id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn library_card(root: &Path, words: &str) -> dpl_doc::Slide {
        let mut slide = dpl_doc::Slide::new("title@1");
        slide.blocks = vec![dpl_doc::Block::text("title", words)];
        dpl_doc::save(&dpl_doc::library_slide_dir(root, &slide.id), &slide).unwrap();
        slide
    }

    /// Run one frame of the page against `root`.
    fn frame(page: &mut Lessons, root: &Path) {
        let ctx = egui::Context::default();
        // The cards paint in the app's own fonts.
        cl::apply_theme(&ctx);
        // epaint 0.36 panics when unapplied texture deltas are dropped.
        ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| page.ui_in(ui, root));
        })
        .textures_delta
        .clear();
    }

    /// The role words are the schema's roles, in the same order.
    #[test]
    fn every_role_has_plain_words() {
        let ids: Vec<&str> = ROLE_WORDS.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, dpl_doc::roles());
        assert_eq!(role_words(None), NO_ROLE);
        assert_eq!(role_words(Some("practice")), "Let them try");
    }

    /// RL: Run asks for a run of this lesson from the card tapped — once —
    /// and each card says its taps and whether its words are written.
    #[test]
    fn run_asks_for_this_lesson_from_the_card_tapped() {
        let root = scratch("run");
        let a = library_card(&root, "First");
        let mut b = library_card(&root, "Second");
        b.blocks.push(dpl_doc::Block::text("subtitle", "Later"));
        b.reveals = vec!["subtitle".into()];
        dpl_doc::save(&dpl_doc::library_slide_dir(&root, &b.id), &b).unwrap();
        std::fs::write(
            dpl_doc::library_slide_dir(&root, &b.id).join(dpl_doc::script::SCRIPT_FILE),
            "---\nslideId: b\ntitle: B\nbeats:\n  - id: setup\n    cue: hello\n  - id: subtitle\n---\n",
        )
        .unwrap();
        let (dir, mut l) = lesson::create_lesson(&root, "Talk").unwrap();
        l.add(&a);
        l.add(&b);
        lesson::save_lesson(&dir, &l).unwrap();

        let mut page = Lessons::default();
        page.open(&dir, &root);
        assert_eq!(page.take_run(), None, "nothing asked yet");
        page.view.as_mut().unwrap().run = Some(1);
        assert_eq!(page.take_run(), Some((l.id.clone(), 1)));
        assert_eq!(page.take_run(), None, "asked once");

        let view = page.view.as_ref().unwrap();
        let words: Vec<String> = view
            .cards
            .iter()
            .map(|c| taps_words(c.taps, c.script))
            .collect();
        assert_eq!(
            words,
            ["no taps · no script yet", "1 tap · script, 1 to write"]
        );
        assert_eq!(taps_words(2, Some(0)), "2 taps · script");
        frame(&mut page, &root);
    }

    /// Opening a lesson draws each use at its pinned version, and a card
    /// changed elsewhere shows Newer version.
    #[test]
    fn a_lesson_draws_its_pinned_cards_and_what_changed() {
        let root = scratch("view");
        let mut a = library_card(&root, "First");
        let b = library_card(&root, "Second");
        let (dir, mut l) = lesson::create_lesson(&root, "Costs").unwrap();
        l.add(&a);
        l.add(&b);
        lesson::save_lesson(&dir, &l).unwrap();
        // A changes somewhere else.
        a.blocks = vec![dpl_doc::Block::text("title", "First, changed")];
        a.rev = 2;
        dpl_doc::save(&dpl_doc::library_slide_dir(&root, &a.id), &a).unwrap();

        let mut page = Lessons::default();
        frame(&mut page, &root);
        assert_eq!(page.listed.as_ref().unwrap()[0].waiting, 1);
        page.open(&dir, &root);
        frame(&mut page, &root);
        let view = page.view.as_ref().unwrap();
        assert_eq!(view.cards[0].label, "First", "the pinned version");
        assert_eq!(view.cards[0].state, UseState::UpdateAvailable { newest: 2 });
        assert_eq!(view.cards[1].state, UseState::Current);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Compare shows both versions; Update moves the pin and saves it.
    #[test]
    fn compare_then_update_takes_the_newest() {
        let root = scratch("compare");
        let mut a = library_card(&root, "Old words");
        let (dir, mut l) = lesson::create_lesson(&root, "Compare").unwrap();
        let key = l.add(&a);
        lesson::save_lesson(&dir, &l).unwrap();
        a.blocks = vec![dpl_doc::Block::text("title", "New words")];
        a.rev = 2;
        dpl_doc::save(&dpl_doc::library_slide_dir(&root, &a.id), &a).unwrap();

        let mut page = Lessons::default();
        page.open(&dir, &root);
        let view = page.view.as_mut().unwrap();
        view.open_compare(&root, &key, 2);
        let compare = view.compare.as_ref().unwrap();
        assert_eq!((compare.pinned, compare.newest), (1, 2));
        assert_eq!(compare.label, "Old words");
        assert!(compare.before.is_some() && compare.after.is_some());

        view.compare = None;
        view.lesson.update(&key, 2);
        assert!(view.commit(&root).is_none());
        let saved = lesson::load_lesson(&dir).unwrap();
        assert_eq!(saved.get(&key).unwrap().slide.rev, 2);
        assert_eq!(view.cards[0].label, "New words");
        assert_eq!(view.cards[0].state, UseState::Current);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A card changed from inside a lesson is taken by that lesson — every
    /// place it shows the card; another lesson using it asks.
    #[test]
    fn an_edit_from_the_lesson_is_taken_and_others_ask() {
        let root = scratch("edit");
        let a = library_card(&root, "Shared");
        let (here, mut first) = lesson::create_lesson(&root, "Here").unwrap();
        let key = first.add(&a);
        let again = first.add(&a);
        lesson::save_lesson(&here, &first).unwrap();
        let (there, mut second) = lesson::create_lesson(&root, "There").unwrap();
        second.add(&a);
        lesson::save_lesson(&there, &second).unwrap();

        let mut page = Lessons::default();
        page.open(&here, &root);
        let dir = dpl_doc::library_slide_dir(&root, &a.id);
        let mut editor = dpl_editor::Editor::open(&dir, &root).unwrap();
        editor.set_enter(Some("none".into()));
        page.view.as_mut().unwrap().editing = Some((1, editor));
        page.finish_edit(&root);
        let view = page.view.as_ref().unwrap();
        assert!(view
            .cards
            .iter()
            .all(|c| c.state == UseState::Current && c.pinned == 2));
        assert_eq!(view.lesson.get(&again).unwrap().slide.rev, 2);
        assert!(
            page.status
                .contains("1 other lesson using this card will ask first"),
            "{}",
            page.status
        );

        assert_eq!(
            lesson::load_lesson(&here)
                .unwrap()
                .get(&key)
                .unwrap()
                .slide
                .rev,
            2
        );
        let other = lesson::load_lesson(&there).unwrap();
        assert_eq!(
            lesson::use_state(&root, &other.uses[0]),
            UseState::UpdateAvailable { newest: 2 }
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// New lesson from the list: named, saved, and opened.
    #[test]
    fn a_new_lesson_opens_named() {
        let root = scratch("new");
        let mut page = Lessons::default();
        page.make_lesson("How the cost adds up", &root);
        let view = page.view.as_ref().expect("opened");
        assert_eq!(view.lesson.title, "How the cost adds up");
        assert!(view.dir.join("lesson.json").is_file());
        frame(&mut page, &root);
        let _ = std::fs::remove_dir_all(&root);
    }
}
