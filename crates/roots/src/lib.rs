//! Portable library root ownership; the dependency ceiling is serde, serde_json, toml.
//!
//! Discovery is read-only. Only explicit setup creates identity/configuration.
//! ST1 supplies snapshots and marker checks; ST2 owns live IO/store lifetimes.

mod discovery;
mod disk;
mod handle;
mod model;
mod setup;

pub use discovery::candidates_on_roots;
pub use disk::read_marker;
pub use handle::Roots;
pub use model::{
    AccountId, Config, DrillRoots, Layout, LibraryId, Marker, Mode, State, CONFIG_FILE,
    MARKER_FILE, SCHEMA_VERSION,
};
pub use setup::{setup_at, stage_roots};

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};

static ROOTS: OnceLock<RwLock<Option<Roots>>> = OnceLock::new();

fn global() -> &'static RwLock<Option<Roots>> {
    ROOTS.get_or_init(|| RwLock::new(None))
}

fn program_dir() -> Result<PathBuf, String> {
    std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "The executable has no parent directory".into())
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
    let roots = process_roots();
    let result = roots.layout();
    *global()
        .write()
        .map_err(|_| "Library root state could not be locked")? = Some(roots);
    result
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

pub fn setup_imported(label: &str) -> Result<Layout, String> {
    let program = program_dir()?;
    setup_at(&program, &program.join("library"), Mode::Imported, label)?;
    initialize()
}

pub fn setup_linked(path: &Path, label: &str) -> Result<Layout, String> {
    let program = program_dir()?;
    setup_at(&program, path, Mode::Linked, label)?;
    initialize()
}

#[cfg(test)]
mod tests;
