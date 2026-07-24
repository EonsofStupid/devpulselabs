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
//! - Filing happens exactly once in `save`: rename-uniquify the temp folder,
//!   write `guide.json`. (Sidecar + artifact index arrive with the PV1c
//!   inbox/index port — tracked in docs/plans/port-viziwiz.md.)

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use dpl_engine_capture as engine;

pub mod edit;

/// Lossy-WebP quality for step images (carried: visually lossless for UI).
const STEP_QUALITY: f32 = 82.0;
/// Settle between the "do" and "done" frames (carried).
const SETTLE_MS: u64 = 250;

// ── the forced-metadata law (ported verbatim) ───────────────────────────

/// One metadata implementation for every capture vehicle: kebab name with
/// auto `_v1`, ≥1 kebab tag, episode/General destination.
#[derive(Clone, Debug, Deserialize, Serialize)]
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

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
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

struct Session {
    dir: PathBuf,
    steps: Vec<StepRec>,
}

#[derive(Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct StepsState {
    pub recording: bool,
    pub count: u32,
}

#[derive(Serialize, Clone)]
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
static TRIGGERS: OnceLock<Sender<Trigger>> = OnceLock::new();

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

pub struct StepsRecorder {
    inner: Mutex<Option<Session>>,
    running: Arc<AtomicBool>,
    sink: EventSink,
}

impl StepsRecorder {
    pub fn new(sink: EventSink) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
            sink,
        })
    }

    fn emit_state(&self, recording: bool, count: u32) {
        (self.sink)(StepsEvent::State { recording, count });
    }

    fn emit_skip(&self, reason: &str) {
        eprintln!("[steps] skip: {reason}");
        (self.sink)(StepsEvent::Skip(reason.to_string()));
    }

    /// Begin recording into `<inbox>/<auto-name>`. The 5s prep countdown is
    /// the UI's job BEFORE calling this, carried.
    pub fn start(
        self: &Arc<Self>,
        inbox: &std::path::Path,
        meta: CaptureMeta,
    ) -> Result<(), String> {
        if self.running.load(Ordering::Relaxed) {
            return Err("already recording steps".into());
        }
        // A stopped-but-never-filed PENDING session must not leak.
        if let Some(old) = self.inner.lock().expect("steps lock").take() {
            let _ = std::fs::remove_dir_all(&old.dir);
        }
        let meta = normalize_meta_auto(meta, "guide")?;
        let dir = inbox.join(&meta.name);
        std::fs::create_dir_all(&dir).map_err(|e| format!("guide dir: {e}"))?;
        *self.inner.lock().expect("steps lock") = Some(Session {
            dir,
            steps: Vec::new(),
        });
        ensure_hook(self.clone());
        self.running.store(true, Ordering::Relaxed);
        self.emit_state(true, 0);
        Ok(())
    }

    /// The Mark button / hotkey — always kept, never deduped.
    pub fn mark(&self) -> Result<(), String> {
        if !self.running.load(Ordering::Relaxed) {
            return Err("not recording".into());
        }
        if let Some(tx) = TRIGGERS.get() {
            let _ = tx.send(Trigger::Marked);
        }
        Ok(())
    }

    /// Stop; the session stays PENDING for the Save card.
    pub fn stop(&self) -> Result<StopResult, String> {
        self.running.store(false, Ordering::Relaxed);
        let mut guard = self.inner.lock().expect("steps lock");
        let Some(sess) = guard.as_ref() else {
            return Err("not recording".into());
        };
        if sess.steps.is_empty() {
            let dir = sess.dir.clone();
            *guard = None;
            drop(guard);
            let _ = std::fs::remove_dir_all(&dir);
            self.emit_state(false, 0);
            return Err("no steps captured".into());
        }
        let count = sess.steps.len() as u32;
        let seed_title = sess
            .steps
            .first()
            .map(|s| s.title.clone())
            .unwrap_or_default();
        drop(guard);
        self.emit_state(false, count);
        Ok(StopResult { count, seed_title })
    }

    /// File the pending guide EXACTLY ONCE: rename-uniquify + guide.json.
    pub fn save(&self, inbox: &std::path::Path, meta: CaptureMeta) -> Result<String, String> {
        let meta = normalize_meta(meta)?;
        let mut sess = match self.inner.lock().expect("steps lock").take() {
            Some(s) => s,
            None => return Err("no guide to save".into()),
        };
        if sess.steps.is_empty() {
            let _ = std::fs::remove_dir_all(&sess.dir);
            return Err("no steps captured".into());
        }
        let mut final_dir = inbox.join(&meta.name);
        let mut n = 2u32;
        while final_dir.exists() && final_dir != sess.dir {
            final_dir = inbox.join(format!("{}-{n}", meta.name));
            n += 1;
        }
        if final_dir != sess.dir {
            std::fs::rename(&sess.dir, &final_dir).map_err(|e| format!("rename guide: {e}"))?;
            sess.dir = final_dir;
        }
        // The folder name is authoritative (it may have been uniquified).
        let name = sess
            .dir
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or(meta.name);

        let total: usize = sess.steps.iter().map(|s| s.bytes).sum();
        let manifest = serde_json::json!({
            "kind": "guide",
            "name": name,
            "stepCount": sess.steps.len(),
            "totalBytes": total,
            "steps": sess.steps,
        });
        std::fs::write(
            sess.dir.join("guide.json"),
            serde_json::to_vec_pretty(&manifest).map_err(|e| format!("guide manifest: {e}"))?,
        )
        .map_err(|e| format!("guide.json: {e}"))?;
        self.emit_state(false, 0);
        (self.sink)(StepsEvent::Saved { name: name.clone() });
        Ok(name)
    }

    /// Drop the pending guide without filing (the Save card's Discard).
    pub fn discard(&self) {
        if let Some(sess) = self.inner.lock().expect("steps lock").take() {
            let _ = std::fs::remove_dir_all(&sess.dir);
        }
        self.emit_state(false, 0);
    }

    pub fn status(&self) -> StepsState {
        let count = self
            .inner
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|s| s.steps.len() as u32))
            .unwrap_or(0);
        StepsState {
            recording: self.running.load(Ordering::Relaxed),
            count,
        }
    }

    /// One step, on the worker thread (carried two-frame model; the session
    /// lock is released across the settle so status/stop never block).
    fn capture_step(&self, trigger: Trigger) {
        let our_pid = std::process::id();
        let (dir, order) = {
            let guard = match self.inner.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            match guard.as_ref() {
                Some(sess) => (sess.dir.clone(), sess.steps.len() as u32 + 1),
                None => return,
            }
        };
        let action_str = match trigger {
            Trigger::Marked => "marked",
            Trigger::KeyAction => "key",
            Trigger::Click { .. } => "click",
        };

        let file = format!("step-{order:03}.webp");
        let path = dir.join(&file);
        let shot = match engine::capture_focused_window_step(&path, STEP_QUALITY, our_pid) {
            Ok(s) => s,
            Err(e) => {
                let reason = if e.contains("no external window") {
                    "switch to the app you're demonstrating".to_string()
                } else {
                    e
                };
                self.emit_skip(&reason);
                return;
            }
        };
        let click = trigger
            .click_point()
            .and_then(|(gx, gy)| relative_click(gx, gy, &shot));

        let mut result_file = None;
        let mut total_bytes = shot.bytes;
        if !trigger.is_marked() {
            std::thread::sleep(Duration::from_millis(SETTLE_MS));
            let rfile = format!("step-{order:03}-done.webp");
            let rpath = dir.join(&rfile);
            match engine::capture_focused_window_step(&rpath, STEP_QUALITY, our_pid) {
                Ok(done) => {
                    if engine::fingerprint_distance(shot.hash, done.hash) == 0 {
                        let _ = std::fs::remove_file(&path);
                        let _ = std::fs::remove_file(&rpath);
                        self.emit_skip("nothing changed");
                        return;
                    }
                    result_file = Some(rfile);
                    total_bytes += done.bytes;
                }
                Err(_) => {
                    let _ = std::fs::remove_file(&rpath);
                }
            }
        }

        let mut guard = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let discard = || {
            let _ = std::fs::remove_file(&path);
            if let Some(ref rf) = result_file {
                let _ = std::fs::remove_file(dir.join(rf));
            }
        };
        let Some(sess) = guard.as_mut() else {
            discard();
            return;
        };
        if sess.dir != dir {
            discard();
            return;
        }
        sess.steps.push(StepRec {
            order,
            file,
            result_file: result_file.clone(),
            action: action_str.into(),
            click: click.map(|(x, y)| [x, y]),
            app: shot.app.clone(),
            title: shot.title.clone(),
            width: shot.width,
            height: shot.height,
            bytes: total_bytes,
        });
        let count = sess.steps.len() as u32;
        drop(guard);
        self.emit_state(true, count);
    }
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

/// Install the global hook + capture worker exactly once (carried design).
/// Call at BOOT with a sink so the chords work before any recording;
/// hotkeys fire regardless of the `running` gate — that is their point.
pub fn install_hook(recorder: &Arc<StepsRecorder>, hotkeys: Option<HotkeySink>) {
    if TRIGGERS.get().is_some() {
        return;
    }
    let (tx, rx) = channel::<Trigger>();
    if TRIGGERS.set(tx).is_err() {
        return;
    }
    let worker = recorder.clone();
    std::thread::spawn(move || {
        while let Ok(trigger) = rx.recv() {
            worker.capture_step(trigger);
        }
    });
    let running = recorder.running.clone();
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
            if !running.load(Ordering::Relaxed) {
                return;
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
                if let Some(tx) = TRIGGERS.get() {
                    let _ = tx.send(t);
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
    fn pending_guide_files_once_with_rename_uniquify() {
        let sink: EventSink = Arc::new(|_| {});
        let rec = StepsRecorder::new(sink);
        let inbox = std::env::temp_dir().join("dpl-steps-test-inbox");
        let _ = std::fs::remove_dir_all(&inbox);
        std::fs::create_dir_all(&inbox).unwrap();

        // Seed a fake pending session (no display needed).
        let dir = inbox.join("guide-tmp");
        std::fs::create_dir_all(&dir).unwrap();
        *rec.inner.lock().unwrap() = Some(Session {
            dir,
            steps: vec![StepRec {
                order: 1,
                file: "step-001.webp".into(),
                result_file: None,
                action: "marked".into(),
                click: None,
                app: "Test".into(),
                title: "Test - App".into(),
                width: 10,
                height: 10,
                bytes: 123,
            }],
        });
        // An existing folder with the target name forces the -2 uniquify.
        std::fs::create_dir_all(inbox.join("my-guide_v1")).unwrap();
        let meta = CaptureMeta {
            library: true,
            show: String::new(),
            episode: String::new(),
            name: "my-guide".into(),
            tags: vec!["t".into()],
        };
        let name = rec.save(&inbox, meta).unwrap();
        assert_eq!(name, "my-guide_v1-2", "never clobber: uniquified");
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(inbox.join(&name).join("guide.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["kind"], "guide");
        assert_eq!(manifest["stepCount"], 1);
        assert!(
            rec.save(
                &inbox,
                CaptureMeta {
                    library: true,
                    show: String::new(),
                    episode: String::new(),
                    name: "x".into(),
                    tags: vec!["t".into()],
                }
            )
            .is_err(),
            "filing happens exactly once"
        );
        let _ = std::fs::remove_dir_all(&inbox);
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
