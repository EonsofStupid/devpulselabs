//! The slide catalog — elements, blocks and components. Ported from the prior
//! app's `slide-catalog` domain, reauthored for egui with the paint
//! vocabulary.
//!
//! ## Where things live (`docs/design/design-system.md` §4.7)
//!
//! - **elements/** — atomic, insertable units (eyebrow, badge, stat). Local
//!   state from arguments only; an element never reads routes or global state.
//! - **blocks/** — a few elements composed into one reusable chunk. Earned by
//!   the promotion rule below, never authored speculatively.
//! - **components/** — full-slide compositions, layouts and charts alike. One
//!   folder per component, named by its contract id, holding its
//!   `component.json` and its code. Registered in [`registry::demos`] — ONE
//!   line each, and the Library gallery renders every registration live (the
//!   gallery gate).
//! - **chart/** — what the chart components share: a 960×540 logical space
//!   fitted into any stage rect, and the series tone cycle.
//! - **render/** — the ONE path that paints a slide document, everywhere.
//!
//! ## Doctrine (ported verbatim from the catalog readme)
//!
//! - **Presenter law**: every state is a rest state; nothing auto-advances.
//!   Animations are pure functions of the clock (sweep once, then rest).
//! - **Density** changes spacing/type only — never colors.
//! - **Tone** rides the slide; per-item data may override it. All color
//!   resolves through the generated pool (`dpl_coldlight::ramp`).
//! - **Promotion rule**: a pattern climbs a rung only after it recurs in
//!   ≥ 2 templates. An element earns `elements/`; a composite of elements
//!   earns `blocks/`. Nothing is promoted to fill a shelf.
//! - Sizing is `Fluid::of(stage)` — the same slide is correct in a Workbench
//!   preview pane and on the 75" kiosk.

pub mod blocks;
pub mod chart;
pub mod components;
pub mod contract;
pub mod elements;
pub mod frame;
pub mod motion;
pub mod registry;
pub mod render;

/// The two-notch authoring knob. Spacing and type slots only — never color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    #[default]
    Comfy,
    Compact,
}
