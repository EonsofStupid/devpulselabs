//! TitleCard — episode / segment opener. Eyebrow in the slide's tone,
//! display-scale title, muted subtitle. Align left (default) or center.
//!
//! Entrance law (S1b): after the authored settle, the three lines RISE in,
//! staggered — the one-time event, then rest. An optional media slot
//! (image / video / pre-made ambience) composes behind the type.

use dpl_coldlight::Tone;
use dpl_coldlight::{self as cl};
use egui::{Align2, FontId, Ui};

use crate::elements::eyebrow::tracked;
use crate::elements::{media_backdrop, Media};
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("title"),
    label: "Title card",
    class: TemplateClass::Static,
    blurb: "Episode/segment opener: toned eyebrow, display title, muted subtitle.",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleAlign {
    Left,
    Center,
}

pub struct TitleCard {
    pub tone: Tone,
    pub eyebrow: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub align: TitleAlign,
    pub media: Option<Media>,
    clock: Clock,
}

impl TitleCard {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Amber,
            eyebrow: "Echoes of Sentience",
            title: "The stack, laid bare",
            subtitle: "What actually runs when you ask a model a question — and what it costs.",
            align: TitleAlign::Left,
            // A REAL pipeline artifact (media-cli output) — the demo doubles
            // as the gallery gate for webp display. Missing file degrades to
            // the labeled plate, never a crash.
            media: Some(Media::Image("assets/generated/dpl-poster.webp")),
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for TitleCard {
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
        if let Some(m) = self.media {
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
            tracked(self.eyebrow),
            FontId::monospace(eyebrow_size),
            self.tone.color().gamma_multiply(p0),
        );
        painter.text(
            egui::pos2(x, top + eyebrow_size + fluid.sp_5() + lift(p1)),
            anchor,
            self.title,
            FontId::proportional(title_size),
            cl::ink::STRONG.gamma_multiply(p1),
        );
        painter.text(
            egui::pos2(
                x,
                top + eyebrow_size + fluid.sp_5() + title_size * 0.95 + fluid.sp_6() + lift(p2),
            ),
            anchor,
            self.subtitle,
            FontId::proportional(sub_size),
            cl::ink::MUTED.gamma_multiply(p2),
        );
        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
