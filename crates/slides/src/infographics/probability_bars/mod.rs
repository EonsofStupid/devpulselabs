//! ProbabilityBars — ranked next-word predictions ("Call the next word").
//!
//! Layout chart (no 960×540 stage — rows are typography-first, like the
//! original's fluid HTML). Upgrades: bars grow in with a stagger; the top
//! prediction wears a tone halo painted as layered translucent rings (crisp
//! falloff, no blur); the joke tail keeps its magenta.

use dpl_coldlight::{self as cl, Fluid, Tone};
use egui::{Align2, FontId, Rect, Ui};

use crate::blocks::eyebrow::tracked;
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("probability-bars"),
    label: "Probability bars",
    class: TemplateClass::Static,
    blurb: "Ranked next-word predictions: wash tracks, halo on the top pick, magenta joke tail.",
};

pub struct Prediction {
    pub token: &'static str,
    /// 0..100
    pub pct: f32,
    pub tone: Option<Tone>,
}

pub struct ProbabilityBars {
    pub tone: Tone,
    pub prompt: &'static str,
    pub predictions: Vec<Prediction>,
    clock: Clock,
}

impl ProbabilityBars {
    /// The authored beat-A demo, carried (moon and tax return included).
    pub fn demo() -> Self {
        Self {
            tone: Tone::Amber,
            prompt: "The cat sat on the ___",
            predictions: vec![
                Prediction { token: "mat", pct: 62.0, tone: None },
                Prediction { token: "floor", pct: 21.0, tone: None },
                Prediction { token: "couch", pct: 9.0, tone: None },
                Prediction { token: "moon", pct: 0.4, tone: Some(Tone::Magenta) },
                Prediction { token: "tax return", pct: 0.1, tone: Some(Tone::Magenta) },
            ],
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for ProbabilityBars {
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
        let t0 = self.clock.tick(now);
        let painter = ui.painter();

        let pad = fluid.sp_6();
        let area = full.shrink(pad);
        let row_h = fluid.sp_5();
        let gap = fluid.sp_3();
        let prompt_size = fluid.fs_h2();
        let base = self.tone.color();

        // The prompt, with the blank in the slide's tone.
        let mut cursor = area.min;
        let parts: Vec<&str> = self.prompt.splitn(2, "___").collect();
        let font = FontId::monospace(prompt_size);
        let lead = painter.layout_no_wrap(parts[0].to_string(), font.clone(), cl::ink::STRONG);
        painter.galley(cursor, lead.clone(), cl::ink::STRONG);
        painter.text(
            cursor + egui::vec2(lead.size().x, 0.0),
            Align2::LEFT_TOP,
            "___",
            font,
            base,
        );
        cursor.y += prompt_size + fluid.sp_5();

        // Rows: token · track+fill · pct.
        let top = self.predictions.iter().map(|p| p.pct).fold(1.0f32, f32::max);
        let token_w = fluid.sp_8() * 2.2;
        let pct_w = fluid.sp_8();
        let grow_fx = cl::motion::BAR_GROW;
        for (i, p) in self.predictions.iter().enumerate() {
            let tone = p.tone.unwrap_or(self.tone);
            let paint = tone.color();
            let y = cursor.y;
            if y + row_h > area.max.y {
                break; // small stage: rows that don't fit are simply not drawn
            }
            let ink = if i == 0 { cl::ink::STRONG } else { cl::ink::MUTED };
            painter.text(
                egui::pos2(area.min.x + token_w, y + row_h * 0.5),
                Align2::RIGHT_CENTER,
                p.token,
                FontId::monospace(fluid.fs_body()),
                ink,
            );

            let track = Rect::from_min_max(
                egui::pos2(area.min.x + token_w + gap, y + row_h * 0.25),
                egui::pos2(area.max.x - pct_w - gap, y + row_h * 0.75),
            );
            let radius = track.height() * 0.5;
            // The wash track.
            painter.rect_filled(
                track,
                radius,
                egui::Color32::from_rgba_unmultiplied(paint.r(), paint.g(), paint.b(), 26),
            );
            // The fill, grown in with the ported stagger.
            let grow = grow_fx.at(now, t0, i);
            let frac = ((p.pct / top).max(0.015)) * grow;
            let fill = Rect::from_min_size(
                track.min,
                egui::vec2(track.width() * frac, track.height()),
            );
            if i == 0 {
                // The halo: two crisp translucent rings around the top fill.
                for (grow_px, alpha) in [(3.0, 40), (6.0, 18)] {
                    painter.rect_filled(
                        fill.expand(grow_px),
                        radius + grow_px,
                        egui::Color32::from_rgba_unmultiplied(paint.r(), paint.g(), paint.b(), alpha),
                    );
                }
            }
            painter.rect_filled(fill, radius, paint);

            let pct_text = if p.pct < 1.0 {
                format!("{}%", p.pct)
            } else {
                format!("{}%", p.pct.round() as i32)
            };
            painter.text(
                egui::pos2(area.max.x, y + row_h * 0.5),
                Align2::RIGHT_CENTER,
                tracked(&pct_text),
                FontId::monospace(fluid.fs_eyebrow()),
                cl::ink::MUTED,
            );
            cursor.y += row_h + gap;
        }
        if grow_fx.in_flight(now, t0, self.predictions.len()) {
            ui.ctx().request_repaint();
        }
        ui.allocate_rect(full, egui::Sense::hover());
    }
}
