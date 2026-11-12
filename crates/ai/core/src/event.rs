//! What a provider emits into a conversation.

use crate::usage::TokenUsage;

pub const MAX_NATIVE_PAYLOAD_BYTES: usize = 8 * 1024;

/// A provider-native diagnostic projection with a hard byte ceiling.
///
/// The value is expected to be redacted before construction. This type keeps
/// unknown provider data discoverable without retaining an unbounded raw
/// record in application state.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

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
