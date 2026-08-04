//! Provider-neutral CONVERSATION state — sessions, turns, events, and the
//! content view Chat reads.
//!
//! The AI contract — identity, capability, presence, activation, and guided
//! values — moved to `dpl-ai-core` at the kernel boundary. What stays here is
//! chat's alone. See `docs/plans/kernel-boundary.md`.

use dpl_ai_core::ProviderId;
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

#[cfg(test)]
mod tests {
    use super::*;
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
}
