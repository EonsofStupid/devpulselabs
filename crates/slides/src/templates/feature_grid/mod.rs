//! FeatureGrid — a 2×2 board of feature cards, each a tone glyph badge, a
//! title, and a one-line description. The "here's what's in the box" slide;
//! cards fade in on a diagonal stagger.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Ui};

use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("features"),
    label: "Feature grid",
    class: TemplateClass::Split,
    blurb: "A 2×2 board of feature cards — tone glyph, title, one-liner. Diagonal fade-in.",
};

pub struct Feature {
    /// A short font-safe glyph label (1–2 chars) — no tofu boxes.
    pub glyph: &'static str,
    pub title: &'static str,
    pub body: &'static str,
}

pub struct FeatureGrid {
    pub tone: Tone,
    pub heading: &'static str,
    pub features: Vec<Feature>,
    clock: Clock,
}

impl FeatureGrid {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Magenta,
            heading: "What the runtime gives you",
            features: vec![
                Feature {
                    glyph: "><",
                    title: "Streaming",
                    body: "Tokens arrive as they're decided",
                },
                Feature {
                    glyph: "[]",
                    title: "Tools",
                    body: "The model calls your functions",
                },
                Feature {
                    glyph: "@",
                    title: "Memory",
                    body: "Context that survives the turn",
                },
                Feature {
                    glyph: "//",
                    title: "Local",
                    body: "Runs on the box in front of you",
                },
            ],
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for FeatureGrid {
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
        if fade.in_flight(now, t0, self.features.len()) {
            ui.ctx().request_repaint();
        }

        painter.text(
            egui::pos2(stage.rect.min.x, stage.rect.min.y),
            Align2::LEFT_TOP,
            self.heading,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG,
        );

        let top = stage.rect.min.y + fluid.fs_h2() + fluid.sp_6();
        let gap = fluid.sp_4();
        let cell_w = (stage.rect.width() - gap) * 0.5;
        let cell_h = ((stage.rect.max.y - top) - gap) * 0.5;
        let radius = fluid.r_lg() as u8;
        let tone = self.tone.color();

        for (i, feat) in self.features.iter().take(4).enumerate() {
            let (cx, cy) = (i % 2, i / 2);
            // Diagonal stagger: row+col index orders the fade.
            let order = cx + cy;
            let alpha = fade.at(now, t0, order).clamp(0.0, 1.0);
            let cell = Rect::from_min_size(
                egui::pos2(
                    stage.rect.min.x + (cell_w + gap) * cx as f32,
                    top + (cell_h + gap) * cy as f32,
                ),
                egui::vec2(cell_w, cell_h),
            );
            painter.rect_filled(
                cell,
                radius,
                cl::surface::RAISED.gamma_multiply(alpha * 0.8),
            );
            painter.rect_stroke(
                cell,
                radius,
                egui::Stroke::new(1.0, cl::surface::HAIRLINE.gamma_multiply(alpha)),
                egui::StrokeKind::Inside,
            );
            cl::sheen(painter, cell);

            // The glyph badge.
            let badge_r = fluid.sp_6();
            let bc = egui::pos2(
                cell.min.x + fluid.sp_5() + badge_r,
                cell.min.y + fluid.sp_5() + badge_r,
            );
            painter.circle_filled(
                bc,
                badge_r,
                egui::Color32::from_rgba_unmultiplied(
                    tone.r(),
                    tone.g(),
                    tone.b(),
                    (200.0 * alpha) as u8,
                ),
            );
            painter.text(
                bc,
                Align2::CENTER_CENTER,
                feat.glyph,
                FontId::monospace(fluid.fs_h4()),
                cl::surface::WELL.gamma_multiply(alpha),
            );

            let tx = cell.min.x + fluid.sp_5();
            let ty = bc.y + badge_r + fluid.sp_4();
            painter.text(
                egui::pos2(tx, ty),
                Align2::LEFT_TOP,
                feat.title,
                FontId::proportional(fluid.fs_h4()),
                cl::ink::STRONG.gamma_multiply(alpha),
            );
            painter.text(
                egui::pos2(tx, ty + fluid.fs_h4() * 1.3),
                Align2::LEFT_TOP,
                feat.body,
                FontId::proportional(fluid.fs_body()),
                cl::ink::MUTED.gamma_multiply(alpha),
            );
        }

        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
