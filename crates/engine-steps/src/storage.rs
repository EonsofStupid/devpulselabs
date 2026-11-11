//! Recorder-owned, append-only sources and retryable guide publication.
//! Static path checks do not defend against hostile concurrent path replacement.
//! An OS lease serializes source ownership across recorder instances. Lifecycle
//! callers serialize operations within that owner. Staging files are retained.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{CaptureMeta, Session, StepRec};

const SOURCE: &str = ".steps-source.json";
const LEASE: &str = ".steps-lock";
const ARCHIVED: &str = ".steps-archived.json";
const RECEIPT: &str = ".steps-save.json";
const PENDING: &str = ".guide-revision-pending";
const COMPLETE: &str = ".guide-revision-complete";
const PREFIX: &str = ".steps-recording-";
const MAX_JSON: u64 = 4 * 1024 * 1024;
const MAX_ENTRIES: usize = 50_000;
const MAX_STEPS: usize = 10_000;
static NONCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct RecoveryCandidate {
    pub dir: PathBuf,
    pub count: u32,
    pub seed_title: String,
    /// A prepared publication must finish with its original metadata.
    pub prepared_meta: Option<CaptureMeta>,
}

/// Source-level problems do not hide other recoverable recordings.
#[derive(Clone, Debug, Default)]
pub struct RecoveryScan {
    pub candidates: Vec<RecoveryCandidate>,
    pub issues: Vec<String>,
}

/// The actual published destination, including any library drive rebasing.
#[derive(Clone, Debug)]
pub struct SavedGuide {
    pub name: String,
    pub dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    version: u32,
    source_id: SourceId,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
struct SourceId(String);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Checkpoint {
    version: u32,
    source_id: SourceId,
    next_attempt: u64,
    steps: Vec<StepRec>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    version: u32,
    source_id: SourceId,
    target: String,
    count: u32,
    meta: CaptureMeta,
}

fn token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "{nanos}-{}-{}",
        std::process::id(),
        NONCE.fetch_add(1, Ordering::Relaxed)
    )
}

fn io_error(path: &Path, error: impl std::fmt::Display) -> String {
    format!("{}: {error}", path.display())
}

fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

fn ordinary_file(path: &Path) -> Result<fs::Metadata, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| io_error(path, e))?;
    if is_link(&meta) || !meta.is_file() {
        return Err(io_error(
            path,
            "recorder files must be regular files, never links or folders",
        ));
    }
    Ok(meta)
}

fn ancestry(path: &Path, allow_missing: bool) -> Result<(), String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let mut cursor = PathBuf::new();
    for part in absolute.components() {
        if matches!(part, Component::ParentDir) {
            return Err("recorder paths cannot contain '..'".into());
        }
        cursor.push(part.as_os_str());
        if matches!(part, Component::Prefix(_)) {
            continue;
        }
        match fs::symlink_metadata(&cursor) {
            Ok(meta) if meta.is_dir() && !is_link(&meta) => {}
            Ok(_) => {
                return Err(io_error(
                    &cursor,
                    "recorder ancestry must contain ordinary directories",
                ))
            }
            Err(e) if allow_missing && e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io_error(&cursor, e)),
        }
    }
    Ok(())
}

fn writable_dir(path: &Path, create: bool) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    let path = dpl_roots::writable(&absolute)?;
    ancestry(&path, create)?;
    if create {
        fs::create_dir_all(&path).map_err(|e| io_error(&path, e))?;
    }
    ancestry(&path, false)?;
    Ok(path)
}

// A reconnect between phases must restart the operation from rebased paths,
// never continue writing through an old drive letter.
fn before_write(path: &Path) -> Result<(), String> {
    if dpl_roots::writable(path)? != path {
        return Err(
            "the library moved during this operation; retry to use its new location".into(),
        );
    }
    ancestry(path.parent().ok_or("recorder write has no parent")?, false)
}

/// Check each capture phase immediately before entering the native backend.
pub(super) fn capture_target(path: &Path) -> Result<(), String> {
    before_write(path)?;
    let dir = path.parent().ok_or("capture target has no source")?;
    owner(dir)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("invalid capture filename")?;
    let attempt = numbered(name, "capture-", ".webp", 20)
        .or_else(|| numbered(name, "capture-", "-done.webp", 20))
        .filter(|attempt| *attempt > 0)
        .ok_or("capture target is not a reserved frame")?;
    ordinary_file(&dir.join(attempt_name(attempt)))?;
    if present(path)? {
        return Err("capture target already exists; source files are never overwritten".into());
    }
    Ok(())
}

/// The empty file is permanent; ownership is the OS lock, never file existence.
/// Existing-file discovery acquires only a transient lock and writes no bytes.
fn lease(dir: &Path, create: bool) -> Result<Option<Arc<File>>, String> {
    let path = dir.join(LEASE);
    if create {
        before_write(&path)?;
    } else {
        ordinary_file(&path)?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(create)
        .open(&path)
        .map_err(|e| io_error(&path, e))?;
    match file.try_lock() {
        Ok(()) => {
            if create {
                file.sync_all().map_err(|e| io_error(&path, e))?;
            }
            Ok(Some(Arc::new(file)))
        }
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(error) => Err(io_error(&path, error)),
    }
}

fn safe_name(name: &str) -> Result<(), String> {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end()
        .to_ascii_uppercase();
    let device = matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|tail| {
            matches!(
                tail,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    if name.is_empty()
        || name == "."
        || name.contains("..")
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
        || device
    {
        return Err(format!("unsafe recorder filename: {name:?}"));
    }
    Ok(())
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    if ordinary_file(path)?.len() > MAX_JSON {
        return Err(io_error(path, "metadata exceeds 4 MiB"));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| io_error(path, e))?
        .take(MAX_JSON + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| io_error(path, e))?;
    if bytes.len() as u64 > MAX_JSON {
        return Err(io_error(path, "metadata exceeds 4 MiB"));
    }
    Ok(bytes)
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    serde_json::from_slice(&read_bytes(path)?).map_err(|e| io_error(path, e))
}

fn present(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(io_error(path, e)),
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    before_write(path)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| io_error(path, e))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| io_error(path, e))
}

// The lifecycle gate is the single writer. Do not replace existing metadata,
// including a malformed file left by another writer or interrupted old build.
fn publish_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_JSON {
        return Err("recorder metadata exceeds 4 MiB".into());
    }
    if present(path)? {
        return if read_bytes(path)? == bytes {
            Ok(())
        } else {
            Err(io_error(
                path,
                "immutable metadata already exists with different content",
            ))
        };
    }
    let stage = path.with_file_name(format!(".steps-stage-{}.pending", token()));
    write_new(&stage, &bytes)?;
    before_write(path)?;
    if present(path)? {
        return Err(io_error(
            path,
            "metadata appeared during publication; source retained",
        ));
    }
    fs::rename(&stage, path).map_err(|e| io_error(path, e))
}

fn owner(dir: &Path) -> Result<Owner, String> {
    ancestry(dir, false)?;
    let name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("recorder source name is not UTF-8")?;
    if !name.starts_with(PREFIX) {
        return Err("folder is not a recorder-owned source".into());
    }
    safe_name(name)?;
    let owner: Owner = read_json(&dir.join(SOURCE))?;
    if owner.version != 1 || owner.source_id.0 != name {
        return Err("recorder source identity/version mismatch".into());
    }
    Ok(owner)
}

fn entries(dir: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| io_error(dir, e))? {
        let entry = entry.map_err(|e| io_error(dir, e))?;
        if entries.len() >= MAX_ENTRIES {
            return Err("recorder directory has too many entries".into());
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "recorder filename is not UTF-8")?;
        entries.push((name, entry.path()));
    }
    Ok(entries)
}

fn checkpoint_name(count: usize) -> String {
    format!("checkpoint-{count:010}.json")
}
fn attempt_name(attempt: u64) -> String {
    format!(".attempt-{attempt:020}")
}
fn frame_names(attempt: u64) -> (String, String) {
    (
        format!("capture-{attempt:020}.webp"),
        format!("capture-{attempt:020}-done.webp"),
    )
}

fn numbered(name: &str, prefix: &str, suffix: &str, digits: usize) -> Option<u64> {
    let number = name.strip_prefix(prefix)?.strip_suffix(suffix)?;
    (number.len() == digits && number.bytes().all(|b| b.is_ascii_digit()))
        .then(|| number.parse().ok())
        .flatten()
}

fn validate_steps(session: &Session) -> Result<(), String> {
    if session.steps.len() > MAX_STEPS {
        return Err("recorder checkpoint exceeds 10000 steps".into());
    }
    let mut used = HashSet::new();
    for (index, step) in session.steps.iter().enumerate() {
        if step.order as usize != index + 1
            || step.width == 0
            || step.height == 0
            || step.click.is_some_and(|xy| {
                xy.iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
            })
        {
            return Err("invalid recorder step order, dimensions, or click coordinates".into());
        }
        let attempt = numbered(&step.file, "capture-", ".webp", 20)
            .filter(|n| *n > 0)
            .ok_or("step file is not a reserved recorder frame")?;
        ordinary_file(&session.dir.join(attempt_name(attempt)))?;
        let (_, done) = frame_names(attempt);
        if step.result_file.as_ref().is_some_and(|name| name != &done) {
            return Err("result frame does not belong to its capture attempt".into());
        }
        let mut bytes = 0_u64;
        for name in std::iter::once(&step.file).chain(step.result_file.iter()) {
            safe_name(name)?;
            if !used.insert(name) {
                return Err("recorder checkpoint reuses a frame".into());
            }
            bytes = bytes
                .checked_add(ordinary_file(&session.dir.join(name))?.len())
                .ok_or("step size overflow")?;
        }
        if bytes != step.bytes as u64 {
            return Err("recorder frame size does not match checkpoint".into());
        }
    }
    Ok(())
}

pub(crate) fn create(inbox: &Path) -> Result<Session, String> {
    let inbox = writable_dir(inbox, true)?;
    let dir = inbox.join(format!("{PREFIX}{}", token()));
    before_write(&dir)?;
    fs::create_dir(&dir).map_err(|e| io_error(&dir, e))?;
    let lease = lease(&dir, true)?.ok_or("new recorder source is already in use")?;
    let source_id = dir
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("invalid source filename")?
        .to_owned();
    publish_json(
        &dir.join(SOURCE),
        &Owner {
            version: 1,
            source_id: SourceId(source_id),
        },
    )?;
    let session = Session {
        dir,
        steps: Vec::new(),
        next_attempt: 1,
        _lease: lease,
    };
    checkpoint(&session)?;
    Ok(session)
}

pub(crate) fn reserve_attempt(session: &mut Session) -> Result<(PathBuf, PathBuf), String> {
    session.dir = writable_dir(&session.dir, false)?;
    owner(&session.dir)?;
    loop {
        let attempt = session.next_attempt.max(1);
        session.next_attempt = attempt
            .checked_add(1)
            .ok_or("capture attempt counter exhausted")?;
        let (do_name, done_name) = frame_names(attempt);
        let do_path = session.dir.join(do_name);
        let done_path = session.dir.join(done_name);
        if present(&do_path)? || present(&done_path)? {
            continue;
        }
        let marker = session.dir.join(attempt_name(attempt));
        before_write(&marker)?;
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)
        {
            Ok(file) => {
                file.sync_all().map_err(|e| io_error(&marker, e))?;
                return Ok((do_path, done_path));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(io_error(&marker, e)),
        }
    }
}

pub(crate) fn checkpoint(session: &Session) -> Result<(), String> {
    let mut session = session.clone();
    session.dir = writable_dir(&session.dir, false)?;
    let owner = owner(&session.dir)?;
    validate_steps(&session)?;
    // Lifecycle callers append exactly one step. Earlier immutable frames were
    // synced with their preceding checkpoints; only the new pair needs flushing.
    if let Some(step) = session.steps.last() {
        for name in std::iter::once(&step.file).chain(step.result_file.iter()) {
            let path = session.dir.join(name);
            before_write(&path)?;
            OpenOptions::new()
                .write(true)
                .open(&path)
                .and_then(|file| file.sync_all())
                .map_err(|e| io_error(&path, e))?;
        }
    }
    publish_json(
        &session.dir.join(checkpoint_name(session.steps.len())),
        &Checkpoint {
            version: 1,
            source_id: owner.source_id,
            next_attempt: session.next_attempt,
            steps: session.steps,
        },
    )
}

fn load(dir: &Path, lease: Arc<File>) -> Result<Session, String> {
    let owner = owner(dir)?;
    let mut checkpoints = Vec::new();
    let mut next_attempt = 1;
    for (name, path) in entries(dir)? {
        safe_name(&name)?;
        ordinary_file(&path)?;
        if let Some(count) = numbered(&name, "checkpoint-", ".json", 10) {
            checkpoints.push((count, path));
        } else if name.starts_with("checkpoint-") && name.ends_with(".json") {
            return Err("malformed completed checkpoint name".into());
        }
        if let Some(attempt) = numbered(&name, ".attempt-", "", 20) {
            next_attempt = next_attempt.max(
                attempt
                    .checked_add(1)
                    .ok_or("capture attempt counter exhausted")?,
            );
        }
    }
    let (count, path) = checkpoints
        .into_iter()
        .max_by_key(|(count, _)| *count)
        .ok_or("source has no complete checkpoint")?;
    let checkpoint: Checkpoint = read_json(&path)?;
    if checkpoint.version != 1
        || checkpoint.source_id != owner.source_id
        || count != checkpoint.steps.len() as u64
        || checkpoint.next_attempt == 0
    {
        return Err("recorder checkpoint identity, count, or version mismatch".into());
    }
    let session = Session {
        dir: dir.to_path_buf(),
        steps: checkpoint.steps,
        next_attempt: next_attempt.max(checkpoint.next_attempt),
        _lease: lease,
    };
    validate_steps(&session)?;
    Ok(session)
}

fn receipt(session: &Session) -> Result<Option<Receipt>, String> {
    let path = session.dir.join(RECEIPT);
    if !present(&path)? {
        return Ok(None);
    }
    let receipt: Receipt = read_json(&path)?;
    safe_name(&receipt.target)?;
    if receipt.version != 1
        || receipt.source_id != owner(&session.dir)?.source_id
        || receipt.count as usize != session.steps.len()
        || receipt.target.starts_with('.')
        || crate::normalize_meta(receipt.meta.clone())? != receipt.meta
    {
        return Err("prepared save receipt identity, metadata, or version mismatch".into());
    }
    Ok(Some(receipt))
}

fn is_archived(dir: &Path) -> Result<bool, String> {
    let path = dir.join(ARCHIVED);
    if !present(&path)? {
        return Ok(false);
    }
    let archive: Owner = read_json(&path)?;
    let expected = owner(dir)?;
    if archive.version != expected.version || archive.source_id != expected.source_id {
        return Err("archive marker identity mismatch".into());
    }
    Ok(true)
}

pub(crate) fn prepared_meta(session: &Session) -> Result<Option<CaptureMeta>, String> {
    let mut session = session.clone();
    session.dir = writable_dir(&session.dir, false)?;
    Ok(receipt(&session)?.map(|receipt| receipt.meta))
}

pub(crate) fn recover(dir: &Path) -> Result<Session, String> {
    let dir = writable_dir(dir, false)?;
    owner(&dir)?;
    let lease = lease(&dir, false)?
        .ok_or("this recording is in use by another recorder; close it before recovering")?;
    let session = load(&dir, lease)?;
    if is_archived(&dir)? {
        return Err("this recording was archived; its source files are retained".into());
    }
    if let Some(receipt) = receipt(&session)? {
        let target = receipt_target(&session, &receipt)?;
        if publication_complete(&receipt, &target)? {
            return Err(format!(
                "this recording is already saved as {}",
                receipt.target
            ));
        }
        verify_target(&session, &receipt, &target)?;
    }
    Ok(session)
}

pub(crate) fn discover(inbox: &Path) -> Result<RecoveryScan, String> {
    let absolute = if inbox.is_absolute() {
        inbox.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(inbox)
    };
    let inbox = dpl_roots::writable(&absolute)?;
    let mut scan = RecoveryScan::default();
    if !present(&inbox)? {
        return Ok(scan);
    }
    ancestry(&inbox, false)?;
    let mut omitted = 0_usize;
    for (index, entry) in fs::read_dir(&inbox)
        .map_err(|e| io_error(&inbox, e))?
        .enumerate()
    {
        if index >= MAX_ENTRIES {
            return Err("recorder inbox has too many entries to scan".into());
        }
        let entry = entry.map_err(|e| io_error(&inbox, e))?;
        let name = entry.file_name();
        let Some(name) = name.to_str().filter(|name| name.starts_with(PREFIX)) else {
            continue;
        };
        match discover_source(&entry.path()) {
            Ok(Some(candidate)) if scan.candidates.len() < 256 => scan.candidates.push(candidate),
            Ok(Some(_)) => omitted += 1,
            Ok(None) => {}
            Err(error) if scan.issues.len() < 256 => scan.issues.push(format!(
                "Cannot recover {name}: {error}. Source files are retained."
            )),
            Err(_) => omitted += 1,
        }
    }
    if omitted > 0 {
        scan.issues.push(format!("{omitted} further sources omitted from this bounded list; archive resolved recordings and refresh."));
    }
    scan.candidates.sort_by(|a, b| a.dir.cmp(&b.dir));
    Ok(scan)
}

fn discover_source(dir: &Path) -> Result<Option<RecoveryCandidate>, String> {
    let metadata = fs::symlink_metadata(dir).map_err(|e| io_error(dir, e))?;
    if metadata.is_file() && !is_link(&metadata) {
        return Ok(None);
    }
    ancestry(dir, false)?;
    if !present(&dir.join(SOURCE))? {
        return Ok(None);
    }
    owner(dir)?;
    let Some(lease) = lease(dir, false)? else {
        return Ok(None);
    };
    if is_archived(dir)? {
        return Ok(None);
    }
    // Startup can crash before checkpoint 0. Malformed *complete* names still
    // go through load and become visible issues rather than silent rollback.
    if !entries(dir)?
        .iter()
        .any(|(name, _)| name.starts_with("checkpoint-") && name.ends_with(".json"))
    {
        return Ok(None);
    }
    let session = load(dir, lease)?;
    let receipt = receipt(&session)?;
    if let Some(receipt) = &receipt {
        let target = receipt_target(&session, receipt)?;
        if publication_complete(receipt, &target)? {
            return Ok(None);
        }
        verify_target(&session, receipt, &target)?;
    }
    Ok(Some(RecoveryCandidate {
        dir: dir.to_path_buf(),
        count: session.steps.len() as u32,
        seed_title: session
            .steps
            .first()
            .map(|step| step.title.clone())
            .unwrap_or_default(),
        prepared_meta: receipt.map(|receipt| receipt.meta),
    }))
}

pub(crate) fn archive(session: &Session) -> Result<(), String> {
    let dir = writable_dir(&session.dir, false)?;
    publish_json(&dir.join(ARCHIVED), &owner(&dir)?)
}

fn manifest(session: &Session, meta: &CaptureMeta, name: &str) -> Result<Vec<u8>, String> {
    let total = session.steps.iter().try_fold(0_u64, |sum, step| {
        sum.checked_add(step.bytes as u64)
            .ok_or("guide size overflow")
    })?;
    serde_json::to_vec_pretty(&serde_json::json!({
        "kind": "guide", "name": name, "stepCount": session.steps.len(),
        "totalBytes": total, "steps": session.steps, "tags": meta.tags,
    }))
    .map_err(|e| e.to_string())
}

fn assets(session: &Session) -> impl Iterator<Item = &String> {
    session
        .steps
        .iter()
        .flat_map(|step| std::iter::once(&step.file).chain(step.result_file.iter()))
}

fn same_file(a: &Path, b: &Path) -> Result<bool, String> {
    if ordinary_file(a)?.len() != ordinary_file(b)?.len() {
        return Ok(false);
    }
    let mut a = File::open(a).map_err(|e| e.to_string())?;
    let mut b = File::open(b).map_err(|e| e.to_string())?;
    let mut left = [0; 8192];
    let mut right = [0; 8192];
    loop {
        let n = a.read(&mut left).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(true);
        }
        b.read_exact(&mut right[..n]).map_err(|e| e.to_string())?;
        if left[..n] != right[..n] {
            return Ok(false);
        }
    }
}

fn receipt_target(session: &Session, receipt: &Receipt) -> Result<PathBuf, String> {
    let inbox = session.dir.parent().ok_or("recorder source has no inbox")?;
    Ok(inbox.join(&receipt.target))
}

// Completion is historical identity, not a promise that a filed guide will
// never be edited. Refresh must not reread every published image in the library.
fn publication_complete(receipt: &Receipt, target: &Path) -> Result<bool, String> {
    ancestry(target, false)?;
    let pending = present(&target.join(PENDING))?;
    let complete = present(&target.join(COMPLETE))?;
    if pending == complete {
        return Err("prepared guide must have exactly one publication marker".into());
    }
    let marker: Owner = read_json(&target.join(if pending { PENDING } else { COMPLETE }))?;
    if marker.version != 1 || marker.source_id != receipt.source_id {
        return Err("prepared guide belongs to a different source".into());
    }
    Ok(complete)
}

fn verify_target(session: &Session, receipt: &Receipt, target: &Path) -> Result<(), String> {
    publication_complete(receipt, target)?;
    if read_bytes(&target.join("guide.json"))? != manifest(session, &receipt.meta, &receipt.target)?
    {
        return Err("prepared guide manifest differs from its receipt; source retained".into());
    }
    for name in assets(session) {
        if !same_file(&session.dir.join(name), &target.join(name))? {
            return Err("prepared guide frame differs from source; source retained".into());
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SaveStage {
    Copying,
    Copied,
    Manifest,
    Prepared,
}

pub(crate) fn save_with_path(
    session: &Session,
    inbox: &Path,
    meta: &CaptureMeta,
) -> Result<SavedGuide, String> {
    save_transaction(session, inbox, meta, |_| Ok(()))
}

#[cfg(test)]
pub(crate) fn save(session: &Session, inbox: &Path, meta: &CaptureMeta) -> Result<String, String> {
    save_with_path(session, inbox, meta).map(|saved| saved.name)
}

#[cfg(test)]
fn save_impl(
    session: &Session,
    inbox: &Path,
    meta: &CaptureMeta,
    fault: impl Fn(SaveStage) -> Result<(), String>,
) -> Result<String, String> {
    save_transaction(session, inbox, meta, fault).map(|saved| saved.name)
}

fn save_transaction(
    session: &Session,
    inbox: &Path,
    meta: &CaptureMeta,
    fault: impl Fn(SaveStage) -> Result<(), String>,
) -> Result<SavedGuide, String> {
    let mut session = session.clone();
    session.dir = writable_dir(&session.dir, false)?;
    let inbox = writable_dir(inbox, true)?;
    if fs::canonicalize(session.dir.parent().ok_or("source has no inbox")?)
        .map_err(|e| e.to_string())?
        != fs::canonicalize(&inbox).map_err(|e| e.to_string())?
    {
        return Err(
            "save must use the recording's original inbox (including its rebased drive)".into(),
        );
    }
    if session.steps.is_empty() {
        return Err("no steps captured".into());
    }
    if is_archived(&session.dir)? {
        return Err("this recording is archived".into());
    }
    let committed = load(&session.dir, Arc::clone(&session._lease))?;
    if committed.steps != session.steps {
        return Err("pending steps differ from the durable checkpoint".into());
    }
    let normalized = crate::normalize_meta(meta.clone())?;
    if &normalized != meta {
        return Err("save metadata must be normalized before publication".into());
    }
    let receipt = if let Some(receipt) = receipt(&session)? {
        if &receipt.meta != meta {
            return Err(format!("Finish the prepared save as '{}' with its original tags; its metadata is already reserved.", receipt.meta.name));
        }
        receipt
    } else {
        let mut selected = None;
        for index in 1..=10_000 {
            let name = if index == 1 {
                meta.name.clone()
            } else {
                format!("{}-{index}", meta.name)
            };
            safe_name(&name)?;
            let target = inbox.join(&name);
            before_write(&target)?;
            match fs::create_dir(&target) {
                Ok(()) => {
                    selected = Some((name, target));
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(io_error(&target, e)),
            }
        }
        let (name, target) = selected.ok_or("too many existing guide names; source retained")?;
        let source = owner(&session.dir)?;
        publish_json(&target.join(PENDING), &source)?;
        for name in assets(&session) {
            let from = session.dir.join(name);
            let to = target.join(name);
            ordinary_file(&from)?;
            let mut input = File::open(&from).map_err(|e| io_error(&from, e))?;
            before_write(&to)?;
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&to)
                .map_err(|e| io_error(&to, e))?;
            fault(SaveStage::Copying)?;
            std::io::copy(&mut input, &mut output)
                .and_then(|_| output.sync_all())
                .map_err(|e| io_error(&to, e))?;
        }
        fault(SaveStage::Copied)?;
        write_new(
            &target.join("guide.json"),
            &manifest(&session, meta, &name)?,
        )?;
        fault(SaveStage::Manifest)?;
        let receipt = Receipt {
            version: 1,
            source_id: source.source_id,
            target: name,
            count: session.steps.len() as u32,
            meta: meta.clone(),
        };
        verify_target(&session, &receipt, &target)?;
        publish_json(&session.dir.join(RECEIPT), &receipt)?;
        receipt
    };
    fault(SaveStage::Prepared)?;
    let target = receipt_target(&session, &receipt)?;
    verify_target(&session, &receipt, &target)?;
    if !present(&target.join(COMPLETE))? {
        before_write(&target.join(COMPLETE))?;
        fs::rename(target.join(PENDING), target.join(COMPLETE))
            .map_err(|e| io_error(&target, e))?;
    }
    Ok(SavedGuide {
        name: receipt.target,
        dir: target,
    })
}

#[cfg(test)]
mod tests;
