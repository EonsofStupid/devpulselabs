//! The ONE import seam: an external media file becomes an ordinary
//! artifact — copied (sources NEVER mutated), postered, timed, and
//! write-through indexed. Browse windows and Explorer drops both land
//! here; later feeders (Drive) reuse it unchanged.
//!
//! Kinds: video/image copy into `content/inbox/`; `.aep`/`.lottie`
//! (operator-ratified 2026-07-24, personal-use tool) copy into
//! `content/elements/imported/` as ELEMENT artifacts — parsing them
//! into comp layers is its own later spike, never faked.

use dpl_index::Record;
use std::path::{Path, PathBuf};

/// "video" / "image" / "audio" / "element" by extension; None = not
/// media we import. The element tier carries the operator's real pack
/// formats (2026-07-24 census): AE projects, Lottie, Motion Graphics
/// Templates, LUTs, AE/Premiere presets.
pub fn media_kind(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" => Some("video"),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" => Some("image"),
        "wav" | "mp3" | "ogg" | "flac" | "m4a" | "aac" => Some("audio"),
        "aep" | "lottie" | "mogrt" | "cube" | "ffx" | "prfpset" => Some("element"),
        _ => None,
    }
}

/// The first free `<stem>.<ext>` / `<stem>-N.<ext>` inside `dir`.
pub fn no_clobber(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.{ext}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{stem}-{n}.{ext}")))
        .find(|p| !p.exists())
        .unwrap()
}

/// A comp/guide name the filesystem and the kebab law both accept;
/// None when nothing usable survives.
pub fn sanitize_name(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_whitespace() { '-' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    let cleaned = cleaned.trim_matches('-').to_string();
    (!cleaned.is_empty()).then_some(cleaned)
}

/// Copy `src` into the content tree and index it. Returns the indexed
/// record. The SOURCE file is never touched (copy, never move).
pub fn import_file(src: &Path) -> Result<Record, String> {
    let kind = media_kind(src).ok_or_else(|| format!("not a media file: {}", src.display()))?;
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("import")
        .to_string();
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let root = dpl_engine_capture::content_root();
    let dest_dir = if kind == "element" {
        root.join("elements").join("imported")
    } else {
        root.join("inbox")
    };
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("dest dir: {e}"))?;
    let dest = no_clobber(&dest_dir, &stem, &ext);
    std::fs::copy(src, &dest).map_err(|e| format!("copy: {e}"))?;

    let mut rec = dpl_index::record_for_capture(&dest, &root, kind, "import");
    if kind == "element" {
        enrich_element(&mut rec, &dest, &root);
    }
    if kind == "video" {
        let poster = dest.with_file_name(format!(
            "{}-poster.jpg",
            dest.file_stem().and_then(|s| s.to_str()).unwrap_or(&stem)
        ));
        // A missing poster degrades the card, never the import.
        if dpl_engine_media::write_poster(&dest, &poster).is_ok() {
            rec.poster_rel_path = poster
                .strip_prefix(&root)
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"));
        }
        rec.seconds = dpl_video::clip::clip_duration(&dest)
            .filter(|d| *d > 0.0)
            .map(|d| d.round() as u32);
    }
    dpl_index::Index::open_default()?.upsert(&rec)?;
    Ok(rec)
}

/// Enrich an element record from what the FILE ITSELF declares — today
/// that is `.mogrt` (R2a.2): the capsule's real name, its tags, and the
/// embedded `thumb.png` extracted beside the copy as the poster.
/// Best-effort: any failure leaves the plain element record intact.
pub(crate) fn enrich_element(rec: &mut Record, dest: &Path, root: &Path) {
    let is_mogrt = dest
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("mogrt"));
    if !is_mogrt {
        return;
    }
    let Ok(m) = crate::mogrt::read(dest) else {
        return;
    };
    if !m.name.trim().is_empty() {
        rec.name = m.name;
    }
    rec.tags = m.tags;
    if !m.thumb_png.is_empty() {
        let poster = dest.with_file_name(format!(
            "{}-poster.png",
            dest.file_stem().and_then(|s| s.to_str()).unwrap_or("mogrt")
        ));
        if std::fs::write(&poster, &m.thumb_png).is_ok() {
            rec.poster_rel_path = poster
                .strip_prefix(root)
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_kind_matrix_covers_the_ratified_extensions() {
        assert_eq!(media_kind(Path::new("a.mp4")), Some("video"));
        assert_eq!(media_kind(Path::new("a.MOV")), Some("video"));
        assert_eq!(media_kind(Path::new("b.JPG")), Some("image"));
        assert_eq!(media_kind(Path::new("b.webp")), Some("image"));
        assert_eq!(media_kind(Path::new("song.WAV")), Some("audio"));
        assert_eq!(media_kind(Path::new("song.mp3")), Some("audio"));
        assert_eq!(media_kind(Path::new("gig.aep")), Some("element"));
        assert_eq!(media_kind(Path::new("gig.lottie")), Some("element"));
        assert_eq!(media_kind(Path::new("title.MOGRT")), Some("element"));
        assert_eq!(media_kind(Path::new("grade.cube")), Some("element"));
        assert_eq!(media_kind(Path::new("preset.ffx")), Some("element"));
        assert_eq!(media_kind(Path::new("preset.prfpset")), Some("element"));
        assert_eq!(media_kind(Path::new("c.txt")), None);
        assert_eq!(media_kind(Path::new("pack.zip")), None);
        assert_eq!(media_kind(Path::new("noext")), None);
    }

    #[test]
    fn no_clobber_numbers_from_two() {
        let dir = std::env::temp_dir()
            .join("dpl-composer-tests")
            .join(format!("nc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let first = no_clobber(&dir, "clip", "mp4");
        assert!(first.ends_with("clip.mp4"));
        std::fs::write(&first, b"x").unwrap();
        let second = no_clobber(&dir, "clip", "mp4");
        assert!(second.ends_with("clip-2.mp4"));
    }

    #[test]
    fn sanitize_name_kebabs_or_declines() {
        assert_eq!(sanitize_name("My Teaser 04"), Some("my-teaser-04".into()));
        assert_eq!(sanitize_name("  ep. one!  "), Some("ep-one".into()));
        assert_eq!(sanitize_name("---"), None);
        assert_eq!(sanitize_name(""), None);
    }

    #[test]
    fn a_mogrt_element_record_gains_name_tags_and_poster() {
        use std::io::Write;
        let root = std::env::temp_dir()
            .join("dpl-composer-tests")
            .join(format!("enrich-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dest_dir = root.join("elements").join("imported");
        std::fs::create_dir_all(&dest_dir).unwrap();
        let dest = dest_dir.join("title-01.mogrt");
        let file = std::fs::File::create(&dest).unwrap();
        let mut z = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        z.start_file("definition.json", opts).unwrap();
        z.write_all(br#"{"capsuleName":"Neon Title","capsuleTags":["title","neon"]}"#)
            .unwrap();
        z.start_file("thumb.png", opts).unwrap();
        z.write_all(b"png-bytes").unwrap();
        z.finish().unwrap();

        let mut rec = Record::default();
        enrich_element(&mut rec, &dest, &root);
        assert_eq!(rec.name, "Neon Title", "capsule name wins the card");
        assert_eq!(rec.tags, vec!["title", "neon"]);
        assert_eq!(
            rec.poster_rel_path.as_deref(),
            Some("elements/imported/title-01-poster.png")
        );
        assert!(dest_dir.join("title-01-poster.png").is_file());
        // A cube (no zip) is untouched — best-effort, never a failure.
        let cube = dest_dir.join("grade.cube");
        std::fs::write(&cube, b"TITLE x").unwrap();
        let mut plain = Record::default();
        enrich_element(&mut plain, &cube, &root);
        assert!(plain.name.is_empty() && plain.poster_rel_path.is_none());
        let _ = std::fs::remove_dir_all(&root);
    }
}
