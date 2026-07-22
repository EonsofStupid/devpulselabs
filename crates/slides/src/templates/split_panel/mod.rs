//! SplitPanel — the then→now A/B panel. Class: split.
//!
//! Convention carried verbatim: LEFT = then / hype, RIGHT = now / truth.
//! Moving right = moving into truth. Columns fade in left-then-right on the
//! settle; the accent side wears the tone ring.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Ui};

use crate::blocks::eyebrow::tracked;
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("split"),
    label: "Split panel",
    class: TemplateClass::Split,
    blurb: "Then→now A/B: left = the hype, right = the truth (accent ring). Staggered fade-in.",
};

pub struct SplitSide {
    pub label: &'static str,
    pub sub: &'static str,
    pub accent: bool,
    pub body: Vec<&'static str>,
}

pub struct SplitPanel {
    pub tone: Tone,
    pub heading: &'static str,
    pub left: SplitSide,
    pub right: SplitSide,
    clock: Clock,
}

impl SplitPanel {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Lime,
            heading: "\"AI will replace developers\"",
            left: SplitSide {
                label: "2023 said",
                sub: "the hype",
                accent: false,
                body: vec![
                    "Prompt in, product out",
                    "No more code review",
                    "One person = a whole team",
                ],
            },
            right: SplitSide {
                label: "2026 says",
                sub: "the truth",
                accent: true,
                body: vec![
                    "Developers who drive agents ship 10×",
                    "Review matters MORE, not less",
                    "The bottleneck moved to judgment",
                ],
            },
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for SplitPanel {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
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
        if fade.in_flight(now, t0, 2) {
            ui.ctx().request_repaint();
        }

        painter.text(
            egui::pos2(stage.rect.center().x, stage.rect.min.y),
            Align2::CENTER_TOP,
            self.heading,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG,
        );

        let top = stage.rect.min.y + fluid.fs_h2() + fluid.sp_6();
        let gutter = fluid.sp_5();
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
            let edge = if side.accent {
                let c = self.tone.color();
                egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (170.0 * alpha) as u8)
            } else {
                cl::surface::HAIRLINE.gamma_multiply(alpha)
            };
            painter.rect_stroke(col, radius, egui::Stroke::new(1.0, edge), egui::StrokeKind::Inside);
            cl::sheen(painter, col);

            let x = col.min.x + fluid.sp_5();
            painter.text(
                egui::pos2(x, col.min.y + fluid.sp_4()),
                Align2::LEFT_TOP,
                side.label,
                FontId::proportional(fluid.fs_h3()),
                cl::ink::STRONG.gamma_multiply(alpha),
            );
            painter.text(
                egui::pos2(col.max.x - fluid.sp_5(), col.min.y + fluid.sp_4() + fluid.fs_h3() * 0.5),
                Align2::RIGHT_CENTER,
                tracked(side.sub),
                FontId::monospace(fluid.fs_eyebrow()),
                if side.accent { self.tone.color() } else { cl::ink::MUTED }.gamma_multiply(alpha),
            );
            let mut y = col.min.y + fluid.sp_4() + fluid.fs_h3() + fluid.sp_5();
            for line in side.body.iter() {
                painter.text(
                    egui::pos2(x, y),
                    Align2::LEFT_TOP,
                    *line,
                    FontId::proportional(fluid.fs_body_lg()),
                    cl::ink::BASE.gamma_multiply(alpha),
                );
                y += fluid.fs_body_lg() * 1.7;
            }
        }
        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
