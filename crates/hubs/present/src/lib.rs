//! Present — the show machine's front door (D2).
//!
//! SHOWS lists the episodes; tapping one opens the DeckHost and asks the
//! shell for full-bleed over the fabric (`ShellEvent::FullBleedRequest`).
//! Leaving the show (toolbar Home / home dot) hands the chrome back.

use dpl_coldlight::{self as cl, Tone};
use dpl_fabric::{Fabric, ShellEvent};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use egui::RichText;
use std::path::PathBuf;

pub const HUB: HubId = HubId("present");
pub const SHOWS: SubId = SubId("shows");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Present",
    icon: cl::icon::PRESENT,
    tone: Tone::Coral,
    subroutes: &[SubRoute {
        id: SHOWS,
        label: "Shows",
        blurb: "The shows — tap one to present it full-bleed on the big screen.",
    }],
};

pub struct PresentHub {
    fabric: Fabric,
    deck: Option<dpl_deck::DeckHost>,
    /// The last open failure, shown honestly on the picker.
    error: Option<String>,
}

impl PresentHub {
    pub fn new(fabric: Fabric) -> Self {
        Self {
            fabric,
            deck: None,
            error: None,
        }
    }

    /// Open a seed show by its folder id (also the `--present` harness
    /// path). Unknown ids degrade to staying on the picker.
    pub fn open_show(&mut self, id: &str) {
        let Some((_, dir)) = dpl_doc::list_episodes().into_iter().find(|(e, _)| e == id) else {
            return;
        };
        self.open_dir(&dir);
    }

    fn open_dir(&mut self, dir: &std::path::Path) {
        match dpl_deck::DeckHost::open(dir) {
            Ok(host) => {
                self.deck = Some(host);
                self.error = None;
                self.fabric
                    .publish_shell(ShellEvent::FullBleedRequest(true));
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn close_deck(&mut self) {
        self.deck = None;
        self.fabric
            .publish_shell(ShellEvent::FullBleedRequest(false));
    }

    fn picker(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new("The shows — tap one to present it")
                .small()
                .color(cl::ink::SUBTLE),
        );
        if let Some(e) = &self.error {
            ui.label(
                RichText::new(format!("could not open the show: {e}"))
                    .small()
                    .color(SPEC.tone.color()),
            );
        }
        ui.add_space(cl::space::S2);
        let mut open: Option<PathBuf> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (id, dir) in dpl_doc::list_episodes() {
                    let (title, count) = match dpl_doc::load_episode(&dir) {
                        Ok(ep) => {
                            let n = dpl_doc::episode_slides(&dir, &ep).len();
                            (ep.title, n)
                        }
                        Err(_) => (id.clone(), 0),
                    };
                    let resp = cl::card_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width().min(560.0));
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&title).strong().color(cl::ink::STRONG));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new("present ›").small().color(SPEC.tone.color()),
                                    );
                                    ui.label(
                                        RichText::new(format!("{count} slides"))
                                            .small()
                                            .color(cl::ink::SUBTLE),
                                    );
                                    ui.label(
                                        RichText::new(&id)
                                            .monospace()
                                            .small()
                                            .color(cl::ink::SUBTLE),
                                    );
                                },
                            );
                        });
                    });
                    if resp.response.interact(egui::Sense::click()).clicked() {
                        open = Some(dir);
                    }
                    ui.add_space(cl::space::S2);
                }
            });
        if let Some(dir) = open {
            self.open_dir(&dir);
        }
    }
}

impl Hub for PresentHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        let _ = route; // one subroute — SHOWS is the whole hub.
        if let Some(deck) = self.deck.as_mut() {
            if deck.ui(ui) == dpl_deck::DeckAction::Exit {
                self.close_deck();
            }
            return;
        }
        self.picker(ui);
    }
}
