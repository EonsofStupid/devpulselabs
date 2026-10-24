//! BigNumber — the receipt: ONE verified number, wall-sized.
//!
//! Doctrine carried: every chapter lands exactly one of these; the footnote
//! is where the source/verification note lives — an unverified number never
//! gets this treatment. Value rides the slide's tone; density changes gaps
//! and the value scale only.

use dpl_coldlight::Tone;
use egui::Ui;
use serde::{Deserialize, Serialize};

use crate::elements::Media;
use crate::motion::Clock;
use crate::registry::SlideTemplate;
use crate::Density;
use dpl_doc::component::ComponentContract;

/// Fields are the contract's slots, props and axes (`component.json`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BigNumber {
    #[serde(with = "crate::contract::tone")]
    pub tone: Tone,
    pub density: Density,
    #[serde(default)]
    pub eyebrow: String,
    pub value: String,
    pub label: String,
    /// Where the source/verification note lives (the doctrine above).
    #[serde(default)]
    pub footnote: String,
    /// The demo's Pulse is a SLIDE ambience (ratified 2026-07-22) — this
    /// demo is its gallery-gate surface.
    #[serde(default)]
    pub media: Option<Media>,
    #[serde(skip)]
    clock: Clock,
}

impl BigNumber {
    crate::component_contract!(include_str!("component.json"));

    pub fn demo() -> Self {
        crate::contract::instantiate(Self::contract(), 0).expect("the stat demo is its example")
    }
}

impl SlideTemplate for BigNumber {
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
