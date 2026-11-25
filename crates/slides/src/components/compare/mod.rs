//! CompareVs — a head-to-head A/B: two panels, each a name, a toned strength
//! meter, and a short list of points, with a "VS" chip in the gutter. The
//! side-by-side call the split class exists for — pick this over `split_panel`
//! when both sides are contenders (not then→now).

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Ui};
use serde::{Deserialize, Serialize};

use crate::elements::bar::bar;
use crate::elements::eyebrow::tracked;
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

/// One side of the head-to-head.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contender {
    pub name: String,
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    /// 0..1 strength meter.
    pub score: f32,
    #[serde(default)]
    pub points: Vec<String>,
}

/// Fields are the contract's slots, props and axes (`component.json`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompareVs {
    pub heading: String,
    pub left: Contender,
    pub right: Contender,
    #[serde(skip)]
    clock: Clock,
}

impl CompareVs {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0).expect("the compare demo is its example")
    }
}

impl SlideTemplate for CompareVs {
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
        let grow = cl::motion::BAR_GROW;
        if fade.in_flight(now, t0, 2) || grow.in_flight(now, t0, 2) {
            ui.ctx().request_repaint();
        }

        painter.text(
            egui::pos2(stage.rect.center().x, stage.rect.min.y),
            Align2::CENTER_TOP,
            &self.heading,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG,
        );

        let top = stage.rect.min.y + fluid.fs_h2() + fluid.sp_6();
        let gutter = fluid.sp_8();
        let col_w = (stage.rect.width() - gutter) * 0.5;
        let col_h = stage.rect.max.y - top;
        let radius = fluid.r_lg() as u8;

        for (i, side) in [&self.left, &self.right].into_iter().enumerate() {
            let alpha = fade.at(now, t0, i).clamp(0.0, 1.0);
            let col = Rect::from_min_size(
                egui::pos2(stage.rect.min.x + (col_w + gutter) * i as f32, top),
                egui::vec2(col_w, col_h),
            );
            painter.rect_filled(col, radius, cl::surface::RAISED.gamma_multiply(alpha * 0.8));
            let c = side.tone.color();
            painter.rect_stroke(
                col,
                radius,
                egui::Stroke::new(
                    1.5,
                    egui::Color32::from_rgba_unmultiplied(
                        c.r(),
                        c.g(),
                        c.b(),
                        (170.0 * alpha) as u8,
                    ),
                ),
                egui::StrokeKind::Inside,
            );
            cl::sheen(painter, col);

            let x = col.min.x + fluid.sp_5();
            painter.text(
                egui::pos2(x, col.min.y + fluid.sp_5()),
                Align2::LEFT_TOP,
                &side.name,
                FontId::proportional(fluid.fs_h3()),
                cl::ink::STRONG.gamma_multiply(alpha),
            );

            // The strength meter grows in on BAR_GROW.
            let g = grow.at(now, t0, i).clamp(0.0, 1.0);
            let meter = Rect::from_min_size(
                egui::pos2(x, col.min.y + fluid.sp_5() + fluid.fs_h3() + fluid.sp_4()),
                egui::vec2(col_w - fluid.sp_5() * 2.0, fluid.sp_3()),
            );
            bar(painter, meter, side.score * g, side.tone, alpha);

            let mut y = meter.max.y + fluid.sp_5();
            for point in side.points.iter() {
                painter.text(
                    egui::pos2(x, y),
                    Align2::LEFT_TOP,
                    format!("— {point}"),
                    FontId::proportional(fluid.fs_body_lg()),
                    cl::ink::BASE.gamma_multiply(alpha),
                );
                y += fluid.fs_body_lg() * 1.7;
            }
        }

        // The VS chip, centered in the gutter.
        let vs = egui::pos2(stage.rect.center().x, top + col_h * 0.5);
        let r = fluid.sp_6();
        painter.circle_filled(vs, r, cl::surface::WELL);
        painter.circle_stroke(vs, r, egui::Stroke::new(1.5, cl::surface::HAIRLINE));
        painter.text(
            vs,
            Align2::CENTER_CENTER,
            tracked("vs"),
            FontId::monospace(fluid.fs_body()),
            cl::ink::MUTED,
        );

        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
