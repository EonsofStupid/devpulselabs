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

use crate::CodexRuntimeProfile;
use dpl_chat_runtime::seam::{
    ProviderRuntime, RuntimeCapabilities, RuntimeCommand, RuntimeEvent, RuntimeFailure,
    RuntimeFailureKind, RuntimeSendError, RuntimeUpdate,
};
use serde_json::{json, Value};
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, Stdio};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;

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
            // `ThreadTokenUsage { last, total, modelContextWindow }`, where each
            // breakdown is `{ inputTokens, cachedInputTokens,
            // cacheWriteInputTokens, outputTokens, reasoningOutputTokens,
            // totalTokens }`.
            //
            // The NESTING is why the schema is pinned: this adapter first read
            // `totalTokens` at the top level, which parses fine and reports
            // zero forever. A live turn showed `0 of 0` and the schema said
            // why. Guessed field names fail silently; pinned ones fail loudly.
            //
            // FIDELITY LIMIT, stated rather than hidden: Codex reports a full
            // input/cache/output split and the seam's `Usage` carries only
            // context occupancy, so the split is DROPPED here. It is not
            // folded into another number — a folded total cannot be taken
            // apart again, whereas an absent one is honestly absent.
            // Widening the seam to carry a breakdown is its own decision.
            let usage = params.get("tokenUsage")?;
            let total = |key: &str| {
                usage
                    .get("total")
                    .and_then(|total| total.get(key))
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
            };
            Some(RuntimeEvent::SessionUpdate {
                provider_session_id: session()?,
                update: RuntimeUpdate::Usage {
                    used: total("totalTokens"),
                    size: usage
                        .get("modelContextWindow")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
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

/// One decoded line from the server.
///
/// Responses omit `jsonrpc` entirely, so the ONLY reliable discriminator is
/// structural: a frame is a response iff it carries an `id` and no `method`.
/// Measured on the live protocol — guessing this wrong is how a response gets
/// fed to the notification mapper and silently becomes an `Other` update.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Notification { method: String, params: Value },
    Response { id: u64, failed: bool },
}

/// Classify one line of the server's stdout.
///
/// `None` for anything unparseable — a partial or non-JSON line is noise from
/// a CLI that also prints diagnostics, never a protocol event.
pub fn classify(line: &str) -> Option<Frame> {
    let frame: Value = serde_json::from_str(line.trim()).ok()?;
    match (frame.get("method").and_then(Value::as_str), frame.get("id")) {
        (Some(method), _) => Some(Frame::Notification {
            method: method.to_owned(),
            params: frame.get("params").cloned().unwrap_or(Value::Null),
        }),
        (None, Some(id)) => Some(Frame::Response {
            id: id.as_u64()?,
            failed: frame.get("error").is_some(),
        }),
        (None, None) => None,
    }
}

/// The id the handshake's `initialize` carries. Fixed, so the reader can
/// recognise the one response this adapter acts on without tracking state.
const INITIALIZE_ID: u64 = 1;

/// How many commands may queue before `try_send` reports real backpressure.
const COMMAND_CAPACITY: usize = 32;

/// How many events may queue before the reader thread blocks on the host.
/// Generous: a fast turn emits a delta per token, and the host drains once a
/// frame.
const EVENT_CAPACITY: usize = 1024;

/// The live runtime: a `codex app-server` child, a worker, two queues.
pub struct CodexAppServerRuntime {
    commands: SyncSender<Outbound>,
    events: Mutex<Receiver<RuntimeEvent>>,
    capabilities: Mutex<Option<RuntimeCapabilities>>,
    /// The child this runtime spawned, kept so `Drop` can reap it. `None` for
    /// a [`CodexAppServerRuntime::from_parts`] runtime, which owns no process.
    child: Mutex<Option<Child>>,
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
            child: Mutex::new(None),
        }
    }

    /// Spawn `codex app-server` and wire the queues to it.
    ///
    /// Three threads, because the protocol is full-duplex and any one of them
    /// blocking must not stall the others:
    ///
    /// - **reader** — parses stdout, maps notifications through [`event_for`];
    /// - **writer** — owns stdin for its whole life (see below) and serves the
    ///   command queue;
    /// - **stderr drain** — a full stderr pipe wedges the child, and its last
    ///   line is the only explanation available when the child dies early.
    ///
    /// **stdin stays OPEN for the life of the runtime.** This is the single
    /// most expensive fact learned about this protocol: closing stdin after
    /// the handshake makes `app-server` exit immediately, which cost six
    /// consecutive probe attempts before it was measured.
    pub fn start(profile: &CodexRuntimeProfile) -> io::Result<Self> {
        let mut command =
            crate::activation::profiled_command(profile.executable(), profile.working_directory());
        command
            .args(APP_SERVER_ARGS)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;

        let missing = |what: &str| io::Error::other(format!("codex app-server exposed no {what}"));
        let mut stdin = child.stdin.take().ok_or_else(|| missing("stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| missing("stdout"))?;
        let stderr = child.stderr.take();

        let (command_tx, command_rx) = sync_channel::<Outbound>(COMMAND_CAPACITY);
        let (event_tx, event_rx) = sync_channel::<RuntimeEvent>(EVENT_CAPACITY);

        // The child's own account of why it died, when it dies before saying
        // anything on the protocol. Bounded to one line: this is a diagnostic,
        // not a log sink.
        let stderr_tail = Arc::new(Mutex::new(String::new()));
        if let Some(stderr) = stderr {
            let tail = Arc::clone(&stderr_tail);
            thread::Builder::new()
                .name("devpulse-codex-stderr".to_owned())
                .spawn(move || {
                    for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                        if !line.trim().is_empty() {
                            *tail.lock().unwrap_or_else(|p| p.into_inner()) = line;
                        }
                    }
                })?;
        }

        let reader_events = event_tx.clone();
        let reader_tail = Arc::clone(&stderr_tail);
        thread::Builder::new()
            .name("devpulse-codex-reader".to_owned())
            .spawn(move || read_frames(BufReader::new(stdout), &reader_events, &reader_tail))?;

        thread::Builder::new()
            .name("devpulse-codex-writer".to_owned())
            .spawn(move || {
                // The handshake, sent without waiting: `initialized` is
                // mandatory and the server accepts it pipelined behind
                // `initialize`. Proved against the live CLI — a real turn
                // completed with exactly this ordering.
                let handshake = [
                    request_line(INITIALIZE_ID, "initialize", &initialize_params()),
                    json!({ "jsonrpc": "2.0", "method": "initialized" }).to_string(),
                ];
                for line in handshake {
                    if writeln!(stdin, "{line}")
                        .and_then(|()| stdin.flush())
                        .is_err()
                    {
                        return;
                    }
                }
                let mut next_id = INITIALIZE_ID + 1;
                // A blocking recv: the writer sleeps until the host speaks, and
                // holds stdin open the entire time.
                while let Ok(outbound) = command_rx.recv() {
                    let Outbound::Request { method, params } = outbound else {
                        break; // Shutdown: fall through and release stdin.
                    };
                    let line = request_line(next_id, method, &params);
                    next_id = next_id.saturating_add(1);
                    if writeln!(stdin, "{line}")
                        .and_then(|()| stdin.flush())
                        .is_err()
                    {
                        break;
                    }
                }
                // Dropping stdin here is what tells `app-server` to exit, and
                // it happens ONLY on shutdown or a dead pipe.
                drop(stdin);
            })?;

        Ok(Self {
            commands: command_tx,
            events: Mutex::new(event_rx),
            capabilities: Mutex::new(None),
            child: Mutex::new(Some(child)),
        })
    }
}

/// One JSON-RPC request, serialised. `jsonrpc` is sent even though the server
/// omits it in responses — the client half of the contract still requires it.
fn request_line(id: u64, method: &str, params: &Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
}

/// Read frames until the child's stdout ends, translating as we go.
///
/// Ends by emitting [`RuntimeEvent::Stopped`], so a child that dies is a
/// state the host observes rather than a queue that quietly goes silent.
fn read_frames(
    stdout: impl BufRead,
    events: &SyncSender<RuntimeEvent>,
    stderr_tail: &Arc<Mutex<String>>,
) {
    let mut connected = false;
    for line in stdout.lines().map_while(Result::ok) {
        let Some(frame) = classify(&line) else {
            continue;
        };
        let event = match frame {
            Frame::Notification { method, params } => event_for(&method, &params),
            // `initialize` answering is what "connected" MEANS — not that the
            // process spawned. Capability comes from what this adapter can
            // honour, which is the seam's guarantee (see
            // `advertised_capabilities`).
            Frame::Response { id, failed } if id == INITIALIZE_ID => {
                connected = true;
                Some(if failed {
                    RuntimeEvent::Failure(RuntimeFailure::new(
                        RuntimeFailureKind::Protocol,
                        "codex app-server rejected the initialize handshake",
                    ))
                } else {
                    RuntimeEvent::Connected {
                        capabilities: advertised_capabilities(),
                    }
                })
            }
            // Every other response is an acknowledgement; the work it started
            // reports through notifications.
            Frame::Response { .. } => None,
        };
        if let Some(event) = event {
            if events.send(event).is_err() {
                return; // The host is gone.
            }
        }
    }
    // Stdout closed. If it closed before `initialize` was ever answered, the
    // child failed to start properly and its stderr is the only explanation
    // anyone will get.
    if !connected {
        let reason = stderr_tail
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        let _ = events.send(RuntimeEvent::Failure(RuntimeFailure::new(
            RuntimeFailureKind::Launch,
            if reason.is_empty() {
                "codex app-server exited before completing the handshake".to_owned()
            } else {
                format!("codex app-server exited during startup: {reason}")
            },
        )));
    }
    let _ = events.send(RuntimeEvent::Stopped);
}

impl Drop for CodexAppServerRuntime {
    fn drop(&mut self) {
        // Closing the command queue ends the writer, which releases stdin and
        // asks `app-server` to exit on its own terms.
        let _ = self.commands.try_send(Outbound::Shutdown);
        if let Some(mut child) = self
            .child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            let _ = child.kill();
            let _ = child.wait();
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
    /// Token usage is NESTED. Reading `totalTokens` at the top level parses
    /// cleanly and reports zero forever — which is exactly what a live turn
    /// did before the schema was consulted. This pins the real shape.
    #[test]
    fn token_usage_is_read_from_the_nested_total_not_the_top_level() {
        let event = event_for(
            "thread/tokenUsage/updated",
            &json!({
                "threadId": "th",
                "turnId": "t",
                "tokenUsage": {
                    "last": {"inputTokens": 10, "cachedInputTokens": 0,
                             "outputTokens": 5, "reasoningOutputTokens": 0,
                             "totalTokens": 15},
                    "total": {"inputTokens": 100, "cachedInputTokens": 20,
                              "outputTokens": 50, "reasoningOutputTokens": 10,
                              "totalTokens": 180},
                    "modelContextWindow": 272_000,
                },
            }),
        )
        .expect("mapped");
        assert_eq!(
            event,
            RuntimeEvent::SessionUpdate {
                provider_session_id: "th".into(),
                update: RuntimeUpdate::Usage {
                    used: 180,
                    size: 272_000,
                },
            }
        );
    }

    /// The nested field names this adapter reaches through must still exist,
    /// so a Codex upgrade that reshapes usage fails HERE and not as a silent
    /// zero in the operator's cost ledger.
    #[test]
    fn the_pinned_schema_still_declares_the_usage_shape_we_reach_through() {
        let schema = include_str!("../schema/ServerNotification.json");
        for field in ["tokenUsage", "modelContextWindow", "totalTokens", "total"] {
            assert!(
                schema.contains(&format!("\"{field}\"")),
                "usage field {field} is gone from the pinned schema ({PINNED_CLI_VERSION})"
            );
        }
    }

    /// Responses omit `jsonrpc`, so the discriminator is structural. Getting
    /// this backwards feeds a response to the notification mapper, where it
    /// becomes a bogus `Other` update instead of an acknowledgement.
    #[test]
    fn a_response_frame_is_told_from_a_notification() {
        assert_eq!(
            classify(r#"{"id":1,"result":{"userAgent":"codex"}}"#),
            Some(Frame::Response {
                id: 1,
                failed: false
            })
        );
        assert_eq!(
            classify(r#"{"id":7,"error":{"code":-32601,"message":"nope"}}"#),
            Some(Frame::Response {
                id: 7,
                failed: true
            })
        );
        assert_eq!(
            classify(r#"{"jsonrpc":"2.0","method":"turn/started","params":{"threadId":"th"}}"#),
            Some(Frame::Notification {
                method: "turn/started".to_owned(),
                params: json!({"threadId":"th"}),
            })
        );
        // A CLI that also prints diagnostics must not have them read as
        // protocol.
        assert_eq!(classify("Listening on stdio"), None);
        assert_eq!(classify(""), None);
    }

    /// A request the writer emits must carry `jsonrpc` and the id the reader
    /// recognises the handshake by.
    #[test]
    fn the_handshake_request_is_addressed_to_the_id_the_reader_watches() {
        let line = request_line(INITIALIZE_ID, "initialize", &initialize_params());
        let frame: Value = serde_json::from_str(&line).expect("valid JSON");
        assert_eq!(frame["jsonrpc"], "2.0");
        assert_eq!(frame["id"], INITIALIZE_ID);
        assert_eq!(frame["method"], "initialize");
        assert!(frame["params"]["clientInfo"]["name"] == "devpulselabs");
    }

    /// "Connected" means `initialize` was ANSWERED — not that a process
    /// spawned. A failed handshake is a protocol failure, never a silent
    /// connect.
    #[test]
    fn connected_is_emitted_only_when_initialize_is_answered() {
        let drain = |lines: &str| {
            let (tx, rx) = sync_channel(64);
            let tail = Arc::new(Mutex::new(String::new()));
            read_frames(BufReader::new(lines.as_bytes()), &tx, &tail);
            drop(tx);
            rx.into_iter().collect::<Vec<_>>()
        };

        let ok = drain("{\"id\":1,\"result\":{}}\n");
        assert_eq!(
            ok.first(),
            Some(&RuntimeEvent::Connected {
                capabilities: advertised_capabilities(),
            })
        );
        assert_eq!(ok.last(), Some(&RuntimeEvent::Stopped));

        let rejected = drain("{\"id\":1,\"error\":{\"message\":\"bad client\"}}\n");
        assert!(matches!(
            rejected.first(),
            Some(RuntimeEvent::Failure(failure))
                if failure.kind == RuntimeFailureKind::Protocol
        ));

        // A child that dies before answering must explain itself rather than
        // leave the host with a silent queue.
        let died = drain("");
        assert!(matches!(
            died.first(),
            Some(RuntimeEvent::Failure(failure))
                if failure.kind == RuntimeFailureKind::Launch
        ));
    }

    /// An acknowledgement is not an event: only `initialize`'s response means
    /// anything to the host, and the rest report through notifications.
    #[test]
    fn a_plain_acknowledgement_produces_no_event() {
        let (tx, rx) = sync_channel(64);
        let tail = Arc::new(Mutex::new(String::new()));
        read_frames(
            BufReader::new("{\"id\":1,\"result\":{}}\n{\"id\":2,\"result\":{}}\n".as_bytes()),
            &tx,
            &tail,
        );
        drop(tx);
        let events = rx.into_iter().collect::<Vec<_>>();
        assert_eq!(events.len(), 2, "connected + stopped, and nothing for id 2");
    }

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
