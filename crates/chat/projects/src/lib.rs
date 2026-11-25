//! Per-project chat storage (R303, ruled 2026-11-14). A project's chats live
//! in the library, at `<store>/<project id>/`, where the caller passes the
//! store (`<library>/.usr/.ai/projects`). The project folder keeps only the
//! link back: `<workspace>/.devpulse/chat/project.json`, its identity marker.
//!
//! No ambient workspace or library discovery, provider processes, credentials
//! or UI. Every handle binds a checked marker identity.
//!
//! Saves create immutable, checksummed revisions and a final commit record.
//! The first revision is full; subsequent revisions store changed JSON leaves.
//! Interrupted revisions remain intact; corrupted committed data fails closed.

mod carry;
mod delta;
mod disk;
mod journal;
pub mod profile;
pub mod registry;
pub mod trust;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const VERSION: u32 = 1;
const MARKER: &str = "project.json";
/// The link a project folder keeps: where its marker sits.
const LINK: &str = ".devpulse/chat";
/// The journal of a project's chats and drafts, inside its archive.
const CHATS: &str = "chats";
const MAX_MARKER_BYTES: u64 = 4096;
static IDS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProjectId(String);

impl ProjectId {
    pub fn parse(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 96
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err("Invalid project identity".into());
        }
        let upper = value.to_ascii_uppercase();
        if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (upper.len() == 4
                && (upper.starts_with("COM") || upper.starts_with("LPT"))
                && matches!(upper.as_bytes()[3], b'1'..=b'9'))
        {
            return Err("Invalid reserved project identity".into());
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn mint() -> Result<Self, String> {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos();
        Self::parse(format!(
            "project-{time:x}-{:x}-{:x}",
            std::process::id(),
            IDS.fetch_add(1, Ordering::Relaxed)
        ))
    }
}

impl TryFrom<String> for ProjectId {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl From<ProjectId> for String {
    fn from(value: ProjectId) -> Self {
        value.0
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    v: u32,
    id: ProjectId,
    label: String,
}

fn label(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 240 || value.chars().any(char::is_control) {
        return Err(
            "Give the project a name of at most 240 bytes without control characters".into(),
        );
    }
    Ok(value.to_owned())
}

#[derive(Debug, Clone)]
pub struct Project {
    workspace: PathBuf,
    /// `<workspace>/.devpulse/chat`: the marker, and nothing else.
    link: PathBuf,
    /// `<store>/<id>`: the chats, in the library.
    archive: PathBuf,
    marker: Marker,
    expected_revision: Arc<Mutex<Option<u64>>>,
}

impl Project {
    /// Set a project up in an explicitly chosen existing workspace, keeping
    /// its chats in `store`. An existing marked project is opened; existing
    /// unmarked data is refused.
    pub fn create(store: &Path, workspace: &Path, display_name: &str) -> Result<Self, String> {
        label(display_name)?;
        disk::existing_directory(workspace)?;
        let link = workspace.join(LINK);
        disk::path(&link)?;
        if fs::symlink_metadata(link.join(MARKER)).is_ok() {
            return Self::open(store, workspace);
        }
        Self::create_new(store, workspace, display_name)
    }

    /// Create a distinct project for a recovery copy. Unlike `create`, this
    /// never opens an existing marker, including another creator's race winner.
    /// Any existing marked or populated unmarked link remains untouched.
    pub fn create_new(store: &Path, workspace: &Path, display_name: &str) -> Result<Self, String> {
        let display_name = label(display_name)?;
        disk::existing_directory(workspace)?;
        disk::path(store)?;
        let link = workspace.join(LINK);
        disk::path(&link)?;
        disk::empty_or_absent(&link)?;
        disk::create_directories(&link)?;
        let marker = Marker {
            v: VERSION,
            id: ProjectId::mint()?,
            label: display_name,
        };
        let archive = store.join(marker.id.as_str());
        disk::empty_or_absent(&archive)?;
        disk::create_directories(&archive)?;
        // The marker is created exclusively, so one creator wins a race; a
        // loser takes back the empty archive folder it made.
        if let Err(error) = disk::create_json(&link.join(MARKER), &marker) {
            let _ = fs::remove_dir(&archive);
            return Err(error);
        }
        let project = Self {
            workspace: workspace.to_path_buf(),
            link,
            archive,
            marker,
            expected_revision: Arc::new(Mutex::new(None)),
        };
        project.load_state()?;
        Ok(project)
    }

    pub fn open(store: &Path, workspace: &Path) -> Result<Self, String> {
        disk::existing_directory(workspace)?;
        disk::path(store)?;
        let link = workspace.join(LINK);
        disk::existing_directory(&link)?;
        let marker: Marker = disk::json(&link.join(MARKER), MAX_MARKER_BYTES)?;
        if marker.v != VERSION {
            return Err(
                "This project chat format is not supported by this version of DevPULSE".into(),
            );
        }
        label(&marker.label)?;
        let archive = store.join(marker.id.as_str());
        // Chats an older build kept inside the project folder are copied into
        // the library once; the project folder's copy is left as it was.
        carry::from_link(&link, store, &archive)?;
        if !archive.is_dir() {
            return Err(format!(
                "The chats of '{}' are not in this library. Open the library they were saved in, or save the current chats as a new project",
                marker.label
            ));
        }
        let project = Self {
            workspace: workspace.to_path_buf(),
            link,
            archive,
            marker,
            expected_revision: Arc::new(Mutex::new(None)),
        };
        // Corrupt committed state must never become an apparently empty project.
        project.load_state()?;
        Ok(project)
    }

    pub fn id(&self) -> &ProjectId {
        &self.marker.id
    }
    pub fn label(&self) -> &str {
        &self.marker.label
    }
    pub fn workspace(&self) -> &Path {
        &self.workspace
    }
    /// Where this project's chats are kept, in the library.
    pub fn archive_dir(&self) -> &Path {
        &self.archive
    }
    /// The link the project folder keeps: the folder its marker sits in.
    pub fn link_dir(&self) -> &Path {
        &self.link
    }
    /// The store this project's archive is in.
    pub fn store(&self) -> &Path {
        self.archive.parent().unwrap_or(&self.archive)
    }

    pub fn verify(&self) -> Result<(), String> {
        disk::existing_directory(&self.workspace)?;
        disk::existing_directory(&self.link)?;
        disk::existing_directory(&self.archive)?;
        let marker: Marker = disk::json(&self.link.join(MARKER), MAX_MARKER_BYTES)?;
        if marker != self.marker {
            return Err("The project at this location has changed; reconnect the original project before saving".into());
        }
        Ok(())
    }

    /// Accept the current committed state and advance this handle's expected
    /// revision. Clones share that expectation and represent one logical
    /// editor; callers must replace their in-memory state with the return value
    /// before saving again. Independent editors must use independently opened
    /// Project handles so stale saves are refused.
    pub fn load_state(&self) -> Result<Option<Value>, String> {
        let mut expected = self
            .expected_revision
            .lock()
            .map_err(|_| "Project revision tracking is unavailable; reopen the project")?;
        self.verify()?;
        let saved = journal::latest(&self.archive.join(CHATS))?;
        *expected = saved.as_ref().map(|(revision, _)| *revision);
        Ok(saved.map(|(_, value)| value))
    }

    /// Save only after verifying this handle still names the same project.
    /// Identical snapshots are deduplicated without another revision.
    pub fn save_state(&self, state: &Value) -> Result<(), String> {
        self.verify()?;
        let mut expected = self
            .expected_revision
            .lock()
            .map_err(|_| "Project revision tracking is unavailable; reopen the project")?;
        let _lock = disk::lock(&self.archive)?;
        self.verify()?;
        *expected = Some(journal::append(
            &self.archive.join(CHATS),
            state,
            Some(*expected),
        )?);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
