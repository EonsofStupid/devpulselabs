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

pub mod ink;

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
    FFMPEG
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("ffmpeg"))
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
            exif::Reader::new()
                .read_from_container(&mut std::io::BufReader::new(f))
                .ok()
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
        img.resize(
            MAX_IMAGE_WIDTH,
            u32::MAX,
            image::imageops::FilterType::Lanczos3,
        )
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
    let pixels: Vec<rgb::RGBA8> = rgba
        .pixels()
        .map(|p| rgb::RGBA8::new(p[0], p[1], p[2], p[3]))
        .collect();
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
    enc.encode_image(&rgb)
        .map_err(|e| format!("jpg encode: {e}"))
}

/// One output format: extension + its encoder.
type FormatArm = (
    &'static str,
    fn(&image::DynamicImage, &Path) -> Result<(), String>,
);

/// The standard ladder: `<stem>.avif` + `<stem>.webp` + `<stem>.jpg`.
pub fn optimize_image(src: &Path, out_dir: &Path) -> Result<Outputs, String> {
    run_ladder(
        src,
        out_dir,
        &[
            ("avif", encode_avif),
            ("webp", encode_webp),
            ("jpg", encode_jpeg),
        ],
    )
}

/// The cutout ladder — alpha preserved: avif + webp + PNG, never JPEG
/// ("JPEG can't hold an alpha channel" — docs/media.md).
pub fn optimize_cutout(src: &Path, out_dir: &Path) -> Result<Outputs, String> {
    fn encode_png(img: &image::DynamicImage, out: &Path) -> Result<(), String> {
        img.to_rgba8()
            .save(out)
            .map_err(|e| format!("png write: {e}"))
    }
    run_ladder(
        src,
        out_dir,
        &[
            ("avif", encode_avif),
            ("webp", encode_webp),
            ("png", encode_png),
        ],
    )
}

fn run_ladder(src: &Path, out_dir: &Path, formats: &[FormatArm]) -> Result<Outputs, String> {
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

/// One poster frame for `src`, written to `out` — the import/render
/// seams' thumbnail source (the same args the capture pipeline uses).
pub fn write_poster(src: &Path, out: &Path) -> Result<(), String> {
    run_ffmpeg(poster_args(src, out))
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

// ── the STITCH render (R1b) — concat many trimmed clips into one mp4 ─────
//
// The FINAL render tier (R1 §3): ffmpeg concatenates trimmed, uniform
// frames. Decoupled on purpose — primitive inputs, no dpl-comp dep, so the
// encode engine never learns the document schema. The Composer maps its
// Composition → these clips.

/// One trimmed source clip in a stitch render.
#[derive(Debug, Clone, PartialEq)]
pub struct StitchClip {
    pub path: PathBuf,
    pub in_pt: f32,
    pub out_pt: f32,
}

/// The concat filtergraph args to stitch `clips` end-to-end into one H.264
/// mp4 at `w`×`h`@`fps` (ADR-009: args verbatim). Each input is trimmed to
/// [in,out], PTS-reset, fps-normalized, then scaled + PADDED (never cropped
/// — the comp frame is the canvas) to a uniform frame, then `concat`. Audio
/// dropped (`-an`), faststart + crf like the single-clip `video_args`.
pub fn stitch_args(clips: &[StitchClip], w: u32, h: u32, fps: u32, out: &Path) -> Vec<String> {
    let mut args: Vec<String> = vec!["-y".into(), "-v".into(), "error".into()];
    for c in clips {
        args.push("-i".into());
        args.push(c.path.to_string_lossy().into_owned());
    }
    let mut graph = String::new();
    for (i, c) in clips.iter().enumerate() {
        let s = c.in_pt.max(0.0);
        let e = c.out_pt.max(s);
        graph.push_str(&format!(
            "[{i}:v]trim=start={s:.3}:end={e:.3},setpts=PTS-STARTPTS,fps={fps},scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1[v{i}];"
        ));
    }
    for i in 0..clips.len() {
        graph.push_str(&format!("[v{i}]"));
    }
    graph.push_str(&format!("concat=n={}:v=1:a=0[out]", clips.len()));
    args.extend([
        "-filter_complex".into(),
        graph,
        "-map".into(),
        "[out]".into(),
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
    ]);
    args
}

/// Render a stitch to `out` (creating its parent). Honest errors: no clips,
/// or ffmpeg failure. Blocks — callers run it off the UI thread.
pub fn render_stitch(
    clips: &[StitchClip],
    w: u32,
    h: u32,
    fps: u32,
    out: &Path,
) -> Result<(), String> {
    if clips.is_empty() {
        return Err("nothing to render — the composition is empty".into());
    }
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("render dir: {e}"))?;
    }
    run_ffmpeg(stitch_args(clips, w, h, fps, out))
}

// ── the GUIDE renders (W5) — the light replay + the standalone video ─────

/// The "incredibly light" session-replay re-encode: screen capture
/// compresses extremely well, so the bundled copy drops to ≤720p / 15fps /
/// crf 30 — phone-friendly, embeddable, never the original file.
pub const REPLAY_MAX_HEIGHT: u32 = 720;
pub const REPLAY_FPS: u32 = 15;
pub const REPLAY_CRF: u32 = 30;

pub fn replay_args(src: &Path, out: &Path) -> Vec<String> {
    vec![
        "-y".into(),
        "-v".into(),
        "error".into(),
        "-i".into(),
        src.to_string_lossy().into_owned(),
        "-vf".into(),
        format!("scale=-2:'min({REPLAY_MAX_HEIGHT},ih)',fps={REPLAY_FPS}"),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        REPLAY_CRF.to_string(),
        "-movflags".into(),
        "+faststart".into(),
        "-an".into(),
        out.to_string_lossy().into_owned(),
    ]
}

/// Re-encode a session recording into its light bundle copy (NEW file,
/// source untouched). Blocks — callers run it off the UI thread.
pub fn render_replay(src: &Path, out: &Path) -> Result<(), String> {
    if !src.is_file() {
        return Err(format!("no session video at {}", src.display()));
    }
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("replay dir: {e}"))?;
    }
    run_ffmpeg(replay_args(src, out))
}

/// The standalone guide VIDEO: every still held `secs_per_still`, then the
/// replay footage, concatenated through the same scale+pad chain as the
/// stitch (never cropped). Stills decode as RGBA — `format=yuv420p` keeps
/// every leg encodable.
pub fn guide_video_args(
    stills: &[PathBuf],
    secs_per_still: f32,
    session: Option<&Path>,
    w: u32,
    h: u32,
    fps: u32,
    out: &Path,
) -> Vec<String> {
    let mut args: Vec<String> = vec!["-y".into(), "-v".into(), "error".into()];
    for s in stills {
        args.extend([
            "-loop".into(),
            "1".into(),
            "-t".into(),
            format!("{secs_per_still:.3}"),
            "-i".into(),
            s.to_string_lossy().into_owned(),
        ]);
    }
    if let Some(v) = session {
        args.push("-i".into());
        args.push(v.to_string_lossy().into_owned());
    }
    let n = stills.len() + usize::from(session.is_some());
    let mut graph = String::new();
    for i in 0..n {
        graph.push_str(&format!(
            "[{i}:v]fps={fps},scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,format=yuv420p[v{i}];"
        ));
    }
    for i in 0..n {
        graph.push_str(&format!("[v{i}]"));
    }
    graph.push_str(&format!("concat=n={n}:v=1:a=0[out]"));
    args.extend([
        "-filter_complex".into(),
        graph,
        "-map".into(),
        "[out]".into(),
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
    ]);
    args
}

/// Render the standalone guide video. Honest errors: no stills, or ffmpeg
/// failure. Blocks — callers run it off the UI thread.
pub fn render_guide_video(
    stills: &[PathBuf],
    secs_per_still: f32,
    session: Option<&Path>,
    w: u32,
    h: u32,
    fps: u32,
    out: &Path,
) -> Result<(), String> {
    if stills.is_empty() {
        return Err("nothing to render — the guide has no stills".into());
    }
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("render dir: {e}"))?;
    }
    run_ffmpeg(guide_video_args(
        stills,
        secs_per_still,
        session,
        w,
        h,
        fps,
        out,
    ))
}

#[cfg(test)]
mod guide_render_tests {
    use super::*;

    #[test]
    fn replay_args_are_light_and_faststarted() {
        let a = replay_args(Path::new("session.mp4"), Path::new("out.mp4")).join(" ");
        assert!(a.contains("scale=-2:'min(720,ih)',fps=15"), "{a}");
        assert!(a.contains("-c:v libx264 -preset veryfast -crf 30"), "{a}");
        assert!(a.contains("-movflags +faststart -an out.mp4"), "{a}");
    }

    #[test]
    fn guide_video_args_hold_stills_then_append_the_replay() {
        let stills = vec![
            PathBuf::from("step-001.webp"),
            PathBuf::from("step-002.webp"),
        ];
        let a = guide_video_args(
            &stills,
            3.0,
            Some(Path::new("session.mp4")),
            1280,
            720,
            30,
            Path::new("guide-video.mp4"),
        )
        .join(" ");
        assert!(a.contains("-loop 1 -t 3.000 -i step-001.webp"), "{a}");
        assert!(a.contains("-loop 1 -t 3.000 -i step-002.webp"), "{a}");
        assert!(a.contains("-i session.mp4"), "{a}");
        assert!(a.contains("concat=n=3:v=1:a=0[out]"), "{a}");
        assert!(a.contains("format=yuv420p[v0]"), "{a}");
        assert!(
            a.contains("scale=1280:720:force_original_aspect_ratio=decrease"),
            "{a}"
        );
        assert!(
            a.contains("-map [out] -c:v libx264 -preset slow -crf 21"),
            "{a}"
        );
    }

    #[test]
    fn guide_video_without_a_session_still_concats() {
        let stills = vec![PathBuf::from("s1.webp")];
        let a = guide_video_args(&stills, 2.5, None, 1280, 720, 30, Path::new("o.mp4")).join(" ");
        assert!(a.contains("-t 2.500"), "{a}");
        assert!(a.contains("concat=n=1:v=1:a=0[out]"), "{a}");
        assert!(!a.contains("session"), "{a}");
    }

    #[test]
    fn guide_render_refuses_no_stills_and_replay_refuses_a_missing_source() {
        assert!(render_guide_video(&[], 3.0, None, 1280, 720, 30, Path::new("x.mp4")).is_err());
        assert!(render_replay(Path::new("definitely-missing.mp4"), Path::new("o.mp4")).is_err());
    }
}

#[cfg(test)]
mod stitch_tests {
    use super::*;

    #[test]
    fn stitch_args_trims_scales_pads_and_concats() {
        let clips = vec![
            StitchClip {
                path: "a.mp4".into(),
                in_pt: 0.0,
                out_pt: 6.0,
            },
            StitchClip {
                path: "b.mp4".into(),
                in_pt: 1.0,
                out_pt: 9.0,
            },
        ];
        let a = stitch_args(&clips, 1920, 1080, 30, Path::new("out.mp4")).join(" ");
        assert!(a.contains("-i a.mp4"), "{a}");
        assert!(a.contains("-i b.mp4"), "{a}");
        assert!(a.contains("trim=start=0.000:end=6.000"), "{a}");
        assert!(a.contains("trim=start=1.000:end=9.000"), "{a}");
        assert!(
            a.contains("scale=1920:1080:force_original_aspect_ratio=decrease"),
            "{a}"
        );
        assert!(a.contains("pad=1920:1080:(ow-iw)/2:(oh-ih)/2"), "{a}");
        assert!(a.contains("concat=n=2:v=1:a=0[out]"), "{a}");
        assert!(
            a.contains("-map [out] -c:v libx264 -preset slow -crf 21"),
            "{a}"
        );
        assert!(a.contains("-movflags +faststart -an out.mp4"), "{a}");
    }

    #[test]
    fn a_single_clip_still_concats_cleanly() {
        let clips = vec![StitchClip {
            path: "solo.mp4".into(),
            in_pt: 2.0,
            out_pt: 5.0,
        }];
        let a = stitch_args(&clips, 1280, 720, 24, Path::new("o.mp4")).join(" ");
        assert!(a.contains("concat=n=1:v=1:a=0[out]"), "{a}");
        assert!(a.contains("[v0]"), "{a}");
    }

    #[test]
    fn render_refuses_an_empty_stitch() {
        assert!(render_stitch(&[], 1920, 1080, 30, Path::new("x.mp4")).is_err());
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
        (
            out_dir.join(format!("{stem}.{main_ext}")),
            main_args(src, &out_dir.join(format!("{stem}.{main_ext}"))),
        ),
        (
            out_dir.join(format!("{stem}-poster.jpg")),
            poster_args(src, &out_dir.join(format!("{stem}-poster.jpg"))),
        ),
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
        walk(
            &cutout,
            &generated.join("cutout"),
            &mut report,
            Mode::Cutout,
        )?;
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
                && matches!(
                    path.file_name().and_then(|n| n.to_str()),
                    Some("cutout" | "anim")
                )
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
        assert!(
            round.pixels().any(|p| p[3] == 0),
            "alpha survived the encode"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ffmpeg_args_are_the_old_scripts_verbatim() {
        let a = anim_args(Path::new("in.mp4"), Path::new("out.webp")).join(" ");
        assert!(a.contains("fps=24"), "{a}");
        assert!(
            a.contains("-c:v libwebp -lossless 0 -q:v 70 -loop 0 -an"),
            "{a}"
        );
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
