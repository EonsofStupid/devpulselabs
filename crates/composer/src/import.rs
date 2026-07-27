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
        // Native Lottie (L2): rendered in-app. `.json` is ambiguous —
        // `import_file` content-sniffs it (Bodymovin only) so app JSON
        // (comp/guide/slide) is never imported as media.
        "lottie" | "json" => Some("lottie"),
        "aep" | "mogrt" | "cube" | "ffx" | "prfpset" => Some("element"),
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

/// Rasterize each page of a PDF to a PNG via `pdftoppm` — a SUBPROCESS
/// (ADR-009, the ffmpeg pattern: NEVER a crate PDF parser) wrapped in the
/// quiet-spawn law. Pages land in `out_dir` as `<stem>-<n>.png` (pdftoppm
/// zero-pads within a document, so a lexical sort is page order). Returns
/// the page paths in order. The tool is resolved off PATH today; its absence
/// is an HONEST error naming the fix (embedded-deps staging is the portable
/// follow-up, exactly like managed ffmpeg).
pub fn rasterize_pdf(pdf: &Path, out_dir: &Path, stem: &str) -> Result<Vec<PathBuf>, String> {
    let is_pdf = pdf
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false);
    if !is_pdf {
        return Err(format!("not a pdf: {}", pdf.display()));
    }
    if !pdf.is_file() {
        return Err(format!("pdf not found: {}", pdf.display()));
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("stage dir: {e}"))?;
    let prefix = out_dir.join(stem);

    let mut cmd = std::process::Command::new("pdftoppm");
    cmd.arg("-png").arg("-r").arg("150").arg(pdf).arg(&prefix);
    let status = dpl_proc::quiet(&mut cmd).status().map_err(|e| {
        format!("pdftoppm unavailable ({e}) — install poppler-utils or stage it in embedded-deps")
    })?;
    if !status.success() {
        return Err(format!("pdftoppm failed on {}", pdf.display()));
    }

    let mut pages: Vec<PathBuf> = std::fs::read_dir(out_dir)
        .map_err(|e| format!("read pages: {e}"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|x| x.to_str()) == Some("png")
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with(stem))
        })
        .collect();
    pages.sort();
    if pages.is_empty() {
        return Err(format!("pdftoppm produced no pages for {}", pdf.display()));
    }
    Ok(pages)
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
    // A `.json` only imports as Lottie when it's actually Bodymovin —
    // this refuses comp.json/guide.json/slide.json and other data JSON.
    if kind == "lottie" && ext == "json" {
        let bytes = std::fs::read(src).map_err(|e| format!("read {}: {e}", src.display()))?;
        if !dpl_lottie::is_bodymovin_json(&bytes) {
            return Err(format!(
                "{} is not a Lottie (Bodymovin) JSON",
                src.display()
            ));
        }
    }
    let root = dpl_engine_capture::content_root();
    let dest_dir = if kind == "element" {
        root.join("elements").join("imported")
    } else {
        root.join("inbox")
    };
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("dest dir: {e}"))?;
    let dest = no_clobber(&dest_dir, &stem, &ext);
    std::fs::copy(src, &dest).map_err(|e| format!("copy: {e}"))?;
    finalize_import(&dest, kind, "import", &root, &stem)
}

/// Import an artifact the app GENERATED (Chat Imagine drafts being promoted
/// into the library). Identical landing to `import_file` — copy into
/// `content/inbox/`, write-through index — but the record's source says
/// `generated` so the gallery can tell operator captures from AI output.
pub fn import_generated(src: &Path) -> Result<Record, String> {
    let kind = media_kind(src).ok_or_else(|| format!("not a media file: {}", src.display()))?;
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("generated")
        .to_string();
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let root = dpl_engine_capture::content_root();
    let dest_dir = root.join("inbox");
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("dest dir: {e}"))?;
    let dest = no_clobber(&dest_dir, &stem, &ext);
    std::fs::copy(src, &dest).map_err(|e| format!("copy: {e}"))?;
    finalize_import(&dest, kind, "generated", &root, &stem)
}

/// REFERENCE an external file IN-PLACE instead of copying it — the operator's
/// paid AE presets / image folders stay where they live; a single-FILE symlink
/// lands in `content/` and is indexed like any artifact. QoL: no duplication of
/// a themeforest/AE library into the app. Single-FILE symlinks (never folders)
/// keep rescans cheap — the folder-symlink rescan blow-up the pack shelf
/// rejected does not apply here. Falls back to a copy where the OS denies
/// symlinks (e.g. Windows without the privilege), so an import never fails.
pub fn import_reference(src: &Path) -> Result<Record, String> {
    let kind = media_kind(src).ok_or_else(|| format!("not a media file: {}", src.display()))?;
    // Resolve to an absolute target so the symlink survives the app's cwd.
    let target =
        std::fs::canonicalize(src).map_err(|e| format!("resolve {}: {e}", src.display()))?;
    let stem = target
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("ref")
        .to_string();
    let ext = target
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let root = dpl_engine_capture::content_root();
    let dest_dir = if kind == "element" {
        root.join("elements").join("referenced")
    } else {
        root.join("inbox")
    };
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("dest dir: {e}"))?;
    let dest = no_clobber(&dest_dir, &stem, &ext);
    if symlink_file(&target, &dest).is_err() {
        // Symlinks denied (Windows w/o privilege) — a copy still works.
        std::fs::copy(&target, &dest).map_err(|e| format!("reference fallback copy: {e}"))?;
    }
    finalize_import(&dest, kind, "reference", &root, &stem)
}

/// Create a single-FILE symlink `dest -> target`, cross-platform. `unix`
/// covers Linux AND macOS; Windows has its own call (needs the create-symlink
/// privilege or Developer Mode — the caller falls back to a copy on failure).
#[cfg(unix)]
fn symlink_file(target: &Path, dest: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, dest)
}
#[cfg(windows)]
fn symlink_file(target: &Path, dest: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(target, dest)
}

/// Index a placed file (copied OR referenced) as an artifact: poster + seconds
/// for video, capsule enrich for an element, then write-through. Shared by
/// `import_file` and `import_reference` so copy and reference land identically.
fn finalize_import(
    dest: &Path,
    kind: &str,
    source: &str,
    root: &Path,
    stem: &str,
) -> Result<Record, String> {
    let mut rec = dpl_index::record_for_capture(dest, root, kind, source);
    if kind == "element" {
        enrich_element(&mut rec, dest, root);
    }
    if kind == "video" {
        let poster = dest.with_file_name(format!(
            "{}-poster.jpg",
            dest.file_stem().and_then(|s| s.to_str()).unwrap_or(stem)
        ));
        // A missing poster degrades the card, never the import.
        if dpl_engine_media::write_poster(dest, &poster).is_ok() {
            rec.poster_rel_path = poster
                .strip_prefix(root)
                .ok()
                .map(|p| p.to_string_lossy().replace('\\', "/"));
        }
        rec.seconds = dpl_video::clip::clip_duration(dest)
            .filter(|d| *d > 0.0)
            .map(|d| d.round() as u32);
    }
    dpl_index::Index::open_default()?.upsert(&rec)?;
    Ok(rec)
}

/// PROMOTION (ADR-014, Ring 1 → Ring 2, transactional): the file enters
/// `content/` through the one import seam above — copy + poster + index
/// all COMMIT first — then the working original is removed, but ONLY
/// when it lives under `assets/`. External sources are undeletable by
/// construction; the failure direction is a duplicate, never a loss.
pub fn promote_asset(src: &Path) -> Result<Record, String> {
    let rec = import_file(src)?;
    if removable_after_promote(src, &dpl_engine_capture::assets_root()) {
        std::fs::remove_file(src).map_err(|e| format!("promote cleanup {}: {e}", src.display()))?;
    }
    Ok(rec)
}

/// The promote guard, pure for unit proof: true only for files INSIDE
/// the assets ring (canonicalized where the paths exist, so `..` and
/// symlinks can't smuggle an external source into deletability).
pub fn removable_after_promote(src: &Path, assets_root: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    canon(src).starts_with(canon(assets_root))
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

    /// A minimal, well-formed 2-page PDF (correct xref, blank pages) — the
    /// rasterize drill's fixture, no external file needed.
    fn two_page_pdf() -> Vec<u8> {
        let objs: [&[u8]; 4] = [
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>",
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] >>",
        ];
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, o) in objs.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            pdf.extend_from_slice(o);
            pdf.extend_from_slice(b"\nendobj\n");
        }
        let xref = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", objs.len() + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for off in &offsets {
            pdf.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                objs.len() + 1,
                xref
            )
            .as_bytes(),
        );
        pdf
    }

    /// A real-`pdftoppm` drill (ADR-009): a 2-page PDF explodes to 2 ORDERED
    /// page PNGs. `#[ignore]` — poppler-utils must be installed; it runs where
    /// the tool exists, exactly like the ffmpeg drills.
    #[test]
    #[ignore = "needs pdftoppm (poppler-utils) installed"]
    fn rasterize_pdf_explodes_ordered_pages() {
        let base = std::env::temp_dir().join(format!("dpl-pdf-drill-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let pdf = base.join("report.pdf");
        std::fs::write(&pdf, two_page_pdf()).unwrap();

        let pages = rasterize_pdf(&pdf, &base.join("stage"), "report").unwrap();
        assert_eq!(pages.len(), 2, "two pages: {pages:?}");
        assert!(pages.iter().all(|p| p.is_file()), "pages exist: {pages:?}");
        let names: Vec<String> = pages
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names[0] < names[1], "page order preserved: {names:?}");

        std::fs::write(base.join("nope.txt"), b"x").unwrap();
        assert!(rasterize_pdf(&base.join("nope.txt"), &base.join("stage"), "x").is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    // The reference mechanism: a single-file symlink resolves to the source
    // content IN PLACE (unix covers Linux + macOS; Windows uses its own call).
    #[cfg(unix)]
    #[test]
    fn reference_symlinks_the_source_in_place() {
        let dir = std::env::temp_dir().join(format!("dpl-ref-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("preset.png");
        std::fs::write(&src, b"PRESET-BYTES").unwrap();
        let dest = dir.join("linked.png");
        symlink_file(&std::fs::canonicalize(&src).unwrap(), &dest).unwrap();
        assert!(
            std::fs::symlink_metadata(&dest)
                .unwrap()
                .file_type()
                .is_symlink(),
            "dest is a symlink, not a copy"
        );
        assert_eq!(
            std::fs::read(&dest).unwrap(),
            b"PRESET-BYTES",
            "the symlink resolves to the source in place"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_promote_guard_only_ever_removes_inside_assets() {
        let scratch =
            std::env::temp_dir().join(format!("dpl-promote-guard-{}", std::process::id()));
        let assets = scratch.join("assets");
        std::fs::create_dir_all(assets.join("captures")).unwrap();
        let inside = assets.join("captures").join("s.png");
        std::fs::write(&inside, b"x").unwrap();
        let outside = scratch.join("elsewhere.png");
        std::fs::write(&outside, b"x").unwrap();
        assert!(removable_after_promote(&inside, &assets));
        assert!(!removable_after_promote(&outside, &assets));
        // `..` cannot smuggle an external file into deletability.
        let sneaky = assets
            .join("captures")
            .join("..")
            .join("..")
            .join("elsewhere.png");
        assert!(!removable_after_promote(&sneaky, &assets));
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn media_kind_matrix_covers_the_ratified_extensions() {
        assert_eq!(media_kind(Path::new("a.mp4")), Some("video"));
        assert_eq!(media_kind(Path::new("a.MOV")), Some("video"));
        assert_eq!(media_kind(Path::new("b.JPG")), Some("image"));
        assert_eq!(media_kind(Path::new("b.webp")), Some("image"));
        assert_eq!(media_kind(Path::new("song.WAV")), Some("audio"));
        assert_eq!(media_kind(Path::new("song.mp3")), Some("audio"));
        assert_eq!(media_kind(Path::new("gig.aep")), Some("element"));
        assert_eq!(media_kind(Path::new("title.lottie")), Some("lottie"));
        assert_eq!(media_kind(Path::new("title.JSON")), Some("lottie"));
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
