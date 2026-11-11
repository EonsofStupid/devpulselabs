//! Steps recorder engine — the ViziWiz "Steps" capability, ported from the
//! prior repo's `commands/steps.rs` + the metadata law from
//! `replay_capture.rs`, with the Tauri layer replaced by an event SINK the
//! embedding shell bridges onto the fabric (real events only — a recorder
//! that skipped a step says so; nothing is ever synthesized).
//!
//! Carried design notes that matter:
//! - `rdev::listen` never returns and has no stop API, so the hook installs
//!   ONCE and is gated by an atomic flag. The callback only forwards a
//!   trigger; every capture runs on a worker thread (a slow global hook makes
//!   the whole desktop feel laggy).
//! - Two-frame model: the "do" frame fires the instant the action lands
//!   (cursor at the target → accurate highlight); real actions then settle
//!   250ms and shoot the "done" frame — kept only if the screen actually
//!   changed, which IS the no-op dedup. A MARKED moment is always kept.
//! - Stop acknowledges a quiescent capture barrier. Save publishes a copy
//!   through durable checkpoints/receipts; original source bytes remain.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use dpl_engine_capture as engine;

pub mod edit;
mod lifecycle;
mod storage;
pub use lifecycle::StepsRecorder;
pub use storage::{RecoveryCandidate, RecoveryScan, SavedGuide};

/// Lossy-WebP quality for step images (carried: visually lossless for UI).
const STEP_QUALITY: f32 = 82.0;
/// Settle between the "do" and "done" frames (carried).
const SETTLE_MS: u64 = 250;

// ── the forced-metadata law (ported verbatim) ───────────────────────────

/// One metadata implementation for every capture vehicle: kebab name with
/// auto `_v1`, ≥1 kebab tag, episode/General destination.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct CaptureMeta {
    pub library: bool,
    #[serde(default)]
    pub show: String,
    #[serde(default)]
    pub episode: String,
    pub name: String,
    pub tags: Vec<String>,
}

fn kebab_ok(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_'))
}

fn v_suffix(name: &str) -> bool {
    name.rsplit_once("_v")
        .map(|(_, tail)| !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()))
        .unwrap_or(false)
}

/// `YYYYMMDD-HHMMSS` (UTC) — the one timestamp every capture vehicle
/// names artifacts with (auto capture names here; session folders, S1).
pub fn compact_timestamp() -> String {
    let t = time::OffsetDateTime::now_utc();
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    )
}

/// The strict manual-form path.
pub fn normalize_meta(mut meta: CaptureMeta) -> Result<CaptureMeta, String> {
    if !kebab_ok(&meta.name) {
        return Err("Capture name is required (kebab-case).".into());
    }
    if !v_suffix(&meta.name) {
        meta.name = format!("{}_v1", meta.name);
    }
    meta.tags.retain(|t| kebab_ok(t));
    if meta.tags.is_empty() {
        return Err("At least one tag is required.".into());
    }
    if !meta.library && (!kebab_ok(&meta.show) || !kebab_ok(&meta.episode)) {
        return Err("Pick an episode or the General library.".into());
    }
    Ok(meta)
}

/// The palette/auto path: auto-save never blocks on a name. A blank name
/// becomes `<prefix>-<yyyymmdd-hhmmss>` (the timestamp IS the version);
/// empty tags default to ["unsorted"]. A NAMED capture routes through the
/// strict path (gets `_v1`).
pub fn normalize_meta_auto(mut meta: CaptureMeta, prefix: &str) -> Result<CaptureMeta, String> {
    if meta.tags.is_empty() {
        meta.tags = vec!["unsorted".into()];
    }
    if meta.name.trim().is_empty() {
        meta.name = format!("{prefix}-{}", compact_timestamp());
        if !meta.library && (!kebab_ok(&meta.show) || !kebab_ok(&meta.episode)) {
            return Err("Pick an episode or the General library.".into());
        }
        return Ok(meta);
    }
    normalize_meta(meta)
}

// ── the recorder ────────────────────────────────────────────────────────

/// Real recorder events, bridged onto the fabric by the embedding shell.
#[derive(Clone, Debug)]
pub enum StepsEvent {
    State {
        recording: bool,
        count: u32,
    },
    /// A click did NOT become a step, and why — never a silently frozen
    /// counter (carried law).
    Skip(String),
    Saved {
        name: String,
    },
}

/// Called in recorder event order. Sinks must only enqueue/publish events;
/// they must not call lifecycle methods. Reentrant status() is supported.
pub type EventSink = Arc<dyn Fn(StepsEvent) + Send + Sync>;

#[derive(Clone, Copy)]
pub enum Trigger {
    Click { x: f64, y: f64 },
    KeyAction,
    Marked,
}

impl Trigger {
    fn is_marked(self) -> bool {
        matches!(self, Trigger::Marked)
    }
    fn click_point(self) -> Option<(f64, f64)> {
        match self {
            Trigger::Click { x, y } => Some((x, y)),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StepRec {
    pub order: u32,
    pub file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_file: Option<String>,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click: Option<[f64; 2]>,
    pub app: String,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub bytes: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct Session {
    pub(crate) dir: PathBuf,
    pub(crate) steps: Vec<StepRec>,
    pub(crate) next_attempt: u64,
    /// The OS releases this source lease after the final session clone drops.
    pub(crate) _lease: Arc<std::fs::File>,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct StepsState {
    pub recording: bool,
    pub count: u32,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StopResult {
    pub count: u32,
    /// First captured window's title — seeds the Save-card name.
    pub seed_title: String,
}

/// Latest global mouse position (rdev's ButtonPress carries no coordinates).
static MOUSE_X: AtomicI32 = AtomicI32::new(0);
static MOUSE_Y: AtomicI32 = AtomicI32::new(0);
/// Live Ctrl/Shift state, tracked from global press/release — rdev events
/// carry no modifier field, so the hook remembers (C2).
static CTRL_DOWN: AtomicBool = AtomicBool::new(false);
static SHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static HOOK_RECORDER: OnceLock<Mutex<Weak<StepsRecorder>>> = OnceLock::new();
static HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);

/// The OS-global chords (C2) — the old app's three shortcuts, now ridden
/// on the SAME rdev hook as the click capture (no shortcut plugin).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hotkey {
    /// Ctrl+Shift+S — summon/dismiss the capture palette.
    Palette,
    /// Ctrl+Shift+9 — start/stop the Steps recorder from any app.
    ToggleSteps,
    /// Ctrl+Shift+X — mark THIS moment as a step (always kept).
    MarkStep,
}

/// Receives chords on the HOOK thread — push to a queue and request a
/// repaint; never do real work here (a slow global hook lags the desktop).
pub type HotkeySink = Arc<dyn Fn(Hotkey) + Send + Sync>;

/// Pure chord map — unit-testable without OS events.
fn chord_hotkey(key: rdev::Key, ctrl: bool, shift: bool) -> Option<Hotkey> {
    if !(ctrl && shift) {
        return None;
    }
    match key {
        rdev::Key::KeyS => Some(Hotkey::Palette),
        rdev::Key::Num9 => Some(Hotkey::ToggleSteps),
        rdev::Key::KeyX => Some(Hotkey::MarkStep),
        _ => None,
    }
}

/// Track a modifier press/release; true when the key WAS a modifier.
fn track_modifier(key: rdev::Key, down: bool) -> bool {
    let flag = match key {
        rdev::Key::ControlLeft | rdev::Key::ControlRight => &CTRL_DOWN,
        rdev::Key::ShiftLeft | rdev::Key::ShiftRight => &SHIFT_DOWN,
        _ => return false,
    };
    flag.store(down, Ordering::Relaxed);
    true
}

/// Map a global click point into a shot as a normalized (0..1) point.
/// None when the click clearly wasn't inside the captured window (carried).
fn relative_click(gx: f64, gy: f64, shot: &engine::StepShot) -> Option<(f64, f64)> {
    let (w, h) = (f64::from(shot.width), f64::from(shot.height));
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let fx = (gx - f64::from(shot.origin_x)) / w;
    let fy = (gy - f64::from(shot.origin_y)) / h;
    if !(-0.5..=1.5).contains(&fx) || !(-0.5..=1.5).contains(&fy) {
        return None;
    }
    Some((fx.clamp(0.0, 1.0), fy.clamp(0.0, 1.0)))
}

/// The lazy fallback `start()` uses — hotkeys need the boot-time install.
fn ensure_hook(recorder: Arc<StepsRecorder>) {
    install_hook(&recorder, None);
}

/// Install the global hook once; each recorder owns its bounded worker queue.
/// Call at BOOT with a sink so the chords work before any recording;
/// hotkeys fire regardless of the `running` gate — that is their point.
pub fn install_hook(recorder: &Arc<StepsRecorder>, hotkeys: Option<HotkeySink>) {
    *HOOK_RECORDER
        .get_or_init(|| Mutex::new(Weak::new()))
        .lock()
        .expect("hook recorder lock") = Arc::downgrade(recorder);
    if HOOK_INSTALLED.swap(true, Ordering::AcqRel) {
        return;
    }
    std::thread::spawn(move || {
        if let Err(e) = rdev::listen(move |event| {
            match event.event_type {
                rdev::EventType::MouseMove { x, y } => {
                    MOUSE_X.store(x as i32, Ordering::Relaxed);
                    MOUSE_Y.store(y as i32, Ordering::Relaxed);
                    return;
                }
                rdev::EventType::KeyPress(key) => {
                    if track_modifier(key, true) {
                        return;
                    }
                    let ctrl = CTRL_DOWN.load(Ordering::Relaxed);
                    let shift = SHIFT_DOWN.load(Ordering::Relaxed);
                    if let Some(hotkey) = chord_hotkey(key, ctrl, shift) {
                        if let Some(sink) = &hotkeys {
                            sink(hotkey);
                        }
                        return; // a chord is never a step trigger
                    }
                }
                rdev::EventType::KeyRelease(key) => {
                    track_modifier(key, false);
                    return;
                }
                _ => {}
            }
            let trigger = match event.event_type {
                rdev::EventType::ButtonPress(rdev::Button::Left) => Some(Trigger::Click {
                    x: f64::from(MOUSE_X.load(Ordering::Relaxed)),
                    y: f64::from(MOUSE_Y.load(Ordering::Relaxed)),
                }),
                rdev::EventType::KeyPress(rdev::Key::Return) => Some(Trigger::KeyAction),
                _ => None,
            };
            if let Some(t) = trigger {
                // Never wait for a recorder lock or invoke an event sink on
                // the OS hook thread. Rebinding briefly drops hook events.
                if let Some(recorder) = HOOK_RECORDER
                    .get()
                    .and_then(|slot| slot.try_lock().ok())
                    .and_then(|slot| slot.upgrade())
                {
                    recorder.submit_hook(t);
                }
            }
        }) {
            eprintln!("[steps] global input hook failed: {e:?}");
        }
    });
}

/// Send a REAL Ctrl+Shift+9 through the OS (`rdev::simulate`) — the
/// hotkey drill's input side. Real input, never a faked hook event; the
/// settles between events are rdev's own guidance.
pub fn simulate_toggle_chord() -> Result<(), String> {
    use rdev::{simulate, EventType, Key};
    let sequence = [
        EventType::KeyPress(Key::ControlLeft),
        EventType::KeyPress(Key::ShiftLeft),
        EventType::KeyPress(Key::Num9),
        EventType::KeyRelease(Key::Num9),
        EventType::KeyRelease(Key::ShiftLeft),
        EventType::KeyRelease(Key::ControlLeft),
    ];
    for ev in sequence {
        simulate(&ev).map_err(|e| format!("simulate {ev:?}: {e:?}"))?;
        std::thread::sleep(Duration::from_millis(30));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_metadata_law_holds() {
        let base = CaptureMeta {
            library: true,
            show: String::new(),
            episode: String::new(),
            name: "my-guide".into(),
            tags: vec!["setup".into()],
        };
        assert_eq!(normalize_meta(base.clone()).unwrap().name, "my-guide_v1");
        let versioned = CaptureMeta {
            name: "my-guide_v3".into(),
            ..base.clone()
        };
        assert_eq!(normalize_meta(versioned).unwrap().name, "my-guide_v3");
        let bad = CaptureMeta {
            name: "Bad Name".into(),
            ..base.clone()
        };
        assert!(normalize_meta(bad).is_err(), "kebab law");
        let auto = CaptureMeta {
            name: "  ".into(),
            tags: vec![],
            ..base
        };
        let auto = normalize_meta_auto(auto, "guide").unwrap();
        assert!(
            auto.name.starts_with("guide-"),
            "timestamp auto-name: {}",
            auto.name
        );
        assert_eq!(auto.tags, vec!["unsorted".to_string()], "auto tags default");
    }

    #[test]
    fn the_chords_map_and_miss_honestly() {
        use rdev::Key;
        assert_eq!(chord_hotkey(Key::KeyS, true, true), Some(Hotkey::Palette));
        assert_eq!(
            chord_hotkey(Key::Num9, true, true),
            Some(Hotkey::ToggleSteps)
        );
        assert_eq!(chord_hotkey(Key::KeyX, true, true), Some(Hotkey::MarkStep));
        assert_eq!(chord_hotkey(Key::Num9, true, false), None, "shift required");
        assert_eq!(chord_hotkey(Key::Num9, false, true), None, "ctrl required");
        assert_eq!(
            chord_hotkey(Key::Return, true, true),
            None,
            "not a chord key"
        );
        assert!(
            track_modifier(Key::ControlRight, true),
            "either ctrl tracks"
        );
        assert!(
            !track_modifier(Key::KeyS, true),
            "letters are not modifiers"
        );
        track_modifier(Key::ControlRight, false);
    }

    #[test]
    fn relative_click_maps_and_rejects() {
        let shot = engine::StepShot {
            app: String::new(),
            title: String::new(),
            width: 100,
            height: 100,
            origin_x: 50,
            origin_y: 50,
            bytes: 0,
            hash: 0,
        };
        assert_eq!(relative_click(100.0, 100.0, &shot), Some((0.5, 0.5)));
        assert_eq!(
            relative_click(1000.0, 100.0, &shot),
            None,
            "off-window click rejected"
        );
    }
}
