//! Chat's composer census: what the `+` menu offers, which modes stand on the
//! composer, and what each one needs from the AI behind the conversation.
//!
//! ## Why this is chat's, and pure data
//!
//! The composer is a chat surface, so its census lives with chat and nowhere
//! else. Every entry is gated on a [`Capability`] — the vocabulary a provider
//! ADVERTISES — so "Thinking" is not a chip chat decides to show, it is
//! `Capability::Modes` being true for the AI the operator is talking to. A
//! surface that hard-coded its own list would lie the first time it met an AI
//! without that capability; here a surface reads the census and asks it.
//!
//! This module holds no egui and does no IO. The window paints it; the host
//! performs what a pick asks for.
//!
//! ## Two groups, one list
//!
//! - **Menu** — the `+` popup: things the operator brings or opens (attach,
//!   image/video, a component, voice). Each happens once.
//! - **Mode** — chips on the composer: how the NEXT turn should run
//!   (thinking, plan first, web search, brainstorm). Each stays on until it
//!   is turned off, and rides the send.
//!
//! ## The aliases are the point
//!
//! An operator arrives with the words another product taught them. "Think
//! longer", "extended thinking" and "reasoning" are one mode, and [`resolve`]
//! answers all of them with it — case- and whitespace-insensitively, because
//! it answers what somebody typed, never a wire format we control.

use dpl_ai_core::capability::Capability;
use std::collections::BTreeSet;

/// Where an entry is painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ComposerGroup {
    /// The `+` popup — something the operator brings in or opens, once.
    Menu,
    /// A chip on the composer — how the next turn should run.
    Mode,
}

impl ComposerGroup {
    pub const ALL: [Self; 2] = [Self::Menu, Self::Mode];
}

/// What an entry needs before it can be offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// Ours alone — no AI can withhold it.
    Ours,
    /// The AI must advertise at least one of these.
    AnyOf(&'static [Capability]),
    /// Not built yet. Always shown, never offered, and it says why.
    Unbuilt(&'static str),
}

/// Whether an entry can be used with the AI behind the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Offered,
    /// The AI did not advertise what the entry needs.
    NotAdvertised,
    /// The entry is not built yet; the reason is plain words.
    Unbuilt(&'static str),
}

/// One composer entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposerEntry {
    /// Our canonical name. Stable — it is what gets traced and sent.
    pub slug: &'static str,
    /// What the operator reads.
    pub label: &'static str,
    /// The tooltip, in plain words.
    pub tip: &'static str,
    /// The registry icon the surface draws (`tokens/icons.json`).
    pub icon: &'static str,
    pub group: ComposerGroup,
    pub gate: Gate,
    /// Every other name this entry answers to, stored lowercase.
    pub aliases: &'static [&'static str],
}

impl ComposerEntry {
    /// Can this entry be used with an AI that advertises `advertised`?
    pub fn availability(&self, advertised: &[Capability]) -> Availability {
        match self.gate {
            Gate::Ours => Availability::Offered,
            Gate::AnyOf(needs) if needs.iter().any(|need| advertised.contains(need)) => {
                Availability::Offered
            }
            Gate::AnyOf(_) => Availability::NotAdvertised,
            Gate::Unbuilt(reason) => Availability::Unbuilt(reason),
        }
    }

    /// Why this entry is disabled, in plain words naming the AI — `None`
    /// when it is offered. `ai` is `None` when no AI is chosen yet.
    pub fn withheld_reason(&self, ai: Option<&str>, advertised: &[Capability]) -> Option<String> {
        match self.availability(advertised) {
            Availability::Offered => None,
            Availability::Unbuilt(reason) => Some(reason.to_owned()),
            Availability::NotAdvertised => Some(match ai {
                Some(ai) => format!("{ai} doesn't offer {} yet.", self.label),
                None => format!("Choose an AI to use {}.", self.label),
            }),
        }
    }
}

/// The census. Adding a row here is how the composer grows — there is no
/// second list, and a surface painting anything not in here paints a lie.
pub const COMPOSER: &[ComposerEntry] = &[
    // ── the `+` menu ─────────────────────────────────────────────────────
    ComposerEntry {
        slug: "attach",
        label: "Attach files",
        tip: "Bring files or pictures from this computer into your message.",
        icon: "attach",
        group: ComposerGroup::Menu,
        gate: Gate::AnyOf(&[Capability::Attachments]),
        aliases: &[
            "add files or photos",
            "add photos & files",
            "add photos and files",
            "upload a file",
            "upload",
            "files",
        ],
    },
    ComposerEntry {
        slug: "media",
        label: "Image / video",
        tip: "Make a picture or a short clip from what you type.",
        icon: "media",
        group: ComposerGroup::Menu,
        gate: Gate::AnyOf(&[Capability::Images, Capability::Video]),
        aliases: &[
            "create image",
            "create an image",
            "imagine",
            "image",
            "video",
        ],
    },
    ComposerEntry {
        slug: "component",
        label: "Component",
        tip: "Drop a working piece into the chat — today, a brainstorm board.",
        icon: "component",
        group: ComposerGroup::Menu,
        // Ours: the board is DevPULSE's, no AI can withhold it.
        gate: Gate::Ours,
        aliases: &["board", "brainstorm board", "interaction component"],
    },
    ComposerEntry {
        slug: "voice",
        label: "Voice",
        tip: "Talk instead of typing.",
        icon: "voice",
        group: ComposerGroup::Menu,
        gate: Gate::Unbuilt("Talking to Chat is not ready yet — type for now."),
        aliases: &["dictate", "speak", "microphone"],
    },
    // ── the modes ────────────────────────────────────────────────────────
    ComposerEntry {
        slug: "thinking",
        label: "Thinking",
        tip: "Ask the AI to think longer before it answers.",
        icon: "mode-thinking",
        group: ComposerGroup::Mode,
        gate: Gate::AnyOf(&[Capability::Modes]),
        aliases: &[
            "think longer",
            "extended thinking",
            "extended-thinking",
            "reasoning",
        ],
    },
    ComposerEntry {
        slug: "plan",
        label: "Plan first",
        tip: "Ask the AI for a plan before it does any work.",
        icon: "mode-plan",
        group: ComposerGroup::Mode,
        gate: Gate::AnyOf(&[Capability::Plans]),
        aliases: &["plan mode", "planning"],
    },
    ComposerEntry {
        slug: "web-search",
        label: "Web search",
        tip: "Let the AI look things up online instead of answering from memory.",
        icon: "mode-web",
        group: ComposerGroup::Mode,
        gate: Gate::AnyOf(&[Capability::Tools]),
        aliases: &["search the web", "search", "browse"],
    },
    ComposerEntry {
        slug: "brainstorm",
        label: "Brainstorm",
        tip: "Ask for many ideas instead of one answer.",
        icon: "mode-brainstorm",
        group: ComposerGroup::Mode,
        // Ours: asking for ideas needs nothing an AI could withhold.
        gate: Gate::Ours,
        aliases: &["brainstorming", "ideas", "group discussion"],
    },
];

/// The entries in one group, in census order.
pub fn in_group(group: ComposerGroup) -> impl Iterator<Item = &'static ComposerEntry> {
    COMPOSER.iter().filter(move |entry| entry.group == group)
}

/// Resolve a slug, a label or any alias to its entry.
///
/// Case- and whitespace-insensitive. Prefixes never match: "web" is not
/// "web search", and offering the wrong mode is worse than offering none.
pub fn resolve(name: &str) -> Option<&'static ComposerEntry> {
    let needle = name.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return None;
    }
    COMPOSER.iter().find(|entry| {
        entry.slug.eq_ignore_ascii_case(&needle)
            || entry.label.eq_ignore_ascii_case(&needle)
            || entry.aliases.iter().any(|alias| *alias == needle)
    })
}

/// A toggle refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModeError {
    /// No entry answers to that name.
    Unknown(String),
    /// The entry exists but is a `+` menu entry, which happens once.
    NotAMode(&'static str),
}

impl std::fmt::Display for ModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown(name) => write!(f, "{name} is not a composer entry"),
            Self::NotAMode(slug) => write!(f, "{slug} happens once; it is not a mode"),
        }
    }
}

impl std::error::Error for ModeError {}

/// The modes standing on the composer — what the NEXT turn is sent with.
///
/// Held as canonical slugs, so the set survives a label changing and a host
/// matches on names it controls. It rides the send: a mode that never
/// reached the turn would be a light on a surface, not a fact.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComposerModes {
    active: BTreeSet<&'static str>,
}

impl ComposerModes {
    /// Turn a mode on or off. Returns the new state.
    ///
    /// A `+` menu entry is refused rather than stored: "attach" left
    /// switched on would mean nothing, and a set that holds it would lie.
    pub fn toggle(&mut self, name: &str) -> Result<bool, ModeError> {
        let entry = resolve(name).ok_or_else(|| ModeError::Unknown(name.to_owned()))?;
        if entry.group != ComposerGroup::Mode {
            return Err(ModeError::NotAMode(entry.slug));
        }
        let now_on = !self.active.remove(entry.slug);
        if now_on {
            self.active.insert(entry.slug);
        }
        Ok(now_on)
    }

    pub fn is_active(&self, name: &str) -> bool {
        resolve(name).is_some_and(|entry| self.active.contains(entry.slug))
    }

    /// The standing modes, in census order so a surface paints them stably.
    pub fn active(&self) -> Vec<&'static ComposerEntry> {
        COMPOSER
            .iter()
            .filter(|entry| self.active.contains(entry.slug))
            .collect()
    }

    /// The standing modes' slugs, in census order — what a host traces.
    pub fn slugs(&self) -> Vec<&'static str> {
        self.active().iter().map(|entry| entry.slug).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.active.is_empty()
    }

    /// Drop every mode the AI does not advertise, and say which went.
    ///
    /// Switching to an AI without web search must not leave "Web search"
    /// lit — the operator would believe the turn searched when it could not.
    /// A mode that cannot be honoured is removed and NAMED, never kept.
    pub fn retain_offered(&mut self, advertised: &[Capability]) -> Vec<&'static ComposerEntry> {
        let dropped: Vec<_> = self
            .active()
            .into_iter()
            .filter(|entry| entry.availability(advertised) != Availability::Offered)
            .collect();
        for entry in &dropped {
            self.active.remove(entry.slug);
        }
        dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_census_is_unambiguous() {
        // One name, one entry: two entries answering to the same word is how
        // a menu starts doing something other than what was asked.
        let mut names = BTreeSet::new();
        for entry in COMPOSER {
            // An entry's slug may read the same as its label ("voice" and
            // "Voice"); that is one name, so dedupe within the entry first.
            let own: BTreeSet<String> = std::iter::once(entry.slug.to_owned())
                .chain(std::iter::once(entry.label.to_ascii_lowercase()))
                .chain(entry.aliases.iter().map(|alias| (*alias).to_owned()))
                .collect();
            for name in own {
                assert!(names.insert(name.clone()), "{name} is claimed twice");
            }
            for alias in entry.aliases {
                assert_eq!(
                    *alias,
                    alias.to_ascii_lowercase(),
                    "aliases are stored lowercase so a match is a comparison"
                );
            }
            assert!(!entry.label.is_empty() && !entry.tip.is_empty() && !entry.icon.is_empty());
        }
    }

    #[test]
    fn every_name_resolves_back_to_its_entry() {
        for entry in COMPOSER {
            assert_eq!(resolve(entry.slug), Some(entry), "slug");
            assert_eq!(resolve(entry.label), Some(entry), "label");
            assert_eq!(resolve(&entry.label.to_ascii_uppercase()), Some(entry));
            assert_eq!(resolve(&format!("  {}  ", entry.slug)), Some(entry));
            for alias in entry.aliases {
                assert_eq!(resolve(alias), Some(entry), "alias {alias}");
            }
        }
    }

    #[test]
    fn an_unknown_name_is_none_not_a_near_miss() {
        assert_eq!(resolve("teleport"), None);
        assert_eq!(resolve(""), None);
        assert_eq!(resolve("   "), None);
        assert_eq!(resolve("web"), None, "a prefix is not a match");
        assert_eq!(resolve("think"), None);
    }

    #[test]
    fn the_menu_is_attach_media_component_voice_in_that_order() {
        let menu: Vec<_> = in_group(ComposerGroup::Menu)
            .map(|entry| entry.slug)
            .collect();
        assert_eq!(menu, ["attach", "media", "component", "voice"]);
        let modes: Vec<_> = in_group(ComposerGroup::Mode)
            .map(|entry| entry.slug)
            .collect();
        assert_eq!(modes, ["thinking", "plan", "web-search", "brainstorm"]);
        let counted: usize = ComposerGroup::ALL
            .iter()
            .map(|group| in_group(*group).count())
            .sum();
        assert_eq!(counted, COMPOSER.len(), "every entry belongs to a group");
    }

    #[test]
    fn availability_follows_what_the_ai_advertises() {
        let thinking = resolve("thinking").expect("thinking");
        assert_eq!(thinking.availability(&[]), Availability::NotAdvertised);
        assert_eq!(
            thinking.availability(&[Capability::Modes]),
            Availability::Offered
        );
        // Any ONE of the listed capabilities is enough.
        let media = resolve("media").expect("media");
        assert_eq!(
            media.availability(&[Capability::Video]),
            Availability::Offered
        );
        assert_eq!(
            media.availability(&[Capability::Images]),
            Availability::Offered
        );
        assert_eq!(
            media.availability(&[Capability::Tools]),
            Availability::NotAdvertised
        );
        // Ours is offered against an AI that advertises nothing.
        assert_eq!(
            resolve("brainstorm").expect("brainstorm").availability(&[]),
            Availability::Offered
        );
        assert_eq!(
            resolve("component").expect("component").availability(&[]),
            Availability::Offered
        );
        // Unbuilt is never offered, whatever the AI advertises.
        let every: Vec<_> = COMPOSER
            .iter()
            .filter_map(|entry| match entry.gate {
                Gate::AnyOf(needs) => Some(needs),
                _ => None,
            })
            .flatten()
            .copied()
            .collect();
        assert!(matches!(
            resolve("voice").expect("voice").availability(&every),
            Availability::Unbuilt(_)
        ));
    }

    #[test]
    fn modes_per_provider_follow_their_advertised_capabilities() {
        // The shapes the providers ship today: a Codex-like AI advertises
        // Tools and Plans but (in the authored fixture) not Modes; a bare
        // experimental AI advertises nothing a mode needs.
        let codex_like = [Capability::Tools, Capability::Plans, Capability::Images];
        let offered = |advertised: &[Capability]| -> Vec<&str> {
            in_group(ComposerGroup::Mode)
                .filter(|entry| entry.availability(advertised) == Availability::Offered)
                .map(|entry| entry.slug)
                .collect()
        };
        assert_eq!(offered(&codex_like), ["plan", "web-search", "brainstorm"]);
        assert_eq!(offered(&[]), ["brainstorm"]);
        let full = [Capability::Modes, Capability::Plans, Capability::Tools];
        assert_eq!(
            offered(&full),
            ["thinking", "plan", "web-search", "brainstorm"]
        );
    }

    #[test]
    fn a_withheld_entry_names_the_ai_and_the_mode() {
        let thinking = resolve("thinking").expect("thinking");
        assert_eq!(
            thinking.withheld_reason(Some("Codex"), &[]).as_deref(),
            Some("Codex doesn't offer Thinking yet.")
        );
        assert_eq!(
            thinking.withheld_reason(None, &[]).as_deref(),
            Some("Choose an AI to use Thinking.")
        );
        assert_eq!(
            thinking.withheld_reason(Some("Codex"), &[Capability::Modes]),
            None
        );
        assert!(resolve("voice")
            .expect("voice")
            .withheld_reason(Some("Codex"), &[])
            .is_some_and(|reason| reason.contains("not ready")));
    }

    #[test]
    fn only_modes_toggle_and_aliases_reach_the_same_mode() {
        let mut modes = ComposerModes::default();
        assert_eq!(modes.toggle("web-search"), Ok(true));
        assert!(modes.is_active("search the web"));
        assert_eq!(modes.toggle("Web search"), Ok(false));
        assert!(modes.is_empty());
        assert_eq!(modes.toggle("attach"), Err(ModeError::NotAMode("attach")));
        assert_eq!(
            modes.toggle("nonsense"),
            Err(ModeError::Unknown("nonsense".to_owned()))
        );
        assert!(modes.is_empty());
    }

    #[test]
    fn active_modes_come_back_in_census_order() {
        let mut modes = ComposerModes::default();
        for slug in ["brainstorm", "web-search", "thinking"] {
            modes.toggle(slug).expect("a mode");
        }
        assert_eq!(modes.slugs(), ["thinking", "web-search", "brainstorm"]);
    }

    #[test]
    fn a_mode_the_ai_cannot_honour_is_dropped_and_named() {
        let mut modes = ComposerModes::default();
        modes.toggle("web-search").expect("a mode");
        modes.toggle("thinking").expect("a mode");
        modes.toggle("brainstorm").expect("a mode");

        let kept = modes.retain_offered(&[Capability::Tools, Capability::Modes]);
        assert!(kept.is_empty(), "all three are honourable here");

        let dropped = modes.retain_offered(&[Capability::Modes]);
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].slug, "web-search");
        assert!(!modes.is_active("web-search"));
        assert!(modes.is_active("thinking"));

        // Ours survives an AI that advertises nothing.
        let dropped = modes.retain_offered(&[]);
        assert_eq!(dropped.len(), 1);
        assert_eq!(modes.slugs(), ["brainstorm"]);
    }
}
