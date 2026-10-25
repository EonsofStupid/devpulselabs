//! The DeckHost (D2) — the show machine's runtime surface.
//!
//! An episode or a lesson (RL) becomes a full-bleed presentation: beat-wise
//! navigation (the verbatim [`nav`] grammar), touch made for the big
//! screen — tap anywhere for the next beat, the left edge to go back, a
//! swipe either way, a long press for the navigator (RL, ruled 2026-10-25
//! "Tap anywhere + presenter drives") — a progress rail with chapter
//! ticks, a bottom toolbar, a navigator overlay, and the chrome-hide escape
//! hatches — the bottom edge-reveal strip and the always-there home dot
//! ("ALWAYS a way back on a touchscreen").
//!
//! Rendering rides the ONE document path (`dpl_slides::document::
//! render_step`); a slide that fails to load degrades to an honest labeled
//! plate, never a crash. Ink/annotate is D3; presenter+sync is D4.

pub mod ink;
pub mod nav;

use dpl_coldlight::{self as cl};
use dpl_doc::episode::Showing;
use dpl_slides::document;
use egui::{Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Ui};
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::path::Path;

/// Where a touch on the stage goes (RL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    Next,
    Back,
}

/// The share of the stage, from its left edge, where a tap goes back.
pub const BACK_ZONE: f32 = 0.15;
/// The shortest drag, in points, that counts as a swipe.
pub const SWIPE_MIN: f32 = 60.0;

/// What a tap at `x` means: the left [`BACK_ZONE`] of the stage goes back,
/// anywhere else goes to the next beat.
pub fn tap_intent(x: f32, stage: Rect) -> Move {
    if x < stage.min.x + stage.width() * BACK_ZONE {
        Move::Back
    } else {
        Move::Next
    }
}

/// What a drag of `delta` means: a mostly-sideways drag of at least
/// [`SWIPE_MIN`] is a swipe — leftward to the next beat, rightward back;
/// anything else is not a move.
pub fn swipe_intent(delta: egui::Vec2) -> Option<Move> {
    let sideways = delta.x.abs();
    if sideways >= SWIPE_MIN && sideways > delta.y.abs() * 1.5 {
        Some(if delta.x < 0.0 {
            Move::Next
        } else {
            Move::Back
        })
    } else {
        None
    }
}

/// What the host wants from its mount after a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckAction {
    None,
    /// The operator asked to leave the show (toolbar Home / home dot).
    Exit,
    /// The toolbar Cue button — the mount opens/closes the presenter
    /// teleprompter (D4).
    TogglePresenter,
}

/// One slide in the flat walk: its chapter's title, what the card is
/// called, and the loaded document — or the honest load error.
struct DeckSlide {
    chapter: String,
    name: String,
    doc: Result<dpl_doc::Slide, String>,
}

/// A chapter marker on the progress rail: (flat slide index, title).
struct Marker {
    index: usize,
    title: String,
}

pub struct DeckHost {
    title: String,
    /// The show's theme and the looks its slides may follow (LK2).
    styling: dpl_doc::Styling,
    slides: Vec<DeckSlide>,
    /// Per-slide step counts, derived once — the nav math's whole world.
    steps: Vec<usize>,
    markers: Vec<Marker>,
    pos: nav::Position,
    chrome_hidden: bool,
    navigator_open: bool,
    /// The ink layer (D3): stream markup, cleared when the SLIDE changes
    /// (steps keep it — the old slide-key law).
    annotate_on: bool,
    ink: ink::InkLayer,
    /// Restarted on EVERY beat — each beat animates exactly one thing.
    clock: dpl_slides::motion::Clock,
    /// The sync ear's other end (D4): every real move publishes a
    /// `DeckEvent::Beat` on the fabric's deck domain. None = unwired
    /// (headless tests, standalone mounts) — the deck never requires it.
    fabric: Option<dpl_fabric::Fabric>,
    /// The toolbar Cue button fired this frame — `ui()` converts it to
    /// [`DeckAction::TogglePresenter`] (Exit wins when both fire).
    presenter_asked: bool,
    /// Where the drag in progress on the stage began — a swipe is judged
    /// from here when it ends.
    swipe_from: Option<egui::Pos2>,
}

impl DeckHost {
    /// Open a show — an episode folder or a lesson (RL) — through the one
    /// walk, so an episode built from lessons (found in `content_root`'s
    /// library), a seed template and a lesson on its own play alike, with
    /// the looks its slides may follow (the built-in ones plus the
    /// operator's own). It starts at card `start` (clamped to the show).
    pub fn open(
        showing: &Showing,
        content_root: &Path,
        looks: dpl_doc::Looks,
        start: usize,
    ) -> Result<Self, String> {
        let shown = showing.shown(content_root).map_err(|e| e.to_string())?;
        let mut slides = Vec::new();
        let mut markers: Vec<Marker> = Vec::new();
        for chapter in &shown.chapters {
            if let Some(first) = chapter.cards.first() {
                markers.push(Marker {
                    index: first.index,
                    title: chapter.title.clone(),
                });
            }
            for card in &chapter.cards {
                slides.push(DeckSlide {
                    chapter: chapter.title.clone(),
                    name: card.label.clone(),
                    doc: card.slide.clone(),
                });
            }
        }
        if slides.is_empty() {
            return Err("there are no cards to show".into());
        }
        let steps = slides
            .iter()
            .map(|s| s.doc.as_ref().map(|d| d.step_count()).unwrap_or(1))
            .collect();
        let start = nav::clamp_index(start, slides.len());
        Ok(Self {
            title: shown.title,
            styling: dpl_doc::Styling {
                theme: shown.theme,
                looks,
            },
            slides,
            steps,
            pos: nav::Position {
                index: start,
                step: 0,
            },
            markers,
            chrome_hidden: false,
            navigator_open: false,
            annotate_on: false,
            ink: ink::InkLayer::default(),
            clock: dpl_slides::motion::Clock::default(),
            fabric: None,
            presenter_asked: false,
            swipe_from: None,
        })
    }

    /// Wire the sync channel (D4): from here on every real move
    /// publishes a `DeckEvent::Beat` the presenter consumes.
    pub fn wire_fabric(&mut self, fabric: dpl_fabric::Fabric) {
        self.fabric = Some(fabric);
    }

    /// Publish the CURRENT position — the presenter's opening snapshot
    /// (a fresh subscriber only hears traffic after it subscribes).
    pub fn publish_position(&self) {
        if let Some(f) = &self.fabric {
            f.publish_deck(dpl_fabric::DeckEvent::Beat {
                index: self.pos.index,
                step: self.pos.step,
                slides: self.slides.len(),
                steps: self.steps[self.pos.index],
            });
        }
    }

    pub fn position(&self) -> nav::Position {
        self.pos
    }

    fn go(&mut self, pos: nav::Position) {
        if pos != self.pos {
            // Ink is per-SLIDE: a new slide clears it, a step keeps it.
            if pos.index != self.pos.index {
                self.ink.clear();
            }
            self.pos = pos;
            self.clock.restart();
            self.publish_position();
        }
    }

    /// The next beat — the next tap's line, or the next card.
    pub fn next(&mut self) {
        self.go(nav::advance(&self.steps, self.pos));
    }

    /// The beat before — back to the last tap of the card before, at a
    /// card's start.
    pub fn back(&mut self) {
        self.go(nav::retreat(&self.steps, self.pos));
    }

    /// Straight to card `index` at tap `step` (both clamped) — the
    /// presenter's tap-to-jump.
    pub fn jump(&mut self, index: usize, step: usize) {
        let index = nav::clamp_index(index, self.steps.len());
        let step = step.min(self.steps[index].saturating_sub(1));
        self.go(nav::Position { index, step });
    }

    fn beat_next(&mut self) {
        self.next();
    }
    fn beat_prev(&mut self) {
        self.back();
    }
    fn slide_next(&mut self) {
        let i = nav::clamp_index(self.pos.index + 1, self.steps.len());
        self.go(nav::Position { index: i, step: 0 });
    }
    fn slide_prev(&mut self) {
        let i = self.pos.index.saturating_sub(1);
        self.go(nav::Position { index: i, step: 0 });
    }

    /// The keyboard map, ported: ←/PgUp retreat a beat; →/PgDn/Space
    /// advance; ↑/↓ jump whole slides; G navigator; H chrome. (A/annotate
    /// is D3.) One code path — `ui()` feeds real key presses through here.
    pub fn press(&mut self, key: egui::Key) {
        match key {
            egui::Key::ArrowLeft | egui::Key::PageUp => self.beat_prev(),
            egui::Key::ArrowRight | egui::Key::PageDown | egui::Key::Space => self.beat_next(),
            egui::Key::ArrowUp => self.slide_prev(),
            egui::Key::ArrowDown => self.slide_next(),
            egui::Key::G => self.navigator_open = !self.navigator_open,
            egui::Key::H => self.chrome_hidden = !self.chrome_hidden,
            egui::Key::A => self.annotate_on = !self.annotate_on,
            _ => {}
        }
    }

    /// One frame of the deck. The mount gives it the whole rect it owns
    /// (full-bleed when presenting).
    pub fn ui(&mut self, ui: &mut Ui) -> DeckAction {
        let rect = ui.max_rect();
        let mut action = DeckAction::None;

        // Real input → the one key path.
        let pressed: Vec<egui::Key> = ui.input(|i| {
            [
                egui::Key::ArrowLeft,
                egui::Key::ArrowRight,
                egui::Key::ArrowUp,
                egui::Key::ArrowDown,
                egui::Key::PageUp,
                egui::Key::PageDown,
                egui::Key::Space,
                egui::Key::G,
                egui::Key::H,
                egui::Key::A,
            ]
            .into_iter()
            .filter(|k| i.key_pressed(*k))
            .collect()
        });
        for k in pressed {
            self.press(k);
        }

        // The slide, full bleed, through the one document path.
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        match &self.slides[self.pos.index].doc {
            Ok(slide) => {
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                child.set_clip_rect(rect);
                let _ = document::render_step(
                    &mut child,
                    rect,
                    slide,
                    &self.styling,
                    self.pos.step,
                    now,
                    t0,
                );
            }
            Err(e) => {
                ui.painter().rect_filled(rect, 0, cl::surface::WELL);
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    format!("slide does not load: {e}"),
                    FontId::monospace(13.0),
                    cl::ink::SUBTLE,
                );
            }
        }

        // The ink, above the slide and below every control.
        self.ink.paint(ui, rect);

        // The stage answers touch (RL): the left [`BACK_ZONE`] goes back,
        // everywhere else goes next; a swipe either way; a long press (or a
        // right click) opens the navigator. Always live; the chevrons only
        // PAINT while chrome shows. Registered first, so the toolbar, rail
        // and navigator — drawn later — win their own taps.
        let zone_w = rect.width() * BACK_ZONE;
        let left = Rect::from_min_size(rect.min, egui::vec2(zone_w, rect.height()));
        let right = Rect::from_min_max(egui::pos2(rect.min.x + zone_w, rect.min.y), rect.max);
        let lr = ui.interact(left, ui.id().with("deck_prev"), Sense::click_and_drag());
        let rr = ui.interact(right, ui.id().with("deck_next"), Sense::click_and_drag());
        ui.pilot_track(
            &lr,
            NodeMetadata::new(
                PilotId::new("deck").child(&PilotKey::new("prev")),
                egui_pilot_ai::protocol::Role::Button,
                "Previous beat",
            )
            .value(if nav::can_retreat(&self.steps, self.pos) {
                "enabled"
            } else {
                "disabled"
            }),
        );
        ui.pilot_track(
            &rr,
            NodeMetadata::new(
                PilotId::new("deck").child(&PilotKey::new("next")),
                egui_pilot_ai::protocol::Role::Button,
                "Next beat",
            )
            .value(if nav::can_advance(&self.steps, self.pos) {
                "enabled"
            } else {
                "disabled"
            }),
        );
        if !self.annotate_on {
            for resp in [&lr, &rr] {
                if resp.long_touched() || resp.secondary_clicked() {
                    self.navigator_open = true;
                } else if resp.clicked() {
                    if let Some(at) = resp.interact_pointer_pos() {
                        match tap_intent(at.x, rect) {
                            Move::Next => self.beat_next(),
                            Move::Back => self.beat_prev(),
                        }
                    }
                }
                if resp.drag_started() {
                    self.swipe_from = ui.input(|i| i.pointer.press_origin());
                }
                if resp.drag_stopped() {
                    let to = ui.input(|i| i.pointer.interact_pos());
                    if let (Some(from), Some(to)) = (self.swipe_from.take(), to) {
                        match swipe_intent(to - from) {
                            Some(Move::Next) => self.beat_next(),
                            Some(Move::Back) => self.beat_prev(),
                            None => {}
                        }
                    }
                }
            }
        }
        if !self.chrome_hidden && !self.annotate_on {
            chevron(
                ui,
                left,
                "‹",
                nav::can_retreat(&self.steps, self.pos),
                false,
            );
            chevron(
                ui,
                right,
                "›",
                nav::can_advance(&self.steps, self.pos),
                true,
            );
        }

        // Annotating: a full-rect drag capture registered AFTER the zones
        // (it wins — an edge tap can't fire mid-stroke; the old "EdgeNav
        // hidden while annotating").
        if self.annotate_on {
            let dr = ui.interact(rect, ui.id().with("deck_ink"), Sense::drag());
            // Touch force when the screen reports it; mouse = the old 0.5.
            let force = ui.input(|i| {
                i.events.iter().rev().find_map(|e| match e {
                    egui::Event::Touch { force, .. } => *force,
                    _ => None,
                })
            });
            if let Some(pos) = dr.interact_pointer_pos() {
                let p = ink::normalize(pos, rect);
                if dr.drag_started() {
                    self.ink.begin(p, force);
                } else if dr.dragged() {
                    self.ink.extend(p, force);
                }
            }
            if dr.drag_stopped() {
                self.ink.finish();
            }
            ui.pilot_track(
                &dr,
                NodeMetadata::new(
                    PilotId::new("deck").child(&PilotKey::new("ink")),
                    egui_pilot_ai::protocol::Role::Region,
                    "Ink surface",
                )
                .value(if self.ink.is_empty() {
                    "empty"
                } else {
                    "inked"
                }),
            );
            // The Clear chip, bottom-right above the toolbar.
            let fl = cl::Fluid::of(rect);
            let chip = Rect::from_min_size(
                egui::pos2(
                    rect.max.x - 120.0 - cl::space::S4,
                    rect.max.y - fl.tap_min() * 2.0 - cl::space::S6 * 2.0,
                ),
                egui::vec2(120.0, 34.0),
            );
            if tool_button(ui, chip, "Clear ink", "deck_ink_clear", "Wipe the markup") {
                self.ink.clear();
            }
            self.track_button(ui, chip, "ink.clear", "Clear ink");
        }

        // Progress rail: top strip, fill = (index+1)/total, chapter ticks.
        if !self.chrome_hidden {
            self.progress_rail(ui, rect);
        }

        // Toolbar (bottom) — painted after the zones so its buttons win.
        if !self.chrome_hidden {
            if self.toolbar(ui, rect) {
                action = DeckAction::Exit;
            }
        } else {
            // Chrome hidden: the bottom 7% strip restores it; the home dot
            // (always visible, quiet) exits the show.
            let strip = Rect::from_min_max(
                egui::pos2(rect.min.x, rect.max.y - rect.height() * 0.07),
                rect.max,
            );
            let sr = ui.interact(strip, ui.id().with("deck_reveal"), Sense::click());
            if sr.clicked() {
                self.chrome_hidden = false;
            }
            if home_dot(ui, rect) {
                action = DeckAction::Exit;
            }
        }

        // The navigator overlay, above everything.
        if self.navigator_open {
            self.navigator(ui, rect);
        }

        // The Cue ask, unless Exit already won this frame.
        if self.presenter_asked {
            self.presenter_asked = false;
            if action == DeckAction::None {
                action = DeckAction::TogglePresenter;
            }
        }

        ui.ctx().request_repaint(); // backgrounds never rest
        action
    }

    fn progress_rail(&self, ui: &mut Ui, rect: Rect) {
        let total = self.slides.len();
        let rail = Rect::from_min_size(rect.min, egui::vec2(rect.width(), 4.0));
        let painter = ui.painter().clone();
        painter.rect_filled(rail, 0, cl::surface::RAISED);
        let pct = if total <= 1 {
            1.0
        } else {
            (self.pos.index + 1) as f32 / total as f32
        };
        let fill = Rect::from_min_size(rail.min, egui::vec2(rail.width() * pct, rail.height()));
        painter.rect_filled(fill, 0, cl::Tone::Coral.color());
        for m in &self.markers {
            if m.index == 0 {
                continue;
            }
            let x = rail.min.x + rail.width() * (m.index as f32 / total as f32);
            let tick =
                Rect::from_min_size(egui::pos2(x, rail.min.y), egui::vec2(1.0, rail.height()));
            painter.rect_filled(tick, 0, cl::ink::MUTED);
            // The chapter title rides the tick (the old rail's title=).
            ui.interact(
                tick.expand2(egui::vec2(4.0, 3.0)),
                ui.id().with(("deck_tick", m.index)),
                Sense::hover(),
            )
            .on_hover_text(&m.title);
        }
    }

    /// The bottom toolbar. Returns true when Home (exit) was tapped.
    fn toolbar(&mut self, ui: &mut Ui, rect: Rect) -> bool {
        let fl = cl::Fluid::of(rect);
        let h = fl.tap_min() + cl::space::S3 * 2.0;
        let bar = Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - h), rect.max);
        let painter = ui.painter().clone();
        painter.rect_filled(bar, 0, Color32::from_black_alpha(150));
        painter.rect_filled(
            Rect::from_min_size(bar.min, egui::vec2(bar.width(), 1.0)),
            0,
            cl::surface::HAIRLINE,
        );

        let mut exit = false;
        let pad = cl::space::S4;
        let btn = fl.tap_min();

        // Left: Home, Navigator.
        let home = Rect::from_center_size(
            egui::pos2(bar.min.x + pad + btn * 0.5, bar.center().y),
            egui::vec2(btn, btn),
        );
        if tool_button(ui, home, cl::icon::HOME, "deck_home", "Leave the show") {
            exit = true;
        }
        self.track_button(ui, home, "home", "Leave the show");
        let navi = home.translate(egui::vec2(btn + cl::space::S2, 0.0));
        if tool_button(
            ui,
            navi,
            cl::icon::LIBRARY,
            "deck_navigator",
            "All slides (G)",
        ) {
            self.navigator_open = !self.navigator_open;
        }
        self.track_button(ui, navi, "navigator", "All slides");
        let inkb = navi.translate(egui::vec2(btn + cl::space::S2, 0.0));
        if tool_button(ui, inkb, "Ink", "deck_ink_toggle", "Draw on the slide (A)") {
            self.annotate_on = !self.annotate_on;
        }
        self.track_button(ui, inkb, "ink.toggle", "Toggle ink");
        // Cue (D4): the mount opens/closes the presenter teleprompter.
        let cue = inkb.translate(egui::vec2(btn + cl::space::S2, 0.0));
        if tool_button(ui, cue, "Cue", "deck_presenter", "Presenter window") {
            self.presenter_asked = true;
        }
        self.track_button(ui, cue, "presenter", "Presenter window");

        // Right: counter + chrome eye.
        let eye = Rect::from_center_size(
            egui::pos2(bar.max.x - pad - btn * 0.5, bar.center().y),
            egui::vec2(btn, btn),
        );
        if tool_button(
            ui,
            eye,
            cl::icon::FULLBLEED,
            "deck_chrome",
            "Hide controls (H)",
        ) {
            self.chrome_hidden = true;
        }
        self.track_button(ui, eye, "chrome", "Hide controls");
        let steps = self.steps[self.pos.index];
        let counter = if steps > 1 {
            format!(
                "{:02} / {:02} · step {}/{}",
                self.pos.index + 1,
                self.slides.len(),
                self.pos.step,
                steps - 1
            )
        } else {
            format!("{:02} / {:02}", self.pos.index + 1, self.slides.len())
        };
        painter.text(
            egui::pos2(eye.min.x - cl::space::S3, bar.center().y),
            Align2::RIGHT_CENTER,
            counter,
            FontId::monospace(13.0),
            cl::ink::MUTED,
        );

        // Center: episode title + chapter · slide.
        let here = &self.slides[self.pos.index];
        painter.text(
            egui::pos2(bar.center().x, bar.center().y - 9.0),
            Align2::CENTER_CENTER,
            self.title.to_uppercase(),
            FontId::monospace(10.5),
            cl::ink::SUBTLE,
        );
        painter.text(
            egui::pos2(bar.center().x, bar.center().y + 9.0),
            Align2::CENTER_CENTER,
            format!("{} · {}", here.chapter, here.name),
            FontId::proportional(14.0),
            cl::ink::BASE,
        );
        exit
    }

    fn track_button(&self, ui: &mut Ui, rect: Rect, key: &str, label: &str) {
        let resp = ui.interact(rect, ui.id().with(("deck_track", key)), Sense::hover());
        ui.pilot_track(
            &resp,
            NodeMetadata::new(
                PilotId::new("deck").child(&PilotKey::new(key)),
                egui_pilot_ai::protocol::Role::Button,
                label,
            ),
        );
    }

    /// The navigator overlay: every slide as a card; tap jumps to
    /// `{ index, step 0 }`.
    fn navigator(&mut self, ui: &mut Ui, rect: Rect) {
        let painter = ui.painter().clone();
        painter.rect_filled(rect, 0, Color32::from_black_alpha(230));
        painter.text(
            egui::pos2(rect.min.x + cl::space::S6, rect.min.y + cl::space::S6),
            Align2::LEFT_CENTER,
            format!("NAVIGATOR — {}", self.title),
            FontId::monospace(13.0),
            cl::Tone::Coral.color(),
        );
        let close = Rect::from_center_size(
            egui::pos2(
                rect.max.x - cl::space::S6 - 14.0,
                rect.min.y + cl::space::S6,
            ),
            egui::vec2(34.0, 30.0),
        );
        if tool_button(ui, close, cl::icon::CLOSE, "deck_nav_close", "Close (G)") {
            self.navigator_open = false;
        }

        // The card grid.
        let cols = ((rect.width() / 300.0).floor() as usize).max(1);
        let card_w =
            (rect.width() - cl::space::S6 * 2.0 - cl::space::S3 * (cols - 1) as f32) / cols as f32;
        let card_h = card_w * 9.0 / 32.0 + 26.0;
        let top = rect.min.y + cl::space::S6 * 2.0;
        let mut jump: Option<usize> = None;
        for (i, s) in self.slides.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let card = Rect::from_min_size(
                egui::pos2(
                    rect.min.x + cl::space::S6 + (card_w + cl::space::S3) * col as f32,
                    top + (card_h + cl::space::S3) * row as f32,
                ),
                egui::vec2(card_w, card_h),
            );
            if card.max.y > rect.max.y {
                break; // navigator paging rides D4's polish
            }
            let resp = ui.interact(card, ui.id().with(("deck_nav", i)), Sense::click());
            let active = i == self.pos.index;
            let painter = ui.painter().clone();
            painter.rect_filled(
                card,
                CornerRadius::same(cl::RADIUS_SM),
                if active {
                    cl::Tone::Coral.wash()
                } else if resp.hovered() {
                    cl::surface::OVERLAY
                } else {
                    cl::surface::RAISED
                },
            );
            painter.rect_stroke(
                card,
                cl::RADIUS_SM,
                Stroke::new(1.0, cl::surface::HAIRLINE),
                StrokeKind::Inside,
            );
            painter.text(
                card.left_top() + egui::vec2(10.0, 16.0),
                Align2::LEFT_CENTER,
                format!("{:02} · {}", i + 1, s.chapter),
                FontId::monospace(10.5),
                cl::ink::SUBTLE,
            );
            painter.text(
                card.left_top() + egui::vec2(10.0, 38.0),
                Align2::LEFT_CENTER,
                &s.name,
                FontId::proportional(15.0),
                if active {
                    cl::ink::STRONG
                } else {
                    cl::ink::BASE
                },
            );
            if resp.clicked() {
                jump = Some(i);
            }
        }
        if let Some(i) = jump {
            self.go(nav::Position { index: i, step: 0 });
            self.navigator_open = false;
        }
    }
}

/// The edge chevron pill — painted only while chrome shows; 30% alpha
/// when the direction can't move.
fn chevron(ui: &Ui, zone: Rect, glyph: &str, enabled: bool, right: bool) {
    let x = if right {
        zone.max.x - 26.0
    } else {
        zone.min.x + 26.0
    };
    let c = egui::pos2(x, zone.center().y);
    let painter = ui.painter();
    let alpha = if enabled { 200 } else { 60 };
    painter.circle_filled(c, 18.0, Color32::from_black_alpha(alpha.min(140)));
    painter.text(
        c,
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(22.0),
        cl::ink::BASE.gamma_multiply(if enabled { 1.0 } else { 0.3 }),
    );
}

/// A quiet icon button on the deck chrome. Returns clicked.
fn tool_button(ui: &mut Ui, rect: Rect, glyph: &str, salt: &str, tip: &str) -> bool {
    let resp = ui.interact(rect, ui.id().with(salt), Sense::click());
    let painter = ui.painter().clone();
    painter.rect_filled(
        rect,
        CornerRadius::same(cl::RADIUS_SM),
        if resp.hovered() {
            cl::surface::OVERLAY
        } else {
            cl::surface::RAISED
        },
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(16.0),
        if resp.hovered() {
            cl::ink::STRONG
        } else {
            cl::ink::BASE
        },
    );
    resp.clone().on_hover_text(tip);
    resp.clicked()
}

/// The home dot — bottom-left, quiet (40%), full strength on hover; the
/// one-tap exit while the show hides its chrome.
fn home_dot(ui: &mut Ui, rect: Rect) -> bool {
    let c = egui::pos2(rect.min.x + 34.0, rect.max.y - 34.0);
    let dot = Rect::from_center_size(c, egui::vec2(44.0, 44.0));
    let resp = ui.interact(dot, ui.id().with("deck_home_dot"), Sense::click());
    let strong = resp.hovered() || resp.is_pointer_button_down_on();
    let painter = ui.painter().clone();
    painter.circle_filled(
        c,
        22.0,
        Color32::from_black_alpha(if strong { 200 } else { 90 }),
    );
    painter.text(
        c,
        Align2::CENTER_CENTER,
        cl::icon::HOME,
        FontId::proportional(17.0),
        if strong {
            cl::ink::STRONG
        } else {
            cl::ink::BASE.gamma_multiply(0.4)
        },
    );
    resp.clicked()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A scratch episode shaped like the seed show: chapter one holds a
    /// 1-step and a 2-step (one reveal) slide, chapter two a 1-step.
    fn scratch_episode(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("dpl-deck-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let slide = |name: &str, reveals: Vec<String>| {
            let mut doc = dpl_doc::Slide::new("title@1");
            doc.blocks = vec![
                dpl_doc::Block::text("title", format!("slide {name}")),
                dpl_doc::Block::text("sub", "beat"),
            ];
            doc.reveals = reveals;
            dpl_doc::save(&dir.join(name), &doc).unwrap();
        };
        slide("a", vec![]);
        slide("b", vec!["sub".into()]);
        slide("c", vec![]);
        let ep = dpl_doc::Episode {
            v: dpl_doc::SCHEMA_V,
            id: "t".into(),
            title: "Test show".into(),
            chapters: vec![
                dpl_doc::Chapter {
                    id: "one".into(),
                    title: "One".into(),
                    slides: vec!["a".into(), "b".into()],
                },
                dpl_doc::Chapter {
                    id: "two".into(),
                    title: "Two".into(),
                    slides: vec!["c".into()],
                },
            ],
        };
        dpl_doc::save_episode(&dir, &ep).unwrap();
        dir
    }

    #[test]
    fn open_builds_steps_and_markers_from_the_documents() {
        let host = DeckHost::open(
            &Showing::Episode(scratch_episode("open")),
            Path::new("/nowhere"),
            dpl_doc::Looks::builtin_only(),
            0,
        )
        .unwrap();
        assert_eq!(host.steps, vec![1, 2, 1]);
        assert_eq!(host.slides.len(), 3);
        let marks: Vec<usize> = host.markers.iter().map(|m| m.index).collect();
        assert_eq!(marks, vec![0, 2], "chapter two starts at flat index 2");
    }

    #[test]
    fn the_keyboard_walks_the_whole_beat_list_and_holds() {
        let mut host = DeckHost::open(
            &Showing::Episode(scratch_episode("keys")),
            Path::new("/nowhere"),
            dpl_doc::Looks::builtin_only(),
            0,
        )
        .unwrap();
        let p = |i, s| nav::Position { index: i, step: s };
        let mut trace = vec![host.position()];
        for _ in 0..6 {
            host.press(egui::Key::ArrowRight);
            trace.push(host.position());
        }
        assert_eq!(
            trace,
            vec![
                p(0, 0),
                p(1, 0),
                p(1, 1),
                p(2, 0),
                p(2, 0),
                p(2, 0),
                p(2, 0)
            ],
            "the beat list, including the reveal beat, then holding"
        );
        host.press(egui::Key::ArrowLeft);
        assert_eq!(host.position(), p(1, 1), "retreat lands on the LAST step");
        host.press(egui::Key::ArrowUp);
        assert_eq!(host.position(), p(0, 0), "slide jump resets the step");
        host.press(egui::Key::H);
        assert!(host.chrome_hidden);
        host.press(egui::Key::G);
        assert!(host.navigator_open);
    }

    /// D4: a wired deck publishes every real move on the fabric's deck
    /// domain; no-op moves publish nothing; the snapshot seam works.
    #[test]
    fn advancing_publishes_the_beat_on_the_deck_channel() {
        let mut host = DeckHost::open(
            &Showing::Episode(scratch_episode("sync")),
            Path::new("/nowhere"),
            dpl_doc::Looks::builtin_only(),
            0,
        )
        .unwrap();
        let fabric = dpl_fabric::Fabric::new();
        let mut rx = fabric.subscribe_deck();
        host.wire_fabric(fabric);
        host.press(egui::Key::ArrowRight);
        assert_eq!(
            rx.try_recv().unwrap(),
            dpl_fabric::DeckEvent::Beat {
                index: 1,
                step: 0,
                slides: 3,
                steps: 2
            }
        );
        // Holding at the end is a no-op — nothing publishes.
        for _ in 0..6 {
            host.press(egui::Key::ArrowRight);
        }
        while rx.try_recv().is_ok() {}
        host.press(egui::Key::ArrowRight);
        assert!(rx.try_recv().is_err(), "a held position publishes nothing");
        // The presenter's opening snapshot: current position on demand.
        host.publish_position();
        assert_eq!(
            rx.try_recv().unwrap(),
            dpl_fabric::DeckEvent::Beat {
                index: 2,
                step: 0,
                slides: 3,
                steps: 1
            }
        );
    }

    /// RL's touch: the left edge goes back, anywhere else next; a long
    /// sideways drag is a swipe, a short or upright one is not a move.
    #[test]
    fn a_tap_or_a_swipe_says_where_to_go() {
        let stage = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 600.0));
        assert_eq!(tap_intent(100.0, stage), Move::Back);
        assert_eq!(tap_intent(160.0, stage), Move::Next);
        assert_eq!(tap_intent(990.0, stage), Move::Next);
        assert_eq!(swipe_intent(egui::vec2(-120.0, 10.0)), Some(Move::Next));
        assert_eq!(swipe_intent(egui::vec2(140.0, -20.0)), Some(Move::Back));
        assert_eq!(swipe_intent(egui::vec2(-30.0, 0.0)), None, "too short");
        assert_eq!(
            swipe_intent(egui::vec2(-80.0, 90.0)),
            None,
            "mostly upright"
        );
    }

    /// A run can start at any card, and the presenter can jump straight to
    /// a card's tap; both clamp to the show.
    #[test]
    fn a_run_starts_where_it_is_asked_and_jumps_clamp() {
        let mut host = DeckHost::open(
            &Showing::Episode(scratch_episode("start")),
            Path::new("/nowhere"),
            dpl_doc::Looks::builtin_only(),
            1,
        )
        .unwrap();
        let p = |i, s| nav::Position { index: i, step: s };
        assert_eq!(host.position(), p(1, 0));
        host.jump(1, 1);
        assert_eq!(host.position(), p(1, 1));
        host.jump(9, 9);
        assert_eq!(host.position(), p(2, 0), "clamped to the last card's taps");
        host.back();
        assert_eq!(host.position(), p(1, 1));
        host.next();
        assert_eq!(host.position(), p(2, 0));
        let late = DeckHost::open(
            &Showing::Episode(scratch_episode("late")),
            Path::new("/nowhere"),
            dpl_doc::Looks::builtin_only(),
            40,
        )
        .unwrap();
        assert_eq!(late.position(), p(2, 0));
    }

    #[test]
    fn ink_survives_steps_and_clears_on_slide_change() {
        let mut host = DeckHost::open(
            &Showing::Episode(scratch_episode("ink")),
            Path::new("/nowhere"),
            dpl_doc::Looks::builtin_only(),
            0,
        )
        .unwrap();
        host.press(egui::Key::A);
        assert!(host.annotate_on, "A toggles the ink layer");
        host.ink.begin(egui::Pos2::new(0.2, 0.2), None);
        host.ink.extend(egui::Pos2::new(0.4, 0.4), None);
        host.ink.finish();
        assert_eq!(host.ink.stroke_count(), 1);
        // slide 0 -> slide 1 step 0 (SLIDE change) — clears.
        host.press(egui::Key::ArrowRight);
        assert!(host.ink.is_empty(), "a new slide clears the ink");
        // Draw again, then advance WITHIN slide 1 (step change) — keeps.
        host.ink.begin(egui::Pos2::new(0.5, 0.5), None);
        host.ink.extend(egui::Pos2::new(0.6, 0.6), None);
        host.ink.finish();
        host.press(egui::Key::ArrowRight);
        assert_eq!(host.position(), nav::Position { index: 1, step: 1 });
        assert_eq!(host.ink.stroke_count(), 1, "steps keep the ink");
    }

    #[test]
    fn a_broken_slide_degrades_to_a_plate_not_a_crash() {
        let dir = scratch_episode("broken");
        std::fs::write(dir.join("b").join("slide.json"), "{ not json").unwrap();
        let host = DeckHost::open(
            &Showing::Episode(dir.clone()),
            Path::new("/nowhere"),
            dpl_doc::Looks::builtin_only(),
            0,
        )
        .unwrap();
        assert!(host.slides[1].doc.is_err());
        assert_eq!(host.steps[1], 1, "a broken slide is one honest step");
    }

    #[test]
    fn frames_render_at_every_position_without_panicking() {
        let mut host = DeckHost::open(
            &Showing::Episode(scratch_episode("frame")),
            Path::new("/nowhere"),
            dpl_doc::Looks::builtin_only(),
            0,
        )
        .unwrap();
        // The pool fonts must exist BEFORE layout — an unregistered
        // FontFamily::Name panics inside epaint (the same idiom as the
        // renderer's own tests).
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        loop {
            ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let _ = host.ui(ui);
                });
            })
            .textures_delta
            .clear();
            let before = host.position();
            host.press(egui::Key::ArrowRight);
            if host.position() == before {
                break;
            }
        }
        // And once with chrome hidden + navigator open — every chrome arm.
        host.press(egui::Key::H);
        host.press(egui::Key::G);
        ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = host.ui(ui);
            });
        })
        .textures_delta
        .clear();
    }

    /// An episode built from lessons plays like a template: its lessons are
    /// the chapters, each card at its lesson's pin, in the episode's theme.
    #[test]
    fn an_episode_built_from_lessons_plays_in_its_own_theme() {
        let template = scratch_episode("lessons-template");
        let mut theme = dpl_doc::Theme::fallback();
        theme.font = "michroma".into();
        dpl_doc::save_theme(&template, &theme).unwrap();
        let root = template.with_file_name("lessons-content");
        let _ = std::fs::remove_dir_all(&root);
        let dir = dpl_doc::clone_episode(&template, &root, "show", "ep").unwrap();
        let host = DeckHost::open(
            &Showing::Episode(dir),
            &root,
            dpl_doc::Looks::builtin_only(),
            0,
        )
        .unwrap();
        assert_eq!(host.steps, vec![1, 2, 1]);
        let titles: Vec<&str> = host.markers.iter().map(|m| m.title.as_str()).collect();
        assert_eq!(titles, ["Test show — One", "Test show — Two"]);
        assert_eq!(
            host.slides[0].name, "slide a",
            "a card is named by its words"
        );
        assert_eq!(host.styling.theme, theme, "the episode's theme, handed in");
        let _ = std::fs::remove_dir_all(&root);
    }
}
