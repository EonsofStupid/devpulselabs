//! The series palette — ported `TONE_CYCLE` order, drawing from the pool.
//!
//! Old order: `[brand, cyan, lime, magenta, violet, coral]`. "Brand" is the
//! slide's own tone (slot 0); free accents follow in the carried order.

use dpl_coldlight::Tone;

const ACCENTS: [Tone; 5] = [Tone::Cyan, Tone::Lime, Tone::Magenta, Tone::Violet, Tone::Coral];

/// Tone for series `index` under a slide toned `base`. Slot 0 is the slide's
/// tone; later slots walk the accent order, skipping the base so two series
/// never share a color.
pub fn series_tone(index: usize, base: Tone) -> Tone {
    if index == 0 {
        return base;
    }
    let free: Vec<Tone> = ACCENTS.iter().copied().filter(|t| *t != base).collect();
    free[(index - 1) % free.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carries_the_old_cycle_order() {
        // Base amber (a brand tone outside the accent list): the accents run
        // in the carried order cyan → lime → magenta → violet → coral.
        let order: Vec<Tone> = (0..6).map(|i| series_tone(i, Tone::Amber)).collect();
        assert_eq!(
            order,
            vec![Tone::Amber, Tone::Cyan, Tone::Lime, Tone::Magenta, Tone::Violet, Tone::Coral]
        );
    }

    #[test]
    fn base_never_repeats_in_the_free_slots() {
        for i in 1..8 {
            assert_ne!(series_tone(i, Tone::Cyan), Tone::Cyan);
        }
    }
}
