//! Today's folders move into a library once (ST1b, R291). The plan only
//! reads. The move copies, checks every file byte for byte, publishes the
//! library's identity last, and only ever reads the source.

use crate::copy::{copy_file, destination_free, inside, same_bytes, undo, walk, MoveProgress};
use crate::{disk, handle, setup, AccountId, Layout, Publish};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

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
        walk(&from, &[], &mut |_, meta| {
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
    publish: Publish,
) -> Result<Layout, String> {
    setup::config_available(program, publish)?;
    crate::model::label(label)?;
    // Planned again here: the folders may have changed since the plan shown.
    let plan = plan_move(source, library)?;
    progress.planned(plan.files(), plan.bytes());
    setup::reject_config_location(program, &plan.library)?;
    let made_folder = !plan.library.exists();
    match copy_and_publish(program, &plan, label, progress, publish) {
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
    publish: Publish,
) -> Result<Layout, String> {
    let layout = Layout::for_library(&plan.library, &AccountId::default());
    setup::create_layout(&layout)?;
    for item in &plan.items {
        walk(&item.from, &[], &mut |path, meta| {
            let to = landing(item, path)?;
            if meta.is_dir() {
                return fs::create_dir_all(&to).map_err(|e| format!("{}: {e}", to.display()));
            }
            copy_file(path, &to, progress)?;
            progress.copied_files.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })?;
    }
    for item in &plan.items {
        let mut checked = 0u64;
        walk(&item.from, &[], &mut |path, meta| {
            if meta.is_file() {
                same_bytes(path, &landing(item, path)?, progress)?;
                checked += 1;
                progress.checked_files.fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        })?;
        let mut landed = 0u64;
        walk(&item.to, &[], &mut |_, meta| {
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
    setup::write_config(program, &plan.library, mode, marker, publish)?;
    Ok(layout)
}

fn landing(item: &MoveItem, path: &Path) -> Result<PathBuf, String> {
    path.strip_prefix(&item.from)
        .map(|rel| item.to.join(rel))
        .map_err(|e| format!("{}: {e}", path.display()))
}
