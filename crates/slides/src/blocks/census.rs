//! The block census — the second rung's registry.
//!
//! Census law (architecture.md §4): anywhere a LIST of things exists, ONE
//! registry function enumerates it and a census test fails when reality and the
//! list drift — nothing else enumerates. [`blocks`] is that function for the
//! chunk tier, mirroring [`crate::elements::census::elements`] for elements and
//! `crate::registry::demos` for templates.
//!
//! [`blocks`] returns EMPTY today. The census test asserts that emptiness
//! deliberately, so the day a block is promoted the count moves in the same
//! commit that registers it — the drift the Census law exists to catch.
//!
//! Adding a block, once it has EARNED the rung (recurs in ≥ 2 templates): its
//! module under `blocks/`, then a spec + demo + ONE line in [`blocks`] here.

use crate::elements::{CostTier, ElementId};
use egui::Ui;

/// Block identity — never a bare string (mirrors [`ElementId`] and
/// `TemplateId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub &'static str);

impl std::fmt::Display for BlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// What the Library shelf shows for a block, plus the metadata the treatment
/// and fidelity layers read. One `static` per block, once any exist.
#[derive(Debug)]
pub struct BlockSpec {
    pub id: BlockId,
    pub label: &'static str,
    /// The elements this block composes — the answer to "what is this built
    /// from", which the authoring surface reads when it offers a block and
    /// when it decides whether new elements must be minted first.
    pub made_of: &'static [ElementId],
    /// Baseline fidelity tier (design-system §5), same contract as elements: a
    /// block is never cheaper than the elements it composes.
    pub cost_tier: CostTier,
    /// One line — the Library shelf copy.
    pub blurb: &'static str,
}

/// A live, self-contained sample of one block — the census unit, mirroring
/// [`crate::elements::census::ElementDemo`].
pub trait BlockDemo {
    fn spec(&self) -> &'static BlockSpec;
    /// Paint the sample into the caller's `ui` (a Library plate, or a test
    /// stage).
    fn paint(&self, ui: &mut Ui);
}

/// One demo per registered block. ONE line per block — the census: a block
/// absent here cannot show in the Library.
///
/// Empty by design — see the module doc. Promotion, not authorship, fills this.
pub fn blocks() -> Vec<Box<dyn BlockDemo>> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// The census holds even when it is empty: the count is asserted, so
    /// promoting a block without registering it (or vice versa) fails here.
    #[test]
    fn the_registry_is_the_census() {
        let all = blocks();
        assert_eq!(
            all.len(),
            0,
            "the chunk tier is empty until a composite earns promotion \
             (recurs in >= 2 templates) — update this count in the SAME commit \
             that registers one"
        );
        let ids: HashSet<&str> = all.iter().map(|b| b.spec().id.0).collect();
        assert_eq!(ids.len(), all.len(), "block ids must be unique");
        for b in &all {
            assert!(!b.spec().label.is_empty());
            assert!(!b.spec().blurb.is_empty());
            assert!(
                !b.spec().made_of.is_empty(),
                "a block composes elements — `made_of` cannot be empty"
            );
        }
    }

    /// The GALLERY GATE, teeth (mirrors the element and template censuses):
    /// every registered block must actually PAINT, at entrance and at rest,
    /// without panicking. Vacuous today; live the moment one registers.
    #[test]
    fn every_block_paints_at_start_and_at_rest() {
        let ctx = egui::Context::default();
        dpl_coldlight::apply_theme(&ctx);
        for block in blocks() {
            for time in [0.05_f64, 3.0_f64] {
                let mut input = egui::RawInput {
                    time: Some(time),
                    ..Default::default()
                };
                input.screen_rect = Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(1280.0, 720.0),
                ));
                let _ = ctx.run_ui(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        block.paint(ui);
                    });
                });
            }
        }
    }
}
