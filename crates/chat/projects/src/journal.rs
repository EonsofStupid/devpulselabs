//! Immutable revisions with an exclusive commit record. Incomplete revisions
//! remain on disk for inspection and never replace the last complete save.

use crate::delta::Delta;
use crate::disk;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_REVISIONS: usize = 100_000;
const MAX_DELTAS: usize = 63;
const MAX_REPLAY_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    v: u32,
    revision: u64,
    base: Option<u64>,
    change: Delta,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Commit {
    v: u32,
    revision: u64,
    bytes: u64,
    checksum: String,
    checkpoint: bool,
}

struct Replay {
    revision: u64,
    value: Value,
    deltas: usize,
    bytes: u64,
}

fn checksum(bytes: &[u8]) -> String {
    // Detect torn/corrupt data; this is not an authenticity/security hash.
    let mut value = 0xcbf29ce484222325_u64;
    for byte in bytes {
        value ^= u64::from(*byte);
        value = value.wrapping_mul(0x100000001b3);
    }
    format!("{value:016x}")
}

fn file(dir: &Path, revision: u64, extension: &str) -> PathBuf {
    dir.join(format!("{revision:020}.{extension}"))
}

fn revisions(dir: &Path) -> Result<Vec<(u64, bool)>, String> {
    disk::path(dir)?;
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{}: {error}", dir.display())),
    };
    let mut revisions = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        if revisions.len() >= MAX_REVISIONS * 2 {
            return Err("Project chat revision limit reached; existing saves are preserved".into());
        }
        let name = entry.file_name();
        let name = name
            .to_str()
            .ok_or("Unrecognized file in project chat revisions")?;
        let (digits, extension) = name
            .split_once('.')
            .ok_or("Unrecognized file in project chat revisions")?;
        if digits.len() != 20
            || !digits.bytes().all(|byte| byte.is_ascii_digit())
            || !matches!(extension, "json" | "commit")
        {
            return Err(format!(
                "Unrecognized project chat revision {name}; nothing was replaced"
            ));
        }
        let metadata = entry.metadata().map_err(|error| error.to_string())?;
        if !metadata.is_file() || disk::is_link(&metadata) {
            return Err(format!("Project chat revision {name} is not a file"));
        }
        revisions.push((
            digits.parse::<u64>().map_err(|error| error.to_string())?,
            extension == "commit",
        ));
    }
    revisions.sort_unstable();
    Ok(revisions)
}

pub(crate) fn latest(dir: &Path) -> Result<Option<(u64, Value)>, String> {
    Ok(replay(dir)?.map(|saved| (saved.revision, saved.value)))
}

fn replay(dir: &Path) -> Result<Option<Replay>, String> {
    let entries = revisions(dir)?;
    let mut chain = Vec::new();
    let mut replay_bytes = 0_u64;
    for (revision, committed) in entries.into_iter().rev() {
        if !committed {
            continue;
        }
        let bytes = disk::read(&file(dir, revision, "commit"), 4096)?;
        replay_bytes += bytes.len() as u64;
        if replay_bytes > MAX_REPLAY_BYTES {
            return Err("Project chat commit scan exceeds its byte limit".into());
        }
        // A trailing newline publishes the complete commit record. A crash
        // before it arrived leaves an inspectable but unpublished revision.
        if !bytes.ends_with(b"\n") {
            continue;
        }
        let commit: Commit = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Damaged project chat commit: {error}"))?;
        if commit.v != 1 || commit.revision != revision || commit.bytes > disk::MAX_RECORD_BYTES {
            return Err(
                "Unsupported or inconsistent project chat commit; existing saves are preserved"
                    .into(),
            );
        }
        let checkpoint = commit.checkpoint;
        chain.push((revision, commit));
        if checkpoint {
            break;
        }
        if chain.len() > MAX_DELTAS {
            return Err("Project chat replay chain exceeds its checkpoint limit".into());
        }
    }
    let mut selected: Option<Replay> = None;
    for (revision, commit) in chain.into_iter().rev() {
        replay_bytes = replay_bytes
            .checked_add(commit.bytes)
            .ok_or("Project chat replay size overflow")?;
        if replay_bytes > MAX_REPLAY_BYTES {
            return Err(
                "Project chat replay exceeds its byte limit; existing saves are preserved".into(),
            );
        }
        let bytes = disk::read(&file(dir, revision, "json"), disk::MAX_RECORD_BYTES)?;
        if bytes.len() as u64 != commit.bytes || checksum(&bytes) != commit.checksum {
            return Err("A saved project chat revision is damaged; it was not replaced".into());
        }
        let record: Record = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        if record.v != 1
            || record.revision != revision
            || record.base != selected.as_ref().map(|saved| saved.revision)
            || commit.checkpoint != selected.is_none()
        {
            return Err(
                "Unsupported or inconsistent project chat revision; existing saves are preserved"
                    .into(),
            );
        }
        if selected.is_none() && !matches!(&record.change, Delta::Replace { .. }) {
            return Err("The first project chat revision has no full state".into());
        }
        let deltas = selected.as_ref().map(|saved| saved.deltas + 1).unwrap_or(0);
        let mut value = selected
            .take()
            .map(|saved| saved.value)
            .unwrap_or(Value::Null);
        record.change.apply(&mut value)?;
        check_size(&value)?;
        selected = Some(Replay {
            revision,
            value,
            deltas,
            bytes: replay_bytes,
        });
    }
    Ok(selected)
}

fn check_size(value: &Value) -> Result<(), String> {
    // The reconstructed state is bounded too, not only individual deltas.
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > disk::MAX_RECORD_BYTES {
        return Err("Project chat state is too large; the previous save is preserved".into());
    }
    Ok(())
}

/// Caller holds the owner's writer lock and has just verified its marker.
pub(crate) fn append(
    dir: &Path,
    data: &Value,
    expected: Option<Option<u64>>,
) -> Result<u64, String> {
    check_size(data)?;
    let previous = replay(dir)?;
    let previous_revision = previous.as_ref().map(|saved| saved.revision);
    if expected.is_some_and(|expected| expected != previous_revision) {
        return Err("This project's chats changed in another app instance. Reopen the project before saving; your current edits have not been replaced".into());
    }
    if let Some(saved) = previous.as_ref().filter(|saved| &saved.value == data) {
        return Ok(saved.revision);
    }
    let entries = revisions(dir)?;
    if entries.len() > MAX_REVISIONS * 2 - 2 {
        return Err("Project chat revision limit reached; existing saves are preserved".into());
    }
    let revision = entries
        .last()
        .map(|(revision, _)| *revision)
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("Project chat revision space exhausted")?;
    let mut checkpoint = previous
        .as_ref()
        .is_none_or(|saved| saved.deltas >= MAX_DELTAS);
    let encode = |checkpoint: bool| {
        let (base, change) = match previous.as_ref().filter(|_| !checkpoint) {
            Some(saved) => (Some(saved.revision), Delta::between(&saved.value, data)),
            None => (
                None,
                Delta::Replace {
                    value: data.clone(),
                },
            ),
        };
        serde_json::to_vec(&Record {
            v: 1,
            revision,
            base,
            change,
        })
        .map_err(|error| error.to_string())
    };
    let mut bytes = encode(checkpoint)?;
    if !checkpoint
        && previous.as_ref().map(|saved| saved.bytes).unwrap_or(0) + bytes.len() as u64 + 4096
            > MAX_REPLAY_BYTES
    {
        checkpoint = true;
        bytes = encode(true)?;
    }
    if bytes.len() as u64 > disk::MAX_RECORD_BYTES {
        return Err(
            "Project chat state is too large to save; the previous save is preserved".into(),
        );
    }
    // Apply the reader's nesting/format limits before publishing a revision.
    // A caller-built deeply nested Value must not create an unreadable save.
    serde_json::from_slice::<Record>(&bytes).map_err(|error| error.to_string())?;
    disk::path(dir)?;
    disk::create_directories(dir)?;
    disk::create(&file(dir, revision, "json"), &bytes)?;
    let mut commit = serde_json::to_vec(&Commit {
        v: 1,
        revision,
        bytes: bytes.len() as u64,
        checksum: checksum(&bytes),
        checkpoint,
    })
    .map_err(|error| error.to_string())?;
    commit.push(b'\n');
    disk::create(&file(dir, revision, "commit"), &commit)?;
    Ok(revision)
}
