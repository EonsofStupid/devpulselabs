//! CostMeter — the-meter-is-running. Presenter taps SEND once per exchange;
//! the running total climbs (superlinearity lives in the DATA). After the
//! last exchange the room commits on the call-it line, then one more tap
//! lands the flip — same ask, tiny cost — and the LAND line holds.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Sense, Ui};

use crate::blocks::eyebrow::tracked;
use crate::frame::Stage;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("cost-meter"),
    label: "Cost meter",
    class: TemplateClass::Interactive,
    blurb:
        "SEND fills the meter exchange by exchange; then the flip lands the same ask for pennies.",
};

pub struct Exchange {
    pub label: &'static str,
    pub cost: f32,
}

pub struct CostMeter {
    pub tone: Tone,
    pub prompt: &'static str,
    pub unit: &'static str,
    pub exchanges: Vec<Exchange>,
    pub call_it: &'static str,
    pub flip: Exchange,
    pub reveal: &'static str,
    sent: usize,
    /// (fraction before the last send, when it was sent) — the fill tween.
    tween: (f32, f64),
    flipped: Option<f64>,
}

impl CostMeter {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Amber,
            prompt: "Same chat, priced per exchange",
            unit: "tokens",
            exchanges: vec![
                Exchange {
                    label: "\"Write my bio\"",
                    cost: 900.0,
                },
                Exchange {
                    label: "\"Make it funnier\"",
                    cost: 2400.0,
                },
                Exchange {
                    label: "\"Now the whole thread as context\"",
                    cost: 9800.0,
                },
            ],
            call_it: "Call it: what does the SAME ask cost on your own box?",
            flip: Exchange {
                label: "Your box, same ask",
                cost: 12.0,
            },
            reveal: "The meter only ever runs one way in the cloud — locally it rounds to zero.",
            sent: 0,
            tween: (0.0, f64::NEG_INFINITY),
            flipped: None,
        }
    }

    fn total(&self) -> f32 {
        self.exchanges.iter().take(self.sent).map(|e| e.cost).sum()
    }

    fn max_total(&self) -> f32 {
        self.exchanges
            .iter()
            .map(|e| e.cost)
            .sum::<f32>()
            .max(f32::EPSILON)
    }

    /// One SEND: clamps at the end, arms the fill tween (pure — tested).
    fn send(&mut self, now: f64) {
        if self.sent < self.exchanges.len() {
            self.tween = (self.total() / self.max_total(), now);
            self.sent += 1;
        }
    }
}

impl SlideTemplate for CostMeter {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
    }

    fn restart(&mut self) {
        self.sent = 0;
        self.tween = (0.0, f64::NEG_INFINITY);
        self.flipped = None;
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let fluid = &stage.fluid;
        let now = ui.input(|i| i.time);
        let painter = ui.painter().clone();
        let paint = self.tone.color();

        painter.text(
            egui::pos2(stage.rect.center().x, stage.rect.min.y),
            Align2::CENTER_TOP,
            self.prompt,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG,
        );

        // The meter: running total + tweening fill bar.
        let top = stage.rect.min.y + fluid.fs_h2() + fluid.sp_6();
        painter.text(
            egui::pos2(stage.rect.min.x, top + fluid.fs_h1() * 0.5),
            Align2::LEFT_CENTER,
            tracked("running total"),
            FontId::monospace(fluid.fs_eyebrow()),
            cl::ink::MUTED,
        );
        painter.text(
            egui::pos2(stage.rect.max.x, top + fluid.fs_h1() * 0.5),
            Align2::RIGHT_CENTER,
            format!("{} {}", self.total() as i64, self.unit),
            FontId::proportional(fluid.fs_h1()),
            paint,
        );
        let bar = Rect::from_min_size(
            egui::pos2(stage.rect.min.x, top + fluid.fs_h1() + fluid.sp_3()),
            egui::vec2(stage.rect.width(), fluid.sp_4()),
        );
        let radius = bar.height() * 0.5;
        painter.rect_filled(bar, radius, cl::surface::RAISED);
        let (from, t_send) = self.tween;
        let target = self.total() / self.max_total();
        let tp = cl::motion::FLIP_FADE.at(now, t_send, 0);
        if tp < 1.0 && t_send.is_finite() {
            ui.ctx().request_repaint();
        }
        let frac = from + (target - from) * tp;
        if frac > 0.001 {
            painter.rect_filled(
                Rect::from_min_size(bar.min, egui::vec2(bar.width() * frac, bar.height())),
                radius,
                paint,
            );
        }

        // The exchange log — fills as the presenter sends; the flip row lands last.
        let mut y = bar.max.y + fluid.sp_4();
        let row_h = fluid.fs_body() * 1.8;
        for e in self.exchanges.iter().take(self.sent) {
            painter.text(
                egui::pos2(stage.rect.min.x, y),
                Align2::LEFT_TOP,
                e.label,
                FontId::proportional(fluid.fs_body()),
                cl::ink::BASE,
            );
            painter.text(
                egui::pos2(stage.rect.max.x, y),
                Align2::RIGHT_TOP,
                format!("+{} {}", e.cost as i64, self.unit),
                FontId::monospace(fluid.fs_body()),
                cl::ink::MUTED,
            );
            y += row_h;
        }
        if self.flipped.is_some() {
            let row = Rect::from_min_size(
                egui::pos2(stage.rect.min.x - fluid.sp_2(), y - fluid.sp_1()),
                egui::vec2(stage.rect.width() + fluid.sp_2() * 2.0, row_h),
            );
            painter.rect_filled(row, fluid.r_sm() as u8, self.tone.wash());
            painter.rect_stroke(
                row,
                fluid.r_sm() as u8,
                egui::Stroke::new(1.0, paint),
                egui::StrokeKind::Inside,
            );
            painter.text(
                egui::pos2(stage.rect.min.x, y),
                Align2::LEFT_TOP,
                self.flip.label,
                FontId::proportional(fluid.fs_body()),
                cl::ink::STRONG,
            );
            painter.text(
                egui::pos2(stage.rect.max.x, y),
                Align2::RIGHT_TOP,
                format!("+{} {}", self.flip.cost as i64, self.unit),
                FontId::monospace(fluid.fs_body()),
                paint,
            );
        }

        // The control + the land line, bottom-anchored.
        let all_sent = self.sent >= self.exchanges.len();
        let foot = egui::pos2(stage.rect.center().x, stage.rect.max.y);
        let button = |ui: &mut Ui, painter: &egui::Painter, label: String, filled: bool| -> bool {
            let galley = painter.layout_no_wrap(
                tracked(&label),
                FontId::monospace(fluid.fs_eyebrow()),
                cl::surface::WELL,
            );
            let btn = Rect::from_center_size(
                egui::pos2(foot.x, foot.y - galley.size().y),
                galley.size() + egui::vec2(fluid.sp_6() * 2.0, fluid.sp_4()),
            );
            let resp = ui.allocate_rect(btn, Sense::click());
            let (fill, ink) = if filled {
                (
                    if resp.hovered() {
                        paint
                    } else {
                        paint.gamma_multiply(0.85)
                    },
                    cl::surface::WELL,
                )
            } else {
                (
                    if resp.hovered() {
                        cl::surface::OVERLAY
                    } else {
                        cl::surface::RAISED
                    },
                    cl::ink::BASE,
                )
            };
            painter.rect_filled(btn, btn.height() * 0.5, fill);
            painter.galley(btn.center() - galley.size() * 0.5, galley, ink);
            resp.clicked()
        };

        if !all_sent {
            if button(
                ui,
                &painter,
                format!("Send ({}/{})", self.sent + 1, self.exchanges.len()),
                true,
            ) {
                self.send(now);
            }
        } else if self.flipped.is_none() {
            painter.text(
                egui::pos2(foot.x, foot.y - fluid.sp_8() - fluid.sp_4()),
                Align2::CENTER_BOTTOM,
                self.call_it,
                FontId::proportional(fluid.fs_h4()),
                cl::ink::BASE,
            );
            if button(ui, &painter, "The flip →".to_string(), false) {
                self.flipped = Some(now);
            }
        } else if let Some(t0) = self.flipped {
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sends_clamp_totals_accumulate_and_restart_zeroes() {
        let mut m = CostMeter::demo();
        for _ in 0..10 {
            m.send(1.0);
        }
        assert_eq!(m.sent, 3, "send clamps at the exchange count");
        assert_eq!(m.total() as i64, 13100);
        m.flipped = Some(2.0);
        m.restart();
        assert_eq!((m.sent, m.total() as i64), (0, 0));
        assert!(m.flipped.is_none());
    }
}
