//! RadialProgress — a full-circle progress ring that sweeps to a percent,
//! a toned arc over a hairline track with a big center number. The clean
//! single-metric "we're N% there" instrument.
//!
//! Presenter law: the ring sweeps once as a pure clock function, then rests.

use dpl_coldlight::{self as cl, Tone};
use dpl_vector::arc_stroke;
use egui::{Align2, Sense, Ui};
use serde::{Deserialize, Serialize};
use std::f32::consts::{FRAC_PI_2, TAU};

use super::stage::{ChartStage, LOGICAL_H, LOGICAL_W};
use crate::motion::Clock;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

/// Fields are the contract's slots and props (`component.json`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RadialProgress {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    /// 0..1 fill.
    pub frac: f32,
    #[serde(default)]
    pub label: String,
    #[serde(skip)]
    clock: Clock,
}

impl RadialProgress {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0)
            .expect("the radial-progress demo is its example")
    }
}

impl SlideTemplate for RadialProgress {
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

        let sweep = cl::motion::CHART_SWEEP;
        if sweep.in_flight(now, t0, 1) {
            ui.ctx().request_repaint();
        }
        let p = sweep.at(now, t0, 0).clamp(0.0, 1.0);
        let frac = self.frac.clamp(0.0, 1.0) * p;

        let center = stage.pt(LOGICAL_W * 0.5, LOGICAL_H * 0.5);
        let radius = stage.len(LOGICAL_H * 0.36);
        let width = stage.len(26.0);
        let tone = self.tone.color();

        // Full track, then the value arc from 12 o'clock clockwise.
        painter.add(egui::Shape::mesh(arc_stroke(
            center,
            radius,
            width,
            0.0,
            TAU,
            cl::surface::WELL,
        )));
        if frac > 0.001 {
            let start = -FRAC_PI_2;
            painter.add(egui::Shape::mesh(arc_stroke(
                center,
                radius,
                width,
                start,
                start + TAU * frac,
                tone,
            )));
        }

        // Center readout: the live percent (counts up with the sweep).
        let pct = (frac * 100.0).round() as i32;
        stage.display_text(
            painter,
            (LOGICAL_W * 0.5, LOGICAL_H * 0.5 - 12.0),
            Align2::CENTER_CENTER,
            &format!("{pct}%"),
            72.0,
            tone,
        );
        stage.mono_label(
            painter,
            (LOGICAL_W * 0.5, LOGICAL_H * 0.5 + 44.0),
            Align2::CENTER_CENTER,
            &self.label,
            13.0,
            cl::ink::MUTED,
        );

        ui.allocate_rect(stage.rect, Sense::hover());
    }
}
