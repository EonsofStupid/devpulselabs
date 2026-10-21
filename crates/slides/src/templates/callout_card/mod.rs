//! CalloutCard — one emphasized statement on a toned plate.
//!
//! The display-sized cousin of the badge block, carried: left tone bar +
//! top sheen on a raised card. Entrance: the card rises in on the settle.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Rect, Ui};
use serde::{Deserialize, Serialize};

use crate::elements::eyebrow::tracked;
use crate::elements::{media_backdrop, Media};
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

/// Fields are the contract's slots, props and axes (`component.json`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalloutCard {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    #[serde(default)]
    pub kicker: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    /// The demo's Drift is a SLIDE ambience (ratified 2026-07-22) — this
    /// demo is its gallery-gate surface.
    #[serde(default)]
    pub media: Option<Media>,
    #[serde(skip)]
    clock: Clock,
}

impl CalloutCard {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0).expect("the callout demo is its example")
    }
}

impl SlideTemplate for CalloutCard {
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
        if let Some(m) = &self.media {
            media_backdrop(ui, ui.max_rect(), m, self.tone, now, t0);
        }
        let painter = ui.painter();

        let m = cl::motion::ENTER_RISE.sample(now, t0, 0);
        if cl::motion::ENTER_RISE.in_flight(now, t0, 1) {
            ui.ctx().request_repaint();
        }
        let alpha = m.alpha;
        // The effect says HOW FAR as a fraction; Fluid says how far that is in
        // this window. Neither one owns both.
        let lift = fluid.sp_6() * m.offset.y;

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
            tracked(&self.kicker),
            FontId::monospace(fluid.fs_eyebrow()),
            c.gamma_multiply(alpha),
        );
        y += fluid.fs_eyebrow() + fluid.sp_5();
        painter.text(
            egui::pos2(x, y),
            Align2::LEFT_TOP,
            &self.title,
            FontId::proportional(fluid.fs_h2()),
            cl::ink::STRONG.gamma_multiply(alpha),
        );
        y += fluid.fs_h2() * 1.15 + fluid.sp_4();
        painter.text(
            egui::pos2(x, y),
            Align2::LEFT_TOP,
            &self.body,
            FontId::proportional(fluid.fs_body_lg()),
            cl::ink::MUTED.gamma_multiply(alpha),
        );
        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
