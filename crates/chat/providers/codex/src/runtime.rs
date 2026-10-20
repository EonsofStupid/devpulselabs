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
use dpl_chat_core::{AccountSummary, PermissionMode};
use dpl_chat_runtime::seam::{
    ProviderRuntime, RuntimeCapabilities, RuntimeCommand, RuntimeEvent, RuntimeFailure,
    RuntimeFailureKind, RuntimePermissionOption, RuntimePermissionOptionKind, RuntimeSendError,
    RuntimeUpdate,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, Stdio};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// The subcommand that speaks the protocol. `app-server` is marked
/// `[experimental]` by the vendor — recorded rather than hidden, because a
/// provider that stops answering must fail with a reason, not a shrug.
pub const APP_SERVER_ARGS: &[&str] = &["app-server"];

/// The CLI the vendored schema under `schema/` was generated from.
pub const PINNED_CLI_VERSION: &str = "0.163.0";

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

/// Every request the SERVER may send that this adapter answers. A server
/// request left unanswered stalls the turn that asked it, so each one gets a
/// response: the approval kinds become the operator's Approve / Deny card,
/// and the rest are refused by name.
pub const SERVER_REQUESTS_HANDLED: &[&str] = &[
    "item/commandExecution/requestApproval",
    "item/fileChange/requestApproval",
    "item/permissions/requestApproval",
    "applyPatchApproval",
    "execCommandApproval",
    "item/tool/call",
    "item/tool/requestUserInput",
    "mcpServer/elicitation/request",
    "attestation/generate",
    "account/chatgptAuthTokens/refresh",
];

/// A frame going out, before it is given a JSON-RPC id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outbound {
    /// A request expecting a response.
    Request { method: &'static str, params: Value },
    /// The answer to a server request, echoing the server's own id.
    Respond { id: Value, result: Value },
    /// A server request this client declines, by name.
    RespondError { id: Value, message: String },
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
pub fn outbound_for(command: &RuntimeCommand, mode: PermissionMode) -> Option<Outbound> {
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
        } => {
            let (sandbox, approval) = permission_params(mode);
            Some(Outbound::Request {
                method: "turn/start",
                // The operator's permission mode rides EVERY turn, so a
                // change applies from the next message without restarting
                // the thread.
                params: json!({
                    "threadId": provider_session_id,
                    "input": [{ "type": "text", "text": text }],
                    "sandboxPolicy": sandbox,
                    "approvalPolicy": approval,
                }),
            })
        }
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
        RuntimeCommand::ListSessions { .. } | RuntimeCommand::LoadSession { .. } => None,
        // Stateful: they need the approval ledger and the stored mode, so
        // `try_send` answers them rather than this pure mapping.
        RuntimeCommand::ResolvePermission { .. } | RuntimeCommand::SetPermissionMode { .. } => None,
    }
}

/// The Codex controls behind each permission mode: `(sandboxPolicy,
/// approvalPolicy)` for `turn/start`. "Read only" also asks, and the host
/// declines every ask, so a request still shows up in the chat as declined.
pub fn permission_params(mode: PermissionMode) -> (Value, Value) {
    let sandbox = match mode {
        PermissionMode::Workspace => json!({ "type": "workspaceWrite", "networkAccess": false }),
        PermissionMode::AskFirst | PermissionMode::ReadOnly => {
            json!({ "type": "readOnly", "networkAccess": false })
        }
    };
    (sandbox, json!("on-request"))
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
        permission_modes: true,
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

/// The approval kinds: the server requests that become the operator's
/// Approve / Deny card. Every other handled request is refused by name.
pub const APPROVAL_REQUESTS: &[&str] = &[
    "item/commandExecution/requestApproval",
    "item/fileChange/requestApproval",
    "item/permissions/requestApproval",
    "applyPatchApproval",
    "execCommandApproval",
];

/// The four choices the card offers, by the ids [`approval_response`] reads.
pub fn approval_options() -> Vec<RuntimePermissionOption> {
    let option = |id: &str, label: &str, kind| RuntimePermissionOption {
        option_id: id.to_owned(),
        label: label.to_owned(),
        kind,
    };
    vec![
        option(
            "allow",
            "Allow once",
            RuntimePermissionOptionKind::AllowOnce,
        ),
        option(
            "allow-session",
            "Allow for this chat",
            RuntimePermissionOptionKind::AllowAlways,
        ),
        option("deny", "Deny", RuntimePermissionOptionKind::RejectOnce),
        option(
            "stop",
            "Deny and stop",
            RuntimePermissionOptionKind::RejectAlways,
        ),
    ]
}

/// What an approval request is about, in a sentence the operator can judge,
/// and the thread it belongs to. `None` when `method` is not an approval.
pub fn approval_request(method: &str, params: &Value) -> Option<(String, String)> {
    if !APPROVAL_REQUESTS.contains(&method) {
        return None;
    }
    let text = |key: &str| params.get(key).and_then(Value::as_str).map(str::to_owned);
    // The v2 kinds name the thread; the legacy two name the conversation.
    let thread = text("threadId").or_else(|| text("conversationId"))?;
    let reason = text("reason")
        .map(|reason| format!(" ({reason})"))
        .unwrap_or_default();
    let summary = match method {
        "item/commandExecution/requestApproval" | "execCommandApproval" => {
            let command = match params.get("command") {
                Some(Value::String(command)) => command.clone(),
                Some(Value::Array(parts)) => parts
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" "),
                _ => "a command".to_owned(),
            };
            format!("Codex wants to run: {command}{reason}")
        }
        "item/fileChange/requestApproval" | "applyPatchApproval" => {
            format!("Codex wants to change files{reason}")
        }
        _ => format!("Codex wants more access than this chat allows{reason}"),
    };
    Some((thread, summary))
}

/// The answer to an approval request, in that request's own vocabulary.
/// No option, or one this client never offered, is a refusal.
pub fn approval_response(method: &str, option_id: Option<&str>, params: &Value) -> Value {
    let choice = option_id.unwrap_or("deny");
    match method {
        "item/permissions/requestApproval" => match choice {
            // Granting means echoing back exactly what was asked for.
            "allow" | "allow-session" => json!({
                "permissions": params.get("permissions").cloned().unwrap_or_else(|| json!({})),
                "scope": if choice == "allow" { "turn" } else { "session" },
            }),
            _ => json!({ "permissions": {} }),
        },
        "applyPatchApproval" | "execCommandApproval" => json!({
            "decision": match choice {
                "allow" => "approved",
                "allow-session" => "approved_for_session",
                "stop" => "abort",
                _ => "denied",
            }
        }),
        _ => json!({
            "decision": match choice {
                "allow" => "accept",
                "allow-session" => "acceptForSession",
                "stop" => "cancel",
                _ => "decline",
            }
        }),
    }
}

/// The account `account/read` reports. `None` is signed out. The email is
/// not kept: the card names the plan, never the person.
pub fn account_from(result: &Value) -> Option<AccountSummary> {
    let requires_auth = result
        .get("requiresOpenaiAuth")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let Some(account) = result.get("account").filter(|account| !account.is_null()) else {
        return (!requires_auth).then(|| AccountSummary {
            display_name: "No sign-in needed".to_owned(),
            plan: None,
        });
    };
    let kind = account
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Some(match kind {
        "chatgpt" => AccountSummary {
            display_name: "ChatGPT".to_owned(),
            plan: account
                .get("planType")
                .and_then(Value::as_str)
                .filter(|plan| *plan != "unknown")
                .map(plan_label),
        },
        "apiKey" => AccountSummary {
            display_name: "OpenAI API key".to_owned(),
            plan: None,
        },
        "amazonBedrock" => AccountSummary {
            display_name: "Amazon Bedrock".to_owned(),
            plan: None,
        },
        other => AccountSummary {
            display_name: format!("Codex account ({other})"),
            plan: None,
        },
    })
}

/// `plus` → `Plus`, `self_serve_business_prolite` → `Self Serve Business Prolite`.
fn plan_label(plan: &str) -> String {
    plan.split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut letters = word.chars();
            letters.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(letters).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// One decoded line from the server.
///
/// Responses omit `jsonrpc` entirely, so the ONLY reliable discriminator is
/// structural: a frame with a `method` and an `id` is a SERVER REQUEST (it
/// waits for an answer), one with a `method` and no `id` is a notification,
/// and one with an `id` and no `method` is a response. Measured on the live
/// protocol: reading a server request as a notification is how an approval
/// went unanswered and its turn hung.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Notification {
        method: String,
        params: Value,
    },
    /// A request from the server. `id` is echoed back verbatim in the answer.
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    /// The answer to one of OUR requests: its result, or the error's message.
    Response {
        id: u64,
        result: Value,
        error: Option<String>,
    },
}

/// Classify one line of the server's stdout.
///
/// `None` for anything unparseable — a partial or non-JSON line is noise from
/// a CLI that also prints diagnostics, never a protocol event.
pub fn classify(line: &str) -> Option<Frame> {
    let frame: Value = serde_json::from_str(line.trim()).ok()?;
    match (frame.get("method").and_then(Value::as_str), frame.get("id")) {
        (Some(method), Some(id)) => Some(Frame::Request {
            id: id.clone(),
            method: method.to_owned(),
            params: frame.get("params").cloned().unwrap_or(Value::Null),
        }),
        (Some(method), None) => Some(Frame::Notification {
            method: method.to_owned(),
            params: frame.get("params").cloned().unwrap_or(Value::Null),
        }),
        (None, Some(id)) => Some(Frame::Response {
            id: id.as_u64()?,
            result: frame.get("result").cloned().unwrap_or(Value::Null),
            error: frame.get("error").map(|error| {
                error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("the server reported an error without a message")
                    .to_owned()
            }),
        }),
        (None, None) => None,
    }
}

/// The id the handshake's `initialize` carries. Fixed, so the reader can
/// recognise the one response this adapter acts on without tracking state.
const INITIALIZE_ID: u64 = 1;

/// The id of the `account/read` sent straight after the handshake: the
/// account, not the process, is what "Ready" means.
const ACCOUNT_READ_ID: u64 = 2;

/// How long a turn may go without a single event before it is failed (and
/// interrupted) rather than left spinning. Paused while an approval waits on
/// the operator: a person deciding is not a hang.
pub const TURN_DEADLINE: Duration = Duration::from_secs(120);

/// How many commands may queue before `try_send` reports real backpressure.
const COMMAND_CAPACITY: usize = 32;

/// How many events may queue before the reader thread blocks on the host.
/// Generous: a fast turn emits a delta per token, and the host drains once a
/// frame.
const EVENT_CAPACITY: usize = 1024;

/// An approval waiting on the operator: what to answer, and in which
/// vocabulary.
#[derive(Debug, Clone, PartialEq)]
struct PendingApproval {
    server_id: Value,
    method: String,
    params: Value,
}

/// What the reader, the writer and the watchdog share.
#[derive(Debug, Default)]
struct Ledger {
    /// OUR request id → (its method, the thread it concerns), so a failed
    /// response fails the right turn.
    requests: HashMap<u64, (String, Option<String>)>,
    /// The handle given to the host → the server request it answers.
    approvals: HashMap<u64, PendingApproval>,
    next_approval: u64,
    /// The turn in flight and when it last showed life.
    turn: Option<(String, Instant)>,
}

impl Ledger {
    fn touch(&mut self, thread: Option<&str>) {
        if let (Some((current, last)), Some(thread)) = (self.turn.as_mut(), thread) {
            if current == thread {
                *last = Instant::now();
            }
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The live runtime: a `codex app-server` child, a worker, two queues.
pub struct CodexAppServerRuntime {
    commands: SyncSender<Outbound>,
    events: Mutex<Receiver<RuntimeEvent>>,
    capabilities: Mutex<Option<RuntimeCapabilities>>,
    ledger: Arc<Mutex<Ledger>>,
    mode: Mutex<PermissionMode>,
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
            ledger: Arc::new(Mutex::new(Ledger::default())),
            mode: Mutex::new(PermissionMode::default()),
            child: Mutex::new(None),
        }
    }

    /// Spawn `codex app-server` and wire the queues to it.
    ///
    /// Four threads, because the protocol is full-duplex and any one of them
    /// blocking must not stall the others:
    ///
    /// - **reader** — parses stdout, maps notifications through [`event_for`],
    ///   turns approval requests into the operator's card and refuses the
    ///   rest;
    /// - **writer** — owns stdin for its whole life (see below) and serves the
    ///   command queue;
    /// - **stderr drain** — a full stderr pipe wedges the child, and its last
    ///   line is the only explanation available when the child dies early;
    /// - **watchdog** — fails a turn that has gone silent past
    ///   [`TURN_DEADLINE`].
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
        let ledger = Arc::new(Mutex::new(Ledger::default()));
        let alive = Arc::new(std::sync::atomic::AtomicBool::new(true));

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

        let reader = Reader {
            events: event_tx.clone(),
            commands: command_tx.clone(),
            ledger: Arc::clone(&ledger),
            stderr_tail: Arc::clone(&stderr_tail),
        };
        let reader_alive = Arc::clone(&alive);
        thread::Builder::new()
            .name("devpulse-codex-reader".to_owned())
            .spawn(move || {
                reader.read(BufReader::new(stdout));
                reader_alive.store(false, std::sync::atomic::Ordering::Relaxed);
            })?;

        let watchdog_ledger = Arc::clone(&ledger);
        let watchdog_events = event_tx;
        let watchdog_commands = command_tx.clone();
        thread::Builder::new()
            .name("devpulse-codex-watchdog".to_owned())
            .spawn(move || {
                while alive.load(std::sync::atomic::Ordering::Relaxed) {
                    thread::sleep(Duration::from_secs(2));
                    let Some(thread_id) = overdue(&mut lock(&watchdog_ledger), Instant::now())
                    else {
                        continue;
                    };
                    let _ = watchdog_commands.try_send(Outbound::Request {
                        method: "turn/interrupt",
                        params: json!({ "threadId": thread_id }),
                    });
                    let failure = RuntimeEvent::Failure(RuntimeFailure::for_session(
                        RuntimeFailureKind::Timeout,
                        "Codex stopped responding: nothing for two minutes, so the turn was stopped",
                        thread_id,
                    ));
                    if watchdog_events.send(failure).is_err() {
                        return; // The host is gone.
                    }
                }
            })?;

        let writer_ledger = Arc::clone(&ledger);
        thread::Builder::new()
            .name("devpulse-codex-writer".to_owned())
            .spawn(move || {
                // The handshake, sent without waiting: `initialized` is
                // mandatory and the server accepts it pipelined behind
                // `initialize`. Proved against the live CLI — a real turn
                // completed with exactly this ordering. `account/read` rides
                // behind it: the account, not the process, means "Ready".
                let handshake = [
                    request_line(INITIALIZE_ID, "initialize", &initialize_params()),
                    json!({ "jsonrpc": "2.0", "method": "initialized" }).to_string(),
                    request_line(ACCOUNT_READ_ID, "account/read", &json!({})),
                ];
                for line in handshake {
                    if writeln!(stdin, "{line}")
                        .and_then(|()| stdin.flush())
                        .is_err()
                    {
                        return;
                    }
                }
                let mut next_id = ACCOUNT_READ_ID + 1;
                // A blocking recv: the writer sleeps until the host speaks, and
                // holds stdin open the entire time.
                while let Ok(outbound) = command_rx.recv() {
                    let line = match outbound {
                        Outbound::Request { method, params } => {
                            let id = next_id;
                            next_id = next_id.saturating_add(1);
                            let thread = params
                                .get("threadId")
                                .and_then(Value::as_str)
                                .map(str::to_owned);
                            let mut ledger = lock(&writer_ledger);
                            if method == "turn/start" {
                                ledger.turn = thread.clone().map(|thread| (thread, Instant::now()));
                            }
                            ledger.requests.insert(id, (method.to_owned(), thread));
                            drop(ledger);
                            request_line(id, method, &params)
                        }
                        Outbound::Respond { id, result } => {
                            json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
                        }
                        Outbound::RespondError { id, message } => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": { "code": -32601, "message": message },
                        })
                        .to_string(),
                        Outbound::Shutdown => break, // Fall through; release stdin.
                    };
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
            ledger,
            mode: Mutex::new(PermissionMode::default()),
            child: Mutex::new(Some(child)),
        })
    }
}

/// The thread of a turn that has been silent past [`TURN_DEADLINE`], if any;
/// it is cleared so it fails once. Never while an approval waits on the
/// operator.
fn overdue(ledger: &mut Ledger, now: Instant) -> Option<String> {
    if !ledger.approvals.is_empty() {
        if let Some((_, last)) = ledger.turn.as_mut() {
            *last = now;
        }
        return None;
    }
    let (_, last) = ledger.turn.as_ref()?;
    if now.duration_since(*last) < TURN_DEADLINE {
        return None;
    }
    ledger.turn.take().map(|(thread, _)| thread)
}

/// One JSON-RPC request, serialised. `jsonrpc` is sent even though the server
/// omits it in responses — the client half of the contract still requires it.
fn request_line(id: u64, method: &str, params: &Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string()
}

/// The reader's half of the runtime: everything it must reach to translate
/// one frame, including the writer's queue for the answers it gives itself.
struct Reader {
    events: SyncSender<RuntimeEvent>,
    commands: SyncSender<Outbound>,
    ledger: Arc<Mutex<Ledger>>,
    stderr_tail: Arc<Mutex<String>>,
}

impl Reader {
    /// Read frames until the child's stdout ends, translating as we go.
    ///
    /// Ends by emitting [`RuntimeEvent::Stopped`], so a child that dies is a
    /// state the host observes rather than a queue that quietly goes silent.
    fn read(&self, stdout: impl BufRead) {
        let mut connected = false;
        for line in stdout.lines().map_while(Result::ok) {
            let Some(frame) = classify(&line) else {
                continue;
            };
            let events = match frame {
                Frame::Notification { method, params } => self.notification(&method, &params),
                Frame::Request { id, method, params } => self.server_request(id, method, params),
                Frame::Response { id, result, error } => {
                    if id == INITIALIZE_ID {
                        connected = true;
                    }
                    self.response(id, &result, error)
                }
            };
            for event in events {
                if self.events.send(event).is_err() {
                    return; // The host is gone.
                }
            }
        }
        // Stdout closed. If it closed before `initialize` was ever answered, the
        // child failed to start properly and its stderr is the only explanation
        // anyone will get.
        if !connected {
            let reason = lock(&self.stderr_tail).clone();
            let _ = self.events.send(RuntimeEvent::Failure(RuntimeFailure::new(
                RuntimeFailureKind::Launch,
                if reason.is_empty() {
                    "codex app-server exited before completing the handshake".to_owned()
                } else {
                    format!("codex app-server exited during startup: {reason}")
                },
            )));
        }
        let _ = self.events.send(RuntimeEvent::Stopped);
    }

    fn notification(&self, method: &str, params: &Value) -> Vec<RuntimeEvent> {
        let thread = params.get("threadId").and_then(Value::as_str);
        let mut ledger = lock(&self.ledger);
        ledger.touch(thread);
        let finished = method == "turn/completed"
            || (method == "error"
                && !params
                    .get("willRetry")
                    .and_then(Value::as_bool)
                    .unwrap_or(false));
        if finished && ledger.turn.as_ref().map(|(current, _)| current.as_str()) == thread {
            ledger.turn = None;
        }
        drop(ledger);
        event_for(method, params).into_iter().collect()
    }

    /// Every server request gets an answer. Approvals go to the operator as a
    /// card; the rest are declined here, by name, so no turn waits on a reply
    /// that will never come.
    fn server_request(&self, id: Value, method: String, params: Value) -> Vec<RuntimeEvent> {
        if let Some((thread, summary)) = approval_request(&method, &params) {
            let mut ledger = lock(&self.ledger);
            ledger.next_approval = ledger.next_approval.saturating_add(1);
            let handle = ledger.next_approval;
            ledger.approvals.insert(
                handle,
                PendingApproval {
                    server_id: id,
                    method,
                    params,
                },
            );
            return vec![RuntimeEvent::PermissionRequested {
                request_id: handle,
                provider_session_id: thread,
                summary,
                options: approval_options(),
            }];
        }
        let message = if method == "account/chatgptAuthTokens/refresh" {
            "DevPULSE never holds your tokens; Codex manages its own sign-in".to_owned()
        } else {
            format!("DevPULSE chat does not support `{method}` yet")
        };
        let _ = self
            .commands
            .try_send(Outbound::RespondError { id, message });
        Vec::new()
    }

    fn response(&self, id: u64, result: &Value, error: Option<String>) -> Vec<RuntimeEvent> {
        match id {
            // `initialize` answering is what "connected" MEANS — not that the
            // process spawned. Capability comes from what this adapter can
            // honour, which is the seam's guarantee (see
            // `advertised_capabilities`).
            INITIALIZE_ID => vec![match error {
                Some(reason) => RuntimeEvent::Failure(RuntimeFailure::new(
                    RuntimeFailureKind::Protocol,
                    format!("codex app-server rejected the handshake: {reason}"),
                )),
                None => RuntimeEvent::Connected {
                    capabilities: advertised_capabilities(),
                },
            }],
            ACCOUNT_READ_ID => match error {
                // The account could not be read; the process still works, so
                // this is reported, not fatal.
                Some(reason) => vec![RuntimeEvent::Failure(RuntimeFailure::new(
                    RuntimeFailureKind::CredentialUnavailable,
                    format!("Codex could not report its account: {reason}"),
                ))],
                None => vec![RuntimeEvent::Account(account_from(result))],
            },
            _ => {
                let request = lock(&self.ledger).requests.remove(&id);
                let Some(reason) = error else {
                    // An acknowledgement; the work reports through
                    // notifications.
                    return Vec::new();
                };
                let (method, thread) = request.unwrap_or_else(|| ("a request".to_owned(), None));
                if method == "turn/start" {
                    lock(&self.ledger).turn = None;
                }
                let reason = format!("Codex refused {method}: {reason}");
                vec![RuntimeEvent::Failure(match thread {
                    Some(thread) => {
                        RuntimeFailure::for_session(RuntimeFailureKind::Protocol, reason, thread)
                    }
                    None => RuntimeFailure::new(RuntimeFailureKind::Protocol, reason),
                })]
            }
        }
    }
}

impl Drop for CodexAppServerRuntime {
    fn drop(&mut self) {
        // Closing the command queue ends the writer, which releases stdin and
        // asks `app-server` to exit on its own terms.
        let _ = self.commands.try_send(Outbound::Shutdown);
        if let Some(mut child) = lock(&self.child).take() {
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
        let outbound = match command {
            RuntimeCommand::SetPermissionMode { mode } => {
                *lock(&self.mode) = mode;
                return Ok(());
            }
            RuntimeCommand::ResolvePermission {
                request_id,
                option_id,
            } => {
                let Some(pending) = lock(&self.ledger).approvals.remove(&request_id) else {
                    // Already answered (or never ours): nothing to send.
                    return Ok(());
                };
                Outbound::Respond {
                    result: approval_response(
                        &pending.method,
                        option_id.as_deref(),
                        &pending.params,
                    ),
                    id: pending.server_id,
                }
            }
            command => outbound_for(&command, *lock(&self.mode))
                // Unimplemented AND unadvertised. Refusing loudly beats
                // pretending.
                .ok_or(RuntimeSendError::Closed)?,
        };
        self.commands
            .try_send(outbound)
            .map_err(|error| match error {
                TrySendError::Full(_) => RuntimeSendError::Full,
                TrySendError::Disconnected(_) => RuntimeSendError::Closed,
            })
    }

    fn try_event(&self) -> Result<Option<RuntimeEvent>, RuntimeSendError> {
        let events = lock(&self.events);
        match events.try_recv() {
            Ok(event) => {
                if let RuntimeEvent::Connected { capabilities } = &event {
                    *lock(&self.capabilities) = Some(*capabilities);
                }
                Ok(Some(event))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(RuntimeSendError::Closed),
        }
    }

    fn capabilities(&self) -> Option<RuntimeCapabilities> {
        *lock(&self.capabilities)
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
        let outbound = outbound_for(
            &RuntimeCommand::Prompt {
                provider_session_id: "th-1".into(),
                text: "ship it".into(),
            },
            PermissionMode::AskFirst,
        )
        .expect("prompt is implemented");
        assert_eq!(
            outbound,
            Outbound::Request {
                method: "turn/start",
                params: json!({
                    "threadId": "th-1",
                    "input": [{ "type": "text", "text": "ship it" }],
                    "sandboxPolicy": { "type": "readOnly", "networkAccess": false },
                    "approvalPolicy": "on-request",
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
        let mode = PermissionMode::default();
        assert!(outbound_for(&RuntimeCommand::ListSessions { cursor: None }, mode).is_none());
        assert!(outbound_for(
            &RuntimeCommand::LoadSession {
                provider_session_id: "x".into()
            },
            mode
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
                result: json!({"userAgent":"codex"}),
                error: None,
            })
        );
        assert_eq!(
            classify(r#"{"id":7,"error":{"code":-32601,"message":"nope"}}"#),
            Some(Frame::Response {
                id: 7,
                result: Value::Null,
                error: Some("nope".to_owned()),
            })
        );
        // A method AND an id: the server is asking, and waits for an answer.
        assert_eq!(
            classify(
                r#"{"id":"r1","method":"execCommandApproval","params":{"conversationId":"th"}}"#
            ),
            Some(Frame::Request {
                id: json!("r1"),
                method: "execCommandApproval".to_owned(),
                params: json!({"conversationId":"th"}),
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
    /// Drive the real reader over canned server lines: what it emitted, and
    /// what it queued for the writer (its own answers).
    fn drain_with(lines: &str, ledger: Arc<Mutex<Ledger>>) -> (Vec<RuntimeEvent>, Vec<Outbound>) {
        let (events, events_rx) = sync_channel(64);
        let (commands, commands_rx) = sync_channel(64);
        let reader = Reader {
            events,
            commands,
            ledger,
            stderr_tail: Arc::new(Mutex::new(String::new())),
        };
        reader.read(BufReader::new(lines.as_bytes()));
        drop(reader);
        (
            events_rx.into_iter().collect(),
            commands_rx.into_iter().collect(),
        )
    }

    fn drain(lines: &str) -> Vec<RuntimeEvent> {
        drain_with(lines, Arc::new(Mutex::new(Ledger::default()))).0
    }

    #[test]
    fn connected_is_emitted_only_when_initialize_is_answered() {
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
        let events = drain("{\"id\":1,\"result\":{}}\n{\"id\":9,\"result\":{}}\n");
        assert_eq!(events.len(), 2, "connected + stopped, and nothing for id 9");
    }

    /// An approval is the operator's call: it becomes a card, and the answer
    /// goes back in that request's own vocabulary.
    #[test]
    fn an_approval_request_becomes_a_card_and_is_answered_in_its_own_words() {
        let ledger = Arc::new(Mutex::new(Ledger::default()));
        let (events, answers) = drain_with(
            "{\"id\":\"r7\",\"method\":\"item/commandExecution/requestApproval\",\
             \"params\":{\"threadId\":\"th\",\"turnId\":\"t\",\"itemId\":\"i\",\
             \"startedAtMs\":1,\"command\":\"cargo test\"}}\n",
            Arc::clone(&ledger),
        );
        assert!(
            answers.is_empty(),
            "nothing is answered before the operator decides"
        );
        let Some(RuntimeEvent::PermissionRequested {
            request_id,
            provider_session_id,
            summary,
            options,
        }) = events.first()
        else {
            panic!("expected a card, got {events:?}");
        };
        assert_eq!(provider_session_id, "th");
        assert_eq!(summary, "Codex wants to run: cargo test");
        assert_eq!(options.len(), 4);

        let (commands, commands_rx) = sync_channel(8);
        let (_events_tx, events_rx) = sync_channel(8);
        let runtime = CodexAppServerRuntime::from_parts(commands, events_rx);
        *lock(&runtime.ledger) = std::mem::take(&mut *lock(&ledger));
        runtime
            .try_send(RuntimeCommand::ResolvePermission {
                request_id: *request_id,
                option_id: Some("allow".to_owned()),
            })
            .expect("answered");
        assert_eq!(
            commands_rx.try_recv().expect("an answer"),
            Outbound::Respond {
                id: json!("r7"),
                result: json!({ "decision": "accept" }),
            }
        );
    }

    #[test]
    fn each_approval_kind_is_answered_in_its_own_vocabulary() {
        let none = Value::Null;
        assert_eq!(
            approval_response("item/fileChange/requestApproval", Some("stop"), &none),
            json!({ "decision": "cancel" })
        );
        assert_eq!(
            approval_response("execCommandApproval", Some("allow-session"), &none),
            json!({ "decision": "approved_for_session" })
        );
        assert_eq!(
            approval_response("applyPatchApproval", None, &none),
            json!({ "decision": "denied" }),
            "no answer is a refusal"
        );
        let asked = json!({ "permissions": { "network": { "enabled": true } } });
        assert_eq!(
            approval_response("item/permissions/requestApproval", Some("allow"), &asked),
            json!({ "permissions": { "network": { "enabled": true } }, "scope": "turn" })
        );
        assert_eq!(
            approval_response("item/permissions/requestApproval", Some("deny"), &asked),
            json!({ "permissions": {} })
        );
    }

    /// Anything that is not an approval is declined by name at once, so no
    /// turn waits on a reply that will never come.
    #[test]
    fn any_other_server_request_is_refused_by_name() {
        let (events, answers) = drain_with(
            "{\"id\":4,\"method\":\"account/chatgptAuthTokens/refresh\",\"params\":{}}\n\
             {\"id\":5,\"method\":\"item/tool/call\",\"params\":{}}\n",
            Arc::new(Mutex::new(Ledger::default())),
        );
        assert_eq!(
            events,
            vec![
                RuntimeEvent::Failure(RuntimeFailure::new(
                    RuntimeFailureKind::Launch,
                    "codex app-server exited before completing the handshake",
                )),
                RuntimeEvent::Stopped
            ]
        );
        assert!(matches!(
            &answers[..],
            [
                Outbound::RespondError { id: first, message: tokens },
                Outbound::RespondError { id: second, message: tool },
            ] if *first == json!(4)
                && tokens.contains("never holds your tokens")
                && *second == json!(5)
                && tool.contains("item/tool/call")
        ));
    }

    /// A refused `turn/start` fails ITS turn with Codex's own words, instead
    /// of leaving the chat waiting.
    #[test]
    fn a_refused_request_fails_its_own_turn() {
        let ledger = Arc::new(Mutex::new(Ledger::default()));
        lock(&ledger)
            .requests
            .insert(3, ("turn/start".to_owned(), Some("th".to_owned())));
        lock(&ledger).turn = Some(("th".to_owned(), Instant::now()));
        let (events, _) = drain_with(
            "{\"id\":1,\"result\":{}}\n{\"id\":3,\"error\":{\"message\":\"model unavailable\"}}\n",
            Arc::clone(&ledger),
        );
        assert!(
            events.contains(&RuntimeEvent::Failure(RuntimeFailure::for_session(
                RuntimeFailureKind::Protocol,
                "Codex refused turn/start: model unavailable",
                "th",
            )))
        );
        assert_eq!(lock(&ledger).turn, None, "the turn is over");
    }

    /// "Ready" is the account, not the process.
    #[test]
    fn the_account_is_read_from_account_read() {
        assert_eq!(
            account_from(&json!({
                "requiresOpenaiAuth": true,
                "account": { "type": "chatgpt", "email": "x@y", "planType": "plus" },
            })),
            Some(AccountSummary {
                display_name: "ChatGPT".to_owned(),
                plan: Some("Plus".to_owned()),
            })
        );
        assert_eq!(
            account_from(&json!({ "requiresOpenaiAuth": true, "account": { "type": "apiKey" } }))
                .map(|account| account.display_name),
            Some("OpenAI API key".to_owned())
        );
        assert_eq!(
            account_from(&json!({ "requiresOpenaiAuth": true, "account": null })),
            None,
            "signed out"
        );
        let events = drain(
            "{\"id\":1,\"result\":{}}\n\
             {\"id\":2,\"result\":{\"requiresOpenaiAuth\":true,\"account\":null}}\n",
        );
        assert_eq!(events.get(1), Some(&RuntimeEvent::Account(None)));
    }

    /// A silent turn fails once at the deadline; a turn waiting on the
    /// operator's card never does.
    #[test]
    fn a_silent_turn_fails_at_the_deadline_but_never_while_the_operator_decides() {
        let start = Instant::now();
        let mut ledger = Ledger {
            turn: Some(("th".to_owned(), start)),
            ..Ledger::default()
        };
        assert_eq!(overdue(&mut ledger, start + TURN_DEADLINE / 2), None);
        ledger.approvals.insert(
            1,
            PendingApproval {
                server_id: json!(1),
                method: "execCommandApproval".to_owned(),
                params: Value::Null,
            },
        );
        assert_eq!(overdue(&mut ledger, start + TURN_DEADLINE * 3), None);
        ledger.approvals.clear();
        let decided = start + TURN_DEADLINE * 3;
        assert_eq!(overdue(&mut ledger, decided + TURN_DEADLINE / 2), None);
        assert_eq!(
            overdue(&mut ledger, decided + TURN_DEADLINE * 2),
            Some("th".to_owned())
        );
        assert_eq!(
            overdue(&mut ledger, decided + TURN_DEADLINE * 4),
            None,
            "it fails once"
        );
    }

    #[test]
    fn each_permission_mode_maps_to_codex_controls() {
        assert_eq!(
            permission_params(PermissionMode::Workspace),
            (
                json!({ "type": "workspaceWrite", "networkAccess": false }),
                json!("on-request")
            )
        );
        for mode in [PermissionMode::AskFirst, PermissionMode::ReadOnly] {
            assert_eq!(
                permission_params(mode).0,
                json!({ "type": "readOnly", "networkAccess": false })
            );
        }
        assert!(advertised_capabilities().permission_modes);
    }

    #[test]
    fn the_pinned_schema_still_declares_every_server_request_we_answer() {
        let schema = include_str!("../schema/ServerRequest.json");
        for request in SERVER_REQUESTS_HANDLED.iter().chain(APPROVAL_REQUESTS) {
            assert!(
                schema.contains(&format!("\"{request}\"")),
                "{request} is gone from the pinned schema ({PINNED_CLI_VERSION})"
            );
        }
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

    /// The constant and the vendored files move together: `vendor-schema`
    /// writes `VERSION`, and this is what keeps the code from claiming a
    /// different build than the schema it was checked against.
    #[test]
    fn the_pinned_version_is_the_version_the_schema_came_from() {
        let vendored = include_str!("../schema/VERSION").trim();
        assert_eq!(vendored, format!("codex-cli {PINNED_CLI_VERSION}"));
    }
}
