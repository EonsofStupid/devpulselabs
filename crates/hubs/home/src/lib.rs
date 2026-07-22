//! Home — the mainpage. Its backdrop is one of the two authored ambient
//! loops (PV0), and the SUBROUTES are the switcher: `home/drift` and
//! `home/pulse` — rail-clickable for the operator, `--route`-addressable
//! for the harness. Shows/deck land here in the presenter pass.

use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use dpl_slides::blocks::{media_backdrop, Ambient, Media};
use dpl_slides::motion::Clock;
use egui::{Align2, FontId};

pub const HUB: HubId = HubId("home");
pub const DRIFT: SubId = SubId("drift");
pub const PULSE: SubId = SubId("pulse");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Home",
    icon: "⚑",
    tone: Tone::Cyan,
    subroutes: &[
        SubRoute {
            id: DRIFT,
            label: "Drift loop",
            blurb: "Slow diagonal tone washes — the calm mainpage ambience.",
        },
        SubRoute {
            id: PULSE,
            label: "Pulse loop",
            blurb: "A breathing ring constellation — the live mainpage ambience.",
        },
    ],
};

#[derive(Default)]
pub struct HomeHub {
    clock: Clock,
}

impl Hub for HomeHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        let full = ui.max_rect();
        let now = ui.input(|i| i.time);
        let t0 = self.clock.tick(now);
        let fluid = cl::Fluid::of(full);

        // The backdrop: subroute picks the loop (unknown degrades to Drift).
        let (ambient, tone) = if route.sub == PULSE {
            (Ambient::Pulse, Tone::Violet)
        } else {
            (Ambient::Drift, Tone::Cyan)
        };
        media_backdrop(ui, full, Media::Anim(ambient), tone, now, t0);

        // The hero over the ambience.
        let painter = ui.painter();
        let cy = full.center().y - fluid.fs_display() * 0.5;
        painter.text(
            egui::pos2(full.center().x, cy - fluid.sp_5()),
            Align2::CENTER_BOTTOM,
            dpl_slides::blocks::eyebrow::tracked("one stop · capture · author · present"),
            FontId::monospace(fluid.fs_eyebrow()),
            tone.color(),
        );
        painter.text(
            egui::pos2(full.center().x, cy),
            Align2::CENTER_TOP,
            "DevPULSE Labs",
            FontId::proportional(fluid.fs_display()),
            cl::ink::STRONG,
        );
        painter.text(
            egui::pos2(full.center().x, cy + fluid.fs_display() + fluid.sp_4()),
            Align2::CENTER_TOP,
            "Capture it. Author it. Put it on the wall.",
            FontId::proportional(fluid.fs_h3()),
            cl::ink::MUTED,
        );
    }
}
