//! Blocks — atomic, insertable units. Promotion rule: a pattern lives here
//! only after recurring in ≥ 2 templates (eyebrow recurred 9× before its
//! extraction in the prior app — it arrives pre-promoted).
//!
//! The [`census`] module is the element registry (Census law): it categorizes
//! every block, declares its `cost_tier`, and enumerates the catalog the
//! Library shelves. The blocks themselves stay ergonomic free functions.

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
pub use media::{ambient_backdrop, media_backdrop, media_block_backdrop, Ambient, Media};
pub use stat::stat;
