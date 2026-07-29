//! Checklist — tap to tick items off, one at a time, with a progress meter
//! that fills as you go. Presenter law: the WHOLE stage is the tap target, so
//! tapping checks the next open item (never hunt for a box); each check pops
//! on `reward-pulse` and holds. The bottom `bar` block shows how far you are.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Sense, Ui};

use crate::elements::bar::bar;
use crate::elements::eyebrow::tracked;
use crate::frame::Stage;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("checklist"),
    label: "Checklist",
    class: TemplateClass::Interactive,
    blurb: "Tap to tick items off one at a time; each pops, and a meter fills as you go.",
};

pub struct Checklist {
    pub tone: Tone,
    pub heading: &'static str,
    pub items: Vec<&'static str>,
    /// When each item was checked — the per-item pop clock.
    checked_at: Vec<Option<f64>>,
}

impl Checklist {
    pub fn demo() -> Self {
        let items = vec![
            "Pick a model that fits your box",
            "Wire the streaming loop",
            "Give it your tools",
            "Ship it behind a real task",
        ];
        let n = items.len();
        Self {
            tone: Tone::Lime,
            heading: "Ship your first agent",
            items,
            checked_at: vec![None; n],
        }
    }

    fn done(&self) -> usize {
        self.checked_at.iter().filter(|t| t.is_some()).count()
    }

    /// Check the next open item at `now`; clamps at the end (pure — tested).
    fn advance(&mut self, now: f64) {
        if let Some(slot) = self.checked_at.iter_mut().find(|t| t.is_none()) {
            *slot = Some(now);
        }
    }
}

impl SlideTemplate for Checklist {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
    }

    fn restart(&mut self) {
        for t in self.checked_at.iter_mut() {
            *t = None;
        }
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let fluid = &stage.fluid;
        let now = ui.input(|i| i.time);
        let painter = ui.painter().clone();
        let pulse = cl::motion::REWARD_PULSE;
        let tone = self.tone.color();

        painter.text(
            egui::pos2(stage.rect.min.x, stage.rect.min.y),
            Align2::LEFT_TOP,
            self.heading,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG,
        );

        let n = self.items.len() as f32;
        let top = stage.rect.min.y + fluid.fs_h2() + fluid.sp_6();
        let foot = fluid.sp_4() + fluid.fs_eyebrow() + fluid.sp_5();
        let gap = fluid.sp_3();
        let row_h = ((stage.rect.max.y - foot - top) - gap * (n - 1.0)) / n;
        let radius = fluid.r_md() as u8;
        let mut animating = false;

        for (i, item) in self.items.iter().enumerate() {
            let y = top + (row_h + gap) * i as f32;
            let row = Rect::from_min_size(
                egui::pos2(stage.rect.min.x, y),
                egui::vec2(stage.rect.width(), row_h),
            );
            let checked = self.checked_at[i];
            let filled = checked.is_some();
            painter.rect_filled(
                row,
                radius,
                cl::surface::RAISED.gamma_multiply(if filled { 0.9 } else { 0.5 }),
            );

            // The box: empty hairline square, or a tone-filled tick.
            let box_side = row_h * 0.5;
            let bx = Rect::from_center_size(
                egui::pos2(row.min.x + fluid.sp_5() + box_side * 0.5, row.center().y),
                egui::vec2(box_side, box_side),
            );
            let sm = fluid.r_sm() as u8;
            if let Some(t0) = checked {
                let p = pulse.at(now, t0, 0);
                if p < 1.0 {
                    animating = true;
                }
                let pop = (0.6 + 0.4 * p).min(1.0);
                let filledbox = Rect::from_center_size(bx.center(), bx.size() * pop);
                painter.rect_filled(filledbox, sm, tone);
                // A font-safe check mark.
                painter.text(
                    filledbox.center(),
                    Align2::CENTER_CENTER,
                    "x",
                    FontId::monospace(box_side * 0.8),
                    cl::surface::WELL,
                );
            } else {
                painter.rect_stroke(
                    bx,
                    sm,
                    egui::Stroke::new(1.5, cl::surface::HAIRLINE),
                    egui::StrokeKind::Inside,
                );
            }

            let tx = bx.max.x + fluid.sp_5();
            let ink = if filled {
                cl::ink::MUTED
            } else {
                cl::ink::STRONG
            };
            painter.text(
                egui::pos2(tx, row.center().y),
                Align2::LEFT_CENTER,
                *item,
                FontId::proportional(fluid.fs_h4()),
                ink,
            );
        }
        if animating {
            ui.ctx().request_repaint();
        }

        // The progress meter + count.
        let done = self.done();
        let meter = Rect::from_min_size(
            egui::pos2(
                stage.rect.min.x,
                stage.rect.max.y - fluid.fs_eyebrow() - fluid.sp_3(),
            ),
            egui::vec2(stage.rect.width() * 0.6, fluid.sp_2()),
        );
        bar(
            &painter,
            meter,
            done as f32 / self.items.len() as f32,
            self.tone,
            1.0,
        );
        painter.text(
            egui::pos2(stage.rect.max.x, meter.center().y),
            Align2::RIGHT_CENTER,
            tracked(&format!("{done} / {}", self.items.len())),
            FontId::monospace(fluid.fs_eyebrow()),
            cl::ink::MUTED,
        );

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
        let mut c = Checklist::demo();
        assert_eq!(c.done(), 0);
        for _ in 0..10 {
            c.advance(1.0);
        }
        assert_eq!(c.done(), 4, "advance clamps at the item count");
        c.restart();
        assert_eq!(c.done(), 0);
    }
}
