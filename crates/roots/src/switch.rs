//! The whole library moves where the operator chooses (ST3; ruled
//! 2026-10-31: "A Library cell in the status bar", "Restart, copy, then
//! open"). A tap only records the move in `move.toml`; the next launch runs
//! it before anything opens the library. The copy keeps the library's id,
//! checks every file byte for byte, rebases the locations and publishes the
//! marker last. The old copy is retired, never deleted (ADR-014 law 4): its
//! marker is renamed so nothing finds it again, and its folder is labelled.

use crate::copy::{destination_free, inside, same_bytes, undo, walk, MoveProgress};
use crate::{disk, handle, locations, setup, Config, Layout, Mode, MARKER_FILE, SCHEMA_VERSION};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

/// Where a requested move waits for the next launch, beside `roots.toml`.
pub const MOVE_FILE: &str = "move.toml";
/// What the old copy's marker becomes: no search, Locate… or setup reads it.
pub const RETIRED_MARKER: &str = "library.moved.json";

/// What a move will do, worked out without writing anything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchPlan {
    pub from: PathBuf,
    pub to: PathBuf,
    pub label: String,
    /// Imported when it lands in the program's own `library\`, else linked.
    pub mode: Mode,
    pub files: u64,
    pub bytes: u64,
}

/// A finished move: the library's new layout, where the old copy is now,
/// and anything the operator should know about it.
#[derive(Clone, Debug)]
pub struct Switched {
    pub layout: Layout,
    pub old: PathBuf,
    pub note: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Pending {
    v: u32,
    to: PathBuf,
}

/// What the copy leaves out: `common/temp` is regenerable (ADR-014), so the
/// new library gets an empty one, and the marker is published last.
fn copied_skips(library: &Path) -> [PathBuf; 2] {
    [
        library.join("common").join("temp"),
        library.join(MARKER_FILE),
    ]
}

/// The open library, where it goes, and how much it holds. Refuses a
/// library that isn't connected, a destination that holds files, and a
/// destination inside the library (or the library inside it).
pub(crate) fn plan(program: &Path, to: &Path) -> Result<SwitchPlan, String> {
    if !to.is_absolute() {
        return Err("Use a full folder path, starting with the drive".into());
    }
    let config = disk::read_config(program)?;
    let from = config.library_path(program)?;
    let marker = disk::read_marker(&from)
        .map_err(|e| format!("Your library isn't connected, so it can't be moved ({e})"))?;
    if marker.id != config.id {
        return Err(format!(
            "{} holds a different library, so nothing was moved",
            from.display()
        ));
    }
    let to = disk::absolute(to, program)?;
    disk::no_symlinks(&to)?;
    setup::reject_config_location(program, &to)?;
    if inside(&to, &from) {
        return Err(format!(
            "Your library can't move inside itself. Choose a folder outside {}.",
            from.display()
        ));
    }
    if inside(&from, &to) {
        return Err(format!(
            "{} holds your library. Choose a separate, empty folder.",
            to.display()
        ));
    }
    destination_free(&to)?;
    let (mut files, mut bytes) = (0, 0);
    walk(&from, &copied_skips(&from), &mut |_, meta| {
        if meta.is_file() {
            files += 1;
            bytes += meta.len();
        }
        Ok(())
    })?;
    let mode = setup::mode_for(program, &to);
    Ok(SwitchPlan {
        from,
        to,
        label: marker.label,
        mode,
        files,
        bytes,
    })
}

/// Copy the library to `to`, switch `roots.toml` to it and retire the old
/// copy. On any failure what the copy made is removed and nothing else
/// changes.
pub(crate) fn switch(
    program: &Path,
    to: &Path,
    progress: &MoveProgress,
) -> Result<Switched, String> {
    // Planned again here: the library may have changed since the plan shown.
    let plan = plan(program, to)?;
    progress.planned(plan.files, plan.bytes);
    let config = disk::read_config(program)?;
    let made_folder = !plan.to.exists();
    match copy_and_switch(program, &plan, &config, progress) {
        Ok(layout) => Ok(retire(&plan, layout)),
        Err(error) => Err(match undo(&plan.to, made_folder) {
            Ok(()) => format!("{error}. Nothing was changed; your library is where it was."),
            Err(undo) => format!(
                "{error}. The partial copy at {} couldn't be removed ({undo}); your library is where it was.",
                plan.to.display()
            ),
        }),
    }
}

fn copy_and_switch(
    program: &Path,
    plan: &SwitchPlan,
    config: &Config,
    progress: &MoveProgress,
) -> Result<Layout, String> {
    let marker = disk::read_marker(&plan.from)?;
    let skip = copied_skips(&plan.from);
    let landing = |path: &Path| -> Result<PathBuf, String> {
        path.strip_prefix(&plan.from)
            .map(|rel| plan.to.join(rel))
            .map_err(|e| format!("{}: {e}", path.display()))
    };
    fs::create_dir_all(&plan.to).map_err(|e| format!("{}: {e}", plan.to.display()))?;
    walk(&plan.from, &skip, &mut |path, meta| {
        progress.go_on()?;
        let to = landing(path)?;
        if meta.is_dir() {
            return fs::create_dir_all(&to).map_err(|e| format!("{}: {e}", to.display()));
        }
        let bytes = fs::copy(path, &to).map_err(|e| format!("Copying {}: {e}", path.display()))?;
        progress.copied_files.fetch_add(1, Ordering::Relaxed);
        progress.copied_bytes.fetch_add(bytes, Ordering::Relaxed);
        Ok(())
    })?;
    let mut checked = 0u64;
    walk(&plan.from, &skip, &mut |path, meta| {
        progress.go_on()?;
        if meta.is_file() {
            same_bytes(path, &landing(path)?)?;
            checked += 1;
            progress.checked_files.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    })?;
    let mut landed = 0u64;
    walk(&plan.to, &[], &mut |_, meta| {
        landed += u64::from(meta.is_file());
        Ok(())
    })?;
    if landed != checked {
        return Err(format!(
            "{} changed while it was copied",
            plan.from.display()
        ));
    }
    let accounts = plan.to.join("accounts");
    if accounts.is_dir() {
        let homes = fs::read_dir(&accounts).map_err(|e| format!("{}: {e}", accounts.display()))?;
        for home in homes {
            let home = home.map_err(|e| format!("{}: {e}", accounts.display()))?;
            locations::rebase(&home.path().join(".userdata"), &plan.from, &plan.to)?;
        }
    }
    let layout = Layout::for_library(&plan.to, &config.account);
    setup::create_layout(&layout)?;
    disk::create_json(&plan.to.join(MARKER_FILE), &marker)?;
    handle::validate_layout(&layout, &config.account)?;
    disk::replace_config(
        program,
        &Config {
            v: SCHEMA_VERSION,
            mode: plan.mode.clone(),
            id: config.id.clone(),
            label: config.label.clone(),
            account: config.account.clone(),
            last_path: (plan.mode == Mode::Linked).then(|| plan.to.clone()),
        },
    )?;
    Ok(layout)
}

/// The old copy's marker becomes `library.moved.json`, then its folder is
/// labelled. Neither failing undoes the move: the new library is already
/// the one in use, so the operator is told where the old copy is instead.
fn retire(plan: &SwitchPlan, layout: Layout) -> Switched {
    if let Err(error) = fs::rename(plan.from.join(MARKER_FILE), plan.from.join(RETIRED_MARKER)) {
        return Switched {
            layout,
            old: plan.from.clone(),
            note: Some(format!(
                "Your old copy at {} still has its {MARKER_FILE} ({error}). Rename it to {RETIRED_MARKER} so it's never opened by mistake.",
                plan.from.display()
            )),
        };
    }
    let Some(labelled) = labelled(plan) else {
        return Switched {
            layout,
            old: plan.from.clone(),
            note: Some(format!(
                "Your old copy stays at {}. It is no longer used.",
                plan.from.display()
            )),
        };
    };
    match fs::rename(&plan.from, &labelled) {
        Ok(()) => Switched {
            layout,
            old: labelled,
            note: None,
        },
        Err(error) => Switched {
            layout,
            old: plan.from.clone(),
            note: Some(format!(
                "Your old copy stays at {} ({error}). It is no longer used.",
                plan.from.display()
            )),
        },
    }
}

/// `<folder>.moved-to-program`, or `.moved-to-<new folder's name>`, beside
/// the old copy; a number is added when that name is taken.
fn labelled(plan: &SwitchPlan) -> Option<PathBuf> {
    let parent = plan.from.parent()?;
    let name = plan.from.file_name()?.to_string_lossy().into_owned();
    let place = match plan.mode {
        Mode::Imported => "program".to_owned(),
        Mode::Linked => plan
            .to
            .file_name()
            .map(|name| {
                name.to_string_lossy()
                    .chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '-' || c == '_' {
                            c
                        } else {
                            '-'
                        }
                    })
                    .collect()
            })
            .unwrap_or_else(|| "drive".to_owned()),
    };
    let base = format!("{name}.moved-to-{place}");
    (1..100)
        .map(|n| {
            if n == 1 {
                parent.join(&base)
            } else {
                parent.join(format!("{base}-{n}"))
            }
        })
        .find(|path| fs::symlink_metadata(path).is_err())
}

/// Record a move for the next launch, after planning it. A request made
/// earlier is replaced; nothing in the library is touched.
pub(crate) fn request(program: &Path, to: &Path) -> Result<SwitchPlan, String> {
    let plan = plan(program, to)?;
    let raw = toml::to_string_pretty(&Pending {
        v: SCHEMA_VERSION,
        to: plan.to.clone(),
    })
    .map_err(|e| e.to_string())?;
    let file = program.join(MOVE_FILE);
    let staged = program.join(format!("{MOVE_FILE}.new"));
    disk::no_symlinks(&file)?;
    fs::write(&staged, raw).map_err(|e| format!("{}: {e}", staged.display()))?;
    fs::rename(&staged, &file).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(plan)
}

/// Where a waiting move goes, if one is waiting.
pub(crate) fn pending(program: &Path) -> Result<Option<PathBuf>, String> {
    let file = program.join(MOVE_FILE);
    match fs::symlink_metadata(&file) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", file.display())),
        Ok(_) => {}
    }
    let pending: Pending =
        toml::from_str(&disk::read_text(&file)?).map_err(|e| format!("{}: {e}", file.display()))?;
    crate::model::version(pending.v, MOVE_FILE)?;
    Ok(Some(pending.to))
}

/// Forget a waiting move.
pub(crate) fn cancel(program: &Path) -> Result<(), String> {
    let file = program.join(MOVE_FILE);
    match fs::remove_file(&file) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("{}: {e}", file.display()))
        }
        _ => Ok(()),
    }
}

/// Run the waiting move. It is forgotten whatever happens, so a failure
/// never repeats at every launch.
pub(crate) fn run_pending(program: &Path, progress: &MoveProgress) -> Result<Switched, String> {
    let to = match pending(program) {
        Ok(Some(to)) => to,
        Ok(None) => return Err("No move is waiting".into()),
        Err(error) => {
            let _ = cancel(program);
            return Err(error);
        }
    };
    let result = switch(program, &to, progress);
    match (result, cancel(program)) {
        (Ok(switched), Err(error)) => Ok(Switched {
            note: Some(
                switched
                    .note
                    .map_or(error.clone(), |note| format!("{note} {error}")),
            ),
            ..switched
        }),
        (result, _) => result,
    }
}
