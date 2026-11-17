//! What is on this computer (AH1a): which catalog AIs are installed, and which
//! are running right now.
//!
//! Read only, and never on the frame loop:
//! - **installed** is an existence check of the install locations the catalog
//!   names. No file is opened.
//! - **running** is one process list of names and executable paths. No command
//!   line, environment or memory of another process is read.
//!
//! Sign-in is not here. It stays with each vendor's own status command
//! (`presence`), because only the vendor can say who is signed in.

use dpl_ai_core::catalog::{Catalog, CatalogEntry, Os};
use dpl_ai_core::provider::ProviderId;
use std::path::{Path, PathBuf};

/// What the scan saw for one catalog AI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sighting {
    pub provider: ProviderId,
    /// The first catalog install location that exists on this computer.
    pub installed_at: Option<PathBuf>,
    /// A process matching one of its tells is running.
    pub running: bool,
}

/// A running process, as the scan reads it: its executable name and path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunningProcess {
    pub name: String,
    pub exe: String,
}

/// Expand an install location written in the catalog: `%VAR%` segments on
/// Windows and a leading `~/`. `None` when a variable it needs is not set,
/// so a missing variable never turns into a relative path.
pub fn expand(path: &str, var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let mut out = String::new();
    let mut rest = path;
    if let Some(tail) = rest.strip_prefix("~/") {
        out.push_str(&var("HOME").or_else(|| var("USERPROFILE"))?);
        out.push('/');
        rest = tail;
    }
    while let Some(start) = rest.find('%') {
        let after = &rest[start + 1..];
        let end = after.find('%')?;
        out.push_str(&rest[..start]);
        out.push_str(&var(&after[..end])?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Some(PathBuf::from(out))
}

/// Where one AI is installed on this OS, if anywhere the catalog knows.
fn installed_at(
    entry: &CatalogEntry,
    os: Os,
    var: &impl Fn(&str) -> Option<String>,
    exists: &impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    entry
        .installs
        .iter()
        .filter(|install| install.os == os)
        .filter_map(|install| expand(install.path, var))
        .find(|path| exists(path))
}

/// Every catalog AI the scan has something to say about, in catalog order.
/// Pure: the environment, the file system and the process list come in as
/// arguments, so the rules are tested without touching this machine.
pub fn sightings(
    catalog: &Catalog,
    os: Os,
    processes: &[RunningProcess],
    var: impl Fn(&str) -> Option<String>,
    exists: impl Fn(&Path) -> bool,
) -> Vec<Sighting> {
    catalog
        .entries
        .iter()
        .filter(|entry| !entry.installs.is_empty() || !entry.tells.is_empty())
        .map(|entry| Sighting {
            provider: entry.id(),
            installed_at: installed_at(entry, os, &var, &exists),
            running: processes.iter().any(|process| {
                entry
                    .tells
                    .iter()
                    .any(|tell| tell.matches(os, &process.name, &process.exe))
            }),
        })
        .collect()
}

/// The processes running now: names and executable paths only.
pub fn running_processes() -> Vec<RunningProcess> {
    let Ok(mut system) = sysinfo::System::new() else {
        return Vec::new();
    };
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    system
        .processes()
        .values()
        .map(|process| RunningProcess {
            name: process.name().to_string_lossy().into_owned(),
            exe: process
                .exe()
                .map(|exe| exe.to_string_lossy().into_owned())
                .unwrap_or_default(),
        })
        .collect()
}

/// An install location exists. `symlink_metadata`, because Windows app
/// execution aliases (`…\WindowsApps\claude-desktop.exe`) are reparse points
/// that a plain `exists` cannot follow.
pub fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// Scan this computer once, on a worker thread. The receiver yields one list
/// and then closes; the frame loop polls it and never waits.
pub fn scan() -> std::sync::mpsc::Receiver<Vec<Sighting>> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let spawned = std::thread::Builder::new()
        .name("devpulse-ai-machine-scan".to_owned())
        .spawn(move || {
            let Some(os) = Os::current() else {
                return;
            };
            let found = sightings(
                dpl_ai_core::catalog::catalog(),
                os,
                &running_processes(),
                |name| std::env::var(name).ok(),
                exists,
            );
            let _ = sender.send(found);
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "the AI machine scan could not start");
    }
    receiver
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpl_ai_core::catalog::catalog;

    fn env(name: &str) -> Option<String> {
        match name {
            "LOCALAPPDATA" => Some(r"C:\Users\me\AppData\Local".to_owned()),
            "ProgramFiles" => Some(r"C:\Program Files".to_owned()),
            "USERPROFILE" => Some(r"C:\Users\me".to_owned()),
            "HOME" => Some("/home/me".to_owned()),
            _ => None,
        }
    }

    fn process(name: &str, exe: &str) -> RunningProcess {
        RunningProcess {
            name: name.to_owned(),
            exe: exe.to_owned(),
        }
    }

    #[test]
    fn install_paths_expand_variables_and_home() {
        assert_eq!(
            expand("%ProgramFiles%/Grok Bot/Grok Bot.exe", env),
            Some(PathBuf::from(r"C:\Program Files/Grok Bot/Grok Bot.exe"))
        );
        assert_eq!(
            expand("~/.local/bin/claude", env),
            Some(PathBuf::from("/home/me/.local/bin/claude"))
        );
        assert_eq!(expand("%NOPE%/x", env), None, "never a relative path");
        assert_eq!(expand("%UNCLOSED/x", env), None);
    }

    #[test]
    fn the_desktop_apps_on_this_laptop_are_seen_running_and_installed() {
        // The processes read off the operator's laptop on 2026-11-16.
        let processes = [
            process(
                "ChatGPT.exe",
                r"C:\Program Files\WindowsApps\OpenAI.Codex_26.1106.6628.0_x64__2p2nqsd0c76g0\app\ChatGPT.exe",
            ),
            process(
                "claude.exe",
                r"C:\Program Files\WindowsApps\Claude_5.11492.5.0_x64__pzs8sxrjxfjjc\app\Claude.exe",
            ),
            process("Grok Bot.exe", r"C:\Program Files\Grok Bot\Grok Bot.exe"),
        ];
        let installed = |path: &Path| {
            let path = path.to_string_lossy().replace('\\', "/");
            path.ends_with("WindowsApps/claude-desktop.exe")
                || path.ends_with("Grok Bot/Grok Bot.exe")
        };
        let found = sightings(catalog(), Os::Windows, &processes, env, installed);
        let seen = |id: &str| {
            found
                .iter()
                .find(|sighting| sighting.provider.0 == id)
                .cloned()
                .unwrap_or_else(|| panic!("{id} is scanned"))
        };
        assert!(seen("claude-desktop").running && seen("claude-desktop").installed_at.is_some());
        assert!(seen("chatgpt-desktop").running && seen("chatgpt-desktop").installed_at.is_none());
        assert!(seen("grok-bot").running && seen("grok-bot").installed_at.is_some());
        // Claude Desktop's process is not the Claude Code CLI.
        assert!(!seen("claude").running);
    }

    #[test]
    fn an_entry_with_nothing_to_look_for_is_not_scanned() {
        let found = sightings(catalog(), Os::Windows, &[], env, |_| true);
        assert!(found
            .iter()
            .all(|sighting| sighting.provider.0 != "meta-ai"));
        assert!(found.iter().all(|sighting| sighting.provider.0 != "goose"));
    }

    #[test]
    fn another_os_install_path_is_never_checked() {
        let found = sightings(catalog(), Os::Linux, &[], env, |path| {
            !path.to_string_lossy().contains("AppData")
        });
        let codex = found.iter().find(|sighting| sighting.provider.0 == "codex");
        assert!(codex.is_none_or(|sighting| sighting.installed_at.is_none()));
    }
}
