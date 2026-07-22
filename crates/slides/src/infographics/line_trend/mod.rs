//! LineTrend — one or two series over time ("where it's heading").
//!
//! Upgrades: the line is a crisp mesh stroke along a true monotone curve
//! (no overshoot — a data floor never dips), DRAWN IN point-by-point as a
//! function of the clock; the area wash underneath is a per-vertex gradient
//! fading to the baseline.

use dpl_coldlight::{self as cl, Tone};
use dpl_vector::{area_fill, polyline_stroke, sample_monotone};
use egui::{Align2, Pos2, Ui};

use super::stage::ChartStage;
use super::tone_cycle::series_tone;
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("line-trend"),
    label: "Line trend",
    class: TemplateClass::Static,
    blurb: "1–2 series over time: monotone curve draw-in, gradient area wash, end-of-line labels.",
};

pub struct Series {
    pub name: &'static str,
    pub values: Vec<f32>,
    pub tone: Option<Tone>,
}

pub struct LineTrend {
    pub tone: Tone,
    pub unit: &'static str,
    pub labels: Vec<&'static str>,
    pub series: Vec<Series>,
    clock: Clock,
}

impl LineTrend {
    /// The authored "capability per dollar" demo, carried.
    pub fn demo() -> Self {
        Self {
            tone: Tone::Amber,
            unit: "capability per dollar",
            labels: vec!["2022", "2023", "2024", "2025", "2026"],
            series: vec![
                Series { name: "frontier per $", values: vec![1.0, 3.0, 9.0, 24.0, 60.0], tone: None },
                Series { name: "local per $", values: vec![0.2, 1.0, 6.0, 22.0, 70.0], tone: Some(Tone::Lime) },
            ],
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for LineTrend {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
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
            self.unit,
            17.0,
            cl::ink::MUTED,
        );
        painter.line_segment(
            [stage.pt(x0, y1), stage.pt(x1, y1)],
            egui::Stroke::new(stage.len(2.0), stage.hairline()),
        );

        // Point scale on x, linear on y (ported).
        let nx = self.labels.len().max(2) as f32 - 1.0;
        let max_v = self
            .series
            .iter()
            .flat_map(|s| s.values.iter().copied())
            .fold(0.0f32, f32::max)
            * 1.1;

        for (j, label) in self.labels.iter().enumerate() {
            stage.mono_label(
                painter,
                (x0 + w * (j as f32 / nx), y1 + 20.0),
                Align2::CENTER_TOP,
                label,
                17.0,
                cl::ink::MUTED,
            );
        }

        let draw_fx = cl::motion::CHART_DRAW;
        for (i, s) in self.series.iter().enumerate() {
            let tone = s.tone.unwrap_or_else(|| series_tone(i, self.tone));
            let paint = tone.color();
            let anchors: Vec<Pos2> = s
                .values
                .iter()
                .enumerate()
                .map(|(j, v)| {
                    stage.pt(
                        x0 + w * (j as f32 / nx),
                        y1 - (v / max_v.max(f32::EPSILON)) * h,
                    )
                })
                .collect();
            let curve = sample_monotone(&anchors, 64);

            // Draw-in: the visible prefix of the curve grows with the clock.
            let progress = draw_fx.at(now, t0, i);
            let visible = ((curve.len() as f32 * progress).ceil() as usize).clamp(2, curve.len());
            let shown = &curve[..visible];

            // Area wash fades in with the line, gradient to the baseline.
            let wash = egui::Color32::from_rgba_unmultiplied(
                paint.r(),
                paint.g(),
                paint.b(),
                (26.0 * progress) as u8,
            );
            painter.add(egui::Shape::mesh(area_fill(
                shown,
                stage.pt(0.0, y1).y,
                wash,
                egui::Color32::TRANSPARENT,
            )));
            painter.add(egui::Shape::mesh(polyline_stroke(shown, stage.len(6.0), paint)));

            // End-of-line series label in the series paint.
            if let Some(end) = shown.last() {
                painter.text(
                    *end - egui::vec2(0.0, stage.len(14.0)),
                    Align2::RIGHT_BOTTOM,
                    s.name,
                    egui::FontId::proportional(stage.len(22.0)),
                    paint,
                );
            }
        }
        if draw_fx.in_flight(now, t0, self.series.len()) {
            ui.ctx().request_repaint();
        }
        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
