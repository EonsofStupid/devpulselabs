//! The content media-optimization pipeline — the old `optimize-media.mjs`
//! (sharp + ffmpeg), ported faithfully to pure Rust. Every constant is
//! verbatim from the old script; the outputs are the same three-format
//! ladder the old `<picture>` element consumed (AVIF first, WebP second,
//! JPEG always-works), plus the alpha-preserving cutout set and the
//! ffmpeg-subprocess anim/video paths (ADR-009: ffmpeg is NEVER a crate
//! dep).
//!
//! Laws carried: idempotent (outputs newer than their source are skipped),
//! metadata stripped by re-encode after the EXIF-orientation bake, sources
//! never touched, outputs land beside-tree in `generated/`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

// ── the constants, verbatim from optimize-media.mjs ─────────────────────
pub const MAX_IMAGE_WIDTH: u32 = 2560;
pub const AVIF_QUALITY: f32 = 60.0;
pub const WEBP_QUALITY: f32 = 80.0;
pub const JPEG_QUALITY: u8 = 82;
pub const VIDEO_MAX_HEIGHT: u32 = 1080;
pub const VIDEO_CRF: u32 = 21;
pub const ANIM_FPS: u32 = 24;
pub const ANIM_QUALITY: u32 = 70;

static FFMPEG: OnceLock<PathBuf> = OnceLock::new();

/// Managed ffmpeg injection — same contract as the other engines. Before
/// injection, `ffmpeg` from PATH (the dev fallback).
pub fn set_ffmpeg_path(p: PathBuf) {
    let _ = FFMPEG.set(p);
}

fn ffmpeg_program() -> PathBuf {
    FFMPEG.get().cloned().unwrap_or_else(|| PathBuf::from("ffmpeg"))
}

/// Freshness law: skip when the output exists and is at least as new as the
/// source (the old `isFresh`).
pub fn is_fresh(src: &Path, out: &Path) -> bool {
    let (Ok(s), Ok(o)) = (std::fs::metadata(src), std::fs::metadata(out)) else {
        return false;
    };
    match (s.modified(), o.modified()) {
        (Ok(sm), Ok(om)) => om >= sm,
        _ => false,
    }
}

/// What one source produced (paths that exist when the call returns).
#[derive(Debug, Default)]
pub struct Outputs {
    pub written: Vec<PathBuf>,
    pub skipped: Vec<PathBuf>,
}

// ── the image ladder ────────────────────────────────────────────────────

/// EXIF orientation → the baked-in transform (sharp's `.rotate()` with no
/// args). Unknown/missing orientation is a no-op.
fn bake_orientation(img: image::DynamicImage, src: &Path) -> image::DynamicImage {
    let orientation = std::fs::File::open(src)
        .ok()
        .and_then(|f| {
            exif::Reader::new().read_from_container(&mut std::io::BufReader::new(f)).ok()
        })
        .and_then(|ex| {
            ex.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
                .and_then(|f| f.value.get_uint(0))
        })
        .unwrap_or(1);
    match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

fn load_capped(src: &Path) -> Result<image::DynamicImage, String> {
    let img = image::open(src).map_err(|e| format!("decode {}: {e}", src.display()))?;
    let img = bake_orientation(img, src);
    // `withoutEnlargement: true` — only shrink, never upscale.
    Ok(if img.width() > MAX_IMAGE_WIDTH {
        img.resize(MAX_IMAGE_WIDTH, u32::MAX, image::imageops::FilterType::Lanczos3)
    } else {
        img
    })
}

fn encode_webp(img: &image::DynamicImage, out: &Path) -> Result<(), String> {
    let rgba = img.to_rgba8();
    let enc = webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height());
    let data = enc.encode(WEBP_QUALITY);
    std::fs::write(out, &*data).map_err(|e| format!("webp write: {e}"))
}

fn encode_avif(img: &image::DynamicImage, out: &Path) -> Result<(), String> {
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let pixels: Vec<rgb::RGBA8> =
        rgba.pixels().map(|p| rgb::RGBA8::new(p[0], p[1], p[2], p[3])).collect();
    let encoded = ravif::Encoder::new()
        .with_quality(AVIF_QUALITY)
        .with_alpha_quality(AVIF_QUALITY)
        // Effort/speed comparable to sharp's default libaom effort; quality
        // is the contract, this only trades encode time.
        .with_speed(6)
        .encode_rgba(ravif::Img::new(&pixels, w, h))
        .map_err(|e| format!("avif encode: {e}"))?;
    std::fs::write(out, encoded.avif_file).map_err(|e| format!("avif write: {e}"))
}

fn encode_jpeg(img: &image::DynamicImage, out: &Path) -> Result<(), String> {
    // JPEG has no alpha — composite is the caller's concern (cutouts never
    // come here). Re-encode strips metadata, same as sharp.
    let rgb = img.to_rgb8();
    let f = std::fs::File::create(out).map_err(|e| format!("jpg create: {e}"))?;
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(
        std::io::BufWriter::new(f),
        JPEG_QUALITY,
    );
    enc.encode_image(&rgb).map_err(|e| format!("jpg encode: {e}"))
}

/// The standard ladder: `<stem>.avif` + `<stem>.webp` + `<stem>.jpg`.
pub fn optimize_image(src: &Path, out_dir: &Path) -> Result<Outputs, String> {
    run_ladder(
        src,
        out_dir,
        &[
            ("avif", encode_avif as fn(&image::DynamicImage, &Path) -> Result<(), String>),
            ("webp", encode_webp),
            ("jpg", encode_jpeg),
        ],
    )
}

/// The cutout ladder — alpha preserved: avif + webp + PNG, never JPEG
/// ("JPEG can't hold an alpha channel" — docs/media.md).
pub fn optimize_cutout(src: &Path, out_dir: &Path) -> Result<Outputs, String> {
    fn encode_png(img: &image::DynamicImage, out: &Path) -> Result<(), String> {
        img.to_rgba8().save(out).map_err(|e| format!("png write: {e}"))
    }
    run_ladder(src, out_dir, &[("avif", encode_avif), ("webp", encode_webp), ("png", encode_png)])
}

fn run_ladder(
    src: &Path,
    out_dir: &Path,
    formats: &[(&str, fn(&image::DynamicImage, &Path) -> Result<(), String>)],
) -> Result<Outputs, String> {
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("no stem: {}", src.display()))?;
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;

    let mut outs = Outputs::default();
    let targets: Vec<(PathBuf, _)> = formats
        .iter()
        .map(|(ext, f)| (out_dir.join(format!("{stem}.{ext}")), *f))
        .collect();

    // Decode once, only if anything is stale.
    if targets.iter().all(|(p, _)| is_fresh(src, p)) {
        outs.skipped = targets.into_iter().map(|(p, _)| p).collect();
        return Ok(outs);
    }
    let img = load_capped(src)?;
    for (out, encode) in targets {
        if is_fresh(src, &out) {
            outs.skipped.push(out);
            continue;
        }
        encode(&img, &out)?;
        outs.written.push(out);
    }
    Ok(outs)
}

// ── the ffmpeg paths (subprocess, args verbatim) ────────────────────────

/// Animation source video → looping animated WebP + `<stem>-poster.jpg`.
pub fn anim_args(src: &Path, out: &Path) -> Vec<String> {
    vec![
        "-y".into(),
        "-v".into(),
        "error".into(),
        "-i".into(),
        src.to_string_lossy().into_owned(),
        "-vf".into(),
        format!("fps={ANIM_FPS},scale=-2:'min({VIDEO_MAX_HEIGHT},ih)'"),
        "-c:v".into(),
        "libwebp".into(),
        "-lossless".into(),
        "0".into(),
        "-q:v".into(),
        ANIM_QUALITY.to_string(),
        "-loop".into(),
        "0".into(),
        "-an".into(),
        out.to_string_lossy().into_owned(),
    ]
}

/// Video source → H.264 mp4 (slow/crf21/faststart, ≤1080p).
pub fn video_args(src: &Path, out: &Path) -> Vec<String> {
    vec![
        "-y".into(),
        "-v".into(),
        "error".into(),
        "-i".into(),
        src.to_string_lossy().into_owned(),
        "-vf".into(),
        format!("scale=-2:'min({VIDEO_MAX_HEIGHT},ih)'"),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "slow".into(),
        "-crf".into(),
        VIDEO_CRF.to_string(),
        "-movflags".into(),
        "+faststart".into(),
        "-an".into(),
        out.to_string_lossy().into_owned(),
    ]
}

fn poster_args(src: &Path, out: &Path) -> Vec<String> {
    vec![
        "-y".into(),
        "-v".into(),
        "error".into(),
        "-i".into(),
        src.to_string_lossy().into_owned(),
        "-frames:v".into(),
        "1".into(),
        out.to_string_lossy().into_owned(),
    ]
}

fn run_ffmpeg(args: Vec<String>) -> Result<(), String> {
    let status = Command::new(ffmpeg_program())
        .args(&args)
        .status()
        .map_err(|e| format!("ffmpeg spawn: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("ffmpeg exited {status}"))
    }
}

/// Anim: `<stem>.webp` (animated) + `<stem>-poster.jpg`, freshness-skipped.
pub fn optimize_anim(src: &Path, out_dir: &Path) -> Result<Outputs, String> {
    ffmpeg_pair(src, out_dir, "webp", anim_args)
}

/// Video: `<stem>.mp4` + `<stem>-poster.jpg`, freshness-skipped.
pub fn optimize_video(src: &Path, out_dir: &Path) -> Result<Outputs, String> {
    ffmpeg_pair(src, out_dir, "mp4", video_args)
}

fn ffmpeg_pair(
    src: &Path,
    out_dir: &Path,
    main_ext: &str,
    main_args: fn(&Path, &Path) -> Vec<String>,
) -> Result<Outputs, String> {
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("no stem: {}", src.display()))?;
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let mut outs = Outputs::default();
    for (path, args) in [
        (out_dir.join(format!("{stem}.{main_ext}")), main_args(src, &out_dir.join(format!("{stem}.{main_ext}")))),
        (out_dir.join(format!("{stem}-poster.jpg")), poster_args(src, &out_dir.join(format!("{stem}-poster.jpg")))),
    ] {
        if is_fresh(src, &path) {
            outs.skipped.push(path);
            continue;
        }
        run_ffmpeg(args)?;
        outs.written.push(path);
    }
    Ok(outs)
}

// ── the tree walk ───────────────────────────────────────────────────────

const IMAGE_EXTS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
const VIDEO_EXTS: [&str; 3] = ["mp4", "mov", "webm"];

/// One source directory → its `generated` sibling (the old episode layout:
/// `assets/source` → `assets/generated`, with `source/cutout/*` keeping
/// alpha and `source/anim/*` becoming animated WebP). Errors on single
/// files are collected, not fatal — one bad asset never kills the run.
pub fn optimize_dir(source: &Path, generated: &Path) -> Result<Report, String> {
    let mut report = Report::default();
    walk(source, generated, &mut report, Mode::Standard)?;
    let cutout = source.join("cutout");
    if cutout.is_dir() {
        walk(&cutout, &generated.join("cutout"), &mut report, Mode::Cutout)?;
    }
    let anim = source.join("anim");
    if anim.is_dir() {
        walk(&anim, &generated.join("anim"), &mut report, Mode::Anim)?;
    }
    Ok(report)
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Standard,
    Cutout,
    Anim,
}

#[derive(Debug, Default)]
pub struct Report {
    pub written: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

fn walk(dir: &Path, out: &Path, report: &mut Report, mode: Mode) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // cutout/ and anim/ are handled by optimize_dir at the top level.
            if mode == Mode::Standard
                && matches!(path.file_name().and_then(|n| n.to_str()), Some("cutout" | "anim"))
            {
                continue;
            }
            walk(&path, &out.join(entry.file_name()), report, mode)?;
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        let result = match mode {
            Mode::Cutout if IMAGE_EXTS.contains(&ext.as_str()) => optimize_cutout(&path, out),
            Mode::Anim if VIDEO_EXTS.contains(&ext.as_str()) => optimize_anim(&path, out),
            Mode::Standard if IMAGE_EXTS.contains(&ext.as_str()) => optimize_image(&path, out),
            Mode::Standard if VIDEO_EXTS.contains(&ext.as_str()) => optimize_video(&path, out),
            _ => continue,
        };
        match result {
            Ok(o) => {
                report.written += o.written.len();
                report.skipped += o.skipped.len();
            }
            Err(e) => report.errors.push(e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("dpl-engine-media-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn small_png(dir: &Path, name: &str, alpha: bool) -> PathBuf {
        let path = dir.join(name);
        if alpha {
            let mut img = image::RgbaImage::new(64, 48);
            for (x, y, p) in img.enumerate_pixels_mut() {
                *p = image::Rgba([x as u8 * 4, y as u8 * 5, 90, if x > 32 { 0 } else { 255 }]);
            }
            img.save(&path).unwrap();
        } else {
            let img = image::RgbImage::from_fn(64, 48, |x, y| {
                image::Rgb([x as u8 * 4, y as u8 * 5, 120])
            });
            img.save(&path).unwrap();
        }
        path
    }

    #[test]
    fn the_ladder_writes_all_three_formats_then_skips() {
        let root = temp_root("ladder");
        let src = small_png(&root, "shot.png", false);
        let out = root.join("generated");

        let first = optimize_image(&src, &out).unwrap();
        assert_eq!(first.written.len(), 3, "avif+webp+jpg: {first:?}");
        for ext in ["avif", "webp", "jpg"] {
            let p = out.join(format!("shot.{ext}"));
            assert!(p.is_file(), "missing {}", p.display());
            assert!(std::fs::metadata(&p).unwrap().len() > 0);
        }

        // The freshness law: a second run touches nothing.
        let second = optimize_image(&src, &out).unwrap();
        assert!(second.written.is_empty(), "idempotent: {second:?}");
        assert_eq!(second.skipped.len(), 3);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cutouts_keep_alpha_and_never_get_a_jpeg() {
        let root = temp_root("cutout");
        let src = small_png(&root, "cut.png", true);
        let out = root.join("generated");
        optimize_cutout(&src, &out).unwrap();
        assert!(out.join("cut.avif").is_file());
        assert!(out.join("cut.webp").is_file());
        assert!(out.join("cut.png").is_file());
        assert!(!out.join("cut.jpg").exists(), "JPEG can't hold alpha");
        // The webp round-trips with transparency intact.
        let round = image::open(out.join("cut.webp")).unwrap().to_rgba8();
        assert!(round.pixels().any(|p| p[3] == 0), "alpha survived the encode");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ffmpeg_args_are_the_old_scripts_verbatim() {
        let a = anim_args(Path::new("in.mp4"), Path::new("out.webp")).join(" ");
        assert!(a.contains("fps=24"), "{a}");
        assert!(a.contains("-c:v libwebp -lossless 0 -q:v 70 -loop 0 -an"), "{a}");
        let v = video_args(Path::new("in.mp4"), Path::new("out.mp4")).join(" ");
        assert!(v.contains("-c:v libx264 -preset slow -crf 21"), "{v}");
        assert!(v.contains("+faststart"), "{v}");
        assert!(v.contains("min(1080,ih)"), "{v}");
    }

    #[test]
    fn the_tree_walk_routes_cutout_and_skips_unknown() {
        let root = temp_root("tree");
        let source = root.join("source");
        std::fs::create_dir_all(source.join("cutout")).unwrap();
        small_png(&source, "plain.png", false);
        small_png(&source.join("cutout"), "sticker.png", true);
        std::fs::write(source.join("notes.txt"), "not media").unwrap();

        let report = optimize_dir(&source, &root.join("generated")).unwrap();
        assert_eq!(report.errors.len(), 0, "{:?}", report.errors);
        assert_eq!(report.written, 6, "3 standard + 3 cutout");
        assert!(root.join("generated/plain.jpg").is_file());
        assert!(root.join("generated/cutout/sticker.png").is_file());
        assert!(!root.join("generated/cutout/sticker.jpg").exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}
