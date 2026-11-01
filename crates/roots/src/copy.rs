//! The copy both moves share (ST1b's bring-in, ST3's switch): walk a tree
//! without following a link, check a copy byte for byte, and remove what a
//! failed copy made. The source is only ever read.

use crate::disk;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;

/// Counters a screen reads while a move runs on another thread.
#[derive(Debug, Default)]
pub struct MoveProgress {
    pub copied_files: AtomicU64,
    pub copied_bytes: AtomicU64,
    pub checked_files: AtomicU64,
}

/// Every folder and file under `dir`, folders before what they hold, except
/// what `skip` names and everything under it. A link or anything that is
/// neither stops the walk: a move never follows one.
pub(crate) fn walk(
    dir: &Path,
    skip: &[PathBuf],
    visit: &mut dyn FnMut(&Path, &fs::Metadata) -> Result<(), String>,
) -> Result<(), String> {
    let mut entries = fs::read_dir(dir)
        .and_then(|entries| entries.collect::<Result<Vec<_>, _>>())
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        if skip.contains(&path) {
            continue;
        }
        let meta = fs::symlink_metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if disk::is_link(&meta) {
            return Err(format!(
                "{} is a link. Links aren't copied: move what it points to into the folder, then try again",
                path.display()
            ));
        }
        if meta.is_dir() {
            visit(&path, &meta)?;
            walk(&path, skip, visit)?;
        } else if meta.is_file() {
            visit(&path, &meta)?;
        } else {
            return Err(format!("{} is neither a file nor a folder", path.display()));
        }
    }
    Ok(())
}

pub(crate) fn same_bytes(source: &Path, copy: &Path) -> Result<(), String> {
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

pub(crate) fn destination_free(library: &Path) -> Result<(), String> {
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
pub(crate) fn inside(path: &Path, folder: &Path) -> bool {
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
pub(crate) fn undo(library: &Path, made_folder: bool) -> std::io::Result<()> {
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
