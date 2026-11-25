//! What the chart components share: the fitted 960×540 logical stage and
//! the series tone cycle.
//!
//! Convention carried from the old `ChartFrame`: charts compose in a
//! **960×540 logical space** fitted (aspect-preserving, centered) into
//! whatever stage rect the host gives. Strokes and type scale WITH the
//! stage. Paint always resolves through the pool via [`tone_cycle`].

pub mod stage;
pub mod tone_cycle;
