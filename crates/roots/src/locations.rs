//! Locations (ST5; R83, R288): folders that join the library in place — a
//! drive of videos, a share, a pack folder. The library keeps a record of
//! each one and never writes anything inside it (ruled 2026-10-27, Q-ST5-1).
//! A linked file is named `locations/<id>/<path inside>`, which resolves to
//! wherever the folder is now: slides keep working when the drive comes back
//! on another letter, and an export can copy the file in under the same name.

use crate::{disk, guard, migrate};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError, RwLock};

/// The first part of every linked file's name.
pub const PREFIX: &str = "locations";
const FILE: &str = "locations.json";
const VERSION: u32 = 1;
/// How many of a folder's files are noted to recognise it later.
const SAMPLE: usize = 32;
/// How deep the sample looks: the folder and two levels below it.
const SAMPLE_DEPTH: usize = 3;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    pub id: String,
    pub label: String,
    /// Where it was last seen: relative to the library when it is on the
    /// library's own drive (so it follows that drive's letter), else absolute.
    pub path: PathBuf,
    /// Some of its files and their sizes when it was added or located, to
    /// recognise it on another drive letter without writing anything into it.
    pub sample: Vec<(String, u64)>,
}

#[derive(Serialize, Deserialize)]
struct Record {
    v: u32,
    locations: Vec<Location>,
}

/// A location and where it is now; `at` is `None` while it is offline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    pub location: Location,
    pub at: Option<PathBuf>,
}

/// One library's locations: the library folder (for relative paths) and the
/// account's `.userdata` (where the record lives).
pub(crate) struct Store<'a> {
    pub library: &'a Path,
    pub userdata: &'a Path,
}

/// The record as last read or written, so the per-frame `resolve` never
/// reads the file: keyed by the `.userdata` it came from.
static CACHE: RwLock<Option<(PathBuf, Vec<Location>)>> = RwLock::new(None);

/// Where each location was at the last look (ST5a′): lists and painters read
/// this, so no frame ever waits on a drive. Keyed by its `.userdata`.
static PLACED: RwLock<Option<(PathBuf, Vec<Placed>)>> = RwLock::new(None);

/// Counts every look that found a location gone, back, or somewhere new.
static CHANGES: AtomicU64 = AtomicU64::new(0);

/// One edit of the record at a time: the watch's thread and a tap can both
/// change it.
static EDIT: Mutex<()> = Mutex::new(());

pub fn changes() -> u64 {
    CHANGES.load(Ordering::Acquire)
}

impl Store<'_> {
    fn file(&self) -> PathBuf {
        self.userdata.join(FILE)
    }

    pub fn load(&self) -> Result<Vec<Location>, String> {
        if let Some((from, locations)) = CACHE.read().ok().as_deref().and_then(Option::as_ref) {
            if from == self.userdata {
                return Ok(locations.clone());
            }
        }
        let file = self.file();
        let locations = match std::fs::read_to_string(&file) {
            Ok(raw) => {
                let record: Record =
                    serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", file.display()))?;
                crate::model::version(record.v, FILE)?;
                record.locations
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(format!("{}: {e}", file.display())),
        };
        self.remember(&locations);
        Ok(locations)
    }

    fn remember(&self, locations: &[Location]) {
        if let Ok(mut cache) = CACHE.write() {
            *cache = Some((self.userdata.to_path_buf(), locations.to_vec()));
        }
    }

    /// Write the record whole (a `.tmp`, then a rename), asking the library
    /// first (ST2b).
    fn save(&self, locations: &[Location]) -> Result<(), String> {
        let userdata = guard::writable(self.userdata)?;
        std::fs::create_dir_all(&userdata).map_err(|e| format!("{}: {e}", userdata.display()))?;
        let file = userdata.join(FILE);
        let staged = userdata.join(format!("{FILE}.tmp"));
        let mut json = serde_json::to_string_pretty(&Record {
            v: VERSION,
            locations: locations.to_vec(),
        })
        .map_err(|e| e.to_string())?;
        json.push('\n');
        std::fs::write(&staged, json).map_err(|e| format!("{}: {e}", staged.display()))?;
        std::fs::rename(&staged, &file).map_err(|e| format!("{}: {e}", file.display()))?;
        self.remember(locations);
        self.publish(self.place(locations.to_vec()));
        Ok(())
    }

    /// Where a location's record points, as a full path.
    pub fn full(&self, location: &Location) -> PathBuf {
        disk::absolute(&location.path, self.library).unwrap_or_else(|_| location.path.clone())
    }

    /// How a folder is recorded: relative to the library when they share a
    /// drive, so a drive that comes back on another letter carries both.
    fn stored(&self, folder: &Path) -> PathBuf {
        relative(folder, self.library).unwrap_or_else(|| folder.to_path_buf())
    }

    /// Join `folder` as a location. Nothing inside it is written.
    pub fn add(&self, folder: &Path, label: &str) -> Result<Location, String> {
        if !folder.is_absolute() {
            return Err("Paste the full path of the folder.".into());
        }
        let folder = disk::absolute(folder, folder)?;
        disk::no_symlinks(&folder)?;
        if !folder.is_dir() {
            return Err(format!(
                "{} isn't a folder that can be opened.",
                folder.display()
            ));
        }
        if migrate::inside(&folder, self.library) {
            return Err("That folder is already inside your library.".into());
        }
        if migrate::inside(self.library, &folder) {
            return Err("Your library is inside that folder; choose a folder beside it.".into());
        }
        let _edit = EDIT.lock().unwrap_or_else(PoisonError::into_inner);
        let mut locations = self.load()?;
        if let Some(known) = locations.iter().find(|l| self.full(l) == folder) {
            return Err(format!(
                "That folder is already the location '{}'.",
                known.label
            ));
        }
        let label = match label.trim() {
            "" => folder
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| folder.display().to_string()),
            label => label.to_owned(),
        };
        crate::model::label(&label)?;
        let location = Location {
            id: new_id(),
            label,
            path: self.stored(&folder),
            sample: sample(&folder),
        };
        locations.push(location.clone());
        self.save(&locations)?;
        Ok(location)
    }

    /// Every location, online where its folder is, offline where it isn't:
    /// looked at now, and kept as the last look.
    pub fn list(&self) -> Result<Vec<Placed>, String> {
        let placed = self.place(self.load()?);
        self.publish(placed.clone());
        Ok(placed)
    }

    fn place(&self, locations: Vec<Location>) -> Vec<Placed> {
        locations
            .into_iter()
            .map(|location| {
                let full = self.full(&location);
                let at = full.is_dir().then_some(full);
                Placed { location, at }
            })
            .collect()
    }

    /// Keep `placed` as the last look, counting it when anything moved.
    fn publish(&self, placed: Vec<Placed>) {
        let mut last = PLACED.write().unwrap_or_else(PoisonError::into_inner);
        let same = last
            .as_ref()
            .is_some_and(|(from, was)| from == self.userdata && *was == placed);
        if !same {
            *last = Some((self.userdata.to_path_buf(), placed));
            CHANGES.fetch_add(1, Ordering::AcqRel);
        }
    }

    /// `pick` from the last look, without touching a drive; the first ask
    /// looks.
    fn with_placed<R>(&self, pick: impl FnOnce(&[Placed]) -> R) -> Result<R, String> {
        if let Some((from, placed)) = PLACED.read().ok().as_deref().and_then(Option::as_ref) {
            if from == self.userdata {
                return Ok(pick(placed));
            }
        }
        Ok(pick(&self.list()?))
    }

    /// Every location as at the last look.
    pub fn placed(&self) -> Result<Vec<Placed>, String> {
        self.with_placed(<[Placed]>::to_vec)
    }

    /// Look for each offline location in `elsewhere(its last path)` — the
    /// same place on other drives — and keep the first that holds the files
    /// it is known by. Returns how many came back.
    pub fn refresh_with(&self, elsewhere: impl Fn(&Path) -> Vec<PathBuf>) -> Result<usize, String> {
        // Look without holding the record: a drive can be slow to answer.
        let moved: Vec<(String, PathBuf)> = self
            .load()?
            .iter()
            .filter(|location| !self.full(location).is_dir())
            .filter_map(|location| {
                elsewhere(&self.full(location))
                    .into_iter()
                    .find(|candidate| candidate.is_dir() && matches(candidate, &location.sample))
                    .map(|place| (location.id.clone(), place))
            })
            .collect();
        if moved.is_empty() {
            return Ok(0);
        }
        let _edit = EDIT.lock().unwrap_or_else(PoisonError::into_inner);
        let mut locations = self.load()?;
        let mut found = 0;
        for (id, place) in moved {
            if let Some(location) = locations.iter_mut().find(|l| l.id == id) {
                location.path = self.stored(&place);
                found += 1;
            }
        }
        if found > 0 {
            self.save(&locations)?;
        }
        Ok(found)
    }

    /// Point a location at where it is now. A folder that doesn't hold the
    /// files it is known by is accepted only when the person confirms
    /// (`Ok(false)` asks); then its files are noted again.
    pub fn locate(&self, id: &str, folder: &Path, confirmed: bool) -> Result<bool, String> {
        if !folder.is_absolute() || !folder.is_dir() {
            return Err("Paste the full path of the folder where it is now.".into());
        }
        let folder = disk::absolute(folder, folder)?;
        disk::no_symlinks(&folder)?;
        let _edit = EDIT.lock().unwrap_or_else(PoisonError::into_inner);
        let mut locations = self.load()?;
        let location = locations
            .iter_mut()
            .find(|l| l.id == id)
            .ok_or("That location is no longer in your library.")?;
        let known = matches(&folder, &location.sample);
        if !known && !confirmed {
            return Ok(false);
        }
        location.path = self.stored(&folder);
        if !known {
            location.sample = sample(&folder);
        }
        self.save(&locations)?;
        Ok(true)
    }

    /// Forget a location. Its folder and files are never touched.
    pub fn remove(&self, id: &str) -> Result<(), String> {
        let _edit = EDIT.lock().unwrap_or_else(PoisonError::into_inner);
        let mut locations = self.load()?;
        let before = locations.len();
        locations.retain(|l| l.id != id);
        if locations.len() == before {
            return Err("That location is no longer in your library.".into());
        }
        self.save(&locations)
    }

    /// `locations/<id>/<path inside>` to the file where its location was at
    /// the last look. An offline location's drive is never touched.
    pub fn resolve(&self, reference: &Path) -> Option<PathBuf> {
        let (id, inside) = split(reference)?;
        let folder = self
            .with_placed(|placed| placed.iter().find(|p| p.location.id == id)?.at.clone())
            .ok()??;
        let file = folder.join(inside);
        file.is_file().then_some(file)
    }

    /// The location a reference names, online or not, for the offline note.
    pub fn of(&self, reference: &Path) -> Option<Location> {
        let (id, _) = split(reference)?;
        self.with_placed(|placed| {
            placed
                .iter()
                .find(|p| p.location.id == id)
                .map(|p| p.location.clone())
        })
        .ok()?
    }
}

/// `locations/<id>/<path inside>` split into the id and the path inside.
/// Anything else — or a path that tries to climb out — is not a reference.
pub fn split(reference: &Path) -> Option<(String, PathBuf)> {
    let mut parts = reference.components();
    if parts.next()? != Component::Normal(PREFIX.as_ref()) {
        return None;
    }
    let Component::Normal(id) = parts.next()? else {
        return None;
    };
    let inside = parts.as_path();
    if inside.as_os_str().is_empty()
        || inside
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return None;
    }
    Some((id.to_string_lossy().into_owned(), inside.to_path_buf()))
}

/// The reference a slide stores for `file` inside a location's folder.
pub fn reference(location: &Location, inside: &Path) -> String {
    let inside = inside.to_string_lossy().replace('\\', "/");
    format!("{PREFIX}/{}/{inside}", location.id)
}

fn new_id() -> String {
    let one = RandomState::new().build_hasher().finish();
    let two = RandomState::new().build_hasher().finish();
    format!("loc-{one:016x}{two:016x}")
}

/// The first files of a folder (sorted, the folder and two levels below),
/// with their sizes.
fn sample(folder: &Path) -> Vec<(String, u64)> {
    fn visit(base: &Path, dir: &Path, depth: usize, out: &mut Vec<(String, u64)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.flatten().map(|entry| entry.path()).collect();
        entries.sort();
        for path in entries {
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.is_file() {
                let rel = path
                    .strip_prefix(base)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, meta.len()));
            } else if meta.is_dir() && !disk::is_link(&meta) && depth + 1 < SAMPLE_DEPTH {
                visit(base, &path, depth + 1, out);
            }
        }
    }
    let mut out = Vec::new();
    visit(folder, folder, 0, &mut out);
    out.truncate(SAMPLE);
    out
}

/// The folder still holds at least half of the files a location is known
/// by, at their sizes: new files may have been added and a few removed.
fn matches(folder: &Path, sample: &[(String, u64)]) -> bool {
    if sample.is_empty() {
        return folder.is_dir();
    }
    let present = sample
        .iter()
        .filter(|(rel, size)| {
            std::fs::metadata(folder.join(rel))
                .is_ok_and(|meta| meta.is_file() && meta.len() == *size)
        })
        .count();
    present * 2 >= sample.len()
}

/// `path` relative to `base` when both are on the same drive (or, outside
/// Windows, under the same mount), with `..` where it climbs.
fn relative(path: &Path, base: &Path) -> Option<PathBuf> {
    let (path_parts, base_parts): (Vec<_>, Vec<_>) =
        (path.components().collect(), base.components().collect());
    let same_root = |a: &[Component], b: &[Component]| match (a.first(), b.first()) {
        (Some(Component::Prefix(x)), Some(Component::Prefix(y))) => {
            x.as_os_str().to_string_lossy().to_ascii_lowercase()
                == y.as_os_str().to_string_lossy().to_ascii_lowercase()
        }
        (Some(x), Some(y)) => x == y,
        _ => false,
    };
    if !same_root(&path_parts, &base_parts) {
        return None;
    }
    #[cfg(not(windows))]
    {
        // Outside Windows a removable drive is a mount point: a path travels
        // with the library only when both sit under the same one, or
        // neither is under any.
        if crate::discovery::mount_root(path) != crate::discovery::mount_root(base) {
            return None;
        }
    }
    let common = path_parts
        .iter()
        .zip(&base_parts)
        .take_while(|(a, b)| a == b)
        .count();
    let mut out = PathBuf::new();
    for _ in common..base_parts.len() {
        out.push("..");
    }
    for part in &path_parts[common..] {
        out.push(part.as_os_str());
    }
    Some(out)
}
