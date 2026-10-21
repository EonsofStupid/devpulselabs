//! Quote — the pull-quote slide: an oversized quotation mark, the line in
//! display type, a tone divider, then attribution (name + role). The staple
//! for a testimonial, a thesis statement, or a memorable one-liner.
//!
//! Entrance: the mark, quote, and attribution RISE in together on the settle,
//! then rest (presenter law — one event, then a still frame).

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, Color32, FontId, Ui};
use serde::{Deserialize, Serialize};

use crate::elements::divider::rule;
use crate::elements::eyebrow::tracked;
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

/// Fields are the contract's slots, props and axes (`component.json`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quote {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    #[serde(default)]
    pub kicker: String,
    pub quote: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(skip)]
    clock: Clock,
}

impl Quote {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0).expect("the quote demo is its example")
    }
}

impl SlideTemplate for Quote {
    fn spec(&self) -> &'static ComponentContract {
        Self::contract()
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
        let lift = fluid.sp_6() * (1.0 - p);
        let tone = self.tone.color();

        // The decorative mark (ASCII quote, font-safe) — big, dim, tone.
        painter.text(
            egui::pos2(stage.rect.min.x, stage.rect.min.y + lift),
            Align2::LEFT_TOP,
            "\"",
            FontId::proportional(fluid.fs_display_xl()),
            Color32::from_rgba_unmultiplied(tone.r(), tone.g(), tone.b(), (90.0 * alpha) as u8),
        );

        // The kicker eyebrow.
        painter.text(
            egui::pos2(
                stage.rect.min.x,
                stage.rect.min.y + fluid.fs_display() * 0.9 + lift,
            ),
            Align2::LEFT_TOP,
            tracked(&self.kicker),
            FontId::monospace(fluid.fs_eyebrow()),
            tone.gamma_multiply(alpha),
        );

        // The quote body, wrapped in display type, centered vertically.
        let wrap = stage.rect.width() * 0.92;
        let galley = painter.layout(
            self.quote.clone(),
            FontId::proportional(fluid.fs_h1()),
            cl::ink::STRONG.gamma_multiply(alpha),
            wrap,
        );
        let qy = stage.rect.center().y - galley.size().y * 0.5 + lift;
        painter.galley(
            egui::pos2(stage.rect.min.x, qy),
            galley.clone(),
            cl::ink::STRONG,
        );

        // Divider + attribution below the quote.
        let ay = qy + galley.size().y + fluid.sp_6();
        rule(
            painter,
            stage.rect.min.x,
            ay,
            stage.rect.width() * 0.5,
            self.tone,
            alpha,
        );
        painter.text(
            egui::pos2(stage.rect.min.x, ay + fluid.sp_4()),
            Align2::LEFT_TOP,
            &self.name,
            FontId::proportional(fluid.fs_h4()),
            cl::ink::STRONG.gamma_multiply(alpha),
        );
        painter.text(
            egui::pos2(stage.rect.min.x, ay + fluid.sp_4() + fluid.fs_h4() * 1.2),
            Align2::LEFT_TOP,
            tracked(&self.role),
            FontId::monospace(fluid.fs_eyebrow()),
            cl::ink::MUTED.gamma_multiply(alpha),
        );

        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
