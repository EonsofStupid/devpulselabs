//! Provider-neutral Chat identities and state contracts.
//!
//! This crate deliberately knows nothing about egui, provider processes,
//! filesystem discovery, or transport protocols.

use std::fmt;

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
pub enum ActivationState {
    NotInstalled { reason: String, action: String },
    VersionUnsupported { reason: String, action: String },
    SignedOut { reason: String, action: String },
    SigningIn,
    Ready,
    Degraded { reason: String, action: String },
    Experimental { reason: String, action: String },
    Blocked { reason: String, action: String },
    Failed { reason: String, action: String },
    Disabled { reason: String, action: String },
}

impl ActivationState {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::NotInstalled { .. } => "Not installed",
            Self::VersionUnsupported { .. } => "Version unsupported",
            Self::SignedOut { .. } => "Signed out",
            Self::SigningIn => "Signing in",
            Self::Ready => "Ready",
            Self::Degraded { .. } => "Degraded",
            Self::Experimental { .. } => "Experimental",
            Self::Blocked { .. } => "Blocked",
            Self::Failed { .. } => "Failed",
            Self::Disabled { .. } => "Disabled",
        }
    }

    pub const fn can_send(&self) -> bool {
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
            Self::SigningIn | Self::Ready => None,
        }
    }

    pub fn action(&self) -> Option<&str> {
        match self {
            Self::NotInstalled { action, .. }
            | Self::VersionUnsupported { action, .. }
            | Self::SignedOut { action, .. }
            | Self::Degraded { action, .. }
            | Self::Experimental { action, .. }
            | Self::Blocked { action, .. }
            | Self::Failed { action, .. }
            | Self::Disabled { action, .. } => Some(action),
            Self::SigningIn | Self::Ready => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSummary {
    pub display_name: String,
    pub plan: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSummary {
    pub id: ChatSessionId,
    pub provider_id: ProviderId,
    pub provider_session_id: Option<ProviderSessionId>,
    pub title: String,
    pub workspace: String,
    pub state: SessionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatEventContent {
    SessionStarted,
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
    UsageUpdated {
        summary: String,
    },
    TurnCompleted,
    TurnInterrupted,
    TurnFailed {
        message: String,
    },
    ProviderWarning(String),
    Unknown {
        schema: String,
        redacted_summary: String,
    },
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
    fn only_ready_and_explicit_experimental_states_can_send() {
        assert!(ActivationState::Ready.can_send());
        assert!(ActivationState::Experimental {
            reason: "fixture".into(),
            action: "inspect".into(),
        }
        .can_send());
        assert!(!ActivationState::Blocked {
            reason: "not qualified".into(),
            action: "read the plan".into(),
        }
        .can_send());
    }

    #[test]
    fn unknown_events_remain_first_class() {
        let content = ChatEventContent::Unknown {
            schema: "fixture.v2".into(),
            redacted_summary: "new provider item".into(),
        };
        assert!(matches!(content, ChatEventContent::Unknown { .. }));
    }
}
