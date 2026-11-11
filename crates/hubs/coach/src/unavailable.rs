use super::SPEC;
use dpl_hub::{Hub, HubSpec, Route};

/// Keeps the registered route available without foreign application capabilities.
#[derive(Default)]
pub struct CoachHub;

impl CoachHub {
    pub fn new() -> Self {
        Self
    }
}

impl Hub for CoachHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, _route: Route) {
        ui.heading("Coach");
        ui.label("External app lessons aren't available in this build.");
        ui.label("You can record a guide in ViziWiz.");
    }
}
