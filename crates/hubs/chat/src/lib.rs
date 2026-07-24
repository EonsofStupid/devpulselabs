//! Chat — the five-provider mesh (fabric plan §6): claude · codex · gemini ·
//! grok · clyffy (local, OpenAI-compatible). Route awareness comes free here:
//! this hub subscribes to `ShellEvent::RouteChanged` on the fabric — the
//! signal already exists before the first provider does.

use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use egui::RichText;

pub const HUB: HubId = HubId("chat");
pub const SESSIONS: SubId = SubId("sessions");
pub const COUNCIL: SubId = SubId("council");

/// The provider roster, as data — one row per tab, tone-coded.
const PROVIDERS: &[(&str, Tone)] = &[
    ("claude", Tone::Amber),
    ("codex", Tone::Cyan),
    ("gemini", Tone::Violet),
    ("grok", Tone::Magenta),
    ("clyffy", Tone::Lime),
];

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Chat",
    icon: cl::icon::BOT,
    tone: Tone::Lime,
    subroutes: &[
        SubRoute {
            id: SESSIONS,
            label: "Sessions",
            blurb: "Provider tabs — session readers first (the C1 pattern), then driving.",
        },
        SubRoute {
            id: COUNCIL,
            label: "Council",
            blurb: "Council of C: operator/AI roles, reversible direction, COSTAR engagements.",
        },
    ],
};

#[derive(Default)]
pub struct ChatHub {
    /// The NAVIGATE seam: tapped cards publish through here.
    fabric: Option<dpl_fabric::Fabric>,
}

impl ChatHub {
    pub fn wire_fabric(&mut self, fabric: &dpl_fabric::Fabric) {
        self.fabric = Some(fabric.clone());
    }
}

impl Hub for ChatHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        if let Some(go) = dpl_hub::placeholder::surface(
            ui,
            &SPEC,
            route,
            "Route-aware, scaffolded for five providers from day one.",
        ) {
            if let Some(fabric) = &self.fabric {
                fabric.publish_shell(dpl_fabric::ShellEvent::NavigateRequest { route: go });
            }
        }

        ui.add_space(cl::space::S4);
        ui.label(
            RichText::new("PROVIDER SLOTS")
                .small()
                .color(cl::ink::SUBTLE),
        );
        ui.horizontal(|ui| {
            for (name, tone) in PROVIDERS {
                cl::card_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        cl::tone_dot(ui, *tone);
                        ui.label(RichText::new(*name).monospace().color(cl::ink::BASE));
                    });
                });
            }
        });
    }
}
