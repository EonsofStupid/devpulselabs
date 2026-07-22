//! Workbench — the internal slide + full-episode builder: blocks, elements,
//! templates, then effects (the operator's stated stage order).

use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};

pub const HUB: HubId = HubId("workbench");
pub const STUDIO: SubId = SubId("studio");
pub const LIBRARY: SubId = SubId("library");
pub const TOKENS: SubId = SubId("tokens");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Workbench",
    icon: cl::icon::WORKBENCH,
    tone: Tone::Amber,
    subroutes: &[
        SubRoute {
            id: STUDIO,
            label: "Studio",
            blurb: "Compose episodes: blocks → elements → templates → effects, onto sequenced slides.",
        },
        SubRoute {
            id: LIBRARY,
            label: "Library",
            blurb: "Every registered building block, rendered live with fixture props.",
        },
        SubRoute {
            id: TOKENS,
            label: "Tokens",
            blurb: "ColdLight's tones, spacing, and paint vocabulary — the authored source, visualized.",
        },
    ],
};

#[derive(Default)]
pub struct WorkbenchHub;

impl Hub for WorkbenchHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        dpl_hub::placeholder::surface(
            ui,
            &SPEC,
            route,
            "The episode factory. Everything captured or generated ends up on a slide here.",
        );
    }
}
