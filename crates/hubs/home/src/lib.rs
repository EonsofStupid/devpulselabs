//! Home — the mainpage. The BACKDROP is not this hub's business: the shell
//! paints the app-wide backdrop (two loops + authored random moods,
//! `crates/backdrop`) under every route, and its tiles live in the topbar.
//! Home renders the hero over it; the Shows/episode picker lands here with
//! the content port. Drift/Pulse are SLIDE ambiences (ratified 2026-07-22)
//! and live in the slides gallery, not here.

use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use egui::{Align2, FontId};

pub const HUB: HubId = HubId("home");
pub const MAIN: SubId = SubId("main");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Home",
    icon: cl::icon::HOME,
    tone: Tone::Cyan,
    subroutes: &[SubRoute {
        id: MAIN,
        label: "Mainpage",
        blurb: "The hero over the app backdrop — Shows land here with the content port.",
    }],
};

#[derive(Default)]
pub struct HomeHub;

impl Hub for HomeHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, _route: Route) {
        let full = ui.max_rect();
        let fluid = cl::Fluid::of(full);
        let painter = ui.painter();

        let cy = full.center().y - fluid.fs_display() * 0.5;
        painter.text(
            egui::pos2(full.center().x, cy - fluid.sp_5()),
            Align2::CENTER_BOTTOM,
            dpl_slides::blocks::eyebrow::tracked("one stop · capture · author · present"),
            FontId::monospace(fluid.fs_eyebrow()),
            SPEC.tone.color(),
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
