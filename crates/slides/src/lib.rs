//! The slide catalog — blocks, templates, infographics. Ported from the prior
//! app's `slide-catalog` domain, reauthored for egui with the paint vocabulary.
//!
//! ## The tiers (carried law)
//!
//! - **blocks** — atomic, insertable units (eyebrow, badge, stat). Local state
//!   from arguments only; a block never reads routes or global state.
//! - **templates** — full-slide compositions. One module per template, owning
//!   its props/demo data and any interaction state. Registered in
//!   [`registry::demos`] — ONE line per template, and the Library gallery
//!   renders every registration live (the gallery gate).
//! - **infographics** — code-authored charts on the [`dpl_vector`] engine,
//!   960×540 logical space fitted into any stage rect.
//!
//! ## Doctrine (ported verbatim from the catalog readme)
//!
//! - **Presenter law**: every state is a rest state; nothing auto-advances.
//!   Animations are pure functions of the clock (sweep once, then rest).
//! - **Density** changes spacing/type only — never colors.
//! - **Tone** rides the slide; per-item data may override it. All color
//!   resolves through the generated pool (`dpl_coldlight::ramp`).
//! - **Promotion rule**: a pattern becomes a block only after it recurs in
//!   ≥ 2 templates.
//! - Sizing is `Fluid::of(stage)` — the same slide is correct in a Workbench
//!   preview pane and on the 75" kiosk.

pub mod blocks;
pub mod frame;
pub mod infographics;
pub mod motion;
pub mod registry;
pub mod templates;

pub use registry::{demos, SlideTemplate, TemplateClass, TemplateId, TemplateSpec};

/// The two-notch authoring knob. Spacing and type slots only — never color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Density {
    #[default]
    Comfy,
    Compact,
}
