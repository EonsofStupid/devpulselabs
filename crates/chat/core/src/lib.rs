//! Provider-neutral CONVERSATION state — sessions, turns, events, and the
//! content view Chat reads.
//!
//! The AI contract — identity, capability, presence, activation, and guided
//! values — moved to `dpl-ai-core` at the kernel boundary. What stays here is
//! chat's alone. See `docs/plans/kernel-boundary.md`.

use dpl_ai_core::{
    GuidedValue, GuidedValueConfidence, GuidedValueSensitivity, GuidedValueSource, ProviderId,
};
use std::fmt;

pub const MAX_NATIVE_PAYLOAD_BYTES: usize = 8 * 1024;

/// Stable top-level Chat modes. Each domain owns independent selection,
/// draft, scroll, unread, and overlay state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum ChatDomain {
    #[default]
    Ai,
    /// The operator's content — shows, episodes, and their slide spines.
    /// Its own domain because it is neither AI's nor Social's: Chat is the
    /// point of operations for the content, not a place content is filed
    /// under a provider.
    Content,
    Social,
}

impl ChatDomain {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ai => "AI",
            Self::Content => "Content",
            Self::Social => "Social",
        }
    }
}

/// The address of one episode in the seeds+content union.
///
/// The grammar is the harness grammar the Workbench already accepts:
/// `"<show>/<episode>"` for operator content, `"template/<episode>"` for
/// a seed. A runtime value, so NOT a [`static_id`] — those mint
/// `&'static str` identities.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentEpisodeId(pub String);

impl fmt::Display for ContentEpisodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One episode as Chat sees it — a census row, no spine loaded yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentEpisode {
    pub id: ContentEpisodeId,
    /// `None` = a seed template rather than operator content.
    pub show: Option<String>,
    pub title: String,
}

/// One slide inside a loaded spine. A `Slide` document has no title of
/// its own, so `name` is its folder — the deck's own convention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentSlide {
    pub chapter: String,
    pub name: String,
    pub template: String,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentChapter {
    pub id: String,
    pub title: String,
    pub slides: Vec<ContentSlide>,
}

/// One episode opened for inspection — the chapter→slide structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentSpine {
    pub id: ContentEpisodeId,
    pub title: String,
    pub chapters: Vec<ContentChapter>,
}

impl ContentSpine {
    /// Every slide in chapter order — the flat walk, for counting and for
    /// surfaces that do not group.
    pub fn slides(&self) -> impl Iterator<Item = &ContentSlide> {
        self.chapters.iter().flat_map(|chapter| &chapter.slides)
    }
}

/// Everything Chat knows about the operator's content.
///
/// `error` is a FIELD, not a panic path: an unreadable content root is a
/// truthful state Chat displays, exactly as an unavailable provider is.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContentView {
    pub episodes: Vec<ContentEpisode>,
    pub spine: Option<ContentSpine>,
    pub error: Option<String>,
}

/// DevPULSE-owned identity for a session, independent of provider-native IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChatSessionId(pub u64);

impl fmt::Display for ChatSessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chat-{}", self.0)
    }
}

/// DevPULSE-owned identity for an event inside one normalized history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChatEventId(pub u64);

impl fmt::Display for ChatEventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "event-{}", self.0)
    }
}

/// Provider-native identities remain typed even though their payload is owned.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProviderSessionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TurnId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    ReadOnly,
    Idle,
    Running,
    Attention,
    Failed,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SessionGroup {
    Pinned,
    Active,
    Recent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    pub id: ChatSessionId,
    pub provider_id: ProviderId,
    pub provider_session_id: Option<ProviderSessionId>,
    pub title: String,
    pub workspace: String,
    pub group: SessionGroup,
    pub state: SessionState,
}

/// A provider-native diagnostic projection with a hard byte ceiling.
///
/// The value is expected to be redacted before construction. This type keeps
/// unknown provider data discoverable without retaining an unbounded raw
/// record in application state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedNativePayload {
    pub schema: String,
    pub redacted_summary: String,
    pub original_bytes: usize,
    pub truncated: bool,
}

impl BoundedNativePayload {
    pub fn redacted(schema: impl Into<String>, summary: impl AsRef<str>) -> Self {
        Self::redacted_with_limit(schema, summary, MAX_NATIVE_PAYLOAD_BYTES)
    }

    pub fn redacted_with_limit(
        schema: impl Into<String>,
        summary: impl AsRef<str>,
        max_bytes: usize,
    ) -> Self {
        let summary = summary.as_ref();
        let original_bytes = summary.len();
        let limit = max_bytes.min(original_bytes);
        let mut boundary = limit;
        while boundary > 0 && !summary.is_char_boundary(boundary) {
            boundary -= 1;
        }
        Self {
            schema: schema.into(),
            redacted_summary: summary[..boundary].to_owned(),
            original_bytes,
            truncated: original_bytes > boundary,
        }
    }
}

/// Structured token accounting from a provider's usage report.
///
/// Every field is optional because providers report DIFFERENT things:
/// Codex reports an input/output split, Grok reports context-window
/// occupancy. A `None` means "this provider did not say", never zero —
/// the distinction matters, because zero is a measurement and absent is
/// not, and cost computed from a guessed zero would be a lie.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenUsage {
    /// FRESH input — tokens the provider actually processed anew.
    pub input_tokens: Option<u64>,
    /// Tokens served from cache. Billed at a small fraction of fresh
    /// input, so they MUST stay separate: folding them into `input`
    /// makes cost an overestimate, and the split cannot be recovered
    /// afterwards from a folded number.
    pub cache_read_tokens: Option<u64>,
    /// Tokens written INTO the cache — billed above fresh input.
    pub cache_write_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    /// Context-window occupancy, when the provider reports it that way.
    pub context_used: Option<u64>,
    pub context_size: Option<u64>,
}

impl TokenUsage {
    pub const fn is_empty(self) -> bool {
        self.input_tokens.is_none()
            && self.cache_read_tokens.is_none()
            && self.cache_write_tokens.is_none()
            && self.output_tokens.is_none()
            && self.total_tokens.is_none()
            && self.context_used.is_none()
            && self.context_size.is_none()
    }

    /// Accumulate per-turn reports into a session total. Absent stays
    /// absent; a present value adds to whatever came before, so summing
    /// never invents a number out of two absences.
    pub fn accumulate(self, turn: Self) -> Self {
        fn plus(a: Option<u64>, b: Option<u64>) -> Option<u64> {
            match (a, b) {
                (Some(a), Some(b)) => Some(a.saturating_add(b)),
                (Some(v), None) | (None, Some(v)) => Some(v),
                (None, None) => None,
            }
        }
        Self {
            input_tokens: plus(self.input_tokens, turn.input_tokens),
            cache_read_tokens: plus(self.cache_read_tokens, turn.cache_read_tokens),
            cache_write_tokens: plus(self.cache_write_tokens, turn.cache_write_tokens),
            output_tokens: plus(self.output_tokens, turn.output_tokens),
            total_tokens: plus(self.total_tokens, turn.total_tokens),
            // Context occupancy is a LEVEL, never a sum — the latest
            // reading is the truth, adding them would be meaningless.
            context_used: turn.context_used.or(self.context_used),
            context_size: turn.context_size.or(self.context_size),
        }
    }

    /// Later reports supersede earlier ones field-by-field, so a provider
    /// that reports input now and output later still yields one whole
    /// picture at the end of the turn.
    pub fn merge(self, newer: Self) -> Self {
        Self {
            input_tokens: newer.input_tokens.or(self.input_tokens),
            cache_read_tokens: newer.cache_read_tokens.or(self.cache_read_tokens),
            cache_write_tokens: newer.cache_write_tokens.or(self.cache_write_tokens),
            output_tokens: newer.output_tokens.or(self.output_tokens),
            total_tokens: newer.total_tokens.or(self.total_tokens),
            context_used: newer.context_used.or(self.context_used),
            context_size: newer.context_size.or(self.context_size),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEventContent {
    SessionStarted,
    /// The provider named the model it is actually running. Emitted from
    /// the provider's own session-configured fact — never guessed.
    ModelSelected {
        model: String,
    },
    UserMessage(String),
    AssistantMessage(String),
    PlanUpdated(String),
    StatusUpdated(String),
    ToolStarted {
        label: String,
    },
    ToolCompleted {
        label: String,
        succeeded: bool,
    },
    DiffUpdated(String),
    ApprovalRequested {
        summary: String,
    },
    ApprovalResolved {
        accepted: bool,
    },
    /// `summary` is the human line the activity feed shows; `usage` is the
    /// machine truth the AI domain records. Prose was all this carried
    /// before — you cannot aggregate a sentence.
    UsageUpdated {
        summary: String,
        usage: TokenUsage,
    },
    TurnCompleted,
    TurnInterrupted,
    TurnFailed {
        message: String,
    },
    ProviderWarning(String),
    Unknown(BoundedNativePayload),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChatEventKind {
    SessionStarted,
    ModelSelected,
    UserMessage,
    AssistantMessage,
    PlanUpdated,
    StatusUpdated,
    ToolStarted,
    ToolCompleted,
    DiffUpdated,
    ApprovalRequested,
    ApprovalResolved,
    UsageUpdated,
    TurnCompleted,
    TurnInterrupted,
    TurnFailed,
    ProviderWarning,
    Unknown,
}

/// Where a normalized provider event is allowed to appear.
///
/// Every event remains in history. This classification prevents protocol
/// activity and bounded native diagnostics from masquerading as messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPresentation {
    Conversation,
    Activity,
    Diagnostics,
}

impl ChatEventContent {
    pub const fn kind(&self) -> ChatEventKind {
        match self {
            Self::SessionStarted => ChatEventKind::SessionStarted,
            Self::ModelSelected { .. } => ChatEventKind::ModelSelected,
            Self::UserMessage(_) => ChatEventKind::UserMessage,
            Self::AssistantMessage(_) => ChatEventKind::AssistantMessage,
            Self::PlanUpdated(_) => ChatEventKind::PlanUpdated,
            Self::StatusUpdated(_) => ChatEventKind::StatusUpdated,
            Self::ToolStarted { .. } => ChatEventKind::ToolStarted,
            Self::ToolCompleted { .. } => ChatEventKind::ToolCompleted,
            Self::DiffUpdated(_) => ChatEventKind::DiffUpdated,
            Self::ApprovalRequested { .. } => ChatEventKind::ApprovalRequested,
            Self::ApprovalResolved { .. } => ChatEventKind::ApprovalResolved,
            Self::UsageUpdated { .. } => ChatEventKind::UsageUpdated,
            Self::TurnCompleted => ChatEventKind::TurnCompleted,
            Self::TurnInterrupted => ChatEventKind::TurnInterrupted,
            Self::TurnFailed { .. } => ChatEventKind::TurnFailed,
            Self::ProviderWarning(_) => ChatEventKind::ProviderWarning,
            Self::Unknown(_) => ChatEventKind::Unknown,
        }
    }

    pub const fn presentation(&self) -> EventPresentation {
        match self {
            Self::SessionStarted
            | Self::ModelSelected { .. }
            | Self::StatusUpdated(_)
            | Self::TurnCompleted
            | Self::TurnInterrupted => EventPresentation::Activity,
            Self::Unknown(_) => EventPresentation::Diagnostics,
            Self::UserMessage(_)
            | Self::AssistantMessage(_)
            | Self::PlanUpdated(_)
            | Self::ToolStarted { .. }
            | Self::ToolCompleted { .. }
            | Self::DiffUpdated(_)
            | Self::ApprovalRequested { .. }
            | Self::ApprovalResolved { .. }
            | Self::UsageUpdated { .. }
            | Self::TurnFailed { .. }
            | Self::ProviderWarning(_) => EventPresentation::Conversation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatEvent {
    pub id: ChatEventId,
    pub provider_id: ProviderId,
    pub session_id: ChatSessionId,
    pub turn_id: Option<TurnId>,
    pub content: ChatEventContent,
}

// ── the command layer (BND1: the boundary between chat and AI) ──────────
//
// The window RECOGNIZES and RENDERS commands; this module is their
// VOCABULARY — census, recognition, argument validation, guided seeds.
// The host DISPATCHES. Nothing else enumerates commands, anywhere.

/// What kind of value one command argument takes — each kind names the
/// census its values are validated against. Out-of-census values refuse
/// by naming what IS registered; free text never validates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandArgKind {
    /// A `hub/sub` route from the platform's live route census.
    RoutePick,
    /// A registered provider id from the roster.
    ProviderPick,
    /// A registered workflow id from the workflow census.
    WorkflowPick,
    /// Free text (validated non-empty only when required).
    Text,
}

/// One argument a command takes, in ask order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandArg {
    pub id: &'static str,
    /// The ask, in the operator's language — the strip shows this.
    pub ask: &'static str,
    pub kind: CommandArgKind,
    pub required: bool,
}

/// One registered operator command. `privileged` commands route through
/// the determinism gate's audited mint site and refuse synthetic input
/// BEFORE the gate is touched — the registry never weakens that law.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    /// The `/slug` (without the slash), kebab.
    pub slug: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    /// Whole-buffer word aliases that also trigger (provenance law:
    /// exact, trimmed, case-insensitive — a word in a sentence never
    /// escalates).
    pub aliases: &'static [&'static str],
    pub args: &'static [CommandArg],
    pub privileged: bool,
}

/// THE command census — one entry per command, nothing else enumerates.
pub fn commands() -> Vec<CommandSpec> {
    vec![
        CommandSpec {
            slug: "route",
            label: "Go to route",
            blurb: "Open any hub/sub route in the main window — loaded live from the platform.",
            aliases: &[],
            args: &[CommandArg {
                id: "route",
                ask: "Where to? (hub/sub)",
                kind: CommandArgKind::RoutePick,
                required: true,
            }],
            privileged: false,
        },
        CommandSpec {
            slug: "model",
            label: "Clyffy backing",
            blurb: "Pick which signed-in provider powers the Clyffy session.",
            aliases: &[],
            args: &[CommandArg {
                id: "provider",
                ask: "Which provider?",
                kind: CommandArgKind::ProviderPick,
                required: true,
            }],
            privileged: false,
        },
        CommandSpec {
            slug: "routines",
            label: "Workflows",
            blurb: "List the registered workflows and open the Workflows tab.",
            aliases: &[],
            args: &[],
            privileged: false,
        },
        CommandSpec {
            slug: "brainstorm",
            label: "Brainstorm board",
            blurb: "Open the phased brainstorm board right here.",
            aliases: &["brainstorm", "brainstorming"],
            args: &[],
            privileged: false,
        },
        CommandSpec {
            slug: "strict",
            label: "Strict determinism",
            blurb: "Demand strict determinism (operator-audited).",
            aliases: &[],
            args: &[],
            privileged: true,
        },
        CommandSpec {
            slug: "behavioral",
            label: "Behavioral determinism",
            blurb: "Return to behavioral determinism.",
            aliases: &[],
            args: &[],
            privileged: true,
        },
    ]
}

/// Look one command up by slug.
pub fn command(slug: &str) -> Option<CommandSpec> {
    commands().into_iter().find(|c| c.slug == slug)
}

/// A recognized-at-submit command plus whatever argument text rode the
/// buffer (`/route library/templates` → slug "route", rest
/// "library/templates"). Argument VALIDATION happens later, against the
/// live censuses the host owns — recognition never guesses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandDraft {
    pub slug: String,
    pub rest: String,
}

/// Recognize a command from the operator's submit buffer — the ONE
/// recognizer (provenance law verbatim): the trimmed buffer must START
/// with `/slug` (followed by nothing or whitespace-separated args), or
/// exactly equal a registered alias word. `/strict` buried in a
/// sentence is just text; an unknown `/x` is just text.
pub fn recognize(draft: &str) -> Option<CommandDraft> {
    let trimmed = draft.trim();
    if let Some(body) = trimmed.strip_prefix('/') {
        let (slug, rest) = match body.split_once(char::is_whitespace) {
            Some((slug, rest)) => (slug, rest.trim()),
            None => (body, ""),
        };
        // Case-insensitive like the aliases — "/Strict" is a shift held
        // a beat too long, not a message for the provider.
        let spec = commands()
            .into_iter()
            .find(|c| c.slug.eq_ignore_ascii_case(slug))?;
        return Some(CommandDraft {
            slug: spec.slug.to_owned(),
            rest: rest.to_owned(),
        });
    }
    let spec = commands()
        .into_iter()
        .find(|c| c.aliases.iter().any(|a| trimmed.eq_ignore_ascii_case(a)))?;
    Some(CommandDraft {
        slug: spec.slug.to_owned(),
        rest: String::new(),
    })
}

/// The commands a partially-typed `/` draft offers — the typeahead's
/// ONE source (derived from the census; no second filter table). A
/// draft that already carries whitespace is past the slug and offers
/// nothing; a non-slash draft offers nothing.
pub fn matching_commands(draft: &str) -> Vec<CommandSpec> {
    let trimmed = draft.trim_start();
    let Some(body) = trimmed.strip_prefix('/') else {
        return Vec::new();
    };
    if body.contains(char::is_whitespace) {
        return Vec::new();
    }
    commands()
        .into_iter()
        .filter(|c| c.slug.starts_with(body))
        .collect()
}

/// The live censuses argument validation runs against. The HOST fills
/// this (it owns the route table, the roster, the workflow census);
/// core never reaches for them itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandCensuses {
    /// Every `hub/sub` route, in rail order.
    pub routes: Vec<String>,
    /// Every registered provider id.
    pub providers: Vec<String>,
    /// Every registered workflow id.
    pub workflows: Vec<String>,
}

/// One named argument fault — `path` is the arg id, `message` names
/// what IS registered (the census-naming refusal law).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandIssue {
    pub path: String,
    pub message: String,
}

/// Validate a draft's argument text against the spec and the live
/// censuses. Refuses with EVERY fault named; a refusal changes nothing.
pub fn validate_command(
    draft: &CommandDraft,
    censuses: &CommandCensuses,
) -> Result<(), Vec<CommandIssue>> {
    let Some(spec) = command(&draft.slug) else {
        return Err(vec![CommandIssue {
            path: "slug".into(),
            message: format!(
                "unknown command {:?} — registered: {}",
                draft.slug,
                commands()
                    .iter()
                    .map(|c| c.slug)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }]);
    };
    let mut issues = Vec::new();
    let value = draft.rest.trim();
    if spec.args.is_empty() && !value.is_empty() {
        issues.push(CommandIssue {
            path: "rest".into(),
            message: format!("/{} takes no arguments", spec.slug),
        });
    }
    for arg in spec.args {
        if value.is_empty() {
            if arg.required {
                issues.push(CommandIssue {
                    path: arg.id.into(),
                    message: format!("{} — this argument is required", arg.ask),
                });
            }
            continue;
        }
        let (census, what): (&[String], &str) = match arg.kind {
            CommandArgKind::RoutePick => (&censuses.routes, "route"),
            CommandArgKind::ProviderPick => (&censuses.providers, "provider"),
            CommandArgKind::WorkflowPick => (&censuses.workflows, "workflow"),
            CommandArgKind::Text => continue,
        };
        if !census.iter().any(|c| c == value) {
            issues.push(CommandIssue {
                path: arg.id.into(),
                message: format!(
                    "unknown {what} {value:?} — registered: {}",
                    census.join(", ")
                ),
            });
        }
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// Seed one argument's value with provenance — the guided-value law:
/// the platform proposes, the operator disposes. The host supplies the
/// seed text from its live state (current route, selected provider).
pub fn seed_arg(kind: CommandArgKind, seed: String) -> GuidedValue<String> {
    let source = match kind {
        CommandArgKind::RoutePick => GuidedValueSource::Route,
        CommandArgKind::ProviderPick => GuidedValueSource::Provider,
        CommandArgKind::WorkflowPick | CommandArgKind::Text => GuidedValueSource::Session,
    };
    GuidedValue::new(
        seed,
        source,
        GuidedValueConfidence::Suggested(70),
        true,
        true,
        GuidedValueSensitivity::Public,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    // Capability moved to dpl-ai-core at the kernel boundary; the slug
    // round-trip proof stays here because chat is what persists the slugs.
    use dpl_ai_core::Capability;
    #[test]
    fn typed_session_identity_has_a_stable_display_form() {
        assert_eq!(ChatSessionId(42).to_string(), "chat-42");
    }
    #[test]
    fn ai_is_the_default_chat_domain() {
        assert_eq!(ChatDomain::default(), ChatDomain::Ai);
        assert_eq!(ChatDomain::Social.label(), "Social");
    }
    #[test]
    fn content_is_a_domain_with_its_own_label() {
        assert_eq!(ChatDomain::Content.label(), "Content");
        assert_ne!(ChatDomain::Content, ChatDomain::Ai);
    }
    #[test]
    fn a_spine_walks_its_slides_in_chapter_order() {
        let slide = |chapter: &str, name: &str, index| ContentSlide {
            chapter: chapter.to_owned(),
            name: name.to_owned(),
            template: "static/title".to_owned(),
            index,
        };
        let spine = ContentSpine {
            id: ContentEpisodeId("demo/pilot".to_owned()),
            title: "Pilot".to_owned(),
            chapters: vec![
                ContentChapter {
                    id: "open".to_owned(),
                    title: "Opening".to_owned(),
                    slides: vec![slide("open", "title-card", 0), slide("open", "problem", 1)],
                },
                ContentChapter {
                    id: "close".to_owned(),
                    title: "Closing".to_owned(),
                    slides: vec![slide("close", "wrap", 2)],
                },
            ],
        };
        let walked: Vec<usize> = spine.slides().map(|slide| slide.index).collect();
        assert_eq!(walked, vec![0, 1, 2]);
    }
    #[test]
    fn an_unreadable_content_root_is_state_not_a_panic() {
        let view = ContentView {
            error: Some("content root is unreadable".to_owned()),
            ..ContentView::default()
        };
        assert!(view.episodes.is_empty());
        assert!(view.spine.is_none());
        assert_eq!(view.error.as_deref(), Some("content root is unreadable"));
    }
    #[test]
    fn unknown_events_remain_first_class() {
        let content = ChatEventContent::Unknown(BoundedNativePayload::redacted(
            "fixture.v2",
            "new provider item",
        ));
        assert!(matches!(content, ChatEventContent::Unknown(_)));
        assert_eq!(content.presentation(), EventPresentation::Diagnostics);
    }
    #[test]
    fn protocol_activity_never_becomes_conversation_content() {
        assert_eq!(
            ChatEventContent::StatusUpdated("thinking".into()).presentation(),
            EventPresentation::Activity
        );
        assert_eq!(
            ChatEventContent::TurnCompleted.presentation(),
            EventPresentation::Activity
        );
        assert_eq!(
            ChatEventContent::AssistantMessage("answer".into()).presentation(),
            EventPresentation::Conversation
        );
    }
    #[test]
    fn native_payloads_are_utf8_safe_and_bounded() {
        let payload = BoundedNativePayload::redacted_with_limit("future", "ééé", 5);
        assert_eq!(payload.redacted_summary, "éé");
        assert_eq!(payload.original_bytes, 6);
        assert!(payload.truncated);
    }

    #[test]
    fn every_capability_slug_round_trips_and_is_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for capability in Capability::ALL {
            let slug = capability.slug();
            assert!(
                seen.insert(slug),
                "{slug} is used by two capabilities — a stored set would be ambiguous"
            );
            assert_eq!(
                Capability::from_slug(slug),
                Some(capability),
                "{slug} must resolve back to the capability that wrote it"
            );
        }
        assert_eq!(
            seen.len(),
            Capability::ALL.len(),
            "ALL must list every variant — it is what makes this proof exhaustive"
        );
    }

    #[test]
    fn an_unknown_capability_slug_is_dropped_not_invented() {
        assert_eq!(Capability::from_slug("teleportation"), None);
        assert_eq!(
            Capability::from_slug("read sessions"),
            None,
            "the human label is not the wire name — only the slug resolves"
        );
    }

    #[test]
    fn the_command_registry_is_the_census() {
        let all = commands();
        assert_eq!(
            all.len(),
            6,
            "route + model + routines + brainstorm + strict + behavioral (BND1 v1)"
        );
        let mut slugs = std::collections::BTreeSet::new();
        for c in &all {
            assert!(slugs.insert(c.slug), "duplicate command slug {}", c.slug);
            assert!(!c.label.is_empty() && !c.blurb.is_empty());
        }
        assert!(command("route").is_some());
        assert!(command("ghost").is_none());
        let privileged: Vec<_> = all
            .iter()
            .filter(|c| c.privileged)
            .map(|c| c.slug)
            .collect();
        assert_eq!(
            privileged,
            ["strict", "behavioral"],
            "only the determinism pair is privileged"
        );
    }

    #[test]
    fn recognition_is_whole_buffer_and_never_escalates_patterns() {
        assert_eq!(recognize("/route").unwrap().slug, "route");
        assert_eq!(
            recognize("  /route library/templates  ").unwrap(),
            CommandDraft {
                slug: "route".into(),
                rest: "library/templates".into(),
            }
        );
        assert_eq!(recognize("Brainstorming").unwrap().slug, "brainstorm");
        assert_eq!(
            recognize("/Strict").unwrap().slug,
            "strict",
            "a held shift is not a message for the provider"
        );
        assert!(
            recognize("please /route home now").is_none(),
            "pattern-in-string"
        );
        assert!(recognize("/router").is_none(), "prefix never matches");
        assert!(recognize("/x unknown stays text").is_none());
        assert!(recognize("let's do some brainstorming").is_none());
        assert!(recognize("").is_none());
    }

    #[test]
    fn out_of_census_arguments_refuse_naming_the_census() {
        let censuses = CommandCensuses {
            routes: vec!["home/shows".into(), "library/templates".into()],
            providers: vec!["codex".into()],
            workflows: vec!["storyboard".into()],
        };
        let bad = CommandDraft {
            slug: "route".into(),
            rest: "library/ghost".into(),
        };
        let issues = validate_command(&bad, &censuses).unwrap_err();
        assert!(
            issues[0].message.contains("library/templates"),
            "names the census"
        );
        let missing = CommandDraft {
            slug: "route".into(),
            rest: String::new(),
        };
        assert!(
            validate_command(&missing, &censuses).is_err(),
            "required arg"
        );
        let good = CommandDraft {
            slug: "route".into(),
            rest: "home/shows".into(),
        };
        assert!(validate_command(&good, &censuses).is_ok());
        let argless = CommandDraft {
            slug: "routines".into(),
            rest: String::new(),
        };
        assert!(validate_command(&argless, &censuses).is_ok());
    }

    #[test]
    fn the_typeahead_offers_only_census_commands() {
        let slugs = |d: &str| {
            matching_commands(d)
                .iter()
                .map(|c| c.slug)
                .collect::<Vec<_>>()
        };
        assert_eq!(slugs("/r"), ["route", "routines"], "registry order");
        assert_eq!(slugs("/").len(), 6, "a bare slash offers the census");
        assert!(slugs("/x").is_empty(), "unknown prefix offers nothing");
        assert!(
            slugs("/route lib").is_empty(),
            "past the slug, the strip yields"
        );
        assert!(slugs("hello").is_empty());
        assert!(slugs("").is_empty());
    }

    #[test]
    fn seeds_carry_provenance_and_yield_to_the_operator() {
        let mut seed = seed_arg(CommandArgKind::RoutePick, "home/shows".into());
        assert_eq!(seed.source, GuidedValueSource::Route);
        assert!(matches!(
            seed.confidence,
            GuidedValueConfidence::Suggested(_)
        ));
        assert!(seed.replace_by_operator("library/templates".into()));
        assert_eq!(seed.source, GuidedValueSource::Operator);
        assert!(seed.overridden);
    }
}
