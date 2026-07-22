//! The stage — the ONE outer shell every slide renders inside (the
//! `SlideFrame` port). Safe-area padding from fluid tokens, and the `Fluid`
//! scale for everything inside. Paints nothing: slides sit directly on
//! whatever surface hosts them (the WELL in-app, black on the kiosk).

use dpl_coldlight::Fluid;
use egui::Rect;

/// A slide's working area: the padded rect and its fluid scale.
#[derive(Clone, Copy)]
pub struct Stage {
    pub rect: Rect,
    pub fluid: Fluid,
}

impl Stage {
    /// Stage for the rect a host gave this slide. Fluid is measured on the
    /// FULL rect (the stage is the sizing container), padding comes out of it.
    pub fn of(full: Rect) -> Self {
        let fluid = Fluid::of(full);
        let pad = fluid.sp_7();
        Self { rect: full.shrink(pad), fluid }
    }

    /// The bleed variant — media walls and full-bleed loops skip the safe area.
    pub fn bleed(full: Rect) -> Self {
        Self { rect: full, fluid: Fluid::of(full) }
    }
}
