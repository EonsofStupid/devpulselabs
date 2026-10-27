//! Portable library root ownership; the dependency ceiling is serde, serde_json, toml.
//!
//! Discovery is read-only. Only explicit setup creates identity/configuration.
//! ST1 supplies snapshots and marker checks; ST2a watches for a library that
//! isn't connected and remembers where it reappears; ST2b owns live IO.

mod discovery;
mod disk;
mod handle;
mod migrate;
mod model;
mod setup;

pub use discovery::candidates_on_roots;
pub use disk::read_marker;
pub use handle::Roots;
pub use migrate::{has_legacy, plan_move, MoveItem, MovePlan, MoveProgress, LEGACY_RINGS};
pub use model::{
    AccountId, Config, DrillRoots, Layout, LibraryId, Marker, Mode, Publish, State, CONFIG_FILE,
    MARKER_FILE, SCHEMA_VERSION,
};
pub use setup::{setup_at, stage_roots};

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;

static ROOTS: OnceLock<RwLock<Option<Roots>>> = OnceLock::new();

fn global() -> &'static RwLock<Option<Roots>> {
    ROOTS.get_or_init(|| RwLock::new(None))
}

fn exe_dir() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "The executable has no parent directory".into())
}

/// Where this program keeps `roots.toml` and an imported `library\`: the
/// executable's folder, or a build's project (`project_of_build`).
fn program_dir() -> Result<PathBuf, String> {
    let exe = exe_dir()?;
    Ok(project_of_build(&exe).unwrap_or(exe))
}

/// A build attunes to its project (R291): an executable in
/// `<project>/target/<profile>/` beside `<project>/Cargo.toml` keeps its
/// library in `<project>`, out of `cargo clean`'s reach. Exactly two fixed
/// levels are checked; nothing walks.
pub fn project_of_build(exe_dir: &Path) -> Option<PathBuf> {
    let target = exe_dir.parent()?;
    if target.file_name()? != "target" {
        return None;
    }
    let project = target.parent()?;
    project
        .join("Cargo.toml")
        .is_file()
        .then(|| project.to_path_buf())
}

/// The folder `program_dir` settled on, for the startup screen to name.
pub fn program_folder() -> Result<PathBuf, String> {
    program_dir()
}

/// True when this executable is a build whose program folder is its project.
pub fn is_build() -> bool {
    exe_dir().is_ok_and(|exe| project_of_build(&exe).is_some())
}

/// The program folder's own old-layout rings, when it has any: the source
/// the first screen offers to bring in. Only that one folder is looked at.
pub fn legacy_source() -> Option<PathBuf> {
    program_dir().ok().filter(|folder| has_legacy(folder))
}

/// Parse only explicit root seams; absence never means the launch directory.
pub fn parse_overrides(
    args: &[OsString],
    content: Option<OsString>,
    assets: Option<OsString>,
    userdata: Option<OsString>,
    cwd: &Path,
) -> Result<Option<DrillRoots>, String> {
    let mut selected = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let candidate = if arg == OsStr::new("--content-root") {
            let next = args.next().ok_or("--content-root needs a folder")?;
            if next.to_string_lossy().starts_with("--") {
                return Err("--content-root needs a folder, not another flag".into());
            }
            Some(next.clone())
        } else {
            arg.to_str()
                .and_then(|s| s.strip_prefix("--content-root="))
                .map(OsString::from)
        };
        if let Some(candidate) = candidate {
            if candidate.is_empty() {
                return Err("--content-root needs a non-empty folder".into());
            }
            if selected.is_some() {
                return Err("Specify --content-root only once".into());
            }
            selected = Some(candidate);
        }
    }
    let Some(content) = selected.or(content) else {
        if assets.is_some() || userdata.is_some() {
            return Err("ASSETS_ROOT and USERDATA_ROOT require an explicit content root".into());
        }
        return Ok(None);
    };
    Ok(Some(DrillRoots {
        content: disk::absolute(Path::new(&content), cwd)?,
        assets: assets
            .map(|p| disk::absolute(Path::new(&p), cwd))
            .transpose()?,
        userdata: userdata
            .map(|p| disk::absolute(Path::new(&p), cwd))
            .transpose()?,
    }))
}

fn process_roots() -> Roots {
    let program = match program_dir() {
        Ok(path) => path,
        Err(error) => return Roots::failed(PathBuf::new(), error),
    };
    // CWD is used only to interpret an explicitly supplied relative path,
    // never as a candidate library. A failed current_dir need not prevent
    // ordinary executable-adjacent configuration from loading.
    let cwd = std::env::current_dir().unwrap_or_default();
    let overrides = parse_overrides(
        &std::env::args_os().skip(1).collect::<Vec<_>>(),
        std::env::var_os("CONTENT_ROOT"),
        std::env::var_os("ASSETS_ROOT"),
        std::env::var_os("USERDATA_ROOT"),
        &cwd,
    );
    match overrides {
        Ok(overrides) => {
            let mut roots = Roots::open(&program, overrides);
            if matches!(roots.state(), State::Disconnected { .. }) {
                let _ = roots.refresh();
            }
            roots
        }
        Err(error) => Roots::failed(program, error),
    }
}

/// Install a fresh handle before any stores open. Repeating this is intended
/// for the startup Retry action, not for switching open stores mid-session.
pub fn initialize() -> Result<Layout, String> {
    let mut roots = process_roots();
    let result = roots.layout();
    let mut installed = global()
        .write()
        .map_err(|_| "Library root state could not be locked")?;
    remember(&mut roots);
    *installed = Some(roots);
    result
}

/// A linked library found at a new path keeps it in `roots.toml`. Failing
/// to write that never stops the library opening; it is said instead.
fn remember(roots: &mut Roots) {
    if let Err(error) = roots.remember() {
        roots.notice = Some(format!(
            "Your library was found in a new place, but roots.toml couldn't be updated ({error}). It will be looked for again next time."
        ));
    }
}

/// What the last `initialize` noticed without failing, for the app to log.
pub fn notice() -> Option<String> {
    global().read().ok()?.as_ref()?.notice.clone()
}

/// One look for a library that isn't connected, for the Disconnected
/// screen's watch. The probe runs outside the lock, so a slow or empty drive
/// never blocks a frame. Its result is installed only when it connects and
/// no `roots.toml` was written meanwhile: the screen's own choice always
/// wins. A seam never probes.
pub fn reconnect() -> Option<Layout> {
    reconnect_with(process_roots)
}

fn reconnect_with(probe: impl FnOnce() -> Roots) -> Option<Layout> {
    let writes = disk::CONFIG_WRITES.load(Ordering::Acquire);
    if let Some(roots) = global().read().ok()?.as_ref() {
        if roots.is_drill() {
            return None;
        }
        if let State::Connected(layout) = roots.state() {
            return Some(layout.clone());
        }
    }
    let mut roots = probe();
    let State::Connected(layout) = roots.state().clone() else {
        return None;
    };
    let mut installed = global().write().ok()?;
    if disk::CONFIG_WRITES.load(Ordering::Acquire) != writes {
        return None;
    }
    remember(&mut roots);
    *installed = Some(roots);
    Some(layout)
}

/// Looks for the library every two seconds on its own thread, then calls
/// `found` once it is connected. Dropping the watch stops it.
pub struct Watch {
    stop: Arc<AtomicBool>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

pub fn watch(found: impl FnOnce() + Send + 'static) -> Result<Watch, String> {
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = Arc::clone(&stop);
    std::thread::Builder::new()
        .name("dpl-roots-watch".into())
        .spawn(move || loop {
            // Short naps, so a dropped watch ends within a tenth of a second.
            for _ in 0..20 {
                if stopped.load(Ordering::Acquire) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            if reconnect().is_some() {
                if !stopped.load(Ordering::Acquire) {
                    found();
                }
                return;
            }
        })
        .map_err(|e| format!("The library watch could not start: {e}"))?;
    Ok(Watch { stop })
}

/// The configured library while it isn't connected, for the Disconnected
/// screen to name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub label: String,
    pub linked: bool,
    /// Where it was last seen.
    pub last_path: PathBuf,
    pub reason: String,
}

pub fn status() -> Option<Status> {
    let roots = global().read().ok()?;
    let roots = roots.as_ref()?;
    let (State::Disconnected { reason }, Some(config)) = (roots.state(), roots.config()) else {
        return None;
    };
    Some(Status {
        label: config.label.clone(),
        linked: config.mode == Mode::Linked,
        last_path: config.library_path(&roots.program).ok()?,
        reason: reason.clone(),
    })
}

/// What a folder the person points at (Locate…) holds. Nothing is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Located {
    /// This program's own library, in a new place.
    Same(PathBuf),
    /// A different library: the screen asks before switching to it.
    Other { library: PathBuf, label: String },
}

/// `program` is `program_folder()` outside tests.
pub fn locate(program: &Path, folder: &Path) -> Result<Located, String> {
    if !folder.is_absolute() {
        return Err("Paste the full path of your library's folder.".into());
    }
    let library = disk::absolute(folder, program)?;
    if !library.join(MARKER_FILE).is_file() {
        return Err(format!(
            "{} isn't a DevPULSE library: it has no {MARKER_FILE}. Choose the folder that holds {MARKER_FILE}.",
            library.display()
        ));
    }
    let marker = read_marker(&library)?;
    let account = AccountId::default();
    handle::validate_layout(&Layout::for_library(&library, &account), &account)?;
    Ok(match disk::read_config(program) {
        Ok(config) if config.id == marker.id => Located::Same(library),
        _ => Located::Other {
            library,
            label: marker.label,
        },
    })
}

/// Point `program` at the library in `folder` (Locate…): `roots.toml` is
/// replaced whole and the folder is only read. The caller then opens it
/// with `initialize`.
pub fn link_to(program: &Path, folder: &Path) -> Result<(), String> {
    setup::stage(program, folder, Publish::Replace)
}

pub fn needs_setup() -> bool {
    global()
        .read()
        .ok()
        .and_then(|roots| {
            roots
                .as_ref()
                .map(|r| matches!(r.state(), State::Unconfigured))
        })
        .unwrap_or(false)
}

pub fn try_layout() -> Result<Layout, String> {
    {
        let roots = global()
            .read()
            .map_err(|_| "Library root state could not be locked")?;
        if let Some(roots) = roots.as_ref() {
            return roots.layout();
        }
    }
    initialize()
}

fn connected() -> Layout {
    try_layout()
        .expect("Library access requires a connected root; gate startup with dpl_roots::initialize")
}

pub fn content_root() -> PathBuf {
    connected().content
}
pub fn assets_root() -> PathBuf {
    connected().assets
}
pub fn userdata_root() -> PathBuf {
    connected().userdata
}
pub fn aidata_root() -> PathBuf {
    connected().aidata
}
pub fn cache_root() -> PathBuf {
    connected().cache
}
pub fn temp_root() -> PathBuf {
    connected().temp
}

pub fn ensure_rings() -> Result<Layout, String> {
    try_layout()?;
    let mut roots = global()
        .write()
        .map_err(|_| "Library root state could not be locked")?;
    roots
        .as_mut()
        .ok_or("Library is not configured")?
        .ensure_rings()
}

pub fn assets_capture_dir(kind: &str, job: &str) -> Result<PathBuf, String> {
    AccountId::parse(kind)?;
    AccountId::parse(job)?;
    let layout = ensure_rings()?;
    let dir = layout.assets.join("captures").join(kind).join(job);
    disk::no_symlinks(&dir)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Explicit legacy root-parameterized tools retain the same sibling derivation.
pub fn ring_sibling(content: &Path, name: &str) -> PathBuf {
    content.parent().unwrap_or(content).join(name)
}

pub fn checked_roots() -> Vec<String> {
    let _ = try_layout();
    global()
        .read()
        .ok()
        .and_then(|roots| {
            roots.as_ref().map(|roots| {
                roots
                    .checked_roots()
                    .iter()
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .collect()
            })
        })
        .unwrap_or_default()
}

pub fn locate_asset(rel: &str) -> Option<PathBuf> {
    try_layout().ok()?;
    global().read().ok()?.as_ref()?.locate_asset(rel)
}

/// The first screen's choices write `program`'s `roots.toml`; the caller
/// then opens the library with `initialize`. `publish` is `Replace` only
/// for "Use a library on this machine" on the Disconnected screen; a first
/// launch never replaces a `roots.toml`.
pub fn setup_imported(program: &Path, label: &str, publish: Publish) -> Result<Layout, String> {
    setup::setup_publishing(
        program,
        &program.join("library"),
        Mode::Imported,
        label,
        publish,
    )
}

pub fn setup_linked(
    program: &Path,
    path: &Path,
    label: &str,
    publish: Publish,
) -> Result<Layout, String> {
    setup::setup_publishing(program, path, Mode::Linked, label, publish)
}

/// Copy today's folders under `source` into a new library at `library` and
/// write `program`'s `roots.toml` (ST1b); the caller then opens it with
/// `initialize`. The program's own `library\` is imported; anywhere else is
/// linked.
pub fn bring_in(
    program: &Path,
    source: &Path,
    library: &Path,
    label: &str,
    progress: &MoveProgress,
    publish: Publish,
) -> Result<Layout, String> {
    migrate::move_in(program, source, library, label, progress, publish)
}

#[cfg(test)]
mod tests;
