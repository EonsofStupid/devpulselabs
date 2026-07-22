//! TitleCard — episode / segment opener. Eyebrow in the slide's tone,
//! display-scale title, muted subtitle. Align left (default) or center.

use dpl_coldlight::{self as cl};
use dpl_coldlight::Tone;
use egui::{Align2, FontId, Ui};

use crate::blocks::eyebrow::tracked;
use crate::frame::Stage;
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
}

impl TitleCard {
    pub fn demo() -> Self {
        Self {
            tone: Tone::Amber,
            eyebrow: "Echoes of Sentience",
            title: "The stack, laid bare",
            subtitle: "What actually runs when you ask a model a question — and what it costs.",
            align: TitleAlign::Left,
        }
    }
}

impl SlideTemplate for TitleCard {
    fn spec(&self) -> &'static TemplateSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut Ui) {
        let stage = Stage::of(ui.max_rect());
        let fluid = &stage.fluid;
        let painter = ui.painter();

        let title_size = fluid.fs_display();
        let sub_size = fluid.fs_h3();
        let eyebrow_size = fluid.fs_eyebrow();
        // The block centers vertically; title leads at 0.95 (ported).
        let block_h = eyebrow_size + fluid.sp_5() + title_size * 0.95 + fluid.sp_6() + sub_size * 1.3;
        let top = stage.rect.center().y - block_h * 0.5;
        let (x, anchor) = match self.align {
            TitleAlign::Left => (stage.rect.min.x, Align2::LEFT_TOP),
            TitleAlign::Center => (stage.rect.center().x, Align2::CENTER_TOP),
        };

        painter.text(
            egui::pos2(x, top),
            anchor,
            tracked(self.eyebrow),
            FontId::monospace(eyebrow_size),
            self.tone.color(),
        );
        painter.text(
            egui::pos2(x, top + eyebrow_size + fluid.sp_5()),
            anchor,
            self.title,
            FontId::proportional(title_size),
            cl::ink::STRONG,
        );
        painter.text(
            egui::pos2(x, top + eyebrow_size + fluid.sp_5() + title_size * 0.95 + fluid.sp_6()),
            anchor,
            self.subtitle,
            FontId::proportional(sub_size),
            cl::ink::MUTED,
        );
        ui.allocate_rect(stage.rect, egui::Sense::hover());
    }
}
