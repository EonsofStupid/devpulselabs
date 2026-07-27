//! MediaFrame — the drop-anything slide (W11). An imported image, video,
//! PDF page, or session replay, rendered FULL-BLEED. This is the template the
//! Library "Make slide" gesture stamps onto raw media: the slide itself is
//! DATA (a `dpl_doc::Block::Media`); this template is how it paints in the
//! gallery. The document renderer paints the real authored slides — this demo
//! exists so the gallery gate covers the media path (a template that never
//! shows cannot exist).
//!
//! Presenter law: a still rests; a looping video is a rest state by
//! construction. Nothing auto-advances.

use dpl_coldlight::{self as cl, Tone};
use egui::{Align2, FontId, Ui};

use crate::blocks::{media_backdrop, Media};
use crate::frame::Stage;
use crate::motion::Clock;
use crate::registry::{SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

static SPEC: TemplateSpec = TemplateSpec {
    id: TemplateId("media"),
    label: "Media frame",
    class: TemplateClass::Static,
    blurb: "Drop-anything slide: an image, video, PDF page, or session replay — full-bleed.",
};

pub struct MediaFrame {
    pub tone: Tone,
    pub media: Media,
    pub caption: &'static str,
    clock: Clock,
}

impl MediaFrame {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Cyan,
            // A REAL pipeline artifact — doubles as the gallery gate for the
            // media path. A missing file degrades to the labeled plate, never
            // a crash (the same honesty the image slot carries everywhere).
            media: Media::Image("assets/generated/dpl-poster.webp"),
            caption: "any image · video · PDF page · replay",
            clock: Clock::default(),
        }
    }
}

impl SlideTemplate for MediaFrame {
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

        // The whole stage is the media — the media_frame is a full-bleed slot.
        media_backdrop(ui, stage.rect, self.media, self.tone, now, t0);

        // A quiet corner caption naming what a media slide carries. It rises
        // once with the entrance, then rests.
        let p = cl::motion::ENTER_RISE.at(now, t0, 0);
        if cl::motion::ENTER_RISE.in_flight(now, t0, 1) {
            ui.ctx().request_repaint();
        }
        let alpha = p.clamp(0.0, 1.0);
        let lift = fluid.sp_5() * (1.0 - p);
        let pos = egui::pos2(
            stage.rect.min.x + fluid.sp_7(),
            stage.rect.max.y - fluid.sp_6() + lift,
        );
        ui.painter().text(
            pos,
            Align2::LEFT_BOTTOM,
            self.caption,
            FontId::monospace(fluid.fs_eyebrow()),
            self.tone.color().gamma_multiply(alpha),
        );
    }
}
