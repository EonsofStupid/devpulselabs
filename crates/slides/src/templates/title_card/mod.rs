//! TitleCard — episode / segment opener. Eyebrow in the slide's tone,
//! display-scale title, muted subtitle. Align left (default) or center.
//!
//! Entrance law (S1b): after the authored settle, the three lines RISE in,
//! staggered — the one-time event, then rest. An optional media slot
//! (image / video / pre-made ambience) composes behind the type.

use dpl_coldlight::Tone;
use egui::Ui;
use serde::{Deserialize, Serialize};

use crate::elements::Media;
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
        // Its own values, painted by the one renderer (LS4a): the gallery
        // shows exactly what a card started from this layout looks like.
        let values = crate::registry::ComponentValues::values(self);
        crate::document::paint_template(ui, Self::contract(), &values, &mut self.clock);
    }
}
