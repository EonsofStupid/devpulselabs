//! BarCompare — labeled value bars (the receipts chart, EOS-001 CH02).
//!
//! Upgrades: bars are solid pool color with a per-vertex gradient sheen on
//! the upper band (crisp, no blur), rising in with the ported 120ms stagger
//! as a pure function of the clock; the display value lands above each bar
//! in its paint color.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, Rect, Ui};
use serde::{Deserialize, Serialize};

use super::stage::ChartStage;
use super::tone_cycle::series_tone;
use crate::motion::Clock;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

/// One bar — an item of the `bars` list slot.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarDatum {
    pub label: String,
    pub value: f32,
    pub display: String,
    /// `None` = the next tone of the series cycle.
    #[serde(default, with = "crate::contract::tone_opt")]
    pub tone: Option<Tone>,
}

/// Fields are the contract's slots and props (`component.json`). The demo
/// is the authored EOS-001 CH02 receipt, carried.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarCompare {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    #[serde(default)]
    pub unit: String,
    pub bars: Vec<BarDatum>,
    #[serde(skip)]
    clock: Clock,
}

impl BarCompare {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0)
            .expect("the bar-compare demo is its example")
    }
}

impl SlideTemplate for BarCompare {
    fn spec(&self) -> &'static ComponentContract {
        Self::contract()
    }

    fn restart(&mut self) {
        self.clock.restart();
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = ChartStage::fit(ui.max_rect());
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        let painter = ui.painter();
        let (x0, y0, x1, y1) = stage.plot();
        let w = x1 - x0;
        let h = y1 - y0;

        stage.mono_label(
            painter,
            ((x0 + x1) * 0.5, y0 - 4.0),
            Align2::CENTER_BOTTOM,
            &self.unit,
            17.0,
            cl::ink::MUTED,
        );

        // Band scale, padding 0.35 (ported from scaleBand).
        let n = self.bars.len() as f32;
        let step = w / n;
        let bw = step * (1.0 - 0.35);
        let max_v = self.bars.iter().map(|b| b.value).fold(0.0f32, f32::max) * 1.15;

        // Baseline hairline.
        painter.line_segment(
            [stage.pt(x0, y1), stage.pt(x1, y1)],
            egui::Stroke::new(stage.len(2.0), stage.hairline()),
        );

        let rise_fx = cl::motion::CHART_RISE;
        for (i, bar) in self.bars.iter().enumerate() {
            let tone = bar.tone.unwrap_or_else(|| series_tone(i, self.tone));
            let paint = tone.color();
            let rise = rise_fx.at(now, t0, i);
            let full_h = (bar.value / max_v.max(f32::EPSILON)) * h;
            let bar_h = full_h * rise;
            let bx = x0 + step * i as f32 + (step - bw) * 0.5;
            let rect = Rect::from_min_max(stage.pt(bx, y1 - bar_h), stage.pt(bx + bw, y1));
            let radius = stage.len(8.0) as u8;
            painter.rect_filled(rect, radius, paint);
            // The sheen: a lighter band over the top third — sprite depth.
            let ramp = cl::ramp(tone.hue_id()).expect("tone in pool");
            let lighter = ramp.steps[cl::STEP_500 - 2];
            let band = Rect::from_min_max(
                rect.min + egui::vec2(stage.len(1.0), stage.len(1.0)),
                egui::pos2(
                    rect.max.x - stage.len(1.0),
                    rect.min.y + rect.height() * 0.33,
                ),
            );
            if band.height() > 0.0 {
                let lit = egui::Color32::from_rgba_unmultiplied(
                    lighter.r(),
                    lighter.g(),
                    lighter.b(),
                    90,
                );
                // Top-lit quad fading to transparent.
                let mesh = dpl_vector::area_fill(
                    &[band.left_top(), band.right_top()],
                    band.max.y,
                    lit,
                    egui::Color32::TRANSPARENT,
                );
                painter.add(egui::Shape::mesh(mesh));
            }
            // Value above the bar, in the bar's paint.
            stage.display_text(
                painter,
                (bx + bw * 0.5, y1 - full_h * rise - 14.0),
                Align2::CENTER_BOTTOM,
                &bar.display,
                30.0,
                paint,
            );
            // Label under the axis, mono uppercase.
            stage.mono_label(
                painter,
                (bx + bw * 0.5, y1 + 20.0),
                Align2::CENTER_TOP,
                &bar.label,
                17.0,
                cl::ink::MUTED,
            );
        }
        if rise_fx.in_flight(now, t0, self.bars.len()) {
            ui.ctx().request_repaint();
        }
        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
