//! The LIBRARY — the generated-ingredient shelf both editors read.
//!
//! The law (operator-ratified): presets are never one-offs. Every entry on
//! this wall comes from a GENERATED ColdLight registry (`tokens/*.json` →
//! `build.rs` → the statics below) — traits, look presets, motion presets,
//! fonts, moods. Authoring the Nth ingredient is editing JSON and
//! regenerating; this surface picks it up with zero code change. Nothing
//! here is hand-listed.

use dpl_coldlight::{self as cl, Tone};
use egui::{CornerRadius, Rect, RichText, Sense};

/// The Library surface — what the WiredFRONT hub renders for its Library
/// subroute (and Workbench will read the same shelf).
pub fn library_ui(ui: &mut egui::Ui) {
    // The shelf sits on its own recessed plate — the mood backdrop never
    // fights the reading surface.
    let full = ui.max_rect();
    ui.painter()
        .rect_filled(full, CornerRadius::same(cl::RADIUS), cl::surface::WELL);
    let mut inner = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(full.shrink(cl::space::S3))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let ui = &mut inner;
    ui.label(
        RichText::new("The ingredient library")
            .heading()
            .color(cl::ink::STRONG),
    );
    ui.label(
        RichText::new(
            "Everything both editors compose from — generated from the authored tokens, \
             never a one-off. Author a token, regenerate, and it appears here.",
        )
        .small()
        .color(cl::ink::SUBTLE),
    );
    ui.add_space(cl::space::S3);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            section(
                ui,
                "TRAITS",
                "The composable treatments every surface is dressed in.",
            );
            ui.horizontal_wrapped(|ui| {
                for def in cl::TRAITS {
                    trait_swatch(ui, def);
                    ui.add_space(cl::space::S2);
                }
            });
            ui.add_space(cl::space::S4);

            section(
                ui,
                "LOOK PRESETS",
                "Named bundles of refs — font, moods, motion. Applying one swaps refs, never code.",
            );
            for p in cl::LOOK_PRESETS {
                cl::clay().show(ui, |ui| {
                    ui.set_width((ui.available_width() - cl::space::S4).min(640.0));
                    ui.label(RichText::new(p.label).strong().color(cl::ink::STRONG));
                    ui.label(RichText::new(p.blurb).small().color(cl::ink::SUBTLE));
                    ui.label(
                        RichText::new(format!(
                            "font {} · moods {} › {} · motion {}",
                            p.font, p.moods.0, p.moods.1, p.motion
                        ))
                        .small()
                        .color(cl::ink::MUTED),
                    );
                });
                ui.add_space(cl::space::S2);
            }
            ui.add_space(cl::space::S4);

            section(
                ui,
                "MOTION PRESETS",
                "Named entrances over the effect vocabulary — how things arrive.",
            );
            for p in cl::MOTION_PRESETS {
                cl::clay().show(ui, |ui| {
                    ui.set_width((ui.available_width() - cl::space::S4).min(640.0));
                    ui.label(RichText::new(p.label).strong().color(cl::ink::STRONG));
                    ui.label(RichText::new(p.blurb).small().color(cl::ink::SUBTLE));
                    ui.label(
                        RichText::new(format!("enter: {}", p.enter))
                            .small()
                            .color(cl::ink::MUTED),
                    );
                });
                ui.add_space(cl::space::S2);
            }
            ui.add_space(cl::space::S4);

            section(
                ui,
                "FONTS",
                "The vendored font pool — each face shown in itself.",
            );
            for f in cl::FONTS {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(f.label)
                            .font(egui::FontId::new(20.0, cl::font_family(f.id)))
                            .color(cl::ink::STRONG),
                    );
                    ui.label(
                        RichText::new(format!("{} · {}", f.class, f.blurb))
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                });
                ui.add_space(cl::space::S1);
            }
            ui.add_space(cl::space::S4);

            section(
                ui,
                "MOODS",
                "The authored backdrop gradients — multi-hue, never one color.",
            );
            for m in cl::MOODS {
                ui.horizontal(|ui| {
                    mood_strip(ui, &m);
                    ui.label(RichText::new(m.name).strong().color(cl::ink::BASE));
                });
                ui.add_space(cl::space::S1);
            }
            ui.add_space(cl::space::S4);
        });
}

/// A section header: the label + a layman one-liner.
fn section(ui: &mut egui::Ui, label: &str, hint: &str) {
    ui.label(RichText::new(label).small().strong().color(cl::ink::MUTED));
    ui.label(RichText::new(hint).small().color(cl::ink::SUBTLE));
    ui.add_space(cl::space::S2);
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
    let (slot, resp) = ui.allocate_exact_size(egui::vec2(136.0, 52.0), Sense::hover());
    let rect = Rect::from_min_size(slot.min, slot.size() - egui::vec2(4.0, 4.0));
    let painter = ui.painter();
    for shape in style.behind_shapes(rect) {
        painter.add(shape);
    }
    for sh in &style.shadows {
        painter.rect_filled(
            rect.translate(egui::vec2(sh.offset[0] as f32, sh.offset[1] as f32)),
            CornerRadius::same(style.radius),
            sh.color,
        );
    }
    painter.rect_filled(rect, CornerRadius::same(style.radius), style.fill);
    painter.rect_stroke(
        rect,
        CornerRadius::same(style.radius),
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

/// One mood as a sampled gradient strip — the same `Mood::at` the backdrop
/// paints through, so the strip IS the mood, not an approximation.
fn mood_strip(ui: &mut egui::Ui, m: &cl::Mood) {
    const SLICES: usize = 32;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(220.0, 22.0), Sense::hover());
    let painter = ui.painter();
    let w = rect.width() / SLICES as f32;
    for i in 0..SLICES {
        let t = i as f32 / (SLICES - 1) as f32;
        let slice = Rect::from_min_size(
            egui::pos2(rect.min.x + w * i as f32, rect.min.y),
            egui::vec2(w + 0.5, rect.height()),
        );
        painter.rect_filled(slice, CornerRadius::ZERO, m.at(t));
    }
    painter.rect_stroke(
        rect,
        CornerRadius::same(cl::RADIUS_SM),
        egui::Stroke::new(1.0, cl::surface::HAIRLINE),
        egui::StrokeKind::Inside,
    );
    painter.circle_filled(
        egui::pos2(rect.max.x - 12.0, rect.center().y),
        4.0,
        m.accent_color(),
    );
}
