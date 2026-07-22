//! StackLayers — "the stack, laid bare": frontier API vs local box.
//!
//! Two labeled columns of layer cards. Upgrades: cards are painted plates —
//! raised fill, crisp hairline, a faint top sheen — and the bottom layer
//! (the one you own) gets the tone wash + tone edge treatment the old
//! glass-strong approximated. Layers fade in bottom-up, staggered.

use dpl_coldlight::{self as cl, Fluid, Tone};
use egui::{Align2, FontId, Rect, Ui};

use crate::blocks::eyebrow::tracked;
use crate::motion::{Beat, Clock};
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("stack-layers"),
    label: "Stack layers",
    class: TemplateClass::Static,
    blurb: "Two columns of layer plates (their stack vs yours); the owned layer wears the tone.",
};

pub struct StackColumn {
    pub title: &'static str,
    pub sub: &'static str,
    pub tone: Option<Tone>,
    /// Top → bottom, ported convention.
    pub layers: Vec<&'static str>,
}

pub struct StackLayers {
    pub tone: Tone,
    pub left: StackColumn,
    pub right: StackColumn,
    clock: Clock,
}

impl StackLayers {
    /// The authored CH02 concept beat, carried.
    pub fn demo() -> Self {
        Self {
            tone: Tone::Amber,
            left: StackColumn {
                title: "Frontier API",
                sub: "their stack",
                tone: None,
                layers: vec![
                    "Your app",
                    "SDK / API calls",
                    "Their cloud",
                    "Their weights",
                    "Their GPUs",
                ],
            },
            right: StackColumn {
                title: "Local box",
                sub: "your stack",
                tone: Some(Tone::Lime),
                layers: vec![
                    "Your app",
                    "Local runtime",
                    "Your machine",
                    "Open weights",
                    "Your GPU",
                ],
            },
            clock: Clock::default(),
        }
    }
}

fn column(
    painter: &egui::Painter,
    fluid: &Fluid,
    rect: Rect,
    col: &StackColumn,
    base: Tone,
    beat: Beat,
) -> bool {
    let tone = col.tone.unwrap_or(base);
    let mut animating = false;

    // Header: title + sub eyebrow on a shared baseline.
    let title_size = fluid.fs_h3();
    painter.text(
        rect.min,
        Align2::LEFT_TOP,
        col.title,
        FontId::proportional(title_size),
        cl::ink::STRONG,
    );
    painter.text(
        egui::pos2(rect.max.x, rect.min.y + title_size * 0.75),
        Align2::RIGHT_BOTTOM,
        tracked(col.sub),
        FontId::monospace(fluid.fs_eyebrow()),
        cl::ink::MUTED,
    );

    // Layer plates fill the remaining height, gap sp_2.
    let top = rect.min.y + title_size + fluid.sp_3();
    let gap = fluid.sp_2();
    let n = col.layers.len() as f32;
    let plate_h = ((rect.max.y - top) - gap * (n - 1.0)) / n;
    let radius = fluid.r_md() as u8;
    for (i, layer) in col.layers.iter().enumerate() {
        // Bottom-up: the layer you own lands first.
        let fade = cl::motion::LAYER_FADE.at(beat.now, beat.t0, col.layers.len() - 1 - i);
        if fade < 1.0 {
            animating = true;
        }
        let y = top + (plate_h + gap) * i as f32;
        let plate =
            Rect::from_min_size(egui::pos2(rect.min.x, y), egui::vec2(rect.width(), plate_h));
        let owned = i == col.layers.len() - 1;
        let alpha = |a: u8| (a as f32 * fade) as u8;
        if owned {
            let c = tone.color();
            painter.rect_filled(
                plate,
                radius,
                egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha(30)),
            );
            painter.rect_stroke(
                plate,
                radius,
                egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha(140)),
                ),
                egui::StrokeKind::Inside,
            );
        } else {
            painter.rect_filled(
                plate,
                radius,
                egui::Color32::from_rgba_unmultiplied(0x11, 0x14, 0x1d, alpha(255)),
            );
            painter.rect_stroke(
                plate,
                radius,
                egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(0x23, 0x28, 0x36, alpha(255)),
                ),
                egui::StrokeKind::Inside,
            );
            // The faint top sheen every plate wears.
            let band = Rect::from_min_size(
                plate.min + egui::vec2(1.0, 1.0),
                egui::vec2(plate.width() - 2.0, (plate.height() * 0.4).min(10.0)),
            );
            painter.rect_filled(
                band,
                radius,
                egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, alpha(5)),
            );
        }
        painter.text(
            plate.center(),
            Align2::CENTER_CENTER,
            *layer,
            FontId::proportional(fluid.fs_body()),
            egui::Color32::from_rgba_unmultiplied(0xee, 0xf2, 0xf9, alpha(255)),
        );
    }
    animating
}

impl SlideTemplate for StackLayers {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
    }

    fn restart(&mut self) {
        self.clock.restart();
    }

    fn ui(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let fluid = Fluid::of(full);
        let now = ui.input(|i| i.time);
        let beat = self.clock.beat(now);
        let painter = ui.painter();

        let area = full.shrink(fluid.sp_6());
        let gutter = fluid.sp_6();
        let col_w = (area.width() - gutter) * 0.5;
        let left = Rect::from_min_size(area.min, egui::vec2(col_w, area.height()));
        let right = Rect::from_min_size(
            egui::pos2(area.min.x + col_w + gutter, area.min.y),
            egui::vec2(col_w, area.height()),
        );
        let a = column(painter, &fluid, left, &self.left, self.tone, beat);
        let b = column(
            painter,
            &fluid,
            right,
            &self.right,
            self.tone,
            beat.delayed(0.2),
        );
        if a || b {
            ui.ctx().request_repaint();
        }
        ui.allocate_rect(full, egui::Sense::hover());
    }
}
