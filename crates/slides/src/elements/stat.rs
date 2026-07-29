//! Stat — a value + label pair, the unit BigNumber wall-sizes and a stats
//! grid repeats. Value color is the caller's (tone or ink); label is ink.

use dpl_coldlight::{self as cl, Fluid};
use egui::{Color32, RichText, Ui};

/// Vertical value-over-label stack, centered by the caller's layout.
pub fn stat(ui: &mut Ui, fluid: &Fluid, value: &str, label: &str, value_color: Color32) {
    ui.label(
        RichText::new(value)
            .strong()
            .size(fluid.fs_h2())
            .color(value_color),
    );
    ui.label(
        RichText::new(label)
            .size(fluid.fs_body())
            .color(cl::ink::BASE),
    );
}
