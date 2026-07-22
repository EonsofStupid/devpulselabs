//! Home — the mainpage, faithful to the old `BackdropStage` chain:
//! **aurora gradient → the selected loop → scrim → hero**. The two loops are
//! the operator's real files (`assets/backdrops/dpl.mp4`,
//! `clyffy-chases-the-dragon.mp4`, copied — never moved — from the old
//! repo), played via the ffmpeg-subprocess player (ADR-009); the authored
//! Drift/Pulse vector ambiences stay as extra options. Subroutes are the
//! switcher — rail-clickable, `--route`-addressable. Default is `dpl`,
//! exactly like the old backdrop store.

use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use dpl_slides::blocks::{media_backdrop, Ambient, Media};
use dpl_slides::motion::Clock;
use egui::{Align2, FontId};

pub const HUB: HubId = HubId("home");
pub const DPL: SubId = SubId("dpl");
pub const DRAGON: SubId = SubId("dragon");
pub const DRIFT: SubId = SubId("drift");
pub const PULSE: SubId = SubId("pulse");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Home",
    icon: cl::icon::HOME,
    tone: Tone::Cyan,
    subroutes: &[
        SubRoute {
            id: DPL,
            label: "DPL loop",
            blurb: "The mainpage loop — dpl.mp4, played live over the aurora.",
        },
        SubRoute {
            id: DRAGON,
            label: "Dragon loop",
            blurb: "Clyffy chases the dragon — the second mainpage loop.",
        },
        SubRoute {
            id: DRIFT,
            label: "Drift ambience",
            blurb: "Slow diagonal tone washes — the authored vector calm.",
        },
        SubRoute {
            id: PULSE,
            label: "Pulse ambience",
            blurb: "A breathing ring constellation — the authored vector live.",
        },
    ],
};

/// What a subroute selects: a video loop or an authored ambience.
enum Backdrop {
    Loop(&'static str),
    Ambience(Ambient, Tone),
}

fn backdrop_of(sub: SubId) -> Backdrop {
    match sub {
        s if s == DRAGON => Backdrop::Loop("assets/backdrops/clyffy-chases-the-dragon.mp4"),
        s if s == DRIFT => Backdrop::Ambience(Ambient::Drift, Tone::Cyan),
        s if s == PULSE => Backdrop::Ambience(Ambient::Pulse, Tone::Violet),
        // Default (and unknown): the DPL loop, like the old store's "dpl".
        _ => Backdrop::Loop("assets/backdrops/dpl.mp4"),
    }
}

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

        // Layer 1 — the aurora base, ALWAYS painted (the old BackdropStage
        // law: it is the fallback under everything).
        cl::aurora(ui.painter(), full);

        // Layer 2 — the selected loop/ambience, painted DIRECTLY (not via
        // the slide media slot: that block adds its own legibility diffusion,
        // which would stack with the scrim and land darker than the old app).
        // Layer 3 — the scrim: 70% over video/poster, 35% otherwise —
        // verbatim `bg-surface/70` / `bg-surface/35`.
        let tone = match backdrop_of(route.sub) {
            Backdrop::Loop(rel) => {
                let media_up = match dpl_video::locate_asset(rel) {
                    Some(file) => {
                        dpl_video::loop_backdrop(ui, full, &file) || {
                            let poster = dpl_video::poster_of(&file);
                            let up = poster.is_file();
                            if up {
                                // file:/// — two slashes is a UNC host on Windows.
                                let uri = poster.to_string_lossy().replace('\\', "/");
                                egui::Image::new(format!("file:///{uri}")).paint_at(ui, full);
                            }
                            up
                        }
                    }
                    None => false, // aurora-only — honest, never a crash
                };
                let a = if media_up { cl::SCRIM_MEDIA_A } else { cl::SCRIM_BARE_A };
                ui.painter().rect_filled(full, 0, cl::scrim(a));
                Tone::Cyan
            }
            Backdrop::Ambience(ambient, tone) => {
                media_backdrop(ui, full, Media::Anim(ambient), tone, now, t0);
                ui.painter().rect_filled(full, 0, cl::scrim(cl::SCRIM_BARE_A));
                tone
            }
        };

        // Layer 4 — the hero.
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
