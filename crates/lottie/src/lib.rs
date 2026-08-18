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

pub mod controls;

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
        Self::open_with_fonts(lottie, w, h, None)
    }

    /// As [`Player::open`], with a directory of scalable fonts registered
    /// before rendering.
    ///
    /// **Text renders as NOTHING for a family ThorVG was never given, and it
    /// does not error** — measured 2026-08-07, a text fixture produced a
    /// blank frame at exit code 0. So a Lottie with text needs this, and the
    /// caller supplies the directory: this crate cannot reach ColdLight's
    /// font pool itself (ColdLight is egui-linked, and this crate is on the
    /// egui-free side of the line), and the pool is `include_bytes!`-embedded
    /// rather than on disk, so it must be staged by whoever owns it.
    pub fn open_with_fonts(
        lottie: &Path,
        w: u32,
        h: u32,
        fonts: Option<&Path>,
    ) -> Result<Self, String> {
        let mut cmd = Command::new(helper_path());
        dpl_proc::quiet(&mut cmd);
        cmd.arg("serve")
            .arg(lottie)
            .arg(w.to_string())
            .arg(h.to_string());
        if let Some(dir) = fonts {
            cmd.arg("--fonts").arg(dir);
        }
        let mut child = cmd
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
        self.frame_blurred(t_secs, 1, 0.0)
    }

    /// A frame WITH MOTION BLUR — `samples` sub-renders averaged across
    /// `span` seconds of shutter.
    ///
    /// The same integral ffmpeg's `tmix` computes on export, so the viewer
    /// stops promising blur in a badge and simply shows it. `samples <= 1`
    /// costs exactly what a plain frame costs.
    pub fn frame_blurred(&mut self, t_secs: f32, samples: u32, span: f32) -> Result<Frame, String> {
        if samples <= 1 || span <= 0.0 {
            writeln!(self.stdin, "{t_secs}")
        } else {
            writeln!(self.stdin, "{t_secs} {samples} {span}")
        }
        .map_err(|e| format!("request frame: {e}"))?;
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
    export_sequence_with_fonts(lottie, w, h, fps, out_dir, None)
}

/// As [`export_sequence`], registering a directory of scalable fonts first.
/// A Lottie carrying text needs this — see [`Player::open_with_fonts`].
pub fn export_sequence_with_fonts(
    lottie: &Path,
    w: u32,
    h: u32,
    fps: f32,
    out_dir: &Path,
    fonts: Option<&Path>,
) -> Result<String, String> {
    let mut cmd = Command::new(helper_path());
    dpl_proc::quiet(&mut cmd);
    cmd.arg("export")
        .arg(lottie)
        .arg(w.to_string())
        .arg(h.to_string())
        .arg(fps.to_string())
        .arg(out_dir);
    if let Some(dir) = fonts {
        cmd.arg("--fonts").arg(dir);
    }
    let out = cmd
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

/// A Lottie's own metadata, read from its Bodymovin JSON WITHOUT rendering
/// (no ThorVG, no subprocess) — the composer uses this to size the preview
/// at the animation's native aspect and to time the timeline layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Meta {
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    /// Bodymovin `ip`/`op` — in/out points in FRAMES.
    pub in_point: f32,
    pub out_point: f32,
}

impl Meta {
    /// Playable length in seconds.
    pub fn duration_seconds(&self) -> f32 {
        ((self.out_point - self.in_point) / self.fps.max(1.0)).max(0.0)
    }
}

/// Read `w`/`h`/`fr`/`ip`/`op` from a `.json` (raw Bodymovin) or `.lottie`
/// (dotLottie zip). None if the file isn't a readable Bodymovin doc.
pub fn lottie_meta(path: &Path) -> Option<Meta> {
    let bytes = read_bodymovin_json(path)?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let o = v.as_object()?;
    Some(Meta {
        width: o.get("w")?.as_u64()? as u32,
        height: o.get("h")?.as_u64()? as u32,
        fps: o.get("fr")?.as_f64()? as f32,
        in_point: o.get("ip").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32,
        out_point: o.get("op")?.as_f64()? as f32,
    })
}

/// Is this JSON a Bodymovin/Lottie doc? The import sniff for ambiguous
/// `.json` files — a Bodymovin top level has BOTH `fr` (frame rate) and a
/// `layers` array. Guards against importing comp.json/guide.json/slide.json.
pub fn is_bodymovin_json(bytes: &[u8]) -> bool {
    serde_json::from_slice::<serde_json::Value>(bytes)
        .ok()
        .and_then(|v| {
            let o = v.as_object()?;
            Some(
                o.get("fr").is_some_and(|f| f.is_number())
                    && o.get("layers").is_some_and(|l| l.is_array()),
            )
        })
        .unwrap_or(false)
}

/// The raw Bodymovin JSON bytes: a `.json` file directly, or the first
/// animation JSON inside a `.lottie` (dotLottie) zip.
fn read_bodymovin_json(path: &Path) -> Option<Vec<u8>> {
    let dotlottie = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("lottie"));
    if !dotlottie {
        return std::fs::read(path).ok();
    }
    let file = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    // dotLottie layout: manifest.json + animations/<id>.json. Pick the
    // first animation JSON (or any non-manifest .json).
    let names: Vec<String> = (0..zip.len())
        .filter_map(|i| zip.by_index(i).ok().map(|e| e.name().to_string()))
        .collect();
    let pick = names
        .iter()
        .find(|n| n.starts_with("animations/") && n.ends_with(".json"))
        .or_else(|| {
            names
                .iter()
                .find(|n| n.ends_with(".json") && !n.ends_with("manifest.json"))
        })?;
    let mut entry = zip.by_name(pick).ok()?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).ok()?;
    Some(buf)
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

    #[test]
    fn lottie_meta_reads_size_and_duration_from_json() {
        let dir = std::env::temp_dir().join(format!("dpl-lottie-meta-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("title.json");
        // 512x512 @ 30fps, frames 0..90 = 3.0s.
        std::fs::write(
            &p,
            br#"{"v":"5.7.0","fr":30,"ip":0,"op":90,"w":512,"h":512,"layers":[]}"#,
        )
        .unwrap();
        let m = lottie_meta(&p).unwrap();
        assert_eq!((m.width, m.height), (512, 512));
        assert!((m.fps - 30.0).abs() < 1e-6);
        assert!((m.duration_seconds() - 3.0).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bodymovin_sniff_accepts_lottie_and_rejects_other_json() {
        assert!(is_bodymovin_json(
            br#"{"fr":30,"op":90,"layers":[{"ty":4}]}"#
        ));
        // A comp.json-like doc: no fr/layers → rejected (protects app JSON).
        assert!(!is_bodymovin_json(br#"{"v":1,"tracks":[]}"#));
        assert!(!is_bodymovin_json(br#"{}"#));
        assert!(!is_bodymovin_json(b"not json at all"));
        // fr present but no layers array → not bodymovin.
        assert!(!is_bodymovin_json(br#"{"fr":30}"#));
    }
}
