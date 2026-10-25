//! Divider — a toned horizontal rule with a short bright accent segment on
//! the left, the rest a hairline. The quiet horizontal beat a slide uses to
//! separate a header from its body. Reused by `section_divider` and `quote`
//! (the ≥2-template promotion rule).

use dpl_coldlight as cl;
use egui::{Color32, Painter, Pos2, Stroke};

/// Draw a rule of `width` starting at (`left`, `y`). The first ~18% carries
/// the accent `c` (bright, 2px); the remainder is a 1px hairline. `alpha`
/// fades the whole rule (entrances ride it).
pub fn rule(painter: &Painter, left: f32, y: f32, width: f32, c: Color32, alpha: f32) {
    let a = alpha.clamp(0.0, 1.0);
    let split = left + width * 0.18;
    painter.line_segment(
        [Pos2::new(left, y), Pos2::new(split, y)],
        Stroke::new(
            2.0,
            Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (220.0 * a) as u8),
        ),
    );
    painter.line_segment(
        [Pos2::new(split, y), Pos2::new(left + width, y)],
        Stroke::new(1.0, cl::surface::HAIRLINE.gamma_multiply(a)),
    );
}
