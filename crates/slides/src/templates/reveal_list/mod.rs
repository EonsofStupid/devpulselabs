//! RevealList — tap-to-reveal beat list. One tap, one line, holds forever.
//!
//! Presenter-paced build-up (carried): the WHOLE stage is the tap target so
//! the presenter never hunts for a button; every revealed line pops in on
//! `reveal-pop` (tap feedback — no settle delay); unrevealed items keep
//! ghost plates so the layout is a stable rest state at every count.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Sense, Ui};

use crate::blocks::eyebrow::tracked;
use crate::frame::Stage;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("reveal"),
    label: "Reveal list",
    class: TemplateClass::Interactive,
    blurb: "Tap anywhere to reveal the next beat; each line pops in and holds forever.",
};

pub struct RevealItem {
    pub text: &'static str,
    pub sub: &'static str,
}

pub struct RevealList {
    pub tone: Tone,
    pub heading: &'static str,
    pub hint: &'static str,
    pub items: Vec<RevealItem>,
    /// When each item was revealed — the per-item pop clock.
    revealed_at: Vec<Option<f64>>,
}

impl RevealList {
    pub fn demo() -> Self {
        let items = vec![
            RevealItem {
                text: "It predicts the next word",
                sub: "that's the whole trick",
            },
            RevealItem {
                text: "Scale made it feel like thinking",
                sub: "same trick, more of it",
            },
            RevealItem {
                text: "Your box can run it now",
                sub: "open weights changed the deal",
            },
        ];
        let n = items.len();
        Self {
            tone: Tone::Cyan,
            heading: "Three things that are actually true",
            hint: "Tap to reveal",
            items,
            revealed_at: vec![None; n],
        }
    }

    fn shown(&self) -> usize {
        self.revealed_at.iter().filter(|t| t.is_some()).count()
    }

    /// Advance one reveal at `now`; clamps at the end (pure — tested).
    fn advance(&mut self, now: f64) {
        if let Some(slot) = self.revealed_at.iter_mut().find(|t| t.is_none()) {
            *slot = Some(now);
        }
    }
}

impl SlideTemplate for RevealList {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
    }

    fn restart(&mut self) {
        for t in self.revealed_at.iter_mut() {
            *t = None;
        }
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let fluid = &stage.fluid;
        let now = ui.input(|i| i.time);
        let painter = ui.painter().clone();
        let pop = cl::motion::REVEAL_POP;

        let heading_size = fluid.fs_h2();
        painter.text(
            egui::pos2(stage.rect.center().x, stage.rect.min.y),
            Align2::CENTER_TOP,
            self.heading,
            FontId::proportional(heading_size),
            cl::ink::STRONG,
        );

        let n = self.items.len() as f32;
        let gap = fluid.sp_3();
        let top = stage.rect.min.y + heading_size + fluid.sp_6();
        let foot = fluid.fs_eyebrow() + fluid.sp_5();
        let row_h = ((stage.rect.max.y - foot - top) - gap * (n - 1.0)) / n;
        let radius = fluid.r_md() as u8;
        let mut animating = false;

        for (i, item) in self.items.iter().enumerate() {
            let y = top + (row_h + gap) * i as f32;
            let plate = Rect::from_min_size(
                egui::pos2(stage.rect.min.x, y),
                egui::vec2(stage.rect.width(), row_h),
            );
            match self.revealed_at[i] {
                Some(t0) => {
                    let p = pop.at(now, t0, 0);
                    if p < 1.0 {
                        animating = true;
                    }
                    // The pop: plate scales in around its center on the
                    // spring (overshoot reads as the "lock" landing).
                    let scaled = Rect::from_center_size(
                        plate.center(),
                        plate.size() * (0.9 + 0.1 * p).max(0.05),
                    );
                    painter.rect_filled(scaled, radius, cl::surface::RAISED);
                    painter.rect_stroke(
                        scaled,
                        radius,
                        egui::Stroke::new(1.0, cl::surface::HAIRLINE),
                        egui::StrokeKind::Inside,
                    );
                    let alpha = p.clamp(0.0, 1.0);
                    painter.text(
                        egui::pos2(scaled.min.x + fluid.sp_5(), scaled.center().y),
                        Align2::LEFT_CENTER,
                        format!("{:02}", i + 1),
                        FontId::monospace(fluid.fs_h3()),
                        self.tone.color().gamma_multiply(alpha),
                    );
                    let tx = scaled.min.x + fluid.sp_5() + fluid.sp_8();
                    painter.text(
                        egui::pos2(tx, scaled.center().y - fluid.fs_body() * 0.75),
                        Align2::LEFT_TOP,
                        item.text,
                        FontId::proportional(fluid.fs_h4()),
                        cl::ink::STRONG.gamma_multiply(alpha),
                    );
                    painter.text(
                        egui::pos2(tx, scaled.center().y + fluid.fs_body() * 0.55),
                        Align2::LEFT_TOP,
                        item.sub,
                        FontId::proportional(fluid.fs_body_sm()),
                        cl::ink::MUTED.gamma_multiply(alpha),
                    );
                }
                None => {
                    // The ghost: the box holds its place, says nothing.
                    painter.rect_filled(
                        plate,
                        radius,
                        egui::Color32::from_rgba_unmultiplied(0x11, 0x14, 0x1d, 70),
                    );
                }
            }
        }
        if animating {
            ui.ctx().request_repaint();
        }

        let shown = self.shown();
        let counter = if shown >= self.items.len() {
            format!("{} / {}", shown, self.items.len())
        } else {
            format!("{} · {} / {}", self.hint, shown, self.items.len())
        };
        painter.text(
            egui::pos2(stage.rect.center().x, stage.rect.max.y),
            Align2::CENTER_BOTTOM,
            tracked(&counter),
            FontId::monospace(fluid.fs_eyebrow()),
            cl::ink::MUTED,
        );

        // The whole stage advances — presenter law: taps never hunt.
        if ui.allocate_rect(stage.rect, Sense::click()).clicked() {
            self.advance(now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_clamps_and_restart_clears() {
        let mut r = RevealList::demo();
        assert_eq!(r.shown(), 0);
        for _ in 0..10 {
            r.advance(1.0);
        }
        assert_eq!(r.shown(), 3, "advance clamps at the item count");
        r.restart();
        assert_eq!(r.shown(), 0);
    }
}
