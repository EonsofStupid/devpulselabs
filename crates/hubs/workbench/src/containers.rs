//! What a run of cards is filed under — loaded, never compiled in.
//!
//! A **kind** is the sort of thing you are building (a Lesson, a Guide, an
//! Episode, a Deck). A **container** is one actual instance of a kind, read
//! off disk. The authoring surface's bottom band labels itself with whichever
//! container is loaded, so this module is the only place that decides what a
//! container is called and how many things are already in it.
//!
//! Vocabulary: [`ContainerKind::slug`] is the machine id and is never drawn;
//! [`ContainerKind::label`] is what the operator reads. Same split `Shelf` and
//! `LibSection` make.
//!
//! **Two of the four ratified kinds have nowhere to live yet.** An episode is
//! a real folder (`episode.json`) and a guide is a real folder
//! (`guide.json`), but a deck is a way of *playing* an episode rather than a
//! thing on disk, and a lesson is not stored anywhere at all. Both are
//! registered here and say so in their own words — a kind that quietly
//! vanished would hide the gap instead of naming it. [`ContainerKind::not_stored_yet`]
//! carries that sentence, and the census test pins exactly which kinds are
//! backed, so the day lesson storage lands the test fails and forces the
//! wiring to land with it.

use crate::words::pretty;
use dpl_coldlight::Tone;
use std::path::{Path, PathBuf};

/// The sort of thing a run of cards is filed under.
///
/// Drawn in the operator's ratified order (mock `a3db4aee`), which is not the
/// order of how real they are — see [`ContainerKind::not_stored_yet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContainerKind {
    Lesson,
    Guide,
    Episode,
    Deck,
}

impl ContainerKind {
    pub(crate) const ALL: [ContainerKind; 4] = [
        ContainerKind::Lesson,
        ContainerKind::Guide,
        ContainerKind::Episode,
        ContainerKind::Deck,
    ];

    /// Stable machine id — never drawn.
    pub(crate) fn slug(self) -> &'static str {
        match self {
            ContainerKind::Lesson => "lesson",
            ContainerKind::Guide => "guide",
            ContainerKind::Episode => "episode",
            ContainerKind::Deck => "deck",
        }
    }

    /// What the operator reads.
    pub(crate) fn label(self) -> &'static str {
        match self {
            ContainerKind::Lesson => "Lesson",
            ContainerKind::Guide => "Guide",
            ContainerKind::Episode => "Episode",
            ContainerKind::Deck => "Deck",
        }
    }

    /// What this kind counts, plural — a guide has steps, everything else
    /// has cards.
    pub(crate) fn unit(self) -> &'static str {
        match self {
            ContainerKind::Guide => "steps",
            _ => "cards",
        }
    }

    /// The same unit, singular — for "your first card in here".
    pub(crate) fn unit_one(self) -> &'static str {
        match self {
            ContainerKind::Guide => "step",
            _ => "card",
        }
    }

    /// One line on what the kind is for.
    pub(crate) fn blurb(self) -> &'static str {
        match self {
            ContainerKind::Lesson => "teaches one idea, start to finish",
            ContainerKind::Guide => "walks through a real task, step by step",
            ContainerKind::Episode => "one show instalment, under a show",
            ContainerKind::Deck => "a run of cards for the touchscreen",
        }
    }

    /// The accent this kind wears, so the strip reads at a glance.
    pub(crate) fn tone(self) -> Tone {
        match self {
            ContainerKind::Lesson => Tone::Cyan,
            ContainerKind::Guide => Tone::Amber,
            ContainerKind::Episode => Tone::Magenta,
            ContainerKind::Deck => Tone::Coral,
        }
    }

    /// The tooltip law.
    pub(crate) fn tip(self) -> &'static str {
        match self {
            ContainerKind::Lesson => "A run of cards that teaches one idea from start to finish",
            ContainerKind::Guide => "A recorded walkthrough of a real task, step by step",
            ContainerKind::Episode => "One instalment of a show — lives under the show's name",
            ContainerKind::Deck => "A run of cards meant to be played on the touchscreen",
        }
    }

    /// Why this kind has nothing behind it, in plain words — `None` when the
    /// kind is really stored on disk.
    ///
    /// This is the honest-empty line, not an apology: it says what the kind
    /// would be and what has to exist before it can hold anything.
    pub(crate) fn not_stored_yet(self) -> Option<&'static str> {
        match self {
            ContainerKind::Episode | ContainerKind::Guide => None,
            ContainerKind::Deck => Some(
                "A deck is an episode played full-screen, so it has no folder of its own \
                 yet. Pick an Episode and play it on the touchscreen.",
            ),
            ContainerKind::Lesson => Some(
                "Lessons aren't saved anywhere yet — nothing on disk holds one. Until they \
                 are, build into an Episode and it will carry over.",
            ),
        }
    }

    /// Is this kind actually backed by something on disk?
    pub(crate) fn is_stored(self) -> bool {
        self.not_stored_yet().is_none()
    }
}

/// One container, read off disk.
///
/// `slug` / `label` rather than `id` / `name`, so that every pickable thing on
/// this surface names its two shared fields the same way — the data types
/// following the same law the `slug()`/`label()` methods already do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Container {
    /// The machine id. Never drawn.
    pub(crate) slug: String,
    pub(crate) kind: ContainerKind,
    /// What the operator reads.
    pub(crate) label: String,
    /// How many of the kind's unit are already in it.
    pub(crate) count: usize,
    pub(crate) dir: PathBuf,
}

/// THE census: every container of one kind under `root`, sorted.
///
/// Reads the disk each call, so a folder made outside the app shows up
/// without a restart. Callers hold the result for the sitting rather than
/// calling per frame.
pub(crate) fn containers(kind: ContainerKind, root: &Path) -> Vec<Container> {
    match kind {
        ContainerKind::Episode => episodes(root),
        ContainerKind::Guide => guides(root),
        // Registered, deliberately empty — `not_stored_yet` says why.
        ContainerKind::Lesson | ContainerKind::Deck => Vec::new(),
    }
}

/// `content/<show>/<episode>/episode.json` — the ratified content tree.
fn episodes(root: &Path) -> Vec<Container> {
    dpl_doc::list_content_episodes(root)
        .into_iter()
        .filter_map(|(show, episode, dir)| {
            let ep = dpl_doc::load_episode(&dir).ok()?;
            // The episode's own title is already operator-written; the folder
            // id is the fallback when it is blank.
            let title = if ep.title.trim().is_empty() {
                pretty(&episode)
            } else {
                ep.title.clone()
            };
            Some(Container {
                slug: format!("{show}/{episode}"),
                kind: ContainerKind::Episode,
                label: format!("{} · {title}", pretty(&show)),
                count: dpl_doc::episode_slides(&dir, &ep).len(),
                dir,
            })
        })
        .collect()
}

/// `inbox/<name>/guide.json` — the `.guide` folder artifact. `inbox/` also
/// holds authored drafts, so the manifest's presence is what makes a folder
/// a guide, not its position.
fn guides(root: &Path) -> Vec<Container> {
    let Ok(entries) = std::fs::read_dir(root.join("inbox")) else {
        return Vec::new();
    };
    let mut out: Vec<Container> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let dir = e.path();
            let manifest = dpl_guide::load_guide(&dir).ok()?;
            let slug = e.file_name().to_string_lossy().into_owned();
            let label = if manifest.name.trim().is_empty() {
                pretty(&slug)
            } else {
                manifest.name.clone()
            };
            Some(Container {
                slug,
                kind: ContainerKind::Guide,
                label,
                count: manifest.steps.len(),
                dir,
            })
        })
        .collect();
    out.sort_by(|a, b| a.slug.cmp(&b.slug));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The census law: the chip row IS `ContainerKind::ALL`, and every kind carries a
    /// distinct machine id.
    #[test]
    fn the_kind_row_is_the_census() {
        let slugs: Vec<&str> = ContainerKind::ALL.iter().map(|k| k.slug()).collect();
        let mut sorted = slugs.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            slugs.len(),
            "two kinds share a slug: {slugs:?}"
        );
        assert_eq!(slugs, ["lesson", "guide", "episode", "deck"]);
    }

    /// The promotion rule, applied to containers. Exactly two kinds are
    /// backed today. When a third gains storage this fails, and whoever adds
    /// it must wire `containers()` in the same commit rather than leaving a
    /// kind that lists nothing forever.
    #[test]
    fn every_kind_is_backed_or_says_why() {
        let backed: Vec<&str> = ContainerKind::ALL
            .iter()
            .filter(|k| k.is_stored())
            .map(|k| k.slug())
            .collect();
        assert_eq!(backed, ["guide", "episode"]);
        for kind in ContainerKind::ALL {
            if !kind.is_stored() {
                let why = kind
                    .not_stored_yet()
                    .expect("an unbacked kind must say why");
                assert!(
                    why.len() > 40,
                    "{}'s empty line has to explain, not shrug: {why:?}",
                    kind.slug()
                );
            }
        }
    }

    /// The UI law: nothing the operator reads speaks the code's vocabulary.
    #[test]
    fn no_kind_speaks_engineering() {
        const CODE_WORDS: [&str; 8] = [
            "slug", "enum", "struct", "json", "serde", "registry", "trait", "impl",
        ];
        for kind in ContainerKind::ALL {
            let drawn = format!(
                "{} {} {} {} {}",
                kind.label(),
                kind.blurb(),
                kind.tip(),
                kind.unit(),
                kind.not_stored_yet().unwrap_or("")
            )
            .to_lowercase();
            for word in CODE_WORDS {
                assert!(
                    !drawn.contains(word),
                    "{} says {word:?} on screen: {drawn:?}",
                    kind.slug()
                );
            }
        }
    }

    /// A kind with no storage lists nothing, and asking for containers under
    /// a root that does not exist is empty rather than a panic.
    #[test]
    fn unbacked_kinds_and_missing_roots_are_empty_not_loud() {
        let nowhere = Path::new("/does/not/exist/devpulse-test");
        for kind in ContainerKind::ALL {
            assert!(containers(kind, nowhere).is_empty());
        }
    }

    #[test]
    fn a_guide_folder_is_found_by_its_manifest() {
        let tmp = std::env::temp_dir().join(format!("dpl-containers-{}", std::process::id()));
        let inbox = tmp.join("inbox");
        std::fs::create_dir_all(inbox.join("twilio-keys")).unwrap();
        // A draft slide folder sits in the same inbox and must NOT be listed.
        std::fs::create_dir_all(inbox.join("authored-draft-1")).unwrap();
        std::fs::write(
            inbox.join("twilio-keys").join("guide.json"),
            r#"{"kind":"guide","name":"Twilio notification keys","stepCount":2,
                "steps":[{"order":1,"file":"step-001.webp"},
                         {"order":2,"file":"step-002.webp"}]}"#,
        )
        .unwrap();

        let found = containers(ContainerKind::Guide, &tmp);
        std::fs::remove_dir_all(&tmp).ok();

        assert_eq!(found.len(), 1, "only the folder with a manifest is a guide");
        assert_eq!(found[0].label, "Twilio notification keys");
        assert_eq!(found[0].count, 2);
        assert_eq!(found[0].kind, ContainerKind::Guide);
    }
}
