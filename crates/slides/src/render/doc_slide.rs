//! A document with its own clock.

use super::{render, render_thumbnail};
use crate::motion::Clock;
use egui::{Rect, Ui};

/// A document with its own clock — the wall/stage wrapper.
pub struct DocSlide {
    pub slide: dpl_doc::Slide,
    styling: dpl_doc::Styling,
    clock: Clock,
}

impl DocSlide {
    /// A document outside any show — the fallback theme, with `looks`
    /// (the built-in looks plus the operator's own).
    pub fn new(slide: dpl_doc::Slide, looks: dpl_doc::Looks) -> Self {
        Self::styled(
            slide,
            dpl_doc::Styling {
                theme: dpl_doc::Theme::fallback(),
                looks,
            },
        )
    }

    /// A document inside a show — its theme and looks.
    pub fn styled(slide: dpl_doc::Slide, styling: dpl_doc::Styling) -> Self {
        Self {
            slide,
            styling,
            clock: Clock::default(),
        }
    }

    pub fn restart(&mut self) {
        self.clock.restart();
    }

    pub fn ui(&mut self, ui: &mut Ui, rect: Rect) {
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        render(ui, rect, &self.slide, &self.styling, now, t0);
    }

    /// Draw this document as a miniature — see [`Stage::Thumbnail`].
    pub fn thumbnail_ui(&mut self, ui: &mut Ui, rect: Rect) {
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        render_thumbnail(ui, rect, &self.slide, &self.styling, now, t0);
    }
}
