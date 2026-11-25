//! Components — one folder per component, named by its contract id.
//!
//! The pattern, the same for all of them (`docs/design/design-system.md` §4.7):
//!
//! - `component.json` — the contract: slots, props, variant axes, examples
//!   and layout. Always present.
//! - `mod.rs` — the component's code. Always present.
//!
//! This file lists the folders and nothing else. A component registers with
//! ONE line in [`crate::registry::demos`].

pub mod agenda;
pub mod bar_compare;
pub mod callout;
pub mod checklist;
pub mod choice;
pub mod code;
pub mod compare;
pub mod cost_meter;
pub mod cost_meter_face;
pub mod donut_share;
pub mod drag_match;
pub mod features;
pub mod gauge_dial;
pub mod line_trend;
pub mod media;
pub mod predict;
pub mod probability_bars;
pub mod quiz;
pub mod quote;
pub mod radial_progress;
pub mod reveal;
pub mod section;
pub mod split;
pub mod stack_layers;
pub mod stat;
pub mod stats;
pub mod timeline;
pub mod title;
