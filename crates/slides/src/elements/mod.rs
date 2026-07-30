//! Elements — the ladder's first rung: atomic, insertable units. Promotion
//! rule: a pattern lives here only after recurring in ≥ 2 templates (eyebrow
//! recurred 9× before its extraction in the prior app — it arrives
//! pre-promoted).
//!
//! An element paints from its arguments and nothing else — no route, no global
//! state — which is what lets the same call be correct in a Workbench preview
//! pane and on the 75" kiosk.
//!
//! The [`census`] module is the element registry (Census law): it categorizes
//! every element, declares its `cost_tier`, and enumerates the catalog the
//! Library shelves. The elements themselves stay ergonomic free functions.
//!
//! Composites of these live one rung up, in [`crate::blocks`].

pub mod badge;
pub mod bar;
pub mod census;
pub mod divider;
pub mod eyebrow;
pub mod media;
pub mod stat;

pub use census::{elements, CostTier, ElementCategory, ElementDemo, ElementId, ElementSpec};

pub use badge::badge;
pub use bar::bar;
pub use divider::rule;
pub use eyebrow::{eyebrow, tracked};
pub use media::{
    ambient_backdrop, asset_thumb, media_backdrop, media_block_backdrop, Ambient, Media,
};
pub use stat::stat;
