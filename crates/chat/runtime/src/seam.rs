//! THE PROVIDER RUNTIME SEAM — one shape every live provider speaks.
//!
//! This crate's header carried an open question from CH1: *"CH1 contains no
//! process transport. Live provider ownership is blocked on the Chat
//! provider-runtime ADR."* This module is the answer.
//!
//! ## Why a seam and not a second Grok
//!
//! Grok grew a working runtime (`GrokAcpRuntime`) and Codex did not, so the
//! app ended up with a provider you can talk to and a provider that only
//! knows you are logged in. The instinct is to copy Grok's file. That yields
//! two runtimes with two vocabularies and two sets of signals, and the third
//! provider makes it three.
//!
//! Instead: the vocabulary below is provider-NEUTRAL, and each provider
//! implements [`ProviderRuntime`] over its own transport. The host then drives
//! every provider through one code path and emits ONE set of fabric signals,
//! so an operator's Codex turn and an operator's Grok turn are observable in
//! exactly the same way.
//!
//! Grok's existing types map onto these one-for-one — the vocabulary was
//! already neutral in shape, only Grok-named — so aligning it is a
//! translation, not a rewrite of a working runtime.
//!
//! ## The contract
//!
//! A runtime is a CHANNEL, never a blocking call: commands go in, events come
//! out, and the caller polls. Nothing here may block the UI thread, because
//! every implementation is driven from a frame.

use crate::ProviderId;
use std::fmt;
use std::path::PathBuf;

/// What a connected runtime turned out to support.
///
/// Advertised by the provider at connect time, not assumed. A provider that
/// cannot list sessions is not broken — the surface simply must not offer it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RuntimeCapabilities {
    pub load_session: bool,
    pub list_sessions: bool,
}

/// One session as the PROVIDER describes it.
///
/// Distinct from [`crate::SessionSummary`], which is how the APP describes a
/// session. The host translates; conflating them is how a provider's id
/// vocabulary leaks into the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSessionSummary {
    pub provider_session_id: String,
    pub title: Option<String>,
    pub working_directory: PathBuf,
    pub updated_at: Option<String>,
}

/// A fragment of a turn, as it arrives.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeUpdate {
    UserText(String),
    AssistantText(String),
    ThoughtText(String),
    Tool(String),
    Plan(String),
    SessionInfo(String),
    AvailableCommands {
        count: usize,
    },
    Usage {
        used: u64,
        size: u64,
    },
    /// Anything the adapter recognised as real but has no typed slot for.
    /// Kept rather than dropped — an unknown update is evidence.
    Other(String),
}

/// Why a runtime could not do the thing.
///
/// Typed because the SURFACE reacts differently to each: a credential problem
/// offers sign-in, a launch problem offers install, backpressure is transient
/// and must not look like a failure the operator caused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeFailureKind {
    Launch,
    Protocol,
    CredentialUnavailable,
    Authentication,
    Unsupported,
    InvalidCommand,
    Backpressure,
    Timeout,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFailure {
    pub kind: RuntimeFailureKind,
    pub reason: String,
    pub provider_session_id: Option<String>,
}

impl RuntimeFailure {
    pub fn new(kind: RuntimeFailureKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            reason: reason.into(),
            provider_session_id: None,
        }
    }

    pub fn for_session(
        kind: RuntimeFailureKind,
        reason: impl Into<String>,
        provider_session_id: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            reason: reason.into(),
            provider_session_id: Some(provider_session_id.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Exactly the four the protocol defines. No `Other`: a permission choice
/// the app cannot name is one it must not offer.
pub enum RuntimePermissionOptionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePermissionOption {
    /// Named `option_id` to match `RuntimeCommand::ResolvePermission`, so the
    /// value the operator picks and the value sent back read the same.
    pub option_id: String,
    pub label: String,
    pub kind: RuntimePermissionOptionKind,
}

/// What the operator asked the provider to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeCommand {
    ListSessions {
        cursor: Option<String>,
    },
    NewSession,
    /// Ask what models exist and what each one accepts.
    ListModels,
    LoadSession {
        provider_session_id: String,
    },
    Prompt {
        provider_session_id: String,
        text: String,
        /// The modes standing on the composer when this turn was sent.
        ///
        /// Carried as canonical slugs so an adapter maps ONE vocabulary to its
        /// own wire, and so a mode a provider cannot honour can be named back
        /// to the operator rather than dropped in silence.
        modes: dpl_ai_core::composer::ComposerModes,
    },
    Cancel {
        provider_session_id: String,
    },
    ResolvePermission {
        request_id: u64,
        option_id: Option<String>,
    },
    Shutdown,
}

/// What one model will actually accept, as the provider advertises it.
///
/// Provider-neutral on purpose: every adapter that can describe its models
/// fills this in, and an adapter that cannot leaves it `None` — which is read
/// as "nothing verified", never as "anything goes".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelCapabilities {
    pub id: String,
    pub display_name: String,
    pub is_default: bool,
    /// The reasoning-effort values THIS model advertises. Opaque strings by
    /// design — the protocol calls them "advertised by the model", so a set
    /// hardcoded here would be a guess with a version number on it.
    pub reasoning_efforts: Vec<String>,
    pub default_reasoning_effort: Option<String>,
    pub supports_personality: bool,
}

impl ModelCapabilities {
    /// Does this model advertise the named effort, exactly?
    pub fn advertises_effort(&self, effort: &str) -> bool {
        self.reasoning_efforts.iter().any(|e| e == effort)
    }
}

/// What the provider says back.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeEvent {
    Connected {
        capabilities: RuntimeCapabilities,
    },
    SessionsPage {
        sessions: Vec<RuntimeSessionSummary>,
        next_cursor: Option<String>,
        replace: bool,
    },
    /// What the provider's default model advertises, once it has said so.
    ///
    /// Not session-scoped: the answer is about the model, not a thread, and it
    /// is what turns a composer mode from "refused, unverified" into something
    /// this adapter will actually put on a turn.
    ModelsListed {
        models: Vec<ModelCapabilities>,
    },
    SessionReady {
        provider_session_id: String,
        is_new: bool,
    },
    TurnStarted {
        provider_session_id: String,
    },
    SessionUpdate {
        provider_session_id: String,
        update: RuntimeUpdate,
    },
    PermissionRequested {
        request_id: u64,
        provider_session_id: String,
        summary: String,
        options: Vec<RuntimePermissionOption>,
    },
    TurnFinished {
        provider_session_id: String,
        stop_reason: String,
    },
    Failure(RuntimeFailure),
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSendError {
    /// The command queue is full. Transient — retry, never a failure to show.
    Full,
    /// The runtime is gone. Terminal for this handle.
    Closed,
}

impl fmt::Display for RuntimeSendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => formatter.write_str("the provider command queue is full"),
            Self::Closed => formatter.write_str("the provider runtime is closed"),
        }
    }
}

impl std::error::Error for RuntimeSendError {}

/// A live provider process, as the host sees it.
///
/// Deliberately tiny. Everything specific — the executable, the wire format,
/// the handshake, the retry policy — lives behind the implementation, because
/// that is the ONLY part that legitimately differs between providers.
///
/// `Send` because the host owns these in shared state.
pub trait ProviderRuntime: Send {
    /// Which provider this runtime speaks for. The host stamps every signal
    /// with it, so one code path can serve them all.
    fn provider(&self) -> ProviderId;

    /// Queue a command. Never blocks.
    fn try_send(&self, command: RuntimeCommand) -> Result<(), RuntimeSendError>;

    /// Take the next event, if one is waiting. Never blocks.
    fn try_event(&self) -> Result<Option<RuntimeEvent>, RuntimeSendError>;

    /// The capabilities the provider advertised at connect, if it has
    /// connected. `None` before `Connected` arrives — a runtime must not
    /// claim capability it has not been told it has.
    fn capabilities(&self) -> Option<RuntimeCapabilities>;
}

/// Which chat capabilities a connected runtime has EARNED.
///
/// One definition, so Codex and Grok cannot drift into advertising different
/// things for the same protocol facts. This is what makes `can_send()` true —
/// and it is derived from what the provider said, never from a hard-coded
/// list in the app's boot path.
pub fn qualified_capabilities(capabilities: RuntimeCapabilities) -> Vec<crate::Capability> {
    use crate::Capability;
    // A connected runtime can always start a session and stream a turn —
    // that IS the protocol. Everything else is advertised.
    let mut qualified = vec![
        Capability::StartSession,
        Capability::StreamTurns,
        Capability::InterruptTurn,
    ];
    if capabilities.load_session {
        qualified.push(Capability::ResumeSession);
    }
    if capabilities.list_sessions {
        qualified.push(Capability::ReadSessions);
    }
    qualified
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Capability;

    #[test]
    fn a_connected_runtime_can_always_start_and_stream() {
        let qualified = qualified_capabilities(RuntimeCapabilities::default());
        assert!(qualified.contains(&Capability::StartSession));
        assert!(qualified.contains(&Capability::StreamTurns));
    }

    /// Advertised capability is EARNED, never assumed: a provider that never
    /// said it can list sessions must not have the surface offer it.
    #[test]
    fn unadvertised_capability_is_never_qualified() {
        let qualified = qualified_capabilities(RuntimeCapabilities::default());
        assert!(!qualified.contains(&Capability::ReadSessions));
        assert!(!qualified.contains(&Capability::ResumeSession));

        let full = qualified_capabilities(RuntimeCapabilities {
            load_session: true,
            list_sessions: true,
        });
        assert!(full.contains(&Capability::ReadSessions));
        assert!(full.contains(&Capability::ResumeSession));
    }

    #[test]
    fn a_full_queue_is_transient_and_a_closed_runtime_is_not() {
        assert_eq!(
            RuntimeSendError::Full.to_string(),
            "the provider command queue is full"
        );
        assert_ne!(RuntimeSendError::Full, RuntimeSendError::Closed);
    }

    #[test]
    fn a_failure_can_name_the_session_it_belongs_to() {
        let loose = RuntimeFailure::new(RuntimeFailureKind::Launch, "no executable");
        assert_eq!(loose.provider_session_id, None);
        let scoped =
            RuntimeFailure::for_session(RuntimeFailureKind::Protocol, "bad frame", "session-1");
        assert_eq!(scoped.provider_session_id.as_deref(), Some("session-1"));
    }
}
