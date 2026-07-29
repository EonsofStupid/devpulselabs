//! Eyebrow — THE mono/uppercase micro-label formula (ported; recurred 9×).
//!
//! egui has no letter-spacing, so tracking is authored the sprite way: thin
//! spaces (U+2009) interleaved between glyphs. Same airy uppercase read as
//! the CSS `tracking-[0.2em]` original, still one text galley.

use dpl_coldlight::{self as cl, Fluid};
use egui::{Color32, RichText, Ui};

/// Uppercase + thin-space tracking.
pub fn tracked(s: &str) -> String {
    let upper = s.to_uppercase();
    let mut out = String::with_capacity(upper.len() * 2);
    let mut first = true;
    for ch in upper.chars() {
        if !first {
            out.push('\u{2009}');
        }
        out.push(ch);
        first = false;
    }
    out
}

/// The micro-label. `color: None` = muted ink (the default read); pass a tone
/// color (or any pool color) to re-tone.
pub fn eyebrow(ui: &mut Ui, fluid: &Fluid, text: &str, color: Option<Color32>) {
    ui.label(
        RichText::new(tracked(text))
            .monospace()
            .size(fluid.fs_eyebrow())
            .color(color.unwrap_or(cl::ink::MUTED)),
    );
}
