//! CalloutCard — one emphasized statement on a toned plate.
//!
//! The display-sized cousin of the badge block, carried: left tone bar +
//! top sheen on a raised card. Entrance: the card rises in on the settle.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Ui};

use crate::elements::eyebrow::tracked;
use crate::elements::{media_backdrop, Ambient, Media};
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("callout"),
    label: "Callout card",
    class: TemplateClass::Static,
    blurb: "One emphasized statement: tone bar, sheen, kicker eyebrow — rises in on the settle.",
};

pub struct CalloutCard {
    pub tone: Tone,
    pub kicker: &'static str,
    pub title: &'static str,
    pub body: &'static str,
    pub media: Option<Media>,
    clock: Clock,
}

impl CalloutCard {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Magenta,
            kicker: "the catch",
            title: "Free tiers are the demo, not the product",
            body: "Every hosted model you don't pay for is pricing you in tokens of patience.",
            // Drift is a SLIDE ambience (ratified 2026-07-22) — this demo is
            // its gallery-gate surface.
            media: Some(Media::Anim(Ambient::Drift)),
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for CalloutCard {
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
        if let Some(m) = self.media {
            media_backdrop(ui, ui.max_rect(), m, self.tone, now, t0);
        }
        let painter = ui.painter();

        let p = cl::motion::ENTER_RISE.at(now, t0, 0);
        if cl::motion::ENTER_RISE.in_flight(now, t0, 1) {
            ui.ctx().request_repaint();
        }
        let alpha = p.clamp(0.0, 1.0);
        let lift = fluid.sp_6() * (1.0 - p);

        let card_h = (stage.rect.height() * 0.55).max(fluid.sp_8() * 3.0);
        let card = Rect::from_center_size(
            stage.rect.center() + egui::vec2(0.0, lift),
            egui::vec2(stage.rect.width() * 0.82, card_h),
        );
        let radius = fluid.r_lg() as u8;
        let c = self.tone.color();

        painter.rect_filled(card, radius, cl::surface::RAISED.gamma_multiply(alpha));
        painter.rect_stroke(
            card,
            radius,
            egui::Stroke::new(1.0, cl::surface::HAIRLINE.gamma_multiply(alpha)),
            egui::StrokeKind::Inside,
        );
        // The sheen band + the left tone bar — the carried anatomy.
        cl::sheen(painter, card);
        painter.rect_filled(
            Rect::from_min_size(card.min, egui::vec2(4.0, card.height())),
            0,
            egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (200.0 * alpha) as u8),
        );

        let x = card.min.x + fluid.sp_7();
        let mut y = card.min.y + fluid.sp_6();
        painter.text(
            egui::pos2(x, y),
            Align2::LEFT_TOP,
            tracked(self.kicker),
            FontId::monospace(fluid.fs_eyebrow()),
            c.gamma_multiply(alpha),
        );
        y += fluid.fs_eyebrow() + fluid.sp_5();
        painter.text(
            egui::pos2(x, y),
            Align2::LEFT_TOP,
            self.title,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG.gamma_multiply(alpha),
        );
        y += fluid.fs_h2() * 1.15 + fluid.sp_4();
        painter.text(
            egui::pos2(x, y),
            Align2::LEFT_TOP,
            self.body,
            FontId::proportional(fluid.fs_body_lg()),
            cl::ink::MUTED.gamma_multiply(alpha),
        );
        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
