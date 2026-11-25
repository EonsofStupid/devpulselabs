//! Document refs resolved against the generated registries, and the honest
//! plate an unknown ref degrades to.

use crate::elements::media::Ambient;
use dpl_coldlight::{self as cl};
use egui::{Rect, Ui};

/// Resolve a document size ref ("display", "h3", …) through the fluid ramp.
pub(super) fn size_of(fluid: &cl::Fluid, name: &str) -> Option<f32> {
    Some(match name {
        "eyebrow" => fluid.fs_eyebrow(),
        "body-sm" => fluid.fs_body_sm(),
        "body" => fluid.fs_body(),
        "body-lg" => fluid.fs_body_lg(),
        "h4" => fluid.fs_h4(),
        "h3" => fluid.fs_h3(),
        "h2" => fluid.fs_h2(),
        "h1" => fluid.fs_h1(),
        "display" => fluid.fs_display(),
        "display-xl" => fluid.fs_display_xl(),
        _ => return None,
    })
}

/// Resolve a font ref against the pool — an UNREGISTERED family name would
/// panic at layout, so unknowns fall back to the default face.
pub(super) fn family_of(id: &str) -> egui::FontFamily {
    if cl::FONTS.iter().any(|f| f.id == id) {
        cl::font_family(id)
    } else {
        egui::FontFamily::Proportional
    }
}

pub(super) fn ambient_of(name: &str) -> Option<Ambient> {
    match name {
        "pulse" => Some(Ambient::Pulse),
        "drift" => Some(Ambient::Drift),
        "rings" => Some(Ambient::Rings),
        _ => None,
    }
}

pub(super) fn plate(ui: &Ui, rect: Rect, message: &str) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0, cl::surface::RAISED);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        message,
        egui::FontId::monospace(12.0),
        cl::ink::SUBTLE,
    );
}
