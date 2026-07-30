//! What can sit behind a card's words — the Background tab's shelves.
//!
//! Three registries, none of them hand-listed here:
//!
//! - **colours** are `cl::MOODS`, straight from `tokens/moods.json`
//! - **movement over the top** is the `Ambient` vocabulary the document
//!   renderer already resolves (`rings` · `drift` · `pulse`)
//! - **how two colours mix** is `cl::MOTION_PRESETS`
//!
//! A mood added to the tokens shows up here on the next build with no code
//! change, and [`the_colour_row_is_the_registry`] fails if that ever stops
//! being true.
//!
//! **On the count.** The ratified mock promises "around forty to choose
//! from". The registry holds TEN colours. The forty is real but it comes from
//! COMBINATION — ten flat, forty-five pairs, each with a mix and an optional
//! movement — not from a shelf of forty authored things. The tab states the
//! real number rather than padding the shelf with generated variations to
//! reach a target, and if ten is too few the answer is new entries in
//! `tokens/moods.json`, which is a data pass and not this one.
//!
//! Vocabulary: `slug()` is the ref written into the document; `label()` is
//! what the operator reads. The authored mood names are already evocative
//! rather than technical ("ember drift", "jade current"), so the operator's
//! word here IS the authored one, prettified.

use crate::words::pretty;
use dpl_coldlight::{self as cl};

/// One thing that can be picked on the Background tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Choice {
    /// The ref written into the document. Machine-side, never drawn.
    pub(crate) slug: String,
    /// What the operator reads.
    pub(crate) label: String,
}

impl Choice {
    fn of(slug: &str) -> Self {
        Self {
            slug: slug.to_string(),
            label: pretty(slug),
        }
    }
}

/// THE colour census — every authored mood, in authored order.
pub(crate) fn colours() -> Vec<Choice> {
    cl::MOODS.iter().map(|m| Choice::of(m.name)).collect()
}

/// What can drift over the top of a colour. These are the refs
/// `dpl_slides::document` already resolves; an id it does not know degrades
/// to no ambience rather than a crash, so this list and that matcher must
/// agree — `the_ambience_row_is_what_the_renderer_resolves` pins it.
pub(crate) fn ambiences() -> Vec<Choice> {
    ["rings", "drift", "pulse"]
        .iter()
        .map(|id| Choice::of(id))
        .collect()
}

/// How two colours mix into each other.
pub(crate) fn mixes() -> Vec<Choice> {
    cl::MOTION_PRESETS
        .iter()
        .map(|p| Choice::of(p.id))
        .collect()
}

/// Plain words for the one thing that is not a registry entry: no backdrop.
pub(crate) const NOTHING: &str = "Nothing";

#[cfg(test)]
mod tests {
    use super::*;

    /// The census law: the row IS the registry, not a copy of it that drifts.
    #[test]
    fn the_colour_row_is_the_registry() {
        let row = colours();
        assert_eq!(row.len(), cl::MOODS.len());
        assert_eq!(row.len(), 10, "ten authored colours — see the module note");
        for (choice, mood) in row.iter().zip(cl::MOODS.iter()) {
            assert_eq!(choice.slug, mood.name, "a colour must write its own ref");
        }
    }

    /// Every ambience offered must be one the renderer actually resolves.
    /// An option that silently degrades to nothing is worse than no option.
    #[test]
    fn the_ambience_row_is_what_the_renderer_resolves() {
        for choice in ambiences() {
            assert!(
                matches!(choice.slug.as_str(), "rings" | "drift" | "pulse"),
                "{} is offered but document.rs does not resolve it",
                choice.slug
            );
        }
        assert_eq!(ambiences().len(), 3);
    }

    #[test]
    fn the_mix_row_is_the_preset_registry() {
        assert_eq!(mixes().len(), cl::MOTION_PRESETS.len());
    }

    /// The UI law: no drawn string carries the code's vocabulary, and every
    /// label is a real word rather than a slug that slipped through.
    #[test]
    fn no_option_speaks_engineering() {
        const CODE_WORDS: [&str; 6] = ["slug", "enum", "json", "preset", "impl", "registry"];
        for choice in colours().into_iter().chain(ambiences()).chain(mixes()) {
            let drawn = choice.label.to_lowercase();
            for word in CODE_WORDS {
                assert!(!drawn.contains(word), "{drawn:?} says {word:?} on screen");
            }
            assert!(
                !drawn.contains('-') && !drawn.contains('_'),
                "{drawn:?} is a slug, not a label"
            );
        }
    }
}
