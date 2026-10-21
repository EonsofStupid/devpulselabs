//! Checklist — tap to tick items off, one at a time, with a progress meter
//! that fills as you go. Presenter law: the WHOLE stage is the tap target, so
//! tapping checks the next open item (never hunt for a box); each check pops
//! on `reward-pulse` and holds. The bottom `bar` block shows how far you are.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Sense, Ui};
use serde::{Deserialize, Serialize};

use crate::elements::bar::bar;
use crate::elements::eyebrow::tracked;
use crate::frame::Stage;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

/// Fields are the contract's slots, props and axes (`component.json`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checklist {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    pub heading: String,
    pub items: Vec<String>,
    /// When each item was checked — the per-item pop clock. Sized from
    /// `items` by `restart()` (which `instantiate` runs on every fresh
    /// instance), so it always has one entry per item.
    #[serde(skip)]
    checked_at: Vec<Option<f64>>,
}

impl Checklist {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0)
            .expect("the checklist demo is its example")
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
    fn spec(&self) -> &'static ComponentContract {
        Self::contract()
    }

    fn restart(&mut self) {
        self.checked_at = vec![None; self.items.len()];
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
            &self.heading,
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
                item,
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

    #[test]
    fn restart_sizes_the_checks_from_the_items() {
        let c = Checklist::demo();
        assert_eq!(c.checked_at.len(), c.items.len());
    }
}
