//! ChoicePoll — predict-then-reveal A/B. The audience calls it, THEN the truth.
//!
//! Retrieval practice, carried: commit before you see. Tap either plate →
//! the truth fills its tone with a one-time reward-pulse ring bloom, the
//! loser dims, and the LAND line fades in on the entrance beat. Rest state
//! = revealed, held forever.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Sense, Ui};

use crate::blocks::eyebrow::tracked;
use crate::frame::Stage;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("choice"),
    label: "Choice poll",
    class: TemplateClass::Interactive,
    blurb: "Predict-then-reveal A/B: the room commits, the truth lands with a reward pulse.",
};

pub struct ChoiceOption {
    pub label: &'static str,
    pub detail: &'static str,
}

pub struct ChoicePoll {
    pub tone: Tone,
    pub prompt: &'static str,
    pub a: ChoiceOption,
    pub b: ChoiceOption,
    /// Which side is the truth.
    pub answer: usize,
    pub reveal: &'static str,
    called: Option<(usize, f64)>,
}

impl ChoicePoll {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Amber,
            prompt: "Which costs more to answer one question?",
            a: ChoiceOption { label: "Frontier API", detail: "someone else's GPUs" },
            b: ChoiceOption { label: "Your box", detail: "electricity only" },
            answer: 0,
            reveal: "The API wins on peak quality — your box wins on cost, privacy, and it's yours.",
            called: None,
        }
    }
}

impl SlideTemplate for ChoicePoll {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
    }

    fn restart(&mut self) {
        self.called = None;
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let fluid = &stage.fluid;
        let now = ui.input(|i| i.time);
        let painter = ui.painter().clone();

        painter.text(
            egui::pos2(stage.rect.center().x, stage.rect.min.y),
            Align2::CENTER_TOP,
            self.prompt,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG,
        );

        let top = stage.rect.min.y + fluid.fs_h2() + fluid.sp_6();
        let land_h = fluid.fs_h4() * 2.2 + fluid.sp_5();
        let gutter = fluid.sp_5();
        let plate_w = (stage.rect.width() - gutter) * 0.5;
        let plate_h = stage.rect.max.y - land_h - top;
        let radius = fluid.r_lg() as u8;
        let revealed = self.called.is_some();

        let mut clicked: Option<usize> = None;
        for (side, opt) in [(0usize, &self.a), (1usize, &self.b)] {
            let plate = Rect::from_min_size(
                egui::pos2(stage.rect.min.x + (plate_w + gutter) * side as f32, top),
                egui::vec2(plate_w, plate_h),
            );
            let is_truth = side == self.answer;
            let resp = ui.allocate_rect(plate, Sense::click());
            if resp.clicked() && !revealed {
                clicked = Some(side);
            }

            let (fill, key_ink, body_ink) = match (revealed, is_truth) {
                (true, true) => (self.tone.color(), cl::surface::WELL, cl::surface::BASE),
                (true, false) => (
                    egui::Color32::from_rgba_unmultiplied(0x11, 0x14, 0x1d, 100),
                    cl::ink::SUBTLE,
                    cl::ink::SUBTLE,
                ),
                _ => (
                    if resp.hovered() { cl::surface::OVERLAY } else { cl::surface::RAISED },
                    cl::ink::STRONG,
                    cl::ink::MUTED,
                ),
            };
            painter.rect_filled(plate, radius, fill);
            if !revealed {
                painter.rect_stroke(
                    plate,
                    radius,
                    egui::Stroke::new(1.0, cl::surface::HAIRLINE),
                    egui::StrokeKind::Inside,
                );
            }

            // The one-time reward pulse blooming off the truth plate.
            if let Some((called_side, t0)) = self.called {
                if is_truth {
                    let p = cl::motion::REWARD_PULSE.at(now, t0, 0);
                    if p < 1.0 {
                        ui.ctx().request_repaint();
                        let grow = 24.0 * p;
                        let c = self.tone.color();
                        painter.rect_stroke(
                            plate.expand(grow),
                            radius,
                            egui::Stroke::new(
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(
                                    c.r(), c.g(), c.b(),
                                    (140.0 * (1.0 - p)) as u8,
                                ),
                            ),
                            egui::StrokeKind::Outside,
                        );
                    }
                    painter.rect_stroke(
                        plate,
                        radius,
                        egui::Stroke::new(2.0, cl::ink::STRONG.gamma_multiply(0.6)),
                        egui::StrokeKind::Inside,
                    );
                }
                // "the room's call" marker on the tapped side.
                if called_side == side {
                    painter.text(
                        egui::pos2(plate.center().x, plate.min.y + fluid.sp_3()),
                        Align2::CENTER_TOP,
                        tracked("the room's call"),
                        FontId::monospace(fluid.fs_eyebrow()),
                        if is_truth { cl::surface::BASE } else { cl::ink::SUBTLE },
                    );
                }
            }

            painter.text(
                egui::pos2(plate.center().x, plate.center().y - fluid.fs_h3() * 0.4),
                Align2::CENTER_CENTER,
                opt.label,
                FontId::proportional(fluid.fs_h3()),
                key_ink,
            );
            painter.text(
                egui::pos2(plate.center().x, plate.center().y + fluid.fs_h3() * 0.8),
                Align2::CENTER_CENTER,
                opt.detail,
                FontId::proportional(fluid.fs_body()),
                body_ink,
            );
        }
        if let Some(side) = clicked {
            self.called = Some((side, now));
        }

        // The LAND line — deliberately on the entrance beat (settle, then in).
        let foot = egui::pos2(stage.rect.center().x, stage.rect.max.y - fluid.sp_2());
        match self.called {
            Some((_, t0)) => {
                let p = cl::motion::ENTER_FADE.at(now, t0, 0);
                if p < 1.0 {
                    ui.ctx().request_repaint();
                }
                painter.text(
                    foot,
                    Align2::CENTER_BOTTOM,
                    self.reveal,
                    FontId::proportional(fluid.fs_h4()),
                    cl::ink::BASE.gamma_multiply(p),
                );
            }
            None => {
                painter.text(
                    foot,
                    Align2::CENTER_BOTTOM,
                    tracked("Call it — then tap the room's answer"),
                    FontId::monospace(fluid.fs_eyebrow()),
                    cl::ink::MUTED,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_once_then_rest_and_restart_clears() {
        let mut c = ChoicePoll::demo();
        assert!(c.called.is_none());
        c.called = Some((1, 2.0));
        c.restart();
        assert!(c.called.is_none());
    }
}
