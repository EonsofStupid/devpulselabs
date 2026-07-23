//! The seekable CLIP player (S2) — the loop player's machinery pointed
//! at a finite file. ADR-009 unchanged: ffmpeg is a SUBPROCESS. One
//! `-re -ss <t> -i …` rawvideo pipe per play/seek; the pipe ENDS at
//! EOF (no `-stream_loop`, no respawn — the loop player's respawn is a
//! backdrop law, not a clip law). Duration comes from `ffmpeg -i`'s
//! own stderr banner — no ffprobe dependency.

use crate::{ffmpeg_program, DECODE_H, DECODE_W, FRAME_BYTES};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// The rawvideo pipe from `start` seconds: `-ss` BEFORE `-i` (fast
/// seek), realtime paced, cover-cropped to the shared decode size,
/// finite (a clip ends).
fn clip_pipe_args(path: &Path, start: f32) -> Vec<String> {
    vec![
        "-v".into(),
        "error".into(),
        "-re".into(),
        "-ss".into(),
        format!("{start:.3}"),
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

/// "Duration: HH:MM:SS.cc" from an `ffmpeg -i` stderr banner → seconds.
fn parse_duration(stderr: &str) -> Option<f32> {
    let rest = stderr.split("Duration: ").nth(1)?;
    let stamp = rest.split([',', '\n', '\r']).next()?.trim();
    let mut parts = stamp.split(':');
    let h: f32 = parts.next()?.trim().parse().ok()?;
    let m: f32 = parts.next()?.trim().parse().ok()?;
    let s: f32 = parts.next()?.trim().parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

/// The clip's length via `ffmpeg -i <path>` (exits nonzero — the
/// banner still prints). None when ffmpeg is absent or the banner
/// doesn't parse; callers degrade honestly.
pub fn clip_duration(path: &Path) -> Option<f32> {
    let out = Command::new(ffmpeg_program())
        .args(["-hide_banner", "-i"])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .ok()?;
    parse_duration(&String::from_utf8_lossy(&out.stderr))
}

struct ClipShared {
    frame: Mutex<(u64, Vec<u8>)>,
    seq: AtomicU64,
    stop: AtomicBool,
    /// The pipe reached EOF (a clip ends; the UI offers replay).
    ended: AtomicBool,
}

#[derive(Default)]
struct ClipTex {
    handle: Option<egui::TextureHandle>,
    uploaded_seq: u64,
}

/// One session/clip playback. Play, pause, seek — each (re)spawn takes
/// a FRESH shared slot so the old reader exits on its stop flag and at
/// most one live pipe feeds the texture.
pub struct ClipPlayer {
    path: PathBuf,
    shared: Arc<ClipShared>,
    tex: Mutex<ClipTex>,
    /// The clip's probed length; None degrades (rail spans the events).
    pub duration: Option<f32>,
    /// Media time where the current pipe started.
    base: f32,
    /// Some while playing — `-re` paces at native rate, so wall clock
    /// since spawn IS media progress.
    playing_since: Option<Instant>,
}

impl ClipPlayer {
    /// Open and start playing from 0.
    pub fn open(path: PathBuf) -> Self {
        let duration = clip_duration(&path);
        let mut p = Self {
            shared: Self::spawn(&path, 0.0),
            path,
            tex: Mutex::new(ClipTex::default()),
            duration,
            base: 0.0,
            playing_since: Some(Instant::now()),
        };
        p.base = 0.0;
        p
    }

    fn spawn(path: &Path, start: f32) -> Arc<ClipShared> {
        let shared = Arc::new(ClipShared {
            frame: Mutex::new((0, Vec::new())),
            seq: AtomicU64::new(0),
            stop: AtomicBool::new(false),
            ended: AtomicBool::new(false),
        });
        let reader = Arc::clone(&shared);
        let path = path.to_path_buf();
        std::thread::Builder::new()
            .name("dpl-video-clip".into())
            .spawn(move || read_clip(&reader, &path, start))
            .expect("spawn clip reader thread");
        shared
    }

    fn kill_pipe(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
    }

    pub fn playing(&self) -> bool {
        self.playing_since.is_some() && !self.ended()
    }

    pub fn ended(&self) -> bool {
        self.shared.ended.load(Ordering::Acquire)
    }

    /// Current media position in seconds (frozen while paused; clamped
    /// to the known duration).
    pub fn position(&self) -> f32 {
        let mut t = self.base
            + self
                .playing_since
                .map(|s| s.elapsed().as_secs_f32())
                .unwrap_or(0.0);
        if let Some(d) = self.duration {
            t = t.min(d);
        }
        t
    }

    /// Play/pause. After EOF, toggling replays from 0.
    pub fn toggle(&mut self) {
        if self.ended() {
            self.seek(0.0);
        } else if self.playing_since.is_some() {
            self.base = self.position();
            self.playing_since = None;
            self.kill_pipe();
        } else {
            self.shared = Self::spawn(&self.path, self.base);
            self.playing_since = Some(Instant::now());
        }
    }

    /// Jump to `t` seconds and play from there.
    pub fn seek(&mut self, t: f32) {
        let t = t.max(0.0);
        self.kill_pipe();
        self.shared = Self::spawn(&self.path, t);
        self.base = t;
        self.playing_since = Some(Instant::now());
    }

    /// The current frame as a texture (the loop player's upload
    /// pattern). None until the first frame lands — callers paint the
    /// poster.
    pub fn texture(&self, ctx: &egui::Context) -> Option<egui::TextureId> {
        let latest = self.shared.seq.load(Ordering::Acquire);
        let mut tex = self.tex.lock().expect("clip tex lock");
        if latest > tex.uploaded_seq || tex.uploaded_seq > latest {
            // A fresh pipe restarts seq at 1 — resync on either side.
            let guard = self.shared.frame.lock().expect("clip frame lock");
            if guard.1.len() == FRAME_BYTES {
                let image =
                    egui::ColorImage::from_rgba_unmultiplied([DECODE_W, DECODE_H], &guard.1);
                match &mut tex.handle {
                    Some(h) => h.set(image, egui::TextureOptions::LINEAR),
                    None => {
                        tex.handle = Some(ctx.load_texture(
                            "dpl-video-clip",
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

impl Drop for ClipPlayer {
    fn drop(&mut self) {
        self.kill_pipe();
    }
}

/// One pipe, one life: read frames until EOF or stop. EOF marks
/// `ended` — NO respawn.
fn read_clip(shared: &ClipShared, path: &Path, start: f32) {
    let child = Command::new(ffmpeg_program())
        .args(clip_pipe_args(path, start))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else {
        shared.ended.store(true, Ordering::Release);
        return;
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
                    let mut slot = shared.frame.lock().expect("clip frame lock");
                    slot.0 = seq;
                    if slot.1.len() == FRAME_BYTES {
                        slot.1.copy_from_slice(&buf);
                    } else {
                        slot.1 = buf.clone();
                    }
                }
                shared.seq.store(seq, Ordering::Release);
            }
            Err(_) => break, // EOF (or a dead pipe): the clip is over.
        }
    }
    let _ = child.wait();
    shared.ended.store(true, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_args_fast_seek_and_finite() {
        let args = clip_pipe_args(Path::new("s.mp4"), 12.5);
        let joined = args.join(" ");
        assert!(
            joined.contains("-ss 12.500 -i"),
            "-ss before -i (fast seek): {joined}"
        );
        assert!(
            !joined.contains("-stream_loop"),
            "a clip ends — no loop: {joined}"
        );
        assert!(joined.contains("-re"), "realtime pacing: {joined}");
        assert!(joined.ends_with("-pix_fmt rgba -"), "raw rgba: {joined}");
    }

    #[test]
    fn duration_parses_the_real_banner_and_refuses_garbage() {
        let banner = "Input #0, mov,mp4, from 'session.mp4':\n  Metadata:\n\
                      \x20 Duration: 00:00:06.03, start: 0.000000, bitrate: 2996 kb/s\n";
        let d = parse_duration(banner).unwrap();
        assert!((d - 6.03).abs() < 1e-3, "{d}");
        let long = "Duration: 01:02:03.50, start";
        let d = parse_duration(long).unwrap();
        assert!((d - 3723.5).abs() < 1e-3, "{d}");
        assert_eq!(parse_duration("no banner here"), None);
        assert_eq!(parse_duration("Duration: N/A, start"), None);
    }

    #[test]
    fn a_paused_position_is_frozen_and_clamped() {
        // Constructed directly — no ffmpeg involved in the math.
        let mut p = ClipPlayer {
            path: PathBuf::from("x.mp4"),
            shared: Arc::new(ClipShared {
                frame: Mutex::new((0, Vec::new())),
                seq: AtomicU64::new(0),
                stop: AtomicBool::new(true),
                ended: AtomicBool::new(false),
            }),
            tex: Mutex::new(ClipTex::default()),
            duration: Some(5.0),
            base: 3.25,
            playing_since: None,
        };
        assert!((p.position() - 3.25).abs() < 1e-6, "paused = frozen");
        assert!(!p.playing());
        p.base = 99.0;
        assert!((p.position() - 5.0).abs() < 1e-6, "clamped to duration");
    }
}
