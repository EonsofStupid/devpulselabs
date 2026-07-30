//! The SVG geometry law — the icon subset of SVG (Lucide's discipline:
//! stroke-based shapes in a viewBox) resolved to stroked triangle meshes.
//! Dep ceiling: ∅ (ADR-015) — parsing, flattening, and stroking are owned
//! here, and the buffers are pure `(f32, f32)` / `u32` so the crate stays
//! egui-free. Styling never lives here: geometry is data, callers bring
//! color — the dpl-vector law, extended to SVG.

mod flatten;
mod icon;
mod path;
mod stroke;
mod xml;

pub use icon::{tessellate_icon, Tess};

/// A refusal, in words: the engine names exactly what sat outside the
/// icon subset instead of guessing at intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgError(pub String);

impl std::fmt::Display for SvgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "svg: {}", self.0)
    }
}

impl std::error::Error for SvgError {}

pub(crate) fn err(msg: impl Into<String>) -> SvgError {
    SvgError(msg.into())
}
