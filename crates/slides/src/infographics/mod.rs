//! Infographics — code-authored charts on the vector engine.
//!
//! Convention carried from the old `ChartFrame`: charts compose in a
//! **960×540 logical space** fitted (aspect-preserving, centered) into
//! whatever stage rect the host gives — the viewBox semantics, without SVG.
//! Strokes and type scale WITH the stage. Paint always resolves through the
//! pool via [`tone_cycle`]; multi-series charts take their palette order
//! from it, and per-datum tone overrides pin a specific tone.

pub mod bar_compare;
pub mod cost_meter_face;
pub mod donut_share;
pub mod gauge_dial;
pub mod line_trend;
pub mod probability_bars;
pub mod radial_progress;
pub mod stack_layers;
pub mod stage;
pub mod tone_cycle;
