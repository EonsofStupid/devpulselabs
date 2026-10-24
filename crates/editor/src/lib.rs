//! The tap-to-edit editor — a VIEW over a slide document (W4).
//!
//! The operator's law: layman-friendly, touch-first, the stack invisible.
//! You tap the real slide — a text line, or its background — and a tray of
//! the THING ITSELF slides up (fonts drawn as fonts, moods as living
//! gradient cards). Tap one, it applies on the slide instantly and writes
//! back the colocated `slide.json`. No save button. One big Undo. No JSON,
//! no ids, no "token" — ever, on screen.
//!
//! W4 is select-and-restyle-existing (the operator's literal spec). Drag
//! reorder / add / delete blocks are W4b (egui_dnd, RS2).

use dpl_coldlight::{self as cl};
use dpl_slides::document;
use egui::{Align2, Color32, CornerRadius, FontId, Rect, RichText, Sense, Stroke, StrokeKind, Ui};
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::path::{Path, PathBuf};

/// What the operator has tapped.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
enum Selection {
    /// Nothing selected — the LOOK tray offers one-tap presets.
    #[default]
    None,
    /// The slide background — the Colours/Motion trays.
    Background,
    /// A text line, by block id — the Font/Size/Ink/Text trays.
    Block(String),
}

/// The editor over ONE slide document folder.
pub struct Editor {
    dir: PathBuf,
    slide: dpl_doc::Slide,
    /// The show's theme (W3b) — auto-detected from the parent episode
    /// folder, else the fallback. Trays show RESOLVED values; tapping a
    /// card writes an override; "Show theme" clears one.
    theme: dpl_doc::Theme,
    clock: dpl_slides::motion::Clock,
    selection: Selection,
    /// Snapshots for Undo — the doc is a small value, snapshot per edit.
    undo: Vec<dpl_doc::Slide>,
    /// Last save's outcome, shown as an honest footer crumb.
    saved: Result<(), String>,
    /// A look being TRIED (LK1): the stage shows it, nothing is saved until
    /// Keep.
    trying: Option<String>,
    /// Is the look chip's menu open?
    link_menu: bool,
    /// Every knob, not just the look's (LK1 "More controls").
    more_controls: bool,
    /// The edit burst in progress — typing settles into ONE revision.
    burst: Option<(String, std::time::Instant)>,
    /// The yours-dot whose comparison is open ("Yours: … · The look's: …").
    compare: Option<dpl_doc::link::Knob>,
}

/// A text block's tray view: values resolved through the show theme, with
/// the override flags that drive the "Show theme" chips.
struct TextFields {
    text: String,
    font: String,
    size: String,
    ink: String,
}

const UNDO_CAP: usize = 50;
/// A pause longer than this ends a typing burst — the next keystroke is a
/// new revision and a new undo step.
const BURST_PAUSE: std::time::Duration = std::time::Duration::from_millis(1500);
/// What an unsized line paints at — the renderer's own fallback.
const DEFAULT_SIZE: &str = "body";
/// The slot a card's picture fills in every component that has one.
const MEDIA_SLOT: &str = "media";
/// The component a card becomes when it holds a quote.
const QUOTE_COMPONENT: &str = "quote@1";
/// The quote component's slots the quote tab writes, in reading order.
pub const QUOTE_SLOTS: [&str; 3] = [QUOTE_LINES[0].0, QUOTE_LINES[1].0, QUOTE_LINES[2].0];

/// The quote tab's three lines, in reading order: the slot, and the size
/// and ink a NEW line starts with (ordinary overrides afterwards).
const QUOTE_LINES: [(&str, &str, &str); 3] = [
    ("quote", "h2", "strong"),
    ("name", "body-lg", "tone"),
    ("role", "body-sm", "subtle"),
];
const INKS: [&str; 5] = ["strong", "base", "muted", "subtle", "tone"];
const AMBIENCES: [&str; 3] = ["pulse", "drift", "rings"];
const SIZES: [&str; 10] = [
    "eyebrow",
    "body-sm",
    "body",
    "body-lg",
    "h4",
    "h3",
    "h2",
    "h1",
    "display",
    "display-xl",
];

impl Editor {
    /// Open the editor on a slide folder (the colocated slide.json).
    pub fn open(dir: &Path) -> Result<Self, String> {
        let slide = dpl_doc::load(dir).map_err(|e| e.to_string())?;
        Ok(Self {
            dir: dir.to_path_buf(),
            slide,
            theme: dpl_doc::theme_for_slide(dir),
            clock: dpl_slides::motion::Clock::default(),
            selection: Selection::None,
            trying: None,
            link_menu: false,
            more_controls: false,
            burst: None,
            compare: None,
            undo: Vec::new(),
            saved: Ok(()),
        })
    }

    pub fn slide(&self) -> &dpl_doc::Slide {
        &self.slide
    }

    /// Set what sits behind the words, or `None` to inherit the show's theme.
    ///
    /// The authoring surface's Background tab is a VIEW over this document,
    /// so it asks the editor to change the slide rather than writing the file
    /// itself. Going through `mutate` means a tapped colour lands on the undo
    /// stack and saves like every other edit — one mutation path, not two.
    pub fn set_background(&mut self, background: Option<dpl_doc::Background>) {
        self.mutate(|slide| slide.background = background);
    }

    /// The show's theme this slide resolves against. A preview drawn beside
    /// the editor has to cascade the same way the card does, or it shows
    /// something the deck never will.
    pub fn theme(&self) -> &dpl_doc::Theme {
        &self.theme
    }

    /// What the show's theme says the entrance is — what "follow the show"
    /// resolves to, so a picker can name it instead of showing an invisible
    /// default.
    pub fn look_enter(&self) -> Option<String> {
        self.theme.enter.clone()
    }

    /// The three lines of a quote — the quotation, who said it, where —
    /// empty where the card has no such block. Read for the tab's fields.
    pub fn quote(&self) -> [String; 3] {
        let mut out = [String::new(), String::new(), String::new()];
        for (slot, (id, _, _)) in out.iter_mut().zip(QUOTE_LINES) {
            if let Some(words) = self.slide.block(id).and_then(dpl_doc::Block::words) {
                *slot = words.text.to_owned();
            }
        }
        out
    }

    /// Write the three lines of a quote: the quotation as a real
    /// [`dpl_doc::Block::Quote`] (ruling #3), who said it and where in the
    /// quote component's own `name` and `role` slots.
    ///
    /// - a line with text UPDATES the block with that id, or appends it
    ///   (never a second block with the same id);
    /// - a BLANK line removes its block, because a document carrying an empty
    ///   text block is a rendering artefact, not an empty field;
    /// - the first non-blank line makes the card fill the `quote` component.
    pub fn set_quote(&mut self, lines: [String; 3]) {
        self.mutate(|slide| {
            for (line, (id, size, ink)) in lines.iter().zip(QUOTE_LINES) {
                let words = line.trim();
                let at = slide.blocks.iter().position(|b| b.id() == id);
                match (words.is_empty(), at) {
                    // Blank and present — the block goes.
                    (true, Some(i)) => {
                        slide.blocks.remove(i);
                    }
                    (true, None) => {}
                    // Present — update in place, keeping whatever restyling
                    // the operator has since done to it.
                    (false, Some(i)) => {
                        if let Some(block) = slide.blocks[i].words_mut() {
                            *block.text = words.to_string();
                        }
                    }
                    // New — append with the quote's default styling.
                    (false, None) => {
                        let text = words.to_string();
                        let size = Some(size.to_string());
                        let ink = Some(ink.to_string());
                        slide.blocks.push(if id == "quote" {
                            dpl_doc::Block::Quote {
                                id: id.to_string(),
                                preset: None,
                                tone: None,
                                text,
                                font: None,
                                size,
                                ink,
                            }
                        } else {
                            dpl_doc::Block::Text {
                                id: id.to_string(),
                                preset: None,
                                tone: None,
                                text,
                                font: None,
                                size,
                                ink,
                            }
                        });
                    }
                }
            }
            if lines.iter().any(|l| !l.trim().is_empty()) {
                slide.component = QUOTE_COMPONENT.to_string();
            }
        });
    }

    /// The card's picture, if it has one: `(src, show, fit)`.
    pub fn media(&self) -> Option<(String, String, String)> {
        self.slide.blocks.iter().find_map(|b| match b {
            dpl_doc::Block::Media { show, src, fit, .. } => Some((
                src.clone(),
                show.clone(),
                fit.clone().unwrap_or_else(|| "cover".to_string()),
            )),
            _ => None,
        })
    }

    /// Put a picture on the card, or take it off with `None`.
    ///
    /// `src` must be content-relative — the Picture tab passes through a path
    /// the artifact index already holds, so the content jail is satisfied by
    /// construction rather than by a check here. `show` comes from the index's
    /// own `kind`, never sniffed from the extension: the schema authors that
    /// discriminator so the doctor can name a mismatch.
    ///
    /// One picture per card. A second call replaces rather than appends,
    /// because `Block::Media` is a backdrop and two would fight.
    pub fn set_media(&mut self, media: Option<(String, String, String)>) {
        self.mutate(|slide| {
            slide
                .blocks
                .retain(|b| !matches!(b, dpl_doc::Block::Media { .. }));
            if let Some((src, show, fit)) = media {
                slide.blocks.insert(
                    0,
                    dpl_doc::Block::Media {
                        id: MEDIA_SLOT.to_string(),
                        preset: None,
                        tone: None,
                        show,
                        src,
                        // The schema's default is cover; write it only when it
                        // is the other one, so a document stays sparse.
                        fit: (fit != "cover").then_some(fit),
                        poster: None,
                    },
                );
            }
        });
    }

    /// Set how the words arrive, or `None` to follow the show's theme.
    ///
    /// Restarts the clock so the entrance actually plays: a movement picker
    /// whose preview does not move is a contradiction.
    pub fn set_enter(&mut self, enter: Option<String>) {
        self.mutate(|slide| slide.enter = enter);
        self.clock.restart();
    }

    /// Apply an edit: snapshot for undo, mutate, then write back the
    /// colocated file immediately (no save ceremony — the file is truth).
    fn mutate(&mut self, f: impl FnOnce(&mut dpl_doc::Slide)) {
        self.burst = None;
        self.mutate_in(f);
    }

    fn mutate_in(&mut self, f: impl FnOnce(&mut dpl_doc::Slide)) {
        if self.undo.len() >= UNDO_CAP {
            self.undo.remove(0);
        }
        let before = self.slide.clone();
        f(&mut self.slide);
        if self.slide == before {
            return;
        }
        // Every saved change is a new revision — a lesson pins one (LS3).
        self.slide.rev = before.rev + 1;
        self.undo.push(before);
        self.save();
    }

    /// One edit of a burst (typing): the first keystroke is a revision and
    /// an undo step; the rest of the burst — same field, no pause longer
    /// than [`BURST_PAUSE`] — saves into it. Ten letters, one Undo.
    fn mutate_burst(&mut self, key: &str, f: impl FnOnce(&mut dpl_doc::Slide)) {
        let now = std::time::Instant::now();
        let continuing = self
            .burst
            .as_ref()
            .is_some_and(|(k, at)| k == key && now.duration_since(*at) < BURST_PAUSE);
        if continuing {
            f(&mut self.slide);
            self.save();
        } else {
            self.mutate_in(f);
        }
        self.burst = Some((key.to_owned(), now));
    }

    /// The slide's link to its look, worked out from its own values (LK1).
    pub fn link_state(&self) -> dpl_doc::link::LinkState {
        dpl_doc::link::link_state(&self.slide)
    }

    /// Where a knob's value comes from: yours, the look's, or the show's.
    pub fn source(&self, knob: &dpl_doc::link::Knob) -> dpl_doc::link::Source {
        dpl_doc::link::source(&self.slide, knob)
    }

    /// Give one value back to the look (or the show).
    pub fn reset(&mut self, knob: &dpl_doc::link::Knob) {
        let knob = knob.clone();
        self.mutate(|s| dpl_doc::link::reset(s, &knob));
    }

    /// "Undo all my changes".
    pub fn reset_all(&mut self) {
        self.mutate(dpl_doc::link::reset_all);
    }

    /// "Stop following this look" — nothing changes on screen.
    pub fn stop_following(&mut self) {
        let theme = self.theme.clone();
        self.mutate(|s| dpl_doc::link::stop_following(s, &theme));
    }

    /// "Follow the look again", keeping my changes or starting fresh.
    pub fn follow_again(&mut self, keep_my_changes: bool) {
        self.mutate(|s| dpl_doc::link::follow_again(s, keep_my_changes));
    }

    /// Is there anything to undo?
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    fn save(&mut self) {
        self.saved = dpl_doc::save(&self.dir, &self.slide).map_err(|e| e.to_string());
    }

    /// Undo restores the earlier CONTENT as a new revision: history only
    /// moves forward, so a lesson pinned to a later rev is never handed an
    /// older document under the same number.
    pub fn undo(&mut self) {
        self.burst = None;
        if let Some(prev) = self.undo.pop() {
            let rev = self.slide.rev + 1;
            self.slide = prev;
            self.slide.rev = rev;
            self.save();
        }
    }

    /// One frame of the editor.
    pub fn ui(&mut self, ui: &mut Ui) {
        let full = ui.available_rect_before_wrap();
        // Toolbar row: title, Undo, saved crumb.
        let toolbar_h = cl::space::S6 + cl::space::S2;
        let bar = Rect::from_min_size(full.min, egui::vec2(full.width(), toolbar_h));
        self.toolbar(ui, bar);

        // The stage: a 16:9 well under the toolbar, room for the tray below.
        let below = Rect::from_min_max(egui::pos2(full.min.x, bar.max.y), full.max);
        let tray_h = below.height() * 0.34;
        let stage_area = Rect::from_min_max(
            below.min,
            egui::pos2(below.max.x, below.max.y - tray_h - cl::space::S3),
        );
        let stage = fit_stage(stage_area);
        ui.painter()
            .rect_filled(stage, cl::RADIUS, cl::surface::WELL);

        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        // Trying a look (LK1): the stage shows the slide AS IF it used it;
        // the file is untouched until Keep.
        let shown = match &self.trying {
            Some(id) => {
                let mut tried = self.slide.clone();
                use_look(&mut tried, id);
                tried
            }
            None => self.slide.clone(),
        };
        let map = {
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(stage.shrink(2.0)));
            child.set_clip_rect(stage);
            document::render_traced(&mut child, stage.shrink(2.0), &shown, &self.theme, now, t0)
        };
        ui.painter().rect_stroke(
            stage,
            cl::RADIUS,
            Stroke::new(1.0, cl::surface::HAIRLINE),
            StrokeKind::Inside,
        );

        // Tap → select (topmost block, else background).
        let resp = ui.interact(stage, ui.id().with("dpl_editor_stage"), Sense::click());
        if let Some(pos) = resp.interact_pointer_pos() {
            if resp.clicked() {
                self.selection = match map.block_at(pos) {
                    Some(id) => Selection::Block(id.to_string()),
                    None => Selection::Background,
                };
            }
        }
        self.paint_selection(ui, &map);

        // The contextual tray.
        let tray = Rect::from_min_max(
            egui::pos2(below.min.x, stage.max.y + cl::space::S3),
            below.max,
        );
        self.tray(ui, tray);

        ui.ctx().request_repaint(); // the document's blend never rests
    }

    fn toolbar(&mut self, ui: &mut Ui, bar: Rect) {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bar)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let chip_rect = self.look_chip(&mut child);
        child.add_space(cl::space::S3);
        child.label(
            RichText::new("Tap the slide to change it")
                .color(cl::ink::MUTED)
                .size(cl::Fluid::of(bar).fs_body()),
        );
        if self.link_menu {
            self.link_menu_ui(ui, chip_rect);
        }
        child.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let crumb = match &self.saved {
                Ok(()) if !self.undo.is_empty() => "saved".to_string(),
                Ok(()) => String::new(),
                Err(e) => format!("save failed: {e}"),
            };
            if !crumb.is_empty() {
                ui.label(RichText::new(crumb).small().color(cl::ink::SUBTLE));
                ui.add_space(cl::space::S3);
            }
            let can_undo = !self.undo.is_empty();
            if big_button(ui, "undo", "Undo", can_undo, "Take back the last change") {
                self.undo();
            }
        });
    }

    /// The link bar for surfaces that host the editor's controls without
    /// its toolbar (the Workbench card editor's tabs): the look chip with
    /// its menu, and Undo — so every tab shows what the slide follows and
    /// can take the last change back (LK1).
    pub fn link_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let chip_rect = self.look_chip(ui);
            if self.link_menu {
                self.link_menu_ui(ui, chip_rect);
            }
            ui.add_space(cl::space::S3);
            let can_undo = !self.undo.is_empty();
            if big_button(ui, "undo", "Undo", can_undo, "Take back the last change") {
                self.undo();
            }
        });
    }

    /// The background the slide SHOWS — its own, else its look's, else the
    /// show's.
    pub fn shown_background(&self) -> dpl_doc::Background {
        self.inherited_background()
    }

    /// The look chip (LK1): "Look: Neon intro · 3 changes", "Own style (was
    /// Neon intro)" or "The show's style". Tap for the menu.
    fn look_chip(&mut self, ui: &mut Ui) -> Rect {
        if let Some(id) = &self.trying {
            // Trying (LK1): the stage shows the look, nothing is saved — the
            // chip says so instead of naming a link that has not changed.
            let text = format!("Trying: {}", look_label(id));
            let resp = chip_response(
                ui,
                "look-chip",
                &text,
                true,
                "Keep the look, or Cancel to go back — nothing is saved until you Keep",
            );
            return resp.rect;
        }
        let state = self.link_state();
        let text = state.chip(look_label);
        let tip = match &state {
            dpl_doc::link::LinkState::Follows { .. } => {
                "This slide follows its look. Tap to see your changes, give them back, or stop following"
            }
            dpl_doc::link::LinkState::OwnStyle { .. } => {
                "This slide has its own style. Tap to follow its look again"
            }
            dpl_doc::link::LinkState::Show { .. } => {
                "This slide uses the show's style. Pick a look below to follow one"
            }
        };
        let resp = chip_response(ui, "look-chip", &text, self.link_menu, tip);
        if resp.clicked() {
            self.link_menu = !self.link_menu;
        }
        resp.rect
    }

    /// The look chip's menu: your changes (each with its own way back),
    /// "Undo all my changes", and stop / follow again.
    fn link_menu_ui(&mut self, ui: &mut Ui, under: Rect) {
        let state = self.link_state();
        let changes = dpl_doc::link::changes(&self.slide);
        let area = egui::Area::new(ui.id().with("dpl_editor_link_menu"))
            .order(egui::Order::Foreground)
            .fixed_pos(under.left_bottom() + egui::vec2(0.0, cl::space::S1));
        area.show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(cl::surface::RAISED)
                .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
                .corner_radius(cl::RADIUS)
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_max_width(360.0);
                    ui.label(tray_title(match &state {
                        dpl_doc::link::LinkState::OwnStyle { .. } => "Own style",
                        _ => "Your changes",
                    }));
                    if changes.is_empty() {
                        ui.label(small("Nothing yet — everything follows."));
                    }
                    for knob in &changes {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("• {}", knob.label()))
                                    .small()
                                    .color(cl::ink::BASE),
                            );
                            if !matches!(state, dpl_doc::link::LinkState::OwnStyle { .. }) {
                                let back = give_back_label(&self.slide, knob);
                                if chip(
                                    ui,
                                    &format!("reset-{}", knob_key(knob)),
                                    back,
                                    false,
                                    "Give this value back — the slide follows again here",
                                ) {
                                    self.reset(knob);
                                }
                            }
                        });
                    }
                    ui.add_space(cl::space::S2);
                    match &state {
                        dpl_doc::link::LinkState::Follows { look, .. } => {
                            if !changes.is_empty()
                                && chip(
                                    ui,
                                    "reset-all",
                                    "Undo all my changes",
                                    false,
                                    "Give every value back to the look",
                                )
                            {
                                self.reset_all();
                            }
                            if chip(
                                ui,
                                "stop-following",
                                "Stop following this look",
                                false,
                                &format!(
                                    "Make the slide fully yours. Nothing changes on screen; {} stops updating it",
                                    look_label(look)
                                ),
                            ) {
                                self.stop_following();
                                self.link_menu = false;
                            }
                        }
                        dpl_doc::link::LinkState::OwnStyle { was: Some(was) } => {
                            if chip(
                                ui,
                                "follow-keep",
                                "Follow the look again — keep my changes",
                                false,
                                &format!("Follow {} again; what differs stays yours", look_label(was)),
                            ) {
                                self.follow_again(true);
                                self.link_menu = false;
                            }
                            if chip(
                                ui,
                                "follow-fresh",
                                "Follow the look again — start fresh",
                                false,
                                &format!("Follow {} again and give back everything it styles", look_label(was)),
                            ) {
                                self.follow_again(false);
                                self.link_menu = false;
                            }
                        }
                        dpl_doc::link::LinkState::OwnStyle { was: None }
                        | dpl_doc::link::LinkState::Show { .. } => {
                            if !changes.is_empty()
                                && chip(
                                    ui,
                                    "reset-all",
                                    "Undo all my changes",
                                    false,
                                    "Give every value back to the show",
                                )
                            {
                                self.reset_all();
                            }
                        }
                    }
                    if chip(ui, "menu-done", "Done", false, "Close this menu") {
                        self.link_menu = false;
                    }
                });
        });
    }

    fn paint_selection(&self, ui: &Ui, map: &document::RenderMap) {
        let (rect, tone) = match &self.selection {
            Selection::None => return,
            Selection::Background => (map.background, cl::Tone::Cyan),
            Selection::Block(id) => match map.blocks.iter().find(|(b, _)| b == id) {
                Some((_, r)) => (r.expand(cl::space::S2), cl::Tone::Amber),
                None => return,
            },
        };
        ui.painter().rect_stroke(
            rect,
            cl::RADIUS_SM,
            Stroke::new(2.0, tone.color()),
            StrokeKind::Outside,
        );
    }

    // ── the trays — the THING ITSELF, tap to apply ──────────────────────

    fn tray(&mut self, ui: &mut Ui, area: Rect) {
        ui.painter()
            .rect_filled(area, cl::RADIUS, cl::surface::RAISED);
        let inner = area.shrink(cl::space::S4);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        child.set_clip_rect(inner);
        match self.selection.clone() {
            Selection::None => self.look_tray(&mut child),
            Selection::Background => self.background_tray(&mut child),
            Selection::Block(id) => self.text_tray(&mut child, &id),
        }
    }

    /// Nothing selected → the LOOK presets (W3), applied whole.
    fn look_tray(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(tray_title("Looks — tap one to try it on"));
            // Trying (LK1): nothing is saved until Keep.
            if let Some(id) = self.trying.clone() {
                ui.add_space(cl::space::S3);
                if hero_chip(
                    ui,
                    "keep",
                    "Keep",
                    "Use this look on the slide (one Undo takes it back)",
                ) {
                    self.mutate(|s| use_look(s, &id));
                    self.trying = None;
                }
                if chip(
                    ui,
                    "cancel",
                    "Cancel",
                    false,
                    "Go back to how the slide was",
                ) {
                    self.trying = None;
                }
            }
        });
        // The chosen look's knobs, in plain words (its `exposes`): what the
        // tabs above may change without leaving the look.
        if let Some(look) = dpl_doc::card_preset(&self.slide) {
            let knobs: Vec<&str> = look
                .exposes
                .iter()
                .map(|knob| match *knob {
                    "ambience" => "moving pattern",
                    "motion" => "movement",
                    other => other,
                })
                .collect();
            ui.label(
                RichText::new(format!(
                    "Using {} — you can change its {}",
                    look.label,
                    knobs.join(", ")
                ))
                .small()
                .color(cl::ink::MUTED),
            );
        }
        egui::ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for p in cl::THEME_PRESETS.iter() {
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(150.0, 84.0), Sense::click());
                        let painter = ui.painter_at(rect);
                        if let (Some(a), Some(b)) = (mood(p.moods.0), mood(p.moods.1)) {
                            cl::mood_blend(&painter, rect, a, b, 0.45, 0.0);
                        }
                        painter.text(
                            rect.left_top() + egui::vec2(8.0, 14.0),
                            Align2::LEFT_CENTER,
                            p.label,
                            FontId::new(15.0, cl::font_family(p.font)),
                            cl::ink::STRONG,
                        );
                        let chosen = self.slide.preset.as_deref() == Some(p.id);
                        let tried = self.trying.as_deref() == Some(p.id);
                        card_edge(&painter, rect, resp.hovered() || chosen || tried);
                        resp.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Button,
                                true,
                                chosen,
                                p.label,
                            )
                        });
                        ui.pilot_track(
                            &resp,
                            NodeMetadata::new(
                                PilotId::new("editor.look").child(&PilotKey::new(p.id)),
                                egui_pilot_ai::protocol::Role::Button,
                                p.label,
                            ),
                        );
                        let resp = cl::tip(
                            resp,
                            if chosen {
                                "This slide's look — its background, colour and font come from it"
                            } else {
                                "Try this look on the slide; Keep makes the slide follow it"
                            },
                        );
                        if resp.clicked() && !chosen {
                            self.trying = Some(p.id.to_owned());
                        }
                    }
                });
            });
    }

    /// Background selected → Colours (mood A/B) + Ambience. Shows the
    /// RESOLVED background; "Show theme" clears the slide's override.
    fn background_tray(&mut self, ui: &mut Ui) {
        let knob = dpl_doc::link::Knob::Background;
        ui.horizontal(|ui| {
            ui.label(tray_title("Background"));
            self.yours_row(ui, &knob);
        });
        let resolved = self.inherited_background();
        let (cur_a, cur_b, cur_amb) = match &resolved {
            dpl_doc::Background::MoodBlend {
                moods, ambience, ..
            } => (
                moods.first().cloned(),
                moods.get(1).cloned(),
                ambience.clone(),
            ),
            dpl_doc::Background::Mood { mood } => (Some(mood.clone()), None, None),
            dpl_doc::Background::None => (None, None, None),
        };

        egui::ScrollArea::horizontal()
            .id_salt("bg_colours")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(small("Colour A"));
                        if let Some(pick) = mood_row(ui, "a", cur_a.as_deref()) {
                            self.set_blend_mood(0, pick);
                        }
                    });
                    ui.add_space(cl::space::S3);
                    ui.vertical(|ui| {
                        ui.label(small("Colour B"));
                        if let Some(pick) = mood_row(ui, "b", cur_b.as_deref()) {
                            self.set_blend_mood(1, pick);
                        }
                    });
                });
            });

        ui.add_space(cl::space::S2);
        ui.label(small("Motion"));
        ui.horizontal(|ui| {
            for amb in [None, Some("pulse"), Some("drift"), Some("rings")] {
                let label = amb.unwrap_or("none");
                let on = cur_amb.as_deref() == amb;
                if chip(
                    ui,
                    &format!("ambience-{label}"),
                    label,
                    on,
                    "A moving pattern over the background",
                ) {
                    let amb = amb.map(str::to_string);
                    self.set_ambience(amb);
                }
            }
        });
        let _ = AMBIENCES;
    }

    /// Text block selected → Font / Size / Ink / Text. Font and ink show
    /// the RESOLVED value; "Show theme" clears an override back to the show.
    fn text_tray(&mut self, ui: &mut Ui, id: &str) {
        let Some(tf) = self.text_fields(id) else {
            return;
        };
        let (text, font, size, ink) = (tf.text, tf.font, tf.size, tf.ink);
        ui.label(tray_title("Text"));

        // The words.
        let mut edited = text.clone();
        let field = ui.add(egui::TextEdit::singleline(&mut edited).desired_width(f32::INFINITY));
        ui.pilot_track(
            &field,
            NodeMetadata::new(
                PilotId::new("editor").child(&PilotKey::new("text")),
                egui_pilot_ai::protocol::Role::TextInput,
                "The words",
            ),
        );
        let field = cl::tip(field, "The words on this line — it saves as you type");
        if field.changed() {
            let key = format!("text-{id}");
            let id = id.to_string();
            self.mutate_burst(&key, |s| set_text(s, &id, |b| b.0 = edited.clone()));
        }
        if field.lost_focus() {
            self.burst = None;
        }

        let line = |field| dpl_doc::link::Knob::Line {
            id: id.to_string(),
            field,
        };
        ui.add_space(cl::space::S2);
        ui.horizontal(|ui| {
            ui.label(small("Font"));
            self.yours_row(ui, &line(dpl_doc::link::LineField::Font));
        });
        egui::ScrollArea::horizontal()
            .id_salt("font_tray")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for def in cl::FONTS.iter() {
                        let picked = def.id == font;
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(132.0, 48.0), Sense::click());
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(
                            rect,
                            cl::RADIUS_SM,
                            if picked {
                                cl::Tone::Amber.wash()
                            } else {
                                cl::surface::OVERLAY
                            },
                        );
                        painter.text(
                            rect.center(),
                            Align2::CENTER_CENTER,
                            def.label,
                            FontId::new(16.0, cl::font_family(def.id)),
                            if picked {
                                cl::ink::STRONG
                            } else {
                                cl::ink::BASE
                            },
                        );
                        card_edge(&painter, rect, resp.hovered());
                        resp.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Button,
                                true,
                                picked,
                                def.label,
                            )
                        });
                        ui.pilot_track(
                            &resp,
                            NodeMetadata::new(
                                PilotId::new("editor.font").child(&PilotKey::new(def.id)),
                                egui_pilot_ai::protocol::Role::Button,
                                def.label,
                            ),
                        );
                        let resp = cl::tip(resp, def.blurb);
                        if resp.clicked() {
                            let (id, fid) = (id.to_string(), def.id.to_string());
                            self.mutate(|s| set_text(s, &id, |b| b.1 = fid.clone()));
                        }
                    }
                });
            });

        // The look shows its own knobs (`exposes`); "More controls" opens
        // every knob — full control without leaving the look (LK1).
        let look_gives_only_font = dpl_doc::card_preset(&self.slide).is_some();
        if look_gives_only_font {
            ui.add_space(cl::space::S2);
            let label = if self.more_controls {
                "Fewer controls"
            } else {
                "More controls"
            };
            if chip(
                ui,
                "more-controls",
                label,
                self.more_controls,
                "Show every setting for this line — size and ink too",
            ) {
                self.more_controls = !self.more_controls;
            }
        }
        if look_gives_only_font && !self.more_controls {
            return;
        }

        ui.add_space(cl::space::S2);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(small("Size"));
                    self.yours_row(ui, &line(dpl_doc::link::LineField::Size));
                });
                ui.horizontal_wrapped(|ui| {
                    for name in SIZES {
                        if chip(
                            ui,
                            &format!("size-{name}"),
                            name,
                            name == size,
                            "How big this line is",
                        ) {
                            let (id, name) = (id.to_string(), name.to_string());
                            self.mutate(|s| set_text(s, &id, |b| b.2 = name.clone()));
                        }
                    }
                });
            });
            ui.add_space(cl::space::S4);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(small("Ink"));
                    self.yours_row(ui, &line(dpl_doc::link::LineField::Ink));
                });
                ui.horizontal(|ui| {
                    for name in INKS {
                        if chip(
                            ui,
                            &format!("ink-{name}"),
                            name,
                            name == ink,
                            "How strong this line's colour is",
                        ) {
                            let (id, name) = (id.to_string(), name.to_string());
                            self.mutate(|s| set_text(s, &id, |b| b.3 = name.clone()));
                        }
                    }
                });
            });
        });
    }

    /// Beside a control's label: when its value is yours, a dot (colour plus
    /// a ring — never colour alone) and the honest way back ("Use the
    /// look's" or "Use the show's"). Tapping the dot compares the two
    /// values in words. Following values show nothing.
    pub fn yours_row(&mut self, ui: &mut Ui, knob: &dpl_doc::link::Knob) {
        if self.source(knob) != dpl_doc::link::Source::Yours {
            if self.compare.as_ref() == Some(knob) {
                self.compare = None;
            }
            return;
        }
        let (dot, resp) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), Sense::click());
        ui.painter()
            .circle_filled(dot.center(), 4.0, cl::Tone::Amber.color());
        ui.painter()
            .circle_stroke(dot.center(), 7.0, Stroke::new(1.5, cl::Tone::Amber.color()));
        let back = give_back_label(&self.slide, knob);
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, back));
        ui.pilot_track(
            &resp,
            NodeMetadata::new(
                PilotId::new("editor.yours").child(&PilotKey::new(knob_key(knob))),
                egui_pilot_ai::protocol::Role::Button,
                "Yours",
            ),
        );
        let resp = cl::tip(resp, "Yours — tap to compare it with what it would follow");
        if resp.clicked() {
            self.compare = match &self.compare {
                Some(open) if open == knob => None,
                _ => Some(knob.clone()),
            };
        }
        if chip(
            ui,
            &format!("back-{}", knob_key(knob)),
            back,
            false,
            "Give this value back — the slide follows again here",
        ) {
            self.compare = None;
            self.reset(knob);
            return;
        }
        if self.compare.as_ref() == Some(knob) {
            // A small sheet under the row: it floats, so a long value never
            // pushes the control's row (or the tab) wider. One line per
            // value, so a name never breaks across lines.
            let (yours, whose, theirs) = self.comparison_parts(knob);
            let words = format!("Yours: {yours} · {whose}: {theirs}");
            egui::Area::new(ui.id().with(("dpl_editor_compare", knob_key(knob))))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(
                    dot.left(),
                    ui.min_rect().bottom() + cl::space::S1,
                ))
                .show(ui.ctx(), |ui| {
                    let sheet = egui::Frame::new()
                        .fill(cl::surface::RAISED)
                        .stroke(Stroke::new(1.0, cl::surface::HAIRLINE))
                        .corner_radius(cl::RADIUS)
                        .inner_margin(egui::Margin::same(10))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("Yours: {yours}"))
                                    .small()
                                    .color(cl::ink::STRONG),
                            );
                            ui.label(
                                RichText::new(format!("{whose}: {theirs}"))
                                    .small()
                                    .color(cl::ink::MUTED),
                            );
                        });
                    ui.pilot_track(
                        &sheet.response,
                        NodeMetadata::new(
                            PilotId::new("editor.compare").child(&PilotKey::new(knob_key(knob))),
                            egui_pilot_ai::protocol::Role::Label,
                            &words,
                        ),
                    );
                });
        }
    }

    /// "Yours: Orchid haze · The look's: Ember drift + Deep lagoon" — the
    /// value the slide holds beside the one it would follow, in words.
    pub fn comparison(&self, knob: &dpl_doc::link::Knob) -> String {
        let (yours, whose, theirs) = self.comparison_parts(knob);
        format!("Yours: {yours} · {whose}: {theirs}")
    }

    /// The comparison's three parts: your value, whose value it would
    /// follow ("The look's" or "The show's"), and that value.
    fn comparison_parts(&self, knob: &dpl_doc::link::Knob) -> (String, &'static str, String) {
        let mut given_back = self.slide.clone();
        dpl_doc::link::reset(&mut given_back, knob);
        let whose = match dpl_doc::link::source(&given_back, knob) {
            dpl_doc::link::Source::Look(_) => "The look's",
            _ => "The show's",
        };
        (
            value_words(&self.slide, &self.theme, knob),
            whose,
            value_words(&given_back, &self.theme, knob),
        )
    }

    // ── mutation helpers ────────────────────────────────────────────────

    /// The background the slide shows: its own, else its look's, else the
    /// show's — the cascade's answer, preset tier included.
    fn inherited_background(&self) -> dpl_doc::Background {
        dpl_doc::resolve(&self.slide, &self.theme)
            .background
            .unwrap_or_else(|| self.theme.background.clone())
    }

    /// A block's fields RESOLVED through the theme, plus whether font/ink
    /// are slide-level overrides (vs inherited from the show).
    fn text_fields(&self, id: &str) -> Option<TextFields> {
        let words = self.slide.block(id)?.words()?;
        // What an unset font inherits: the card's look, then the show.
        let inherited_font = dpl_doc::card_preset(&self.slide)
            .map_or_else(|| self.theme.font.clone(), |look| look.font.to_owned());
        Some(TextFields {
            text: words.text.to_owned(),
            font: words.font.map_or(inherited_font, str::to_owned),
            size: words.size.unwrap_or(DEFAULT_SIZE).to_owned(),
            ink: words
                .ink
                .map_or_else(|| self.theme.ink.clone(), str::to_owned),
        })
    }

    fn set_blend_mood(&mut self, slot: usize, pick: String) {
        // Editing an INHERITED background starts from the theme's values —
        // the first touch materializes an override with the show as base.
        let base = self.inherited_background();
        self.mutate(move |s| {
            let (mut a, mut b, motion, ambience) = match &base {
                dpl_doc::Background::MoodBlend {
                    moods,
                    motion,
                    ambience,
                } => (
                    moods.first().cloned().unwrap_or_default(),
                    moods.get(1).cloned().unwrap_or_default(),
                    motion.clone(),
                    ambience.clone(),
                ),
                dpl_doc::Background::Mood { mood } => {
                    (mood.clone(), mood.clone(), "blend-drift".into(), None)
                }
                dpl_doc::Background::None => {
                    (pick.clone(), pick.clone(), "blend-drift".into(), None)
                }
            };
            if slot == 0 {
                a = pick.clone();
            } else {
                b = pick.clone();
            }
            s.background = Some(dpl_doc::Background::MoodBlend {
                moods: vec![a, b],
                motion,
                ambience,
            });
        });
    }

    fn set_ambience(&mut self, amb: Option<String>) {
        let base = self.inherited_background();
        self.mutate(move |s| {
            let mut bg = base;
            if let dpl_doc::Background::MoodBlend { ambience, .. } = &mut bg {
                *ambience = amb;
            }
            s.background = Some(bg);
        });
    }
}

// ── free helpers ────────────────────────────────────────────────────────

/// Use a card look (LS2b): the slide stores the preset's REFERENCE, never
/// a copy of its values (the July ruling, "not bake"). Picking a look
/// adopts it: the overrides it governs — the background, the colour and
/// each line's font — are cleared, so the look shows (Keynote's "change
/// theme" without keeping style changes). Size and ink stay the slide's.
fn use_look(s: &mut dpl_doc::Slide, id: &str) {
    s.preset = Some(id.to_owned());
    s.background = None;
    s.tone = None;
    for block in &mut s.blocks {
        if let Some(words) = block.words_mut() {
            *words.font = None;
        }
    }
}

/// Edit a text block's (text, font, size, ink) tuple in place. Font/ink
/// become overrides ONLY when the closure actually changes them — an
/// untouched inherited field stays absent on disk (sparse stays sparse).
fn set_text(
    s: &mut dpl_doc::Slide,
    id: &str,
    f: impl FnOnce(&mut (String, String, String, String)),
) {
    for b in &mut s.blocks {
        let Some(words) = b.words_mut() else {
            continue;
        };
        if words.id == id {
            let before = (
                words.text.clone(),
                words.font.clone().unwrap_or_default(),
                words
                    .size
                    .clone()
                    .unwrap_or_else(|| DEFAULT_SIZE.to_owned()),
                words.ink.clone().unwrap_or_default(),
            );
            let mut t = before.clone();
            f(&mut t);
            *words.text = t.0;
            if t.1 != before.1 {
                *words.font = Some(t.1);
            }
            if t.2 != before.2 {
                *words.size = Some(t.2);
            }
            if t.3 != before.3 {
                *words.ink = Some(t.3);
            }
            return;
        }
    }
}

fn mood(name: &str) -> Option<&'static cl::Mood> {
    cl::MOODS.iter().find(|m| m.name == name)
}

/// A horizontal row of mood gradient cards; returns a tapped mood name.
fn mood_row(ui: &mut Ui, salt: &str, current: Option<&str>) -> Option<String> {
    let mut picked = None;
    egui::ScrollArea::horizontal()
        .id_salt(salt)
        .auto_shrink([false, false])
        .max_width(360.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for m in cl::MOODS.iter() {
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(72.0, 44.0), Sense::click());
                    let painter = ui.painter_at(rect);
                    cl::mood_gradient(&painter, rect, m);
                    let on = current == Some(m.name);
                    card_edge_tone(&painter, rect, on || resp.hovered(), on);
                    resp.clone().on_hover_text(m.name);
                    if resp.clicked() {
                        picked = Some(m.name.to_string());
                    }
                }
            });
        });
    picked
}

fn fit_stage(area: Rect) -> Rect {
    // Largest 16:9 rect centered in `area`.
    let (aw, ah) = (area.width(), area.height());
    let (w, h) = if aw / ah > 16.0 / 9.0 {
        (ah * 16.0 / 9.0, ah)
    } else {
        (aw, aw * 9.0 / 16.0)
    };
    Rect::from_center_size(area.center(), egui::vec2(w, h))
}

fn tray_title(t: &str) -> RichText {
    RichText::new(t).strong().color(cl::ink::STRONG)
}
fn small(t: &str) -> RichText {
    RichText::new(t).small().color(cl::ink::SUBTLE)
}

fn card_edge(painter: &egui::Painter, rect: Rect, hot: bool) {
    painter.rect_stroke(
        rect,
        cl::RADIUS_SM,
        Stroke::new(
            1.0,
            if hot {
                cl::ink::MUTED
            } else {
                cl::surface::HAIRLINE
            },
        ),
        StrokeKind::Inside,
    );
}
fn card_edge_tone(painter: &egui::Painter, rect: Rect, hot: bool, on: bool) {
    let c = if on {
        cl::Tone::Amber.color()
    } else if hot {
        cl::ink::MUTED
    } else {
        cl::surface::HAIRLINE
    };
    painter.rect(
        rect,
        cl::RADIUS_SM,
        Color32::TRANSPARENT,
        Stroke::new(if on { 2.0 } else { 1.0 }, c),
        StrokeKind::Inside,
    );
}

/// A touch chip — `tap_min`-tall toggle. Returns true when tapped.
/// A look's name, by id (the id when no such look exists).
fn look_label(id: &str) -> String {
    cl::theme_preset(id).map_or_else(|| id.to_owned(), |look| look.label.to_owned())
}

/// The honest way back for a value that is yours.
fn give_back_label(slide: &dpl_doc::Slide, knob: &dpl_doc::link::Knob) -> &'static str {
    let mut probe = slide.clone();
    dpl_doc::link::reset(&mut probe, knob);
    match dpl_doc::link::source(&probe, knob) {
        dpl_doc::link::Source::Look(_) => "Use the look's",
        _ => "Use the show's",
    }
}

/// What `knob` shows on `slide`, in plain words (through the cascade).
fn value_words(
    slide: &dpl_doc::Slide,
    theme: &dpl_doc::Theme,
    knob: &dpl_doc::link::Knob,
) -> String {
    use dpl_doc::link::{Knob, LineField};
    let shown = dpl_doc::resolve(slide, theme);
    let or_none = |v: Option<&str>| v.map_or_else(|| "None".to_owned(), pretty);
    match knob {
        Knob::Background => match shown.background {
            Some(dpl_doc::Background::Mood { mood }) => pretty(&mood),
            Some(dpl_doc::Background::MoodBlend { moods, .. }) => moods
                .iter()
                .map(|m| pretty(m))
                .collect::<Vec<_>>()
                .join(" + "),
            Some(dpl_doc::Background::None) | None => "Nothing".into(),
        },
        Knob::Colour => or_none(shown.tone.as_deref()),
        Knob::Enter => or_none(shown.enter.as_deref()),
        Knob::Line { id, field } => {
            let Some(block) = shown.block(id) else {
                return "None".into();
            };
            let words = block.words();
            match field {
                LineField::Font => words.and_then(|w| w.font).map_or_else(
                    || "None".into(),
                    |font| {
                        cl::FONTS
                            .iter()
                            .find(|def| def.id == font)
                            .map_or_else(|| pretty(font), |def| def.label.to_owned())
                    },
                ),
                LineField::Size => pretty(words.and_then(|w| w.size).unwrap_or(DEFAULT_SIZE)),
                LineField::Ink => or_none(words.and_then(|w| w.ink)),
                LineField::Colour => match block {
                    dpl_doc::Block::Text { tone, .. }
                    | dpl_doc::Block::Media { tone, .. }
                    | dpl_doc::Block::Quote { tone, .. } => {
                        tone.as_deref().map_or_else(|| "The card's".into(), pretty)
                    }
                    dpl_doc::Block::Data { .. } => "None".into(),
                },
            }
        }
    }
}

/// A registry id as a person would say it: `orchid-haze` → "Orchid haze".
fn pretty(id: &str) -> String {
    let mut out = id.replace(['-', '_'], " ");
    if let Some(first) = out.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    out
}

/// A knob's stable key for pilot ids ("background", "line-title-font").
fn knob_key(knob: &dpl_doc::link::Knob) -> String {
    match knob {
        dpl_doc::link::Knob::Background => "background".into(),
        dpl_doc::link::Knob::Colour => "colour".into(),
        dpl_doc::link::Knob::Enter => "enter".into(),
        dpl_doc::link::Knob::Line { id, field } => format!("line-{id}-{field:?}").to_lowercase(),
    }
}

/// A chip: plain label, tooltip, pilot id `editor/<key>`. True when tapped.
fn chip(ui: &mut Ui, key: &str, label: &str, on: bool, tip: &str) -> bool {
    chip_response(ui, key, label, on, tip).clicked()
}

fn chip_response(ui: &mut Ui, key: &str, label: &str, on: bool, tip: &str) -> egui::Response {
    chip_drawn(ui, key, label, on, false, tip)
}

/// The primary chip of a row (Keep): the house Hero weight — a 2px amber
/// edge and amber words — so the one action that saves stands out.
fn hero_chip(ui: &mut Ui, key: &str, label: &str, tip: &str) -> bool {
    chip_drawn(ui, key, label, false, true, tip).clicked()
}

fn chip_drawn(
    ui: &mut Ui,
    key: &str,
    label: &str,
    on: bool,
    hero: bool,
    tip: &str,
) -> egui::Response {
    let galley =
        ui.painter()
            .layout_no_wrap(label.into(), FontId::proportional(13.0), cl::ink::BASE);
    let w = galley.size().x + 22.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 30.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        cl::RADIUS_SM,
        if on || hero {
            cl::Tone::Amber.wash()
        } else {
            cl::surface::OVERLAY
        },
    );
    if hero {
        painter.rect_stroke(
            rect,
            cl::RADIUS_SM,
            Stroke::new(2.0, cl::Tone::Amber.color()),
            egui::StrokeKind::Inside,
        );
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.0),
        if hero {
            cl::Tone::Amber.color()
        } else if on {
            cl::ink::STRONG
        } else {
            cl::ink::BASE
        },
    );
    ui.add_space(cl::space::S1);
    resp.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Button, true, on, label));
    ui.pilot_track(
        &resp,
        NodeMetadata::new(
            PilotId::new("editor").child(&PilotKey::new(key)),
            egui_pilot_ai::protocol::Role::Button,
            label,
        ),
    );
    cl::tip(resp, tip)
}

/// A big, obvious button (Undo). `enabled=false` dims + ignores taps.
fn big_button(ui: &mut Ui, key: &str, label: &str, enabled: bool, tip: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(84.0, 30.0), Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        CornerRadius::same(cl::RADIUS_SM),
        if enabled && resp.hovered() {
            cl::surface::OVERLAY
        } else {
            cl::surface::RAISED
        },
    );
    painter.rect_stroke(
        rect,
        cl::RADIUS_SM,
        Stroke::new(1.0, cl::surface::HAIRLINE),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.5),
        if enabled {
            cl::ink::BASE
        } else {
            cl::ink::SUBTLE
        },
    );
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    ui.pilot_track(
        &resp,
        NodeMetadata::new(
            PilotId::new("editor").child(&PilotKey::new(key)),
            egui_pilot_ai::protocol::Role::Button,
            label,
        ),
    );
    let resp = cl::tip(resp, tip);
    enabled && resp.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fully specified — the pre-W3b shape; overrides everywhere.
    fn seed() -> dpl_doc::Slide {
        let mut slide = dpl_doc::Slide::new("title@1");
        slide.background = Some(dpl_doc::Background::MoodBlend {
            moods: vec!["ember-drift".into(), "deep-lagoon".into()],
            motion: "blend-drift".into(),
            ambience: None,
        });
        slide.blocks = vec![dpl_doc::Block::Text {
            id: "title".into(),
            preset: None,
            tone: None,
            text: "Hello".into(),
            font: Some("spacegrotesk".into()),
            size: Some("display".into()),
            ink: Some("strong".into()),
        }];
        slide.enter = Some("rise".into());
        slide
    }

    /// Styling-free — everything inherits the theme.
    fn sparse_seed() -> dpl_doc::Slide {
        let mut slide = dpl_doc::Slide::new("title@1");
        slide.blocks = vec![dpl_doc::Block::text("title", "Hello")];
        slide
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir()
            .join("dpl-editor-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn a_font_edit_changes_exactly_one_ref_on_disk() {
        let dir = scratch("font");
        dpl_doc::save(&dir, &seed()).unwrap();
        let mut ed = Editor::open(&dir).unwrap();
        ed.mutate(|s| set_text(s, "title", |b| b.1 = "orbitron".into()));

        let on_disk = dpl_doc::load(&dir).unwrap();
        let dpl_doc::Block::Text {
            font,
            text,
            size,
            ink,
            ..
        } = &on_disk.blocks[0]
        else {
            panic!("block 0 is text");
        };
        assert_eq!(font.as_deref(), Some("orbitron"), "font ref swapped");
        // Nothing else moved.
        assert_eq!(text, "Hello");
        assert_eq!(size.as_deref(), Some("display"));
        assert_eq!(ink.as_deref(), Some("strong"));
        assert_eq!(on_disk.background, seed().background);
        // The edit still validates against the real registries.
        assert!(dpl_doc::validate(&on_disk).is_empty());
    }

    /// Every saved change is a new revision, and undo restores the earlier
    /// CONTENT as a further revision — history never runs backwards.
    #[test]
    fn undo_restores_the_content_as_a_new_revision() {
        let dir = scratch("undo");
        let original = seed();
        dpl_doc::save(&dir, &original).unwrap();
        let mut ed = Editor::open(&dir).unwrap();
        ed.mutate(|s| set_text(s, "title", |b| b.1 = "orbitron".into()));
        assert_eq!(dpl_doc::load(&dir).unwrap().rev, 2);
        // A change that changes nothing is not a revision.
        ed.mutate(|s| set_text(s, "title", |b| b.1 = "orbitron".into()));
        assert_eq!(dpl_doc::load(&dir).unwrap().rev, 2);
        ed.undo();
        let on_disk = dpl_doc::load(&dir).unwrap();
        assert_eq!(on_disk.rev, 3);
        let mut expected = original;
        expected.rev = 3;
        assert_eq!(on_disk, expected);
    }

    /// A quote is a real block (ruling #3) with the quote component's own
    /// name and role slots, and the card fills the quote component.
    #[test]
    fn the_quote_tab_writes_a_quote_block_and_its_slots() {
        let dir = scratch("quote");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir).unwrap();
        ed.set_quote(["Ship it.".into(), "Grace".into(), String::new()]);
        let on_disk = dpl_doc::load(&dir).unwrap();
        assert_eq!(on_disk.component, "quote@1");
        assert!(
            matches!(on_disk.block("quote"), Some(dpl_doc::Block::Quote { text, .. }) if text == "Ship it.")
        );
        assert!(matches!(
            on_disk.block("name"),
            Some(dpl_doc::Block::Text { .. })
        ));
        assert!(
            on_disk.block("role").is_none(),
            "a blank line writes no block"
        );
        assert_eq!(
            ed.quote(),
            ["Ship it.".to_string(), "Grace".into(), String::new()]
        );
        assert!(dpl_doc::validate(&on_disk).is_empty());
    }

    /// The picture fills the component's `media` slot.
    #[test]
    fn a_picture_fills_the_media_slot() {
        let dir = scratch("picture");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir).unwrap();
        ed.set_media(Some(("inbox/x.png".into(), "image".into(), "cover".into())));
        let on_disk = dpl_doc::load(&dir).unwrap();
        assert!(matches!(
            on_disk.block("media"),
            Some(dpl_doc::Block::Media { .. })
        ));
    }

    /// Picking a look stores the REFERENCE (LS2b, the July ruling "not
    /// bake"): the file names the preset and holds none of its values, and
    /// the slide resolves to the look's background and font.
    #[test]
    fn picking_a_look_stores_the_reference() {
        let dir = scratch("theme");
        dpl_doc::save(&dir, &seed()).unwrap();
        let mut ed = Editor::open(&dir).unwrap();
        let p = cl::THEME_PRESETS[0];
        ed.mutate(|s| use_look(s, p.id));
        let on_disk = dpl_doc::load(&dir).unwrap();
        assert_eq!(on_disk.preset.as_deref(), Some(p.id));
        assert_eq!(
            on_disk.background, None,
            "the look's background is not copied"
        );
        let dpl_doc::Block::Text { font, .. } = &on_disk.blocks[0] else {
            panic!("block 0 is text");
        };
        assert_eq!(*font, None, "the look's font is not copied");
        let resolved = dpl_doc::resolve(&on_disk, &dpl_doc::Theme::fallback());
        let dpl_doc::Block::Text { font, .. } = &resolved.blocks[0] else {
            panic!("block 0 is text");
        };
        assert_eq!(font.as_deref(), Some(p.font));
        if let Some(dpl_doc::Background::MoodBlend { moods, .. }) = &resolved.background {
            assert_eq!(moods, &vec![p.moods.0.to_string(), p.moods.1.to_string()]);
        } else {
            panic!("the look gives a mood-blend background");
        }
        assert!(dpl_doc::validate(&on_disk).is_empty());
    }

    #[test]
    fn setting_a_blend_mood_slot_keeps_the_other_and_validates() {
        let dir = scratch("mood");
        dpl_doc::save(&dir, &seed()).unwrap();
        let mut ed = Editor::open(&dir).unwrap();
        ed.set_blend_mood(1, "jade-current".into());
        let on_disk = dpl_doc::load(&dir).unwrap();
        if let Some(dpl_doc::Background::MoodBlend { moods, .. }) = &on_disk.background {
            assert_eq!(moods[0], "ember-drift");
            assert_eq!(moods[1], "jade-current");
        } else {
            panic!("still a blend");
        }
        assert!(dpl_doc::validate(&on_disk).is_empty());
    }

    // ── the cascade in the editor (W3b) ─────────────────────────────────

    #[test]
    fn editing_text_on_a_sparse_slide_does_not_densify_styling() {
        let dir = scratch("sparse-text");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir).unwrap();
        ed.mutate(|s| set_text(s, "title", |b| b.0 = "New words".into()));
        let raw = std::fs::read_to_string(dir.join("slide.json")).unwrap();
        assert!(raw.contains("New words"));
        for absent in ["\"font\"", "\"ink\"", "\"background\""] {
            assert!(!raw.contains(absent), "{absent} crept in: {raw}");
        }
    }

    #[test]
    fn giving_a_font_back_clears_it_on_disk() {
        let dir = scratch("show-theme");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir).unwrap();
        // Make the font yours, then give it back.
        ed.mutate(|s| set_text(s, "title", |b| b.1 = "orbitron".into()));
        let raw = std::fs::read_to_string(dir.join("slide.json")).unwrap();
        assert!(raw.contains("\"font\": \"orbitron\""), "yours, on disk");
        let knob = dpl_doc::link::Knob::Line {
            id: "title".into(),
            field: dpl_doc::link::LineField::Font,
        };
        assert_eq!(ed.source(&knob), dpl_doc::link::Source::Yours);
        ed.reset(&knob);
        let raw = std::fs::read_to_string(dir.join("slide.json")).unwrap();
        assert!(!raw.contains("\"font\""), "given back on disk: {raw}");
        // The tray resolves back to the show's font.
        let tf = ed.text_fields("title").unwrap();
        assert_eq!(tf.font, dpl_doc::Theme::fallback().font);
        assert_eq!(ed.source(&knob), dpl_doc::link::Source::Show);
    }

    /// The dot's comparison names the value that is yours and the one the
    /// slide would follow — the look's while it follows one.
    #[test]
    fn the_comparison_names_yours_and_the_looks() {
        let dir = scratch("compare");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir).unwrap();
        let look = cl::THEME_PRESETS
            .iter()
            .find(|p| p.id == "neon-intro")
            .unwrap();
        ed.mutate(|s| use_look(s, look.id));
        ed.mutate(|s| {
            s.background = Some(dpl_doc::Background::Mood {
                mood: "orchid-haze".into(),
            });
        });
        assert_eq!(
            ed.comparison(&dpl_doc::link::Knob::Background),
            format!(
                "Yours: Orchid haze · The look's: {} + {}",
                pretty(look.moods.0),
                pretty(look.moods.1)
            )
        );
        let font = dpl_doc::link::Knob::Line {
            id: "title".into(),
            field: dpl_doc::link::LineField::Font,
        };
        ed.mutate(|s| set_text(s, "title", |b| b.1 = "vt323".into()));
        assert!(
            ed.comparison(&font).ends_with(&format!(
                "The look's: {}",
                cl::FONTS.iter().find(|d| d.id == look.font).unwrap().label
            )),
            "{}",
            ed.comparison(&font)
        );
    }

    /// Typing is one revision and one undo step, not one per letter.
    #[test]
    fn a_typing_burst_is_one_revision() {
        let dir = scratch("burst");
        dpl_doc::save(&dir, &sparse_seed()).unwrap();
        let mut ed = Editor::open(&dir).unwrap();
        for word in ["H", "He", "Hel", "Hell", "Hello!"] {
            let word = word.to_string();
            ed.mutate_burst("text-title", |s| {
                set_text(s, "title", |b| b.0 = word.clone())
            });
        }
        let on_disk = dpl_doc::load(&dir).unwrap();
        assert_eq!(on_disk.rev, 2, "one revision for the burst");
        assert!(
            matches!(on_disk.block("title"), Some(dpl_doc::Block::Text { text, .. }) if text == "Hello!")
        );
        ed.undo();
        assert!(!ed.can_undo(), "one undo step took the whole burst back");
        assert!(
            matches!(dpl_doc::load(&dir).unwrap().block("title"), Some(dpl_doc::Block::Text { text, .. }) if text == "Hello")
        );
    }

    /// Stopping following changes nothing on screen; following again with
    /// a fresh start gives the look's values back (LK1).
    #[test]
    fn stop_and_follow_again_through_the_editor() {
        let dir = scratch("link");
        let mut slide = sparse_seed();
        slide.preset = Some(cl::THEME_PRESETS[0].id.into());
        dpl_doc::save(&dir, &slide).unwrap();
        let mut ed = Editor::open(&dir).unwrap();
        let theme = dpl_doc::Theme::fallback();
        let before = dpl_doc::resolve(ed.slide(), &theme);
        ed.stop_following();
        assert!(matches!(
            ed.link_state(),
            dpl_doc::link::LinkState::OwnStyle { .. }
        ));
        assert_eq!(dpl_doc::resolve(ed.slide(), &theme).blocks, before.blocks);
        ed.follow_again(false);
        assert!(matches!(
            ed.link_state(),
            dpl_doc::link::LinkState::Follows { changes: 0, .. }
        ));
        assert!(dpl_doc::validate(&dpl_doc::load(&dir).unwrap()).is_empty());
    }
}
