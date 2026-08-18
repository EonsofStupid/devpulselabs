//! The composer `+` menu, as ONE census with vendor spellings as aliases.
//!
//! ## Why this is agnostic vocabulary and not chat's
//!
//! Every entry here is gated on a [`Capability`], and capability is what a
//! provider declares. "Create image" is not a menu item we decide to show — it
//! is `Capability::Images` being true for the backing this session holds. Put
//! the census in the chat surface and the menu becomes a hardcoded list that
//! lies the moment a provider without images is selected.
//!
//! ## What ChatGPT and Claude actually do, and why the grouping exists
//!
//! They ship the same concepts behind two different shapes:
//!
//! - **Claude** puts everything under one `+` — nine entries in 2026, separated
//!   into attach (files/photos, screenshot, add to project), connect (skills,
//!   connectors, plugins) and modes (research, web search, use style).
//! - **ChatGPT** splits them: `+` holds attach and cloud connect, while the
//!   modes (agent mode, deep research, canvas, create image, study and learn)
//!   live in a separate tools control on the composer.
//!
//! Neither shape is more correct, and we should be able to render either
//! without a second list existing. That is what [`ComposerGroup`] is for: the
//! census carries the grouping, and a surface decides whether to draw one menu
//! with separators or two controls. A surface that hardcoded the split would
//! make the other shape a rewrite.
//!
//! ## The aliases are the point
//!
//! An operator arriving from either product types or looks for the words that
//! product taught them. "Add photos & files", "Upload a file" and "Add files or
//! photos" are three names for one action, and [`resolve`] answers all three
//! with the same [`ComposerAction`]. That is the whole reason the aliases live
//! beside the canonical name rather than in a translation table somewhere: one
//! store, checked by one test, and a new vendor spelling is a one-line edit.

use crate::Capability;

/// Where an entry belongs when a surface draws the menu.
///
/// Ordered as a menu reads: what you bring, what you connect, how it thinks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ComposerGroup {
    /// Content the operator brings into the turn.
    Attach,
    /// Standing connections to somewhere else.
    Connect,
    /// How the turn is run — a toggle or a mode, never content.
    Mode,
}

impl ComposerGroup {
    pub const ALL: [Self; 3] = [Self::Attach, Self::Connect, Self::Mode];

    /// The separator label a merged (Claude-shaped) menu paints.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Attach => "Attach",
            Self::Connect => "Connect",
            Self::Mode => "Mode",
        }
    }
}

/// One entry in the `+` menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComposerAction {
    /// Our canonical name. Stable — it is what gets persisted and traced.
    pub slug: &'static str,
    /// What the operator reads.
    pub label: &'static str,
    /// One honest sentence. Not marketing.
    pub blurb: &'static str,
    pub glyph: &'static str,
    pub group: ComposerGroup,
    /// The capability the backing must declare. `None` means the action is
    /// ours alone and no provider can withhold it.
    pub requires: Option<Capability>,
    /// A mode the operator turns on and leaves on, rather than an action that
    /// happens once. Claude paints a check on these; ChatGPT a pill.
    pub sticky: bool,
    /// Every other name this action is known by — the vendors' spellings, and
    /// the short words people type. Matched case-insensitively by [`resolve`].
    pub aliases: &'static [&'static str],
}

/// The census. Adding a row here is how the menu grows — there is no second
/// list, and a surface that renders anything not in here is rendering a lie.
pub const COMPOSER_ACTIONS: &[ComposerAction] = &[
    ComposerAction {
        slug: "attach-files",
        label: "Add files or photos",
        blurb: "Bring files or images from this machine into the turn.",
        glyph: "+",
        group: ComposerGroup::Attach,
        requires: Some(Capability::Attachments),
        sticky: false,
        aliases: &[
            "add files or photos",
            "add photos & files",
            "add photos and files",
            "upload a file",
            "upload files",
            "attach",
            "upload",
            "files",
        ],
    },
    ComposerAction {
        slug: "screenshot",
        label: "Take a screenshot",
        blurb: "Capture part of the screen and attach it without leaving the composer.",
        glyph: "⧉",
        group: ComposerGroup::Attach,
        requires: Some(Capability::Attachments),
        sticky: false,
        aliases: &["take a screenshot", "capture", "screen shot"],
    },
    ComposerAction {
        slug: "add-to-project",
        label: "Add to project",
        blurb: "File this conversation under a project so its context carries forward.",
        glyph: "▤",
        group: ComposerGroup::Attach,
        // Ours: a workspace is our unit, not something a provider grants.
        requires: None,
        sticky: false,
        aliases: &["add to project", "project", "add to workspace"],
    },
    ComposerAction {
        slug: "connectors",
        label: "Add connectors",
        blurb: "Connect a service — Drive, GitHub, Slack — so the turn can read from it.",
        glyph: "⚯",
        group: ComposerGroup::Connect,
        requires: Some(Capability::Tools),
        sticky: true,
        aliases: &[
            "add connectors",
            "connect apps",
            "add from google drive",
            "add from github",
            "google drive",
            "github",
            "apps",
        ],
    },
    ComposerAction {
        slug: "plugins",
        label: "Add plugins",
        blurb: "Attach an MCP server's tools to this session.",
        glyph: "⊞",
        group: ComposerGroup::Connect,
        requires: Some(Capability::Tools),
        sticky: true,
        aliases: &["add plugins", "mcp", "add mcp server"],
    },
    ComposerAction {
        slug: "skills",
        label: "Skills",
        blurb: "Repeatable behaviour the assistant loads for this kind of work.",
        glyph: "◈",
        group: ComposerGroup::Connect,
        requires: Some(Capability::Tools),
        sticky: true,
        // Claude's Skills absorbed what Styles used to be, so both names land
        // here and `style` stays separate for the surfaces still calling it that.
        aliases: &["skill"],
    },
    ComposerAction {
        slug: "web-search",
        label: "Web search",
        blurb: "Let the turn search the live web instead of answering from training.",
        glyph: "⌕",
        group: ComposerGroup::Mode,
        requires: Some(Capability::Tools),
        sticky: true,
        aliases: &["web search", "search the web", "search", "browse"],
    },
    ComposerAction {
        slug: "research",
        label: "Research",
        blurb: "A long multi-source pass that reads before it answers.",
        glyph: "◎",
        group: ComposerGroup::Mode,
        requires: Some(Capability::Modes),
        sticky: false,
        aliases: &["deep research", "deep-research"],
    },
    ComposerAction {
        slug: "extended-thinking",
        label: "Think longer",
        blurb: "Spend more of the turn reasoning before the first token.",
        glyph: "◌",
        group: ComposerGroup::Mode,
        requires: Some(Capability::Modes),
        sticky: true,
        aliases: &[
            "think longer",
            "extended thinking",
            "thinking",
            "reasoning",
        ],
    },
    ComposerAction {
        slug: "create-image",
        label: "Create image",
        blurb: "Generate an image in the conversation.",
        glyph: "▦",
        group: ComposerGroup::Mode,
        requires: Some(Capability::Images),
        sticky: false,
        aliases: &["create image", "create an image", "image", "imagine", "draw"],
    },
    ComposerAction {
        slug: "canvas",
        label: "Canvas",
        blurb: "Work on a document beside the conversation instead of inside it.",
        glyph: "▭",
        group: ComposerGroup::Mode,
        requires: Some(Capability::Modes),
        sticky: true,
        aliases: &["artifacts", "artifact"],
    },
    ComposerAction {
        slug: "agent",
        label: "Agent mode",
        blurb: "Hand the turn a machine of its own and let it act, with approvals.",
        glyph: "⎔",
        group: ComposerGroup::Mode,
        requires: Some(Capability::Modes),
        sticky: false,
        aliases: &["agent mode", "autonomous"],
    },
    ComposerAction {
        slug: "style",
        label: "Use style",
        blurb: "Answer in a chosen voice and shape.",
        glyph: "✎",
        group: ComposerGroup::Mode,
        requires: Some(Capability::Modes),
        sticky: true,
        aliases: &["use style", "styles", "study and learn", "learning style"],
    },
];

/// Every action, in census order.
pub fn composer_actions() -> &'static [ComposerAction] {
    COMPOSER_ACTIONS
}

/// The actions a backing with these capabilities can honestly offer.
///
/// Filtering here rather than in a surface is what keeps the menu truthful:
/// a provider that never declared `Images` cannot be asked to create one, and
/// the operator is not shown a control that would fail.
pub fn offered_for(capabilities: &[Capability]) -> Vec<&'static ComposerAction> {
    COMPOSER_ACTIONS
        .iter()
        .filter(|a| a.requires.is_none_or(|c| capabilities.contains(&c)))
        .collect()
}

/// Actions in one group, for a surface drawing sections.
pub fn in_group(group: ComposerGroup) -> Vec<&'static ComposerAction> {
    COMPOSER_ACTIONS.iter().filter(|a| a.group == group).collect()
}

/// Resolve a canonical slug or any vendor spelling to its action.
///
/// Case- and whitespace-insensitive, because this answers what somebody typed
/// or what another product called it — never a wire format we control.
pub fn resolve(name: &str) -> Option<&'static ComposerAction> {
    let needle = name.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return None;
    }
    COMPOSER_ACTIONS.iter().find(|a| {
        a.slug.eq_ignore_ascii_case(&needle)
            || a.label.eq_ignore_ascii_case(&needle)
            || a.aliases.iter().any(|alias| alias.eq_ignore_ascii_case(&needle))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_census_is_unambiguous() {
        // One name, one action. Two actions answering to the same word is how a
        // menu starts doing something other than what the operator asked for.
        let mut names = std::collections::BTreeSet::new();
        for action in COMPOSER_ACTIONS {
            assert!(
                names.insert(action.slug.to_ascii_lowercase()),
                "duplicate slug {}",
                action.slug
            );
            assert!(!action.label.is_empty() && !action.blurb.is_empty());
            assert!(!action.glyph.is_empty());
            for alias in action.aliases {
                assert_eq!(
                    *alias,
                    alias.to_ascii_lowercase(),
                    "aliases are stored lowercased so the match is a comparison, not a guess"
                );
                assert!(
                    names.insert((*alias).to_owned()),
                    "{alias} is claimed twice — resolve() would answer arbitrarily"
                );
            }
        }
    }

    #[test]
    fn every_name_resolves_back_to_its_action() {
        for action in COMPOSER_ACTIONS {
            assert_eq!(resolve(action.slug), Some(action), "slug");
            assert_eq!(resolve(action.label), Some(action), "label");
            for alias in action.aliases {
                assert_eq!(resolve(alias), Some(action), "alias {alias}");
            }
            // The whole reason for case-insensitivity: menus title-case, people
            // do not.
            assert_eq!(resolve(&action.slug.to_ascii_uppercase()), Some(action));
            assert_eq!(resolve(&format!("  {}  ", action.slug)), Some(action));
        }
    }

    #[test]
    fn both_vendors_reach_the_same_action() {
        // The point of the aliases, pinned. If either product renames something,
        // this is the test that should fail.
        for (chatgpt, claude) in [
            ("Add photos & files", "Add files or photos"),
            ("Deep research", "Research"),
            ("Connect apps", "Add connectors"),
            ("Artifacts", "Canvas"),
            ("Study and learn", "Use style"),
        ] {
            let a = resolve(chatgpt).unwrap_or_else(|| panic!("{chatgpt} unresolved"));
            let b = resolve(claude).unwrap_or_else(|| panic!("{claude} unresolved"));
            assert_eq!(a, b, "{chatgpt} and {claude} are one action");
        }
    }

    #[test]
    fn an_unknown_name_is_none_not_a_near_miss() {
        assert_eq!(resolve("teleport"), None);
        assert_eq!(resolve(""), None);
        assert_eq!(resolve("   "), None);
        // Prefixes never match — "web" is not "web search", and offering the
        // wrong mode is worse than offering none.
        assert_eq!(resolve("web"), None);
        assert_eq!(resolve("attach-file"), None);
    }

    #[test]
    fn the_menu_offers_only_what_the_backing_declared() {
        // A provider with nothing declared still gets the actions that are ours.
        let bare = offered_for(&[]);
        assert!(bare.iter().all(|a| a.requires.is_none()));
        assert!(bare.iter().any(|a| a.slug == "add-to-project"));
        assert!(
            !bare.iter().any(|a| a.slug == "create-image"),
            "images are a declared capability, never an assumption"
        );

        let with_images = offered_for(&[Capability::Images]);
        assert!(with_images.iter().any(|a| a.slug == "create-image"));
        assert!(!with_images.iter().any(|a| a.slug == "attach-files"));

        let everything: Vec<_> = COMPOSER_ACTIONS.iter().filter_map(|a| a.requires).collect();
        assert_eq!(offered_for(&everything).len(), COMPOSER_ACTIONS.len());
    }

    #[test]
    fn every_group_has_entries_and_every_action_has_a_group() {
        for group in ComposerGroup::ALL {
            assert!(
                !in_group(group).is_empty(),
                "{} renders as an empty section",
                group.label()
            );
        }
        let counted: usize = ComposerGroup::ALL.iter().map(|g| in_group(*g).len()).sum();
        assert_eq!(
            counted,
            COMPOSER_ACTIONS.len(),
            "an action outside ALL would be invisible to a surface drawing sections"
        );
    }
}
