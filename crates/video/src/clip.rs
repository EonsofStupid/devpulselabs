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

/// L4 preview grading for a clip pipe: an optional `.cube` LUT and an
/// optional luma-matte companion — both EXACT in preview because the
/// same ffmpeg applies them inside the decode pipe.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PipeGrade {
    pub lut: Option<PathBuf>,
    pub matte: Option<PathBuf>,
}

/// A path as an ffmpeg FILTER argument (lut3d): forward slashes, drive
/// colon escaped, single-quoted — the filtergraph parser eats bare `:`.
/// (engine-media has the same 3 lines; the crates share no dep — the
/// repo's small-utility philosophy.)
fn filter_path(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/").replace(':', "\\:");
    format!("'{s}'")
}

/// The rawvideo pipe from `start` seconds: `-ss` BEFORE `-i` (fast
/// seek), realtime paced, cover-cropped to the shared decode size,
/// finite (a clip ends). A LUT rides the filter chain; a matte joins as
/// a second input whose luminance alphamerges onto the frame (the L4
/// proven forms, at preview scale).
fn clip_pipe_args(path: &Path, start: f32, grade: &PipeGrade) -> Vec<String> {
    let fit = format!(
        "scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h}",
        w = DECODE_W,
        h = DECODE_H
    );
    let lut = grade
        .lut
        .as_deref()
        .map(|p| format!(",lut3d={}", filter_path(p)))
        .unwrap_or_default();
    let mut args: Vec<String> = vec![
        "-v".into(),
        "error".into(),
        "-re".into(),
        "-ss".into(),
        format!("{start:.3}"),
        "-i".into(),
        path.to_string_lossy().into_owned(),
    ];
    match &grade.matte {
        Some(m) => {
            args.extend([
                "-ss".into(),
                format!("{start:.3}"),
                "-i".into(),
                m.to_string_lossy().into_owned(),
                "-an".into(),
                "-filter_complex".into(),
                format!("[0:v]{fit}{lut}[f];[1:v]{fit},format=gray[m];[f][m]alphamerge[out]"),
                "-map".into(),
                "[out]".into(),
            ]);
        }
        None => {
            args.extend(["-an".into(), "-vf".into(), format!("{fit}{lut}")]);
        }
    }
    args.extend([
        "-f".into(),
        "rawvideo".into(),
        "-pix_fmt".into(),
        "rgba".into(),
        "-".into(),
    ]);
    args
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
    let mut cmd = Command::new(ffmpeg_program());
    cmd.args(["-hide_banner", "-i"])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let out = dpl_proc::quiet(&mut cmd).output().ok()?;
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
    /// The L4 pipe grade (LUT/matte) — every (re)spawn carries it.
    grade: PipeGrade,
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
        Self::open_graded(path, PipeGrade::default())
    }

    /// Open with an L4 pipe grade (LUT and/or luma matte) — the preview
    /// is EXACT for both; a changed grade means a lane reopen (callers
    /// compare via [`ClipPlayer::grade`]).
    pub fn open_graded(path: PathBuf, grade: PipeGrade) -> Self {
        let duration = clip_duration(&path);
        let mut p = Self {
            shared: Self::spawn(&path, 0.0, &grade),
            path,
            grade,
            tex: Mutex::new(ClipTex::default()),
            duration,
            base: 0.0,
            playing_since: Some(Instant::now()),
        };
        p.base = 0.0;
        p
    }

    /// The grade this player's pipes carry.
    pub fn grade(&self) -> &PipeGrade {
        &self.grade
    }

    fn spawn(path: &Path, start: f32, grade: &PipeGrade) -> Arc<ClipShared> {
        let shared = Arc::new(ClipShared {
            frame: Mutex::new((0, Vec::new())),
            seq: AtomicU64::new(0),
            stop: AtomicBool::new(false),
            ended: AtomicBool::new(false),
        });
        let reader = Arc::clone(&shared);
        let path = path.to_path_buf();
        let grade = grade.clone();
        std::thread::Builder::new()
            .name("dpl-video-clip".into())
            .spawn(move || read_clip(&reader, &path, start, &grade))
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
            self.shared = Self::spawn(&self.path, self.base, &self.grade);
            self.playing_since = Some(Instant::now());
        }
    }

    /// Jump to `t` seconds and play from there.
    pub fn seek(&mut self, t: f32) {
        let t = t.max(0.0);
        self.kill_pipe();
        self.shared = Self::spawn(&self.path, t, &self.grade);
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
///
/// The stop flag is honored AFTER a frame lands, never before the
/// first: a paused open is open→seek→pause within one UI tick, so a
/// top-of-loop check loses the race every time and the stage never
/// gets its poster frame — the pipe died looking like a missing
/// ffmpeg. One frame always escapes; THEN the kill wins.
fn read_clip(shared: &ClipShared, path: &Path, start: f32, grade: &PipeGrade) {
    let mut cmd = Command::new(ffmpeg_program());
    cmd.args(clip_pipe_args(path, start, grade))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let child = dpl_proc::quiet(&mut cmd).spawn();
    let Ok(mut child) = child else {
        shared.ended.store(true, Ordering::Release);
        return;
    };
    let mut out = child.stdout.take().expect("piped stdout");
    let mut buf = vec![0u8; FRAME_BYTES];
    loop {
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
        if shared.stop.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            return;
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
        let args = clip_pipe_args(Path::new("s.mp4"), 12.5, &PipeGrade::default());
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

    /// L4: a LUT rides the -vf chain; a matte becomes a second input at
    /// the SAME seek, grayed and alphamerged at decode scale — the
    /// preview is exact, one pipe either way.
    #[test]
    fn graded_pipes_carry_lut_and_matte_exactly() {
        let g = PipeGrade {
            lut: Some("C:\\g\\cc.cube".into()),
            matte: None,
        };
        let joined = clip_pipe_args(Path::new("s.mp4"), 0.0, &g).join(" ");
        assert!(
            joined.contains(",lut3d='C\\:/g/cc.cube'"),
            "escaped grade in -vf: {joined}"
        );
        assert!(joined.contains("-vf"), "single input keeps -vf: {joined}");

        let g = PipeGrade {
            lut: None,
            matte: Some("m.mov".into()),
        };
        let joined = clip_pipe_args(Path::new("s.mp4"), 2.0, &g).join(" ");
        assert!(
            joined.contains("-ss 2.000 -i s.mp4 -ss 2.000 -i m.mov"),
            "matte seeks WITH the clip: {joined}"
        );
        assert!(
            joined.contains(",format=gray[m];[f][m]alphamerge[out]"),
            "{joined}"
        );
        assert!(joined.contains("-map [out]"), "{joined}");
        assert!(joined.ends_with("-pix_fmt rgba -"), "{joined}");
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
            grade: PipeGrade::default(),
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
