//! Agenda — a numbered running order: a heading and N rows, each a tone
//! number chip, a title, and a one-line sub. Everything is shown (static —
//! this is the "here's where we're going" board, not a reveal). Rows fade in
//! staggered on the settle, then rest.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Ui};
use serde::{Deserialize, Serialize};

use crate::elements::eyebrow::tracked;
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

/// One row of the running order (the `items` slot's item fields).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgendaItem {
    pub title: String,
    #[serde(default)]
    pub sub: String,
}

/// Fields are the contract's slots, props and axes (`component.json`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Agenda {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    pub heading: String,
    pub items: Vec<AgendaItem>,
    #[serde(skip)]
    clock: Clock,
}

impl Agenda {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0).expect("the agenda demo is its example")
    }
}

impl SlideTemplate for Agenda {
    fn spec(&self) -> &'static ComponentContract {
        Self::contract()
    }

    fn restart(&mut self) {
        self.clock.restart();
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let fluid = &stage.fluid;
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        let painter = ui.painter();

        let fade = cl::motion::LAYER_FADE;
        if fade.in_flight(now, t0, self.items.len()) {
            ui.ctx().request_repaint();
        }

        painter.text(
            egui::pos2(stage.rect.min.x, stage.rect.min.y),
            Align2::LEFT_TOP,
            &self.heading,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG,
        );

        let n = self.items.len() as f32;
        let top = stage.rect.min.y + fluid.fs_h2() + fluid.sp_6();
        let gap = fluid.sp_3();
        let row_h = ((stage.rect.max.y - top) - gap * (n - 1.0)) / n;
        let radius = fluid.r_md() as u8;
        let tone = self.tone.color();

        for (i, item) in self.items.iter().enumerate() {
            let alpha = fade.at(now, t0, i).clamp(0.0, 1.0);
            let y = top + (row_h + gap) * i as f32;
            let row = Rect::from_min_size(
                egui::pos2(stage.rect.min.x, y),
                egui::vec2(stage.rect.width(), row_h),
            );
            painter.rect_filled(row, radius, cl::surface::RAISED.gamma_multiply(alpha * 0.7));

            // The tone number chip on the left.
            let chip_w = row_h;
            let chip = Rect::from_min_size(row.min, egui::vec2(chip_w, row_h));
            painter.rect_filled(
                chip,
                radius,
                egui::Color32::from_rgba_unmultiplied(
                    tone.r(),
                    tone.g(),
                    tone.b(),
                    (200.0 * alpha) as u8,
                ),
            );
            painter.text(
                chip.center(),
                Align2::CENTER_CENTER,
                format!("{:02}", i + 1),
                FontId::monospace(fluid.fs_h3()),
                cl::surface::WELL.gamma_multiply(alpha),
            );

            let tx = row.min.x + chip_w + fluid.sp_5();
            painter.text(
                egui::pos2(tx, row.center().y - fluid.fs_body() * 0.75),
                Align2::LEFT_TOP,
                &item.title,
                FontId::proportional(fluid.fs_h4()),
                cl::ink::STRONG.gamma_multiply(alpha),
            );
            painter.text(
                egui::pos2(tx, row.center().y + fluid.fs_body() * 0.5),
                Align2::LEFT_TOP,
                tracked(&item.sub),
                FontId::monospace(fluid.fs_eyebrow()),
                cl::ink::MUTED.gamma_multiply(alpha),
            );
        }

        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
