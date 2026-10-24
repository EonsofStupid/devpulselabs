//! CalloutCard — one emphasized statement on a toned plate.
//!
//! The display-sized cousin of the badge block, carried: left tone bar +
//! top sheen on a raised card. Entrance: the card rises in on the settle.

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
        // Its own values, painted by the one renderer (LS4a): the gallery
        // shows exactly what a card started from this layout looks like.
        let values = crate::registry::ComponentValues::values(self);
        crate::document::paint_template(ui, Self::contract(), &values, &mut self.clock);
    }
}
