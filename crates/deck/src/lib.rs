//! The DeckHost (D2) — the show machine's runtime surface.
//!
//! An episode folder becomes a full-bleed presentation: beat-wise
//! navigation (the verbatim [`nav`] grammar), edge tap zones, a progress
//! rail with chapter ticks, a bottom toolbar, a navigator overlay, and the
//! chrome-hide escape hatches — the bottom edge-reveal strip and the
//! always-there home dot ("ALWAYS a way back on a touchscreen").
//!
//! Rendering rides the ONE document path (`dpl_slides::document::
//! render_step`); a slide that fails to load degrades to an honest labeled
//! plate, never a crash. Ink/annotate is D3; presenter+sync is D4.

pub mod ink;
pub mod nav;

use dpl_coldlight::{self as cl};
use dpl_slides::document;
use egui::{Align2, Color32, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Ui};
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::path::Path;

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

/// One slide in the flat walk: its chapter, folder name, and the loaded
/// document — or the honest load error.
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
    theme: dpl_doc::Theme,
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
}

impl DeckHost {
    /// Open a show from its episode folder.
    pub fn open(dir: &Path) -> Result<Self, String> {
        let ep = dpl_doc::load_episode(dir).map_err(|e| e.to_string())?;
        let theme = dpl_doc::load_theme(dir)
            .ok()
            .flatten()
            .unwrap_or_else(dpl_doc::Theme::fallback);
        let mut slides = Vec::new();
        let mut markers: Vec<Marker> = Vec::new();
        let mut last_chapter: Option<String> = None;
        for r in dpl_doc::episode_slides(dir, &ep) {
            if last_chapter.as_deref() != Some(r.chapter.as_str()) {
                let title = ep
                    .chapters
                    .iter()
                    .find(|c| c.id == r.chapter)
                    .map(|c| c.title.clone())
                    .unwrap_or_else(|| r.chapter.clone());
                markers.push(Marker {
                    index: r.index,
                    title,
                });
                last_chapter = Some(r.chapter.clone());
            }
            slides.push(DeckSlide {
                chapter: r.chapter,
                name: r.name,
                doc: dpl_doc::load(&r.dir).map_err(|e| e.to_string()),
            });
        }
        if slides.is_empty() {
            return Err("the episode has no slides".into());
        }
        let steps = slides
            .iter()
            .map(|s| s.doc.as_ref().map(|d| d.step_count()).unwrap_or(1))
            .collect();
        Ok(Self {
            title: ep.title,
            theme,
            slides,
            steps,
            markers,
            pos: nav::Position { index: 0, step: 0 },
            chrome_hidden: false,
            navigator_open: false,
            annotate_on: false,
            ink: ink::InkLayer::default(),
            clock: dpl_slides::motion::Clock::default(),
            fabric: None,
            presenter_asked: false,
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

    fn beat_next(&mut self) {
        self.go(nav::advance(&self.steps, self.pos));
    }
    fn beat_prev(&mut self) {
        self.go(nav::retreat(&self.steps, self.pos));
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
                    &self.theme,
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

        // Edge tap zones (12% width, full height) — always tappable; the
        // chevrons only PAINT while chrome shows.
        let zone_w = rect.width() * 0.12;
        let left = Rect::from_min_size(rect.min, egui::vec2(zone_w, rect.height()));
        let right = Rect::from_min_size(
            egui::pos2(rect.max.x - zone_w, rect.min.y),
            egui::vec2(zone_w, rect.height()),
        );
        let lr = ui.interact(left, ui.id().with("deck_prev"), Sense::click());
        let rr = ui.interact(right, ui.id().with("deck_next"), Sense::click());
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
        if lr.clicked() && !self.annotate_on {
            self.beat_prev();
        }
        if rr.clicked() && !self.annotate_on {
            self.beat_next();
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
            let doc = dpl_doc::Slide {
                v: dpl_doc::SCHEMA_V,
                template: "title".into(),
                background: None,
                blocks: vec![
                    dpl_doc::Block::Text {
                        id: "title".into(),
                        text: format!("slide {name}"),
                        font: None,
                        size: "h1".into(),
                        ink: None,
                    },
                    dpl_doc::Block::Text {
                        id: "sub".into(),
                        text: "beat".into(),
                        font: None,
                        size: "h3".into(),
                        ink: None,
                    },
                ],
                enter: None,
                reveals,
            };
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
        let host = DeckHost::open(&scratch_episode("open")).unwrap();
        assert_eq!(host.steps, vec![1, 2, 1]);
        assert_eq!(host.slides.len(), 3);
        let marks: Vec<usize> = host.markers.iter().map(|m| m.index).collect();
        assert_eq!(marks, vec![0, 2], "chapter two starts at flat index 2");
    }

    #[test]
    fn the_keyboard_walks_the_whole_beat_list_and_holds() {
        let mut host = DeckHost::open(&scratch_episode("keys")).unwrap();
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
        let mut host = DeckHost::open(&scratch_episode("sync")).unwrap();
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

    #[test]
    fn ink_survives_steps_and_clears_on_slide_change() {
        let mut host = DeckHost::open(&scratch_episode("ink")).unwrap();
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
        let host = DeckHost::open(&dir).unwrap();
        assert!(host.slides[1].doc.is_err());
        assert_eq!(host.steps[1], 1, "a broken slide is one honest step");
    }

    #[test]
    fn frames_render_at_every_position_without_panicking() {
        let mut host = DeckHost::open(&scratch_episode("frame")).unwrap();
        // The pool fonts must exist BEFORE layout — an unregistered
        // FontFamily::Name panics inside epaint (the same idiom as the
        // renderer's own tests).
        let ctx = egui::Context::default();
        cl::apply_theme(&ctx);
        loop {
            let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let _ = host.ui(ui);
                });
            });
            let before = host.position();
            host.press(egui::Key::ArrowRight);
            if host.position() == before {
                break;
            }
        }
        // And once with chrome hidden + navigator open — every chrome arm.
        host.press(egui::Key::H);
        host.press(egui::Key::G);
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = host.ui(ui);
            });
        });
    }
}
