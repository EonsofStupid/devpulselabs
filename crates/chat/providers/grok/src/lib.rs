//! Grok provider descriptor, subscription activation, and bounded ACP runtime.
//!
//! The only production child this crate can start is the discovered official
//! Grok Build executable. Login uses the literal `login` argument; conversation
//! uses the fixed `--no-auto-update agent stdio` ACP command.

mod runtime;
mod terminal;

pub use runtime::*;

use dpl_chat_core::{
    ActivationState, Capability, ProviderAccent, ProviderDescriptor, ProviderId, ProviderManifest,
    ProviderProfileId, RecoveryAction, RecoveryActionKind,
};
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};

pub const PROVIDER: ProviderId = ProviderId("grok");
pub const DEFAULT_PROFILE: ProviderProfileId = ProviderProfileId("grok-default");
pub const SETUP_URL: &str = "https://docs.x.ai/build/overview";

pub const QUALIFIED_VERSION: GrokVersion = GrokVersion::new(0, 2, 112);
const MINIMUM_VERSION: GrokVersion = QUALIFIED_VERSION;
const EXCLUSIVE_MAXIMUM_VERSION: GrokVersion = GrokVersion::new(0, 3, 0);

const TARGETS: &[Capability] = &[
    Capability::ReadSessions,
    Capability::StartSession,
    Capability::ResumeSession,
    Capability::StreamTurns,
    Capability::InterruptTurn,
    Capability::Approvals,
    Capability::Tools,
    Capability::Diffs,
    Capability::Plans,
    Capability::Attachments,
    Capability::Images,
    Capability::Video,
    Capability::Models,
    Capability::Modes,
    Capability::SubscriptionAuth,
    Capability::DeviceCodeAuth,
    Capability::Headless,
];

pub const MANIFEST: ProviderManifest = ProviderManifest {
    descriptor: ProviderDescriptor {
        id: PROVIDER,
        label: "Grok",
        short_label: "Grok",
        glyph: "X",
        accent: ProviderAccent::Magenta,
        summary: "xAI coding agent through Grok Build's documented ACP mode.",
    },
    profile: DEFAULT_PROFILE,
    target_capabilities: TARGETS,
    delivery_slice: "CH7",
    evidence: "docs/research/chat-provider-capabilities.md#grok",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GrokVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl GrokVersion {
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

const fn version_at_least(value: GrokVersion, minimum: GrokVersion) -> bool {
    value.major > minimum.major
        || (value.major == minimum.major
            && (value.minor > minimum.minor
                || (value.minor == minimum.minor && value.patch >= minimum.patch)))
}

const fn version_before(value: GrokVersion, maximum: GrokVersion) -> bool {
    value.major < maximum.major
        || (value.major == maximum.major
            && (value.minor < maximum.minor
                || (value.minor == maximum.minor && value.patch < maximum.patch)))
}

impl fmt::Display for GrokVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrokRuntimeProfile {
    executable: PathBuf,
    version: GrokVersion,
    working_directory: PathBuf,
}

impl GrokRuntimeProfile {
    pub fn discover(working_directory: impl AsRef<Path>) -> Result<Self, GrokDiscoveryError> {
        let working_directory = canonical_directory(working_directory.as_ref())?;
        let executable = discover_executable().ok_or(GrokDiscoveryError::NotInstalled)?;
        let version = probe_version(&executable, &working_directory)?;
        if !version.is_activation_supported() {
            return Err(GrokDiscoveryError::VersionUnsupported(version));
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

    pub const fn version(&self) -> GrokVersion {
        self.version
    }

    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrokDiscoveryError {
    NotInstalled,
    WorkingDirectory(String),
    VersionProbe(String),
    VersionUnsupported(GrokVersion),
}

impl fmt::Display for GrokDiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotInstalled => f.write_str("Grok Build is not installed"),
            Self::WorkingDirectory(reason) => {
                write!(f, "the Grok working directory is invalid: {reason}")
            }
            Self::VersionProbe(reason) => write!(f, "Grok version could not be verified: {reason}"),
            Self::VersionUnsupported(version) => write!(
                f,
                "Grok Build {version} is outside the activation-qualified range >= {MINIMUM_VERSION}, < {EXCLUSIVE_MAXIMUM_VERSION}"
            ),
        }
    }
}

impl std::error::Error for GrokDiscoveryError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrokActivationError {
    Unavailable(String),
    AlreadyRunning,
    LaunchFailed(String),
    PollFailed(String),
}

impl fmt::Display for GrokActivationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(reason) => f.write_str(reason),
            Self::AlreadyRunning => f.write_str("Grok login is already running"),
            Self::LaunchFailed(reason) => write!(f, "Grok login could not start: {reason}"),
            Self::PollFailed(reason) => write!(f, "Grok login status failed: {reason}"),
        }
    }
}

impl std::error::Error for GrokActivationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginExit {
    Success,
    Failed(Option<i32>),
}

pub trait LoginProcess: fmt::Debug + Send {
    fn try_exit(&mut self) -> io::Result<Option<LoginExit>>;
    fn terminate_and_reap(&mut self);
}

pub trait LoginLauncher: fmt::Debug + Send {
    fn launch_login(&self, profile: &GrokRuntimeProfile) -> io::Result<Box<dyn LoginProcess>>;
}

#[derive(Debug, Default)]
struct SystemLoginLauncher;

impl LoginLauncher for SystemLoginLauncher {
    fn launch_login(&self, profile: &GrokRuntimeProfile) -> io::Result<Box<dyn LoginProcess>> {
        let mut command = login_command(profile);
        let child = command.spawn()?;
        Ok(Box::new(OwnedChild { child: Some(child) }))
    }
}

#[derive(Debug)]
struct OwnedChild {
    child: Option<Child>,
}

impl LoginProcess for OwnedChild {
    fn try_exit(&mut self) -> io::Result<Option<LoginExit>> {
        let Some(child) = self.child.as_mut() else {
            return Ok(Some(LoginExit::Failed(None)));
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

fn exit_from_status(status: ExitStatus) -> LoginExit {
    if status.success() {
        LoginExit::Success
    } else {
        LoginExit::Failed(status.code())
    }
}

#[derive(Debug)]
pub struct GrokActivationCoordinator {
    discovery_root: Option<PathBuf>,
    profile: Result<GrokRuntimeProfile, GrokDiscoveryError>,
    state: ActivationState,
    launcher: Box<dyn LoginLauncher>,
    child: Option<Box<dyn LoginProcess>>,
}

impl GrokActivationCoordinator {
    pub fn discover(working_directory: impl AsRef<Path>) -> Self {
        let working_directory = working_directory.as_ref().to_owned();
        let mut coordinator = Self::from_parts(
            GrokRuntimeProfile::discover(&working_directory),
            Box::<SystemLoginLauncher>::default(),
        );
        coordinator.discovery_root = Some(working_directory);
        coordinator
    }

    pub fn from_parts(
        profile: Result<GrokRuntimeProfile, GrokDiscoveryError>,
        launcher: Box<dyn LoginLauncher>,
    ) -> Self {
        let state = activation_from_discovery(&profile);
        Self {
            discovery_root: None,
            profile,
            state,
            launcher,
            child: None,
        }
    }

    pub fn activation(&self) -> &ActivationState {
        &self.state
    }

    pub fn activation_capability_qualified(&self) -> bool {
        self.profile.is_ok()
    }

    pub fn runtime_profile(&self) -> Option<GrokRuntimeProfile> {
        self.profile.as_ref().ok().cloned()
    }

    /// Re-runs fixed-path discovery after the operator completes provider-owned
    /// installation or update steps.
    pub fn refresh_discovery(&mut self) -> Result<bool, GrokActivationError> {
        if self.child.is_some() {
            return Err(GrokActivationError::AlreadyRunning);
        }
        let root = self.discovery_root.as_ref().ok_or_else(|| {
            GrokActivationError::Unavailable(
                "Grok discovery refresh is unavailable for this injected profile.".into(),
            )
        })?;
        let profile = GrokRuntimeProfile::discover(root);
        let state = activation_from_discovery(&profile);
        let changed = self.profile != profile || self.state != state;
        self.profile = profile;
        self.state = state;
        Ok(changed)
    }

    pub fn is_login_running(&self) -> bool {
        self.child.is_some()
    }

    pub fn start_login(&mut self) -> Result<(), GrokActivationError> {
        if self.child.is_some() {
            return Err(GrokActivationError::AlreadyRunning);
        }
        let profile = self
            .profile
            .as_ref()
            .map_err(|error| GrokActivationError::Unavailable(error.to_string()))?;
        let child = self
            .launcher
            .launch_login(profile)
            .map_err(|error| GrokActivationError::LaunchFailed(public_io_reason(&error)))?;
        self.child = Some(child);
        self.state = ActivationState::SigningIn;
        Ok(())
    }

    /// Poll the provider-owned login process without blocking the UI.
    ///
    /// Returns `true` when the visible activation state changed.
    pub fn poll(&mut self) -> Result<bool, GrokActivationError> {
        let Some(child) = self.child.as_mut() else {
            return Ok(false);
        };
        let exit = child
            .try_exit()
            .map_err(|error| GrokActivationError::PollFailed(public_io_reason(&error)))?;
        let Some(exit) = exit else {
            return Ok(false);
        };
        self.child = None;
        self.state = match exit {
            LoginExit::Success => ActivationState::Ready,
            LoginExit::Failed(code) => ActivationState::Failed {
                reason: code.map_or_else(
                    || "Grok Build login exited without a success status.".to_owned(),
                    |code| format!("Grok Build login exited with status {code}."),
                ),
                recovery: vec![RecoveryAction::new(
                    RecoveryActionKind::Retry,
                    "Try Grok login again",
                )],
            },
        };
        Ok(true)
    }

    pub fn fail(&mut self, error: GrokActivationError) {
        if let Some(mut child) = self.child.take() {
            child.terminate_and_reap();
        }
        self.state = ActivationState::Failed {
            reason: error.to_string(),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::Retry,
                "Try Grok login again",
            )],
        };
    }
}

impl Drop for GrokActivationCoordinator {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            child.terminate_and_reap();
        }
    }
}

fn activation_from_discovery(
    profile: &Result<GrokRuntimeProfile, GrokDiscoveryError>,
) -> ActivationState {
    match profile {
        Ok(profile) => ActivationState::SignedOut {
            reason: format!(
                "Grok Build {} is available. Log in to verify this DevPULSE session.",
                profile.version()
            ),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::SignIn,
                "Log in with Grok",
            )],
        },
        Err(GrokDiscoveryError::NotInstalled) => ActivationState::NotInstalled {
            reason: "Grok Build is required for provider-owned subscription login.".to_owned(),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::InstallInstructions,
                "Get Grok Build",
            )],
        },
        Err(GrokDiscoveryError::VersionUnsupported(version)) => {
            ActivationState::VersionUnsupported {
                reason: format!("Grok Build {version} is not activation-qualified by DevPULSE."),
                recovery: vec![RecoveryAction::new(
                    RecoveryActionKind::UpdateProvider,
                    "Update Grok Build",
                )],
            }
        }
        Err(error) => ActivationState::Blocked {
            reason: error.to_string(),
            recovery: vec![RecoveryAction::new(
                RecoveryActionKind::OpenDiagnostics,
                "Open Grok diagnostics",
            )],
        },
    }
}

fn canonical_directory(path: &Path) -> Result<PathBuf, GrokDiscoveryError> {
    let path = fs::canonicalize(path)
        .map_err(|error| GrokDiscoveryError::WorkingDirectory(public_io_reason(&error)))?;
    path.is_dir().then_some(path).ok_or_else(|| {
        GrokDiscoveryError::WorkingDirectory("the selected path is not a directory".to_owned())
    })
}

fn discover_executable() -> Option<PathBuf> {
    let home = std::env::var_os(home_environment_key()).map(PathBuf::from);
    let path = std::env::var_os("PATH");
    resolve_named_executable(home.as_deref(), path.as_deref())
}

fn resolve_named_executable(home: Option<&Path>, path: Option<&OsStr>) -> Option<PathBuf> {
    let file_name = executable_file_name();
    let official = home.map(|root| root.join(".grok").join("bin").join(file_name));
    official
        .into_iter()
        .chain(
            path.into_iter()
                .flat_map(std::env::split_paths)
                .map(|directory| directory.join(file_name)),
        )
        .find_map(|candidate| {
            candidate
                .is_file()
                .then(|| fs::canonicalize(candidate).ok())
                .flatten()
        })
}

#[cfg(windows)]
const fn executable_file_name() -> &'static str {
    "grok.exe"
}

#[cfg(not(windows))]
const fn executable_file_name() -> &'static str {
    "grok"
}

#[cfg(windows)]
const fn home_environment_key() -> &'static str {
    "USERPROFILE"
}

#[cfg(not(windows))]
const fn home_environment_key() -> &'static str {
    "HOME"
}

fn probe_version(
    executable: &Path,
    working_directory: &Path,
) -> Result<GrokVersion, GrokDiscoveryError> {
    let mut command = profiled_command(executable, working_directory);
    command
        .arg("version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = command
        .output()
        .map_err(|error| GrokDiscoveryError::VersionProbe(public_io_reason(&error)))?;
    if !output.status.success() {
        return Err(GrokDiscoveryError::VersionProbe(format!(
            "the version command exited with status {}",
            output.status.code().unwrap_or(-1)
        )));
    }
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.len() < 256 {
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    text.truncate(256);
    GrokVersion::parse(&text).ok_or_else(|| {
        GrokDiscoveryError::VersionProbe("the version output was not recognized".to_owned())
    })
}

fn login_command(profile: &GrokRuntimeProfile) -> Command {
    let mut command = profiled_command(profile.executable(), profile.working_directory());
    command
        .arg("login")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    hide_child_console(&mut command);
    command
}

fn profiled_command(executable: &Path, working_directory: &Path) -> Command {
    let mut command = Command::new(executable);
    command.current_dir(working_directory).env_clear();
    for key in allowed_environment_keys() {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
}

#[cfg(windows)]
fn hide_child_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_child_console(_command: &mut Command) {}

fn allowed_environment_keys() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &[
            "APPDATA",
            "LOCALAPPDATA",
            "PATH",
            "PATHEXT",
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
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };

    fn temp_dir(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("dpl-grok-{name}-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("temporary directory");
        directory
    }

    fn fixture_profile() -> GrokRuntimeProfile {
        let directory = temp_dir("profile");
        let executable = directory.join(executable_file_name());
        fs::write(&executable, b"fixture").expect("fixture executable");
        GrokRuntimeProfile {
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

    impl LoginLauncher for FakeLauncher {
        fn launch_login(&self, _profile: &GrokRuntimeProfile) -> io::Result<Box<dyn LoginProcess>> {
            self.launches.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(self.process.clone()))
        }
    }

    #[derive(Debug, Clone)]
    struct FakeProcess {
        exit: Option<LoginExit>,
        terminated: Arc<AtomicBool>,
    }

    impl LoginProcess for FakeProcess {
        fn try_exit(&mut self) -> io::Result<Option<LoginExit>> {
            Ok(self.exit)
        }

        fn terminate_and_reap(&mut self) {
            self.terminated.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn version_parser_and_band_are_explicit() {
        assert_eq!(
            GrokVersion::parse("Grok Build 0.2.112 (stable)"),
            Some(MINIMUM_VERSION)
        );
        assert!(MINIMUM_VERSION.is_activation_supported());
        assert!(GrokVersion::new(0, 2, 999).is_activation_supported());
        assert!(!GrokVersion::new(0, 2, 111).is_activation_supported());
        assert!(!GrokVersion::new(0, 3, 0).is_activation_supported());
        assert_eq!(GrokVersion::parse("not a version"), None);
    }

    #[test]
    fn official_install_precedes_path_and_names_are_fixed() {
        let home = temp_dir("home");
        let official = home.join(".grok").join("bin");
        fs::create_dir_all(&official).expect("official directory");
        let official_executable = official.join(executable_file_name());
        fs::write(&official_executable, b"official").expect("official executable");

        let path_directory = temp_dir("path");
        fs::write(path_directory.join(executable_file_name()), b"path").expect("path executable");
        let path = std::env::join_paths([path_directory]).expect("PATH");

        assert_eq!(
            resolve_named_executable(Some(&home), Some(&path)),
            Some(fs::canonicalize(official_executable).expect("canonical executable"))
        );
    }

    #[test]
    fn successful_login_transitions_once_and_duplicate_launch_is_rejected() {
        let launches = Arc::new(AtomicUsize::new(0));
        let terminated = Arc::new(AtomicBool::new(false));
        let launcher = FakeLauncher {
            launches: Arc::clone(&launches),
            process: FakeProcess {
                exit: Some(LoginExit::Success),
                terminated,
            },
        };
        let mut coordinator =
            GrokActivationCoordinator::from_parts(Ok(fixture_profile()), Box::new(launcher));

        coordinator.start_login().expect("start login");
        assert!(matches!(
            coordinator.activation(),
            ActivationState::SigningIn
        ));
        assert_eq!(
            coordinator.start_login(),
            Err(GrokActivationError::AlreadyRunning)
        );
        assert!(coordinator.poll().expect("poll"));
        assert!(matches!(coordinator.activation(), ActivationState::Ready));
        assert_eq!(launches.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn failed_login_is_truthful_and_retryable() {
        let launcher = FakeLauncher {
            launches: Arc::new(AtomicUsize::new(0)),
            process: FakeProcess {
                exit: Some(LoginExit::Failed(Some(7))),
                terminated: Arc::new(AtomicBool::new(false)),
            },
        };
        let mut coordinator =
            GrokActivationCoordinator::from_parts(Ok(fixture_profile()), Box::new(launcher));
        coordinator.start_login().expect("start login");
        assert!(coordinator.poll().expect("poll"));
        let ActivationState::Failed { reason, recovery } = coordinator.activation() else {
            panic!("failed activation");
        };
        assert!(reason.contains('7'));
        assert_eq!(recovery[0].kind, RecoveryActionKind::Retry);
    }

    #[test]
    fn dropping_a_running_login_reaps_the_owned_child() {
        let terminated = Arc::new(AtomicBool::new(false));
        let launcher = FakeLauncher {
            launches: Arc::new(AtomicUsize::new(0)),
            process: FakeProcess {
                exit: None,
                terminated: Arc::clone(&terminated),
            },
        };
        let mut coordinator =
            GrokActivationCoordinator::from_parts(Ok(fixture_profile()), Box::new(launcher));
        coordinator.start_login().expect("start login");
        drop(coordinator);
        assert!(terminated.load(Ordering::SeqCst));
    }

    #[test]
    fn fixed_login_command_has_no_shell_or_variable_arguments() {
        let profile = fixture_profile();
        let command = login_command(&profile);
        assert_eq!(command.get_program(), profile.executable());
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            vec![OsStr::new("login")]
        );
        assert_eq!(command.get_current_dir(), Some(profile.working_directory()));
        let keys = command
            .get_envs()
            .filter_map(|(key, value)| value.map(|_| key.to_os_string()))
            .collect::<Vec<std::ffi::OsString>>();
        assert!(keys
            .iter()
            .all(|key| allowed_environment_keys().contains(&key.to_string_lossy().as_ref())));
    }
}
