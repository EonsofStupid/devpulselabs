//! Home — shows and the touchscreen runtime land here (P3+).

use dpl_coldlight::Tone;
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};

pub const HUB: HubId = HubId("home");
pub const SHOWS: SubId = SubId("shows");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Home",
    icon: "⚑",
    tone: Tone::Cyan,
    subroutes: &[SubRoute {
        id: SHOWS,
        label: "Shows",
        blurb: "Episodes and the touchscreen runtime — the deck this app exists to feed.",
    }],
};

#[derive(Default)]
pub struct HomeHub;

impl Hub for HomeHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        dpl_hub::placeholder::surface(
            ui,
            &SPEC,
            route,
            "One stop for capture, editing, authoring, and — eventually — streaming and posting.",
        );
    }
}
