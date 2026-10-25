//! Quote — the pull-quote slide: an oversized quotation mark, the line in
//! display type, a tone divider, then attribution (name + role). The staple
//! for a testimonial, a thesis statement, or a memorable one-liner.
//!
//! Entrance: the mark, quote, and attribution RISE in together on the settle,
//! then rest (presenter law — one event, then a still frame).

use dpl_coldlight::Tone;
use egui::Ui;
use serde::{Deserialize, Serialize};

use crate::motion::Clock;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

/// Fields are the contract's slots, props and axes (`component.json`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quote {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    #[serde(default)]
    pub kicker: String,
    pub quote: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(skip)]
    clock: Clock,
}

impl Quote {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0).expect("the quote demo is its example")
    }
}

impl SlideTemplate for Quote {
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
