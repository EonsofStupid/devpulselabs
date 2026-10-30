//! INGEST PACK — copy one of the operator's element/media packs
//! (themeforest gigs and friends) into the library, on purpose. The packs
//! themselves live in place as Locations (ST5b): their folders are linked,
//! never copied in bulk (the 80 GB census vs 14 GB of C: made copy-all
//! arithmetic nonsense), and are READ-ONLY to this app, always.

use crate::import::media_kind;
use std::path::{Path, PathBuf};

/// Where a pack's LOCAL copy lives once ingested.
pub fn local_dir(root: &Path, pack_name: &str) -> PathBuf {
    let kebab = crate::import::sanitize_name(pack_name).unwrap_or_else(|| "pack".into());
    root.join("elements").join(kebab)
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
/// file), RTFX grammar tags for every kind (L3), the pack's OWN preview
/// jpg as the poster when it ships one (an alpha element's first frame is
/// blank — a generated poster would lie), poster+seconds fallback for
/// videos, write-through into the CALLER'S index. The caller owns index
/// placement: the app passes `Index::open_default()` (the ADR-014
/// `.userdata/index.db` every read surface opens — the old ad-hoc
/// `root/artifacts.db` open wrote a retired legacy DB nothing read);
/// tests pass a scratch DB. Returns the number COPIED this run. The
/// source pack is never touched. `done` ticks per processed file.
pub fn ingest_copy(
    root: &Path,
    pack: &Path,
    pack_name: &str,
    ix: &dpl_index::Index,
    done: &std::sync::atomic::AtomicUsize,
) -> Result<usize, String> {
    let dest_root = local_dir(root, pack_name);
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
        if kind == "element" {
            crate::import::enrich_element(&mut rec, &dest, root);
        }
        // RTFX grammar → tags, for EVERY kind (movs are "video", SFX are
        // "audio" — the element tier isn't the only tagged one).
        if let (Some(stem), Some(bucket)) = (
            dest.file_stem().and_then(|s| s.to_str()),
            rel.parent().map(|p| p.to_string_lossy()),
        ) {
            if let Some(tags) = crate::rtfx::rtfx_tags(stem, &bucket) {
                rec.tags = tags;
            }
            // Grammar-independent tags (L4: bare "X Matte NN" sources).
            for t in crate::rtfx::generic_media_tags(stem) {
                if !rec.tags.contains(&t) {
                    rec.tags.push(t);
                }
            }
        }
        if kind == "video" {
            let poster = dest.with_file_name(format!(
                "{}-poster.jpg",
                dest.file_stem().and_then(|s| s.to_str()).unwrap_or("frame")
            ));
            // The pack's shipped preview (probed from the SOURCE, still in
            // the pinned tree) beats a generated first-frame poster.
            let postered = match crate::rtfx::preview_poster(&src) {
                Some(jpg) => std::fs::copy(&jpg, &poster).is_ok(),
                None => dpl_engine_media::write_poster(&dest, &poster).is_ok(),
            };
            if postered {
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
    fn ingest_copies_indexes_and_resumes() {
        use std::sync::atomic::AtomicUsize;
        let root = scratch("ingest");
        let pack = root.join("d-store").join("videohive-ZZ-lower-thirds");
        std::fs::create_dir_all(pack.join("Full HD")).unwrap();
        std::fs::write(pack.join("Full HD").join("lt-01.mp4"), b"vid").unwrap();
        std::fs::write(pack.join("song.wav"), b"aud").unwrap();
        std::fs::write(pack.join("readme.txt"), b"no").unwrap();

        let done = AtomicUsize::new(0);
        // The CALLER owns index placement (the app passes open_default —
        // the ADR-014 DB; a scratch DB keeps this test hermetic).
        let ix = dpl_index::Index::open(&root.join("scratch-index.db")).unwrap();
        let copied = ingest_copy(&root, &pack, "videohive-ZZ-lower-thirds", &ix, &done).unwrap();
        assert_eq!(copied, 2, "both media files copied; txt skipped");
        let local = local_dir(&root, "videohive-ZZ-lower-thirds");
        assert!(local.join("Full HD").join("lt-01.mp4").is_file());
        assert!(local.join("song.wav").is_file());
        // The SOURCE is untouched and re-ingest copies nothing (resume).
        assert!(pack.join("song.wav").is_file());
        let again = ingest_copy(&root, &pack, "videohive-ZZ-lower-thirds", &ix, &done).unwrap();
        assert_eq!(again, 0, "idempotent");
        // Index rows exist in the CALLER'S db.
        let rows = ix.recent(50, None).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.source == "ingest"));
        assert!(local_dir(&root, "videohive-ZZ-lower-thirds").is_dir());
    }
}
