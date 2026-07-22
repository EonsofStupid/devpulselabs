//! Library — the gallery GATE.
//!
//! The anti-recurrence law, rendered: the old app generated 18 tone slots and
//! used three, because nothing ever surfaced the rest. This hub renders
//! **everything that exists**: the entire generated color pool on one wall,
//! and every registered slide template LIVE (real instances, real state,
//! real animation) on the other. If it exists, it is on a wall here.

use dpl_coldlight::{self as cl, Tone};
use dpl_hub::{Hub, HubId, HubSpec, Route, SubId, SubRoute};
use dpl_slides::SlideTemplate;
use egui::RichText;

pub const HUB: HubId = HubId("library");
pub const COMPONENTS: SubId = SubId("components");
pub const TEMPLATES: SubId = SubId("templates");
pub const MOODS: SubId = SubId("moods");

static SPEC: HubSpec = HubSpec {
    id: HUB,
    label: "Library",
    icon: cl::icon::LIBRARY,
    tone: Tone::Violet,
    subroutes: &[
        SubRoute {
            id: COMPONENTS,
            label: "Color pool",
            blurb: "The whole generated pool — every hue, every step. The visual regression floor.",
        },
        SubRoute {
            id: TEMPLATES,
            label: "Slide templates",
            blurb: "Every registered template, live — blocks, templates, infographics, animating.",
        },
        SubRoute {
            id: MOODS,
            label: "Backdrop moods",
            blurb: "Every authored mood from tokens/moods.json — the backdrop's random pulls.",
        },
    ],
};

pub struct LibraryHub {
    /// One live instance per registered template — the gallery gate's stock.
    demos: Vec<Box<dyn SlideTemplate>>,
    /// Solo view: one template rendered full-stage (tap a card header to
    /// enter, tap again to return to the wall). Also the harness's
    /// per-template proof hook (`--focus <template-id>`).
    focus: Option<String>,
}

impl Default for LibraryHub {
    fn default() -> Self {
        Self {
            demos: dpl_slides::demos(),
            focus: None,
        }
    }
}

impl LibraryHub {
    /// Focus one template by id (unknown ids degrade to the wall).
    pub fn set_focus(&mut self, id: impl Into<String>) {
        self.focus = Some(id.into());
    }
}

impl Hub for LibraryHub {
    fn spec(&self) -> &'static HubSpec {
        &SPEC
    }

    fn ui(&mut self, ui: &mut egui::Ui, route: Route) {
        if route.sub == TEMPLATES {
            self.templates_gallery(ui);
        } else if route.sub == MOODS {
            moods_wall(ui);
        } else {
            // Unknown subs degrade to the pool wall, never crash.
            pool_wall(ui);
        }
    }
}

impl LibraryHub {
    /// The live template gallery: every `registry::demos()` entry on a 16:9
    /// stage over the WELL, labeled with its id and class. Interactions and
    /// sweeps run for real — this IS the app surface, not a picture of it.
    fn templates_gallery(&mut self, ui: &mut egui::Ui) {
        // Solo view: one template on the full stage.
        if let Some(id) = self.focus.clone() {
            if let Some(demo) = self.demos.iter_mut().find(|d| d.spec().id.0 == id) {
                let spec = demo.spec();
                let back = ui.horizontal(|ui| {
                    let back = ui
                        .label(
                            RichText::new(format!("« {}", spec.label))
                                .strong()
                                .color(cl::ink::STRONG),
                        )
                        .interact(egui::Sense::click());
                    ui.label(
                        RichText::new(spec.class.label())
                            .monospace()
                            .small()
                            .color(SPEC.tone.color()),
                    );
                    ui.label(
                        RichText::new(spec.id.0)
                            .monospace()
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                    let replay = ui
                        .label(
                            RichText::new("replay")
                                .monospace()
                                .small()
                                .color(SPEC.tone.color()),
                        )
                        .interact(egui::Sense::click());
                    (back, replay)
                });
                let stage = ui.available_rect_before_wrap().shrink(cl::space::S2);
                ui.painter()
                    .rect_filled(stage, cl::RADIUS, cl::surface::WELL);
                ui.painter().rect_stroke(
                    stage,
                    cl::RADIUS,
                    egui::Stroke::new(1.0, cl::surface::HAIRLINE),
                    egui::StrokeKind::Inside,
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(stage.shrink(2.0)), |ui| {
                    demo.ui(ui);
                });
                let (back, replay) = back.inner;
                if replay.clicked() {
                    demo.restart();
                }
                if back.clicked() {
                    self.focus = None;
                }
                return;
            }
            // Unknown focus id: degrade to the wall, never crash.
            self.focus = None;
        }

        let fluid = cl::Fluid::of(ui.max_rect());

        ui.add_space(cl::space::S2);
        ui.horizontal(|ui| {
            ui.label(RichText::new(SPEC.icon).size(22.0).color(SPEC.tone.color()));
            ui.label(
                RichText::new("The catalog")
                    .heading()
                    .color(cl::ink::STRONG),
            );
        });
        ui.label(
            RichText::new(format!(
                "{} templates registered — every one rendered live below. \
                 A template that exists but doesn't show here cannot exist.",
                self.demos.len(),
            ))
            .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S4);

        let mut focus_request: Option<String> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            let avail_w = ui.available_width();
            let stage_w = avail_w - cl::space::S4 * 2.0;
            let stage_h = stage_w * 9.0 / 16.0;
            for demo in self.demos.iter_mut() {
                let spec = demo.spec();
                ui.horizontal(|ui| {
                    let title = ui
                        .label(
                            RichText::new(spec.label)
                                .size(fluid.fs_h4())
                                .strong()
                                .color(cl::ink::STRONG),
                        )
                        .interact(egui::Sense::click());
                    if title.clicked() {
                        focus_request = Some(spec.id.0.to_string());
                        demo.restart();
                    }
                    ui.label(
                        RichText::new(spec.class.label())
                            .monospace()
                            .small()
                            .color(SPEC.tone.color()),
                    );
                    ui.label(
                        RichText::new(spec.id.0)
                            .monospace()
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                });
                ui.label(RichText::new(spec.blurb).small().color(cl::ink::MUTED));
                ui.add_space(cl::space::S1);

                // The stage: a recessed 16:9 WELL plate the slide renders on —
                // the same surface contract the deck runtime will provide.
                let (stage, _) =
                    ui.allocate_exact_size(egui::vec2(stage_w, stage_h), egui::Sense::hover());
                ui.painter()
                    .rect_filled(stage, cl::RADIUS, cl::surface::WELL);
                ui.painter().rect_stroke(
                    stage,
                    cl::RADIUS,
                    egui::Stroke::new(1.0, cl::surface::HAIRLINE),
                    egui::StrokeKind::Inside,
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(stage.shrink(2.0)), |ui| {
                    demo.ui(ui);
                });
                ui.add_space(cl::space::S5);
            }
        });
        if focus_request.is_some() {
            self.focus = focus_request;
        }
    }
}

/// The color pool wall (unchanged from P1): 53 hues × 13 steps, all rendered.
fn pool_wall(ui: &mut egui::Ui) {
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

/// The moods wall — every AUTHORED backdrop mood renders as a live gradient
/// swatch (the gallery gate: if it's in tokens/moods.json, it's on this
/// wall). Card = the mood's 135° gradient under its own scrim, name, stop
/// count, and accent dot.
fn moods_wall(ui: &mut egui::Ui) {
    ui.label(
        RichText::new(format!(
            "BACKDROP MOODS — {} authored · every click on a backdrop tile pulls one at random",
            cl::MOOD_COUNT
        ))
        .small()
        .color(cl::ink::SUBTLE),
    );
    ui.add_space(cl::space::S2);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let card = egui::vec2(236.0, 128.0);
            ui.horizontal_wrapped(|ui| {
                for mood in cl::MOODS.iter() {
                    let (rect, _) = ui.allocate_exact_size(card, egui::Sense::hover());
                    let painter = ui.painter();
                    // The treatment exactly as the backdrop composes it:
                    // gradient, then the mood's tinted scrim at half alpha so
                    // the swatch stays legible on the wall.
                    cl::mood_gradient(painter, rect, mood);
                    let s = mood.scrim_color();
                    painter.rect_filled(
                        rect,
                        cl::RADIUS_SM,
                        egui::Color32::from_rgba_unmultiplied(s.r(), s.g(), s.b(), s.a() / 2),
                    );
                    painter.rect(
                        rect,
                        cl::RADIUS_SM,
                        egui::Color32::TRANSPARENT,
                        egui::Stroke::new(1.0, cl::surface::HAIRLINE),
                        egui::StrokeKind::Inside,
                    );
                    painter.circle_filled(
                        egui::pos2(rect.min.x + 14.0, rect.max.y - 16.0),
                        5.0,
                        mood.accent_color(),
                    );
                    painter.text(
                        egui::pos2(rect.min.x + 26.0, rect.max.y - 16.0),
                        egui::Align2::LEFT_CENTER,
                        mood.name,
                        egui::FontId::proportional(13.0),
                        cl::ink::STRONG,
                    );
                    painter.text(
                        egui::pos2(rect.max.x - 10.0, rect.max.y - 16.0),
                        egui::Align2::RIGHT_CENTER,
                        format!("{} stops", mood.stops.len()),
                        egui::FontId::monospace(10.5),
                        cl::ink::BASE,
                    );
                }
            });
        });
}
