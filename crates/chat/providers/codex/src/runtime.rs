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
    ModelCapabilities, ProviderRuntime, RuntimeCapabilities, RuntimeCommand, RuntimeEvent,
    RuntimeFailure, RuntimeFailureKind, RuntimeSendError, RuntimeUpdate,
};
use std::collections::HashMap;

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
pub const PINNED_CLI_VERSION: &str = "0.147.0";

/// Every method this adapter calls. Pinned so a Codex upgrade that removes
/// one fails a TEST with the method's name, rather than a turn that hangs.
pub const METHODS_USED: &[&str] = &[
    "initialize",
    "thread/start",
    "turn/start",
    "turn/interrupt",
    "model/list",
];

/// Every notification this adapter maps. Anything else is kept as
/// `RuntimeUpdate::Other` — an unknown frame is evidence, never a drop.
/// The composer modes this adapter can actually put on the wire, and the
/// `turn/start` field each one becomes.
///
/// Read out of the pinned CLI's own schema, not guessed:
/// `codex app-server generate-json-schema` gives TurnStartParams with
/// `effort: ReasoningEffort` and `personality: Personality` among its optional
/// fields. Everything else the composer offers — web search, research, canvas,
/// agent mode — has NO field there, so this adapter refuses those by name
/// instead of pretending.
///
/// Adding one is the same shape as any other method here: a mapping, plus the
/// capability that admits it.
pub const HONOURED_MODES: &[&str] = &["extended-thinking", "style"];

/// `Personality` is a CLOSED enum in the schema — `none | friendly |
/// pragmatic` — so the VALUE is verifiable. Whether a given model accepts one
/// at all is not: `Model.supportsPersonality` decides that, per model.
const STYLE_PERSONALITY: &str = "friendly";

/// The effort `extended-thinking` asks for, IF the model advertises it.
///
/// `ReasoningEffort` is documented as "a non-empty reasoning effort value
/// ADVERTISED BY THE MODEL" — an open string whose valid set is per-model, not
/// a fixed enum. So this is a preference checked against
/// `Model.supportedReasoningEfforts`, never a value sent on faith. A model
/// that advertises something else entirely refuses the mode by name; adding
/// that name here is a one-line change once a real model advertises it.
const THINKING_EFFORT: &str = "high";

/// The modes this adapter can honour FOR A GIVEN MODEL.
///
/// [`HONOURED_MODES`] is what the adapter could ever carry; this is what it
/// can carry right now, against what the model actually advertises. `None`
/// means nothing has been verified, and nothing unverified is sent — the
/// operator is told the mode was refused instead of being given a turn that
/// silently ignored it, or one the CLI rejects outright.
pub fn honoured_for(model: Option<&ModelCapabilities>) -> Vec<&'static str> {
    let Some(model) = model else {
        return Vec::new();
    };
    let mut honoured = Vec::new();
    if model.advertises_effort(THINKING_EFFORT) {
        honoured.push("extended-thinking");
    }
    if model.supports_personality {
        honoured.push("style");
    }
    honoured
}

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
pub fn outbound_for(
    command: &RuntimeCommand,
    model: Option<&ModelCapabilities>,
) -> Option<Outbound> {
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
            modes,
        } => {
            let mut params = json!({
                "threadId": provider_session_id,
                "input": [{ "type": "text", "text": text }],
            });
            let object = params.as_object_mut().expect("json! built an object");
            // Only what the schema has a field for. A mode that is OFF sends
            // nothing at all, so the operator's own ~/.codex/config.toml stays
            // the default — this app does not silently re-configure their CLI.
            let honoured = honoured_for(model);
            if modes.is_active("extended-thinking") && honoured.contains(&"extended-thinking") {
                object.insert("effort".to_owned(), json!(THINKING_EFFORT));
            }
            if modes.is_active("style") && honoured.contains(&"style") {
                object.insert("personality".to_owned(), json!(STYLE_PERSONALITY));
            }
            Some(Outbound::Request {
                method: "turn/start",
                params,
            })
        }
        // What each model accepts is the ONLY thing that makes `effort` and
        // `personality` safe to send — see `honoured_for`. Every field we read
        // off the response is in `schema/v2/ModelListResponse.json`.
        RuntimeCommand::ListModels => Some(Outbound::Request {
            method: "model/list",
            params: json!({}),
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

/// Read `model/list`'s answer into the seam's shape.
///
/// Field names come from `schema/v2/ModelListResponse.json`: `{ data: Model[] }`
/// with `Model { id, displayName, isDefault, supportedReasoningEfforts:
/// ReasoningEffortOption[], defaultReasoningEffort, supportsPersonality }` and
/// `ReasoningEffortOption { description, reasoningEffort }`.
///
/// A model with no `id` is SKIPPED rather than defaulted. A model we cannot
/// read is one we must not claim capabilities for — the whole point of Phase
/// 0a was that an unverified capability is worse than an absent one.
pub fn models_from(result: &Value) -> Vec<ModelCapabilities> {
    let Some(data) = result.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    data.iter()
        .filter_map(|model| {
            Some(ModelCapabilities {
                id: model.get("id").and_then(Value::as_str)?.to_owned(),
                display_name: model
                    .get("displayName")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                is_default: model
                    .get("isDefault")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                reasoning_efforts: model
                    .get("supportedReasoningEfforts")
                    .and_then(Value::as_array)
                    .map(|efforts| {
                        efforts
                            .iter()
                            .filter_map(|e| {
                                e.get("reasoningEffort")
                                    .and_then(Value::as_str)
                                    .map(str::to_owned)
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                default_reasoning_effort: model
                    .get("defaultReasoningEffort")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                supports_personality: model
                    .get("supportsPersonality")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
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
    Notification {
        method: String,
        params: Value,
    },
    Response {
        id: u64,
        failed: bool,
        /// The `result` body. Previously discarded, which is why no request
        /// but `initialize` could ever be acted on.
        result: Value,
    },
    /// The agent asking US for something — an approval, a tool call, an
    /// elicitation. It carries an `id` and EXPECTS a reply echoing it, and the
    /// spec is explicit that the server pauses until one arrives.
    ServerRequest {
        id: u64,
        method: String,
        params: Value,
    },
}

/// Classify one line of the server's stdout.
///
/// `None` for anything unparseable — a partial or non-JSON line is noise from
/// a CLI that also prints diagnostics, never a protocol event.
pub fn classify(line: &str) -> Option<Frame> {
    let frame: Value = serde_json::from_str(line.trim()).ok()?;
    let params = || frame.get("params").cloned().unwrap_or(Value::Null);
    // The rule is the vendored envelope's, not an approximation of it
    // (`schema/JSONRPCMessage.json`):
    //
    //   JSONRPCRequest       id + method     ← the agent asking US
    //   JSONRPCNotification  method          ← no id, ever
    //   JSONRPCResponse      id + result
    //   JSONRPCError         id + error
    //
    // `id` is what separates a server request from a notification, and this
    // used to read any `method` as a notification. An approval would have
    // become an `Other` update and never been answered, and the spec says the
    // server BLOCKS until the client replies — so that was a hung turn waiting
    // for the day approvals are advertised.
    match (frame.get("method").and_then(Value::as_str), frame.get("id")) {
        (Some(method), Some(id)) => Some(Frame::ServerRequest {
            id: id.as_u64()?,
            method: method.to_owned(),
            params: params(),
        }),
        (Some(method), None) => Some(Frame::Notification {
            method: method.to_owned(),
            params: params(),
        }),
        (None, Some(id)) => Some(Frame::Response {
            id: id.as_u64()?,
            failed: frame.get("error").is_some(),
            result: frame.get("result").cloned().unwrap_or(Value::Null),
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
    /// What the chosen model advertises, once anything has verified it.
    ///
    /// `None` until then, and `None` is read strictly: no mode field is put on
    /// a turn on the strength of an assumption. `model/list` is what fills
    /// this in, and until that lands every mode here is refused by name.
    model: Mutex<Option<ModelCapabilities>>,
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
            model: Mutex::new(None),
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

        // id → the method it asked. Shared by the writer that assigns ids and
        // the reader that sees them answered; without it a response body has
        // no meaning even now that we keep it.
        let pending: Arc<Mutex<HashMap<u64, &'static str>>> = Arc::new(Mutex::new(HashMap::new()));
        let reader_pending = Arc::clone(&pending);

        let reader_events = event_tx.clone();
        let reader_tail = Arc::clone(&stderr_tail);
        thread::Builder::new()
            .name("devpulse-codex-reader".to_owned())
            .spawn(move || {
                read_frames(
                    BufReader::new(stdout),
                    &reader_events,
                    &reader_tail,
                    &reader_pending,
                )
            })?;

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
                // The writer assigns ids; the reader sees them come back. This
                // map is the only thing that connects the two, and without it
                // a response is an anonymous acknowledgement — which is
                // exactly why nothing but `initialize` could ever be acted on.
                //
                // Entries are REMOVED when claimed, so the map is bounded by
                // what is genuinely in flight rather than growing per request.
                let in_flight = Arc::clone(&pending);
                // A blocking recv: the writer sleeps until the host speaks, and
                // holds stdin open the entire time.
                while let Ok(outbound) = command_rx.recv() {
                    let Outbound::Request { method, params } = outbound else {
                        break; // Shutdown: fall through and release stdin.
                    };
                    let line = request_line(next_id, method, &params);
                    if let Ok(mut map) = in_flight.lock() {
                        map.insert(next_id, method);
                    }
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
            model: Mutex::new(None),
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
    pending: &Arc<Mutex<HashMap<u64, &'static str>>>,
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
            Frame::Response { id, failed, .. } if id == INITIALIZE_ID => {
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
            // Every other response is claimed from the in-flight map. A
            // method we mapped gets read; anything else stays what it always
            // was — an acknowledgement, with the work reporting through
            // notifications.
            Frame::Response { id, failed, result } => {
                let asked = pending.lock().ok().and_then(|mut map| map.remove(&id));
                match asked {
                    Some("model/list") if !failed => Some(RuntimeEvent::ModelsListed {
                        models: models_from(&result),
                    }),
                    // A failed model/list leaves capabilities unverified, and
                    // unverified means every mode stays refused. Said out loud
                    // rather than left as silence.
                    Some("model/list") => Some(RuntimeEvent::Failure(RuntimeFailure::new(
                        RuntimeFailureKind::Protocol,
                        "codex refused model/list; composer modes stay unverified",
                    ))),
                    _ => None,
                }
            }
            // The agent is asking US for something and, per the spec, is now
            // BLOCKED until it gets a reply. This adapter advertises none of
            // the capabilities that produce one (`advertised_capabilities`),
            // so reaching here means the agent asked anyway.
            //
            // Answering needs the writer's stdin, which this thread does not
            // hold. Until that plumbing exists, say so by name: a turn that
            // stalls with "approval requested, nobody can answer" is
            // diagnosable, and one that just stops is not.
            Frame::ServerRequest { method, .. } => {
                Some(RuntimeEvent::Failure(RuntimeFailure::new(
                    RuntimeFailureKind::Protocol,
                    format!(
                        "codex asked for `{method}` and this adapter cannot answer it; \
                         the turn will not continue"
                    ),
                )))
            }
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
    fn provider(&self) -> dpl_ai_core::ProviderId {
        crate::PROVIDER
    }

    fn try_send(&self, command: RuntimeCommand) -> Result<(), RuntimeSendError> {
        let model = self.model.lock().ok().and_then(|held| held.clone());
        let Some(outbound) = outbound_for(&command, model.as_ref()) else {
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
    use dpl_ai_core::composer::ComposerModes;
    use dpl_ai_core::Capability;
    use dpl_chat_runtime::seam::qualified_capabilities;

    #[test]
    fn a_prompt_becomes_a_turn_start_on_its_thread() {
        let outbound = outbound_for(
            &RuntimeCommand::Prompt {
                provider_session_id: "th-1".into(),
                text: "ship it".into(),
                modes: ComposerModes::default(),
            },
            None,
        )
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

    /// No mode standing sends NO extra field, so the operator's own
    /// ~/.codex/config.toml remains the default. Adding a field only when a
    /// mode is on is what keeps this adapter from re-configuring their CLI.
    #[test]
    fn a_standing_mode_becomes_the_field_the_schema_has_for_it() {
        let mut modes = ComposerModes::default();
        modes.toggle("extended-thinking").expect("sticky");
        modes.toggle("style").expect("sticky");
        let outbound = outbound_for(
            &RuntimeCommand::Prompt {
                provider_session_id: "th-1".into(),
                text: "ship it".into(),
                modes,
            },
            Some(&capable_model()),
        )
        .expect("prompt is implemented");
        assert_eq!(
            outbound,
            Outbound::Request {
                method: "turn/start",
                params: json!({
                    "threadId": "th-1",
                    "input": [{ "type": "text", "text": "ship it" }],
                    "effort": "high",
                    "personality": "friendly",
                }),
            }
        );
    }

    fn capable_model() -> ModelCapabilities {
        ModelCapabilities {
            id: "gpt-x".into(),
            display_name: "GPT-X".into(),
            is_default: true,
            reasoning_efforts: vec!["low".into(), "medium".into(), "high".into()],
            default_reasoning_effort: Some("medium".into()),
            supports_personality: true,
        }
    }

    /// Nothing advertised means nothing sent. This is the whole point of the
    /// gate: an unverified `effort` is a turn the CLI may reject outright, and
    /// an unverified `personality` is one the model quietly ignores.
    #[test]
    fn an_unverified_model_gets_no_mode_fields_at_all() {
        let mut modes = ComposerModes::default();
        modes.toggle("extended-thinking").expect("sticky");
        modes.toggle("style").expect("sticky");

        assert!(
            honoured_for(None).is_empty(),
            "nothing is honoured on faith"
        );

        let outbound = outbound_for(
            &RuntimeCommand::Prompt {
                provider_session_id: "th-1".into(),
                text: "ship it".into(),
                modes,
            },
            None,
        )
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

    /// Each field is gated by ITS own advertisement, independently.
    #[test]
    fn each_field_is_gated_by_its_own_advertisement() {
        let mut no_personality = capable_model();
        no_personality.supports_personality = false;
        assert_eq!(honoured_for(Some(&no_personality)), ["extended-thinking"]);

        let mut other_efforts = capable_model();
        other_efforts.reasoning_efforts = vec!["minimal".into(), "medium".into()];
        assert_eq!(
            honoured_for(Some(&other_efforts)),
            ["style"],
            "a model that does not advertise `high` refuses the mode by name"
        );

        let bare = ModelCapabilities::default();
        assert!(honoured_for(Some(&bare)).is_empty());
    }

    /// The refusal half, pinned. Web search has NO field in TurnStartParams —
    /// checked against the pinned CLI's own generated schema — so it must come
    /// back nameable rather than ride along doing nothing.
    #[test]
    fn a_mode_with_no_field_on_this_wire_is_refused_by_name() {
        let mut modes = ComposerModes::default();
        modes.toggle("web-search").expect("sticky");
        modes.toggle("extended-thinking").expect("sticky");

        let (carried, refused) = modes.split(&honoured_for(Some(&capable_model())));
        assert_eq!(
            carried.iter().map(|a| a.slug).collect::<Vec<_>>(),
            ["extended-thinking"]
        );
        assert_eq!(
            refused.iter().map(|a| a.slug).collect::<Vec<_>>(),
            ["web-search"]
        );

        // And the wire really does not carry it — against a model that DOES
        // advertise the honoured one, so the absence is about the field not
        // existing rather than about the model being unverified.
        let outbound = outbound_for(
            &RuntimeCommand::Prompt {
                provider_session_id: "th-1".into(),
                text: "ship it".into(),
                modes,
            },
            Some(&capable_model()),
        )
        .expect("prompt is implemented");
        let Outbound::Request { params, .. } = outbound else {
            panic!("a prompt is a request");
        };
        assert!(params.get("effort").is_some(), "the honoured one is there");
        assert!(
            params
                .as_object()
                .expect("object")
                .keys()
                .all(|k| k != "search"),
            "and nothing was invented for the refused one"
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
        assert!(outbound_for(&RuntimeCommand::ListSessions { cursor: None }, None).is_none());
        assert!(outbound_for(
            &RuntimeCommand::LoadSession {
                provider_session_id: "x".into()
            },
            None
        )
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
                failed: false,
                result: json!({"userAgent":"codex"}),
            })
        );
        assert_eq!(
            classify(r#"{"id":7,"error":{"code":-32601,"message":"nope"}}"#),
            Some(Frame::Response {
                id: 7,
                failed: true,
                result: Value::Null,
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

    /// `id` is the ONLY thing separating a server request from a
    /// notification, and reading one as the other is a hung turn: the spec
    /// says the agent blocks until the client replies with that id.
    #[test]
    fn a_server_request_is_told_from_a_notification_by_its_id() {
        assert_eq!(
            classify(
                r#"{"id":9,"method":"item/commandExecution/requestApproval","params":{"itemId":"i1"}}"#
            ),
            Some(Frame::ServerRequest {
                id: 9,
                method: "item/commandExecution/requestApproval".to_owned(),
                params: json!({"itemId":"i1"}),
            })
        );
        // Same method shape, no id — a notification, and nobody is waiting.
        assert_eq!(
            classify(r#"{"method":"turn/started","params":{"threadId":"th"}}"#),
            Some(Frame::Notification {
                method: "turn/started".to_owned(),
                params: json!({"threadId":"th"}),
            })
        );
    }

    /// The classification rule is the VENDORED envelope's, so it cannot drift
    /// from the protocol without this failing.
    #[test]
    fn the_frame_rule_matches_the_vendored_envelope() {
        let envelope: Value = serde_json::from_str(include_str!("../schema/JSONRPCMessage.json"))
            .expect("vendored envelope parses");
        let defs = envelope
            .get("definitions")
            .and_then(Value::as_object)
            .expect("definitions");
        let required = |name: &str| -> Vec<String> {
            let mut r: Vec<String> = defs[name]["required"]
                .as_array()
                .expect("required")
                .iter()
                .map(|v| v.as_str().expect("string").to_owned())
                .collect();
            r.sort();
            r
        };
        assert_eq!(
            required("JSONRPCRequest"),
            ["id", "method"],
            "server request"
        );
        assert_eq!(required("JSONRPCNotification"), ["method"], "no id, ever");
        assert_eq!(required("JSONRPCResponse"), ["id", "result"]);
        assert_eq!(required("JSONRPCError"), ["error", "id"]);
    }

    /// The policy is to run the LATEST Codex, so the vendored schema and the
    /// installed CLI must agree. When this fails the fix is to re-vendor, not
    /// to move the pin.
    ///
    /// Skipped where codex is not installed — CI without the CLI is a missing
    /// check, not a failing one, and pretending otherwise would make the suite
    /// red for a reason it cannot act on.
    #[test]
    fn the_vendored_schema_matches_the_installed_cli() {
        let vendored = include_str!("../schema/VERSION").trim().to_owned();
        assert!(
            vendored.contains(PINNED_CLI_VERSION),
            "VERSION ({vendored}) and PINNED_CLI_VERSION ({PINNED_CLI_VERSION}) disagree"
        );
        // Through dpl_proc::quiet like every other spawn in this workspace: on
        // Windows a bare Command flashes a console, and a test is not an
        // excuse for the one law that has no exceptions worth taking.
        let mut probe = std::process::Command::new("codex");
        let Ok(output) = dpl_proc::quiet(probe.arg("--version")).output() else {
            eprintln!("codex not installed — drift unchecked");
            return;
        };
        let installed = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        assert_eq!(
            installed, vendored,
            "installed codex has moved; re-vendor with \
             `codex app-server generate-json-schema --out crates/chat/providers/codex/schema`"
        );
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
            read_frames(
                BufReader::new(lines.as_bytes()),
                &tx,
                &tail,
                &Arc::new(Mutex::new(HashMap::new())),
            );
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

    /// The whole point of the in-flight map: a response body means nothing
    /// until you know which request it answers.
    #[test]
    fn a_model_list_response_is_claimed_and_read() {
        let (tx, rx) = sync_channel(64);
        let tail = Arc::new(Mutex::new(String::new()));
        let pending = Arc::new(Mutex::new(HashMap::from([(2_u64, "model/list")])));
        let line = json!({
            "id": 2,
            "result": {
                "data": [{
                    "id": "gpt-x",
                    "displayName": "GPT-X",
                    "isDefault": true,
                    "supportedReasoningEfforts": [
                        {"description": "quick", "reasoningEffort": "low"},
                        {"description": "thorough", "reasoningEffort": "high"}
                    ],
                    "defaultReasoningEffort": "low",
                    "supportsPersonality": true
                }]
            }
        })
        .to_string();
        read_frames(
            BufReader::new(format!("{line}\n").as_bytes()),
            &tx,
            &tail,
            &pending,
        );
        drop(tx);
        let events: Vec<_> = rx.into_iter().collect();
        let listed = events
            .iter()
            .find_map(|e| match e {
                RuntimeEvent::ModelsListed { models } => Some(models),
                _ => None,
            })
            .expect("the claimed response was read");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "gpt-x");
        assert!(listed[0].advertises_effort("high"));
        assert!(!listed[0].advertises_effort("xhigh"), "only what it said");
        assert!(listed[0].supports_personality);
        assert!(listed[0].is_default);

        // Claimed means claimed — the id is gone, so a duplicate reply cannot
        // be read twice.
        assert!(pending.lock().expect("lock").is_empty());
    }

    /// A refused catalogue leaves capabilities unverified, and unverified
    /// means every mode stays refused. It must SAY so.
    #[test]
    fn a_failed_model_list_is_named_not_swallowed() {
        let (tx, rx) = sync_channel(64);
        let tail = Arc::new(Mutex::new(String::new()));
        let pending = Arc::new(Mutex::new(HashMap::from([(2_u64, "model/list")])));
        read_frames(
            BufReader::new(
                "{\"id\":2,\"error\":{\"code\":-32601,\"message\":\"nope\"}}\n".as_bytes(),
            ),
            &tx,
            &tail,
            &pending,
        );
        drop(tx);
        let events: Vec<_> = rx.into_iter().collect();
        assert!(
            events.iter().any(|e| matches!(
                e,
                RuntimeEvent::Failure(failure) if failure.reason.contains("model/list")
            )),
            "the refusal is reported by name: {events:?}"
        );
    }

    /// An unclaimed id stays an acknowledgement, exactly as before: only
    /// `initialize`'s response means anything to the host on its own, and the
    /// rest report through notifications.
    #[test]
    fn a_plain_acknowledgement_produces_no_event() {
        let (tx, rx) = sync_channel(64);
        let tail = Arc::new(Mutex::new(String::new()));
        read_frames(
            BufReader::new("{\"id\":1,\"result\":{}}\n{\"id\":2,\"result\":{}}\n".as_bytes()),
            &tx,
            &tail,
            &Arc::new(Mutex::new(HashMap::new())),
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
