//! The PACK SHELF's registry — the operator's element/media packs
//! (themeforest gigs and friends) pinned IN PLACE, never copied in
//! bulk (the 80 GB census vs 14 GB of C: made copy-all arithmetic
//! nonsense). `sources.json` lists pinned roots; packs are the roots'
//! top-level folders, listed LIVE from the filesystem. Using a file
//! copies exactly that file through the W2 import seam (copy-on-use).
//! The pinned trees are READ-ONLY to this app, always.

use crate::import::media_kind;
use std::path::{Path, PathBuf};

/// One pack on the shelf: a top-level folder of a pinned source.
pub struct Pack {
    pub name: String,
    pub path: PathBuf,
    /// Media files inside (recursive, capped) — honest scale, not a lie.
    pub files: usize,
    /// A local copy exists under `content/elements/<kebab name>/`
    /// (the operator ingested this favorite).
    pub ingested: bool,
}

/// Where the registry lives: `<content>/elements/sources.json`.
fn sources_path(root: &Path) -> PathBuf {
    root.join("elements").join("sources.json")
}

/// The pinned source roots (absent/invalid file = empty, never a crash).
pub fn load_sources(root: &Path) -> Vec<PathBuf> {
    let Ok(raw) = std::fs::read_to_string(sources_path(root)) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<String>>(&raw)
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

/// Pin one source root (deduped, persisted pretty). App-action only.
pub fn add_source(root: &Path, dir: &Path) -> Result<(), String> {
    let mut sources = load_sources(root);
    if sources.iter().any(|s| s == dir) {
        return Ok(());
    }
    sources.push(dir.to_path_buf());
    let path = sources_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("elements dir: {e}"))?;
    }
    let listed: Vec<String> = sources
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let json = serde_json::to_string_pretty(&listed).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("sources.json: {e}"))
}

/// Count media files under `dir`, stopping at `cap` (70k-file packs
/// deserve an honest "1000+" over a minute-long exact count).
fn count_media(dir: &Path, cap: usize) -> usize {
    let mut stack = vec![dir.to_path_buf()];
    let mut n = 0usize;
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name();
            if name.to_string_lossy().starts_with('.') {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if media_kind(&p).is_some() {
                n += 1;
                if n >= cap {
                    return n;
                }
            }
        }
    }
    n
}

/// Where a pack's LOCAL copy lives once ingested.
pub fn local_dir(root: &Path, pack_name: &str) -> PathBuf {
    let kebab = crate::import::sanitize_name(pack_name).unwrap_or_else(|| "pack".into());
    root.join("elements").join(kebab)
}

/// Every pack on the shelf: each pinned source's top-level folders
/// (name-sorted), with a capped media count. A source that is itself
/// flat (loose files, no subfolders) lists as ONE pack.
pub fn list_packs(root: &Path) -> Vec<Pack> {
    const CAP: usize = 1000;
    let mut out = Vec::new();
    for source in load_sources(root) {
        let Ok(rd) = std::fs::read_dir(&source) else {
            continue;
        };
        let mut dirs: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.is_dir()
                    && !p
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with('.'))
            })
            .collect();
        dirs.sort();
        if dirs.is_empty() {
            let name = source
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| source.to_string_lossy().into_owned());
            out.push(Pack {
                files: count_media(&source, CAP),
                ingested: local_dir(root, &name).is_dir(),
                name,
                path: source,
            });
            continue;
        }
        for dir in dirs {
            let name = dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.push(Pack {
                files: count_media(&dir, CAP),
                ingested: local_dir(root, &name).is_dir(),
                name,
                path: dir,
            });
        }
    }
    out
}

/// Every media file under a pack, recursive, dot-junk skipped,
/// name-sorted — the ingest worklist (UNCAPPED; ingest is deliberate).
pub fn media_files(pack: &Path) -> Vec<PathBuf> {
    let mut stack = vec![pack.to_path_buf()];
    let mut out = Vec::new();
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if e.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
            } else if media_kind(&p).is_some() {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The pack's full media weight — (files, bytes) — for the arm confirm.
pub fn pack_size(pack: &Path) -> (usize, u64) {
    let files = media_files(pack);
    let bytes = files
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .sum();
    (files.len(), bytes)
}

/// INGEST one pack: copy every media file into
/// `content/elements/<kebab name>/<relative subpath>` — SKIP-IF-EXISTS
/// (resumable/idempotent), copy-to-`.part`-then-rename (never a torn
/// file), poster+seconds for videos (best-effort), write-through into
/// the ROOT's index. Returns the number COPIED this run. The source
/// pack is never touched. `done` ticks per processed file (progress).
pub fn ingest_copy(
    root: &Path,
    pack: &Path,
    pack_name: &str,
    done: &std::sync::atomic::AtomicUsize,
) -> Result<usize, String> {
    let dest_root = local_dir(root, pack_name);
    let ix = dpl_index::Index::open(&root.join("artifacts.db"))?;
    let mut copied = 0usize;
    for src in media_files(pack) {
        done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let rel = src
            .strip_prefix(pack)
            .map_err(|_| format!("outside pack: {}", src.display()))?;
        let dest = dest_root.join(rel);
        if dest.exists() {
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        let part = dest.with_extension("part");
        std::fs::copy(&src, &part).map_err(|e| {
            let _ = std::fs::remove_file(&part);
            format!("copy {} after {copied} file(s): {e}", src.display())
        })?;
        std::fs::rename(&part, &dest).map_err(|e| format!("finish {}: {e}", dest.display()))?;
        copied += 1;

        let kind = media_kind(&dest).unwrap_or("element");
        let mut rec = dpl_index::record_for_capture(&dest, root, kind, "ingest");
        if kind == "video" {
            let poster = dest.with_file_name(format!(
                "{}-poster.jpg",
                dest.file_stem().and_then(|s| s.to_str()).unwrap_or("frame")
            ));
            if dpl_engine_media::write_poster(&dest, &poster).is_ok() {
                rec.poster_rel_path = poster
                    .strip_prefix(root)
                    .ok()
                    .map(|p| p.to_string_lossy().replace('\\', "/"));
            }
            rec.seconds = dpl_video::clip::clip_duration(&dest)
                .filter(|d| *d > 0.0)
                .map(|d| d.round() as u32);
        }
        ix.upsert(&rec)?;
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("dpl-composer-tests")
            .join(format!("packs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn add_source_persists_and_dedupes() {
        let root = scratch("reg");
        let src = root.join("fake-source");
        std::fs::create_dir_all(&src).unwrap();
        add_source(&root, &src).unwrap();
        add_source(&root, &src).unwrap();
        let sources = load_sources(&root);
        assert_eq!(sources.len(), 1, "deduped");
        assert_eq!(sources[0], src);
    }

    #[test]
    fn packs_list_from_top_level_dirs_with_media_counts() {
        let root = scratch("list");
        let src = root.join("store");
        let pack = src.join("videohive-XY-things");
        std::fs::create_dir_all(&pack).unwrap();
        std::fs::write(pack.join("clip.mp4"), b"x").unwrap();
        std::fs::write(pack.join("notes.txt"), b"x").unwrap();
        std::fs::write(pack.join("song.wav"), b"x").unwrap();
        add_source(&root, &src).unwrap();
        let packs = list_packs(&root);
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].name, "videohive-XY-things");
        assert_eq!(packs[0].files, 2, "mp4 + wav count; txt does not");
    }

    #[test]
    fn ingest_copies_indexes_and_resumes() {
        use std::sync::atomic::AtomicUsize;
        let root = scratch("ingest");
        let pack = root.join("d-store").join("videohive-ZZ-lower-thirds");
        std::fs::create_dir_all(pack.join("Full HD")).unwrap();
        std::fs::write(pack.join("Full HD").join("lt-01.mp4"), b"vid").unwrap();
        std::fs::write(pack.join("song.wav"), b"aud").unwrap();
        std::fs::write(pack.join("readme.txt"), b"no").unwrap();

        let done = AtomicUsize::new(0);
        let copied = ingest_copy(&root, &pack, "videohive-ZZ-lower-thirds", &done).unwrap();
        assert_eq!(copied, 2, "both media files copied; txt skipped");
        let local = local_dir(&root, "videohive-ZZ-lower-thirds");
        assert!(local.join("Full HD").join("lt-01.mp4").is_file());
        assert!(local.join("song.wav").is_file());
        // The SOURCE is untouched and re-ingest copies nothing (resume).
        assert!(pack.join("song.wav").is_file());
        let again = ingest_copy(&root, &pack, "videohive-ZZ-lower-thirds", &done).unwrap();
        assert_eq!(again, 0, "idempotent");
        // Index rows exist in THIS root's db.
        let rows = dpl_index::Index::open(&root.join("artifacts.db"))
            .and_then(|ix| ix.recent(50, None))
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.source == "ingest"));
        // The shelf marks it local.
        assert!(local_dir(&root, "videohive-ZZ-lower-thirds").is_dir());
    }

    #[test]
    fn a_flat_source_lists_as_one_pack() {
        let root = scratch("flat");
        let src = root.join("loose");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("a.mp3"), b"x").unwrap();
        add_source(&root, &src).unwrap();
        let packs = list_packs(&root);
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].name, "loose");
        assert_eq!(packs[0].files, 1);
    }
}
