//! A library unplugged mid-session (ST2b). Nothing is written while it is
//! away, nor into whatever drive takes its letter next: writers ask
//! `writable` first, and their edits wait in memory until it is back.
//! Nothing runs while connected (ruled 2026-10-27): an unplug is noticed at
//! the next write, or at a read under the root that fails (`io_failed`).

use crate::{disk, global, Roots, State};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Every place a library opened at this session, so a path cached before a
/// reconnect on another letter still lands in the library.
static SEEN: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
/// Bumped each time a library is opened, so the app can tell a reconnect.
static OPENED: AtomicU64 = AtomicU64::new(0);
/// When the marker was last read and matched: writes in a burst (typing)
/// cost one read every half second at most.
static CHECKED: Mutex<Option<Instant>> = Mutex::new(None);
const FRESH: Duration = Duration::from_millis(500);

/// Record a newly installed handle that opened a library.
pub(crate) fn opened(roots: &Roots) {
    let Some(layout) = roots.layout().ok() else {
        return;
    };
    if let Ok(mut seen) = SEEN.lock() {
        if !seen.contains(&layout.library) {
            seen.push(layout.library);
        }
    }
    if let Ok(mut checked) = CHECKED.lock() {
        *checked = Some(Instant::now());
    }
    OPENED.fetch_add(1, Ordering::AcqRel);
}

/// How many times a library has opened this session; it moves on each
/// reconnect.
pub fn generation() -> u64 {
    OPENED.load(Ordering::Acquire)
}

/// True while the library that was open is unplugged.
pub fn is_unplugged() -> bool {
    global()
        .read()
        .ok()
        .and_then(|roots| {
            roots
                .as_ref()
                .map(|roots| matches!(roots.state(), State::Unplugged { .. }))
        })
        .unwrap_or(false)
}

/// The configured library's name, for the app's banner.
pub fn label() -> Option<String> {
    Some(global().read().ok()?.as_ref()?.config()?.label.clone())
}

/// The open library's folder, for the app to compare after a reconnect.
pub fn library() -> Option<PathBuf> {
    global()
        .read()
        .ok()?
        .as_ref()?
        .layout()
        .ok()
        .map(|layout| layout.library)
}

/// Where `path` may be written now. A path inside the library is checked
/// against the library's identity (at most every half second), and one
/// cached from an earlier place this session is moved onto the current
/// one. While the library is unplugged, or another drive holds its place,
/// the write is refused in plain words. Paths outside the library, and
/// seams, pass unchanged.
pub fn writable(path: &Path) -> Result<PathBuf, String> {
    let (library, label, id) = {
        let roots = global()
            .read()
            .map_err(|_| "Library root state could not be locked")?;
        let Some(roots) = roots.as_ref() else {
            return Ok(path.to_path_buf());
        };
        if roots.is_drill() {
            return Ok(path.to_path_buf());
        }
        let label = roots
            .config()
            .map(|config| config.label.clone())
            .unwrap_or_default();
        match roots.state() {
            State::Connected(layout) => (
                layout.library.clone(),
                label,
                roots.config().map(|config| config.id.clone()),
            ),
            State::Unplugged { last, .. } => {
                return if under_any(path, &last.library) {
                    Err(unplugged(&label))
                } else {
                    Ok(path.to_path_buf())
                };
            }
            _ => return Err(roots.layout().err().unwrap_or_default()),
        }
    };
    let path = rebase(path, &library);
    if !path.starts_with(&library) {
        return Ok(path);
    }
    let fresh = CHECKED
        .lock()
        .ok()
        .and_then(|checked| *checked)
        .is_some_and(|at| at.elapsed() < FRESH);
    if fresh || matches(&library, id.as_ref()) {
        if !fresh {
            if let Ok(mut checked) = CHECKED.lock() {
                *checked = Some(Instant::now());
            }
        }
        return Ok(path);
    }
    unplug(&library, &label);
    Err(unplugged(&label))
}

/// A read or write under the library failed: check its identity now, so the
/// app says it was unplugged instead of showing an empty library.
pub fn io_failed(path: &Path) {
    let (library, label, id) = {
        let Ok(roots) = global().read() else {
            return;
        };
        let Some(roots) = roots.as_ref() else {
            return;
        };
        let State::Connected(layout) = roots.state() else {
            return;
        };
        if roots.is_drill() || !path.starts_with(&layout.library) {
            return;
        }
        (
            layout.library.clone(),
            roots
                .config()
                .map(|config| config.label.clone())
                .unwrap_or_default(),
            roots.config().map(|config| config.id.clone()),
        )
    };
    if !matches(&library, id.as_ref()) {
        unplug(&library, &label);
    }
}

fn unplugged(label: &str) -> String {
    format!("Your library '{label}' was unplugged — plug it back in to save.")
}

/// The library at `library` is still the one this program opened.
fn matches(library: &Path, id: Option<&crate::LibraryId>) -> bool {
    disk::read_marker(library).is_ok_and(|marker| Some(&marker.id) == id)
}

fn unplug(library: &Path, label: &str) {
    if let Ok(mut checked) = CHECKED.lock() {
        *checked = None;
    }
    if let Ok(mut roots) = global().write() {
        if let Some(roots) = roots.as_mut() {
            if roots.layout().is_ok_and(|layout| layout.library == library) {
                roots.unplug(format!(
                    "Your library '{label}' was unplugged from {}.",
                    library.display()
                ));
            }
        }
    }
}

/// `path` inside the library, or inside an earlier place it opened at this
/// session.
fn under_any(path: &Path, library: &Path) -> bool {
    path.starts_with(library)
        || SEEN
            .lock()
            .is_ok_and(|seen| seen.iter().any(|old| path.starts_with(old)))
}

/// A path cached under an earlier place this session, moved onto `library`.
fn rebase(path: &Path, library: &Path) -> PathBuf {
    if path.starts_with(library) {
        return path.to_path_buf();
    }
    let seen = SEEN.lock().map(|seen| seen.clone()).unwrap_or_default();
    seen.iter()
        .find_map(|old| path.strip_prefix(old).ok())
        .map(|rest| library.join(rest))
        .unwrap_or_else(|| path.to_path_buf())
}
