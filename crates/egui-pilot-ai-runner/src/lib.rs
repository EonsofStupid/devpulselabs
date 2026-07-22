//! Process-isolated scenario, artifact, trace, and visual runner.
//!
//! Only profiles and fixtures declared in `.eguipilotai.toml` may be launched.
//! Callers never pass executable paths, environment variables, shell strings,
//! or artifact destinations through the control protocol.

use egui_pilot_ai_protocol as protocol;
use egui_pilot_ai_transport::{read_frame, write_frame, TransportError};
use protocol::{
    AdapterPolicy, CaptureResult, ClientCommand, ClientEnvelope, Flow, Handshake, PilotError,
    Project, RendererIdentity, ServerEnvelope, ServerEvent, SessionId, SnapshotRequest,
    StepOutcome, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    Envelope(ServerEnvelope),
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
    sequence: u64,
    timeout: Duration,
    pub run_dir: PathBuf,
    trace: BufWriter<fs::File>,
}

impl ProcessSession {
    pub fn launch(
        workspace: impl AsRef<Path>,
        config: &EguiPilotConfig,
        profile_name: &str,
        fixture_name: Option<&str>,
    ) -> Result<Self, RunnerError> {
        let workspace = workspace.as_ref();
        let profile = config.profile(profile_name)?;
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
                        Ok(Some(envelope)) => {
                            if inbound_tx.send(TargetMessage::Envelope(envelope)).is_err() {
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
            sequence: 0,
            timeout: Duration::from_millis(profile.timeout_ms),
            run_dir,
            trace: BufWriter::new(trace_file),
        };
        launched.write_manifest(profile_name, fixture_name, &executable, profile)?;
        let event = launched.request(ClientCommand::Handshake(Handshake {
            protocol_version: PROTOCOL_VERSION,
            build: env!("CARGO_PKG_VERSION").into(),
            session_nonce: nonce,
        }))?;
        if !matches!(event, ServerEvent::Ready(_)) {
            return Err(RunnerError::Protocol(format!(
                "target did not acknowledge handshake: {event:?}"
            )));
        }
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
    ) -> Result<(), RunnerError> {
        #[derive(Serialize)]
        struct Manifest<'a> {
            protocol_version: u32,
            session: &'a str,
            profile: &'a str,
            fixture: Option<&'a str>,
            executable: String,
            adapter_policy: AdapterPolicy,
        }
        let manifest = Manifest {
            protocol_version: PROTOCOL_VERSION,
            session: &self.session.0,
            profile: profile_name,
            fixture: fixture_name,
            executable: executable.to_string_lossy().into_owned(),
            adapter_policy: profile.adapter_policy,
        };
        write_new_json(self.run_dir.join("manifest.json"), &manifest)
    }

    pub fn request(&mut self, command: ClientCommand) -> Result<ServerEvent, RunnerError> {
        let sequence = self.send(command)?;
        self.receive_for(sequence)
    }

    pub fn send(&mut self, command: ClientCommand) -> Result<u64, RunnerError> {
        self.sequence = self.sequence.saturating_add(1);
        let envelope = ClientEnvelope {
            sequence: self.sequence,
            session: self.session.clone(),
            command,
        };
        trace_record(&mut self.trace, "client", &envelope)?;
        write_frame(&mut self.stdin, &envelope)?;
        Ok(self.sequence)
    }

    pub fn receive_for(&mut self, sequence: u64) -> Result<ServerEvent, RunnerError> {
        // Strict lockstep: exactly ONE message answers a sequence — a stray,
        // stale, or mismatched envelope is a protocol error, never skipped.
        // (No receive loop by design; clippy::never_loop agrees.)
        if self.timeout.is_zero() {
            return Err(RunnerError::Timeout(format!(
                "target did not answer sequence {sequence}"
            )));
        }
        match self.inbound.recv_timeout(self.timeout) {
            Ok(TargetMessage::Envelope(envelope)) => {
                trace_record(&mut self.trace, "server", &envelope)?;
                if envelope.session != self.session {
                    return Err(RunnerError::Protocol("response session mismatch".into()));
                }
                if envelope.sequence != sequence {
                    return Err(RunnerError::Protocol(format!(
                        "expected sequence {sequence}, received {}",
                        envelope.sequence
                    )));
                }
                if let ServerEvent::Error(error) = envelope.event {
                    return Err(RunnerError::Target(Box::new(error)));
                }
                Ok(envelope.event)
            }
            Ok(TargetMessage::Error(error)) => Err(error),
            Ok(TargetMessage::Eof) => Err(RunnerError::Protocol("target disconnected".into())),
            Err(RecvTimeoutError::Timeout) => Err(RunnerError::Timeout(format!(
                "target did not answer sequence {sequence}"
            ))),
            Err(RecvTimeoutError::Disconnected) => {
                Err(RunnerError::Protocol("target reader disconnected".into()))
            }
        }
    }

    pub fn status(&mut self) -> Result<protocol::SessionStatus, RunnerError> {
        match self.request(ClientCommand::Status)? {
            ServerEvent::Status(status) => Ok(status),
            event => Err(unexpected("status", event)),
        }
    }

    pub fn snapshot(
        &mut self,
        request: SnapshotRequest,
    ) -> Result<protocol::AgentSnapshot, RunnerError> {
        match self.request(ClientCommand::Snapshot(request))? {
            ServerEvent::Snapshot(snapshot) => Ok(snapshot),
            event => Err(unexpected("snapshot", event)),
        }
    }

    pub fn capture(&mut self, name: String) -> Result<CaptureResult, RunnerError> {
        match self.request(ClientCommand::Capture { name })? {
            ServerEvent::Capture(capture) => Ok(capture),
            event => Err(unexpected("capture", event)),
        }
    }

    pub fn run(&mut self, flow: Flow) -> Result<Vec<StepOutcome>, RunnerError> {
        let step_count = flow.steps.len();
        let sequence = self.send(ClientCommand::Run(flow))?;
        let mut outcomes = Vec::with_capacity(step_count);
        while outcomes.len() < step_count {
            match self.receive_for(sequence)? {
                ServerEvent::Step(outcome) => {
                    if !outcome.passed {
                        return Err(RunnerError::Protocol(
                            outcome.message.unwrap_or_else(|| "step failed".into()),
                        ));
                    }
                    outcomes.push(outcome);
                }
                ServerEvent::Capture(_) => {}
                event => return Err(unexpected("step outcome", event)),
            }
        }
        Ok(outcomes)
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

fn unexpected(expected: &str, event: ServerEvent) -> RunnerError {
    RunnerError::Protocol(format!("expected {expected}, received {event:?}"))
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
    if path.is_absolute()
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
}
