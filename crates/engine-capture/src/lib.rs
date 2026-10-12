//! Screen capture → ffmpeg → episode content tree. Engine A (xcap native
//! recorder + constant-fps pacing + ffmpeg x264 ultrafast), verified by the
//! Pass R spike (RGBA byte order confirmed; 30fps real-time on Windows).
//!
//! Stage 2 (Pass S2) adds region capture: the same full-monitor recorder, with
//! each frame cropped in Rust to a selected rect before encoding. The recorder
//! path itself is unchanged — only a per-frame crop is added.
//!
//! Portable by construction: episode resolution goes through `CONTENT_ROOT`
//! (default `./content`), ffmpeg is a MANAGED binary injected once via
//! `set_ffmpeg_path` (the app shell resolves the verified private payload),
//! all output paths
//! use forward slashes. Linux X11 is expected-good; Wayland is the known gap
//! (docs/screen-capture.md, open item #18).
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use xcap::{Frame, Monitor, VideoRecorder};

#[derive(Serialize, Clone)]
pub struct CaptureResult {
    pub ok: bool,
    /// Episode-relative path (forward-slash) to the written clip.
    pub path: Option<String>,
    pub error: Option<String>,
}

/// A pixel-space crop rectangle within a monitor frame.
#[derive(Clone, Copy)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// A fractional (0..1) rectangle as reported by the overlay — DPI-proof: the
/// overlay never sends pixels, only fractions of the screen it covers.
#[derive(Clone, Copy)]
pub struct FracRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

fn err(m: impl Into<String>) -> CaptureResult {
    CaptureResult {
        ok: false,
        path: None,
        error: Some(m.into()),
    }
}

/// The managed ffmpeg binary. Injected exactly once by the embedding shell
/// (app: verified `dptools-deps`; capture-cli: explicit exe-adjacent input).
/// An uninjected engine fails honestly and never searches PATH.
static FFMPEG: OnceLock<PathBuf> = OnceLock::new();
const MISSING_FFMPEG: &str = "__devpulse_embedded_ffmpeg_not_configured__";

/// Idempotent: the first caller wins; later calls are ignored.
pub fn set_ffmpeg_path(p: PathBuf) {
    let _ = FFMPEG.set(p);
}

/// The one Command factory every ffmpeg spawn in this crate goes through.
fn ffmpeg_cmd() -> Command {
    let mut cmd = Command::new(
        FFMPEG
            .get()
            .map(PathBuf::as_path)
            .unwrap_or_else(|| Path::new(MISSING_FFMPEG)),
    );
    // The quiet-spawn law (W8): no ffmpeg child ever pops a console.
    dpl_proc::quiet(&mut cmd);
    cmd
}

/// For error messages: where ffmpeg was expected.
fn ffmpeg_display() -> String {
    FFMPEG.get().map_or_else(
        || "verified embedded ffmpeg (not configured)".into(),
        |p| p.display().to_string(),
    )
}

/// The ONE user-data folder — everything the app reads AND writes (inbox,
/// elements, shows, artifacts.db) lives under this root (operator-ratified:
/// one organized folder, truly portable). The resolution law is the
/// original app's `content.rs`, ported: turnkey from ANY launch context —
/// a double-clicked exe must find the SAME folder a repo run uses, never
/// scatter a fresh `content/` into whatever cwd it woke up in.
///
/// ① `--content-root <path>` CLI arg — explicit beats ambient, authoritative
///    even when the folder doesn't exist yet (drills mint fresh roots).
/// ② `CONTENT_ROOT` env — the drill/test harness's spawn-time seam, same
///    authority (existing harness contract, kept).
/// ③ ambient candidates, first that EXISTS wins:
///    `content/` beside the exe (the portable anchor) → `content/` walking
///    UP from the exe dir (≤4 levels — finds the repo root from
///    `target/release/`) → `./content` of the launch cwd.
/// ④ nothing exists → the exe-adjacent candidate (a fresh portable folder
///    self-creates BESIDE THE EXE, never scattered by cwd).
///
/// Absolutized + normalized, resolved ONCE per process. `checked_roots()`
/// is the honesty surface: every location tried, in order, so boot can SAY
/// where it looked (the original's content_status law).
pub fn content_root() -> PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(resolve_content_root).clone()
}

fn resolve_content_root() -> PathBuf {
    if let Some(explicit) = explicit_root() {
        return normalize(&absolutize(&explicit));
    }
    let ambient = ambient_candidates();
    let chosen = ambient
        .iter()
        .find(|p| p.exists())
        .or(ambient.first())
        .cloned()
        .unwrap_or_else(|| PathBuf::from("./content"));
    normalize(&absolutize(&chosen))
}

/// `--content-root <path>` / `--content-root=<path>` from the process args
/// (a CLI arg is explicit and discoverable), else the `CONTENT_ROOT` env
/// seam the harness already speaks.
fn explicit_root() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--content-root" {
            return args.next().map(PathBuf::from);
        }
        if let Some(v) = a.strip_prefix("--content-root=") {
            return Some(PathBuf::from(v));
        }
    }
    std::env::var("CONTENT_ROOT").ok().map(PathBuf::from)
}

/// The ambient candidates, in order: exe-adjacent, the exe's parents
/// (≤4 up), then the launch cwd.
fn ambient_candidates() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        v.push(dir.join("content"));
        let mut up = dir.clone();
        for _ in 0..4 {
            let Some(parent) = up.parent().map(Path::to_path_buf) else {
                break;
            };
            v.push(parent.join("content"));
            up = parent;
        }
    }
    v.push(PathBuf::from("./content"));
    v
}

/// Every location resolution would try, in order, display-normalized —
/// boot logs this so "no content found" always says where it looked.
pub fn checked_roots() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(p) = explicit_root() {
        out.push(display_path(&p));
    }
    out.extend(ambient_candidates().iter().map(|p| display_path(p)));
    out
}

// ── the three-ring root (ADR-014) ───────────────────────────────────────
//
// Ring 2 is `content_root()` above (CONFIRMED — the library universe).
// Ring 1 `assets/` (WORKING — auto-filed captures and in-process
// material) and Ring 0 `.userdata/` (STATE — index, cache, temp,
// profiles) resolve as its SIBLINGS, so the portable unit is the app
// folder: exe + content + assets + .userdata. Drill seams ASSETS_ROOT /
// USERDATA_ROOT mirror the CONTENT_ROOT contract.

/// Ring 1 — the WORKING boundary. Generation lands here automatically;
/// promotion into `content/` is always an explicit gesture (ADR-014 §1).
pub fn assets_root() -> PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| resolve_ring("ASSETS_ROOT", "assets"))
        .clone()
}

/// Ring 0 — app STATE: `index.db`, `cache/`, `temp/`, `profiles/`.
/// `cache/` and `temp/` are 100% regenerable; the rest is precious.
pub fn userdata_root() -> PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| resolve_ring("USERDATA_ROOT", ".userdata"))
        .clone()
}

fn resolve_ring(env: &str, name: &str) -> PathBuf {
    if let Ok(explicit) = std::env::var(env) {
        return normalize(&absolutize(Path::new(&explicit)));
    }
    ring_sibling(&content_root(), name)
}

/// The pure derivation: a ring is the content root's SIBLING; a rootless
/// content root (drive root — degenerate) nests the ring inside itself.
/// Public so root-parameterized tools (dpl-index-cli's `--root`) derive
/// rings for ARBITRARY content roots the same way the app does.
pub fn ring_sibling(content: &Path, name: &str) -> PathBuf {
    content
        .parent()
        .map(|p| p.join(name))
        .unwrap_or_else(|| content.join(name))
}

/// The auto-filing law: `assets/captures/<kind>/<job>/`, created.
/// Shallow by design — context (kind/job) beats deep taxonomies.
pub fn assets_capture_dir(kind: &str, job: &str) -> Result<PathBuf, String> {
    let dir = assets_root().join("captures").join(kind).join(job);
    std::fs::create_dir_all(&dir).map_err(|e| format!("capture dir {}: {e}", dir.display()))?;
    Ok(dir)
}

/// What `ensure_rings` established — boot logs it (the honesty law).
#[derive(Debug)]
pub struct Rings {
    pub content: PathBuf,
    pub assets: PathBuf,
    pub userdata: PathBuf,
}

const USERDATA_README: &str = "DevPULSE Labs app state (.userdata) — ADR-014.\n\
cache\\ and temp\\ are always safe to delete (fully regenerable).\n\
index.db and profiles\\ are precious: deleting them loses the library\n\
catalog and saved sign-in sessions.\n";

/// Create the three rings (idempotent) and Ring 0's deletability README.
pub fn ensure_rings() -> Result<Rings, String> {
    let rings = Rings {
        content: content_root(),
        assets: assets_root(),
        userdata: userdata_root(),
    };
    for dir in [
        rings.content.clone(),
        rings.assets.clone(),
        rings.userdata.join("cache"),
        rings.userdata.join("temp"),
        rings.userdata.join("profiles"),
    ] {
        std::fs::create_dir_all(&dir).map_err(|e| format!("ring {}: {e}", dir.display()))?;
    }
    let readme = rings.userdata.join("README.txt");
    if !readme.exists() {
        std::fs::write(&readme, USERDATA_README).map_err(|e| format!("ring readme: {e}"))?;
    }
    Ok(rings)
}

fn absolutize(p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|c| c.join(p))
            .unwrap_or_else(|_| p.to_path_buf())
    }
}

/// Fold `.`/`..` components so paths never display as `C:/./content`.
fn normalize(p: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

fn display_path(p: &Path) -> String {
    normalize(&absolutize(p))
        .to_string_lossy()
        .replace('\\', "/")
}

/// Resolve an episode dir by id or `id-slug` folder prefix (mirrors
/// scripts/capture-replay.mjs).
fn find_episode_dir(episode_id: &str) -> Option<PathBuf> {
    let shows = content_root().join("shows");
    for show in std::fs::read_dir(&shows).ok()?.flatten() {
        let eps = show.path().join("episodes");
        if let Ok(list) = std::fs::read_dir(&eps) {
            for ep in list.flatten() {
                let name = ep.file_name().to_string_lossy().to_string();
                if name == episode_id || name.starts_with(&format!("{episode_id}-")) {
                    return Some(ep.path());
                }
            }
        }
    }
    None
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Never clobber an existing clip: `<name>.mp4`, then `<name>-2.mp4`, … The
/// poster path mirrors the chosen stem.
fn unique_stem(dir: &Path, name: &str) -> String {
    if !dir.join(format!("{name}.mp4")).exists() {
        return name.to_string();
    }
    let mut n = 2u32;
    loop {
        let candidate = format!("{name}-{n}");
        if !dir.join(format!("{candidate}.mp4")).exists() {
            return candidate;
        }
        n += 1;
    }
}

/// Copy `crop.h` rows of `crop.w*4` RGBA bytes out of a full-monitor frame
/// (row stride = `full_w*4`) into `dst`. Bounds-guarded: a short frame is
/// reported so the caller can skip it rather than emit garbage.
fn crop_frame(full: &[u8], full_w: u32, crop: &CropRect, dst: &mut [u8]) -> bool {
    let row_bytes = (crop.w * 4) as usize;
    for row in 0..crop.h {
        let src = (((crop.y + row) * full_w + crop.x) * 4) as usize;
        let dstoff = (row * crop.w * 4) as usize;
        if src + row_bytes > full.len() || dstoff + row_bytes > dst.len() {
            return false;
        }
        dst[dstoff..dstoff + row_bytes].copy_from_slice(&full[src..src + row_bytes]);
    }
    true
}

/// ffmpeg's flags for a recording: errors only — no banner, no progress — so
/// the stderr we keep is the reason a recording failed and nothing else.
const FFMPEG_ERRORS_ONLY: [&str; 4] = ["-hide_banner", "-nostats", "-loglevel", "error"];

/// How many of ffmpeg's last stderr lines a recording keeps.
const STDERR_TAIL_LINES: usize = 64;

/// The last lines a child wrote to stderr, drained on its own thread so an
/// encoder can never stall on a full pipe.
struct StderrTail(Option<std::thread::JoinHandle<Vec<String>>>);

impl StderrTail {
    fn drain(child: &mut Child) -> Self {
        let Some(err) = child.stderr.take() else {
            return Self(None);
        };
        let handle = std::thread::Builder::new()
            .name("ffmpeg-stderr".into())
            .spawn(move || {
                let mut tail = VecDeque::with_capacity(STDERR_TAIL_LINES);
                for line in BufReader::new(err).lines().map_while(Result::ok) {
                    if tail.len() == STDERR_TAIL_LINES {
                        tail.pop_front();
                    }
                    tail.push_back(line);
                }
                tail.into_iter().collect()
            })
            .ok();
        Self(handle)
    }

    /// Everything kept, once the child has exited and closed its stderr.
    fn text(self) -> String {
        self.0
            .and_then(|h| h.join().ok())
            .map(|lines| lines.join("\n"))
            .unwrap_or_default()
    }
}

/// `what`, plus ffmpeg's own last word on it when it said one.
fn with_reason(what: &str, stderr: &str) -> String {
    match stderr.lines().rev().map(str::trim).find(|l| !l.is_empty()) {
        Some(last) => format!("{what}: {last}"),
        None => what.to_string(),
    }
}

/// The streams `ffmpeg -i` reports for a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MediaStreams {
    pub video: bool,
    pub audio: bool,
}

/// Read the `Stream #…: Video:` / `Stream #…: Audio:` lines of `ffmpeg -i`.
pub fn parse_streams(text: &str) -> MediaStreams {
    let mut streams = MediaStreams::default();
    for line in text.lines().map(str::trim_start) {
        if !line.starts_with("Stream #") {
            continue;
        }
        streams.video |= line.contains(": Video:");
        streams.audio |= line.contains(": Audio:");
    }
    streams
}

/// Ask the bundled ffmpeg which streams a written file really has.
pub fn probe_streams(path: &str) -> Result<MediaStreams, String> {
    let out = ffmpeg_cmd()
        .args(["-hide_banner", "-i", path])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("ffmpeg could not run ({}): {e}", ffmpeg_display()))?;
    Ok(parse_streams(&String::from_utf8_lossy(&out.stderr)))
}

/// What a finished recording proves about itself.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Recorded {
    /// Frames written at the fixed 30 fps.
    pub frames: u32,
    /// Wall-clock seconds the recording loop actually ran.
    pub seconds: f64,
}

impl Recorded {
    /// How far the video timeline (frames ÷ 30) strays from the wall clock a
    /// live microphone records against. File durations cannot show this:
    /// `-shortest` trims both streams to the same length however far apart
    /// they drifted.
    pub fn drift_seconds(&self) -> f64 {
        (f64::from(self.frames) / 30.0 - self.seconds).abs()
    }
}

/// A recording counts only once its file exists, has bytes, and holds the
/// streams that were asked for.
fn verify_recording(out_path: &str, want_audio: bool) -> Result<(), String> {
    let bytes = std::fs::metadata(out_path)
        .map(|m| m.len())
        .map_err(|_| format!("the recording file was not written ({out_path})"))?;
    if bytes == 0 {
        return Err(format!("the recording file is empty ({out_path})"));
    }
    let streams = probe_streams(out_path)?;
    if !streams.video {
        return Err(format!("the recording has no video stream ({out_path})"));
    }
    if want_audio && !streams.audio {
        return Err(format!(
            "the recording has no audio stream — the microphone was not captured ({out_path})"
        ));
    }
    Ok(())
}

/// Close a recording honestly: zero frames, a failed encoder, and a bad file
/// are each an error carrying ffmpeg's own reason — never "saved".
fn finish_recording(
    mut ff: Child,
    tail: StderrTail,
    recorded: Recorded,
    out_path: &str,
    want_audio: bool,
) -> Result<Recorded, String> {
    let frames = recorded.frames;
    let status = ff
        .wait()
        .map_err(|e| format!("ffmpeg did not finish: {e}"))?;
    let stderr = tail.text();
    if frames == 0 {
        return Err(with_reason(
            "no screen frames were captured, so nothing was recorded",
            &stderr,
        ));
    }
    if !status.success() {
        return Err(with_reason(
            &format!("the recording failed (ffmpeg {status})"),
            &stderr,
        ));
    }
    verify_recording(out_path, want_audio)?;
    Ok(recorded)
}

/// The instant frame `tick` is due at exactly 30 fps. Computed from the start,
/// never accumulated, so the clip's timeline matches the wall clock a
/// microphone records against.
fn frame_deadline(start: Instant, tick: u64) -> Instant {
    start + Duration::from_nanos(tick.saturating_mul(1_000_000_000) / 30)
}

/// Record a monitor to `out_path` for `seconds`, optionally cropping every
/// frame to `crop`. Shared by full-display and region capture. Returns the
/// number of frames written, or an error string.
fn record_monitor(
    mon: &Monitor,
    crop: Option<CropRect>,
    out_path: &str,
    seconds: u32,
) -> Result<u32, String> {
    let full_w = mon.width().map_err(|e| format!("width: {e}"))?;
    let full_h = mon.height().map_err(|e| format!("height: {e}"))?;
    let (out_w, out_h) = match crop {
        Some(c) => (c.w, c.h),
        None => (full_w, full_h),
    };
    if out_w == 0 || out_h == 0 {
        return Err("zero output size".into());
    }

    let mut ff = ffmpeg_cmd()
        .args(FFMPEG_ERRORS_ONLY)
        .args([
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s",
            &format!("{out_w}x{out_h}"),
            "-r",
            "30",
            "-i",
            "-",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            out_path,
        ])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg spawn failed ({}): {e}", ffmpeg_display()))?;
    let tail = StderrTail::drain(&mut ff);
    let mut stdin = ff.stdin.take().ok_or("ffmpeg stdin")?;

    let (recorder, rx) = mon
        .video_recorder()
        .map_err(|e| format!("video_recorder: {e}"))?;
    recorder.start().map_err(|_| "recorder start failed")?;

    // Constant-fps pacing: the WGC recorder is change-driven, so drive a 30fps
    // wall clock and (re)write the latest frame each tick — duplicating while
    // the screen is idle so the clip has a correct duration + steady framerate.
    let start = Instant::now();
    let mut tick = 0u64;
    let mut frames = 0u32;
    let mut last: Option<Vec<u8>> = None;
    let mut crop_buf = crop.map(|c| vec![0u8; (c.w * c.h * 4) as usize]);
    while start.elapsed() < Duration::from_secs(seconds.max(1) as u64) {
        while let Ok(f) = rx.try_recv() {
            last = Some(f.raw);
        }
        if last.is_none() {
            if let Ok(f) = rx.recv_timeout(Duration::from_millis(300)) {
                last = Some(f.raw);
            }
        }
        if let Some(ref buf) = last {
            let wrote = match (crop, crop_buf.as_mut()) {
                (Some(c), Some(cbuf)) => {
                    if crop_frame(buf, full_w, &c, cbuf) {
                        stdin.write_all(cbuf).is_ok()
                    } else {
                        true // short frame — skip this tick, keep going
                    }
                }
                _ => stdin.write_all(buf).is_ok(),
            };
            if !wrote {
                break;
            }
            frames += 1;
        }
        tick += 1;
        let next = frame_deadline(start, tick);
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    recorder.stop().ok();
    drop(stdin);
    finish_recording(ff, tail, Recorded { frames, seconds }, out_path, false).map(|r| r.frames)
}

fn primary_monitor() -> Result<Monitor, String> {
    let monitors = Monitor::all().map_err(|e| format!("Monitor::all: {e}"))?;
    monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .ok_or_else(|| "no primary monitor".into())
}

/// Best-effort poster = first frame of the clip.
fn write_poster(mp4: &str, poster: &str) {
    let _ = ffmpeg_cmd()
        .args(["-y", "-i", mp4, "-frames:v", "1", "-q:v", "3", poster])
        .stderr(Stdio::null())
        .status();
}

fn prepare(episode_id: &str, name: &str) -> Result<(PathBuf, String), CaptureResult> {
    if !valid_name(name) {
        return Err(err("name must be [a-z0-9_-]"));
    }
    let ep_dir = find_episode_dir(episode_id).ok_or_else(|| {
        err(format!(
            "no episode matching '{episode_id}' under CONTENT_ROOT"
        ))
    })?;
    let out_dir = ep_dir.join("assets").join("generated").join("screencaps");
    std::fs::create_dir_all(&out_dir).map_err(|e| err(format!("mkdir failed: {e}")))?;
    let stem = unique_stem(&out_dir, name);
    Ok((out_dir, stem))
}

/// Capture the whole primary display for `seconds`.
pub fn capture_display(episode_id: &str, name: &str, seconds: u32) -> CaptureResult {
    let (out_dir, stem) = match prepare(episode_id, name) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mon = match primary_monitor() {
        Ok(m) => m,
        Err(e) => return err(e),
    };
    finish(
        &out_dir,
        &stem,
        record_monitor(&mon, None, path_str(&out_dir, &stem).as_str(), seconds),
    )
}

/// Capture a fractional region of the primary display. `frac` is in [0,1]
/// screen fractions (from the overlay); it's mapped to even-aligned pixels.
pub fn capture_region(episode_id: &str, name: &str, seconds: u32, frac: FracRect) -> CaptureResult {
    let (out_dir, stem) = match prepare(episode_id, name) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mon = match primary_monitor() {
        Ok(m) => m,
        Err(e) => return err(e),
    };
    let (fw, fh) = match (mon.width(), mon.height()) {
        (Ok(w), Ok(h)) => (w, h),
        _ => return err("bad monitor size"),
    };
    let crop = match to_crop(frac, fw, fh) {
        Some(c) => c,
        None => return err("region too small or out of bounds"),
    };
    finish(
        &out_dir,
        &stem,
        record_monitor(
            &mon,
            Some(crop),
            path_str(&out_dir, &stem).as_str(),
            seconds,
        ),
    )
}

fn path_str(dir: &Path, stem: &str) -> String {
    dir.join(format!("{stem}.mp4"))
        .to_string_lossy()
        .to_string()
}

/// Fraction rect → even-aligned, clamped pixel crop. Even alignment keeps
/// yuv420p chroma happy; clamping keeps the crop inside the frame.
fn to_crop(f: FracRect, fw: u32, fh: u32) -> Option<CropRect> {
    let clamp01 = |v: f64| v.clamp(0.0, 1.0);
    let even = |v: f64| ((v.round() as i64) & !1) as u32;
    let x = even(clamp01(f.x) * fw as f64);
    let y = even(clamp01(f.y) * fh as f64);
    let mut w = even(clamp01(f.w) * fw as f64);
    let mut h = even(clamp01(f.h) * fh as f64);
    if x + w > fw {
        w = (fw - x) & !1;
    }
    if y + h > fh {
        h = (fh - y) & !1;
    }
    if w < 16 || h < 16 {
        return None;
    }
    Some(CropRect { x, y, w, h })
}

fn finish(out_dir: &Path, stem: &str, rec: Result<u32, String>) -> CaptureResult {
    match rec {
        Ok(0) => err("captured 0 frames"),
        Ok(_) => {
            let mp4 = path_str(out_dir, stem);
            let poster = out_dir
                .join(format!("{stem}-poster.jpg"))
                .to_string_lossy()
                .to_string();
            write_poster(&mp4, &poster);
            CaptureResult {
                ok: true,
                path: Some(format!("assets/generated/screencaps/{stem}.mp4")),
                error: None,
            }
        }
        Err(e) => err(e),
    }
}

// ── Pass X: ViziWiz target enumeration + stills + record-to-path API ─────────
//
// The app layer owns filing (inbox + sidecar metadata); these functions take
// RESOLVED OUTPUT PATHS and a concrete target. Window clips are a per-frame
// crop of the window's monitor by its geometry (the proven region path
// generalized) — an overlapping window shows in the clip; window STILLS
// capture the window surface directly (xcap Window::capture_image).

use xcap::Window;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub is_primary: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: u32,
    pub app: String,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub fn list_monitors() -> Vec<MonitorInfo> {
    Monitor::all()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            Some(MonitorInfo {
                id: m.id().ok()?,
                name: m.name().ok()?,
                width: m.width().ok()?,
                height: m.height().ok()?,
                scale_factor: m.scale_factor().ok()?,
                is_primary: m.is_primary().unwrap_or(false),
            })
        })
        .collect()
}

/// Everything capturable right now: every monitor + every visible, titled,
/// non-minimized app window (Chrome, Firefox, anything on the computer).
pub fn list_targets() -> (Vec<MonitorInfo>, Vec<WindowInfo>) {
    let monitors = list_monitors();
    let windows = Window::all()
        .into_iter()
        .flatten()
        .filter_map(|w| {
            if w.is_minimized().unwrap_or(true) {
                return None;
            }
            let title = w.title().ok()?;
            if title.trim().is_empty() {
                return None;
            }
            Some(WindowInfo {
                id: w.id().ok()?,
                app: w.app_name().unwrap_or_default(),
                title,
                x: w.x().ok()?,
                y: w.y().ok()?,
                width: w.width().ok()?,
                height: w.height().ok()?,
            })
        })
        .collect();
    (monitors, windows)
}

fn monitor_by_id(id: u32) -> Result<Monitor, String> {
    Monitor::all()
        .map_err(|e| format!("Monitor::all: {e}"))?
        .into_iter()
        .find(|m| m.id().map(|mid| mid == id).unwrap_or(false))
        .ok_or_else(|| format!("no monitor with id {id}"))
}

fn window_by_id(id: u32) -> Result<Window, String> {
    Window::all()
        .map_err(|e| format!("Window::all: {e}"))?
        .into_iter()
        .find(|w| w.id().map(|wid| wid == id).unwrap_or(false))
        .ok_or_else(|| format!("no window with id {id} (closed?)"))
}

/// Pix still of a whole monitor → PNG at `path`.
pub fn still_monitor_to(path: &Path, monitor_id: u32) -> Result<(), String> {
    let img = monitor_by_id(monitor_id)?
        .capture_image()
        .map_err(|e| format!("capture_image: {e}"))?;
    img.save(path).map_err(|e| format!("png save: {e}"))
}

/// Pix still of one window (surface capture — occlusion-independent) → PNG.
pub fn still_window_to(path: &Path, window_id: u32) -> Result<(), String> {
    let img = window_by_id(window_id)?
        .capture_image()
        .map_err(|e| format!("capture_image: {e}"))?;
    img.save(path).map_err(|e| format!("png save: {e}"))
}

/// Pix still of a fractional region of the primary monitor → PNG.
pub fn still_region_to(path: &Path, frac: FracRect) -> Result<(), String> {
    let mon = primary_monitor()?;
    let (fw, fh) = match (mon.width(), mon.height()) {
        (Ok(w), Ok(h)) => (w, h),
        _ => return Err("bad monitor size".into()),
    };
    let crop = to_crop(frac, fw, fh).ok_or("region too small or out of bounds")?;
    let img = mon
        .capture_image()
        .map_err(|e| format!("capture_image: {e}"))?;
    let cropped = image::imageops::crop_imm(&img, crop.x, crop.y, crop.w, crop.h).to_image();
    cropped.save(path).map_err(|e| format!("png save: {e}"))
}

/// Even-odd point-in-polygon test (pixel-center sampling).
fn point_in_polygon(x: f64, y: f64, poly: &[(f64, f64)]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Polygon-crop a captured/loaded frame to its bbox, with alpha 0 outside the
/// polygon. Shared by the fresh-capture and frozen-PNG freeform paths; the
/// frame's own dimensions are the fraction reference (DPI-proof).
fn freeform_crop(img: &image::RgbaImage, points: &[[f64; 2]]) -> Result<image::RgbaImage, String> {
    if points.len() < 3 {
        return Err("freeform needs at least 3 points".into());
    }
    let (fw, fh) = (img.width(), img.height());
    let clamp01 = |v: f64| v.clamp(0.0, 1.0);
    let poly: Vec<(f64, f64)> = points
        .iter()
        .map(|p| (clamp01(p[0]) * fw as f64, clamp01(p[1]) * fh as f64))
        .collect();
    let minx = poly
        .iter()
        .map(|p| p.0)
        .fold(f64::MAX, f64::min)
        .floor()
        .max(0.0) as u32;
    let miny = poly
        .iter()
        .map(|p| p.1)
        .fold(f64::MAX, f64::min)
        .floor()
        .max(0.0) as u32;
    let maxx = (poly.iter().map(|p| p.0).fold(0.0, f64::max).ceil() as u32).min(fw);
    let maxy = (poly.iter().map(|p| p.1).fold(0.0, f64::max).ceil() as u32).min(fh);
    let bw = maxx.saturating_sub(minx);
    let bh = maxy.saturating_sub(miny);
    if bw < 8 || bh < 8 {
        return Err("freeform region too small".into());
    }
    let mut out = image::RgbaImage::new(bw, bh);
    for y in 0..bh {
        for x in 0..bw {
            let gx = f64::from(minx + x) + 0.5;
            let gy = f64::from(miny + y) + 0.5;
            let mut px = *img.get_pixel(minx + x, miny + y);
            if !point_in_polygon(gx, gy, &poly) {
                px.0[3] = 0;
            }
            out.put_pixel(x, y, px);
        }
    }
    Ok(out)
}

/// Pix still of a freeform POLYGON (fractional 0..1 points from the overlay) of
/// the primary monitor → PNG cropped to the polygon's bbox, with alpha 0 outside
/// the polygon (transparent). Snipping-tool freeform, DPI-proof like the rect path.
pub fn still_freeform_to(path: &Path, points: &[[f64; 2]]) -> Result<(), String> {
    let img = primary_monitor()?
        .capture_image()
        .map_err(|e| format!("capture_image: {e}"))?;
    freeform_crop(&img, points)?
        .save(path)
        .map_err(|e| format!("png save: {e}"))
}

/// What one Steps-recorder capture produced — size, dedup fingerprint, bytes on
/// disk, and the source window's identity + screen origin. `hash` fingerprints
/// the frame (do-vs-done comparison); `app`/`title` give each step its
/// cross-app context ("In Chrome…"); `origin_x`/`origin_y` (the window's screen
/// position) let the caller turn a global click coordinate into a point relative
/// to THIS shot — the Scribe-style "click here" highlight. Not `Copy` — it owns
/// two `String`s.
#[derive(Debug, Clone)]
pub struct StepShot {
    pub width: u32,
    pub height: u32,
    pub hash: u64,
    pub bytes: usize,
    pub app: String,
    pub title: String,
    pub origin_x: i32,
    pub origin_y: i32,
}

/// 8x8 average hash (aHash) of a frame — a cheap fingerprint so a click that
/// changed nothing on screen doesn't become a duplicate guide step. Samples a
/// sparse grid per cell so it stays fast on 4K windows.
fn ahash(img: &image::RgbaImage) -> u64 {
    let (w, h) = (img.width().max(1), img.height().max(1));
    let mut cells = [0f64; 64];
    for cy in 0..8u32 {
        for cx in 0..8u32 {
            let x0 = cx * w / 8;
            let x1 = (((cx + 1) * w / 8).max(x0 + 1)).min(w);
            let y0 = cy * h / 8;
            let y1 = (((cy + 1) * h / 8).max(y0 + 1)).min(h);
            let stepx = ((x1 - x0) / 4).max(1);
            let stepy = ((y1 - y0) / 4).max(1);
            let (mut sum, mut n) = (0f64, 0f64);
            let mut y = y0;
            while y < y1 {
                let mut x = x0;
                while x < x1 {
                    let p = img.get_pixel(x, y).0;
                    sum +=
                        0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2]);
                    n += 1.0;
                    x += stepx;
                }
                y += stepy;
            }
            cells[(cy * 8 + cx) as usize] = if n > 0.0 { sum / n } else { 0.0 };
        }
    }
    let mean = cells.iter().sum::<f64>() / 64.0;
    let mut hash = 0u64;
    for (i, c) in cells.iter().enumerate() {
        if *c > mean {
            hash |= 1u64 << i;
        }
    }
    hash
}

/// Steps recorder: capture ONE window's surface → lossy WebP at `path`.
/// Occlusion-independent (window surface, not a screen crop), and a UI
/// screenshot at q~82 lands in single-digit KB instead of ~1 MB of PNG —
/// which is what makes a 40-step guide stay ultra light.
pub fn capture_window_step(path: &Path, window_id: u32, quality: f32) -> Result<StepShot, String> {
    let win = window_by_id(window_id)?;
    let (app, title) = (
        win.app_name().unwrap_or_default(),
        win.title().unwrap_or_default(),
    );
    let (origin_x, origin_y) = (win.x().unwrap_or(0), win.y().unwrap_or(0));
    let img = win
        .capture_image()
        .map_err(|e| format!("capture_image: {e}"))?;
    let mut shot = encode_step(&img, path, quality)?;
    shot.app = app;
    shot.title = title;
    shot.origin_x = origin_x;
    shot.origin_y = origin_y;
    Ok(shot)
}

/// Shared encode: fingerprint the frame, write it as lossy WebP, report size.
fn encode_step(img: &image::RgbaImage, path: &Path, quality: f32) -> Result<StepShot, String> {
    let (width, height) = (img.width(), img.height());
    if width == 0 || height == 0 {
        return Err("window has zero size".into());
    }
    let hash = ahash(img);
    let encoder = webp::Encoder::from_rgba(img.as_raw(), width, height);
    let data = encoder.encode(quality.clamp(1.0, 100.0));
    std::fs::write(path, &*data).map_err(|e| format!("webp write: {e}"))?;
    // Window identity/origin are filled by the window-capture callers; encoding
    // alone doesn't know them.
    Ok(StepShot {
        width,
        height,
        hash,
        bytes: data.len(),
        app: String::new(),
        title: String::new(),
        origin_x: 0,
        origin_y: 0,
    })
}

/// Steps recorder: capture whatever window currently has FOCUS. Steps has no
/// window picker — it follows the operator from app to app. Windows belonging
/// to `skip_pid` (our own app) are ignored, so clicking the recorder's own
/// controls never becomes a lesson step.
pub fn capture_focused_window_step(
    path: &Path,
    quality: f32,
    skip_pid: u32,
) -> Result<StepShot, String> {
    let windows = Window::all().map_err(|e| format!("Window::all: {e}"))?;
    let target = windows.into_iter().find(|w| {
        w.is_focused().unwrap_or(false)
            && w.pid().map(|p| p != skip_pid).unwrap_or(false)
            && !w.is_minimized().unwrap_or(false)
    });
    let Some(win) = target else {
        return Err("no external window focused".into());
    };
    // Grab identity + screen origin BEFORE the capture so they describe exactly
    // the frame we shoot (the caller maps a global click into this window).
    let (app, title) = (
        win.app_name().unwrap_or_default(),
        win.title().unwrap_or_default(),
    );
    let (origin_x, origin_y) = (win.x().unwrap_or(0), win.y().unwrap_or(0));
    let img = win
        .capture_image()
        .map_err(|e| format!("capture_image: {e}"))?;
    let mut shot = encode_step(&img, path, quality)?;
    shot.app = app;
    shot.title = title;
    shot.origin_x = origin_x;
    shot.origin_y = origin_y;
    Ok(shot)
}

/// Hamming distance between two frame fingerprints — 0 means "nothing visibly
/// changed", so the caller can drop the step.
pub fn fingerprint_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Freeze the primary monitor to `path` (PNG) — the frozen frame the snip
/// overlay displays + magnifies, and that region/freeform snips crop from so the
/// capture is exactly what the operator saw (WYSIWYG). Returns the pixel size.
pub fn freeze_primary_to(path: &Path) -> Result<(u32, u32), String> {
    let img = primary_monitor()?
        .capture_image()
        .map_err(|e| format!("capture_image: {e}"))?;
    let (w, h) = (img.width(), img.height());
    img.save(path).map_err(|e| format!("png save: {e}"))?;
    Ok((w, h))
}

/// Region crop from a frozen PNG (WYSIWYG) → PNG at `dest`. The PNG's own
/// dimensions are the fraction reference, so it is DPI-proof like the live path.
pub fn still_region_from_png(src: &Path, dest: &Path, frac: FracRect) -> Result<(), String> {
    let img = image::open(src)
        .map_err(|e| format!("open frozen: {e}"))?
        .to_rgba8();
    let (fw, fh) = (img.width(), img.height());
    let crop = to_crop(frac, fw, fh).ok_or("region too small or out of bounds")?;
    let cropped = image::imageops::crop_imm(&img, crop.x, crop.y, crop.w, crop.h).to_image();
    cropped.save(dest).map_err(|e| format!("png save: {e}"))
}

/// Freeform crop from a frozen PNG (WYSIWYG) → PNG at `dest`.
pub fn still_freeform_from_png(src: &Path, dest: &Path, points: &[[f64; 2]]) -> Result<(), String> {
    let img = image::open(src)
        .map_err(|e| format!("open frozen: {e}"))?
        .to_rgba8();
    freeform_crop(&img, points)?
        .save(dest)
        .map_err(|e| format!("png save: {e}"))
}

/// Vix clip of a whole monitor → mp4 at `path`; returns frames written.
pub fn record_display_to(path: &str, seconds: u32, monitor_id: u32) -> Result<u32, String> {
    let mon = monitor_by_id(monitor_id)?;
    record_monitor(&mon, None, path, seconds)
}

/// Vix clip of a fractional region of the primary monitor → mp4.
pub fn record_region_to(path: &str, seconds: u32, frac: FracRect) -> Result<u32, String> {
    let mon = primary_monitor()?;
    let (fw, fh) = match (mon.width(), mon.height()) {
        (Ok(w), Ok(h)) => (w, h),
        _ => return Err("bad monitor size".into()),
    };
    let crop = to_crop(frac, fw, fh).ok_or("region too small or out of bounds")?;
    record_monitor(&mon, Some(crop), path, seconds)
}

/// Vix clip of one window: per-frame crop of its monitor by the window's
/// current geometry. KNOWN v1 caveat: an overlapping window shows.
pub fn record_window_to(path: &str, seconds: u32, window_id: u32) -> Result<u32, String> {
    let win = window_by_id(window_id)?;
    let mon = win
        .current_monitor()
        .map_err(|e| format!("current_monitor: {e}"))?;
    let (fw, fh) = match (mon.width(), mon.height()) {
        (Ok(w), Ok(h)) => (w, h),
        _ => return Err("bad monitor size".into()),
    };
    let (mx, my) = match (mon.x(), mon.y()) {
        (Ok(x), Ok(y)) => (x, y),
        _ => return Err("bad monitor origin".into()),
    };
    let (wx, wy, ww, wh) = match (win.x(), win.y(), win.width(), win.height()) {
        (Ok(x), Ok(y), Ok(w), Ok(h)) => (x, y, w, h),
        _ => return Err("bad window geometry".into()),
    };
    // Window global coords → monitor-local fractions → the shared crop math
    // (clamping + even alignment for yuv420p).
    let frac = FracRect {
        x: f64::from(wx - mx) / f64::from(fw),
        y: f64::from(wy - my) / f64::from(fh),
        w: f64::from(ww) / f64::from(fw),
        h: f64::from(wh) / f64::from(fh),
    };
    let crop = to_crop(frac, fw, fh).ok_or("window is off-screen or too small")?;
    record_monitor(&mon, Some(crop), path, seconds)
}

/// Best-effort poster for a clip (first frame) — public for the inbox flow.
pub fn poster_for(mp4: &str, poster: &str) {
    write_poster(mp4, poster);
}

// ── Pass Z8 V4: Start/Stop video — 720p downscale + format & audio matrix ────
//
// The fixed-`seconds` path above (capture_vix / record_*_to) is unchanged. This
// is the OVERLAY's Start▸Stop recorder: the caller owns an `AtomicBool` it flips
// to stop, plus an optional hard cap. Codec, container, downscale and an optional
// audio track are all chosen at call time. Everything is best-effort and proven
// on the real binary in the V8 drill — Windows system-audio needs a loopback
// device (Stereo Mix / virtual cable); AV1 encode is CPU-heavy (mp4 is default).

/// One recording's knobs. `height` 0 = native (no downscale); otherwise the clip
/// is scaled to at most that height (never upscaled), width auto/even.
#[derive(Clone)]
pub struct VideoOpts {
    pub height: u32,
    /// "mp4" (H.264) · "webm-vp9" · "webm-av1".
    pub format: String,
    /// "none" · "system" (loopback device) · "mic".
    pub audio: String,
    /// dshow/pulse device name; required for system, optional for mic.
    pub audio_device: Option<String>,
    /// Optional hard cap in seconds; None = run until stopped.
    pub max_seconds: Option<u32>,
}

/// What to record: a whole monitor, a fractional region of the primary monitor,
/// or one window (per-frame crop of its monitor).
pub enum RecTarget {
    Monitor(u32),
    Region(FracRect),
    Window(u32),
}

/// Container extension for a format id — the app layer names the inbox file.
pub fn ext_for_format(format: &str) -> &'static str {
    match format {
        "webm-vp9" | "webm-av1" => "webm",
        _ => "mp4",
    }
}

/// ffmpeg output video-codec args for a format id (defaults to mp4/x264).
fn video_codec_args(format: &str) -> Vec<&'static str> {
    match format {
        "webm-vp9" => vec![
            "-c:v",
            "libvpx-vp9",
            "-b:v",
            "0",
            "-crf",
            "34",
            "-deadline",
            "realtime",
            "-cpu-used",
            "6",
            "-pix_fmt",
            "yuv420p",
        ],
        "webm-av1" => {
            vec![
                "-c:v",
                "libsvtav1",
                "-preset",
                "10",
                "-crf",
                "34",
                "-pix_fmt",
                "yuv420p",
            ]
        }
        _ => vec![
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
        ],
    }
}

/// Even-aligned downscale target, or None when no scaling is needed (native, or
/// the source is already at/under the target height). Never upscales.
fn target_scale(w: u32, h: u32, target_h: u32) -> Option<(u32, u32)> {
    if target_h == 0 || h <= target_h {
        return None;
    }
    let th = target_h & !1;
    if th == 0 {
        return None;
    }
    let tw = ((u64::from(w) * u64::from(th) + u64::from(h) / 2) / u64::from(h)) as u32 & !1;
    Some((tw.max(2), th))
}

/// The audio ffmpeg input args (a second `-i`), or None for silent. Platform
/// split: Windows uses dshow, Linux uses PulseAudio.
#[cfg(target_os = "windows")]
fn audio_input_args(audio: &str, device: Option<&str>) -> Result<Option<Vec<String>>, String> {
    match audio {
        "none" => Ok(None),
        "mic" => {
            let dev =
                device.ok_or("no microphone selected — pick an input device or choose No audio")?;
            Ok(Some(vec![
                "-f".into(),
                "dshow".into(),
                "-i".into(),
                format!("audio={dev}"),
            ]))
        }
        "system" => {
            let dev = device.ok_or(
                "system audio needs a loopback device (e.g. \"Stereo Mix\" or a virtual cable). \
                 None was found/selected — record with No audio or install one.",
            )?;
            Ok(Some(vec![
                "-f".into(),
                "dshow".into(),
                "-i".into(),
                format!("audio={dev}"),
            ]))
        }
        other => Err(format!("unknown audio mode \"{other}\"")),
    }
}

#[cfg(not(target_os = "windows"))]
fn audio_input_args(audio: &str, device: Option<&str>) -> Result<Option<Vec<String>>, String> {
    match audio {
        "none" => Ok(None),
        "mic" => Ok(Some(vec![
            "-f".into(),
            "pulse".into(),
            "-i".into(),
            device.unwrap_or("default").to_string(),
        ])),
        "system" => {
            let dev = device.ok_or(
                "system audio needs a PulseAudio monitor source — none selected. \
                 Record with No audio or pass a `.monitor` source.",
            )?;
            Ok(Some(vec![
                "-f".into(),
                "pulse".into(),
                "-i".into(),
                dev.to_string(),
            ]))
        }
        other => Err(format!("unknown audio mode \"{other}\"")),
    }
}

/// What a capture device records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DeviceKind {
    Video,
    Audio,
}

/// One capture device as the bundled ffmpeg reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaptureDevice {
    /// The friendly name a person recognises ("Integrated Camera").
    pub name: String,
    /// ffmpeg's unique alternative name (`@device_cm_…`), when it printed one.
    pub alt_name: Option<String>,
    pub kind: DeviceKind,
}

impl CaptureDevice {
    /// The handle ffmpeg is given: the unique alternative name when there is
    /// one (two devices may share a friendly name), else the friendly name.
    pub fn id(&self) -> &str {
        self.alt_name.as_deref().unwrap_or(&self.name)
    }
}

/// The first `"…"` span in a line, without the quotes.
fn first_quoted(line: &str) -> Option<(&str, &str)> {
    let open = line.find('"')?;
    let rest = &line[open + 1..];
    let close = rest.find('"')?;
    Some((&rest[..close], &rest[close + 1..]))
}

/// Parse ffmpeg's `-list_devices true -f dshow -i dummy` stderr.
///
/// Two formats are real: ffmpeg 8.x tags each device inline —
/// `[in#0 @ …] "Integrated Camera" (video)` — while older builds printed
/// `DirectShow video devices` / `DirectShow audio devices` section headers
/// above untagged `[dshow @ …] "Name"` lines. Both print an indented
/// `Alternative name "…"` line under each device; it belongs to the device
/// just above it.
pub fn parse_dshow_devices(text: &str) -> Vec<CaptureDevice> {
    let mut devices: Vec<CaptureDevice> = Vec::new();
    let mut section: Option<DeviceKind> = None;
    for line in text.lines() {
        if line.contains("DirectShow video devices") {
            section = Some(DeviceKind::Video);
            continue;
        }
        if line.contains("DirectShow audio devices") {
            section = Some(DeviceKind::Audio);
            continue;
        }
        let Some((quoted, after)) = first_quoted(line) else {
            continue;
        };
        if line.contains("Alternative name") {
            if let Some(last) = devices.last_mut() {
                if last.alt_name.is_none() {
                    last.alt_name = Some(quoted.to_string());
                }
            }
            continue;
        }
        let kind = if after.contains("(video)") {
            Some(DeviceKind::Video)
        } else if after.contains("(audio)") {
            Some(DeviceKind::Audio)
        } else {
            section
        };
        if let (Some(kind), false) = (kind, quoted.is_empty()) {
            devices.push(CaptureDevice {
                name: quoted.to_string(),
                alt_name: None,
                kind,
            });
        }
    }
    devices
}

/// Every camera and microphone the bundled ffmpeg can open on this machine.
/// An ffmpeg that cannot run is an error, never a silently empty list.
#[cfg(target_os = "windows")]
pub fn list_capture_devices() -> Result<Vec<CaptureDevice>, String> {
    let out = ffmpeg_cmd()
        .args([
            "-hide_banner",
            "-list_devices",
            "true",
            "-f",
            "dshow",
            "-i",
            "dummy",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::null())
        .output()
        .map_err(|e| format!("ffmpeg could not run ({}): {e}", ffmpeg_display()))?;
    // Listing always "fails" on the dummy input; the device lines are the
    // answer, so the exit status carries nothing here.
    Ok(parse_dshow_devices(&String::from_utf8_lossy(&out.stderr)))
}

/// Linux: the bundled ffmpeg enumerates PulseAudio sources itself
/// (`-sources pulse`), so the app never shells out to pactl. Cameras (v4l2)
/// arrive with the camera pass (D0.3). An ffmpeg that cannot run is an error.
#[cfg(not(target_os = "windows"))]
pub fn list_capture_devices() -> Result<Vec<CaptureDevice>, String> {
    // Output lines look like:  "alsa_output.pci-….monitor [Monitor of Built-in] (monitor)"
    // and may land on stdout or stderr depending on the build; scan both.
    let out = ffmpeg_cmd()
        .args(["-hide_banner", "-sources", "pulse"])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("ffmpeg could not run ({}): {e}", ffmpeg_display()))?;
    let mut text = String::from_utf8_lossy(&out.stdout).to_string();
    text.push('\n');
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("Auto-detected") && !l.ends_with(':'))
        .filter_map(|l| {
            let name = l.trim_start_matches('*').split_whitespace().next()?;
            (name.contains('.') || name.contains('_')).then(|| CaptureDevice {
                name: name.to_string(),
                alt_name: None,
                kind: DeviceKind::Audio,
            })
        })
        .collect())
}

/// Long-lived DXGI recorders, one per monitor id, reused across clips.
///
/// xcap 0.9.6's DXGI recorder leaks its capture thread + `IDXGIOutputDuplication`
/// forever (no `Drop` stops the thread), and DXGI permits only ONE duplication
/// per output per process — so a SECOND `Monitor::video_recorder()` fails with
/// E_INVALIDARG (0x80070057) and can never recover. We therefore create each
/// monitor's recorder exactly once and reuse it; `start()`/`stop()` only wake and
/// sleep its capture thread, so back-to-back clips just re-run the same recorder.
/// A cached recorder + its frame channel (kept warm between clips).
type RecorderSlot = (VideoRecorder, Receiver<Frame>);
static RECORDERS: OnceLock<Mutex<HashMap<u32, RecorderSlot>>> = OnceLock::new();

fn recorders() -> &'static Mutex<HashMap<u32, (VideoRecorder, Receiver<Frame>)>> {
    RECORDERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The Start/Stop recorder: same constant-fps xcap→ffmpeg pipe as `record_monitor`,
/// but the loop runs until `stop` is set (or `opts.max_seconds` elapses), the
/// encoder/container/scale come from `opts`, and an optional audio device is muxed
/// in as a second input. Returns what the verified recording proves: frames
/// written and the wall-clock seconds the loop ran.
fn record_monitor_ctl(
    mon: &Monitor,
    crop: Option<CropRect>,
    out_path: &str,
    opts: &VideoOpts,
    stop: &AtomicBool,
) -> Result<Recorded, String> {
    let full_w = mon.width().map_err(|e| format!("width: {e}"))?;
    let full_h = mon.height().map_err(|e| format!("height: {e}"))?;
    let (src_w, src_h) = match crop {
        Some(c) => (c.w, c.h),
        None => (full_w, full_h),
    };
    if src_w == 0 || src_h == 0 {
        return Err("zero output size".into());
    }

    let mut args: Vec<String> = FFMPEG_ERRORS_ONLY
        .iter()
        .chain(["-y", "-f", "rawvideo", "-pix_fmt", "rgba"].iter())
        .map(|s| (*s).to_string())
        .collect();
    args.push("-s".into());
    args.push(format!("{src_w}x{src_h}"));
    args.push("-r".into());
    args.push("30".into());
    args.push("-i".into());
    args.push("-".into());

    let audio = audio_input_args(&opts.audio, opts.audio_device.as_deref())?;
    let has_audio = audio.is_some();
    if let Some(a) = audio {
        args.extend(a);
    }

    if let Some((tw, th)) = target_scale(src_w, src_h, opts.height) {
        args.push("-vf".into());
        args.push(format!("scale={tw}:{th}"));
    }
    args.extend(video_codec_args(&opts.format).into_iter().map(String::from));
    if has_audio {
        args.extend(
            ["-map", "0:v:0", "-map", "1:a:0"]
                .iter()
                .map(|s| (*s).to_string()),
        );
        let acodec = if opts.format == "mp4" {
            "aac"
        } else {
            "libopus"
        };
        args.push("-c:a".into());
        args.push(acodec.into());
        args.push("-shortest".into());
    }
    args.push(out_path.to_string());

    // Acquire this monitor's REUSED recorder (created once, kept forever — see
    // RECORDERS). Hold the registry lock for the whole clip; captures are already
    // serialized one-at-a-time, so this also guarantees no two clips race a
    // recorder. Opening the recorder before ffmpeg means a recorder failure can
    // never orphan an ffmpeg process.
    let monitor_id = mon.id().map_err(|e| format!("monitor id: {e}"))?;
    let mut reg = recorders()
        .lock()
        .map_err(|_| "recorder registry poisoned".to_string())?;
    // `entry` rather than contains_key+insert: one lookup, and the recorder is
    // only constructed when the slot is genuinely vacant (clippy::map_entry).
    let (recorder, rx) = match reg.entry(monitor_id) {
        std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
        std::collections::hash_map::Entry::Vacant(e) => {
            let rr = mon
                .video_recorder()
                .map_err(|e| format!("video_recorder: {e}"))?;
            e.insert(rr)
        }
    };
    // Discard any stale frame the previous clip left in the rendezvous channel.
    while rx.try_recv().is_ok() {}
    recorder.start().map_err(|_| "recorder start failed")?;

    let mut ff = match ffmpeg_cmd()
        .args(&args)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            // The recorder is already running — put it back to sleep so a
            // failed start leaves nothing capturing.
            recorder.stop().ok();
            return Err(format!("ffmpeg spawn failed ({}): {e}", ffmpeg_display()));
        }
    };
    let tail = StderrTail::drain(&mut ff);
    let mut stdin = ff.stdin.take().ok_or("ffmpeg stdin")?;

    let start = Instant::now();
    let mut tick = 0u64;
    let mut frames = 0u32;
    let mut last: Option<Vec<u8>> = None;
    let mut crop_buf = crop.map(|c| vec![0u8; (c.w * c.h * 4) as usize]);
    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if let Some(cap) = opts.max_seconds {
            if start.elapsed() >= Duration::from_secs(u64::from(cap.max(1))) {
                break;
            }
        }
        while let Ok(f) = rx.try_recv() {
            last = Some(f.raw);
        }
        if last.is_none() {
            if let Ok(f) = rx.recv_timeout(Duration::from_millis(300)) {
                last = Some(f.raw);
            }
        }
        if let Some(ref buf) = last {
            let wrote = match (crop, crop_buf.as_mut()) {
                (Some(c), Some(cbuf)) => {
                    if crop_frame(buf, full_w, &c, cbuf) {
                        stdin.write_all(cbuf).is_ok()
                    } else {
                        true
                    }
                }
                _ => stdin.write_all(buf).is_ok(),
            };
            if !wrote {
                break;
            }
            frames += 1;
        }
        tick += 1;
        let next = frame_deadline(start, tick);
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    recorder.stop().ok();
    // Keep the recorder + frame channel CACHED in RECORDERS for the next clip —
    // dropping them would leak xcap's capture thread and burn this monitor's one
    // DXGI duplication. Just release the registry lock (at scope end) + ffmpeg.
    drop(stdin);
    finish_recording(ff, tail, Recorded { frames, seconds }, out_path, has_audio)
}

/// Resolve a `RecTarget` to a monitor + optional crop, then record it Start/Stop
/// style with `opts` until `stop` is set. The public entry for the Video tab.
pub fn record_ctl_to(
    out_path: &str,
    target: RecTarget,
    opts: &VideoOpts,
    stop: &AtomicBool,
) -> Result<Recorded, String> {
    match target {
        RecTarget::Monitor(id) => {
            let mon = monitor_by_id(id)?;
            record_monitor_ctl(&mon, None, out_path, opts, stop)
        }
        RecTarget::Region(frac) => {
            let mon = primary_monitor()?;
            let (fw, fh) = match (mon.width(), mon.height()) {
                (Ok(w), Ok(h)) => (w, h),
                _ => return Err("bad monitor size".into()),
            };
            let crop = to_crop(frac, fw, fh).ok_or("region too small or out of bounds")?;
            record_monitor_ctl(&mon, Some(crop), out_path, opts, stop)
        }
        RecTarget::Window(id) => {
            let win = window_by_id(id)?;
            let mon = win
                .current_monitor()
                .map_err(|e| format!("current_monitor: {e}"))?;
            let (fw, fh) = match (mon.width(), mon.height()) {
                (Ok(w), Ok(h)) => (w, h),
                _ => return Err("bad monitor size".into()),
            };
            let (mx, my) = match (mon.x(), mon.y()) {
                (Ok(x), Ok(y)) => (x, y),
                _ => return Err("bad monitor origin".into()),
            };
            let (wx, wy, ww, wh) = match (win.x(), win.y(), win.width(), win.height()) {
                (Ok(x), Ok(y), Ok(w), Ok(h)) => (x, y, w, h),
                _ => return Err("bad window geometry".into()),
            };
            let frac = FracRect {
                x: f64::from(wx - mx) / f64::from(fw),
                y: f64::from(wy - my) / f64::from(fh),
                w: f64::from(ww) / f64::from(fw),
                h: f64::from(wh) / f64::from(fh),
            };
            let crop = to_crop(frac, fw, fh).ok_or("window is off-screen or too small")?;
            record_monitor_ctl(&mon, Some(crop), out_path, opts, stop)
        }
    }
}

#[cfg(test)]
mod root_tests {
    use super::*;

    #[test]
    fn normalize_folds_dots_and_parents() {
        assert_eq!(
            normalize(Path::new("C:/repo/./content")),
            PathBuf::from("C:/repo/content")
        );
        assert_eq!(
            normalize(Path::new("C:/repo/target/release/../../content")),
            PathBuf::from("C:/repo/content")
        );
    }

    #[test]
    fn ambient_candidates_walk_exe_first_then_cwd_last() {
        let v = ambient_candidates();
        assert!(v.len() >= 2, "exe candidates + cwd fallback: {v:?}");
        assert_eq!(
            v.last().unwrap(),
            &PathBuf::from("./content"),
            "the launch cwd is the LAST resort, never the first: {v:?}"
        );
        // Every exe-derived candidate ends in content/ and the first is
        // exe-adjacent (the portable anchor).
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(Path::to_path_buf))
            .unwrap();
        assert_eq!(v[0], exe_dir.join("content"), "portable anchor first");
        assert!(v.iter().all(|p| p.ends_with("content")), "{v:?}");
    }

    #[test]
    fn rings_are_content_siblings_with_a_degenerate_nest() {
        let c = Path::new("C:/apps/devpulse/content");
        assert_eq!(
            ring_sibling(c, "assets"),
            Path::new("C:/apps/devpulse/assets")
        );
        assert_eq!(
            ring_sibling(c, ".userdata"),
            Path::new("C:/apps/devpulse/.userdata")
        );
        // A rootless content root nests the ring inside itself.
        let bare = Path::new("/");
        let nested = ring_sibling(bare, "assets");
        assert!(nested.ends_with("assets"), "{nested:?}");
    }

    #[test]
    fn ensure_rings_is_idempotent_and_writes_the_deletability_readme() {
        // Runs against the real (repo) rings — gitignored, and creating
        // them is exactly what boot does; idempotency is the assertion.
        let a = ensure_rings().unwrap();
        let b = ensure_rings().unwrap();
        assert_eq!(a.userdata, b.userdata);
        assert!(a.userdata.join("temp").is_dir());
        assert!(a.userdata.join("cache").is_dir());
        assert!(a.userdata.join("profiles").is_dir());
        let readme = std::fs::read_to_string(a.userdata.join("README.txt")).unwrap();
        assert!(readme.contains("safe to delete"), "{readme}");
        let cap = assets_capture_dir("web", "unit-smoke").unwrap();
        assert!(
            cap.ends_with("captures/web/unit-smoke") || cap.ends_with("captures\\web\\unit-smoke")
        );
        assert!(cap.starts_with(&a.assets));
    }

    #[test]
    fn checked_roots_name_every_location_display_normalized() {
        let out = checked_roots();
        assert!(!out.is_empty());
        assert!(
            out.iter().all(|s| !s.contains('\\')),
            "forward-slash display: {out:?}"
        );
    }
}

/// D0.1a — every parser here is tested against bytes the bundled ffmpeg
/// 8.1.2 really printed on the operator's laptop (`tests/fixtures/`, kept
/// CRLF by `.gitattributes`), never against hand-typed approximations.
#[cfg(test)]
mod honest_media_tests {
    use super::*;

    const DSHOW_8: &str = include_str!("../tests/fixtures/dshow-list-ffmpeg-8.1.2.txt");
    const PROBE_AV: &str = include_str!("../tests/fixtures/ffmpeg-i-video-audio-8.1.2.txt");
    const PROBE_V: &str = include_str!("../tests/fixtures/ffmpeg-i-video-only-8.1.2.txt");

    fn laptop_devices(devices: &[CaptureDevice]) {
        assert_eq!(devices.len(), 3, "{devices:?}");
        assert_eq!(devices[0].name, "Integrated Camera");
        assert_eq!(devices[0].kind, DeviceKind::Video);
        assert!(devices[0]
            .alt_name
            .as_deref()
            .is_some_and(|a| a.starts_with("@device_pnp_")));
        assert_eq!(devices[1].name, "OBS Virtual Camera");
        assert_eq!(devices[1].kind, DeviceKind::Video);
        // A name with its own parentheses survives whole.
        assert_eq!(devices[2].name, "Microphone Array (Realtek(R) Audio)");
        assert_eq!(devices[2].kind, DeviceKind::Audio);
        assert!(devices[2]
            .alt_name
            .as_deref()
            .is_some_and(|a| a.starts_with("@device_cm_")));
    }

    #[test]
    fn the_fixture_is_still_the_crlf_bytes_ffmpeg_wrote() {
        assert!(DSHOW_8.contains("\r\n"), "fixture was normalised to LF");
    }

    #[test]
    fn ffmpeg_8_inline_tagged_listing_parses_every_device() {
        laptop_devices(&parse_dshow_devices(DSHOW_8));
    }

    #[test]
    fn the_same_listing_with_lf_endings_parses_identically() {
        laptop_devices(&parse_dshow_devices(&DSHOW_8.replace("\r\n", "\n")));
    }

    #[test]
    fn the_older_section_header_format_still_parses() {
        // The layout the previous parser was written against: untagged names
        // under "DirectShow video/audio devices" headers.
        let older = "[dshow @ 0000020f] DirectShow video devices (some may be both video and audio devices)\n\
                     [dshow @ 0000020f]  \"Integrated Camera\"\n\
                     [dshow @ 0000020f]     Alternative name \"@device_pnp_cam\"\n\
                     [dshow @ 0000020f] DirectShow audio devices\n\
                     [dshow @ 0000020f]  \"Microphone (Realtek)\"\n\
                     [dshow @ 0000020f]     Alternative name \"@device_cm_mic\"\n";
        let devices = parse_dshow_devices(older);
        assert_eq!(devices.len(), 2, "{devices:?}");
        assert_eq!(devices[0].kind, DeviceKind::Video);
        assert_eq!(devices[0].id(), "@device_pnp_cam");
        assert_eq!(devices[1].kind, DeviceKind::Audio);
        assert_eq!(devices[1].name, "Microphone (Realtek)");
        assert_eq!(devices[1].id(), "@device_cm_mic");
    }

    #[test]
    fn a_device_id_is_its_unique_alternative_name_when_ffmpeg_gave_one() {
        let devices = parse_dshow_devices(DSHOW_8);
        assert!(devices[2].id().starts_with("@device_cm_"));
        let bare = CaptureDevice {
            name: "Mic".into(),
            alt_name: None,
            kind: DeviceKind::Audio,
        };
        assert_eq!(bare.id(), "Mic");
    }

    #[test]
    fn stream_probe_sees_video_and_audio_only_when_both_exist() {
        assert_eq!(
            parse_streams(PROBE_AV),
            MediaStreams {
                video: true,
                audio: true
            }
        );
        assert_eq!(
            parse_streams(PROBE_V),
            MediaStreams {
                video: true,
                audio: false
            }
        );
        assert_eq!(parse_streams(DSHOW_8), MediaStreams::default());
    }

    #[test]
    fn drift_compares_the_video_timeline_to_the_wall_clock() {
        // The 2026-10-11 mic drill: 1799 frames over a 60 s window.
        let on_clock = Recorded {
            frames: 1799,
            seconds: 59.98,
        };
        assert!(on_clock.drift_seconds() < 1.0 / 30.0);
        // The old fixed 33 ms step wrote ~1818 frames a minute: the video ran
        // ~0.6 s ahead of the microphone, and the drill must fail it.
        let old_pacing = Recorded {
            frames: 1818,
            seconds: 60.0,
        };
        assert!(old_pacing.drift_seconds() > 0.5);
    }

    #[test]
    fn frame_deadlines_are_exact_at_thirty_per_second() {
        let start = Instant::now();
        assert_eq!(frame_deadline(start, 0), start);
        assert_eq!(frame_deadline(start, 30) - start, Duration::from_secs(1));
        assert_eq!(frame_deadline(start, 1800) - start, Duration::from_secs(60));
        // The old fixed 33 ms step drifted 0.6 s a minute; computed deadlines
        // land on the wall clock every whole second.
        assert_eq!(
            frame_deadline(start, 1) - start,
            Duration::from_nanos(33_333_333)
        );
    }

    #[test]
    fn a_failure_carries_ffmpegs_last_word() {
        assert_eq!(
            with_reason("the recording failed", "line one\nCould not open device\n\n"),
            "the recording failed: Could not open device"
        );
        assert_eq!(with_reason("no frames", ""), "no frames");
    }
}
