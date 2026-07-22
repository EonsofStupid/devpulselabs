//! CostMeterFace — the STATIC meter visual (arc + needle + readout).
//!
//! Deliberately non-interactive, carried: the interactive `cost-meter`
//! template is the mechanic; this face is the picture. Upgrades: the arc is
//! a crisp mesh stroke, the fill SWEEPS to its level once (then rests), and
//! the needle tracks the sweep — a real gauge, not a dasharray trick.

use dpl_coldlight::{self as cl, Tone};
use dpl_vector::arc_stroke;
use egui::{Align2, Ui};
use std::f32::consts::PI;

use super::stage::{ChartStage, LOGICAL_W};
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("cost-meter-face"),
    label: "Cost meter face",
    class: TemplateClass::Static,
    blurb: "Static gauge: track arc, tone fill sweeping to its level, needle, wall-sized readout.",
};

pub struct CostMeterFace {
    pub tone: Tone,
    /// 0..1 needle position along the arc.
    pub level: f32,
    pub readout: &'static str,
    pub label: &'static str,
    clock: Clock,
}

impl CostMeterFace {
    /// The authored beat-C face, carried.
    pub fn demo() -> Self {
        Self {
            tone: Tone::Amber,
            level: 0.72,
            readout: "$4.20",
            label: "session cost — and climbing",
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for CostMeterFace {
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

        // Semicircle gauge: center low on the stage, opening upward.
        let center = stage.pt(LOGICAL_W * 0.5, 380.0);
        let r = stage.len(280.0);
        let width = stage.len(30.0);
        let a_start = PI; // left horizon
        let a_end = 2.0 * PI; // right horizon (clockwise over the top)

        // Track.
        painter.add(egui::Shape::mesh(arc_stroke(
            center,
            r,
            width,
            a_start,
            a_end,
            cl::surface::RAISED,
        )));
        // Fill sweeps to the level once, then rests (presenter law).
        // gauge-sweep carries its authored 0.2s delay in the token.
        let sweep = cl::motion::GAUGE_SWEEP.at(now, t0, 0);
        if cl::motion::GAUGE_SWEEP.in_flight(now, t0, 1) {
            ui.ctx().request_repaint();
        }
        let level = self.level.clamp(0.0, 1.0) * sweep;
        let paint = self.tone.color();
        painter.add(egui::Shape::mesh(arc_stroke(
            center,
            r,
            width,
            a_start,
            a_start + PI * level,
            paint,
        )));

        // Needle tracks the fill edge.
        let angle = a_start + PI * level;
        let dir = egui::Vec2::angled(angle);
        painter.line_segment(
            [center + dir * (r * 0.35), center + dir * (r - width)],
            egui::Stroke::new(stage.len(6.0), cl::ink::STRONG),
        );
        painter.circle_filled(center, stage.len(10.0), cl::ink::STRONG);

        // Readout + label under the gauge.
        stage.display_text(
            painter,
            (LOGICAL_W * 0.5, 452.0),
            Align2::CENTER_TOP,
            self.readout,
            56.0,
            paint,
        );
        stage.mono_label(
            painter,
            (LOGICAL_W * 0.5, 516.0),
            Align2::CENTER_TOP,
            self.label,
            17.0,
            cl::ink::MUTED,
        );
        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
