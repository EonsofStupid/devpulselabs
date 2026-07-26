use super::{allowed_environment_keys, hide_child_console, GrokRuntimeProfile};
use crate::terminal::AcpTerminalStore;
use agent_client_protocol::{
    schema::{
        v1::{
            AuthenticateRequest, CancelNotification, ClientCapabilities, ContentBlock,
            CreateTerminalRequest, CreateTerminalResponse, FileSystemCapabilities, Implementation,
            InitializeRequest, KillTerminalRequest, KillTerminalResponse, ListSessionsRequest,
            LoadSessionRequest, NewSessionRequest, PermissionOptionKind, PromptRequest,
            ReadTextFileRequest, ReadTextFileResponse, ReleaseTerminalRequest,
            ReleaseTerminalResponse, RequestPermissionOutcome, RequestPermissionRequest,
            RequestPermissionResponse, SelectedPermissionOutcome, SessionId, SessionNotification,
            SessionUpdate, TerminalOutputRequest, TerminalOutputResponse, TextContent,
            WaitForTerminalExitRequest, WaitForTerminalExitResponse, WriteTextFileRequest,
            WriteTextFileResponse,
        },
        ProtocolVersion,
    },
    Agent, ByteStreams, Client, ConnectTo, ConnectionTo, Error, Responder,
};
use async_channel::{Receiver, Sender, TryRecvError, TrySendError};
use async_process::Child;
use futures_lite::future;
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fmt,
    future::Future,
    io,
    path::PathBuf,
    process::Stdio,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

const COMMAND_CAPACITY: usize = 32;
const EVENT_CAPACITY: usize = 256;
const MAX_PROMPT_BYTES: usize = 64 * 1024;
const MAX_CURSOR_BYTES: usize = 4 * 1024;
const MAX_FILE_BYTES: usize = 4 * 1024 * 1024;
const MAX_READ_LINES: usize = 10_000;
const MAX_PERMISSION_SUMMARY_BYTES: usize = 8 * 1024;
const MAX_SESSION_UPDATE_BYTES: usize = 64 * 1024;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const ACTION_TIMEOUT: Duration = Duration::from_secs(30);
const PROMPT_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const CACHED_TOKEN_AUTH: &str = "cached_token";
const CLIENT_NAME: &str = "devpulse-chat";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrokLaunchSpec {
    pub executable: PathBuf,
    pub arguments: [OsString; 3],
    pub working_directory: PathBuf,
    pub environment: Vec<(OsString, OsString)>,
}

impl GrokLaunchSpec {
    #[must_use]
    pub fn from_profile(profile: &GrokRuntimeProfile) -> Self {
        let environment = allowed_environment_keys()
            .iter()
            .filter_map(|key| std::env::var_os(key).map(|value| (OsString::from(key), value)))
            .collect();
        Self {
            executable: profile.executable().to_owned(),
            arguments: [
                OsString::from("--no-auto-update"),
                OsString::from("agent"),
                OsString::from("stdio"),
            ],
            working_directory: profile.working_directory().to_owned(),
            environment,
        }
    }

    fn command(&self) -> async_process::Command {
        let mut command = std::process::Command::new(&self.executable);
        command
            .args(&self.arguments)
            .current_dir(&self.working_directory)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (key, value) in &self.environment {
            command.env(key, value);
        }
        hide_child_console(&mut command);
        let mut command = async_process::Command::from(command);
        command.kill_on_drop(true);
        command
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrokAcpCapabilities {
    pub load_session: bool,
    pub list_sessions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrokSessionSummary {
    pub provider_session_id: String,
    pub title: Option<String>,
    pub working_directory: PathBuf,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GrokSessionUpdate {
    UserText(String),
    AssistantText(String),
    ThoughtText(String),
    Tool(String),
    Plan(String),
    SessionInfo(String),
    Usage { used: u64, size: u64 },
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GrokRuntimeEvent {
    Connected {
        capabilities: GrokAcpCapabilities,
    },
    SessionsPage {
        sessions: Vec<GrokSessionSummary>,
        next_cursor: Option<String>,
        replace: bool,
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
        update: GrokSessionUpdate,
    },
    PermissionRequested {
        request_id: u64,
        provider_session_id: String,
        summary: String,
        options: Vec<GrokPermissionOption>,
    },
    TurnFinished {
        provider_session_id: String,
        stop_reason: String,
    },
    Failure(GrokRuntimeFailure),
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrokPermissionOptionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrokPermissionOption {
    pub option_id: String,
    pub label: String,
    pub kind: GrokPermissionOptionKind,
}

#[derive(Debug)]
struct PendingPermission {
    provider_session_id: String,
    decision: Sender<Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrokRuntimeFailureKind {
    Launch,
    Protocol,
    Authentication,
    Unsupported,
    InvalidCommand,
    Backpressure,
    Timeout,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrokRuntimeFailure {
    pub kind: GrokRuntimeFailureKind,
    pub reason: String,
    pub provider_session_id: Option<String>,
}

impl GrokRuntimeFailure {
    fn new(kind: GrokRuntimeFailureKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            reason: reason.into(),
            provider_session_id: None,
        }
    }

    fn for_session(
        kind: GrokRuntimeFailureKind,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrokRuntimeCommand {
    ListSessions {
        cursor: Option<String>,
    },
    NewSession,
    LoadSession {
        provider_session_id: String,
    },
    Prompt {
        provider_session_id: String,
        text: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrokRuntimeSendError {
    Full,
    Closed,
}

impl fmt::Display for GrokRuntimeSendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => formatter.write_str("the Grok command queue is full"),
            Self::Closed => formatter.write_str("the Grok runtime is closed"),
        }
    }
}

impl std::error::Error for GrokRuntimeSendError {}

#[derive(Debug)]
pub struct GrokAcpRuntime {
    commands: Sender<GrokRuntimeCommand>,
    events: Receiver<GrokRuntimeEvent>,
    worker: Option<JoinHandle<()>>,
}

impl GrokAcpRuntime {
    pub fn start(profile: GrokRuntimeProfile) -> io::Result<Self> {
        let launch = GrokLaunchSpec::from_profile(&profile);
        let working_directory = profile.working_directory().to_owned();
        let (command_tx, command_rx) = async_channel::bounded(COMMAND_CAPACITY);
        let (event_tx, event_rx) = async_channel::bounded(EVENT_CAPACITY);
        let worker = thread::Builder::new()
            .name("devpulse-grok-acp".to_owned())
            .spawn(move || {
                async_io::block_on(async move {
                    production_worker(launch, working_directory, command_rx, event_tx).await;
                });
            })?;
        Ok(Self {
            commands: command_tx,
            events: event_rx,
            worker: Some(worker),
        })
    }

    pub fn try_send(&self, command: GrokRuntimeCommand) -> Result<(), GrokRuntimeSendError> {
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                TrySendError::Full(_) => GrokRuntimeSendError::Full,
                TrySendError::Closed(_) => GrokRuntimeSendError::Closed,
            })
    }

    pub fn try_event(&self) -> Result<Option<GrokRuntimeEvent>, GrokRuntimeSendError> {
        match self.events.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Closed) => Err(GrokRuntimeSendError::Closed),
        }
    }
}

impl Drop for GrokAcpRuntime {
    fn drop(&mut self) {
        let _ = self.commands.try_send(GrokRuntimeCommand::Shutdown);
        self.commands.close();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

async fn production_worker(
    launch: GrokLaunchSpec,
    working_directory: PathBuf,
    commands: Receiver<GrokRuntimeCommand>,
    events: Sender<GrokRuntimeEvent>,
) {
    let mut command = launch.command();
    let spawn = command.spawn();
    let mut child = match spawn {
        Ok(child) => child,
        Err(error) => {
            let _ = events
                .send(GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                    GrokRuntimeFailureKind::Launch,
                    public_io_reason(&error),
                )))
                .await;
            events.close();
            return;
        }
    };

    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let terminal_failure_emitted = Arc::new(AtomicBool::new(false));
    let authenticated = Arc::new(AtomicBool::new(false));
    let result = match (stdin, stdout) {
        (Some(stdin), Some(stdout)) => {
            run_connection(
                ByteStreams::new(stdin, stdout),
                working_directory,
                commands,
                events.clone(),
                Arc::clone(&terminal_failure_emitted),
                Arc::clone(&authenticated),
            )
            .await
        }
        _ => Err(Error::internal_error()),
    };

    terminate_and_reap(&mut child).await;
    if let Err(error) = result {
        if !terminal_failure_emitted.load(Ordering::SeqCst) {
            let (kind, reason) = unclassified_connection_failure(
                authenticated.load(Ordering::SeqCst),
                &error.to_string(),
            );
            let _ = events
                .send(GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                    kind, reason,
                )))
                .await;
        }
    }
    let _ = events.send(GrokRuntimeEvent::Stopped).await;
    events.close();
}

async fn terminate_and_reap(child: &mut Child) {
    if child.try_status().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.status().await;
}

async fn run_connection(
    transport: impl ConnectTo<Client> + 'static,
    working_directory: PathBuf,
    commands: Receiver<GrokRuntimeCommand>,
    events: Sender<GrokRuntimeEvent>,
    terminal_failure_emitted: Arc<AtomicBool>,
    authenticated: Arc<AtomicBool>,
) -> Result<(), Error> {
    let notification_events = events.clone();
    let permission_events = events.clone();
    let pending_permissions = Arc::new(Mutex::new(BTreeMap::<u64, PendingPermission>::new()));
    let permission_waiters = Arc::clone(&pending_permissions);
    let permission_sequence = Arc::new(AtomicU64::new(1));
    let permission_ids = Arc::clone(&permission_sequence);
    let read_root = working_directory.clone();
    let write_root = working_directory.clone();
    let command_permissions = Arc::clone(&pending_permissions);
    let terminals = AcpTerminalStore::new(working_directory.clone());
    let create_terminals = terminals.clone();
    let output_terminals = terminals.clone();
    let release_terminals = terminals.clone();
    let kill_terminals = terminals.clone();
    let wait_terminals = terminals.clone();
    let result = Client
        .builder()
        .on_receive_notification(
            async move |notification: SessionNotification, _connection: ConnectionTo<Agent>| {
                let event = map_notification(notification);
                emit(&notification_events, event)
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |request: RequestPermissionRequest,
                        responder: Responder<RequestPermissionResponse>,
                        _connection: ConnectionTo<Agent>| {
                let request_id = permission_ids.fetch_add(1, Ordering::Relaxed);
                let options = request
                    .options
                    .iter()
                    .map(|option| GrokPermissionOption {
                        option_id: option.option_id.0.to_string(),
                        label: option.name.clone(),
                        kind: map_permission_kind(option.kind),
                    })
                    .collect::<Vec<_>>();
                let summary = bounded_text(
                    &format!("{:?}", request.tool_call),
                    MAX_PERMISSION_SUMMARY_BYTES,
                );
                let (decision_tx, decision_rx) = async_channel::bounded(1);
                permission_waiters
                    .lock()
                    .map_err(|_| Error::internal_error())?
                    .insert(
                        request_id,
                        PendingPermission {
                            provider_session_id: request.session_id.0.to_string(),
                            decision: decision_tx,
                        },
                    );
                emit(
                    &permission_events,
                    GrokRuntimeEvent::PermissionRequested {
                        request_id,
                        provider_session_id: request.session_id.0.to_string(),
                        summary,
                        options,
                    },
                )?;
                let decision = decision_rx.recv().await.ok().flatten();
                let outcome = decision.map_or(RequestPermissionOutcome::Cancelled, |option_id| {
                    RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option_id))
                });
                responder.respond(RequestPermissionResponse::new(outcome))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: ReadTextFileRequest,
                        responder: Responder<ReadTextFileResponse>,
                        _connection: ConnectionTo<Agent>| {
                match read_workspace_text(&read_root, &request) {
                    Ok(content) => responder.respond(ReadTextFileResponse::new(content)),
                    Err(error) => responder.respond_with_internal_error(error),
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: WriteTextFileRequest,
                        responder: Responder<WriteTextFileResponse>,
                        _connection: ConnectionTo<Agent>| {
                match write_workspace_text(&write_root, &request) {
                    Ok(()) => responder.respond(WriteTextFileResponse::new()),
                    Err(error) => responder.respond_with_internal_error(error),
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: CreateTerminalRequest,
                        responder: Responder<CreateTerminalResponse>,
                        _connection: ConnectionTo<Agent>| {
                match create_terminals.create(request) {
                    Ok(response) => responder.respond(response),
                    Err(error) => responder.respond_with_internal_error(error),
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: TerminalOutputRequest,
                        responder: Responder<TerminalOutputResponse>,
                        _connection: ConnectionTo<Agent>| {
                match output_terminals.output(request) {
                    Ok(response) => responder.respond(response),
                    Err(error) => responder.respond_with_internal_error(error),
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: ReleaseTerminalRequest,
                        responder: Responder<ReleaseTerminalResponse>,
                        _connection: ConnectionTo<Agent>| {
                match release_terminals.release(request) {
                    Ok(response) => responder.respond(response),
                    Err(error) => responder.respond_with_internal_error(error),
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: KillTerminalRequest,
                        responder: Responder<KillTerminalResponse>,
                        _connection: ConnectionTo<Agent>| {
                match kill_terminals.kill(request) {
                    Ok(response) => responder.respond(response),
                    Err(error) => responder.respond_with_internal_error(error),
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: WaitForTerminalExitRequest,
                        responder: Responder<WaitForTerminalExitResponse>,
                        _connection: ConnectionTo<Agent>| {
                match wait_terminals.wait(request).await {
                    Ok(response) => responder.respond(response),
                    Err(error) => responder.respond_with_internal_error(error),
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(transport, async move |connection| {
            let initialize = InitializeRequest::new(ProtocolVersion::V1)
                .client_capabilities(
                    ClientCapabilities::new()
                        .fs(FileSystemCapabilities::new()
                            .read_text_file(true)
                            .write_text_file(true))
                        .terminal(true),
                )
                .client_info(
                    Implementation::new(CLIENT_NAME, env!("CARGO_PKG_VERSION"))
                        .title("DevPULSE Chat"),
                )
                .meta(headless_meta());
            let initialized = match request_with_timeout(
                connection.send_request(initialize).block_task(),
                STARTUP_TIMEOUT,
            )
            .await
            {
                Ok(initialized) => initialized,
                Err(TimedRequestError::Protocol(error)) => {
                    terminal_failure_emitted.store(true, Ordering::SeqCst);
                    emit(
                        &events,
                        GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                            GrokRuntimeFailureKind::Protocol,
                            format!("Grok ACP initialization failed: {error}"),
                        )),
                    )?;
                    return Err(error);
                }
                Err(TimedRequestError::Timeout) => {
                    terminal_failure_emitted.store(true, Ordering::SeqCst);
                    emit(
                        &events,
                        GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                            GrokRuntimeFailureKind::Timeout,
                            "Grok ACP initialization timed out.",
                        )),
                    )?;
                    return Err(Error::internal_error());
                }
            };
            if initialized.protocol_version != ProtocolVersion::V1 {
                terminal_failure_emitted.store(true, Ordering::SeqCst);
                emit(
                    &events,
                    GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                        GrokRuntimeFailureKind::Protocol,
                        format!(
                            "Grok negotiated unsupported ACP protocol {}",
                            initialized.protocol_version
                        ),
                    )),
                )?;
                return Err(Error::invalid_request());
            }

            let cached_method = initialized
                .auth_methods
                .iter()
                .find(|method| method.id().0.as_ref() == CACHED_TOKEN_AUTH)
                .map(|method| method.id().clone());
            let Some(cached_method) = cached_method else {
                terminal_failure_emitted.store(true, Ordering::SeqCst);
                emit(
                    &events,
                    GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                        GrokRuntimeFailureKind::Authentication,
                        "Grok did not advertise cached subscription authentication.",
                    )),
                )?;
                return Err(Error::invalid_request());
            };
            match request_with_timeout(
                connection
                    .send_request(AuthenticateRequest::new(cached_method).meta(headless_meta()))
                    .block_task(),
                STARTUP_TIMEOUT,
            )
            .await
            {
                Ok(_) => {}
                Err(TimedRequestError::Protocol(error)) => {
                    terminal_failure_emitted.store(true, Ordering::SeqCst);
                    emit(
                        &events,
                        GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                            GrokRuntimeFailureKind::Authentication,
                            format!("Grok cached subscription authentication failed: {error}"),
                        )),
                    )?;
                    return Err(error);
                }
                Err(TimedRequestError::Timeout) => {
                    terminal_failure_emitted.store(true, Ordering::SeqCst);
                    emit(
                        &events,
                        GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                            GrokRuntimeFailureKind::Timeout,
                            "Grok cached subscription authentication timed out.",
                        )),
                    )?;
                    return Err(Error::internal_error());
                }
            }
            authenticated.store(true, Ordering::SeqCst);

            let capabilities = GrokAcpCapabilities {
                load_session: initialized.agent_capabilities.load_session,
                list_sessions: initialized
                    .agent_capabilities
                    .session_capabilities
                    .list
                    .is_some(),
            };
            emit(&events, GrokRuntimeEvent::Connected { capabilities })?;

            command_loop(
                connection,
                working_directory,
                capabilities,
                commands,
                events,
                command_permissions,
            )
            .await
        })
        .await;
    terminals.shutdown();
    result
}

fn unclassified_connection_failure(
    authenticated: bool,
    protocol_reason: &str,
) -> (GrokRuntimeFailureKind, String) {
    if authenticated {
        (
            GrokRuntimeFailureKind::Disconnected,
            format!("Grok ACP disconnected: {protocol_reason}"),
        )
    } else {
        (
            GrokRuntimeFailureKind::Authentication,
            "Grok could not establish a cached subscription session. Log in to Grok and retry."
                .to_owned(),
        )
    }
}

async fn command_loop(
    connection: ConnectionTo<Agent>,
    working_directory: PathBuf,
    capabilities: GrokAcpCapabilities,
    commands: Receiver<GrokRuntimeCommand>,
    events: Sender<GrokRuntimeEvent>,
    pending_permissions: Arc<Mutex<BTreeMap<u64, PendingPermission>>>,
) -> Result<(), Error> {
    let active_turns = Arc::new(Mutex::new(BTreeSet::<String>::new()));
    while let Ok(command) = commands.recv().await {
        match command {
            GrokRuntimeCommand::ListSessions { cursor } => {
                if !capabilities.list_sessions {
                    emit(
                        &events,
                        GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                            GrokRuntimeFailureKind::Unsupported,
                            "This Grok runtime does not support session listing.",
                        )),
                    )?;
                    continue;
                }
                if cursor
                    .as_ref()
                    .is_some_and(|value| value.len() > MAX_CURSOR_BYTES)
                {
                    emit(
                        &events,
                        GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                            GrokRuntimeFailureKind::InvalidCommand,
                            "The Grok session cursor exceeded the bounded limit.",
                        )),
                    )?;
                    continue;
                }
                let replace = cursor.is_none();
                let response = match request_with_timeout(
                    connection
                        .send_request(
                            ListSessionsRequest::new()
                                .cwd(working_directory.clone())
                                .cursor(cursor),
                        )
                        .block_task(),
                    ACTION_TIMEOUT,
                )
                .await
                {
                    Ok(response) => response,
                    Err(TimedRequestError::Protocol(error)) => return Err(error),
                    Err(TimedRequestError::Timeout) => {
                        emit(
                            &events,
                            GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                                GrokRuntimeFailureKind::Timeout,
                                "Listing Grok sessions timed out.",
                            )),
                        )?;
                        continue;
                    }
                };
                let sessions = response
                    .sessions
                    .into_iter()
                    .map(|session| GrokSessionSummary {
                        provider_session_id: session.session_id.0.to_string(),
                        title: session.title,
                        working_directory: session.cwd,
                        updated_at: session.updated_at,
                    })
                    .collect();
                emit(
                    &events,
                    GrokRuntimeEvent::SessionsPage {
                        sessions,
                        next_cursor: response.next_cursor,
                        replace,
                    },
                )?;
            }
            GrokRuntimeCommand::NewSession => {
                let response = match request_with_timeout(
                    connection
                        .send_request(NewSessionRequest::new(working_directory.clone()))
                        .block_task(),
                    ACTION_TIMEOUT,
                )
                .await
                {
                    Ok(response) => response,
                    Err(TimedRequestError::Protocol(error)) => return Err(error),
                    Err(TimedRequestError::Timeout) => {
                        emit(
                            &events,
                            GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                                GrokRuntimeFailureKind::Timeout,
                                "Creating a Grok session timed out.",
                            )),
                        )?;
                        continue;
                    }
                };
                emit(
                    &events,
                    GrokRuntimeEvent::SessionReady {
                        provider_session_id: response.session_id.0.to_string(),
                        is_new: true,
                    },
                )?;
            }
            GrokRuntimeCommand::LoadSession {
                provider_session_id,
            } => {
                if !capabilities.load_session {
                    emit(
                        &events,
                        GrokRuntimeEvent::Failure(GrokRuntimeFailure::for_session(
                            GrokRuntimeFailureKind::Unsupported,
                            "This Grok runtime does not support loading sessions.",
                            provider_session_id,
                        )),
                    )?;
                    continue;
                }
                match request_with_timeout(
                    connection
                        .send_request(LoadSessionRequest::new(
                            SessionId::new(provider_session_id.clone()),
                            working_directory.clone(),
                        ))
                        .block_task(),
                    ACTION_TIMEOUT,
                )
                .await
                {
                    Ok(_) => {}
                    Err(TimedRequestError::Protocol(error)) => return Err(error),
                    Err(TimedRequestError::Timeout) => {
                        emit(
                            &events,
                            GrokRuntimeEvent::Failure(GrokRuntimeFailure::for_session(
                                GrokRuntimeFailureKind::Timeout,
                                "Loading the Grok session timed out.",
                                provider_session_id,
                            )),
                        )?;
                        continue;
                    }
                }
                emit(
                    &events,
                    GrokRuntimeEvent::SessionReady {
                        provider_session_id,
                        is_new: false,
                    },
                )?;
            }
            GrokRuntimeCommand::Prompt {
                provider_session_id,
                text,
            } => {
                if text.trim().is_empty() || text.len() > MAX_PROMPT_BYTES {
                    emit(
                        &events,
                        GrokRuntimeEvent::Failure(GrokRuntimeFailure::for_session(
                            GrokRuntimeFailureKind::InvalidCommand,
                            "The Grok message was empty or exceeded the bounded limit.",
                            provider_session_id,
                        )),
                    )?;
                    continue;
                }
                {
                    let mut active = active_turns.lock().map_err(|_| Error::internal_error())?;
                    if !active.insert(provider_session_id.clone()) {
                        emit(
                            &events,
                            GrokRuntimeEvent::Failure(GrokRuntimeFailure::for_session(
                                GrokRuntimeFailureKind::InvalidCommand,
                                "A Grok response is already running for this session.",
                                provider_session_id,
                            )),
                        )?;
                        continue;
                    }
                }
                emit(
                    &events,
                    GrokRuntimeEvent::TurnStarted {
                        provider_session_id: provider_session_id.clone(),
                    },
                )?;
                let prompt_connection = connection.clone();
                let prompt_events = events.clone();
                let prompt_active = Arc::clone(&active_turns);
                connection.spawn(async move {
                    let result = request_with_timeout(
                        prompt_connection
                            .send_request(PromptRequest::new(
                                SessionId::new(provider_session_id.clone()),
                                vec![ContentBlock::Text(TextContent::new(text))],
                            ))
                            .block_task(),
                        PROMPT_TIMEOUT,
                    )
                    .await;
                    if let Ok(mut active) = prompt_active.lock() {
                        active.remove(&provider_session_id);
                    }
                    match result {
                        Ok(response) => emit(
                            &prompt_events,
                            GrokRuntimeEvent::TurnFinished {
                                provider_session_id,
                                stop_reason: format!("{:?}", response.stop_reason),
                            },
                        ),
                        Err(TimedRequestError::Protocol(error)) => emit(
                            &prompt_events,
                            GrokRuntimeEvent::Failure(GrokRuntimeFailure::for_session(
                                GrokRuntimeFailureKind::Disconnected,
                                format!("The Grok response failed: {error}"),
                                provider_session_id,
                            )),
                        ),
                        Err(TimedRequestError::Timeout) => emit(
                            &prompt_events,
                            GrokRuntimeEvent::Failure(GrokRuntimeFailure::for_session(
                                GrokRuntimeFailureKind::Timeout,
                                "The Grok response exceeded the bounded turn timeout.",
                                provider_session_id,
                            )),
                        ),
                    }
                })?;
            }
            GrokRuntimeCommand::Cancel {
                provider_session_id,
            } => {
                cancel_session_permissions(&pending_permissions, &provider_session_id)?;
                connection.send_notification(CancelNotification::new(SessionId::new(
                    provider_session_id,
                )))?;
            }
            GrokRuntimeCommand::ResolvePermission {
                request_id,
                option_id,
            } => {
                let pending = pending_permissions
                    .lock()
                    .map_err(|_| Error::internal_error())?
                    .remove(&request_id);
                if let Some(pending) = pending {
                    let _ = pending.decision.try_send(option_id);
                } else {
                    emit(
                        &events,
                        GrokRuntimeEvent::Failure(GrokRuntimeFailure::new(
                            GrokRuntimeFailureKind::InvalidCommand,
                            "The Grok permission request is no longer pending.",
                        )),
                    )?;
                }
            }
            GrokRuntimeCommand::Shutdown => {
                cancel_all_permissions(&pending_permissions)?;
                return Ok(());
            }
        }
    }
    Ok(())
}

fn map_permission_kind(kind: PermissionOptionKind) -> GrokPermissionOptionKind {
    match kind {
        PermissionOptionKind::AllowOnce => GrokPermissionOptionKind::AllowOnce,
        PermissionOptionKind::AllowAlways => GrokPermissionOptionKind::AllowAlways,
        PermissionOptionKind::RejectOnce => GrokPermissionOptionKind::RejectOnce,
        PermissionOptionKind::RejectAlways => GrokPermissionOptionKind::RejectAlways,
        _ => GrokPermissionOptionKind::RejectOnce,
    }
}

fn read_workspace_text(
    root: &std::path::Path,
    request: &ReadTextFileRequest,
) -> Result<String, String> {
    let path = canonical_workspace_path(root, &request.path)?;
    let metadata = std::fs::metadata(&path).map_err(|error| public_io_reason(&error))?;
    if !metadata.is_file() {
        return Err("the requested path is not a file".to_owned());
    }
    if metadata.len() > MAX_FILE_BYTES as u64 {
        return Err("the requested file exceeds the bounded read limit".to_owned());
    }
    let content = std::fs::read_to_string(path).map_err(|error| public_io_reason(&error))?;
    let start = request.line.unwrap_or(1).saturating_sub(1) as usize;
    let limit = request
        .limit
        .map_or(MAX_READ_LINES, |limit| limit as usize)
        .min(MAX_READ_LINES);
    let selected = content
        .lines()
        .skip(start)
        .take(limit)
        .collect::<Vec<_>>()
        .join("\n");
    Ok(bounded_text(&selected, MAX_FILE_BYTES))
}

fn write_workspace_text(
    root: &std::path::Path,
    request: &WriteTextFileRequest,
) -> Result<(), String> {
    if request.content.len() > MAX_FILE_BYTES {
        return Err("the requested write exceeds the bounded file limit".to_owned());
    }
    let path = if request.path.exists() {
        canonical_workspace_path(root, &request.path)?
    } else {
        let parent = request
            .path
            .parent()
            .ok_or_else(|| "the requested file has no parent directory".to_owned())?;
        let parent = canonical_workspace_path(root, parent)?;
        let file_name = request
            .path
            .file_name()
            .ok_or_else(|| "the requested file name is invalid".to_owned())?;
        parent.join(file_name)
    };
    std::fs::write(path, request.content.as_bytes()).map_err(|error| public_io_reason(&error))
}

fn canonical_workspace_path(
    root: &std::path::Path,
    path: &std::path::Path,
) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("the provider requested a non-absolute path".to_owned());
    }
    let root = std::fs::canonicalize(root).map_err(|error| public_io_reason(&error))?;
    let canonical = std::fs::canonicalize(path).map_err(|error| public_io_reason(&error))?;
    if !canonical.starts_with(&root) {
        return Err("the provider requested a path outside the approved workspace".to_owned());
    }
    Ok(canonical)
}

fn bounded_text(value: &str, max_bytes: usize) -> String {
    let mut boundary = value.len().min(max_bytes);
    while boundary > 0 && !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value[..boundary].to_owned()
}

fn cancel_session_permissions(
    pending: &Arc<Mutex<BTreeMap<u64, PendingPermission>>>,
    provider_session_id: &str,
) -> Result<(), Error> {
    let mut pending = pending.lock().map_err(|_| Error::internal_error())?;
    let request_ids = pending
        .iter()
        .filter_map(|(request_id, request)| {
            (request.provider_session_id == provider_session_id).then_some(*request_id)
        })
        .collect::<Vec<_>>();
    for request_id in request_ids {
        if let Some(request) = pending.remove(&request_id) {
            let _ = request.decision.try_send(None);
        }
    }
    Ok(())
}

fn cancel_all_permissions(
    pending: &Arc<Mutex<BTreeMap<u64, PendingPermission>>>,
) -> Result<(), Error> {
    let mut pending = pending.lock().map_err(|_| Error::internal_error())?;
    for (_, request) in std::mem::take(&mut *pending) {
        let _ = request.decision.try_send(None);
    }
    Ok(())
}

fn map_notification(notification: SessionNotification) -> GrokRuntimeEvent {
    let provider_session_id = notification.session_id.0.to_string();
    let update = match notification.update {
        SessionUpdate::UserMessageChunk(chunk) => {
            GrokSessionUpdate::UserText(content_text(chunk.content))
        }
        SessionUpdate::AgentMessageChunk(chunk) => {
            GrokSessionUpdate::AssistantText(content_text(chunk.content))
        }
        SessionUpdate::AgentThoughtChunk(chunk) => {
            GrokSessionUpdate::ThoughtText(content_text(chunk.content))
        }
        SessionUpdate::ToolCall(value) => GrokSessionUpdate::Tool(bounded_text(
            &format!("{value:?}"),
            MAX_SESSION_UPDATE_BYTES,
        )),
        SessionUpdate::ToolCallUpdate(value) => GrokSessionUpdate::Tool(bounded_text(
            &format!("{value:?}"),
            MAX_SESSION_UPDATE_BYTES,
        )),
        SessionUpdate::Plan(value) => GrokSessionUpdate::Plan(bounded_text(
            &format!("{value:?}"),
            MAX_SESSION_UPDATE_BYTES,
        )),
        SessionUpdate::SessionInfoUpdate(value) => GrokSessionUpdate::SessionInfo(bounded_text(
            &format!("{value:?}"),
            MAX_SESSION_UPDATE_BYTES,
        )),
        SessionUpdate::UsageUpdate(value) => GrokSessionUpdate::Usage {
            used: value.used,
            size: value.size,
        },
        other => GrokSessionUpdate::Other(bounded_text(
            &format!("{other:?}"),
            MAX_SESSION_UPDATE_BYTES,
        )),
    };
    GrokRuntimeEvent::SessionUpdate {
        provider_session_id,
        update,
    }
}

fn content_text(content: ContentBlock) -> String {
    match content {
        ContentBlock::Text(text) => bounded_text(&text.text, MAX_SESSION_UPDATE_BYTES),
        ContentBlock::Image(image) => format!(
            "[image: {}, {} encoded bytes{}]",
            image.mime_type,
            image.data.len(),
            image
                .uri
                .as_deref()
                .map_or_else(String::new, |uri| format!(", {uri}"))
        ),
        ContentBlock::Audio(audio) => format!(
            "[audio: {}, {} encoded bytes]",
            audio.mime_type,
            audio.data.len()
        ),
        ContentBlock::ResourceLink(resource) => {
            format!("[resource: {} — {}]", resource.name, resource.uri)
        }
        ContentBlock::Resource(resource) => bounded_text(
            &format!("[embedded resource: {:?}]", resource.resource),
            MAX_SESSION_UPDATE_BYTES,
        ),
        other => bounded_text(&format!("{other:?}"), MAX_SESSION_UPDATE_BYTES),
    }
}

enum TimedRequestError {
    Protocol(Error),
    Timeout,
}

async fn request_with_timeout<T, F>(request: F, timeout: Duration) -> Result<T, TimedRequestError>
where
    F: Future<Output = Result<T, Error>>,
{
    future::race(
        async move { request.await.map_err(TimedRequestError::Protocol) },
        async move {
            async_io::Timer::after(timeout).await;
            Err(TimedRequestError::Timeout)
        },
    )
    .await
}

fn emit(events: &Sender<GrokRuntimeEvent>, event: GrokRuntimeEvent) -> Result<(), Error> {
    events.try_send(event).map_err(|error| {
        let _kind = match error {
            TrySendError::Full(_) => GrokRuntimeFailureKind::Backpressure,
            TrySendError::Closed(_) => GrokRuntimeFailureKind::Disconnected,
        };
        Error::internal_error()
    })
}

fn headless_meta() -> serde_json::Map<String, serde_json::Value> {
    serde_json::json!({ "headless": true })
        .as_object()
        .cloned()
        .unwrap_or_default()
}

fn public_io_reason(error: &io::Error) -> String {
    match error.kind() {
        io::ErrorKind::NotFound => "the Grok executable was not found".to_owned(),
        io::ErrorKind::PermissionDenied => "permission to start Grok was denied".to_owned(),
        io::ErrorKind::TimedOut => "starting Grok timed out".to_owned(),
        kind => format!("Grok could not start ({kind:?})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GrokVersion, MINIMUM_VERSION, QUALIFIED_VERSION};
    use agent_client_protocol::schema::v1::{
        AgentCapabilities, AuthMethod, AuthMethodAgent, AuthenticateResponse, ContentChunk,
        ImageContent, InitializeResponse, ListSessionsResponse, LoadSessionResponse,
        NewSessionResponse, PermissionOption, PromptResponse, SessionCapabilities, SessionInfo,
        SessionListCapabilities, StopReason, ToolCallUpdate, ToolCallUpdateFields,
    };
    use std::{
        ffi::OsStr,
        sync::atomic::{AtomicBool, Ordering},
    };

    fn fixture_profile() -> GrokRuntimeProfile {
        GrokRuntimeProfile {
            executable: PathBuf::from("grok"),
            version: GrokVersion::new(
                QUALIFIED_VERSION.major,
                QUALIFIED_VERSION.minor,
                QUALIFIED_VERSION.patch,
            ),
            working_directory: std::env::current_dir().expect("current directory"),
        }
    }

    #[test]
    fn launch_is_fixed_and_environment_is_allowlisted() {
        let launch = GrokLaunchSpec::from_profile(&fixture_profile());
        assert_eq!(
            launch
                .arguments
                .iter()
                .map(OsString::as_os_str)
                .collect::<Vec<_>>(),
            vec![
                OsStr::new("--no-auto-update"),
                OsStr::new("agent"),
                OsStr::new("stdio"),
            ]
        );
        assert!(launch.environment.iter().all(|(key, _)| {
            allowed_environment_keys().contains(&key.to_string_lossy().as_ref())
        }));
        assert!(!launch.environment.iter().any(|(key, _)| {
            key.to_string_lossy()
                .eq_ignore_ascii_case("DANGER_PAT_TOKEN")
        }));
    }

    #[test]
    fn startup_disconnect_is_recoverable_subscription_authentication() {
        let (kind, reason) = unclassified_connection_failure(false, "Internal error");
        assert_eq!(kind, GrokRuntimeFailureKind::Authentication);
        assert!(reason.contains("Log in to Grok"));

        let (kind, reason) = unclassified_connection_failure(true, "transport closed");
        assert_eq!(kind, GrokRuntimeFailureKind::Disconnected);
        assert!(reason.contains("transport closed"));
    }

    #[test]
    fn protocol_peer_proves_list_new_stream_and_cancel() {
        async_io::block_on(async {
            let cancelled = Arc::new(AtomicBool::new(false));
            let cancel_flag = Arc::clone(&cancelled);
            let permission_approved = Arc::new(AtomicBool::new(false));
            let permission_flag = Arc::clone(&permission_approved);
            let cancel_observed = Arc::clone(&cancelled);
            let agent = Agent
                .builder()
                .on_receive_request(
                    async move |_request: InitializeRequest,
                                responder: Responder<InitializeResponse>,
                                _connection: ConnectionTo<Client>| {
                        responder.respond(
                            InitializeResponse::new(ProtocolVersion::V1)
                                .agent_capabilities(
                                    AgentCapabilities::new()
                                        .load_session(true)
                                        .session_capabilities(
                                            SessionCapabilities::new()
                                                .list(SessionListCapabilities::new()),
                                        ),
                                )
                                .auth_methods(vec![AuthMethod::Agent(AuthMethodAgent::new(
                                    CACHED_TOKEN_AUTH,
                                    "Cached subscription",
                                ))]),
                        )
                    },
                    agent_client_protocol::on_receive_request!(),
                )
                .on_receive_request(
                    async move |_request: AuthenticateRequest,
                                responder: Responder<AuthenticateResponse>,
                                _connection: ConnectionTo<Client>| {
                        responder.respond(AuthenticateResponse::new())
                    },
                    agent_client_protocol::on_receive_request!(),
                )
                .on_receive_request(
                    async move |_request: ListSessionsRequest,
                                responder: Responder<ListSessionsResponse>,
                                _connection: ConnectionTo<Client>| {
                        responder.respond(ListSessionsResponse::new(vec![SessionInfo::new(
                            "saved",
                            std::env::current_dir().unwrap(),
                        )
                        .title("Saved Grok chat")]))
                    },
                    agent_client_protocol::on_receive_request!(),
                )
                .on_receive_request(
                    async move |_request: NewSessionRequest,
                                responder: Responder<NewSessionResponse>,
                                _connection: ConnectionTo<Client>| {
                        responder.respond(NewSessionResponse::new("new-grok"))
                    },
                    agent_client_protocol::on_receive_request!(),
                )
                .on_receive_request(
                    async move |_request: LoadSessionRequest,
                                responder: Responder<LoadSessionResponse>,
                                _connection: ConnectionTo<Client>| {
                        responder.respond(LoadSessionResponse::new())
                    },
                    agent_client_protocol::on_receive_request!(),
                )
                .on_receive_request(
                    async move |request: PromptRequest,
                                responder: Responder<PromptResponse>,
                                connection: ConnectionTo<Client>| {
                        let prompt_connection = connection.clone();
                        let prompt_permission_flag = Arc::clone(&permission_flag);
                        connection.spawn(async move {
                            let permission = prompt_connection
                                .send_request(RequestPermissionRequest::new(
                                    request.session_id.clone(),
                                    ToolCallUpdate::new(
                                        "tool-1",
                                        ToolCallUpdateFields::new().title("Run focused tests"),
                                    ),
                                    vec![
                                        PermissionOption::new(
                                            "allow-once",
                                            "Allow once",
                                            PermissionOptionKind::AllowOnce,
                                        ),
                                        PermissionOption::new(
                                            "always",
                                            "Always allow",
                                            PermissionOptionKind::AllowAlways,
                                        ),
                                        PermissionOption::new(
                                            "deny",
                                            "Deny",
                                            PermissionOptionKind::RejectOnce,
                                        ),
                                    ],
                                ))
                                .block_task()
                                .await?;
                            if matches!(
                                permission.outcome,
                                RequestPermissionOutcome::Selected(ref selected)
                                    if selected.option_id.0.as_ref() == "allow-once"
                            ) {
                                prompt_permission_flag.store(true, Ordering::SeqCst);
                            }
                            prompt_connection.send_notification(SessionNotification::new(
                                request.session_id,
                                SessionUpdate::AgentMessageChunk(ContentChunk::new(
                                    ContentBlock::Text(TextContent::new("hello from Grok")),
                                )),
                            ))?;
                            responder.respond(PromptResponse::new(StopReason::EndTurn))
                        })?;
                        Ok(())
                    },
                    agent_client_protocol::on_receive_request!(),
                )
                .on_receive_notification(
                    async move |_notification: CancelNotification,
                                _connection: ConnectionTo<Client>| {
                        cancel_flag.store(true, Ordering::SeqCst);
                        Ok(())
                    },
                    agent_client_protocol::on_receive_notification!(),
                );
            let (commands_tx, commands_rx) = async_channel::bounded(COMMAND_CAPACITY);
            let (events_tx, events_rx) = async_channel::bounded(EVENT_CAPACITY);
            let driver = async move {
                commands_tx
                    .send(GrokRuntimeCommand::ListSessions { cursor: None })
                    .await
                    .unwrap();
                commands_tx
                    .send(GrokRuntimeCommand::NewSession)
                    .await
                    .unwrap();
                commands_tx
                    .send(GrokRuntimeCommand::LoadSession {
                        provider_session_id: "saved".to_owned(),
                    })
                    .await
                    .unwrap();
                commands_tx
                    .send(GrokRuntimeCommand::Prompt {
                        provider_session_id: "new-grok".to_owned(),
                        text: "hello".to_owned(),
                    })
                    .await
                    .unwrap();
                let mut saw_sessions = false;
                let mut saw_session = false;
                let mut saw_loaded_session = false;
                let mut saw_text = false;
                let mut saw_finished = false;
                while !(saw_sessions
                    && saw_session
                    && saw_loaded_session
                    && saw_text
                    && saw_finished)
                {
                    enum NextEvent {
                        Event(Result<GrokRuntimeEvent, async_channel::RecvError>),
                        Timeout,
                    }
                    let next = futures_lite::future::race(
                        async { NextEvent::Event(events_rx.recv().await) },
                        async {
                            async_io::Timer::after(Duration::from_secs(5)).await;
                            NextEvent::Timeout
                        },
                    )
                    .await;
                    let NextEvent::Event(event) = next else {
                        panic!(
                            "protocol fixture stalled: sessions={saw_sessions}, \
                             new={saw_session}, loaded={saw_loaded_session}, \
                             text={saw_text}, finished={saw_finished}"
                        );
                    };
                    match event.unwrap() {
                        GrokRuntimeEvent::SessionsPage { sessions, .. } => {
                            saw_sessions = sessions[0].provider_session_id == "saved";
                        }
                        GrokRuntimeEvent::SessionReady {
                            provider_session_id,
                            is_new,
                        } => {
                            if is_new {
                                saw_session = provider_session_id == "new-grok";
                            } else {
                                saw_loaded_session = provider_session_id == "saved";
                            }
                        }
                        GrokRuntimeEvent::SessionUpdate {
                            update: GrokSessionUpdate::AssistantText(text),
                            ..
                        } => saw_text = text == "hello from Grok",
                        GrokRuntimeEvent::PermissionRequested {
                            request_id,
                            options,
                            ..
                        } => {
                            assert_eq!(options.len(), 3);
                            commands_tx
                                .send(GrokRuntimeCommand::ResolvePermission {
                                    request_id,
                                    option_id: Some("allow-once".to_owned()),
                                })
                                .await
                                .unwrap();
                        }
                        GrokRuntimeEvent::TurnFinished { .. } => saw_finished = true,
                        _ => {}
                    }
                }
                commands_tx
                    .send(GrokRuntimeCommand::Cancel {
                        provider_session_id: "new-grok".to_owned(),
                    })
                    .await
                    .unwrap();
                for _ in 0..200 {
                    if cancel_observed.load(Ordering::SeqCst) {
                        break;
                    }
                    async_io::Timer::after(Duration::from_millis(10)).await;
                }
                assert!(
                    cancel_observed.load(Ordering::SeqCst),
                    "protocol fixture did not observe cancellation"
                );
                commands_tx
                    .send(GrokRuntimeCommand::Shutdown)
                    .await
                    .unwrap();
            };
            let runtime = run_connection(
                agent,
                std::env::current_dir().expect("current directory"),
                commands_rx,
                events_tx,
                Arc::new(AtomicBool::new(false)),
                Arc::new(AtomicBool::new(false)),
            );
            let (result, ()) = futures_lite::future::zip(runtime, driver).await;
            result.expect("protocol runtime");
            assert!(cancelled.load(Ordering::SeqCst));
            assert!(permission_approved.load(Ordering::SeqCst));
        });
    }

    #[test]
    fn workspace_file_access_is_bounded_to_the_approved_root() {
        let test_name = std::thread::current()
            .name()
            .unwrap_or("test")
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                    character
                } else {
                    '-'
                }
            })
            .collect::<String>();
        let unique = format!("devpulse-grok-runtime-{}-{}", std::process::id(), test_name);
        let fixture_root = std::env::temp_dir().join(unique);
        let approved = fixture_root.join("approved");
        let outside = fixture_root.join("outside");
        std::fs::create_dir_all(&approved).expect("approved directory");
        std::fs::create_dir_all(&outside).expect("outside directory");
        let approved_file = approved.join("inside.txt");
        let outside_file = outside.join("outside.txt");
        std::fs::write(&approved_file, "one\ntwo\nthree").expect("inside fixture");
        std::fs::write(&outside_file, "secret").expect("outside fixture");

        let read = ReadTextFileRequest::new("session", &approved_file);
        assert_eq!(
            read_workspace_text(&approved, &read).expect("approved read"),
            "one\ntwo\nthree"
        );
        let write_path = approved.join("created.txt");
        write_workspace_text(
            &approved,
            &WriteTextFileRequest::new("session", &write_path, "created"),
        )
        .expect("approved write");
        assert_eq!(
            std::fs::read_to_string(&write_path).expect("written fixture"),
            "created"
        );

        let outside_read = ReadTextFileRequest::new("session", &outside_file);
        assert!(read_workspace_text(&approved, &outside_read)
            .expect_err("outside read must fail")
            .contains("outside the approved workspace"));
        let outside_write = WriteTextFileRequest::new("session", &outside_file, "overwrite");
        assert!(write_workspace_text(&approved, &outside_write)
            .expect_err("outside write must fail")
            .contains("outside the approved workspace"));
        assert_eq!(
            std::fs::read_to_string(&outside_file).expect("outside remains"),
            "secret"
        );

        std::fs::remove_dir_all(&fixture_root).expect("remove fixture tree");
    }

    #[test]
    fn command_backpressure_is_explicit_and_bounded() {
        let (commands, _receiver) = async_channel::bounded(COMMAND_CAPACITY);
        let (_events, event_receiver) = async_channel::bounded(EVENT_CAPACITY);
        let runtime = GrokAcpRuntime {
            commands,
            events: event_receiver,
            worker: None,
        };
        for _ in 0..COMMAND_CAPACITY {
            runtime
                .try_send(GrokRuntimeCommand::ListSessions { cursor: None })
                .expect("bounded slot");
        }
        assert_eq!(
            runtime.try_send(GrokRuntimeCommand::ListSessions { cursor: None }),
            Err(GrokRuntimeSendError::Full)
        );
    }

    #[test]
    fn request_timeout_is_typed_and_bounded() {
        async_io::block_on(async {
            let request = std::future::pending::<Result<(), Error>>();
            let result = request_with_timeout(request, Duration::from_millis(1)).await;
            assert!(matches!(result, Err(TimedRequestError::Timeout)));
        });
    }

    #[test]
    fn media_updates_never_project_encoded_payloads_into_chat_text() {
        let content = ContentBlock::Image(
            ImageContent::new("sensitive-base64-payload", "image/png")
                .uri("file:///approved/preview.png"),
        );
        let summary = content_text(content);
        assert!(summary.contains("image/png"));
        assert!(summary.contains("file:///approved/preview.png"));
        assert!(!summary.contains("sensitive-base64-payload"));
    }

    #[test]
    fn text_updates_are_utf8_safe_and_bounded() {
        let text = "é".repeat(MAX_SESSION_UPDATE_BYTES);
        let projected = content_text(ContentBlock::Text(TextContent::new(text)));
        assert!(projected.len() <= MAX_SESSION_UPDATE_BYTES);
        assert!(projected.is_char_boundary(projected.len()));
    }

    #[test]
    fn qualified_fixture_stays_on_the_activation_floor() {
        assert_eq!(fixture_profile().version(), MINIMUM_VERSION);
    }
}
