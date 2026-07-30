//! Looping video → egui texture, the ADR-009 way: ffmpeg is a SUBPROCESS,
//! never a crate dep. One `ffmpeg -re -stream_loop -1 … -f rawvideo` pipe
//! per playing loop; a reader thread owns the pipe and publishes the newest
//! frame into a shared slot; the UI thread uploads it as a texture when it
//! changes. Decode happens at backdrop resolution (`DECODE_W×DECODE_H`,
//! cover-cropped by ffmpeg itself) — the loop sits under a 70% scrim, so
//! full-res decode would be pure waste.
//!
//! Failure is honest and quiet: no ffmpeg → no frames (callers paint the
//! poster/aurora instead), with a cooldown retry — never a panic, never a
//! spawn storm.

pub mod clip;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Decode size — 16:9, cover-cropped by ffmpeg (`force_original_aspect_ratio
/// =increase` + center crop), so every frame is exactly this many pixels.
pub const DECODE_W: usize = 960;
pub const DECODE_H: usize = 540;
pub(crate) const FRAME_BYTES: usize = DECODE_W * DECODE_H * 4;
/// After a spawn/pipe failure, wait this long before trying ffmpeg again.
const RETRY_COOLDOWN: Duration = Duration::from_secs(10);

static FFMPEG: OnceLock<PathBuf> = OnceLock::new();

/// The managed ffmpeg binary (same injection contract as the capture
/// engine). An uninjected player fails honestly and never searches PATH.
pub fn set_ffmpeg_path(p: PathBuf) {
    let _ = FFMPEG.set(p);
}

/// Whether an ffmpeg path was injected — surfaces use this to tell
/// "still decoding" apart from "this install has no media engine".
pub fn ffmpeg_available() -> bool {
    FFMPEG.get().is_some()
}

pub(crate) fn ffmpeg_program() -> PathBuf {
    FFMPEG
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("__devpulse_embedded_ffmpeg_not_configured__"))
}

/// The args for a realtime, endless, cover-cropped rawvideo pipe.
/// `-re` paces decode at the stream's native rate; `-stream_loop -1` loops
/// the input forever, so the pipe never ends.
fn pipe_args(path: &Path) -> Vec<String> {
    vec![
        "-v".into(),
        "error".into(),
        "-re".into(),
        "-stream_loop".into(),
        "-1".into(),
        "-i".into(),
        path.to_string_lossy().into_owned(),
        "-an".into(),
        "-vf".into(),
        format!(
            "scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}",
            w = DECODE_W,
            h = DECODE_H
        ),
        "-f".into(),
        "rawvideo".into(),
        "-pix_fmt".into(),
        "rgba".into(),
        "-".into(),
    ]
}

struct Shared {
    /// The newest decoded frame (RGBA, FRAME_BYTES long) + its sequence
    /// number. The reader writes, the UI thread reads — one slot is enough,
    /// a backdrop drops frames instead of queueing them.
    frame: Mutex<(u64, Vec<u8>)>,
    seq: AtomicU64,
    stop: AtomicBool,
}

/// One looping video. `Clone` is cheap (Arc) so the handle can live in egui
/// temp memory. Dropping the LAST handle stops the reader and reaps ffmpeg.
#[derive(Clone)]
pub struct Player {
    shared: Arc<Shared>,
    /// UI-thread state, deliberately outside `Shared`.
    tex: Arc<Mutex<PlayerTex>>,
}

#[derive(Default)]
struct PlayerTex {
    handle: Option<egui::TextureHandle>,
    uploaded_seq: u64,
}

impl Player {
    /// Spawn the loop for `path`. Returns immediately; frames appear when
    /// ffmpeg delivers them (or never, if ffmpeg is absent — that's the
    /// poster path).
    pub fn open(path: PathBuf) -> Self {
        let shared = Arc::new(Shared {
            frame: Mutex::new((0, Vec::new())),
            seq: AtomicU64::new(0),
            stop: AtomicBool::new(false),
        });
        let reader = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("dpl-video-loop".into())
            .spawn(move || reader_loop(&reader, &path))
            .expect("spawn video reader thread");
        Self {
            shared,
            tex: Arc::new(Mutex::new(PlayerTex::default())),
        }
    }

    /// The current frame as a texture, uploading only when a new frame
    /// arrived. `None` until the first frame lands.
    pub fn texture(&self, ctx: &egui::Context) -> Option<egui::TextureId> {
        let latest = self.shared.seq.load(Ordering::Acquire);
        let mut tex = self.tex.lock().expect("player tex lock");
        if latest > tex.uploaded_seq {
            let guard = self.shared.frame.lock().expect("player frame lock");
            if guard.1.len() == FRAME_BYTES {
                let image =
                    egui::ColorImage::from_rgba_unmultiplied([DECODE_W, DECODE_H], &guard.1);
                match &mut tex.handle {
                    Some(h) => h.set(image, egui::TextureOptions::LINEAR),
                    None => {
                        tex.handle = Some(ctx.load_texture(
                            "dpl-video-loop",
                            image,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                }
                tex.uploaded_seq = guard.0;
            }
        }
        tex.handle.as_ref().map(|h| h.id())
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        // Last handle (the two Arcs travel together): stop the reader.
        if Arc::strong_count(&self.shared) == 1 {
            self.shared.stop.store(true, Ordering::Release);
        }
    }
}

/// Own the ffmpeg child for the thread's lifetime; respawn on failure after
/// a cooldown; reap on stop.
fn reader_loop(shared: &Shared, path: &Path) {
    while !shared.stop.load(Ordering::Acquire) {
        let mut cmd = Command::new(ffmpeg_program());
        cmd.args(pipe_args(path))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let child = dpl_proc::quiet(&mut cmd).spawn();
        let mut child: Child = match child {
            Ok(c) => c,
            Err(_) => {
                // No ffmpeg — sleep out the cooldown in small stoppable steps.
                cooldown(shared);
                continue;
            }
        };
        let mut out = child.stdout.take().expect("piped stdout");
        let mut buf = vec![0u8; FRAME_BYTES];
        loop {
            if shared.stop.load(Ordering::Acquire) {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
            match out.read_exact(&mut buf) {
                Ok(()) => {
                    let seq = shared.seq.load(Ordering::Relaxed) + 1;
                    {
                        let mut slot = shared.frame.lock().expect("frame lock");
                        slot.0 = seq;
                        // Reuse the allocation once it exists.
                        if slot.1.len() == FRAME_BYTES {
                            slot.1.copy_from_slice(&buf);
                        } else {
                            slot.1 = buf.clone();
                        }
                    }
                    shared.seq.store(seq, Ordering::Release);
                }
                Err(_) => break, // pipe closed / short read → respawn path
            }
        }
        let _ = child.kill();
        let _ = child.wait();
        cooldown(shared);
    }
}

fn cooldown(shared: &Shared) {
    let until = Instant::now() + RETRY_COOLDOWN;
    while Instant::now() < until {
        if shared.stop.load(Ordering::Acquire) {
            return;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Cover-fit UVs: crop the texture (aspect `tex_a`) to fill a target of
/// aspect `rect_a` — the egui side of `object-fit: cover`.
pub fn cover_uv(tex_a: f32, rect_a: f32) -> egui::Rect {
    if !(tex_a.is_finite() && rect_a.is_finite()) || tex_a <= 0.0 || rect_a <= 0.0 {
        return egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    }
    if rect_a > tex_a {
        // Target is wider: full width, crop height.
        let keep = tex_a / rect_a;
        let m = (1.0 - keep) * 0.5;
        egui::Rect::from_min_max(egui::pos2(0.0, m), egui::pos2(1.0, 1.0 - m))
    } else {
        // Target is taller: full height, crop width.
        let keep = rect_a / tex_a;
        let m = (1.0 - keep) * 0.5;
        egui::Rect::from_min_max(egui::pos2(m, 0.0), egui::pos2(1.0 - m, 1.0))
    }
}

/// Paint `path`'s loop cover-fit into `rect`. Returns `false` while no frame
/// exists yet (ffmpeg still starting, or absent) so the caller can paint the
/// poster instead. Players are cached in egui temp memory per path.
pub fn loop_backdrop(ui: &egui::Ui, rect: egui::Rect, path: &Path) -> bool {
    let id = egui::Id::new(("dpl-video", path));
    let player = ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_insert_with(id, || Player::open(path.to_path_buf()))
            .clone()
    });
    let painted = match player.texture(ui.ctx()) {
        Some(tex_id) => {
            let uv = cover_uv(DECODE_W as f32 / DECODE_H as f32, rect.aspect_ratio());
            ui.painter().image(tex_id, rect, uv, egui::Color32::WHITE);
            true
        }
        None => false,
    };
    // A playing loop never rests; poll for the next frame at display pace.
    ui.ctx().request_repaint_after(Duration::from_millis(16));
    painted
}

/// `--content-root <path>` / `--content-root=<path>` from the process args,
/// else the `CONTENT_ROOT` env the harness speaks. Deliberately duplicated
/// from `dpl_engine_capture` rather than depended on: this crate stays a leaf,
/// and the contract is two lines of arg parsing.
fn explicit_content_root() -> Option<PathBuf> {
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

/// Resolve a repo asset (`assets/backdrops/dpl.mp4`) from the exe's folder
/// upward, then the CWD — dev runs from the repo, packaged runs sit next to
/// `assets/`. `None` when nothing exists (callers degrade honestly).
pub fn locate_asset(rel: &str) -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    // The EXPLICIT content root, when the harness named one. Read straight
    // from the process args (and the env seam behind them), which is exactly
    // what `dpl_engine_capture::explicit_root` does — so this honours the same
    // contract with no dependency on that crate, as the note below requires.
    //
    // Without this, every drill that points `--content-root` at a scratch dir
    // resolves nothing: the ambient candidates are exe-adjacent and the cwd,
    // neither of which is the scratch root. A media card and a picture picker
    // both showed "file not found" for files that were plainly there.
    roots.extend(explicit_content_root());
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        // Walk up a few levels: target/release/exe → repo root in dev.
        for _ in 0..4 {
            if let Some(d) = dir {
                roots.push(d.clone());
                dir = d.parent().map(Path::to_path_buf);
            } else {
                break;
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd);
    }
    // Two path spaces resolve here: repo/install-relative refs ("assets/…",
    // tried FIRST so nothing regresses) and CONTENT-root-relative artifact
    // rel_paths ("inbox/clip.mp4", W11 media slides). The content root is
    // exe-adjacent / walk-up per P1, so probing a `content/` subdir of each
    // root bridges the second space without a heavy engine-capture dep. An
    // absolute `rel` passes through unchanged (join with an absolute wins).
    roots
        .into_iter()
        .flat_map(|r| [r.join(rel), r.join("content").join(rel)])
        .find(|p| p.is_file())
}

/// The poster convention from the old repo: `x.mp4` → `x-poster.jpg`.
pub fn poster_of(video: &Path) -> PathBuf {
    let stem = video
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("poster");
    video.with_file_name(format!("{stem}-poster.jpg"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_uv_crops_height_for_wider_targets() {
        // 16:9 texture into a 32:9 target: full width, half the height kept.
        let uv = cover_uv(16.0 / 9.0, 32.0 / 9.0);
        assert_eq!((uv.min.x, uv.max.x), (0.0, 1.0));
        assert!((uv.min.y - 0.25).abs() < 1e-6 && (uv.max.y - 0.75).abs() < 1e-6);
    }

    #[test]
    fn cover_uv_crops_width_for_taller_targets() {
        let uv = cover_uv(16.0 / 9.0, 8.0 / 9.0);
        assert_eq!((uv.min.y, uv.max.y), (0.0, 1.0));
        assert!((uv.min.x - 0.25).abs() < 1e-6 && (uv.max.x - 0.75).abs() < 1e-6);
    }

    #[test]
    fn cover_uv_matching_aspect_is_the_full_texture() {
        let uv = cover_uv(16.0 / 9.0, 16.0 / 9.0);
        assert_eq!(
            uv,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
        );
        // Degenerate inputs degrade to full-texture, never NaN.
        let uv = cover_uv(0.0, 1.0);
        assert_eq!(uv.min, egui::pos2(0.0, 0.0));
    }

    #[test]
    fn pipe_args_loop_forever_at_native_rate_cover_cropped() {
        let args = pipe_args(Path::new("x.mp4"));
        let joined = args.join(" ");
        assert!(joined.contains("-re"), "realtime pacing: {joined}");
        assert!(joined.contains("-stream_loop -1"), "endless loop: {joined}");
        assert!(
            joined.contains("force_original_aspect_ratio=increase,crop=960:540"),
            "cover crop at decode: {joined}"
        );
        assert!(
            joined.ends_with("-pix_fmt rgba -"),
            "raw rgba to stdout: {joined}"
        );
    }

    #[test]
    fn poster_convention_matches_the_old_repo() {
        assert_eq!(
            poster_of(Path::new("assets/backdrops/dpl.mp4")),
            Path::new("assets/backdrops/dpl-poster.jpg")
        );
    }
}
