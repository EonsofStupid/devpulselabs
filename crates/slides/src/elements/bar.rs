//! Bar — a horizontal ratio/progress meter: a rounded WELL track with a toned
//! fill to `frac` (0..1). The unit any slide reaches for to show a proportion.
//! Reused by `compare_vs` (strength meters) and `checklist` (progress),
//! satisfying the ≥2-template promotion rule.

use dpl_coldlight::{self as cl, Tone};
use egui::{Color32, Painter, Rect};

/// Paint `rect` as a pill track with a toned fill to `frac`. `alpha` fades the
/// whole meter (entrances/reveals ride it).
pub fn bar(painter: &Painter, rect: Rect, frac: f32, tone: Tone, alpha: f32) {
    let a = alpha.clamp(0.0, 1.0);
    let radius = rect.height() * 0.5;
    painter.rect_filled(rect, radius, cl::surface::WELL.gamma_multiply(a));
    let f = frac.clamp(0.0, 1.0);
    if f > 0.0 {
        let fill = Rect::from_min_size(rect.min, egui::vec2(rect.width() * f, rect.height()));
        let c = tone.color();
        painter.rect_filled(
            fill,
            radius,
            Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (235.0 * a) as u8),
        );
    }
}
