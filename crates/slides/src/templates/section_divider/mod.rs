//! SectionDivider — the chapter break: a giant tone section number, the
//! section title in display type, a kicker eyebrow, and a divider rule. The
//! palate cleanser between movements of a talk.
//!
//! Entrance: the number scales up while the title rises — one settle, at rest.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Ui};

use crate::blocks::divider::rule;
use crate::blocks::eyebrow::tracked;
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("section"),
    label: "Section divider",
    class: TemplateClass::Static,
    blurb: "A giant tone section number, a display title, and a rule — the chapter break.",
};

pub struct SectionDivider {
    pub tone: Tone,
    pub number: &'static str,
    pub kicker: &'static str,
    pub title: &'static str,
    clock: Clock,
}

impl SectionDivider {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Amber,
            number: "02",
            kicker: "Part two",
            title: "What it actually costs",
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for SectionDivider {
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

        let rise = cl::motion::ENTER_RISE;
        if rise.in_flight(now, t0, 1) {
            ui.ctx().request_repaint();
        }
        let p = rise.at(now, t0, 0);
        let alpha = p.clamp(0.0, 1.0);
        let lift = fluid.sp_7() * (1.0 - p);
        let tone = self.tone.color();

        // The giant number — the eye's anchor, top-left.
        painter.text(
            egui::pos2(stage.rect.min.x, stage.rect.min.y + lift),
            Align2::LEFT_TOP,
            self.number,
            FontId::proportional(fluid.fs_display_xl()),
            tone.gamma_multiply(alpha),
        );

        // Kicker + title stacked in the lower third.
        let ky = stage.rect.max.y - fluid.fs_display() - fluid.fs_eyebrow() - fluid.sp_6();
        rule(
            painter,
            stage.rect.min.x,
            ky - fluid.sp_4(),
            stage.rect.width(),
            self.tone,
            alpha,
        );
        painter.text(
            egui::pos2(stage.rect.min.x, ky - lift),
            Align2::LEFT_TOP,
            tracked(self.kicker),
            FontId::monospace(fluid.fs_eyebrow()),
            tone.gamma_multiply(alpha),
        );
        painter.text(
            egui::pos2(
                stage.rect.min.x,
                ky + fluid.fs_eyebrow() + fluid.sp_2() - lift,
            ),
            Align2::LEFT_TOP,
            self.title,
            FontId::proportional(fluid.fs_display()),
            cl::ink::STRONG.gamma_multiply(alpha),
        );

        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
