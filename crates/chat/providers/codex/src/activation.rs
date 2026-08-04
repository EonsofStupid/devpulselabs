use dpl_ai_core::{ActivationState, RecoveryAction, RecoveryActionKind};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};

pub const SETUP_URL: &str = "https://developers.openai.com/codex/";

const MINIMUM_VERSION: CodexVersion = CodexVersion::new(0, 145, 0);
const EXCLUSIVE_MAXIMUM_VERSION: CodexVersion = CodexVersion::new(0, 146, 0);
const MAX_STATUS_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CodexVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl CodexVersion {
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn parse(output: &str) -> Option<Self> {
        output
            .split(|character: char| !(character.is_ascii_digit() || character == '.'))
            .filter(|candidate| !candidate.is_empty())
            .find_map(|candidate| {
                let mut parts = candidate.split('.');
                let major = parts.next()?.parse().ok()?;
                let minor = parts.next()?.parse().ok()?;
                let patch = parts.next()?.parse().ok()?;
                (parts.next().is_none()).then_some(Self::new(major, minor, patch))
            })
    }

    pub const fn is_activation_supported(self) -> bool {
        version_at_least(self, MINIMUM_VERSION) && version_before(self, EXCLUSIVE_MAXIMUM_VERSION)
    }
}

const fn version_at_least(value: CodexVersion, minimum: CodexVersion) -> bool {
    value.major > minimum.major
        || (value.major == minimum.major
            && (value.minor > minimum.minor
                || (value.minor == minimum.minor && value.patch >= minimum.patch)))
}

const fn version_before(value: CodexVersion, maximum: CodexVersion) -> bool {
    value.major < maximum.major
        || (value.major == maximum.major
            && (value.minor < maximum.minor
                || (value.minor == maximum.minor && value.patch < maximum.patch)))
}

impl fmt::Display for CodexVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexRuntimeProfile {
    executable: PathBuf,
    version: CodexVersion,
    working_directory: PathBuf,
}

impl CodexRuntimeProfile {
    pub fn discover(working_directory: impl AsRef<Path>) -> Result<Self, CodexDiscoveryError> {
        let working_directory = canonical_directory(working_directory.as_ref())?;
        let executable = discover_executable()?;
        let version = probe_version(&executable, &working_directory)?;
        if !version.is_activation_supported() {
            return Err(CodexDiscoveryError::VersionUnsupported(version));
        }
        Ok(Self {
            executable,
            version,
            working_directory,
        })
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub const fn version(&self) -> CodexVersion {
        self.version
    }

    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexDiscoveryError {
    NotInstalled,
    EmbeddedDependency(String),
    WorkingDirectory(String),
    VersionProbe(String),
    VersionUnsupported(CodexVersion),
}

impl fmt::Display for CodexDiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInstalled => f.write_str("the official Codex client is not installed"),
            Self::EmbeddedDependency(reason) => {
                write!(f, "the private Codex payload is unavailable: {reason}")
            }
            Self::WorkingDirectory(reason) => {
                write!(f, "the Codex working directory is invalid: {reason}")
            }
            Self::VersionProbe(reason) => {
                write!(f, "the Codex version could not be verified: {reason}")
            }
            Self::VersionUnsupported(version) => write!(
                f,
                "Codex {version} is outside the activation-qualified range >= {MINIMUM_VERSION}, < {EXCLUSIVE_MAXIMUM_VERSION}"
            ),
        }
    }
}

impl std::error::Error for CodexDiscoveryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexActivationError {
    Unavailable(String),
    AlreadyRunning,
    LaunchFailed(String),
    PollFailed(String),
    StatusFailed(String),
}

impl fmt::Display for CodexActivationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(reason) => f.write_str(reason),
            Self::AlreadyRunning => f.write_str("ChatGPT login is already running"),
            Self::LaunchFailed(reason) => write!(f, "ChatGPT login could not start: {reason}"),
            Self::PollFailed(reason) => write!(f, "ChatGPT login status failed: {reason}"),
            Self::StatusFailed(reason) => {
                write!(f, "Codex could not verify ChatGPT authentication: {reason}")
            }
        }
    }
}

impl std::error::Error for CodexActivationError {}

/// Non-secret classification of `codex login status` output. Only ChatGPT
/// subscription authentication qualifies as `Ready`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexLoginStatus {
    ChatGpt,
    ApiKey,
    SignedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexLoginExit {
    Success,
    Failed(Option<i32>),
}

pub trait CodexLoginProcess: fmt::Debug + Send {
    fn try_exit(&mut self) -> io::Result<Option<CodexLoginExit>>;
    fn terminate_and_reap(&mut self);
}

pub trait CodexLoginLauncher: fmt::Debug + Send {
    fn launch_login(&self, profile: &CodexRuntimeProfile)
        -> io::Result<Box<dyn CodexLoginProcess>>;
}

#[derive(Debug, Default)]
struct SystemCodexLoginLauncher;

impl CodexLoginLauncher for SystemCodexLoginLauncher {
    fn launch_login(
        &self,
        profile: &CodexRuntimeProfile,
    ) -> io::Result<Box<dyn CodexLoginProcess>> {
        let child = login_command(profile).spawn()?;
        Ok(Box::new(OwnedChild { child: Some(child) }))
    }
}

#[derive(Debug)]
struct OwnedChild {
    child: Option<Child>,
}

impl CodexLoginProcess for OwnedChild {
    fn try_exit(&mut self) -> io::Result<Option<CodexLoginExit>> {
        let Some(child) = self.child.as_mut() else {
            return Ok(Some(CodexLoginExit::Failed(None)));
        };
        child.try_wait().map(|status| status.map(exit_from_status))
    }

    fn terminate_and_reap(&mut self) {
        if let Some(mut child) = self.child.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        self.terminate_and_reap();
    }
}

fn exit_from_status(status: ExitStatus) -> CodexLoginExit {
    if status.success() {
        CodexLoginExit::Success
    } else {
        CodexLoginExit::Failed(status.code())
    }
}

pub trait CodexStatusRequest: fmt::Debug + Send {
    /// Nonblocking. `None` while the probe is still running.
    fn try_status(&mut self) -> Option<Result<CodexLoginStatus, String>>;
}

pub trait CodexStatusProbe: fmt::Debug + Send {
    fn begin(&self, profile: &CodexRuntimeProfile) -> io::Result<Box<dyn CodexStatusRequest>>;
}

/// Runs the official non-secret `codex login status` verification on a
/// bounded worker so the UI thread never waits on a provider process.
#[derive(Debug, Default)]
struct SystemCodexStatusProbe;

impl CodexStatusProbe for SystemCodexStatusProbe {
    fn begin(&self, profile: &CodexRuntimeProfile) -> io::Result<Box<dyn CodexStatusRequest>> {
        let profile = profile.clone();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("devpulse-codex-login-status".to_owned())
            .spawn(move || {
                let result = probe_login_status(&profile).map_err(|error| error.to_string());
                let _ = sender.send(result);
            })?;
        Ok(Box::new(PendingStatus { receiver }))
    }
}

#[derive(Debug)]
struct PendingStatus {
    receiver: std::sync::mpsc::Receiver<Result<CodexLoginStatus, String>>,
}

impl CodexStatusRequest for PendingStatus {
    fn try_status(&mut self) -> Option<Result<CodexLoginStatus, String>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(Err("the login status worker stopped".to_owned()))
            }
        }
    }
}

#[derive(Debug)]
pub struct CodexActivationCoordinator {
    discovery_root: Option<PathBuf>,
    profile: Result<CodexRuntimeProfile, CodexDiscoveryError>,
    state: ActivationState,
    launcher: Box<dyn CodexLoginLauncher>,
    status_probe: Box<dyn CodexStatusProbe>,
    child: Option<Box<dyn CodexLoginProcess>>,
    verification: Option<Box<dyn CodexStatusRequest>>,
    login_completed: bool,
    verified_status: Option<CodexLoginStatus>,
}

impl CodexActivationCoordinator {
    pub fn discover(working_directory: impl AsRef<Path>) -> Self {
        let discovery_root = working_directory.as_ref().to_path_buf();
        let profile = CodexRuntimeProfile::discover(&discovery_root);
        let state = activation_from_discovery(&profile);
        Self {
            discovery_root: Some(discovery_root),
            profile,
            state,
            launcher: Box::<SystemCodexLoginLauncher>::default(),
            status_probe: Box::<SystemCodexStatusProbe>::default(),
            child: None,
            verification: None,
            login_completed: false,
            verified_status: None,
        }
    }

    pub fn from_parts(
        profile: Result<CodexRuntimeProfile, CodexDiscoveryError>,
        state: ActivationState,
        launcher: Box<dyn CodexLoginLauncher>,
        status_probe: Box<dyn CodexStatusProbe>,
    ) -> Self {
        Self {
            discovery_root: None,
            profile,
            state,
            launcher,
            status_probe,
            child: None,
            verification: None,
            login_completed: false,
            verified_status: None,
        }
    }

    pub fn activation(&self) -> &ActivationState {
        &self.state
    }

    pub fn activation_capability_qualified(&self) -> bool {
        self.profile.is_ok()
    }

    pub fn runtime_profile(&self) -> Option<CodexRuntimeProfile> {
        self.profile.as_ref().ok().cloned()
    }

    pub fn is_login_running(&self) -> bool {
        self.child.is_some() || self.verification.is_some()
    }

    /// One-shot edge: the official login child exited successfully.
    pub fn take_login_completed(&mut self) -> bool {
        std::mem::take(&mut self.login_completed)
    }

    /// One-shot edge: post-login `codex login status` verification concluded.
    pub fn take_verified_status(&mut self) -> Option<CodexLoginStatus> {
        self.verified_status.take()
    }

    pub fn refresh_discovery(&mut self) -> Result<bool, CodexActivationError> {
        if self.child.is_some() || self.verification.is_some() {
            return Err(CodexActivationError::AlreadyRunning);
        }
        let Some(discovery_root) = self.discovery_root.clone() else {
            return Err(CodexActivationError::Unavailable(
                "the Codex coordinator has no discovery root".to_owned(),
            ));
        };
        let profile = CodexRuntimeProfile::discover(discovery_root);
        let state = activation_from_discovery(&profile);
        let changed = state != self.state;
        self.profile = profile;
        self.state = state;
        Ok(changed)
    }

    pub fn start_login(&mut self) -> Result<(), CodexActivationError> {
        if self.child.is_some() || self.verification.is_some() {
            return Err(CodexActivationError::AlreadyRunning);
        }
        let profile = self
            .profile
            .as_ref()
            .map_err(|error| CodexActivationError::Unavailable(error.to_string()))?;
        let child = self
            .launcher
            .launch_login(profile)
            .map_err(|error| CodexActivationError::LaunchFailed(public_io_reason(&error)))?;
        self.child = Some(child);
        self.login_completed = false;
        self.verified_status = None;
        self.state = ActivationState::SigningIn;
        Ok(())
    }

    pub fn poll(&mut self) -> Result<bool, CodexActivationError> {
        if let Some(child) = self.child.as_mut() {
            let exit = child
                .try_exit()
                .map_err(|error| CodexActivationError::PollFailed(public_io_reason(&error)))?;
            let Some(exit) = exit else {
                return Ok(false);
            };
            self.child = None;
            match exit {
                CodexLoginExit::Success => {
                    self.login_completed = true;
                    let profile = self
                        .profile
                        .as_ref()
                        .map_err(|error| CodexActivationError::StatusFailed(error.to_string()))?;
                    let request = self.status_probe.begin(profile).map_err(|error| {
                        CodexActivationError::StatusFailed(public_io_reason(&error))
                    })?;
                    self.verification = Some(request);
                    self.state = ActivationState::Verifying;
                }
                CodexLoginExit::Failed(code) => self.state = failed_state(code),
            }
            return Ok(true);
        }
        if let Some(request) = self.verification.as_mut() {
            let Some(result) = request.try_status() else {
                return Ok(false);
            };
            self.verification = None;
            let status = result.map_err(CodexActivationError::StatusFailed)?;
            self.verified_status = Some(status);
            self.state = activation_from_status(status);
            return Ok(true);
        }
        Ok(false)
    }

    pub fn fail(&mut self, error: CodexActivationError) {
        if let Some(mut child) = self.child.take() {
            child.terminate_and_reap();
        }
        self.verification = None;
        self.state = ActivationState::Failed {
            reason: error.to_string(),
            recovery: retry_recovery(),
        };
    }
}

impl Drop for CodexActivationCoordinator {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            child.terminate_and_reap();
        }
    }
}

fn activation_from_discovery(
    profile: &Result<CodexRuntimeProfile, CodexDiscoveryError>,
) -> ActivationState {
    match profile {
        Ok(profile) => match probe_login_status(profile) {
            Ok(status) => activation_from_status(status),
            Err(error) => ActivationState::Blocked {
                reason: error.to_string(),
                recovery: vec![RecoveryAction::new(
                    RecoveryActionKind::OpenDiagnostics,
                    "Open Codex diagnostics",
                )],
            },
        },
        Err(CodexDiscoveryError::NotInstalled) => ActivationState::NotInstalled {
            reason: "The official Codex client is required for ChatGPT subscription login."
                .to_owned(),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::InstallInstructions,
                "Get Codex",
            )],
        },
        Err(CodexDiscoveryError::VersionUnsupported(version)) => {
            ActivationState::VersionUnsupported {
                reason: format!("Codex {version} is not activation-qualified by DevPULSE."),
                recovery: vec![RecoveryAction::new(
                    RecoveryActionKind::UpdateProvider,
                    "Update Codex",
                )],
            }
        }
        Err(error) => ActivationState::Blocked {
            reason: error.to_string(),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::OpenDiagnostics,
                "Open Codex diagnostics",
            )],
        },
    }
}

fn activation_from_status(status: CodexLoginStatus) -> ActivationState {
    match status {
        CodexLoginStatus::ChatGpt => ActivationState::Ready,
        CodexLoginStatus::ApiKey => ActivationState::SignedOut {
            reason: "Codex is using API-key billing. This tab requires ChatGPT subscription login."
                .to_owned(),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::SignIn,
                "Log in with ChatGPT",
            )],
        },
        CodexLoginStatus::SignedOut => ActivationState::SignedOut {
            reason: "Log in with ChatGPT to activate subscription-backed Codex.".to_owned(),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::SignIn,
                "Log in with ChatGPT",
            )],
        },
    }
}

fn failed_state(code: Option<i32>) -> ActivationState {
    ActivationState::Failed {
        reason: code.map_or_else(
            || "Codex login exited without a success status.".to_owned(),
            |code| format!("Codex login exited with status {code}."),
        ),
        recovery: retry_recovery(),
    }
}

fn retry_recovery() -> Vec<RecoveryAction> {
    vec![RecoveryAction::new(
        RecoveryActionKind::Retry,
        "Try ChatGPT login again",
    )]
}

fn canonical_directory(path: &Path) -> Result<PathBuf, CodexDiscoveryError> {
    let path = fs::canonicalize(path)
        .map_err(|error| CodexDiscoveryError::WorkingDirectory(public_io_reason(&error)))?;
    path.is_dir().then_some(path).ok_or_else(|| {
        CodexDiscoveryError::WorkingDirectory("the selected path is not a directory".to_owned())
    })
}

fn discover_executable() -> Result<PathBuf, CodexDiscoveryError> {
    dpl_tools_deps::resolve_current(dpl_tools_deps::CODEX)
        .map(|dependency| dependency.path().to_owned())
        .map_err(|error| CodexDiscoveryError::EmbeddedDependency(error.to_string()))
}

fn probe_version(
    executable: &Path,
    working_directory: &Path,
) -> Result<CodexVersion, CodexDiscoveryError> {
    let mut command = profiled_command(executable, working_directory);
    command
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = command
        .output()
        .map_err(|error| CodexDiscoveryError::VersionProbe(public_io_reason(&error)))?;
    if !output.status.success() {
        return Err(CodexDiscoveryError::VersionProbe(format!(
            "the version command exited with status {}",
            output.status.code().unwrap_or(-1)
        )));
    }
    CodexVersion::parse(&bounded_output(&output)).ok_or_else(|| {
        CodexDiscoveryError::VersionProbe("the version output was not recognized".to_owned())
    })
}

fn probe_login_status(profile: &CodexRuntimeProfile) -> Result<CodexLoginStatus, io::Error> {
    let mut command = profiled_command(profile.executable(), profile.working_directory());
    command
        .args(["login", "status"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = command.output()?;
    parse_login_status(&bounded_output(&output))
        .ok_or_else(|| io::Error::other("the login status response was not recognized"))
}

fn parse_login_status(output: &str) -> Option<CodexLoginStatus> {
    let normalized = output.to_ascii_lowercase();
    if normalized.contains("logged in using chatgpt") {
        Some(CodexLoginStatus::ChatGpt)
    } else if normalized.contains("logged in using api key")
        || normalized.contains("logged in with api key")
    {
        Some(CodexLoginStatus::ApiKey)
    } else if normalized.contains("not logged in") || normalized.contains("signed out") {
        Some(CodexLoginStatus::SignedOut)
    } else {
        None
    }
}

fn bounded_output(output: &Output) -> String {
    let mut bytes = Vec::with_capacity(
        output
            .stdout
            .len()
            .saturating_add(output.stderr.len())
            .min(MAX_STATUS_BYTES),
    );
    bytes.extend_from_slice(&output.stdout[..output.stdout.len().min(MAX_STATUS_BYTES)]);
    let remaining = MAX_STATUS_BYTES.saturating_sub(bytes.len());
    bytes.extend_from_slice(&output.stderr[..output.stderr.len().min(remaining)]);
    String::from_utf8_lossy(&bytes).into_owned()
}

fn login_command(profile: &CodexRuntimeProfile) -> Command {
    let mut command = profiled_command(profile.executable(), profile.working_directory());
    command
        .arg("login")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn profiled_command(executable: &Path, working_directory: &Path) -> Command {
    let mut command = Command::new(executable);
    // The quiet-spawn law (W8): probes and login alike are console-free.
    dpl_proc::quiet(&mut command);
    command.current_dir(working_directory).env_clear();
    for key in allowed_environment_keys() {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
}

fn allowed_environment_keys() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &[
            "APPDATA",
            "CODEX_CA_CERTIFICATE",
            "CODEX_HOME",
            "LOCALAPPDATA",
            "PATH",
            "PATHEXT",
            "SSL_CERT_FILE",
            "SystemRoot",
            "TEMP",
            "TMP",
            "USERPROFILE",
            "WINDIR",
        ]
    }
    #[cfg(not(windows))]
    {
        &[
            "CODEX_CA_CERTIFICATE",
            "CODEX_HOME",
            "DBUS_SESSION_BUS_ADDRESS",
            "DISPLAY",
            "HOME",
            "LANG",
            "LC_ALL",
            "PATH",
            "SSL_CERT_DIR",
            "SSL_CERT_FILE",
            "WAYLAND_DISPLAY",
            "XDG_CONFIG_HOME",
            "XDG_RUNTIME_DIR",
        ]
    }
}

fn public_io_reason(error: &io::Error) -> String {
    match error.kind() {
        io::ErrorKind::NotFound => "required file was not found".to_owned(),
        io::ErrorKind::PermissionDenied => "permission was denied".to_owned(),
        io::ErrorKind::TimedOut => "the operation timed out".to_owned(),
        kind => format!("{kind:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };

    fn temp_dir(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("dpl-codex-{name}-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("temporary directory");
        directory
    }

    fn fixture_profile() -> CodexRuntimeProfile {
        let directory = temp_dir("profile");
        let executable = directory.join(format!("codex{}", std::env::consts::EXE_SUFFIX));
        fs::write(&executable, b"fixture").expect("fixture executable");
        CodexRuntimeProfile {
            executable,
            version: MINIMUM_VERSION,
            working_directory: directory,
        }
    }

    #[derive(Debug)]
    struct FakeLauncher {
        launches: Arc<AtomicUsize>,
        process: FakeProcess,
    }

    impl CodexLoginLauncher for FakeLauncher {
        fn launch_login(
            &self,
            _profile: &CodexRuntimeProfile,
        ) -> io::Result<Box<dyn CodexLoginProcess>> {
            self.launches.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(self.process.clone()))
        }
    }

    #[derive(Debug, Clone)]
    struct FakeProcess {
        exit: Option<CodexLoginExit>,
        terminated: Arc<AtomicBool>,
    }

    impl CodexLoginProcess for FakeProcess {
        fn try_exit(&mut self) -> io::Result<Option<CodexLoginExit>> {
            Ok(self.exit)
        }

        fn terminate_and_reap(&mut self) {
            self.terminated.store(true, Ordering::SeqCst);
        }
    }

    /// Each `try_status` call pops the next planned response.
    #[derive(Debug, Clone, Default)]
    struct FakeStatusProbe {
        plan: Vec<Option<Result<CodexLoginStatus, String>>>,
        begins: Arc<AtomicUsize>,
    }

    impl CodexStatusProbe for FakeStatusProbe {
        fn begin(&self, _profile: &CodexRuntimeProfile) -> io::Result<Box<dyn CodexStatusRequest>> {
            self.begins.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(FakeStatusRequest {
                plan: self.plan.clone(),
            }))
        }
    }

    #[derive(Debug)]
    struct FakeStatusRequest {
        plan: Vec<Option<Result<CodexLoginStatus, String>>>,
    }

    impl CodexStatusRequest for FakeStatusRequest {
        fn try_status(&mut self) -> Option<Result<CodexLoginStatus, String>> {
            if self.plan.is_empty() {
                return None;
            }
            self.plan.remove(0)
        }
    }

    fn coordinator_with(
        exit: Option<CodexLoginExit>,
        plan: Vec<Option<Result<CodexLoginStatus, String>>>,
    ) -> CodexActivationCoordinator {
        let launcher = FakeLauncher {
            launches: Arc::new(AtomicUsize::new(0)),
            process: FakeProcess {
                exit,
                terminated: Arc::new(AtomicBool::new(false)),
            },
        };
        CodexActivationCoordinator::from_parts(
            Ok(fixture_profile()),
            activation_from_status(CodexLoginStatus::SignedOut),
            Box::new(launcher),
            Box::new(FakeStatusProbe {
                plan,
                begins: Arc::new(AtomicUsize::new(0)),
            }),
        )
    }

    #[test]
    fn versions_and_login_status_are_fail_closed() {
        assert_eq!(
            CodexVersion::parse("codex-cli 0.145.0-alpha.18"),
            Some(MINIMUM_VERSION)
        );
        assert!(MINIMUM_VERSION.is_activation_supported());
        assert!(!CodexVersion::new(0, 144, 9).is_activation_supported());
        assert!(!CodexVersion::new(0, 146, 0).is_activation_supported());
        assert_eq!(
            parse_login_status("Logged in using ChatGPT"),
            Some(CodexLoginStatus::ChatGpt)
        );
        assert_eq!(
            parse_login_status("Logged in using API key"),
            Some(CodexLoginStatus::ApiKey)
        );
        assert_eq!(
            parse_login_status("Not logged in"),
            Some(CodexLoginStatus::SignedOut)
        );
        assert_eq!(parse_login_status("future status"), None);
    }

    #[test]
    fn production_discovery_is_owned_by_the_embedded_resolver() {
        let discovery = discover_executable();
        if let Ok(path) = discovery {
            assert!(path.is_absolute());
            assert!(path
                .components()
                .any(|component| component.as_os_str() == "dptools-deps"));
        }
    }

    #[test]
    fn login_is_single_owner_and_drop_reaps_it() {
        let launches = Arc::new(AtomicUsize::new(0));
        let terminated = Arc::new(AtomicBool::new(false));
        let launcher = FakeLauncher {
            launches: Arc::clone(&launches),
            process: FakeProcess {
                exit: None,
                terminated: Arc::clone(&terminated),
            },
        };
        let mut coordinator = CodexActivationCoordinator::from_parts(
            Ok(fixture_profile()),
            activation_from_status(CodexLoginStatus::SignedOut),
            Box::new(launcher),
            Box::new(FakeStatusProbe::default()),
        );
        coordinator.start_login().expect("start login");
        assert_eq!(
            coordinator.start_login(),
            Err(CodexActivationError::AlreadyRunning)
        );
        drop(coordinator);
        assert_eq!(launches.load(Ordering::SeqCst), 1);
        assert!(terminated.load(Ordering::SeqCst));
    }

    #[test]
    fn login_success_verifies_status_before_ready() {
        let mut coordinator = coordinator_with(
            Some(CodexLoginExit::Success),
            vec![None, Some(Ok(CodexLoginStatus::ChatGpt))],
        );
        coordinator.start_login().expect("start login");
        assert_eq!(coordinator.activation(), &ActivationState::SigningIn);
        assert!(coordinator.poll().expect("poll exit"));
        assert_eq!(coordinator.activation(), &ActivationState::Verifying);
        assert!(coordinator.take_login_completed());
        assert!(!coordinator.take_login_completed());
        assert!(coordinator.is_login_running());
        assert!(!coordinator.poll().expect("poll pending"));
        assert_eq!(coordinator.activation(), &ActivationState::Verifying);
        assert!(coordinator.poll().expect("poll verified"));
        assert_eq!(coordinator.activation(), &ActivationState::Ready);
        assert_eq!(
            coordinator.take_verified_status(),
            Some(CodexLoginStatus::ChatGpt)
        );
        assert_eq!(coordinator.take_verified_status(), None);
        assert!(!coordinator.is_login_running());
    }

    #[test]
    fn api_key_status_is_not_ready() {
        let mut coordinator = coordinator_with(
            Some(CodexLoginExit::Success),
            vec![Some(Ok(CodexLoginStatus::ApiKey))],
        );
        coordinator.start_login().expect("start login");
        assert!(coordinator.poll().expect("poll exit"));
        assert!(coordinator.poll().expect("poll verified"));
        let ActivationState::SignedOut { recovery, .. } = coordinator.activation() else {
            panic!(
                "API-key login must not be Ready: {:?}",
                coordinator.activation()
            );
        };
        assert_eq!(recovery[0].kind, RecoveryActionKind::SignIn);
        assert_eq!(
            coordinator.take_verified_status(),
            Some(CodexLoginStatus::ApiKey)
        );
    }

    #[test]
    fn status_probe_failure_is_retryable() {
        let mut coordinator = coordinator_with(
            Some(CodexLoginExit::Success),
            vec![Some(Err("boom".to_owned()))],
        );
        coordinator.start_login().expect("start login");
        assert!(coordinator.poll().expect("poll exit"));
        let error = coordinator.poll().expect_err("status failure surfaces");
        assert!(matches!(error, CodexActivationError::StatusFailed(_)));
        coordinator.fail(error);
        let ActivationState::Failed { recovery, .. } = coordinator.activation() else {
            panic!(
                "status failure must fail closed: {:?}",
                coordinator.activation()
            );
        };
        assert_eq!(recovery[0].kind, RecoveryActionKind::Retry);
        assert!(!coordinator.is_login_running());
    }

    #[test]
    fn login_failure_skips_verification() {
        let mut coordinator = coordinator_with(Some(CodexLoginExit::Failed(Some(3))), Vec::new());
        coordinator.start_login().expect("start login");
        assert!(coordinator.poll().expect("poll exit"));
        assert!(matches!(
            coordinator.activation(),
            ActivationState::Failed { .. }
        ));
        assert!(!coordinator.take_login_completed());
        assert_eq!(coordinator.take_verified_status(), None);
        assert!(!coordinator.is_login_running());
    }

    #[test]
    fn verification_blocks_duplicate_login() {
        let mut coordinator = coordinator_with(Some(CodexLoginExit::Success), vec![None]);
        coordinator.start_login().expect("start login");
        assert!(coordinator.poll().expect("poll exit"));
        assert_eq!(coordinator.activation(), &ActivationState::Verifying);
        assert_eq!(
            coordinator.start_login(),
            Err(CodexActivationError::AlreadyRunning)
        );
    }

    #[test]
    fn refresh_is_rejected_while_login_runs() {
        let mut coordinator = coordinator_with(None, Vec::new());
        coordinator.start_login().expect("start login");
        assert_eq!(
            coordinator.refresh_discovery(),
            Err(CodexActivationError::AlreadyRunning)
        );
    }

    #[test]
    fn fixed_login_command_uses_only_the_literal_login_argument() {
        let profile = fixture_profile();
        let command = login_command(&profile);
        assert_eq!(command.get_program(), profile.executable());
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![OsStr::new("login")]
        );
        assert_eq!(command.get_current_dir(), Some(profile.working_directory()));
    }

    #[test]
    fn staged_official_codex_can_be_discovered_when_present() {
        let working_directory = std::env::current_dir().expect("cwd");
        let profile = CodexRuntimeProfile::discover(working_directory);
        if discover_executable().is_ok() {
            let profile = profile.expect("installed official Codex must qualify");
            assert!(profile.version().is_activation_supported());
            assert_eq!(crate::PROVIDER.0, "codex");
        }
    }
}
