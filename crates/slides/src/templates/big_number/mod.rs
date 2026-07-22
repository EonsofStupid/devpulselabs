//! BigNumber — the receipt: ONE verified number, wall-sized.
//!
//! Doctrine carried: every chapter lands exactly one of these; the footnote
//! is where the source/verification note lives — an unverified number never
//! gets this treatment. Value rides the slide's tone; density changes gaps
//! and the value scale only.

use dpl_coldlight::{self as cl};
use dpl_coldlight::Tone;
use egui::{Align2, FontId, Ui};

use crate::blocks::eyebrow::tracked;
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};
use crate::Density;

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("stat"),
    label: "Big number",
    class: TemplateClass::Static,
    blurb: "The receipt: one verified wall-sized number, label, and the source footnote.",
};

pub struct BigNumber {
    pub tone: Tone,
    pub density: Density,
    pub eyebrow: &'static str,
    pub value: &'static str,
    pub label: &'static str,
    pub footnote: &'static str,
    clock: Clock,
}

impl BigNumber {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Lime,
            density: Density::Comfy,
            eyebrow: "the receipt",
            value: "70×",
            label: "local capability per dollar since 2022",
            footnote: "source: EOS-001 CH02 — verified against public inference pricing",
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for BigNumber {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
    }

    fn restart(&mut self) {
        self.clock.restart();
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        // Entrance: the receipt BLOOMS (spring overshoot on scale), the
        // supporting lines fade — settle first, one-time event, then rest.
        let bloom = cl::motion::ENTER_BLOOM.at(now, t0, 0);
        let fade = cl::motion::ENTER_FADE.at(now, t0, 0);
        if cl::motion::ENTER_BLOOM.in_flight(now, t0, 1) {
            ui.ctx().request_repaint();
        }
        let fluid = &stage.fluid;
        let painter = ui.painter();

        let (gap, value_size) = match self.density {
            Density::Comfy => (fluid.sp_5(), fluid.fs_display_xl()),
            Density::Compact => (fluid.sp_3(), fluid.fs_h1()),
        };
        let eyebrow_size = fluid.fs_eyebrow();
        let label_size = fluid.fs_h2();
        let foot_size = fluid.fs_body();

        let block_h = eyebrow_size + gap + value_size * 0.9 + gap + label_size * 1.25 + gap + foot_size;
        let cx = stage.rect.center().x;
        let mut y = stage.rect.center().y - block_h * 0.5;

        painter.text(
            egui::pos2(cx, y),
            Align2::CENTER_TOP,
            tracked(self.eyebrow),
            FontId::monospace(eyebrow_size),
            cl::ink::MUTED.gamma_multiply(fade),
        );
        y += eyebrow_size + gap;
        painter.text(
            egui::pos2(cx, y),
            Align2::CENTER_TOP,
            self.value,
            FontId::proportional(value_size * (0.85 + 0.15 * bloom).max(0.05)),
            self.tone.color().gamma_multiply(bloom.clamp(0.0, 1.0)),
        );
        y += value_size * 0.9 + gap;
        painter.text(
            egui::pos2(cx, y),
            Align2::CENTER_TOP,
            self.label,
            FontId::proportional(label_size),
            cl::ink::BASE.gamma_multiply(fade),
        );
        y += label_size * 1.25 + gap;
        painter.text(
            egui::pos2(cx, y),
            Align2::CENTER_TOP,
            self.footnote,
            FontId::proportional(foot_size),
            cl::ink::MUTED.gamma_multiply(fade),
        );
        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
