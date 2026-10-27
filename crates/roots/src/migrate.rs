//! Today's folders move into a library once (ST1b, R291). The plan only
//! reads. The move copies, checks every file byte for byte, publishes the
//! library's identity last, and only ever reads the source.

use crate::{disk, handle, setup, AccountId, Layout};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// The rings of the old layout, in the order they move.
pub const LEGACY_RINGS: [&str; 3] = ["content", "assets", ".userdata"];

/// One ring: where it is, where it lands, and how much it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveItem {
    pub ring: &'static str,
    pub from: PathBuf,
    pub to: PathBuf,
    pub files: u64,
    pub bytes: u64,
}

/// What a move will do, worked out without writing anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MovePlan {
    pub source: PathBuf,
    pub library: PathBuf,
    pub items: Vec<MoveItem>,
}

impl MovePlan {
    pub fn files(&self) -> u64 {
        self.items.iter().map(|item| item.files).sum()
    }

    pub fn bytes(&self) -> u64 {
        self.items.iter().map(|item| item.bytes).sum()
    }
}

/// Counters a screen reads while a move runs on another thread.
#[derive(Debug, Default)]
pub struct MoveProgress {
    pub copied_files: AtomicU64,
    pub copied_bytes: AtomicU64,
    pub checked_files: AtomicU64,
}

/// True when `folder` itself holds an old-layout ring. Nothing else is read.
pub fn has_legacy(folder: &Path) -> bool {
    LEGACY_RINGS.iter().any(|ring| folder.join(ring).is_dir())
}

/// Each ring under `source`, its size, and where it lands in `library`.
/// Refuses a link anywhere, a library folder that already holds files, and
/// a library inside a ring (or a ring inside the library).
pub fn plan_move(source: &Path, library: &Path) -> Result<MovePlan, String> {
    if !source.is_absolute() || !library.is_absolute() {
        return Err("Use full folder paths, starting with the drive".into());
    }
    let source = disk::absolute(source, source)?;
    let library = disk::absolute(library, &source)?;
    disk::no_symlinks(&source)?;
    let layout = Layout::for_library(&library, &AccountId::default());
    let mut items = Vec::new();
    for ring in LEGACY_RINGS {
        let from = source.join(ring);
        let meta = match fs::symlink_metadata(&from) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{}: {e}", from.display())),
            Ok(meta) => meta,
        };
        if disk::is_link(&meta) || !meta.is_dir() {
            return Err(format!(
                "{} must be a real folder, not a link or a file",
                from.display()
            ));
        }
        let to = match ring {
            "content" => layout.content.clone(),
            "assets" => layout.assets.clone(),
            _ => layout.userdata.clone(),
        };
        let (mut files, mut bytes) = (0, 0);
        walk(&from, &mut |_, meta| {
            if meta.is_file() {
                files += 1;
                bytes += meta.len();
            }
            Ok(())
        })?;
        items.push(MoveItem {
            ring,
            from,
            to,
            files,
            bytes,
        });
    }
    if items.is_empty() {
        return Err(format!(
            "{} has no content, assets or .userdata folder to bring in",
            source.display()
        ));
    }
    for item in &items {
        if inside(&library, &item.from) {
            return Err(format!(
                "The library can't go inside {}. Choose a folder outside it.",
                item.from.display()
            ));
        }
        if inside(&item.from, &library) {
            return Err(format!(
                "{} is inside the folder you chose. Choose a separate, empty folder.",
                item.from.display()
            ));
        }
    }
    destination_free(&library)?;
    Ok(MovePlan {
        source,
        library,
        items,
    })
}

/// Copy today's folders into a new library and publish it. `program` must
/// have no `roots.toml` yet. On any failure what the move made is removed.
pub(crate) fn move_in(
    program: &Path,
    source: &Path,
    library: &Path,
    label: &str,
    progress: &MoveProgress,
) -> Result<Layout, String> {
    setup::config_available(program)?;
    crate::model::label(label)?;
    // Planned again here: the folders may have changed since the plan shown.
    let plan = plan_move(source, library)?;
    setup::reject_config_location(program, &plan.library)?;
    let made_folder = !plan.library.exists();
    match copy_and_publish(program, &plan, label, progress) {
        Ok(layout) => Ok(layout),
        Err(error) => Err(match undo(&plan.library, made_folder) {
            Ok(()) => format!("{error}. Nothing was changed; your folders are as they were."),
            Err(undo) => format!(
                "{error}. The partial copy at {} couldn't be removed ({undo}); your folders are as they were.",
                plan.library.display()
            ),
        }),
    }
}

fn copy_and_publish(
    program: &Path,
    plan: &MovePlan,
    label: &str,
    progress: &MoveProgress,
) -> Result<Layout, String> {
    let layout = Layout::for_library(&plan.library, &AccountId::default());
    setup::create_layout(&layout)?;
    for item in &plan.items {
        walk(&item.from, &mut |path, meta| {
            let to = landing(item, path)?;
            if meta.is_dir() {
                return fs::create_dir_all(&to).map_err(|e| format!("{}: {e}", to.display()));
            }
            let bytes =
                fs::copy(path, &to).map_err(|e| format!("Copying {}: {e}", path.display()))?;
            progress.copied_files.fetch_add(1, Ordering::Relaxed);
            progress.copied_bytes.fetch_add(bytes, Ordering::Relaxed);
            Ok(())
        })?;
    }
    for item in &plan.items {
        let mut checked = 0u64;
        walk(&item.from, &mut |path, meta| {
            if meta.is_file() {
                same_bytes(path, &landing(item, path)?)?;
                checked += 1;
                progress.checked_files.fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        })?;
        let mut landed = 0u64;
        walk(&item.to, &mut |_, meta| {
            landed += u64::from(meta.is_file());
            Ok(())
        })?;
        if landed != checked {
            return Err(format!(
                "{} changed while it was copied",
                item.from.display()
            ));
        }
    }
    let marker = setup::publish_identity(&plan.library, label)?;
    handle::validate_layout(&layout, &AccountId::default())?;
    let mode = setup::mode_for(program, &plan.library);
    setup::write_config(program, &plan.library, mode, marker)?;
    Ok(layout)
}

fn landing(item: &MoveItem, path: &Path) -> Result<PathBuf, String> {
    path.strip_prefix(&item.from)
        .map(|rel| item.to.join(rel))
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Every folder and file under `dir`, folders before what they hold. A link
/// or anything that is neither stops the walk: the move never follows one.
fn walk(
    dir: &Path,
    visit: &mut dyn FnMut(&Path, &fs::Metadata) -> Result<(), String>,
) -> Result<(), String> {
    let mut entries = fs::read_dir(dir)
        .and_then(|entries| entries.collect::<Result<Vec<_>, _>>())
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if disk::is_link(&meta) {
            return Err(format!(
                "{} is a link. Links aren't copied: move what it points to into the folder, then try again",
                path.display()
            ));
        }
        if meta.is_dir() {
            visit(&path, &meta)?;
            walk(&path, visit)?;
        } else if meta.is_file() {
            visit(&path, &meta)?;
        } else {
            return Err(format!("{} is neither a file nor a folder", path.display()));
        }
    }
    Ok(())
}

fn same_bytes(source: &Path, copy: &Path) -> Result<(), String> {
    let open = |path: &Path| fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()));
    let (mut left, mut right) = (open(source)?, open(copy)?);
    let mut one = vec![0u8; 1 << 16];
    let mut two = vec![0u8; 1 << 16];
    loop {
        let n = fill(&mut left, &mut one).map_err(|e| format!("{}: {e}", source.display()))?;
        let m = fill(&mut right, &mut two).map_err(|e| format!("{}: {e}", copy.display()))?;
        if n != m || one[..n] != two[..m] {
            return Err(format!("{} didn't copy exactly", source.display()));
        }
        if n == 0 {
            return Ok(());
        }
    }
}

/// Read until `buf` is full or the file ends.
fn fill(file: &mut fs::File, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match file.read(&mut buf[filled..])? {
            0 => break,
            n => filled += n,
        }
    }
    Ok(filled)
}

fn destination_free(library: &Path) -> Result<(), String> {
    disk::directory_path(library)?;
    let occupied = match fs::read_dir(library) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(format!("{}: {e}", library.display())),
        Ok(mut entries) => entries.next().is_some(),
    };
    if occupied {
        return Err(format!(
            "{} already holds files. Choose an empty folder, or a new one.",
            library.display()
        ));
    }
    Ok(())
}

/// `path` is `folder` or lies under it. Windows paths compare without case.
fn inside(path: &Path, folder: &Path) -> bool {
    let parts = |p: &Path| -> Vec<String> {
        p.components()
            .map(|c| {
                let part = c.as_os_str().to_string_lossy();
                if cfg!(windows) {
                    part.to_ascii_lowercase()
                } else {
                    part.into_owned()
                }
            })
            .collect()
    };
    let (path, folder) = (parts(path), parts(folder));
    path.len() >= folder.len() && path[..folder.len()] == folder[..]
}

/// Remove what a failed move made: the whole library folder if the move
/// created it, otherwise only what it put in the folder (empty beforehand).
fn undo(library: &Path, made_folder: bool) -> std::io::Result<()> {
    if made_folder {
        return match fs::remove_dir_all(library) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            other => other,
        };
    }
    for entry in fs::read_dir(library)? {
        let path = entry?.path();
        if fs::symlink_metadata(&path)?.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}
