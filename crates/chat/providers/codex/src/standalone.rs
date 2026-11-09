//! Named, project-bound Codex CLI entry. No arbitrary command or profile input.
//! Qualification is deliberately closed until the real storage/lifetime drills
//! pass; building this launcher is not evidence that portable Codex is supported.

mod process;

use crate::{CodexRuntimeProfile, CodexVersion};
use dpl_chat_core::PermissionMode;
use dpl_chat_projects::{CodexProfileCandidate, ProfileGeneration, ProjectId, Registry};
use std::io::{self, IsTerminal};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const UNQUALIFIED: &str = "Project-bound standalone Codex is not yet qualified on this platform. Saved project chats remain available in DevPULSE.";
const VERIFY_INTERVAL: Duration = Duration::from_millis(500);

/// There is no caller-controlled override or persisted boolean that can promote
/// the unqualified storage candidate. Live Windows/Linux evidence is pending.
pub const fn is_qualified() -> bool {
    false
}

/// Resolve only an existing, explicitly identified binding and generation.
/// Refuses before filesystem discovery or provider probes while unqualified.
pub fn open_project(
    registry: &Path,
    project: &ProjectId,
    generation: &ProfileGeneration,
    mode: PermissionMode,
) -> Result<CodexStandalone, String> {
    if !is_qualified() {
        return Err(UNQUALIFIED.into());
    }
    let registry = Registry::open_existing(registry)?;
    let project = registry
        .list()?
        .into_iter()
        .find(|binding| &binding.id == project)
        .ok_or("This project is not registered on this computer")?
        .open()?;
    let candidate = CodexProfileCandidate::open(&registry, &project, generation)?;
    let lease = Arc::new(candidate.lease()?);
    let profile =
        CodexRuntimeProfile::discover_candidate(lease).map_err(|error| error.to_string())?;
    CodexStandalone::start(&profile, mode).map_err(|error| error.to_string())
}

/// Owns the provider lifetime independently of the Chat window. The profile
/// retains its shared OS lease through process cleanup. No detach operation.
#[derive(Debug)]
pub struct CodexStandalone {
    child: process::OwnedProcess,
    profile: CodexRuntimeProfile,
    checked_at: Instant,
}

impl CodexStandalone {
    pub fn start(profile: &CodexRuntimeProfile, mode: PermissionMode) -> io::Result<Self> {
        if !is_qualified() || !profile.native_sessions_qualified() {
            return Err(io::Error::new(io::ErrorKind::Unsupported, UNQUALIFIED));
        }
        if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Open the named Codex project entry in an interactive terminal",
            ));
        }
        let command = bound_command(profile, mode)?;
        let child = process::OwnedProcess::spawn(command)?;
        Ok(Self {
            child,
            profile: profile.clone(),
            checked_at: Instant::now(),
        })
    }

    /// Poll from the owning console loop, never from a GUI frame. A missing or
    /// replaced binding stops this owned client; it cannot silently switch home.
    /// Detection is periodic, not a promise to prevent every external disk write.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if let Some(status) = self.child.try_wait()? {
            self.child.terminate_and_reap()?;
            return Ok(Some(status));
        }
        if self.checked_at.elapsed() >= VERIFY_INTERVAL {
            if let Err(error) = self.profile.verify_project_profile() {
                self.child.terminate_and_reap()?;
                return Err(error);
            }
            self.checked_at = Instant::now();
        }
        Ok(None)
    }

    pub fn terminate_and_reap(&mut self) -> io::Result<()> {
        self.child.terminate_and_reap()
    }
}

impl Drop for CodexStandalone {
    fn drop(&mut self) {
        // Explicitly reap before the profile's shared lease can be released.
        if self.child.terminate_and_reap().is_err() {
            // A failed cleanup cannot authorize relocation while an owned
            // provider might still write. Retain its shared lock for this
            // process lifetime instead of claiming the client has stopped.
            // Parent-crash recovery remains a separate qualification gate.
            std::mem::forget(self.profile.clone());
        }
    }
}

pub(crate) fn bound_command(
    profile: &CodexRuntimeProfile,
    mode: PermissionMode,
) -> io::Result<Command> {
    profile.verify_project_profile()?;
    if Some(profile.version()) != CodexVersion::parse(crate::runtime::PINNED_CLI_VERSION) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Standalone Codex requires the exact qualified provider version",
        ));
    }
    let mut command = profile.command()?;
    command
        .args(["--no-daemon", "--no-alt-screen", "--sandbox"])
        .arg(match mode {
            PermissionMode::Workspace => "workspace-write",
            PermissionMode::AskFirst | PermissionMode::ReadOnly => "read-only",
        })
        .arg("--ask-for-approval")
        .arg(match mode {
            PermissionMode::ReadOnly => "never",
            PermissionMode::AskFirst | PermissionMode::Workspace => "on-request",
        })
        .args(["-c", "sandbox_workspace_write.network_access=false", "--cd"])
        .arg(profile.working_directory())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    // The explicitly invoked console entry shares its existing terminal.
    // Activation/status/app-server commands retain their quiet-spawn policy.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0);
    }
    #[cfg(unix)]
    for key in ["TERM", "COLORTERM", "TERMINFO", "TERMINFO_DIRS"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    Ok(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unqualified_entry_refuses_before_registry_or_provider_access() {
        let result = open_project(
            Path::new("missing-registry-must-not-be-created"),
            &ProjectId::parse("project-fixture").unwrap(),
            &ProfileGeneration::parse("generation-fixture").unwrap(),
            PermissionMode::Workspace,
        );
        assert_eq!(result.unwrap_err(), UNQUALIFIED);
        assert!(!is_qualified());
    }
}
