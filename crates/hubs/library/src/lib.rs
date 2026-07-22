//! Library — the gallery GATE.
//!
//! The anti-recurrence law, rendered: the old app generated 18 tone slots and
//! used three, because nothing ever surfaced the rest. This surface renders
//! **the entire generated pool** — every hue, every ladder step — so unused
//! variety is impossible to hide. If it exists, it is on this wall.

use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use egui::RichText;

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
        blurb: "The whole generated pool + every widget, live — the visual regression floor.",
    }],
};

#[derive(Default)]
pub struct LibraryHub;

impl Hub for LibraryHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, _route: Route) {
        // Container-based fluid: this surface sizes to ITS rect — the exact
        // mechanism slides will use on the 75" and in Workbench previews.
        let fluid = cl::Fluid::of(ui.max_rect());
        let swatch_h = fluid.sp_5();
        let swatch_w = fluid.sp_6();
        let gap = 2.0;

        ui.add_space(cl::space::S2);
        ui.horizontal(|ui| {
            ui.label(RichText::new(SPEC.icon).size(22.0).color(SPEC.tone.color()));
            ui.label(RichText::new("The pool").heading().color(cl::ink::STRONG));
        });
        ui.label(
            RichText::new(format!(
                "{} hues × {} steps = {} colors — generated from hues.json by cargo. \
                 Curved OKLCH ladders, gamut-clamped. If it exists, it renders here.",
                cl::HUE_COUNT,
                cl::LADDER_STEPS.len(),
                cl::HUE_COUNT * cl::LADDER_STEPS.len(),
            ))
            .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S4);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (group, ids) in cl::HUE_GROUPS.iter() {
                ui.label(
                    RichText::new(group.to_uppercase())
                        .small()
                        .color(cl::ink::SUBTLE),
                );
                ui.add_space(cl::space::S1);
                for id in ids.iter() {
                    let Some(ramp) = cl::ramp(id) else { continue };
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [110.0, swatch_h],
                            egui::Label::new(
                                RichText::new(*id).monospace().small().color(cl::ink::BASE),
                            ),
                        );
                        // The ladder: 13 crisp rects, no gaps softened, no
                        // feathered edges — sprite-grade.
                        for color in ramp.steps.iter() {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(swatch_w, swatch_h),
                                egui::Sense::hover(),
                            );
                            ui.painter().rect_filled(rect, 2, *color);
                            ui.add_space(gap - ui.spacing().item_spacing.x);
                        }
                    });
                }
                ui.add_space(cl::space::S3);
            }
        });
    }
}
