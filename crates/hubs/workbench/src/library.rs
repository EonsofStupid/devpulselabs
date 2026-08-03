//! The Workbench LIBRARY — ALL the raw ingredients on one surface
//! (doctrine §1.1): seed slides by class, every registered template LIVE, the
//! ingredient ladder (elements → blocks), and the vocabulary walls (color pool
//! · presets · fonts · moods).
//!
//! The anti-recurrence law rode in with the walls: this surface renders
//! **everything that exists** — a template or token that never shows here
//! cannot exist.
//!
//! The chip row spells the ladder coarse-to-fine (DS3), and every word on it
//! is the OPERATOR'S word, not the code's. The two vocabularies are split
//! deliberately — [`LibSection::slug`] is the machine id (harness argument,
//! pilot key), [`LibSection::label`] is what gets drawn:
//!
//! | drawn | code |
//! |---|---|
//! | Starters | `seeds/` |
//! | Layouts | `templates/` + `infographics/` |
//! | Combos | `blocks/` |
//! | Pieces | `elements/` |
//! | Themes | `tokens/frame-patches.json` |
//! | Styles | `presets.json` |
//!
//! Same split as "card" for `dpl_doc::Slide`. The stack is invisible — a
//! layman never has to learn that a small label is called an eyebrow
//! (docs/plans/authoring-architecture.md, the UI law).

use super::SPEC;
use dpl_coldlight::{self as cl, Tone};
use dpl_slides::{SlideTemplate, TemplateClass};
use egui::RichText;
use egui_pilot_ai::{NodeMetadata, PilotId, PilotKey, UiPilotExt};

/// The Library's sections — the chip row's vocabulary. SEEDS leads (the
/// ingredient the editor tailors first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LibSection {
    #[default]
    Seeds,
    Templates,
    Blocks,
    Elements,
    Themes,
    Colors,
    Presets,
    Fonts,
    Icons,
    Moods,
}

impl LibSection {
    /// Ladder order, coarse to fine — Templates → Blocks → Elements — then the
    /// vocabulary walls. Seeds still lead (the ingredient the editor tailors
    /// first).
    pub(crate) const ALL: [LibSection; 10] = [
        LibSection::Seeds,
        LibSection::Templates,
        LibSection::Blocks,
        LibSection::Elements,
        LibSection::Themes,
        LibSection::Colors,
        LibSection::Presets,
        LibSection::Fonts,
        LibSection::Icons,
        LibSection::Moods,
    ];

    /// The STABLE machine id — the `--lib-section` argument and the pilot node
    /// key. Engineering vocabulary lives here and is never drawn on screen.
    /// Renaming one of these breaks proof scripts and recorded pilot runs, so
    /// they change only with a reason.
    pub(crate) fn slug(self) -> &'static str {
        match self {
            LibSection::Seeds => "seeds",
            LibSection::Templates => "templates",
            LibSection::Blocks => "blocks",
            LibSection::Elements => "elements",
            LibSection::Themes => "themes",
            LibSection::Colors => "colors",
            LibSection::Presets => "presets",
            LibSection::Fonts => "fonts",
            LibSection::Icons => "icons",
            LibSection::Moods => "moods",
        }
    }

    /// What the OPERATOR reads on the chip. Plain words only — the stack is
    /// invisible (docs/plans/authoring-architecture.md, the UI law). "Combos"
    /// and "Pieces" are what the code files under `blocks/` and `elements/`;
    /// the operator has no reason to learn either word.
    pub(crate) fn label(self) -> &'static str {
        match self {
            LibSection::Seeds => "Starters",
            LibSection::Templates => "Layouts",
            LibSection::Blocks => "Combos",
            LibSection::Elements => "Pieces",
            LibSection::Themes => "Themes",
            LibSection::Colors => "Colors",
            LibSection::Presets => "Styles",
            LibSection::Fonts => "Type",
            LibSection::Icons => "Icons",
            LibSection::Moods => "Moods",
        }
    }

    /// The tooltip law — each wall explains itself, in the same plain words.
    fn tip(self) -> &'static str {
        match self {
            LibSection::Seeds => "Whole cards, ready to go — copy one and make it yours",
            LibSection::Templates => {
                "Every card design you have, running live — tap one to see it big"
            }
            LibSection::Blocks => "A few pieces that keep turning up together",
            LibSection::Elements => "The smallest things everything else is built from",
            LibSection::Themes => "How a card is dressed — pick one, stack more on top",
            LibSection::Colors => "Every color the app draws from",
            LibSection::Presets => "Saved combinations — one tap dresses the whole card",
            LibSection::Fonts => "Every typeface you have, each shown in itself",
            LibSection::Icons => "Every icon the app draws — your Lucide set, in every dressing",
            LibSection::Moods => "The colors the background drifts through",
        }
    }
}

/// Template class order for gallery section headers (matches seed taxonomy).
const TEMPLATE_CLASSES: [TemplateClass; 4] = [
    TemplateClass::Static,
    TemplateClass::Split,
    TemplateClass::Animation,
    TemplateClass::Interactive,
];

/// The LOOKS wall — every generated frame patch rendered LIVE through the
/// compositor (moved here from the WiredFRONT composer under the
/// two-libraries law: ColdLight ingredients belong to WORKBENCH).
///
/// Named `frame_patches_wall` because `tokens/frame-patches.json` is what it reads; the
/// operator only ever sees the word "Themes".
pub(crate) fn frame_patches_wall(ui: &mut egui::Ui) {
    ui.label(
        RichText::new(format!(
            "{} ways to dress a card. The first two stand on their own; the rest \
             stack on top of them — which is how they are shown here.",
            cl::FRAME_PATCHES.len()
        ))
        .small()
        .color(cl::ink::SUBTLE),
    );
    ui.add_space(cl::space::S2);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for def in cl::FRAME_PATCHES {
                    patch_swatch(ui, def);
                    ui.add_space(cl::space::S2);
                }
            });
        });
}

/// One patch, rendered LIVE through the compositor — base patches stand
/// alone; overlay patches ride the neubrutalist base (their real use).
fn patch_swatch(ui: &mut egui::Ui, def: &cl::FramePatch) {
    let (base, stack): (&str, &[&str]) = if def.role == "base" {
        (def.id, &[])
    } else {
        ("neubrutalist", std::slice::from_ref(&def.id))
    };
    let style = cl::compose(Tone::Magenta, base, stack, cl::WidgetState::Inactive);
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
            egui::CornerRadius::same(style.corner_radius),
            sh.color,
        );
    }
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(style.corner_radius),
        style.fill,
    );
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(style.corner_radius),
        style.stroke,
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        def.label,
        egui::FontId::proportional(13.0),
        style.override_text_color,
    );
    cl::tip(resp, def.blurb);
}

/// The section chip row — tap a chip, that wall renders below.
pub(crate) fn chip_row(ui: &mut egui::Ui, current: &mut LibSection) {
    ui.horizontal(|ui| {
        for section in LibSection::ALL {
            let active = *current == section;
            let chip = cl::tip(
                ui.label(RichText::new(section.label()).color(if active {
                    SPEC.tone.color()
                } else {
                    cl::ink::MUTED
                }))
                .interact(egui::Sense::click()),
                section.tip(),
            );
            ui.pilot_track(
                &chip,
                // Key on the SLUG so recorded runs and proof scripts survive a
                // wording change; the human label rides alongside it.
                NodeMetadata::new(
                    PilotId::new("workbench.library.section").child(&PilotKey::new(section.slug())),
                    egui_pilot_ai::protocol::Role::Tab,
                    section.label(),
                )
                .value(if active { "active" } else { "available" })
                .selected(active),
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
                ui.pilot_track(
                    &back,
                    NodeMetadata::new(
                        PilotId::new("workbench.library.template").child(&PilotKey::new("back")),
                        egui_pilot_ai::protocol::Role::Button,
                        "back to the wall",
                    ),
                );
                ui.pilot_track(
                    &replay,
                    NodeMetadata::new(
                        PilotId::new("workbench.library.template").child(&PilotKey::new("replay")),
                        egui_pilot_ai::protocol::Role::Button,
                        "replay",
                    ),
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
            "{} templates registered — sectioned by class. \
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
        // Index demos once so each class section can borrow mutably in turn.
        let mut by_class: [Vec<usize>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for (i, demo) in demos.iter().enumerate() {
            let idx = match demo.spec().class {
                TemplateClass::Static => 0,
                TemplateClass::Split => 1,
                TemplateClass::Animation => 2,
                TemplateClass::Interactive => 3,
            };
            by_class[idx].push(i);
        }
        for (class, indices) in TEMPLATE_CLASSES.into_iter().zip(by_class.iter()) {
            ui.add_space(cl::space::S2);
            ui.label(
                RichText::new(format!("{} —", class.label().to_uppercase()))
                    .small()
                    .strong()
                    .color(SPEC.tone.color()),
            );
            ui.add_space(cl::space::S2);
            if indices.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "no {} templates registered yet — the class stays honest and empty",
                        class.label()
                    ))
                    .small()
                    .color(cl::ink::MUTED),
                );
                continue;
            }
            for &i in indices {
                let demo = &mut demos[i];
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
                    ui.pilot_track(
                        &title,
                        NodeMetadata::new(
                            PilotId::new("workbench.library.template")
                                .child(&PilotKey::new(spec.id.0)),
                            egui_pilot_ai::protocol::Role::Button,
                            spec.label,
                        ),
                    );
                    if title.clicked() {
                        focus_request = Some(spec.id.0.to_string());
                        demo.restart();
                    }
                    ui.label(
                        RichText::new(spec.id.0)
                            .monospace()
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                });
                ui.label(RichText::new(spec.blurb).small().color(cl::ink::MUTED));
                ui.add_space(cl::space::S1);

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
        }
    });
    if focus_request.is_some() {
        *focus = focus_request;
    }
}

/// The ELEMENTS wall — every paint primitive in `dpl_slides::elements` live on
/// a plate. An element that exists but never shows here cannot exist.
pub(crate) fn elements_wall(ui: &mut egui::Ui) {
    use dpl_slides::elements::{elements, ElementCategory};
    ui.label(
        RichText::new(
            "The smallest things everything else is built from, grouped by what \
             they do. Each one is drawn here for real, not a picture of it.",
        )
        .small()
        .color(cl::ink::MUTED),
    );
    ui.add_space(cl::space::S2);
    // The census IS the wall (Census law): every registered element shows, and
    // nothing is named by hand here — a new element appears the day it registers.
    let census = elements();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for category in ElementCategory::ALL {
                ui.add_space(cl::space::S2);
                ui.label(
                    RichText::new(format!("{} —", category.label().to_uppercase()))
                        .strong()
                        .color(cl::ink::STRONG),
                );
                let members: Vec<_> = census
                    .iter()
                    .filter(|element| element.spec().category == category)
                    .collect();
                if members.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "nothing under {} yet — it stays empty until something belongs here",
                            category.label().to_lowercase()
                        ))
                        .small()
                        .color(cl::ink::MUTED),
                    );
                    continue;
                }
                for element in members {
                    let spec = element.spec();
                    block_plate(ui, spec.label, spec.blurb, |ui, _fluid| element.paint(ui));
                }
            }
        });
}

/// The BLOCKS wall — the chunk tier: a few elements combined into one reusable
/// piece. Census-driven exactly like the elements wall, so a promoted block
/// appears the day it registers.
///
/// Empty today, and it says so plainly rather than pretending otherwise — the
/// same honesty the element shelves already use for their unpopulated
/// categories.
pub(crate) fn blocks_wall(ui: &mut egui::Ui) {
    use dpl_slides::blocks::blocks;
    ui.label(
        RichText::new(
            "A few pieces that keep turning up together, saved as one thing you \
             can drop in. Each one is drawn here for real, not a picture of it.",
        )
        .small()
        .color(cl::ink::MUTED),
    );
    ui.add_space(cl::space::S2);
    let census = blocks();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if census.is_empty() {
                ui.add_space(cl::space::S2);
                ui.label(
                    RichText::new("Nothing here yet — and that is on purpose.")
                        .strong()
                        .color(cl::ink::STRONG),
                );
                ui.add_space(cl::space::S1);
                ui.label(
                    RichText::new(
                        "Something only shows up here once you have used the same handful of \
                         pieces together on two or more cards. Until that happens this stays \
                         empty, rather than filling up with things nothing actually uses.",
                    )
                    .small()
                    .color(cl::ink::MUTED),
                );
                return;
            }
            for block in &census {
                let spec = block.spec();
                block_plate(ui, spec.label, spec.blurb, |ui, _fluid| block.paint(ui));
            }
        });
}

fn block_plate(
    ui: &mut egui::Ui,
    id: &str,
    blurb: &str,
    paint: impl FnOnce(&mut egui::Ui, &cl::Fluid),
) {
    let fluid = cl::Fluid::of(ui.max_rect());
    ui.horizontal(|ui| {
        ui.label(RichText::new(id).strong().color(cl::ink::STRONG));
        ui.label(RichText::new(blurb).small().color(cl::ink::MUTED));
    });
    ui.add_space(cl::space::S1);
    let (plate, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().min(480.0), 140.0),
        egui::Sense::hover(),
    );
    ui.painter()
        .rect_filled(plate, cl::RADIUS, cl::surface::RAISED);
    ui.painter().rect_stroke(
        plate,
        cl::RADIUS,
        egui::Stroke::new(1.0, cl::surface::HAIRLINE),
        egui::StrokeKind::Inside,
    );
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(plate.shrink(cl::space::S3))
            .layout(egui::Layout::top_down(egui::Align::Min)),
        |ui| paint(ui, &fluid),
    );
    ui.add_space(cl::space::S4);
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
/// source) renders as itself: a theme card paints ITS mood blend and sets
/// ITS label in ITS font; motion presets list their entrance effect. The
/// gallery gate for bundles-of-refs — W4's trays offer exactly these.
pub(crate) fn presets_wall(ui: &mut egui::Ui) {
    ui.label(
        RichText::new(format!(
            "PRESETS — {} themes · {} motions · named bundles of refs, editable in tokens/presets.json",
            cl::THEME_PRESET_COUNT,
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
                for p in cl::THEME_PRESETS.iter() {
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

/// The icons wall — every registry icon (tokens/icons.json over the
/// vendored Lucide pool, ADR-015) in three renditions: default ink, tone
/// accent, and the brutal cut (the icon's own mesh as a hard offset
/// shadow). The gallery gate for the 8th token source.
pub(crate) fn icons_wall(ui: &mut egui::Ui) {
    ui.label(
        RichText::new(format!(
            "ICON REGISTRY — {} semantic ids over the vendored Lucide 1.27.0 pool (ISC, assets/lucide) · geometry baked at build, styled at paint",
            cl::ICON_COUNT
        ))
        .small()
        .color(cl::ink::SUBTLE),
    );
    ui.add_space(cl::space::S2);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for def in cl::ICONS.iter() {
                    let resp = cl::card_frame().show(ui, |ui| {
                        ui.set_width(168.0);
                        ui.horizontal(|ui| {
                            let _ = cl::icon_label(ui, def.id, 28.0, cl::ink::STRONG);
                            let _ = cl::icon_label(ui, def.id, 28.0, SPEC.tone.color());
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                            cl::draw_icon_brutal(
                                ui.painter(),
                                def.id,
                                rect,
                                SPEC.tone.color(),
                                egui::Color32::BLACK,
                            );
                        });
                        ui.label(RichText::new(def.label).small().color(cl::ink::STRONG));
                        ui.label(
                            RichText::new(format!("{} · {}", def.id, def.lucide))
                                .monospace()
                                .small()
                                .color(cl::ink::SUBTLE),
                        );
                    });
                    let _ = cl::tip(resp.response, def.blurb);
                    ui.add_space(cl::space::S2);
                }
            });
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// The chip row is a census too: every section appears exactly once under
    /// both names, and every one carries tooltip copy (the tooltip law).
    #[test]
    fn the_chip_row_is_the_census() {
        let slugs: HashSet<&str> = LibSection::ALL.iter().map(|s| s.slug()).collect();
        assert_eq!(
            slugs.len(),
            LibSection::ALL.len(),
            "slugs must be unique — a duplicate makes --lib-section ambiguous"
        );
        let labels: HashSet<&str> = LibSection::ALL.iter().map(|s| s.label()).collect();
        assert_eq!(
            labels.len(),
            LibSection::ALL.len(),
            "labels must be unique — two chips reading the same word is a bug"
        );
        for section in LibSection::ALL {
            assert!(!section.slug().is_empty());
            assert!(!section.label().is_empty());
            assert!(!section.tip().is_empty(), "every chip explains itself");
        }
    }

    /// DS3: the ladder's two lower rungs are SEPARATE shelves reading SEPARATE
    /// censuses. Before DS3 one variant showed the other tier's contents.
    #[test]
    fn elements_and_blocks_are_distinct_shelves() {
        assert_ne!(LibSection::Elements, LibSection::Blocks);
        assert!(
            !dpl_slides::elements::elements().is_empty(),
            "the element tier is populated"
        );
        assert!(
            dpl_slides::blocks::blocks().is_empty(),
            "the chunk tier is empty until a composite earns promotion — if this \
             fails, register the block on the Blocks shelf and update this test"
        );
    }

    /// The operator never reads engineering words off a chip (the UI law:
    /// "the stack is invisible"). The words below are what the CODE calls
    /// these tiers — folders, types, token files. None may reach a label.
    #[test]
    fn no_chip_speaks_engineering() {
        const CODE_WORDS: [&str; 6] = [
            "blocks",
            "elements",
            "frame-patches",
            "presets",
            "seeds",
            "tokens",
        ];
        for section in LibSection::ALL {
            let label = section.label().to_lowercase();
            for word in CODE_WORDS {
                assert_ne!(
                    label,
                    word,
                    "chip `{}` reads as the code's own word for it — translate it",
                    section.slug()
                );
            }
        }
    }

    /// Every piece on the Pieces shelf names itself the way the operator would
    /// name it. `eyebrow` is the code's word (and a typography term); nobody
    /// outside that trade knows it, so it may not be what the shelf says.
    #[test]
    fn no_piece_speaks_engineering() {
        const CODE_WORDS: [&str; 6] = ["eyebrow", "stat", "badge", "bar", "divider", "media"];
        for element in dpl_slides::elements::elements() {
            let spec = element.spec();
            let label = spec.label.to_lowercase();
            for word in CODE_WORDS {
                assert_ne!(
                    label, word,
                    "`{}` shows its code name on the shelf — give it a plain one",
                    spec.id
                );
            }
            assert!(
                !spec.blurb.is_empty(),
                "`{}` needs a line saying what it is",
                spec.id
            );
        }
    }
}
