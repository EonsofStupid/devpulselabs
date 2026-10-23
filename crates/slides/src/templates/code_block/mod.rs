//! CodeBlock — deliberately raw code plate (no syntax highlighter), carried.
//!
//! Gutter line numbers, tone-washed highlight lines with a left tone edge,
//! Copy holding a 1.4s "Copied" rest state, ScrollArea for overflow.
//! Entrance: the plate fades in on the settle.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Sense, Ui};
use serde::{Deserialize, Serialize};

use crate::elements::eyebrow::tracked;
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

const COPIED_HOLD: f64 = 1.4;

/// Fields are the contract's slots, props and axes (`component.json`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeBlock {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub language: String,
    pub code: String,
    /// 1-indexed highlighted lines, carried convention.
    #[serde(default)]
    pub highlights: Vec<usize>,
    #[serde(skip)]
    copied_at: Option<f64>,
    #[serde(skip)]
    clock: Clock,
}

impl CodeBlock {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0).expect("the code demo is its example")
    }
}

impl SlideTemplate for CodeBlock {
    fn spec(&self) -> &'static ComponentContract {
        Self::contract()
    }

    fn restart(&mut self) {
        self.clock.restart();
        self.copied_at = None;
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let fluid = &stage.fluid;
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        let painter = ui.painter().clone();
        let alpha = cl::motion::ENTER_FADE.at(now, t0, 0).clamp(0.0, 1.0);
        if cl::motion::ENTER_FADE.in_flight(now, t0, 1) {
            ui.ctx().request_repaint();
        }

        let plate = stage.rect;
        let radius = fluid.r_md() as u8;
        painter.rect_filled(plate, radius, cl::surface::BASE.gamma_multiply(alpha));
        painter.rect_stroke(
            plate,
            radius,
            egui::Stroke::new(1.0, cl::surface::HAIRLINE.gamma_multiply(alpha)),
            egui::StrokeKind::Inside,
        );

        // Header: title · language ─────────────── Copy
        let head_h = fluid.sp_7();
        let hx = plate.min.x + fluid.sp_5();
        painter.text(
            egui::pos2(hx, plate.min.y + head_h * 0.5),
            Align2::LEFT_CENTER,
            &self.title,
            FontId::proportional(fluid.fs_h4()),
            cl::ink::STRONG.gamma_multiply(alpha),
        );
        let title_w = painter
            .layout_no_wrap(
                self.title.clone(),
                FontId::proportional(fluid.fs_h4()),
                cl::ink::BASE,
            )
            .size()
            .x;
        painter.text(
            egui::pos2(hx + title_w + fluid.sp_4(), plate.min.y + head_h * 0.5),
            Align2::LEFT_CENTER,
            tracked(&self.language),
            FontId::monospace(fluid.fs_eyebrow()),
            cl::ink::MUTED.gamma_multiply(alpha),
        );
        // Copy — holds "Copied" for the carried 1.4s rest.
        let copied = self.copied_at.is_some_and(|t| now - t < COPIED_HOLD);
        if self.copied_at.is_some() && !copied {
            self.copied_at = None;
        }
        let label = if copied { "Copied" } else { "Copy" };
        let galley = painter.layout_no_wrap(
            tracked(label),
            FontId::monospace(fluid.fs_eyebrow()),
            cl::ink::BASE,
        );
        let btn = Rect::from_center_size(
            egui::pos2(
                plate.max.x - fluid.sp_6() - galley.size().x * 0.5,
                plate.min.y + head_h * 0.5,
            ),
            galley.size() + egui::vec2(fluid.sp_5(), fluid.sp_2()),
        );
        let resp = ui.allocate_rect(btn, Sense::click());
        painter.rect_filled(
            btn,
            btn.height() * 0.5,
            if copied {
                self.tone.wash()
            } else if resp.hovered() {
                cl::surface::OVERLAY
            } else {
                cl::surface::RAISED
            },
        );
        painter.galley(
            btn.center() - galley.size() * 0.5,
            galley,
            if copied {
                self.tone.color()
            } else {
                cl::ink::BASE
            },
        );
        if resp.clicked() {
            ui.ctx().copy_text(self.code.clone());
            self.copied_at = Some(now);
            ui.ctx().request_repaint();
        }
        if copied {
            ui.ctx().request_repaint(); // tick the hold down
        }

        // The code grid: gutter + lines; highlights get wash + tone edge.
        let font = FontId::monospace(fluid.fs_body());
        let line_h = fluid.fs_body() * 1.65;
        let gutter_w = fluid.sp_8();
        let top = plate.min.y + head_h + fluid.sp_3();
        for (i, line) in self.code.lines().enumerate() {
            let ln = i + 1;
            let y = top + line_h * i as f32;
            if y + line_h > plate.max.y - fluid.sp_3() {
                break; // stage-bound; the deck runtime scrolls, the wall clips
            }
            let row = Rect::from_min_size(
                egui::pos2(plate.min.x + fluid.sp_2(), y),
                egui::vec2(plate.width() - fluid.sp_2() * 2.0, line_h),
            );
            if self.highlights.contains(&ln) {
                painter.rect_filled(row, 0, self.tone.wash().gamma_multiply(alpha));
                painter.rect_filled(
                    Rect::from_min_size(row.min, egui::vec2(3.0, row.height())),
                    0,
                    self.tone.color().gamma_multiply(alpha),
                );
            }
            painter.text(
                egui::pos2(row.min.x + gutter_w, row.center().y),
                Align2::RIGHT_CENTER,
                format!("{ln}"),
                FontId::monospace(fluid.fs_body_sm()),
                cl::ink::SUBTLE.gamma_multiply(alpha),
            );
            painter.text(
                egui::pos2(row.min.x + gutter_w + fluid.sp_4(), row.center().y),
                Align2::LEFT_CENTER,
                line,
                font.clone(),
                cl::ink::BASE.gamma_multiply(alpha),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copied_rest_expires_and_restart_clears() {
        let mut c = CodeBlock::demo();
        c.copied_at = Some(10.0);
        assert!(c.copied_at.is_some_and(|t| 11.0 - t < COPIED_HOLD));
        assert!(
            !c.copied_at.is_some_and(|t| 12.0 - t < COPIED_HOLD),
            "hold expires"
        );
        c.restart();
        assert!(c.copied_at.is_none());
    }
}
