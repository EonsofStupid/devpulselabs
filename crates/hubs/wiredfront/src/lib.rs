//! wiredFRONT — capture and media editing: PixVix, the video editor
//! (ADR-010: stitch, ComfyUI workflows), image ops (cutout via ort).

use dpl_coldlight::Tone;
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};

pub const HUB: HubId = HubId("wiredfront");
pub const OVERVIEW: SubId = SubId("overview");
pub const PIXVIX: SubId = SubId("pixvix");
pub const VIZIWIZ: SubId = SubId("viziwiz");
pub const GALLERY: SubId = SubId("gallery");
pub const SESSIONS: SubId = SubId("sessions");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "wiredFRONT",
    icon: "⛶",
    tone: Tone::Magenta,
    subroutes: &[
        SubRoute {
            id: OVERVIEW,
            label: "Overview",
            blurb: "The hub floor: inbox review, every capture vehicle one click away.",
        },
        SubRoute {
            id: PIXVIX,
            label: "PixVix",
            blurb: "The snapper — stills, clips, region, freeform. The engine port lands here first.",
        },
        SubRoute {
            id: VIZIWIZ,
            label: "ViziWiz",
            blurb: "Image ops: annotate, flatten, cutout (ort + BiRefNet — no Node, no segfault).",
        },
        SubRoute {
            id: GALLERY,
            label: "Gallery",
            blurb: "The content library: every artifact, thumbnailed, previewed in place.",
        },
        SubRoute {
            id: SESSIONS,
            label: "Sessions",
            blurb: "Session recordings — video-first (the rrweb path retired with the webview).",
        },
    ],
};

#[derive(Default)]
pub struct WiredfrontHub;

impl Hub for WiredfrontHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        dpl_hub::placeholder::surface(
            ui,
            &SPEC,
            route,
            "Capture, stitch, generate, edit. ADR-010 is this hub's law: shot slots, \
             ffmpeg subprocess, ComfyUI as the first pluggable backend.",
        );
    }
}
