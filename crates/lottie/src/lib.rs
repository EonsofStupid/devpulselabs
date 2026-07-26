//! dpl-lottie — the app-facing Lottie façade. Native, AE-independent
//! motion-graphics rendering: it SPAWNS the `dpl-lottie-render` subprocess
//! helper (ADR-009, the ffmpeg/aerender pattern) and NEVER links ThorVG,
//! so this crate — and every crate that depends on it — stays pure-Rust
//! and `unsafe`-free. The helper owns all the C++ FFI.
//!
//! Two shapes: `Player` opens the helper in `serve` mode and streams a
//! frame at time `t` on demand (the timeline scrub path — the `ClipPlayer`
//! analog); `export_sequence` runs the helper's batch mode for ffmpeg.
//!
//! Scope law (`docs/plans/native-lottie.md`): Lottie carries shapes,
//! keyframes, masks, mattes, gradients, and text — NOT effects, 3D, or
//! expressions (those stay the AE orchestrator's job).

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// The render helper's binary name.
fn helper_name() -> &'static str {
    if cfg!(windows) {
        "dpl-lottie-render.exe"
    } else {
        "dpl-lottie-render"
    }
}

/// Locate the helper: beside the running exe first (where the app ships it,
/// like ffmpeg/aerender), then a `DPL_LOTTIE_RENDER` override, else PATH.
pub fn helper_path() -> PathBuf {
    if let Ok(p) = std::env::var("DPL_LOTTIE_RENDER") {
        return PathBuf::from(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(helper_name());
            if p.is_file() {
                return p;
            }
        }
    }
    PathBuf::from(helper_name())
}

/// One rendered frame — packed RGBA8, row-major (upload as an egui texture
/// or feed ffmpeg rawvideo).
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// A live Lottie preview: one persistent helper process in `serve` mode.
/// `frame_at(t)` writes the time and reads back exactly one frame — the
/// timeline scrubs this without re-spawning per frame.
pub struct Player {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub total_frames: f32,
}

impl Player {
    /// Open `lottie` at a fixed render size, spawning the helper and
    /// reading its `READY <w> <h> <fps> <total>` header.
    pub fn open(lottie: &Path, w: u32, h: u32) -> Result<Self, String> {
        let mut child = Command::new(helper_path())
            .arg("serve")
            .arg(lottie)
            .arg(w.to_string())
            .arg(h.to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn lottie helper: {e}"))?;

        let stdin = child.stdin.take().ok_or("helper stdin")?;
        let mut stdout = BufReader::new(child.stdout.take().ok_or("helper stdout")?);

        let mut header = String::new();
        stdout
            .read_line(&mut header)
            .map_err(|e| format!("read READY: {e}"))?;
        let (width, height, fps, total) =
            parse_ready(&header).ok_or_else(|| format!("bad READY header: {header:?}"))?;

        Ok(Self {
            child,
            stdin,
            stdout,
            width,
            height,
            fps,
            total_frames: total,
        })
    }

    /// Render the frame at time `t` (seconds). Writes the request and reads
    /// exactly `width * height * 4` RGBA bytes back.
    pub fn frame_at(&mut self, t_secs: f32) -> Result<Frame, String> {
        writeln!(self.stdin, "{t_secs}").map_err(|e| format!("request frame: {e}"))?;
        self.stdin.flush().ok();
        let n = (self.width as usize) * (self.height as usize) * 4;
        let mut rgba = vec![0u8; n];
        self.stdout
            .read_exact(&mut rgba)
            .map_err(|e| format!("read frame bytes: {e}"))?;
        Ok(Frame {
            width: self.width,
            height: self.height,
            rgba,
        })
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        // Ask the serve loop to quit, then reap.
        let _ = writeln!(self.stdin, "q");
        let _ = self.child.wait();
    }
}

/// Batch-render `lottie` to a PNG sequence at `fps` under `out_dir`
/// (`frame-%05d.png`) for ffmpeg — the export path. Returns the helper's
/// stdout note.
pub fn export_sequence(
    lottie: &Path,
    w: u32,
    h: u32,
    fps: f32,
    out_dir: &Path,
) -> Result<String, String> {
    let out = Command::new(helper_path())
        .arg("export")
        .arg(lottie)
        .arg(w.to_string())
        .arg(h.to_string())
        .arg(fps.to_string())
        .arg(out_dir)
        .output()
        .map_err(|e| format!("spawn lottie export: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "lottie export failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `READY <w> <h> <fps> <total_frames>` → the four numbers.
fn parse_ready(line: &str) -> Option<(u32, u32, f32, f32)> {
    let mut it = line.split_whitespace();
    if it.next()? != "READY" {
        return None;
    }
    Some((
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_ready_header() {
        let (w, h, fps, total) = parse_ready("READY 800 800 25.0000 80\n").unwrap();
        assert_eq!((w, h), (800, 800));
        assert!((fps - 25.0).abs() < 1e-3);
        assert!((total - 80.0).abs() < 1e-6);
        assert!(parse_ready("garbage").is_none());
        assert!(parse_ready("READY 800").is_none());
    }

    #[test]
    fn helper_name_is_platform_correct() {
        let n = helper_name();
        assert!(n.starts_with("dpl-lottie-render"));
        assert_eq!(n.ends_with(".exe"), cfg!(windows));
    }
}
