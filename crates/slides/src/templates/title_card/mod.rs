//! TitleCard — episode / segment opener. Eyebrow in the slide's tone,
//! display-scale title, muted subtitle. Align left (default) or center.
//!
//! Entrance law (S1b): after the authored settle, the three lines RISE in,
//! staggered — the one-time event, then rest. An optional media slot
//! (image / video / pre-made ambience) composes behind the type.

use dpl_coldlight::Tone;
use dpl_coldlight::{self as cl};
use egui::{Align2, FontId, Ui};
use serde::{Deserialize, Serialize};

use crate::elements::eyebrow::tracked;
use crate::elements::{media_backdrop, Media};
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TitleAlign {
    Left,
    Center,
}

/// Fields are the contract's slots, props and axes (`component.json`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitleCard {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    pub eyebrow: String,
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    pub align: TitleAlign,
    /// The demo's poster is a REAL pipeline artifact (media-cli output), so
    /// the demo doubles as the gallery gate for webp display. A missing file
    /// degrades to the labeled plate, never a crash.
    #[serde(default)]
    pub media: Option<Media>,
    #[serde(skip)]
    clock: Clock,
}

impl TitleCard {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0).expect("the title demo is its example")
    }
}

impl SlideTemplate for TitleCard {
    fn spec(&self) -> &'static ComponentContract {
        Self::contract()
    }

    fn restart(&mut self) {
        self.clock.restart();
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        if let Some(m) = &self.media {
            media_backdrop(ui, ui.max_rect(), m, self.tone, now, t0);
        }
        let fluid = &stage.fluid;
        let painter = ui.painter();
        // The entrance: settle, then each line rises in (one-time event).
        let enter = cl::motion::ENTER_RISE;
        let lift = |p: f32| fluid.sp_5() * (1.0 - p);
        let p0 = enter.at(now, t0, 0);
        let p1 = enter.at(now, t0, 1);
        let p2 = enter.at(now, t0, 2);
        if enter.in_flight(now, t0, 3) {
            ui.ctx().request_repaint();
        }

        let title_size = fluid.fs_display();
        let sub_size = fluid.fs_h3();
        let eyebrow_size = fluid.fs_eyebrow();
        // The block centers vertically; title leads at 0.95 (ported).
        let block_h =
            eyebrow_size + fluid.sp_5() + title_size * 0.95 + fluid.sp_6() + sub_size * 1.3;
        let top = stage.rect.center().y - block_h * 0.5;
        let (x, anchor) = match self.align {
            TitleAlign::Left => (stage.rect.min.x, Align2::LEFT_TOP),
            TitleAlign::Center => (stage.rect.center().x, Align2::CENTER_TOP),
        };

        painter.text(
            egui::pos2(x, top + lift(p0)),
            anchor,
            tracked(&self.eyebrow),
            FontId::monospace(eyebrow_size),
            self.tone.color().gamma_multiply(p0),
        );
        painter.text(
            egui::pos2(x, top + eyebrow_size + fluid.sp_5() + lift(p1)),
            anchor,
            &self.title,
            FontId::proportional(title_size),
            cl::ink::STRONG.gamma_multiply(p1),
        );
        painter.text(
            egui::pos2(
                x,
                top + eyebrow_size + fluid.sp_5() + title_size * 0.95 + fluid.sp_6() + lift(p2),
            ),
            anchor,
            &self.subtitle,
            FontId::proportional(sub_size),
            cl::ink::MUTED.gamma_multiply(p2),
        );
        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
