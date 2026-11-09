//! One recording (RA1a, R294): the whole monitor, in segments. Pause ends the
//! segment, so the microphone stops with the picture, and resume starts the
//! next. One frame counter runs across every segment; anything that happens
//! during a recording is stamped from it, so events and video share one
//! timeline. `recording.json` (v1, the document law) says what the folder holds.

use crate::{ext_for_format, monitor_by_id, record_monitor_ctl, VideoOpts};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The folder's own description.
pub const MANIFEST: &str = "recording.json";
const VERSION: u32 = 1;
const FPS: u64 = 30;

const RECORDING: u8 = 0;
const PAUSED: u8 = 1;
const STOPPED: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecState {
    Recording,
    Paused,
    Stopped,
}

/// Shared by the recorder's thread and whatever drives it: the state it should
/// be in, and the frame clock everything is stamped from.
#[derive(Debug, Default)]
pub struct Recording {
    state: AtomicU8,
    frames: AtomicU64,
}

impl Recording {
    /// Pausing a recording that is already paused or stopped does nothing.
    pub fn pause(&self) {
        let _ = self
            .state
            .compare_exchange(RECORDING, PAUSED, Ordering::AcqRel, Ordering::Acquire);
    }

    /// Resuming only ever follows a pause; a stopped recording stays stopped.
    pub fn resume(&self) {
        let _ = self
            .state
            .compare_exchange(PAUSED, RECORDING, Ordering::AcqRel, Ordering::Acquire);
    }

    pub fn stop(&self) {
        self.state.store(STOPPED, Ordering::Release);
    }

    pub fn state(&self) -> RecState {
        match self.state.load(Ordering::Acquire) {
            RECORDING => RecState::Recording,
            PAUSED => RecState::Paused,
            _ => RecState::Stopped,
        }
    }

    /// The frame on screen now, on the recording's own timeline.
    pub fn frame(&self) -> u64 {
        self.frames.load(Ordering::Acquire)
    }

    /// Milliseconds on the recording's timeline: frames are paced to the wall
    /// clock at 30 a second, and a pause stops the clock.
    pub fn ms(&self) -> u64 {
        self.frame() * 1000 / FPS
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Screen {
    pub monitor: u32,
    pub width: u32,
    pub height: u32,
}

/// One stretch between a start (or resume) and a pause (or stop).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub file: String,
    /// Where this segment starts on the recording's timeline.
    pub first_frame: u64,
    pub frames: u64,
    /// Wall-clock seconds it ran, against `frames / 30`.
    pub seconds: f64,
    pub audio: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub v: u32,
    pub kind: String,
    pub screen: Screen,
    pub fps: u32,
    pub started_unix_ms: u64,
    pub segments: Vec<Segment>,
    /// Where the screen capture itself ended, if it did (a lock screen or a
    /// permission prompt can end it); the recording stopped there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture_lost_at_frame: Option<u64>,
}

impl Manifest {
    pub fn frames(&self) -> u64 {
        self.segments.iter().map(|s| s.frames).sum()
    }

    /// Read a recording's description, refusing a newer one honestly.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let path = dir.join(MANIFEST);
        let raw = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let manifest: Self =
            serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", path.display()))?;
        if manifest.v > VERSION {
            return Err(format!(
                "{} is from a newer DevPULSE (v{}); this one reads v{VERSION}",
                path.display(),
                manifest.v
            ));
        }
        Ok(manifest)
    }

    /// Pretty JSON and LF, through `.tmp`, with the old one stepped aside to
    /// `.bak` until the new one is in place (the document law).
    pub(crate) fn save(&self, dir: &Path) -> Result<(), String> {
        let dir = dpl_roots::writable(dir)?;
        let target = dir.join(MANIFEST);
        let tmp = dir.join(format!("{MANIFEST}.tmp"));
        let bak = dir.join(format!("{MANIFEST}.bak"));
        let mut json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        json.push('\n');
        std::fs::write(&tmp, json).map_err(|e| format!("{}: {e}", tmp.display()))?;
        if target.exists() {
            let _ = std::fs::remove_file(&bak);
            std::fs::rename(&target, &bak).map_err(|e| format!("{}: {e}", bak.display()))?;
        }
        std::fs::rename(&tmp, &target).map_err(|e| format!("{}: {e}", target.display()))?;
        let _ = std::fs::remove_file(&bak);
        Ok(())
    }
}

/// Record monitor `monitor` into `dir` until `rec` is stopped: a segment while
/// recording, nothing while paused. The description is saved after every
/// segment, so a crash keeps what was finished. Returns it once stopped.
pub fn record(
    dir: &Path,
    monitor: u32,
    opts: &VideoOpts,
    rec: &Recording,
) -> Result<Manifest, String> {
    let mon = monitor_by_id(monitor)?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut manifest = Manifest {
        v: VERSION,
        kind: "recording".into(),
        screen: Screen {
            monitor,
            width: mon.width().map_err(|e| format!("width: {e}"))?,
            height: mon.height().map_err(|e| format!("height: {e}"))?,
        },
        fps: FPS as u32,
        started_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or_default(),
        segments: Vec::new(),
        capture_lost_at_frame: None,
    };
    let ext = ext_for_format(&opts.format);
    let recording = || rec.state() == RecState::Recording;
    loop {
        match rec.state() {
            RecState::Stopped => break,
            RecState::Paused => {
                std::thread::sleep(Duration::from_millis(30));
                continue;
            }
            RecState::Recording => {}
        }
        let file = format!("segment-{:03}.{ext}", manifest.segments.len() + 1);
        let path = dir.join(&file);
        let first_frame = rec.frame();
        let out = path.to_string_lossy().into_owned();
        let segment = record_monitor_ctl(&mon, None, &out, opts, &recording, Some(&rec.frames))?;
        if segment.frames > 0 {
            manifest.segments.push(Segment {
                file,
                first_frame,
                frames: u64::from(segment.frames),
                seconds: segment.seconds,
                audio: opts.audio != "none",
            });
        }
        if segment.capture_lost_at.is_some() {
            manifest.capture_lost_at_frame = Some(rec.frame());
            rec.stop();
        }
        manifest.save(dir)?;
    }
    if manifest.segments.is_empty() {
        return Err("no screen frames were recorded".into());
    }
    manifest.save(dir)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_resume_and_stop_only_move_the_right_way() {
        let rec = Recording::default();
        assert_eq!(rec.state(), RecState::Recording);
        rec.resume();
        assert_eq!(
            rec.state(),
            RecState::Recording,
            "resume without a pause does nothing"
        );
        rec.pause();
        rec.pause();
        assert_eq!(rec.state(), RecState::Paused);
        rec.resume();
        assert_eq!(rec.state(), RecState::Recording);
        rec.stop();
        rec.resume();
        rec.pause();
        assert_eq!(
            rec.state(),
            RecState::Stopped,
            "a stopped recording stays stopped"
        );
    }

    #[test]
    fn the_clock_is_the_video_timeline() {
        let rec = Recording::default();
        rec.frames.store(45, Ordering::Release);
        assert_eq!((rec.frame(), rec.ms()), (45, 1500));
    }

    #[test]
    fn the_description_round_trips_and_refuses_a_newer_one() {
        let dir = std::env::temp_dir().join(format!("dpl-recording-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = Manifest {
            v: VERSION,
            kind: "recording".into(),
            screen: Screen {
                monitor: 1,
                width: 1920,
                height: 1080,
            },
            fps: 30,
            started_unix_ms: 1,
            segments: vec![
                Segment {
                    file: "segment-001.mp4".into(),
                    first_frame: 0,
                    frames: 90,
                    seconds: 3.0,
                    audio: true,
                },
                Segment {
                    file: "segment-002.mp4".into(),
                    first_frame: 90,
                    frames: 60,
                    seconds: 2.0,
                    audio: true,
                },
            ],
            capture_lost_at_frame: None,
        };
        // Written straight, since `save` asks the process-wide library first.
        let mut json = serde_json::to_string_pretty(&manifest).unwrap();
        json.push('\n');
        std::fs::write(dir.join(MANIFEST), json).unwrap();
        let back = Manifest::load(&dir).unwrap();
        assert_eq!(back, manifest);
        assert_eq!(back.frames(), 150);
        let raw = std::fs::read_to_string(dir.join(MANIFEST)).unwrap();
        assert!(raw.contains("\"firstFrame\": 90"), "{raw}");

        let newer = raw.replacen("\"v\": 1", "\"v\": 2", 1);
        std::fs::write(dir.join(MANIFEST), newer).unwrap();
        assert!(Manifest::load(&dir).unwrap_err().contains("newer"));
    }
}
