//! Library — the component gallery: every registered building block, live.

use dpl_coldlight::Tone;
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};

pub const HUB: HubId = HubId("library");
pub const COMPONENTS: SubId = SubId("components");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Library",
    icon: "⛃",
    tone: Tone::Violet,
    subroutes: &[SubRoute {
        id: COMPONENTS,
        label: "Component gallery",
        blurb: "Every widget in the ColdLight vocabulary, rendered live — the visual regression floor.",
    }],
};

#[derive(Default)]
pub struct LibraryHub;

impl Hub for LibraryHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        dpl_hub::placeholder::surface(
            ui,
            &SPEC,
            route,
            "The proof surface: if it renders here, it exists.",
        );
    }
}
