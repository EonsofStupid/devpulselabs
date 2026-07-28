// spawn-audit: exempt(test harness — a visible target app console aids debugging)
//! Process-isolated scenario, artifact, trace, and visual runner.
//!
//! Only profiles and fixtures declared in `.eguipilotai.toml` may be launched.
//! Callers never pass executable paths, environment variables, shell strings,
//! or artifact destinations through the control protocol.

use egui_pilot_ai_protocol as protocol;
use egui_pilot_ai_transport::{read_frame, write_frame, TransportError};
use protocol::{
    AdapterPolicy, CaptureResult, ClientCommand, CoachCommand, CoachEvent, CoachId,
    CommandEnvelope, CommandResult, Flow, Handshake, HandshakeRequest, PilotError, Project,
    RendererIdentity, RequestId, ResponseOutcome, RunId, RunResult, RuntimeEvent,
    RuntimeEventEnvelope, ServerMessage, SessionId, SnapshotRequest, ViewportSelector,
    PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const CONFIG_FILE: &str = ".eguipilotai.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EguiPilotConfig {
    #[serde(default = "default_artifact_root")]
    pub artifact_root: PathBuf,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
    #[serde(default)]
    pub fixtures: BTreeMap<String, Fixture>,
    #[serde(default)]
    pub projects: Vec<Project>,
}

fn default_artifact_root() -> PathBuf {
    PathBuf::from("target/eguipilotai/runs")
}

impl Default for EguiPilotConfig {
    fn default() -> Self {
        Self {
            artifact_root: default_artifact_root(),
            profiles: BTreeMap::new(),
            fixtures: BTreeMap::new(),
            projects: Vec::new(),
        }
    }
}

impl EguiPilotConfig {
    pub fn load(workspace: impl AsRef<Path>) -> Result<Self, RunnerError> {
        let workspace = workspace.as_ref();
        let path = workspace.join(CONFIG_FILE);
        let text = fs::read_to_string(&path)
            .map_err(|error| RunnerError::config(format!("{}: {error}", path.display())))?;
        let config: Self = toml::from_str(&text)
            .map_err(|error| RunnerError::config(format!("{}: {error}", path.display())))?;
        config.validate(workspace)?;
        Ok(config)
    }

    pub fn validate(&self, workspace: &Path) -> Result<(), RunnerError> {
        if self.profiles.is_empty() {
            return Err(RunnerError::config(
                "at least one named profile is required",
            ));
        }
        validate_relative_root(&self.artifact_root)?;
        for (name, profile) in &self.profiles {
            profile.validate(name, workspace)?;
        }
        for project in &self.projects {
            if !self.profiles.contains_key(&project.profile) {
                return Err(RunnerError::config(format!(
                    "project `{}` names missing profile `{}`",
                    project.id, project.profile
                )));
            }
            if project.workers != 1 {
                return Err(RunnerError::config(format!(
                    "project `{}` requests workers={}; parallel workers are unsupported until slice 4",
                    project.id, project.workers
                )));
            }
            if project.retries != 0 {
                return Err(RunnerError::config(format!(
                    "project `{}` requests retries={}; isolated retries are unsupported until slice 4",
                    project.id, project.retries
                )));
            }
        }
        Ok(())
    }

    pub fn profile(&self, name: &str) -> Result<&Profile, RunnerError> {
        self.profiles.get(name).ok_or_else(|| {
            RunnerError::config(format!(
                "unknown profile `{name}`; select a declared profile"
            ))
        })
    }

    pub fn fixture(&self, name: &str) -> Result<&Fixture, RunnerError> {
        self.fixtures.get(name).ok_or_else(|| {
            RunnerError::config(format!(
                "unknown fixture `{name}`; select a declared fixture"
            ))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub executable: PathBuf,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
    #[serde(default)]
    pub adapter_policy: AdapterPolicy,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    30_000
}

impl Profile {
    pub fn supports_current_platform(&self) -> bool {
        self.platforms.is_empty()
            || self
                .platforms
                .iter()
                .any(|platform| platform == std::env::consts::OS)
    }

    fn validate(&self, name: &str, workspace: &Path) -> Result<(), RunnerError> {
        if self.executable.as_os_str().is_empty() {
            return Err(RunnerError::config(format!(
                "profile `{name}` has no executable"
            )));
        }
        if self.timeout_ms == 0 {
            return Err(RunnerError::config(format!(
                "profile `{name}` timeout must be greater than zero"
            )));
        }
        if self.executable.is_relative() {
            validate_relative_root(&self.executable)?;
            let _ = workspace.join(&self.executable);
        }
        if self
            .arguments
            .iter()
            .any(|argument| argument.contains('\n') || argument.contains('\0'))
        {
            return Err(RunnerError::config(format!(
                "profile `{name}` contains an invalid argument"
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
}

/// A launch-time window size.
///
/// Layout is size-dependent — the rail accordion has no room to unfold in a
/// small window — so a caller that needs a real size must be able to ask for
/// one. It stays inside the "no shell strings from callers" law by being
/// bounded and typed: the command line is rebuilt from parsed integers, so a
/// caller's text can never become a new flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}

impl WindowSize {
    /// Below the app's own `with_min_inner_size`, layout is out of contract.
    pub const MIN: u32 = 320;
    pub const MAX: u32 = 16_384;

    pub fn new(width: u32, height: u32) -> Result<Self, RunnerError> {
        for (axis, value) in [("width", width), ("height", height)] {
            if !(Self::MIN..=Self::MAX).contains(&value) {
                return Err(RunnerError::config(format!(
                    "window {axis} {value} is outside {}..={}",
                    Self::MIN,
                    Self::MAX
                )));
            }
        }
        Ok(Self { width, height })
    }

    fn argument(self) -> String {
        format!("{}x{}", self.width, self.height)
    }
}

impl std::str::FromStr for WindowSize {
    type Err = RunnerError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (width, height) = text.split_once(['x', 'X']).ok_or_else(|| {
            RunnerError::config(format!(
                "window size `{text}` must read <width>x<height>, as in 1280x832"
            ))
        })?;
        let axis = |name: &str, value: &str| {
            value.trim().parse::<u32>().map_err(|_| {
                RunnerError::config(format!("window {name} `{value}` is not a whole number"))
            })
        };
        Self::new(axis("width", width)?, axis("height", height)?)
    }
}

#[derive(Debug)]
pub enum RunnerError {
    Config(String),
    Io(String),
    Transport(String),
    Protocol(String),
    Timeout(String),
    Target(Box<PilotError>),
    Visual(String),
}

impl RunnerError {
    fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }
}

impl std::fmt::Display for RunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(message) => write!(f, "configuration: {message}"),
            Self::Io(message) => write!(f, "I/O: {message}"),
            Self::Transport(message) => write!(f, "transport: {message}"),
            Self::Protocol(message) => write!(f, "protocol: {message}"),
            Self::Timeout(message) => write!(f, "timeout: {message}"),
            Self::Target(error) => write!(f, "target {}: {}", error.code, error.message),
            Self::Visual(message) => write!(f, "visual: {message}"),
        }
    }
}

impl std::error::Error for RunnerError {}

impl From<std::io::Error> for RunnerError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<TransportError> for RunnerError {
    fn from(value: TransportError) -> Self {
        Self::Transport(value.to_string())
    }
}

enum TargetMessage {
    Message(Box<ServerMessage>),
    Error(RunnerError),
    Eof,
}

/// One separately launched native target. The framed protocol is private to
/// the child's inherited pipes and authenticated with a launch nonce.
pub struct ProcessSession {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    inbound: Receiver<TargetMessage>,
    stderr: Receiver<Vec<u8>>,
    pub session: SessionId,
    next_request: u64,
    next_run: u64,
    outstanding: HashSet<RequestId>,
    responses: HashMap<RequestId, ResponseOutcome>,
    events: VecDeque<RuntimeEventEnvelope>,
    last_event_sequence: u64,
    timeout: Duration,
    pub run_dir: PathBuf,
    pub handshake: Option<Handshake>,
    trace: BufWriter<fs::File>,
}

impl ProcessSession {
    pub fn launch(
        workspace: impl AsRef<Path>,
        config: &EguiPilotConfig,
        profile_name: &str,
        fixture_name: Option<&str>,
        window: Option<WindowSize>,
    ) -> Result<Self, RunnerError> {
        let workspace = workspace.as_ref();
        let profile = config.profile(profile_name)?;
        if !profile.supports_current_platform() {
            return Err(RunnerError::config(format!(
                "profile `{profile_name}` does not support `{}`",
                std::env::consts::OS
            )));
        }
        let fixture = fixture_name.map(|name| config.fixture(name)).transpose()?;
        let nonce = unique_id("nonce");
        let session = SessionId::new(unique_id("session"));
        let run_dir = resolve_under(workspace, &config.artifact_root)?.join(unique_id("run"));
        fs::create_dir_all(&run_dir)?;

        let executable = if profile.executable.is_absolute() {
            profile.executable.clone()
        } else {
            workspace.join(&profile.executable)
        };
        let mut command = Command::new(&executable);
        command
            .current_dir(workspace)
            .args(&profile.arguments)
            .envs(&profile.environment)
            .env("EGUIPILOTAI_STDIO", "1")
            .env("EGUIPILOTAI_SESSION", &session.0)
            .env("EGUIPILOTAI_NONCE", &nonce)
            .env("EGUIPILOTAI_ARTIFACT_ROOT", &run_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(fixture) = fixture {
            command.args(&fixture.arguments).envs(&fixture.environment);
        }
        // Last, so an explicitly requested size wins over anything a fixture
        // asked for.
        if let Some(window) = window {
            command.args(["--window", &window.argument()]);
        }
        let mut child = command.spawn().map_err(|error| {
            RunnerError::Io(format!(
                "failed to launch {}: {error}",
                executable.display()
            ))
        })?;
        let stdin = BufWriter::new(
            child
                .stdin
                .take()
                .ok_or_else(|| RunnerError::Protocol("target stdin was not piped".into()))?,
        );
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| RunnerError::Protocol("target stdout was not piped".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| RunnerError::Protocol("target stderr was not piped".into()))?;

        let (inbound_tx, inbound) = mpsc::channel();
        thread::Builder::new()
            .name("eguipilotai-target-reader".into())
            .spawn(move || {
                let mut reader = BufReader::new(stdout);
                loop {
                    match read_frame(&mut reader) {
                        Ok(Some(message)) => {
                            if inbound_tx
                                .send(TargetMessage::Message(Box::new(message)))
                                .is_err()
                            {
                                break;
                            }
                        }
                        Ok(None) => {
                            let _ = inbound_tx.send(TargetMessage::Eof);
                            break;
                        }
                        Err(error) => {
                            let _ = inbound_tx.send(TargetMessage::Error(error.into()));
                            break;
                        }
                    }
                }
            })?;
        let (stderr_tx, stderr_rx) = mpsc::channel();
        thread::Builder::new()
            .name("eguipilotai-target-stderr".into())
            .spawn(move || {
                let mut reader = BufReader::new(stderr);
                let mut bytes = Vec::new();
                let _ = reader.read_to_end(&mut bytes);
                let _ = stderr_tx.send(bytes);
            })?;
        let trace_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(run_dir.join("trace.jsonl"))?;
        let mut launched = Self {
            child,
            stdin,
            inbound,
            stderr: stderr_rx,
            session,
            next_request: 0,
            next_run: 0,
            outstanding: HashSet::new(),
            responses: HashMap::new(),
            events: VecDeque::new(),
            last_event_sequence: 0,
            timeout: Duration::from_millis(profile.timeout_ms),
            run_dir,
            handshake: None,
            trace: BufWriter::new(trace_file),
        };
        launched.write_manifest(profile_name, fixture_name, &executable, profile, window)?;
        let result = launched
            .request(ClientCommand::Handshake(HandshakeRequest {
                protocol: PROTOCOL_VERSION,
                build: env!("CARGO_PKG_VERSION").into(),
                session_nonce: nonce,
                required_capabilities: vec![
                    protocol::CapabilityId::new(protocol::capability::PROTOCOL_V2),
                    protocol::CapabilityId::new(protocol::capability::INTERLEAVED_EVENTS),
                    protocol::CapabilityId::new(protocol::capability::CANCELLATION),
                ],
                trace_level: protocol::TraceLevel::RetainOnFailure,
            }))
            .map_err(disarmed_target)?;
        let handshake = match result {
            CommandResult::Ready { handshake } => handshake,
            result => return Err(unexpected("ready handshake", result)),
        };
        launched.handshake = Some(handshake);
        let renderer_status = launched.status()?;
        validate_status_adapter(profile.adapter_policy, &renderer_status)?;
        write_new_json(launched.run_dir.join("renderer.json"), &renderer_status)?;
        Ok(launched)
    }

    fn write_manifest(
        &self,
        profile_name: &str,
        fixture_name: Option<&str>,
        executable: &Path,
        profile: &Profile,
        window: Option<WindowSize>,
    ) -> Result<(), RunnerError> {
        #[derive(Serialize)]
        struct Manifest<'a> {
            protocol_version: protocol::ProtocolVersion,
            session: &'a str,
            profile: &'a str,
            fixture: Option<&'a str>,
            executable: String,
            adapter_policy: AdapterPolicy,
            // Layout is size-dependent, so a run is only reproducible if the
            // size it ran at is on the record.
            window: Option<WindowSize>,
        }
        let manifest = Manifest {
            protocol_version: PROTOCOL_VERSION,
            session: &self.session.0,
            profile: profile_name,
            fixture: fixture_name,
            executable: executable.to_string_lossy().into_owned(),
            adapter_policy: profile.adapter_policy,
            window,
        };
        write_new_json(self.run_dir.join("manifest.json"), &manifest)
    }

    pub fn request(&mut self, command: ClientCommand) -> Result<CommandResult, RunnerError> {
        let request_id = self.send(command)?;
        self.receive_for(&request_id)
    }

    pub fn send(&mut self, command: ClientCommand) -> Result<RequestId, RunnerError> {
        self.next_request = self.next_request.saturating_add(1);
        let request_id = RequestId::new(format!("request-{:020}", self.next_request));
        let envelope = CommandEnvelope {
            request_id: request_id.clone(),
            session: self.session.clone(),
            command,
        };
        trace_record(&mut self.trace, "client-command", &envelope)?;
        write_frame(&mut self.stdin, &envelope)?;
        if !self.outstanding.insert(request_id.clone()) {
            return Err(RunnerError::Protocol(format!(
                "duplicate request id `{request_id}`"
            )));
        }
        Ok(request_id)
    }

    pub fn receive_for(&mut self, request_id: &RequestId) -> Result<CommandResult, RunnerError> {
        // Route interleaved events and out-of-order responses until this
        // request's one terminal response arrives.
        let deadline = Instant::now() + self.timeout;
        loop {
            if let Some(outcome) = self.responses.remove(request_id) {
                return match outcome {
                    ResponseOutcome::Success { result } => Ok(*result),
                    ResponseOutcome::Error { error } => Err(RunnerError::Target(Box::new(error))),
                };
            }
            if !self.outstanding.contains(request_id) {
                return Err(RunnerError::Protocol(format!(
                    "request `{request_id}` is not outstanding"
                )));
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(RunnerError::Timeout(format!(
                    "target did not answer request `{request_id}`"
                )));
            }
            match self.inbound.recv_timeout(deadline - now) {
                Ok(TargetMessage::Message(message)) => self.route_message(*message)?,
                Ok(TargetMessage::Error(error)) => return Err(error),
                Ok(TargetMessage::Eof) => {
                    return Err(RunnerError::Protocol("target disconnected".into()))
                }
                Err(RecvTimeoutError::Timeout) => {
                    return Err(RunnerError::Timeout(format!(
                        "target did not answer request `{request_id}`"
                    )))
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(RunnerError::Protocol("target reader disconnected".into()))
                }
            }
        }
    }

    fn route_message(&mut self, message: ServerMessage) -> Result<(), RunnerError> {
        trace_record(&mut self.trace, "server-message", &message)?;
        route_server_message(
            &self.session,
            &mut self.outstanding,
            &mut self.responses,
            &mut self.events,
            &mut self.last_event_sequence,
            message,
        )
    }

    pub fn next_event(&mut self) -> Option<RuntimeEventEnvelope> {
        self.events.pop_front()
    }

    pub fn status(&mut self) -> Result<protocol::SessionStatus, RunnerError> {
        match self.request(ClientCommand::Status)? {
            CommandResult::Status { status } => Ok(status),
            result => Err(unexpected("status", result)),
        }
    }

    pub fn snapshot(
        &mut self,
        request: SnapshotRequest,
    ) -> Result<protocol::AgentSnapshot, RunnerError> {
        match self.request(ClientCommand::Snapshot(request))? {
            CommandResult::Snapshot { snapshot } => Ok(snapshot),
            result => Err(unexpected("snapshot", result)),
        }
    }

    pub fn capture(&mut self, name: String) -> Result<CaptureResult, RunnerError> {
        self.capture_viewport(name, None)
    }

    pub fn capture_viewport(
        &mut self,
        name: String,
        viewport: Option<ViewportSelector>,
    ) -> Result<CaptureResult, RunnerError> {
        match self.request(ClientCommand::Capture { name, viewport })? {
            CommandResult::Capture { capture } => Ok(capture),
            result => Err(unexpected("capture", result)),
        }
    }

    /// Mount, unmount, or clear coach touchpoints on the live target.
    pub fn coach(&mut self, command: CoachCommand) -> Result<Vec<CoachEvent>, RunnerError> {
        match self.request(ClientCommand::Coach { coach: command })? {
            CommandResult::Coach { events } => Ok(events),
            result => Err(unexpected("coach", result)),
        }
    }

    /// Block until this touchpoint is activated or its lifetime ends.
    ///
    /// The wait is deliberately independent of the profile's request timeout:
    /// a coached operator takes as long as a person takes.
    pub fn await_coach(
        &mut self,
        coach_id: &CoachId,
        wait: Duration,
    ) -> Result<CoachEvent, RunnerError> {
        let deadline = Instant::now() + wait;
        loop {
            if let Some(index) = self.events.iter().position(|envelope| {
                // Arming and mounting are progress, not activation: a
                // touchpoint waiting for its target keeps the wait alive.
                matches!(&envelope.event, RuntimeEvent::Coach { event }
                if event.coach_id() == coach_id
                    && !matches!(
                        event.state(),
                        protocol::CoachExpectationState::Armed
                            | protocol::CoachExpectationState::Mounted
                    ))
            }) {
                let envelope = self.events.remove(index).expect("matched coach event");
                let RuntimeEvent::Coach { event } = envelope.event else {
                    unreachable!("matched a coach event");
                };
                return Ok(event);
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(RunnerError::Timeout(format!(
                    "touchpoint `{coach_id}` was not activated"
                )));
            }
            match self.inbound.recv_timeout(deadline - now) {
                Ok(TargetMessage::Message(message)) => self.route_message(*message)?,
                Ok(TargetMessage::Error(error)) => return Err(error),
                Ok(TargetMessage::Eof) => {
                    return Err(RunnerError::Protocol("target disconnected".into()))
                }
                Err(RecvTimeoutError::Timeout) => {
                    return Err(RunnerError::Timeout(format!(
                        "touchpoint `{coach_id}` was not activated"
                    )))
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(RunnerError::Protocol("target reader disconnected".into()))
                }
            }
        }
    }

    pub fn run(&mut self, flow: Flow) -> Result<RunResult, RunnerError> {
        let unsupported = protocol::unsupported_capabilities(&flow);
        if !unsupported.is_empty() {
            return Err(RunnerError::Protocol(format!(
                "flow requires unsupported capabilities: {unsupported:?}"
            )));
        }
        self.next_run = self.next_run.saturating_add(1);
        let run_id = RunId::new(format!("run-{:020}", self.next_run));
        match self.request(ClientCommand::Run { run_id, flow })? {
            CommandResult::Run { result } => Ok(result),
            result => Err(unexpected("run result", result)),
        }
    }

    pub fn cancel(&mut self, run_id: RunId) -> Result<protocol::CancellationResult, RunnerError> {
        match self.request(ClientCommand::Cancel { run_id })? {
            CommandResult::Cancel { cancellation } => Ok(cancellation),
            result => Err(unexpected("cancellation", result)),
        }
    }

    pub fn stop(&mut self) -> Result<(), RunnerError> {
        if self.child.try_wait()?.is_some() {
            return Ok(());
        }
        let _ = self.request(ClientCommand::Stop);
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if self.child.try_wait()?.is_some() {
                self.persist_stderr()?;
                return Ok(());
            }
            thread::sleep(Duration::from_millis(10));
        }
        self.child.kill()?;
        let _ = self.child.wait();
        self.persist_stderr()?;
        Ok(())
    }

    fn persist_stderr(&mut self) -> Result<(), RunnerError> {
        if let Ok(bytes) = self.stderr.recv_timeout(Duration::from_millis(100)) {
            write_new(self.run_dir.join("stderr.log"), &bytes)?;
        }
        self.trace.flush()?;
        Ok(())
    }
}

impl Drop for ProcessSession {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        let _ = self.persist_stderr();
    }
}

fn unexpected(expected: &str, result: CommandResult) -> RunnerError {
    RunnerError::Protocol(format!("expected {expected}, received {result:?}"))
}

/// A target that never answers the handshake is nearly always a target built
/// without the devtools feature, which leaves the process perfectly healthy and
/// completely mute. Saying so here costs one line and saves rediscovering it.
fn disarmed_target(error: RunnerError) -> RunnerError {
    match error {
        RunnerError::Protocol(detail) | RunnerError::Timeout(detail) => {
            RunnerError::Protocol(format!(
                "{detail} — the target answered no handshake, so it is most likely built without \
                 the pilot: rebuild it with `--features eguipilotai-devtools`"
            ))
        }
        other => other,
    }
}

fn route_server_message(
    session: &SessionId,
    outstanding: &mut HashSet<RequestId>,
    responses: &mut HashMap<RequestId, ResponseOutcome>,
    events: &mut VecDeque<RuntimeEventEnvelope>,
    last_event_sequence: &mut u64,
    message: ServerMessage,
) -> Result<(), RunnerError> {
    match message {
        ServerMessage::Response { envelope } => {
            if &envelope.session != session {
                return Err(RunnerError::Protocol("response session mismatch".into()));
            }
            if !outstanding.remove(&envelope.request_id) {
                return Err(RunnerError::Protocol(format!(
                    "response for unknown or completed request `{}`",
                    envelope.request_id
                )));
            }
            if responses
                .insert(envelope.request_id.clone(), envelope.outcome)
                .is_some()
            {
                return Err(RunnerError::Protocol(format!(
                    "duplicate terminal response for `{}`",
                    envelope.request_id
                )));
            }
        }
        ServerMessage::Event { envelope } => {
            if &envelope.session != session {
                return Err(RunnerError::Protocol("event session mismatch".into()));
            }
            let sequence = envelope.event_id.sequence().ok_or_else(|| {
                RunnerError::Protocol(format!("invalid event id `{}`", envelope.event_id))
            })?;
            let expected = last_event_sequence.saturating_add(1);
            if sequence != expected {
                return Err(RunnerError::Protocol(format!(
                    "event sequence gap: expected {expected}, received {sequence}"
                )));
            }
            *last_event_sequence = sequence;
            events.push_back(*envelope);
        }
    }
    Ok(())
}

fn trace_record<T: Serialize>(
    writer: &mut impl Write,
    direction: &str,
    value: &T,
) -> Result<(), RunnerError> {
    #[derive(Serialize)]
    struct Record<'a, T> {
        direction: &'a str,
        payload: &'a T,
    }
    serde_json::to_writer(
        &mut *writer,
        &Record {
            direction,
            payload: value,
        },
    )
    .map_err(|error| RunnerError::Protocol(error.to_string()))?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn unique_id(prefix: &str) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{}-{stamp:x}", std::process::id())
}

fn validate_relative_root(path: &Path) -> Result<(), RunnerError> {
    // A Windows drive prefix ("C:\") or UNC ("\\server\") is absolute on
    // Windows but parses as a plain component on Unix — so `is_absolute()`
    // and `Component::Prefix` miss it off-Windows. Reject it on EVERY
    // platform: this is a cross-platform tool, and a root authored for one OS
    // must not escape the jail when validated on another.
    let text = path.to_string_lossy();
    let bytes = text.as_bytes();
    let windows_absolute = text.starts_with("\\\\")
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':');
    if windows_absolute
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(RunnerError::config(format!(
            "path `{}` must stay relative to the workspace",
            path.display()
        )));
    }
    Ok(())
}

fn resolve_under(workspace: &Path, relative: &Path) -> Result<PathBuf, RunnerError> {
    validate_relative_root(relative)?;
    Ok(workspace.join(relative))
}

fn write_new_json(path: PathBuf, value: &impl Serialize) -> Result<(), RunnerError> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| RunnerError::Protocol(error.to_string()))?;
    write_new(path, &bytes)
}

fn write_new(path: PathBuf, bytes: &[u8]) -> Result<(), RunnerError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    Ok(())
}

pub fn load_flow(path: impl AsRef<Path>) -> Result<Flow, RunnerError> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)?;
    toml::from_str(&text)
        .map_err(|error| RunnerError::Config(format!("{}: {error}", path.display())))
}

pub fn save_flow(path: impl AsRef<Path>, flow: &Flow) -> Result<(), RunnerError> {
    let text =
        toml::to_string_pretty(flow).map_err(|error| RunnerError::Protocol(error.to_string()))?;
    write_new(path.as_ref().to_path_buf(), text.as_bytes())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisualTolerance {
    pub max_channel_delta: u8,
    pub max_changed_fraction: f32,
}

impl VisualTolerance {
    pub const EXACT: Self = Self {
        max_channel_delta: 0,
        max_changed_fraction: 0.0,
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualDiff {
    pub width: u32,
    pub height: u32,
    pub changed_pixels: u64,
    pub total_pixels: u64,
    pub maximum_channel_delta: u8,
    pub passed: bool,
}

pub fn compare_images(
    expected: impl AsRef<Path>,
    actual: impl AsRef<Path>,
    tolerance: VisualTolerance,
) -> Result<VisualDiff, RunnerError> {
    let expected = image::open(expected.as_ref())
        .map_err(|error| RunnerError::Visual(error.to_string()))?
        .into_rgba8();
    let actual = image::open(actual.as_ref())
        .map_err(|error| RunnerError::Visual(error.to_string()))?
        .into_rgba8();
    if expected.dimensions() != actual.dimensions() {
        return Err(RunnerError::Visual(format!(
            "dimensions differ: expected {:?}, actual {:?}",
            expected.dimensions(),
            actual.dimensions()
        )));
    }
    let mut changed = 0u64;
    let mut maximum = 0u8;
    for (left, right) in expected.pixels().zip(actual.pixels()) {
        let delta = left
            .0
            .iter()
            .zip(right.0.iter())
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap_or(0);
        maximum = maximum.max(delta);
        changed += u64::from(delta > tolerance.max_channel_delta);
    }
    let total = u64::from(expected.width()) * u64::from(expected.height());
    let fraction = if total == 0 {
        0.0
    } else {
        changed as f32 / total as f32
    };
    Ok(VisualDiff {
        width: expected.width(),
        height: expected.height(),
        changed_pixels: changed,
        total_pixels: total,
        maximum_channel_delta: maximum,
        passed: fraction <= tolerance.max_changed_fraction,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineApproval {
    pub scenario: String,
    pub renderer_key: String,
    pub content_hash: String,
    pub file: String,
}

/// Add an immutable content-addressed baseline and append its approval record.
/// Existing baseline bytes and manifests are never overwritten or deleted.
pub fn approve_baseline(
    workspace: impl AsRef<Path>,
    scenario: &str,
    renderer: &RendererIdentity,
    candidate: impl AsRef<Path>,
) -> Result<BaselineApproval, RunnerError> {
    let workspace = workspace.as_ref();
    let bytes = fs::read(candidate.as_ref())?;
    let hash = protocol::stable_hash(&bytes);
    let key = renderer.baseline_key();
    let directory = workspace
        .join("tests/ui/baselines")
        .join(sanitize_component(scenario))
        .join(&key);
    fs::create_dir_all(&directory)?;
    let image_path = directory.join(format!("{hash}.png"));
    if !image_path.exists() {
        write_new(image_path.clone(), &bytes)?;
    }
    let approval = BaselineApproval {
        scenario: scenario.into(),
        renderer_key: key,
        content_hash: hash,
        file: image_path
            .strip_prefix(workspace)
            .unwrap_or(&image_path)
            .to_string_lossy()
            .into_owned(),
    };
    let manifest = directory.join(format!("approval-{}.json", approval.content_hash));
    if !manifest.exists() {
        write_new_json(manifest, &approval)?;
    }
    Ok(approval)
}

fn sanitize_component(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect();
    cleaned.trim_matches('-').to_string()
}

pub fn validate_adapter_policy(
    policy: AdapterPolicy,
    renderer: &RendererIdentity,
) -> Result<(), RunnerError> {
    let software = renderer.device_type.eq_ignore_ascii_case("cpu");
    match policy {
        AdapterPolicy::SoftwareRequired if !software => Err(RunnerError::Visual(format!(
            "software-required selected non-CPU adapter `{}`",
            renderer.adapter
        ))),
        AdapterPolicy::HardwareRequired if software => Err(RunnerError::Visual(format!(
            "hardware-required selected CPU adapter `{}`",
            renderer.adapter
        ))),
        _ => Ok(()),
    }
}

fn validate_status_adapter(
    policy: AdapterPolicy,
    status: &protocol::SessionStatus,
) -> Result<(), RunnerError> {
    let required = matches!(
        policy,
        AdapterPolicy::SoftwareRequired | AdapterPolicy::HardwareRequired
    );
    let Some(device_type) = status.renderer_device_type.as_deref() else {
        return if required {
            Err(RunnerError::Visual(
                "required adapter policy did not report a renderer identity".into(),
            ))
        } else {
            Ok(())
        };
    };
    let software = device_type.eq_ignore_ascii_case("cpu");
    match policy {
        AdapterPolicy::SoftwareRequired if !software => Err(RunnerError::Visual(format!(
            "software-required target reported `{device_type}`"
        ))),
        AdapterPolicy::HardwareRequired if software => Err(RunnerError::Visual(format!(
            "hardware-required target reported `{device_type}`"
        ))),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(device_type: &str) -> RendererIdentity {
        RendererIdentity {
            os: "linux".into(),
            backend: "vulkan".into(),
            adapter: "test".into(),
            device_type: device_type.into(),
            driver: "mesa".into(),
            renderer_version: "1".into(),
            surface_format: "rgba8".into(),
            pixels_per_point: 1.0,
            font_bundle: "pinned".into(),
            dithering: false,
        }
    }

    #[test]
    fn required_adapter_policies_never_cross_families() {
        assert!(validate_adapter_policy(AdapterPolicy::SoftwareRequired, &identity("cpu")).is_ok());
        assert!(
            validate_adapter_policy(AdapterPolicy::SoftwareRequired, &identity("gpu")).is_err()
        );
        assert!(validate_adapter_policy(AdapterPolicy::HardwareRequired, &identity("gpu")).is_ok());
        assert!(
            validate_adapter_policy(AdapterPolicy::HardwareRequired, &identity("cpu")).is_err()
        );
    }

    #[test]
    fn artifact_roots_cannot_escape_workspace() {
        assert!(validate_relative_root(Path::new("target/eguipilotai")).is_ok());
        assert!(validate_relative_root(Path::new("../outside")).is_err());
        assert!(validate_relative_root(Path::new("C:\\outside")).is_err());
    }

    #[test]
    fn renderer_keys_separate_renderer_families() {
        assert_ne!(
            identity("cpu").baseline_key(),
            identity("gpu").baseline_key()
        );
    }

    fn response(request_id: &str) -> ServerMessage {
        ServerMessage::response(protocol::ResponseEnvelope {
            request_id: RequestId::new(request_id),
            session: SessionId::new("session"),
            outcome: ResponseOutcome::Success {
                result: Box::new(CommandResult::Stopped),
            },
        })
    }

    fn event(sequence: u64) -> ServerMessage {
        ServerMessage::event(protocol::RuntimeEventEnvelope {
            event_id: protocol::EventId::from_sequence(sequence),
            session: SessionId::new("session"),
            scope: protocol::EventScope::default(),
            event: protocol::RuntimeEvent::InputCleanup,
        })
    }

    #[test]
    fn router_accepts_interleaved_events_and_out_of_order_responses() {
        let session = SessionId::new("session");
        let first = RequestId::new("first");
        let second = RequestId::new("second");
        let mut outstanding = HashSet::from([first.clone(), second.clone()]);
        let mut responses = HashMap::new();
        let mut events = VecDeque::new();
        let mut last_event = 0;

        route_server_message(
            &session,
            &mut outstanding,
            &mut responses,
            &mut events,
            &mut last_event,
            event(1),
        )
        .unwrap();
        route_server_message(
            &session,
            &mut outstanding,
            &mut responses,
            &mut events,
            &mut last_event,
            response("second"),
        )
        .unwrap();
        route_server_message(
            &session,
            &mut outstanding,
            &mut responses,
            &mut events,
            &mut last_event,
            response("first"),
        )
        .unwrap();

        assert_eq!(events.len(), 1);
        assert!(responses.contains_key(&first));
        assert!(responses.contains_key(&second));
        assert!(outstanding.is_empty());
    }

    #[test]
    fn router_rejects_event_gaps_and_duplicate_terminal_responses() {
        let session = SessionId::new("session");
        let request = RequestId::new("request");
        let mut outstanding = HashSet::from([request]);
        let mut responses = HashMap::new();
        let mut events = VecDeque::new();
        let mut last_event = 0;

        assert!(route_server_message(
            &session,
            &mut outstanding,
            &mut responses,
            &mut events,
            &mut last_event,
            event(2),
        )
        .is_err());
        route_server_message(
            &session,
            &mut outstanding,
            &mut responses,
            &mut events,
            &mut last_event,
            response("request"),
        )
        .unwrap();
        assert!(route_server_message(
            &session,
            &mut outstanding,
            &mut responses,
            &mut events,
            &mut last_event,
            response("request"),
        )
        .is_err());
    }

    #[test]
    fn a_window_size_is_bounded_and_rebuilt_from_its_integers() {
        let size: WindowSize = "1280x832".parse().expect("a plain size parses");
        assert_eq!(
            size,
            WindowSize {
                width: 1280,
                height: 832
            }
        );
        assert_eq!(size.argument(), "1280x832");
        assert!("1280X832".parse::<WindowSize>().is_ok());

        // The command line is rebuilt from parsed integers, so no caller text
        // can ride along as a second flag, and out-of-range sizes refuse.
        for refused in [
            "1280",
            "1280x",
            "x832",
            "1280x832 --route home",
            "-1x600",
            "0x0",
            "99999x99999",
        ] {
            assert!(
                refused.parse::<WindowSize>().is_err(),
                "`{refused}` must be refused"
            );
        }
    }

    #[test]
    fn unsupported_scheduler_settings_fail_configuration_validation() {
        let profile = Profile {
            executable: PathBuf::from("target/app"),
            platforms: Vec::new(),
            arguments: Vec::new(),
            environment: BTreeMap::new(),
            adapter_policy: AdapterPolicy::HardwareThenSoftware,
            timeout_ms: 1,
        };
        let mut config = EguiPilotConfig::default();
        config.profiles.insert("app".into(), profile);
        config.projects.push(Project {
            id: protocol::ProjectId::new("project"),
            profile: "app".into(),
            adapter_policy: AdapterPolicy::HardwareThenSoftware,
            required: true,
            workers: 2,
            retries: 0,
        });
        assert!(config.validate(Path::new(".")).is_err());
        config.projects[0].workers = 1;
        config.projects[0].retries = 1;
        assert!(config.validate(Path::new(".")).is_err());
    }

    #[test]
    fn viziwiz_multi_viewport_scenario_is_durable_toml() {
        let flow: Flow = toml::from_str(include_str!(
            "../../../tests/eguipilotai/viziwiz-multi-viewport.toml"
        ))
        .expect("ViziWiz multi-viewport scenario");
        assert_eq!(flow.id, protocol::ScenarioId::new("viziwiz-multi-viewport"));
        assert!(flow.steps.iter().any(|step| matches!(
            step,
            protocol::Step::Action {
                action: protocol::Action::FocusViewport { .. }
            }
        )));
        assert!(flow.steps.iter().any(|step| matches!(
            step,
            protocol::Step::Action {
                action: protocol::Action::CloseViewport { .. }
            }
        )));
        assert!(protocol::unsupported_capabilities(&flow).is_empty());
    }

    #[test]
    fn coach_touchpoint_scenario_is_durable_toml() {
        let flow: Flow = toml::from_str(include_str!(
            "../../../tests/eguipilotai/coach-touchpoint.toml"
        ))
        .expect("coach touchpoint scenario");
        assert_eq!(flow.id, protocol::ScenarioId::new("coach-touchpoint"));
        let mounts = flow
            .steps
            .iter()
            .filter(|step| {
                matches!(
                    step,
                    protocol::Step::Coach {
                        coach: protocol::CoachCommand::Mount { .. }
                    }
                )
            })
            .count();
        assert_eq!(mounts, 1);
        assert!(flow.steps.iter().any(|step| matches!(
            step,
            protocol::Step::Expect {
                expectation: protocol::Expectation::Coach {
                    state: protocol::CoachExpectationState::Triggered,
                    ..
                }
            }
        )));
        assert!(protocol::unsupported_capabilities(&flow).is_empty());
    }

    #[test]
    fn coach_dynamic_trigger_scenario_is_durable_toml() {
        let flow: Flow = toml::from_str(include_str!(
            "../../../tests/eguipilotai/coach-dynamic-triggers.toml"
        ))
        .expect("coach dynamic trigger scenario");
        assert_eq!(flow.id, protocol::ScenarioId::new("coach-dynamic-triggers"));
        let triggers: Vec<protocol::TriggerKind> = flow
            .steps
            .iter()
            .filter_map(|step| match step {
                protocol::Step::Coach {
                    coach: protocol::CoachCommand::Mount { touchpoint },
                } => Some(touchpoint.on.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            triggers,
            vec![
                protocol::TriggerKind::NodeAppeared,
                protocol::TriggerKind::FabricEvent {
                    signal: protocol::SignalId::new("shell.route-changed/workbench/episodes"),
                },
            ]
        );
        assert!(flow.steps.iter().any(|step| matches!(
            step,
            protocol::Step::Expect {
                expectation: protocol::Expectation::Coach {
                    state: protocol::CoachExpectationState::Armed,
                    ..
                }
            }
        )));
        assert!(protocol::unsupported_capabilities(&flow).is_empty());
    }

    #[test]
    fn chat_peer_window_scenario_is_durable_toml() {
        let flow: Flow = toml::from_str(include_str!(
            "../../../tests/eguipilotai/chat-peer-window.toml"
        ))
        .expect("Chat peer-window scenario");
        assert_eq!(flow.id, protocol::ScenarioId::new("chat-peer-window"));
        assert_eq!(flow.fixture, Some(protocol::FixtureId::new("chat-peer")));
        assert!(flow.steps.iter().any(|step| matches!(
            step,
            protocol::Step::Action {
                action: protocol::Action::FocusViewport { .. }
            }
        )));
        assert!(flow.steps.iter().any(|step| matches!(
            step,
            protocol::Step::Action {
                action: protocol::Action::MinimizeViewport { .. }
            }
        )));
        assert!(flow.steps.iter().any(|step| matches!(
            step,
            protocol::Step::Action {
                action: protocol::Action::RestoreViewport { .. }
            }
        )));
        assert!(flow.steps.iter().any(|step| matches!(
            step,
            protocol::Step::Action {
                action: protocol::Action::CloseViewport { .. }
            }
        )));
        assert!(protocol::unsupported_capabilities(&flow).is_empty());
    }
}
