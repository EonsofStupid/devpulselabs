//! The Workbench LIBRARY — ALL the raw ingredients on one surface
//! (doctrine §1.1): seed slides by class, every registered template LIVE,
//! and the vocabulary walls (color pool · presets · fonts · moods).
//!
//! The anti-recurrence law rode in with the walls: this surface renders
//! **everything that exists** — a template or token that never shows here
//! cannot exist. Blocks/elements sections join later; the chip row leaves
//! room.

use super::SPEC;
use dpl_coldlight::{self as cl, Tone};
use dpl_slides::SlideTemplate;
use egui::RichText;

/// The Library's sections — the chip row's vocabulary. SEEDS leads (the
/// ingredient the editor tailors first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LibSection {
    #[default]
    Seeds,
    Templates,
    Traits,
    Colors,
    Presets,
    Fonts,
    Moods,
}

impl LibSection {
    pub(crate) const ALL: [LibSection; 7] = [
        LibSection::Seeds,
        LibSection::Templates,
        LibSection::Traits,
        LibSection::Colors,
        LibSection::Presets,
        LibSection::Fonts,
        LibSection::Moods,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            LibSection::Seeds => "seeds",
            LibSection::Templates => "templates",
            LibSection::Traits => "traits",
            LibSection::Colors => "colors",
            LibSection::Presets => "presets",
            LibSection::Fonts => "fonts",
            LibSection::Moods => "moods",
        }
    }

    /// The tooltip law — each wall explains itself in plain words.
    fn tip(self) -> &'static str {
        match self {
            LibSection::Seeds => "Ready-made starter slides you can copy into a show",
            LibSection::Templates => "Every slide design, running live — tap one to see it big",
            LibSection::Traits => "The composable treatments every surface is dressed in",
            LibSection::Colors => "The whole color pool the app draws from",
            LibSection::Presets => "Saved style bundles — one tap applies a whole look",
            LibSection::Fonts => "Every typeface in the app, shown in its own face",
            LibSection::Moods => "The color moods the backdrop rolls on launch",
        }
    }
}

/// The traits wall — every generated trait rendered LIVE through the
/// compositor (moved here from the WiredFRONT composer under the
/// two-libraries law: ColdLight ingredients belong to WORKBENCH).
pub(crate) fn traits_wall(ui: &mut egui::Ui) {
    ui.label(
        RichText::new(format!(
            "TRAITS — {} generated treatments · base traits stand alone, stack traits ride the \
             neubrutalist base (their real use)",
            cl::TRAITS.len()
        ))
        .small()
        .color(cl::ink::SUBTLE),
    );
    ui.add_space(cl::space::S2);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for def in cl::TRAITS {
                    trait_swatch(ui, def);
                    ui.add_space(cl::space::S2);
                }
            });
        });
}

/// One trait, rendered LIVE through the compositor — base traits stand
/// alone; stack traits ride the neubrutalist base (their real use).
fn trait_swatch(ui: &mut egui::Ui, def: &cl::TraitDef) {
    let (base, stack): (&str, &[&str]) = if def.role == "base" {
        (def.id, &[])
    } else {
        ("neubrutalist", std::slice::from_ref(&def.id))
    };
    let style = cl::compose(Tone::Magenta, base, stack, cl::TraitState::Idle);
    // The swatch leaves room for the hard offset shadow on its right/bottom.
    let (slot, resp) = ui.allocate_exact_size(egui::vec2(136.0, 52.0), egui::Sense::hover());
    let rect = egui::Rect::from_min_size(slot.min, slot.size() - egui::vec2(4.0, 4.0));
    let painter = ui.painter();
    for shape in style.behind_shapes(rect) {
        painter.add(shape);
    }
    for sh in &style.shadows {
        painter.rect_filled(
            rect.translate(egui::vec2(sh.offset[0] as f32, sh.offset[1] as f32)),
            egui::CornerRadius::same(style.radius),
            sh.color,
        );
    }
    painter.rect_filled(rect, egui::CornerRadius::same(style.radius), style.fill);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(style.radius),
        style.edge,
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        def.label,
        egui::FontId::proportional(13.0),
        style.ink,
    );
    cl::tip(resp, def.blurb);
}

/// The section chip row — tap a chip, that wall renders below.
pub(crate) fn chip_row(ui: &mut egui::Ui, current: &mut LibSection) {
    ui.horizontal(|ui| {
        for section in LibSection::ALL {
            let active = *current == section;
            let chip = cl::tip(
                ui.label(RichText::new(section.label()).monospace().color(if active {
                    SPEC.tone.color()
                } else {
                    cl::ink::MUTED
                }))
                .interact(egui::Sense::click()),
                section.tip(),
            );
            if chip.clicked() {
                *current = section;
            }
        }
    });
    ui.add_space(cl::space::S2);
}

/// The live template gallery: every `registry::demos()` entry on a 16:9
/// stage over the WELL, labeled with its id and class. Interactions and
/// sweeps run for real — this IS the app surface, not a picture of it.
/// Tap a title to SOLO it (`focus`); replay restarts its clock.
pub(crate) fn templates_gallery(
    ui: &mut egui::Ui,
    demos: &mut [Box<dyn SlideTemplate>],
    focus: &mut Option<String>,
) {
    // Solo view: one template on the full stage.
    if let Some(id) = focus.clone() {
        if let Some(demo) = demos.iter_mut().find(|d| d.spec().id.0 == id) {
            let spec = demo.spec();
            let back = ui.horizontal(|ui| {
                let back = cl::tip(
                    ui.label(
                        RichText::new(format!("« {}", spec.label))
                            .strong()
                            .color(cl::ink::STRONG),
                    )
                    .interact(egui::Sense::click()),
                    "Back to the wall",
                );
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
                let replay = cl::tip(
                    ui.label(
                        RichText::new("replay")
                            .monospace()
                            .small()
                            .color(SPEC.tone.color()),
                    )
                    .interact(egui::Sense::click()),
                    "Play this slide's animation again from the start",
                );
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
                *focus = None;
            }
            return;
        }
        // Unknown focus id: degrade to the wall, never crash.
        *focus = None;
    }

    let fluid = cl::Fluid::of(ui.max_rect());

    ui.label(
        RichText::new(format!(
            "{} templates registered — every one rendered live below. \
             A template that exists but doesn't show here cannot exist.",
            demos.len(),
        ))
        .small()
        .color(cl::ink::MUTED),
    );
    ui.add_space(cl::space::S2);

    let mut focus_request: Option<String> = None;
    egui::ScrollArea::vertical().show(ui, |ui| {
        let avail_w = ui.available_width();
        let stage_w = avail_w - cl::space::S4 * 2.0;
        let stage_h = stage_w * 9.0 / 16.0;
        for demo in demos.iter_mut() {
            let spec = demo.spec();
            ui.horizontal(|ui| {
                let title = cl::tip(
                    ui.label(
                        RichText::new(spec.label)
                            .size(fluid.fs_h4())
                            .strong()
                            .color(cl::ink::STRONG),
                    )
                    .interact(egui::Sense::click()),
                    "Tap to see this one big",
                );
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
            // the same surface contract the deck runtime provides.
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
        *focus = focus_request;
    }
}

/// The color pool wall: every generated hue × step, all rendered.
pub(crate) fn pool_wall(ui: &mut egui::Ui) {
    let fluid = cl::Fluid::of(ui.max_rect());
    let swatch_h = fluid.sp_5();
    let swatch_w = fluid.sp_6();
    let gap = 2.0;

    ui.label(
        RichText::new(format!(
            "COLOR POOL — {} hues × {} steps = {} colors, generated from hues.json by cargo. \
             Curved OKLCH ladders, gamut-clamped. If it exists, it renders here.",
            cl::HUE_COUNT,
            cl::LADDER_STEPS.len(),
            cl::HUE_COUNT * cl::LADDER_STEPS.len(),
        ))
        .small()
        .color(cl::ink::SUBTLE),
    );
    ui.add_space(cl::space::S2);

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

/// The presets wall — every authored preset (tokens/presets.json, the 6th
/// source) renders as itself: a look card paints ITS mood blend and sets
/// ITS label in ITS font; motion presets list their entrance effect. The
/// gallery gate for bundles-of-refs — W4's trays offer exactly these.
pub(crate) fn presets_wall(ui: &mut egui::Ui) {
    ui.label(
        RichText::new(format!(
            "PRESETS — {} looks · {} motions · named bundles of refs, editable in tokens/presets.json",
            cl::LOOK_PRESET_COUNT,
            cl::MOTION_PRESET_COUNT
        ))
        .small()
        .color(cl::ink::SUBTLE),
    );
    ui.add_space(cl::space::S2);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.label(RichText::new("LOOK").small().color(cl::ink::SUBTLE));
            ui.add_space(cl::space::S1);
            let card = egui::vec2(320.0, 170.0);
            ui.horizontal_wrapped(|ui| {
                for p in cl::LOOK_PRESETS.iter() {
                    let (rect, _) = ui.allocate_exact_size(card, egui::Sense::hover());
                    let painter = ui.painter_at(rect);
                    let find = |n: &str| cl::MOODS.iter().find(|m| m.name == n);
                    match (find(p.moods.0), find(p.moods.1)) {
                        (Some(a), Some(b)) => {
                            // The preset's own blend, held at a readable mix.
                            cl::mood_blend(&painter, rect, a, b, 0.45, 0.0);
                            let s = a.scrim_color();
                            painter.rect_filled(
                                rect,
                                cl::RADIUS_SM,
                                egui::Color32::from_rgba_unmultiplied(
                                    s.r(),
                                    s.g(),
                                    s.b(),
                                    s.a() / 2,
                                ),
                            );
                        }
                        _ => {
                            painter.rect_filled(rect, cl::RADIUS_SM, cl::surface::RAISED);
                        }
                    }
                    painter.text(
                        egui::pos2(rect.min.x + 14.0, rect.min.y + 18.0),
                        egui::Align2::LEFT_CENTER,
                        p.label,
                        egui::FontId::new(22.0, cl::font_family(p.font)),
                        cl::ink::STRONG,
                    );
                    painter.text(
                        egui::pos2(rect.min.x + 14.0, rect.max.y - 40.0),
                        egui::Align2::LEFT_CENTER,
                        format!(
                            "{} × {} · {}{}",
                            p.moods.0,
                            p.moods.1,
                            p.motion,
                            p.ambience.map(|a| format!(" + {a}")).unwrap_or_default()
                        ),
                        egui::FontId::monospace(10.5),
                        cl::ink::BASE,
                    );
                    painter.text(
                        egui::pos2(rect.min.x + 14.0, rect.max.y - 18.0),
                        egui::Align2::LEFT_CENTER,
                        p.blurb,
                        egui::FontId::proportional(11.5),
                        cl::ink::MUTED,
                    );
                    painter.rect(
                        rect,
                        cl::RADIUS_SM,
                        egui::Color32::TRANSPARENT,
                        egui::Stroke::new(1.0, cl::surface::HAIRLINE),
                        egui::StrokeKind::Inside,
                    );
                }
            });

            ui.add_space(cl::space::S4);
            ui.label(RichText::new("MOTION").small().color(cl::ink::SUBTLE));
            ui.add_space(cl::space::S1);
            for p in cl::MOTION_PRESETS.iter() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(p.label).strong().color(cl::ink::STRONG));
                    ui.label(
                        RichText::new(p.enter)
                            .monospace()
                            .small()
                            .color(SPEC.tone.color()),
                    );
                    ui.label(RichText::new(p.blurb).small().color(cl::ink::MUTED));
                });
            }
        });
}

/// The fonts wall — every vendored pool font (tokens/fonts.json) renders IN
/// ITSELF: its label at display size, a sample line, class + id. The
/// gallery gate for the 5th token source — pick a face by eye, not by name.
pub(crate) fn fonts_wall(ui: &mut egui::Ui) {
    ui.label(
        RichText::new(format!(
            "FONT POOL — {} vendored faces (SIL OFL, assets/fonts) · chrome default is chosen here, in the design session",
            cl::FONT_COUNT
        ))
        .small()
        .color(cl::ink::SUBTLE),
    );
    ui.add_space(cl::space::S2);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for class in ["display", "retro", "ui", "mono"] {
                ui.label(
                    RichText::new(class.to_uppercase())
                        .small()
                        .color(cl::ink::SUBTLE),
                );
                ui.add_space(cl::space::S1);
                for def in cl::FONTS.iter().filter(|d| d.class == class) {
                    let resp = cl::card_frame().show(ui, |ui| {
                        ui.set_width(ui.available_width().min(720.0));
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(def.label)
                                    .font(egui::FontId::new(26.0, cl::font_family(def.id)))
                                    .color(cl::ink::STRONG),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(def.id)
                                            .monospace()
                                            .small()
                                            .color(cl::ink::SUBTLE),
                                    );
                                },
                            );
                        });
                        ui.label(
                            RichText::new(
                                "The quick brown fox jumps over 0123456789 — DevPULSE Labs",
                            )
                            .font(egui::FontId::new(17.0, cl::font_family(def.id)))
                            .color(cl::ink::BASE),
                        );
                        ui.label(RichText::new(def.blurb).small().color(cl::ink::MUTED));
                    });
                    let _ = resp;
                    ui.add_space(cl::space::S2);
                }
                ui.add_space(cl::space::S3);
            }
        });
}

/// The moods wall — every AUTHORED backdrop mood renders as a live gradient
/// swatch (the gallery gate: if it's in tokens/moods.json, it's on this
/// wall). Card = the mood's 135° gradient under its own scrim, name, stop
/// count, and accent dot.
pub(crate) fn moods_wall(ui: &mut egui::Ui) {
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
