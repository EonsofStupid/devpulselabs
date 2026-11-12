//! Grok, spoken as the provider-neutral runtime seam.
//!
//! `GrokAcpRuntime` grew first and its vocabulary was already neutral in
//! SHAPE — only Grok-named. So aligning it to
//! [`dpl_chat_runtime::seam::ProviderRuntime`] is a translation, not a
//! rewrite of a working runtime: the ACP transport, the retry policy, and the
//! process handling are untouched.
//!
//! Doing it this way is the point. The host stops knowing "Grok" and starts
//! knowing "a provider runtime", which is what lets Codex be added without a
//! second set of signals.

use crate::runtime::{
    GrokAcpCapabilities, GrokAcpRuntime, GrokPermissionOption, GrokPermissionOptionKind,
    GrokRuntimeCommand, GrokRuntimeEvent, GrokRuntimeFailure, GrokRuntimeFailureKind,
    GrokRuntimeSendError, GrokSessionSummary, GrokSessionUpdate,
};
use dpl_chat_runtime::seam::{
    ProviderRuntime, RuntimeCapabilities, RuntimeCommand, RuntimeEvent, RuntimeFailure,
    RuntimeFailureKind, RuntimePermissionOption, RuntimePermissionOptionKind, RuntimeSendError,
    RuntimeSessionSummary, RuntimeUpdate,
};
use std::sync::Mutex;

/// `GrokAcpRuntime` wearing the seam.
///
/// Holds the advertised capabilities because the trait exposes them as a
/// question the host can ask at any time, while the runtime only announces
/// them once, in `Connected`.
pub struct GrokRuntimeSeam {
    inner: GrokAcpRuntime,
    capabilities: Mutex<Option<RuntimeCapabilities>>,
}

impl GrokRuntimeSeam {
    pub fn new(inner: GrokAcpRuntime) -> Self {
        Self {
            inner,
            capabilities: Mutex::new(None),
        }
    }

    /// The underlying runtime, for the Grok-specific paths the host still
    /// owns while the migration finishes. Temporary by intent.
    pub fn inner(&self) -> &GrokAcpRuntime {
        &self.inner
    }
}

impl ProviderRuntime for GrokRuntimeSeam {
    fn provider(&self) -> dpl_chat_core::ProviderId {
        crate::PROVIDER
    }

    fn try_send(&self, command: RuntimeCommand) -> Result<(), RuntimeSendError> {
        // A command Grok cannot honour was never advertised; refusing it
        // loudly beats pretending it was applied.
        let command = GrokRuntimeCommand::try_from(command)?;
        self.inner.try_send(command).map_err(Into::into)
    }

    fn try_event(&self) -> Result<Option<RuntimeEvent>, RuntimeSendError> {
        let event = self.inner.try_event().map_err(RuntimeSendError::from)?;
        let Some(event) = event else {
            return Ok(None);
        };
        let event = RuntimeEvent::from(event);
        // Latch what the provider advertised, so `capabilities()` can answer
        // later without the host having to remember the Connected frame.
        if let RuntimeEvent::Connected { capabilities } = &event {
            *self
                .capabilities
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(*capabilities);
        }
        Ok(Some(event))
    }

    fn capabilities(&self) -> Option<RuntimeCapabilities> {
        *self
            .capabilities
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

// ── translations ────────────────────────────────────────────────────────
//
// Exhaustive on purpose: a new variant on either side must not compile until
// it has been mapped. Silent loss of a provider event is how a turn appears
// to hang.

impl TryFrom<RuntimeCommand> for GrokRuntimeCommand {
    type Error = RuntimeSendError;

    fn try_from(command: RuntimeCommand) -> Result<Self, Self::Error> {
        Ok(match command {
            RuntimeCommand::ListSessions { cursor } => Self::ListSessions { cursor },
            RuntimeCommand::NewSession => Self::NewSession,
            RuntimeCommand::LoadSession {
                provider_session_id,
            } => Self::LoadSession {
                provider_session_id,
            },
            RuntimeCommand::Prompt {
                provider_session_id,
                text,
            } => Self::Prompt {
                provider_session_id,
                text,
            },
            RuntimeCommand::Cancel {
                provider_session_id,
            } => Self::Cancel {
                provider_session_id,
            },
            RuntimeCommand::ResolvePermission {
                request_id,
                option_id,
            } => Self::ResolvePermission {
                request_id,
                option_id,
            },
            RuntimeCommand::Shutdown => Self::Shutdown,
            // Grok's ACP modes are not mapped yet, so `permission_modes` is
            // not advertised and the host never sends this.
            RuntimeCommand::SetPermissionMode { .. } => return Err(RuntimeSendError::Closed),
        })
    }
}

impl From<GrokRuntimeSendError> for RuntimeSendError {
    fn from(error: GrokRuntimeSendError) -> Self {
        match error {
            GrokRuntimeSendError::Full => Self::Full,
            GrokRuntimeSendError::Closed => Self::Closed,
        }
    }
}

impl From<GrokAcpCapabilities> for RuntimeCapabilities {
    fn from(capabilities: GrokAcpCapabilities) -> Self {
        Self {
            load_session: capabilities.load_session,
            list_sessions: capabilities.list_sessions,
            permission_modes: false,
        }
    }
}

impl From<GrokSessionSummary> for RuntimeSessionSummary {
    fn from(summary: GrokSessionSummary) -> Self {
        Self {
            provider_session_id: summary.provider_session_id,
            title: summary.title,
            working_directory: summary.working_directory,
            updated_at: summary.updated_at,
        }
    }
}

impl From<GrokSessionUpdate> for RuntimeUpdate {
    fn from(update: GrokSessionUpdate) -> Self {
        match update {
            GrokSessionUpdate::UserText(text) => Self::UserText(text),
            GrokSessionUpdate::AssistantText(text) => Self::AssistantText(text),
            GrokSessionUpdate::ThoughtText(text) => Self::ThoughtText(text),
            GrokSessionUpdate::Tool(text) => Self::Tool(text),
            GrokSessionUpdate::Plan(text) => Self::Plan(text),
            GrokSessionUpdate::SessionInfo(text) => Self::SessionInfo(text),
            GrokSessionUpdate::AvailableCommands { count } => Self::AvailableCommands { count },
            GrokSessionUpdate::Usage { used, size } => Self::Usage { used, size },
            GrokSessionUpdate::Other(text) => Self::Other(text),
        }
    }
}

impl From<GrokRuntimeFailureKind> for RuntimeFailureKind {
    fn from(kind: GrokRuntimeFailureKind) -> Self {
        match kind {
            GrokRuntimeFailureKind::Launch => Self::Launch,
            GrokRuntimeFailureKind::Protocol => Self::Protocol,
            GrokRuntimeFailureKind::CredentialUnavailable => Self::CredentialUnavailable,
            GrokRuntimeFailureKind::Authentication => Self::Authentication,
            GrokRuntimeFailureKind::Unsupported => Self::Unsupported,
            GrokRuntimeFailureKind::InvalidCommand => Self::InvalidCommand,
            GrokRuntimeFailureKind::Backpressure => Self::Backpressure,
            GrokRuntimeFailureKind::Timeout => Self::Timeout,
            GrokRuntimeFailureKind::Disconnected => Self::Disconnected,
        }
    }
}

impl From<GrokRuntimeFailure> for RuntimeFailure {
    fn from(failure: GrokRuntimeFailure) -> Self {
        Self {
            kind: failure.kind.into(),
            reason: failure.reason,
            provider_session_id: failure.provider_session_id,
        }
    }
}

impl From<GrokPermissionOptionKind> for RuntimePermissionOptionKind {
    fn from(kind: GrokPermissionOptionKind) -> Self {
        match kind {
            GrokPermissionOptionKind::AllowOnce => Self::AllowOnce,
            GrokPermissionOptionKind::AllowAlways => Self::AllowAlways,
            GrokPermissionOptionKind::RejectOnce => Self::RejectOnce,
            GrokPermissionOptionKind::RejectAlways => Self::RejectAlways,
        }
    }
}

impl From<GrokPermissionOption> for RuntimePermissionOption {
    fn from(option: GrokPermissionOption) -> Self {
        Self {
            option_id: option.option_id,
            label: option.label,
            kind: option.kind.into(),
        }
    }
}

impl From<GrokRuntimeEvent> for RuntimeEvent {
    fn from(event: GrokRuntimeEvent) -> Self {
        match event {
            GrokRuntimeEvent::Connected { capabilities } => Self::Connected {
                capabilities: capabilities.into(),
            },
            GrokRuntimeEvent::SessionsPage {
                sessions,
                next_cursor,
                replace,
            } => Self::SessionsPage {
                sessions: sessions.into_iter().map(Into::into).collect(),
                next_cursor,
                replace,
            },
            GrokRuntimeEvent::SessionReady {
                provider_session_id,
                is_new,
            } => Self::SessionReady {
                provider_session_id,
                is_new,
            },
            GrokRuntimeEvent::TurnStarted {
                provider_session_id,
            } => Self::TurnStarted {
                provider_session_id,
            },
            GrokRuntimeEvent::SessionUpdate {
                provider_session_id,
                update,
            } => Self::SessionUpdate {
                provider_session_id,
                update: update.into(),
            },
            GrokRuntimeEvent::PermissionRequested {
                request_id,
                provider_session_id,
                summary,
                options,
            } => Self::PermissionRequested {
                request_id,
                provider_session_id,
                summary,
                options: options.into_iter().map(Into::into).collect(),
            },
            GrokRuntimeEvent::TurnFinished {
                provider_session_id,
                stop_reason,
            } => Self::TurnFinished {
                provider_session_id,
                stop_reason,
            },
            GrokRuntimeEvent::Failure(failure) => Self::Failure(failure.into()),
            GrokRuntimeEvent::Stopped => Self::Stopped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The translation must not quietly drop a session's identity, because a
    /// misrouted update is an answer appearing in the wrong conversation.
    #[test]
    fn a_session_update_keeps_its_session_and_its_text() {
        let event = RuntimeEvent::from(GrokRuntimeEvent::SessionUpdate {
            provider_session_id: "s-1".into(),
            update: GrokSessionUpdate::AssistantText("hello".into()),
        });
        match event {
            RuntimeEvent::SessionUpdate {
                provider_session_id,
                update,
            } => {
                assert_eq!(provider_session_id, "s-1");
                assert_eq!(update, RuntimeUpdate::AssistantText("hello".into()));
            }
            other => panic!("expected a session update, got {other:?}"),
        }
    }

    #[test]
    fn a_prompt_survives_the_round_trip_into_grok_vocabulary() {
        let command = GrokRuntimeCommand::try_from(RuntimeCommand::Prompt {
            provider_session_id: "s-2".into(),
            text: "ship it".into(),
        })
        .expect("a prompt maps");
        assert_eq!(
            command,
            GrokRuntimeCommand::Prompt {
                provider_session_id: "s-2".into(),
                text: "ship it".into(),
            }
        );
    }

    /// Backpressure is transient and must never translate into something the
    /// surface would show as the operator's failure.
    #[test]
    fn backpressure_stays_transient_across_the_seam() {
        assert_eq!(
            RuntimeSendError::from(GrokRuntimeSendError::Full),
            RuntimeSendError::Full
        );
        assert_eq!(
            RuntimeSendError::from(GrokRuntimeSendError::Closed),
            RuntimeSendError::Closed
        );
    }
}
