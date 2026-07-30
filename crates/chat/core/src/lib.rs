//! Provider-neutral Chat identities and state contracts.
//!
//! This crate deliberately knows nothing about egui, provider processes,
//! filesystem discovery, or transport protocols.

use std::fmt;

pub const MAX_NATIVE_PAYLOAD_BYTES: usize = 8 * 1024;

/// Stable top-level Chat modes. Each domain owns independent selection,
/// draft, scroll, unread, and overlay state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum ChatDomain {
    #[default]
    Ai,
    Social,
}

impl ChatDomain {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ai => "AI",
            Self::Social => "Social",
        }
    }
}

macro_rules! static_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub &'static str);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.0)
            }
        }
    };
}

static_id!(ProviderId);
static_id!(ProviderProfileId);

/// Where a guided value came from. The source is retained so authored UI can
/// distinguish provider truth from operator input or an AI suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuidedValueSource {
    Provider,
    Route,
    Session,
    Operator,
    Ai,
}

/// How strongly a consumer may rely on a seeded value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuidedValueConfidence {
    Authoritative,
    Suggested(u8),
}

/// Whether a guided value may cross diagnostics or persistence boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuidedValueSensitivity {
    Public,
    Private,
    TransientSecret,
}

/// A pre-populated value with explicit provenance and operator policy.
///
/// This is the boring foundation for route-aware defaults and AI suggestions.
/// Consumers never need to infer whether a value is editable, copyable, or
/// safe to persist from its contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidedValue<T> {
    pub value: T,
    pub source: GuidedValueSource,
    pub confidence: GuidedValueConfidence,
    pub editable: bool,
    pub copyable: bool,
    pub sensitivity: GuidedValueSensitivity,
    pub overridden: bool,
}

impl<T> GuidedValue<T> {
    pub const fn new(
        value: T,
        source: GuidedValueSource,
        confidence: GuidedValueConfidence,
        editable: bool,
        copyable: bool,
        sensitivity: GuidedValueSensitivity,
    ) -> Self {
        Self {
            value,
            source,
            confidence,
            editable,
            copyable,
            sensitivity,
            overridden: false,
        }
    }

    pub const fn provider_authoritative(
        value: T,
        copyable: bool,
        sensitivity: GuidedValueSensitivity,
    ) -> Self {
        Self::new(
            value,
            GuidedValueSource::Provider,
            GuidedValueConfidence::Authoritative,
            false,
            copyable,
            sensitivity,
        )
    }

    pub fn replace_by_operator(&mut self, value: T) -> bool {
        if !self.editable {
            return false;
        }
        self.value = value;
        self.source = GuidedValueSource::Operator;
        self.confidence = GuidedValueConfidence::Authoritative;
        self.overridden = true;
        true
    }
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
pub enum ProviderAccent {
    Amber,
    Cyan,
    Violet,
    Magenta,
    Lime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub label: &'static str,
    pub short_label: &'static str,
    pub glyph: &'static str,
    pub accent: ProviderAccent,
    pub summary: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    ReadSessions,
    PageHistory,
    LiveTail,
    StartSession,
    ResumeSession,
    ForkSession,
    StreamTurns,
    InterruptTurn,
    SteerTurn,
    Approvals,
    Tools,
    Diffs,
    Plans,
    Attachments,
    Images,
    Video,
    Models,
    Modes,
    RateLimits,
    SubscriptionAuth,
    DeviceCodeAuth,
    Headless,
}

impl Capability {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadSessions => "read sessions",
            Self::PageHistory => "paged history",
            Self::LiveTail => "live tail",
            Self::StartSession => "start session",
            Self::ResumeSession => "resume session",
            Self::ForkSession => "fork session",
            Self::StreamTurns => "stream turns",
            Self::InterruptTurn => "interrupt turn",
            Self::SteerTurn => "steer turn",
            Self::Approvals => "approvals",
            Self::Tools => "tools",
            Self::Diffs => "diffs",
            Self::Plans => "plans",
            Self::Attachments => "attachments",
            Self::Images => "images",
            Self::Video => "video",
            Self::Models => "models",
            Self::Modes => "modes",
            Self::RateLimits => "rate limits",
            Self::SubscriptionAuth => "subscription auth",
            Self::DeviceCodeAuth => "device-code auth",
            Self::Headless => "headless",
        }
    }
}

/// Capabilities a provider intends to qualify. These are not active merely
/// because they are present in a manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderManifest {
    pub descriptor: ProviderDescriptor,
    pub profile: ProviderProfileId,
    pub target_capabilities: &'static [Capability],
    pub delivery_slice: &'static str,
    pub evidence: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAuthorizationPrompt {
    pub provider_id: ProviderId,
    pub verification_url: GuidedValue<String>,
    pub user_code: GuidedValue<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationState {
    NotInstalled {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    VersionUnsupported {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    SignedOut {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    SigningIn,
    AwaitingAuthorization(DeviceAuthorizationPrompt),
    Verifying,
    Ready,
    Degraded {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    Experimental {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    Blocked {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    Failed {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
    Disabled {
        reason: String,
        recovery: Vec<RecoveryAction>,
    },
}

impl ActivationState {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::NotInstalled { .. } => "Not installed",
            Self::VersionUnsupported { .. } => "Version unsupported",
            Self::SignedOut { .. } => "Signed out",
            Self::SigningIn => "Signing in",
            Self::AwaitingAuthorization(_) => "Authorize in browser",
            Self::Verifying => "Verifying",
            Self::Ready => "Ready",
            Self::Degraded { .. } => "Degraded",
            Self::Experimental { .. } => "Experimental",
            Self::Blocked { .. } => "Blocked",
            Self::Failed { .. } => "Failed",
            Self::Disabled { .. } => "Disabled",
        }
    }

    pub const fn is_activated(&self) -> bool {
        matches!(self, Self::Ready | Self::Experimental { .. })
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::NotInstalled { reason, .. }
            | Self::VersionUnsupported { reason, .. }
            | Self::SignedOut { reason, .. }
            | Self::Degraded { reason, .. }
            | Self::Experimental { reason, .. }
            | Self::Blocked { reason, .. }
            | Self::Failed { reason, .. }
            | Self::Disabled { reason, .. } => Some(reason),
            Self::SigningIn | Self::AwaitingAuthorization(_) | Self::Verifying | Self::Ready => {
                None
            }
        }
    }

    pub fn recovery(&self) -> &[RecoveryAction] {
        match self {
            Self::NotInstalled { recovery, .. }
            | Self::VersionUnsupported { recovery, .. }
            | Self::SignedOut { recovery, .. }
            | Self::Degraded { recovery, .. }
            | Self::Experimental { recovery, .. }
            | Self::Blocked { recovery, .. }
            | Self::Failed { recovery, .. }
            | Self::Disabled { recovery, .. } => recovery,
            Self::SigningIn | Self::AwaitingAuthorization(_) | Self::Verifying | Self::Ready => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSummary {
    pub display_name: String,
    pub plan: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoveryActionKind {
    InstallInstructions,
    SignIn,
    Retry,
    UpdateProvider,
    OpenDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryAction {
    pub kind: RecoveryActionKind,
    pub label: String,
}

impl RecoveryAction {
    pub fn new(kind: RecoveryActionKind, label: impl Into<String>) -> Self {
        Self {
            kind,
            label: label.into(),
        }
    }
}

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
    fn guided_values_retain_provenance_and_enforce_edit_policy() {
        let mut code = GuidedValue::provider_authoritative(
            "EH38-T38A".to_owned(),
            true,
            GuidedValueSensitivity::TransientSecret,
        );
        assert_eq!(code.source, GuidedValueSource::Provider);
        assert_eq!(code.confidence, GuidedValueConfidence::Authoritative);
        assert!(code.copyable);
        assert!(!code.editable);
        assert!(!code.replace_by_operator("different".to_owned()));
        assert_eq!(code.value, "EH38-T38A");

        let mut suggestion = GuidedValue::new(
            "DevPULSE".to_owned(),
            GuidedValueSource::Ai,
            GuidedValueConfidence::Suggested(82),
            true,
            true,
            GuidedValueSensitivity::Private,
        );
        assert!(suggestion.replace_by_operator("DevPULSE Labs".to_owned()));
        assert_eq!(suggestion.source, GuidedValueSource::Operator);
        assert!(suggestion.overridden);
    }

    #[test]
    fn only_ready_and_explicit_experimental_states_are_activated() {
        assert!(ActivationState::Ready.is_activated());
        assert!(ActivationState::Experimental {
            reason: "fixture".into(),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::OpenDiagnostics,
                "Inspect",
            )],
        }
        .is_activated());
        assert!(!ActivationState::Blocked {
            reason: "not qualified".into(),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::OpenDiagnostics,
                "Read the plan",
            )],
        }
        .is_activated());
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
