//! SectionDivider — the chapter break: a giant tone section number, the
//! section title in display type, a kicker eyebrow, and a divider rule. The
//! palate cleanser between movements of a talk.
//!
//! Entrance: the number scales up while the title rises — one settle, at rest.

use dpl_coldlight::Tone;
use egui::Ui;
use serde::{Deserialize, Serialize};

use crate::motion::Clock;
use crate::registry::SlideTemplate;
use dpl_doc::component::ComponentContract;

/// Fields are the contract's slots and props (`component.json`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionDivider {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    pub number: String,
    #[serde(default)]
    pub kicker: String,
    pub title: String,
    #[serde(skip)]
    clock: Clock,
}

impl SectionDivider {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0).expect("the section demo is its example")
    }
}

impl SlideTemplate for SectionDivider {
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
