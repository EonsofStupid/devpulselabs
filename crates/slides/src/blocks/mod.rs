//! Blocks — atomic, insertable units. Promotion rule: a pattern lives here
//! only after recurring in ≥ 2 templates (eyebrow recurred 9× before its
//! extraction in the prior app — it arrives pre-promoted).

pub mod badge;
pub mod eyebrow;
pub mod media;
pub mod stat;

pub use badge::badge;
pub use eyebrow::{eyebrow, tracked};
pub use media::{ambient_backdrop, media_backdrop, Ambient, Media};
pub use stat::stat;
