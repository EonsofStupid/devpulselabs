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

use dpl_coldlight::Tone;
use egui::Ui;
use serde::{Deserialize, Serialize};

use crate::elements::Media;
use crate::motion::Clock;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

/// Fields are the contract's slots, props and axes (`component.json`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaFrame {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    /// The demo's poster is a REAL pipeline artifact — it doubles as the
    /// gallery gate for the media path. A missing file degrades to the
    /// labeled plate, never a crash (the same honesty the image slot carries
    /// everywhere).
    pub media: Media,
    #[serde(default)]
    pub caption: String,
    #[serde(skip)]
    clock: Clock,
}

impl MediaFrame {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0).expect("the media demo is its example")
    }
}

impl SlideTemplate for MediaFrame {
    fn spec(&self) -> &'static ComponentContract {
        Self::contract()
    }

    fn restart(&mut self) {
        self.clock.restart();
    }

    fn ui(&mut self, ui: &mut Ui) {
        // Its own values, painted by the one renderer (LS4a): the gallery
        // shows exactly what a card started from this layout looks like.
        let values = crate::registry::ComponentValues::values(self);
        crate::document::paint_template(ui, Self::contract(), &values, &mut self.clock);
    }
}
