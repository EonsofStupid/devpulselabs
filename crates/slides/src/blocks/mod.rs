//! Blocks — the ladder's second rung: a few [`crate::elements`] composed into
//! one reusable chunk.
//!
//! **This tier ships empty, and that is its correct state.** The promotion rule
//! (crate doc) says a pattern climbs a rung only after recurring in ≥ 2
//! templates; nothing is authored here to fill a shelf. The tier exists now so
//! that the folder structure equals the vocabulary the operator sees — Elements
//! → Blocks → Templates — rather than the first rung wearing the second's name
//! (the collision DS3 closed).
//!
//! Which composites have actually earned promotion is a recurrence audit across
//! the registered templates. That audit, and the first real blocks it justifies,
//! are their own pass — not this one.
//!
//! The [`census`] module is this tier's registry, mirroring
//! [`crate::elements::census`] for elements and `crate::registry` for
//! templates: ONE function enumerates, and a census test fails when reality and
//! the list drift.

pub mod census;

pub use census::{blocks, BlockDemo, BlockId, BlockSpec};
