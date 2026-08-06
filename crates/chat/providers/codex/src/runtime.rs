//! Codex conversation transport — the second spoke on the provider seam.
//!
//! Codex does NOT speak ACP. Measured, not assumed: `codex --help` on
//! 0.146.0-alpha.3.1 offers `app-server` (JSON-RPC over stdio, what the IDE
//! integrations drive) and `mcp-server` (which exposes Codex as a TOOL to
//! another agent — the wrong direction for a conversation the operator has).
//! So this is a JSON-RPC client, not a copy of Grok's ACP client.
//!
//! Every frame name and field here was read off the protocol's OWN schema
//! (`codex app-server generate-json-schema`), vendored under `schema/` with
//! the CLI version it came from. We do not trust a wire; we pin it.
//!
//! The whole file exists to satisfy [`ProviderRuntime`]: commands in, events
//! out, never blocking, so the host drives Codex and Grok through one path
//! and emits one set of signals.

use dpl_chat_runtime::seam::{
    ProviderRuntime, RuntimeCapabilities, RuntimeCommand, RuntimeEvent, RuntimeFailure,
    RuntimeFailureKind, RuntimeSendError, RuntimeUpdate,
};
use serde_json::{json, Value};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::Mutex;

/// The subcommand that speaks the protocol. `app-server` is marked
/// `[experimental]` by the vendor — recorded rather than hidden, because a
/// provider that stops answering must fail with a reason, not a shrug.
pub const APP_SERVER_ARGS: &[&str] = &["app-server"];

/// The CLI the vendored schema under `schema/` was generated from.
pub const PINNED_CLI_VERSION: &str = "0.146.0-alpha.3.1";

/// Every method this adapter calls. Pinned so a Codex upgrade that removes
/// one fails a TEST with the method's name, rather than a turn that hangs.
pub const METHODS_USED: &[&str] = &["initialize", "thread/start", "turn/start", "turn/interrupt"];

/// Every notification this adapter maps. Anything else is kept as
/// `RuntimeUpdate::Other` — an unknown frame is evidence, never a drop.
pub const NOTIFICATIONS_MAPPED: &[&str] = &[
    "thread/started",
    "turn/started",
    "turn/completed",
    "item/agentMessage/delta",
    "item/reasoning/textDelta",
    "item/reasoning/summaryTextDelta",
    "thread/tokenUsage/updated",
    "error",
];

/// A frame going out, before it is given a JSON-RPC id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outbound {
    /// A request expecting a response.
    Request { method: &'static str, params: Value },
    /// Stop the worker and the child.
    Shutdown,
}

/// Translate a seam command into the Codex frame that performs it.
///
/// `thread_id` is the CURRENT thread; `RuntimeCommand` speaks
/// `provider_session_id`, which for Codex IS the thread id.
///
/// Returns `None` for a command this adapter does not implement. That is not
/// a silent drop: the capability was never advertised (see
/// [`advertised_capabilities`]), so the surface never offered it.
pub fn outbound_for(command: &RuntimeCommand) -> Option<Outbound> {
    match command {
        RuntimeCommand::NewSession => Some(Outbound::Request {
            method: "thread/start",
            // Every field of ThreadStartParams is optional; defaults are the
            // operator's own `~/.codex/config.toml`, which is the honest
            // choice — this app does not silently re-configure their CLI.
            params: json!({}),
        }),
        RuntimeCommand::Prompt {
            provider_session_id,
            text,
        } => Some(Outbound::Request {
            method: "turn/start",
            params: json!({
                "threadId": provider_session_id,
                "input": [{ "type": "text", "text": text }],
            }),
        }),
        RuntimeCommand::Cancel {
            provider_session_id,
        } => Some(Outbound::Request {
            method: "turn/interrupt",
            params: json!({ "threadId": provider_session_id }),
        }),
        RuntimeCommand::Shutdown => Some(Outbound::Shutdown),
        // Not implemented in this pass, and NOT advertised. `thread/list`,
        // `thread/resume` and `turn/steer` all exist in the protocol; adding
        // them is adding a mapping plus the capability that admits it.
        RuntimeCommand::ListSessions { .. }
        | RuntimeCommand::LoadSession { .. }
        | RuntimeCommand::ResolvePermission { .. } => None,
    }
}

/// What this adapter honestly supports today.
///
/// `list_sessions`/`load_session` are FALSE because the mappings are not
/// written. `qualified_capabilities` derives the chat capability set from
/// this, so a surface never offers what the adapter cannot do — the seam's
/// central guarantee, exercised here for the first time.
pub const fn advertised_capabilities() -> RuntimeCapabilities {
    RuntimeCapabilities {
        load_session: false,
        list_sessions: false,
    }
}

/// Map one server notification onto the seam.
///
/// Field names come from the vendored schema:
/// `AgentMessageDeltaNotification { delta, itemId, threadId, turnId }`,
/// `ReasoningTextDeltaNotification { contentIndex, delta, itemId, threadId,
/// turnId }`, `ThreadStartedNotification { thread }`,
/// `TurnCompletedNotification { threadId, turn }`,
/// `ErrorNotification { error, threadId, turnId, willRetry }`.
pub fn event_for(method: &str, params: &Value) -> Option<RuntimeEvent> {
    let thread = |key: &str| params.get(key).and_then(Value::as_str).map(str::to_owned);
    let session = || thread("threadId");
    let delta = || {
        params
            .get("delta")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    match method {
        "thread/started" => {
            // The id lives on the Thread object, not at the top level.
            let id = params
                .get("thread")
                .and_then(|t| t.get("id"))
                .and_then(Value::as_str)?
                .to_owned();
            Some(RuntimeEvent::SessionReady {
                provider_session_id: id,
                is_new: true,
            })
        }
        "turn/started" => Some(RuntimeEvent::TurnStarted {
            provider_session_id: session()?,
        }),
        "turn/completed" => Some(RuntimeEvent::TurnFinished {
            provider_session_id: session()?,
            // The protocol carries a rich Turn; the seam wants a reason. An
            // absent one reads "completed" rather than inventing a failure.
            stop_reason: params
                .get("turn")
                .and_then(|t| t.get("status"))
                .and_then(Value::as_str)
                .unwrap_or("completed")
                .to_owned(),
        }),
        "item/agentMessage/delta" => Some(RuntimeEvent::SessionUpdate {
            provider_session_id: session()?,
            update: RuntimeUpdate::AssistantText(delta()),
        }),
        "item/reasoning/textDelta" | "item/reasoning/summaryTextDelta" => {
            Some(RuntimeEvent::SessionUpdate {
                provider_session_id: session()?,
                update: RuntimeUpdate::ThoughtText(delta()),
            })
        }
        "thread/tokenUsage/updated" => {
            let usage = params.get("usage").or_else(|| params.get("tokenUsage"))?;
            let field = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
            Some(RuntimeEvent::SessionUpdate {
                provider_session_id: session()?,
                update: RuntimeUpdate::Usage {
                    used: field("total_tokens") + field("totalTokens"),
                    size: field("context_window") + field("contextWindow"),
                },
            })
        }
        "error" => {
            let reason = params
                .get("error")
                .and_then(|e| {
                    e.get("message")
                        .and_then(Value::as_str)
                        .or_else(|| e.as_str())
                })
                .unwrap_or("the provider reported an error without a message")
                .to_owned();
            // `willRetry` is the difference between a transient hiccup and a
            // failure worth showing the operator.
            let kind = if params
                .get("willRetry")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                RuntimeFailureKind::Backpressure
            } else {
                RuntimeFailureKind::Protocol
            };
            Some(RuntimeEvent::Failure(match session() {
                Some(id) => RuntimeFailure::for_session(kind, reason, id),
                None => RuntimeFailure::new(kind, reason),
            }))
        }
        // An unrecognised notification is KEPT. A dropped frame is how a turn
        // appears to hang with nothing to show for it.
        other => Some(RuntimeEvent::SessionUpdate {
            provider_session_id: session()?,
            update: RuntimeUpdate::Other(other.to_owned()),
        }),
    }
}

/// The live runtime: a `codex app-server` child, a worker, two queues.
pub struct CodexAppServerRuntime {
    commands: SyncSender<Outbound>,
    events: Mutex<Receiver<RuntimeEvent>>,
    capabilities: Mutex<Option<RuntimeCapabilities>>,
}

impl CodexAppServerRuntime {
    /// Build from already-wired channels.
    ///
    /// The child and worker are assembled by the host so this type stays
    /// testable without spawning a process — the mapping above is the part
    /// that carries the risk, and it is pure.
    /// A BOUNDED command queue, so `RuntimeSendError::Full` means something.
    /// An unbounded sender can never report backpressure, and the seam
    /// distinguishes transient fullness from a closed runtime.
    pub fn from_parts(commands: SyncSender<Outbound>, events: Receiver<RuntimeEvent>) -> Self {
        Self {
            commands,
            events: Mutex::new(events),
            capabilities: Mutex::new(None),
        }
    }
}

impl ProviderRuntime for CodexAppServerRuntime {
    fn provider(&self) -> dpl_chat_core::ProviderId {
        crate::PROVIDER
    }

    fn try_send(&self, command: RuntimeCommand) -> Result<(), RuntimeSendError> {
        let Some(outbound) = outbound_for(&command) else {
            // Unimplemented AND unadvertised. Refusing loudly beats pretending.
            return Err(RuntimeSendError::Closed);
        };
        self.commands
            .try_send(outbound)
            .map_err(|error| match error {
                TrySendError::Full(_) => RuntimeSendError::Full,
                TrySendError::Disconnected(_) => RuntimeSendError::Closed,
            })
    }

    fn try_event(&self) -> Result<Option<RuntimeEvent>, RuntimeSendError> {
        let events = self
            .events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match events.try_recv() {
            Ok(event) => {
                if let RuntimeEvent::Connected { capabilities } = &event {
                    *self
                        .capabilities
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(*capabilities);
                }
                Ok(Some(event))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(RuntimeSendError::Closed),
        }
    }

    fn capabilities(&self) -> Option<RuntimeCapabilities> {
        *self
            .capabilities
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// The `initialize` frame. `ClientInfo { name, version }` is required.
pub fn initialize_params() -> Value {
    json!({
        "clientInfo": {
            "name": "devpulselabs",
            "version": env!("CARGO_PKG_VERSION"),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_chat_core::Capability;
    use dpl_chat_runtime::seam::qualified_capabilities;

    #[test]
    fn a_prompt_becomes_a_turn_start_on_its_thread() {
        let outbound = outbound_for(&RuntimeCommand::Prompt {
            provider_session_id: "th-1".into(),
            text: "ship it".into(),
        })
        .expect("prompt is implemented");
        assert_eq!(
            outbound,
            Outbound::Request {
                method: "turn/start",
                params: json!({
                    "threadId": "th-1",
                    "input": [{ "type": "text", "text": "ship it" }],
                }),
            }
        );
    }

    #[test]
    fn an_agent_message_delta_becomes_assistant_text() {
        let event = event_for(
            "item/agentMessage/delta",
            &json!({"delta":"hel","itemId":"i1","threadId":"th-1","turnId":"t1"}),
        )
        .expect("mapped");
        assert_eq!(
            event,
            RuntimeEvent::SessionUpdate {
                provider_session_id: "th-1".into(),
                update: RuntimeUpdate::AssistantText("hel".into()),
            }
        );
    }

    #[test]
    fn reasoning_deltas_are_thought_not_answer() {
        for method in [
            "item/reasoning/textDelta",
            "item/reasoning/summaryTextDelta",
        ] {
            let event = event_for(
                method,
                &json!({"delta":"thinking","itemId":"i","threadId":"th","turnId":"t","contentIndex":0}),
            )
            .expect("mapped");
            match event {
                RuntimeEvent::SessionUpdate { update, .. } => {
                    assert_eq!(update, RuntimeUpdate::ThoughtText("thinking".into()));
                }
                other => panic!("expected an update, got {other:?}"),
            }
        }
    }

    /// The thread id lives on the Thread object, not at the top level — the
    /// exact kind of thing reading the schema prevents.
    #[test]
    fn thread_started_reads_its_id_off_the_thread_object() {
        let event = event_for("thread/started", &json!({"thread":{"id":"th-9"}})).expect("mapped");
        assert_eq!(
            event,
            RuntimeEvent::SessionReady {
                provider_session_id: "th-9".into(),
                is_new: true,
            }
        );
    }

    /// A retryable error is transient and must never read as the operator's
    /// failure; a non-retryable one is a protocol failure.
    #[test]
    fn will_retry_separates_backpressure_from_a_real_failure() {
        let transient = event_for(
            "error",
            &json!({"error":{"message":"rate limited"},"threadId":"th","turnId":"t","willRetry":true}),
        )
        .expect("mapped");
        match transient {
            RuntimeEvent::Failure(failure) => {
                assert_eq!(failure.kind, RuntimeFailureKind::Backpressure);
                assert_eq!(failure.provider_session_id.as_deref(), Some("th"));
            }
            other => panic!("expected a failure, got {other:?}"),
        }
        let fatal = event_for(
            "error",
            &json!({"error":{"message":"bad frame"},"threadId":"th","turnId":"t","willRetry":false}),
        )
        .expect("mapped");
        match fatal {
            RuntimeEvent::Failure(failure) => {
                assert_eq!(failure.kind, RuntimeFailureKind::Protocol)
            }
            other => panic!("expected a failure, got {other:?}"),
        }
    }

    /// An unknown frame is EVIDENCE. Dropping it is how a turn appears to
    /// hang with nothing to show for it.
    #[test]
    fn an_unrecognised_notification_is_kept_not_dropped() {
        let event = event_for("thread/goal/updated", &json!({"threadId":"th"})).expect("kept");
        assert_eq!(
            event,
            RuntimeEvent::SessionUpdate {
                provider_session_id: "th".into(),
                update: RuntimeUpdate::Other("thread/goal/updated".into()),
            }
        );
    }

    /// The seam's central guarantee: what is not implemented is not
    /// advertised, so the surface never offers it.
    #[test]
    fn unimplemented_commands_are_never_advertised() {
        assert!(outbound_for(&RuntimeCommand::ListSessions { cursor: None }).is_none());
        assert!(outbound_for(&RuntimeCommand::LoadSession {
            provider_session_id: "x".into()
        })
        .is_none());

        let qualified = qualified_capabilities(advertised_capabilities());
        assert!(
            qualified.contains(&Capability::StreamTurns),
            "chat is the point"
        );
        assert!(qualified.contains(&Capability::StartSession));
        assert!(!qualified.contains(&Capability::ReadSessions));
        assert!(!qualified.contains(&Capability::ResumeSession));
    }

    /// When Codex moves, this fails with the method NAME rather than a turn
    /// that quietly stops working.
    #[test]
    fn the_pinned_schema_still_declares_every_method_we_call() {
        let schema = include_str!("../schema/ClientRequest.json");
        for method in METHODS_USED {
            assert!(
                schema.contains(&format!("\"{method}\"")),
                "{method} is gone from the pinned schema ({PINNED_CLI_VERSION})"
            );
        }
    }

    #[test]
    fn the_pinned_schema_still_declares_every_notification_we_map() {
        let schema = include_str!("../schema/ServerNotification.json");
        for notification in NOTIFICATIONS_MAPPED {
            assert!(
                schema.contains(&format!("\"{notification}\"")),
                "{notification} is gone from the pinned schema ({PINNED_CLI_VERSION})"
            );
        }
    }
}
