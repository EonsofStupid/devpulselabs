//! GaugeDial — a 180° dial that sweeps a needle to a value, with a toned
//! value arc over a hairline track, end ticks, and a big center readout. The
//! "how full / how fast / how hot" single-number instrument.
//!
//! Presenter law: the sweep is a pure clock function — settles once, rests.

use dpl_coldlight::{self as cl, Tone};
use dpl_vector::arc_stroke;
use egui::{Align2, Sense, Stroke, Ui, Vec2};
use std::f32::consts::PI;

use super::stage::{ChartStage, LOGICAL_H, LOGICAL_W};
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("gauge-dial"),
    label: "Gauge dial",
    class: TemplateClass::Animation,
    blurb: "A 180° dial: a needle sweeps to the value over a toned arc, big center readout.",
};

pub struct GaugeDial {
    pub tone: Tone,
    /// 0..1 of the dial's span.
    pub frac: f32,
    pub value: &'static str,
    pub label: &'static str,
    clock: Clock,
}

impl GaugeDial {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Coral,
            frac: 0.73,
            value: "73%",
            label: "GPU utilization",
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for GaugeDial {
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

        let sweep = cl::motion::GAUGE_SWEEP;
        if sweep.in_flight(now, t0, 1) {
            ui.ctx().request_repaint();
        }
        let p = sweep.at(now, t0, 0).clamp(0.0, 1.0);
        let frac = self.frac.clamp(0.0, 1.0) * p;

        // The dial: top semicircle. Center sits low so the arc has headroom.
        let center = stage.pt(LOGICAL_W * 0.5, LOGICAL_H * 0.66);
        let radius = stage.len(LOGICAL_H * 0.42);
        let width = stage.len(22.0);
        let tone = self.tone.color();

        // Track (full span) then the toned value arc. θ: PI (left) → 2*PI
        // (right) sweeps across the TOP in egui's y-down space.
        painter.add(egui::Shape::mesh(arc_stroke(
            center,
            radius,
            width,
            PI,
            2.0 * PI,
            cl::surface::WELL,
        )));
        if frac > 0.001 {
            painter.add(egui::Shape::mesh(arc_stroke(
                center,
                radius,
                width,
                PI,
                PI + PI * frac,
                tone,
            )));
        }

        // End ticks at the two ends of the span.
        for end in [PI, 2.0 * PI] {
            let dir = Vec2::angled(end);
            let a = center + dir * (radius - width);
            let b = center + dir * (radius + width * 0.4);
            painter.line_segment([a, b], Stroke::new(1.5, cl::surface::HAIRLINE));
        }

        // The needle.
        let na = PI + PI * frac;
        let tip = center + Vec2::angled(na) * (radius - width * 0.5);
        painter.line_segment([center, tip], Stroke::new(stage.len(4.0), tone));
        painter.circle_filled(center, stage.len(9.0), cl::ink::STRONG);
        painter.circle_stroke(center, stage.len(9.0), Stroke::new(2.0, tone));

        // Center readout.
        stage.display_text(
            painter,
            (LOGICAL_W * 0.5, LOGICAL_H * 0.72),
            Align2::CENTER_CENTER,
            self.value,
            64.0,
            tone,
        );
        stage.mono_label(
            painter,
            (LOGICAL_W * 0.5, LOGICAL_H * 0.86),
            Align2::CENTER_CENTER,
            self.label,
            14.0,
            cl::ink::MUTED,
        );

        ui.allocate_rect(stage.rect, Sense::hover());
    }
}
