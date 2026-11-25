//! AI data's one-time carry (ruled 2026-11-14, R303): what AI made or learned
//! lives in the library's `.usr/.ai`, not in `.userdata`.
//!
//! When a library opens and a carried folder is in `.userdata` but not yet in
//! `.usr/.ai`, it is copied across and checked byte for byte. The source is
//! only ever read and is left exactly as it was. A copy that fails removes
//! what it made, so the next open tries again from the start.

use crate::copy::{self, MoveProgress};
use crate::disk;
use crate::model::Layout;
use std::fs;
use std::path::Path;

/// The folders of `.userdata` that hold AI data: provider accounts and
/// profiles, and the operator's roles and rooms.
const CARRIED: [&str; 1] = ["chat"];

fn present(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

pub(crate) fn carry(layout: &Layout) -> Result<(), String> {
    for name in CARRIED {
        let from = layout.userdata.join(name);
        let to = layout.ai.join(name);
        if present(&to) || !from.is_dir() {
            continue;
        }
        let staging = layout.ai.join(format!("{name}.carrying"));
        if present(&staging) {
            fs::remove_dir_all(&staging).map_err(|e| format!("{}: {e}", staging.display()))?;
        }
        let copied = copy_tree(&from, &staging)
            .and_then(|()| fs::rename(&staging, &to).map_err(|e| format!("{}: {e}", to.display())));
        if let Err(error) = copied {
            let _ = fs::remove_dir_all(&staging);
            return Err(format!(
                "Your AI data could not be copied into {}: {error}. Nothing was changed in {}",
                layout.ai.display(),
                from.display()
            ));
        }
    }
    Ok(())
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    disk::no_symlinks(to)?;
    fs::create_dir(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let progress = MoveProgress::default();
    copy::walk(from, &[], &mut |path, meta| {
        let rel = path
            .strip_prefix(from)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let target = to.join(rel);
        if meta.is_dir() {
            fs::create_dir(&target).map_err(|e| format!("{}: {e}", target.display()))
        } else {
            copy::copy_file(path, &target, &progress)?;
            copy::same_bytes(path, &target, &progress)
        }
    })
}
