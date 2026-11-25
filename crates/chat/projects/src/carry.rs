//! Chats an older build kept inside the project folder, copied once into the
//! library (R303, ruled 2026-11-14). The project folder's copy is only read
//! and is left exactly as it was.

use crate::{disk, CHATS, MARKER};
use std::fs;
use std::path::Path;

/// The journal's folder name before it moved into the library.
const OLD_JOURNAL: &str = "state";

/// When `archive` is absent and `link` still holds an older build's journal,
/// copy everything but the marker and the writer lock into `archive`, checked
/// byte for byte. A copy that fails removes what it made.
pub(crate) fn from_link(link: &Path, store: &Path, archive: &Path) -> Result<(), String> {
    if fs::symlink_metadata(archive).is_ok() || !link.join(OLD_JOURNAL).is_dir() {
        return Ok(());
    }
    disk::create_directories(store)?;
    let name = archive
        .file_name()
        .ok_or("The project archive has no name")?
        .to_string_lossy();
    let staging = store.join(format!("{name}.carrying"));
    if fs::symlink_metadata(&staging).is_ok() {
        fs::remove_dir_all(&staging).map_err(|error| format!("{}: {error}", staging.display()))?;
    }
    let copied = copy_body(link, &staging).and_then(|()| {
        fs::rename(&staging, archive).map_err(|error| format!("{}: {error}", archive.display()))
    });
    if let Err(error) = copied {
        let _ = fs::remove_dir_all(&staging);
        return Err(format!(
            "This project's chats could not be copied into the library: {error}. Nothing was changed in {}",
            link.display()
        ));
    }
    Ok(())
}

fn copy_body(link: &Path, staging: &Path) -> Result<(), String> {
    fs::create_dir(staging).map_err(|error| format!("{}: {error}", staging.display()))?;
    for entry in entries(link)? {
        let name = entry.file_name();
        if name == MARKER || name == "writer.lock" {
            continue;
        }
        let target = if name == OLD_JOURNAL {
            staging.join(CHATS)
        } else {
            staging.join(&name)
        };
        copy(&entry.path(), &target)?;
    }
    Ok(())
}

fn entries(dir: &Path) -> Result<Vec<fs::DirEntry>, String> {
    fs::read_dir(dir)
        .and_then(Iterator::collect::<Result<Vec<_>, _>>)
        .map_err(|error| format!("{}: {error}", dir.display()))
}

fn copy(from: &Path, to: &Path) -> Result<(), String> {
    let meta =
        fs::symlink_metadata(from).map_err(|error| format!("{}: {error}", from.display()))?;
    if disk::is_link(&meta) {
        return Err(format!(
            "{} is a link; links are not copied",
            from.display()
        ));
    }
    if meta.is_dir() {
        fs::create_dir(to).map_err(|error| format!("{}: {error}", to.display()))?;
        for entry in entries(from)? {
            copy(&entry.path(), &to.join(entry.file_name()))?;
        }
        return Ok(());
    }
    let bytes = fs::read(from).map_err(|error| format!("{}: {error}", from.display()))?;
    disk::create(to, &bytes)?;
    let written = fs::read(to).map_err(|error| format!("{}: {error}", to.display()))?;
    if written != bytes {
        return Err(format!("{} didn't copy exactly", from.display()));
    }
    Ok(())
}
