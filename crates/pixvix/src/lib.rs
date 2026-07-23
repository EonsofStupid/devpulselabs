//! PixVix — the capture overlay (PV2), rebuilt in egui on the ported
//! engines. The palette is a compact ALWAYS-ON-TOP viewport with the old
//! four tabs (Img · Video · Session · Steps); region select is a borderless
//! fullscreen viewport over the FROZEN frame (`freeze_primary_to`) with the
//! magnifier loupe — the old app's own pattern, no window transparency
//! needed. While Steps records, the palette collapses to the REC pill.
//!
//! Every action calls an engine function and reports honestly in the
//! status line. Filing: `content_root()/inbox`, never-clobber names.

use dpl_coldlight::{self as cl, Tone};
use dpl_engine_capture as cap;
use dpl_engine_steps::{normalize_meta_auto, CaptureMeta, StepsRecorder};
use dpl_fabric::{Fabric, ShellEvent};
use egui::{
    Align2, Color32, CornerRadius, FontId, Rect, RichText, Sense, ViewportBuilder, ViewportId,
};
use egui_pilot_ai::protocol::{ActionKind as PilotActionKind, PilotViewportKey, Role as PilotRole};
use egui_pilot_ai::{ContextPilotExt, NodeMetadata, PilotId, UiPilotExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const TONE: Tone = Tone::Magenta; // wiredFRONT's tone — the capture family.
const PALETTE_KEY: &str = "devpulse.pixvix.palette";
const REGION_KEY: &str = "devpulse.pixvix.region";
const PALETTE_TITLE: PilotId = PilotId::new("pixvix.title");
const PALETTE_CLOSE: PilotId = PilotId::new("pixvix.close");
const TAB_IMG: PilotId = PilotId::new("pixvix.tab.img");
const TAB_VIDEO: PilotId = PilotId::new("pixvix.tab.video");
const TAB_SESSION: PilotId = PilotId::new("pixvix.tab.session");
const TAB_STEPS: PilotId = PilotId::new("pixvix.tab.steps");
const REGION_OPEN: PilotId = PilotId::new("pixvix.region.open");
const SNAP_FULL: PilotId = PilotId::new("pixvix.snap.full");
const FREEFORM: PilotId = PilotId::new("pixvix.freeform");
const SNAP_WINDOW: PilotId = PilotId::new("pixvix.snap.window");
const REFRESH_WINDOWS: PilotId = PilotId::new("pixvix.windows.refresh");
const VIDEO_START: PilotId = PilotId::new("pixvix.video.start");
const VIDEO_STOP: PilotId = PilotId::new("pixvix.video.stop");
const SESSION_START: PilotId = PilotId::new("pixvix.session.start");
const SESSION_STOP: PilotId = PilotId::new("pixvix.session.stop");
const STEPS_START: PilotId = PilotId::new("pixvix.steps.start");
const STEPS_MARK: PilotId = PilotId::new("pixvix.steps.mark");
const STEPS_STOP: PilotId = PilotId::new("pixvix.steps.stop");
const REGION_DRAG: PilotId = PilotId::new("pixvix.region.drag");
const DELAY_CYCLE: PilotId = PilotId::new("pixvix.delay");
const RESNIP: PilotId = PilotId::new("pixvix.resnip");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Img,
    Video,
    Session,
    Steps,
}

impl Tab {
    const ALL: [Tab; 4] = [Tab::Img, Tab::Video, Tab::Session, Tab::Steps];
    fn icon(self) -> &'static str {
        match self {
            Tab::Img => cl::icon::IMG,
            Tab::Video => cl::icon::VIDEO,
            Tab::Session => cl::icon::SESSION,
            Tab::Steps => cl::icon::STEPS,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Tab::Img => "Img",
            Tab::Video => "Video",
            Tab::Session => "Session",
            Tab::Steps => "Steps",
        }
    }

    fn pilot_id(self) -> PilotId {
        match self {
            Self::Img => TAB_IMG,
            Self::Video => TAB_VIDEO,
            Self::Session => TAB_SESSION,
            Self::Steps => TAB_STEPS,
        }
    }
}

/// Which shape the frozen-frame editor is collecting (C1).
#[derive(Clone, Copy, PartialEq, Eq)]
enum SnipMode {
    /// Drag → armed rect → nudge (arrows, Shift=10px) → Enter/double-click.
    Rect,
    /// Click vertices → close on the first vertex or Enter (≥3);
    /// Backspace removes the last vertex.
    Free,
}

/// The frozen-frame editor's state (region AND freeform ride one
/// viewport — same freeze, same veil, same loupe).
struct RegionSelect {
    tex: egui::TextureHandle,
    frozen: PathBuf,
    mode: SnipMode,
    /// Rect mode: the ARMED selection (survives release for the nudge
    /// ceremony; a new drag replaces it).
    sel: Option<Rect>,
    /// Rect mode: the live drag's anchor while the button is down.
    dragging: Option<egui::Pos2>,
    /// Free mode: the collected vertices, screen space.
    poly: Vec<egui::Pos2>,
}

/// The last successful still's SHAPE — Re-snip re-fires it against a
/// fresh frame without re-dragging (C1).
#[derive(Clone)]
enum LastSnip {
    Full,
    Window(u32),
    Region(cap::FracRect),
    Free(Vec<[f64; 2]>),
}

/// A capture waiting on the delay timer (C1): what to do when it fires.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingSnip {
    Full,
    Window,
    OpenEditor(SnipMode),
    Resnip,
}

/// A running video recording (record_ctl_to on its own thread).
struct VideoRec {
    stop: Arc<AtomicBool>,
    started: Instant,
    out: PathBuf,
    done: Arc<Mutex<Option<Result<u32, String>>>>,
}

pub mod session;

pub struct Pixvix {
    open: bool,
    tab: Tab,
    status: String,
    rx: tokio::sync::broadcast::Receiver<ShellEvent>,
    /// Kept whole so a session can take a FRESH subscription at start —
    /// a broadcast receiver only sees traffic after it subscribes.
    fabric: Fabric,
    steps: Arc<StepsRecorder>,
    steps_note: Arc<Mutex<String>>,
    region: Option<RegionSelect>,
    video: Option<VideoRec>,
    /// The running session recording (S1), if any.
    session: Option<session::SessionRec>,
    windows: Vec<cap::WindowInfo>,
    picked_window: usize,
    /// The capture delay in seconds — one chip cycles 0 → 3 → 10 (C1).
    delay: u8,
    /// A snap waiting out the delay; the status line counts it down.
    delay_armed: Option<(Instant, PendingSnip)>,
    /// The last successful still's shape — Re-snip's memory.
    last: Option<LastSnip>,
}

impl Pixvix {
    pub fn new(fabric: &Fabric) -> Self {
        let steps_note = Arc::new(Mutex::new(String::new()));
        let note = Arc::clone(&steps_note);
        // Real recorder events only — the sink mirrors them into the status
        // line (the fabric bridge widens this in the wiring pass).
        let sink: dpl_engine_steps::EventSink = Arc::new(move |ev| {
            if let Ok(mut n) = note.lock() {
                *n = format!("{ev:?}");
            }
        });
        Self {
            open: false,
            tab: Tab::Img,
            status: String::new(),
            rx: fabric.subscribe_shell(),
            fabric: fabric.clone(),
            steps: StepsRecorder::new(sink),
            steps_note,
            region: None,
            video: None,
            session: None,
            windows: Vec::new(),
            picked_window: 0,
            delay: 0,
            delay_armed: None,
            last: None,
        }
    }

    /// X1 write-through: sidecar + index upsert for a fresh capture.
    /// Capture NEVER fails because indexing did — a problem comes back as
    /// a status-line suffix instead.
    fn index_capture(path: &Path, kind: &str) -> String {
        let root = cap::content_root();
        let record = dpl_index::record_for_capture(path, &root, kind, "pixvix");
        let result = dpl_index::sidecar::write(path, &record)
            .and_then(|()| dpl_index::Index::open_default()?.upsert(&record));
        match result {
            Ok(()) => String::new(),
            Err(e) => format!(" · index: {e}"),
        }
    }

    fn inbox() -> Result<PathBuf, String> {
        let dir = cap::content_root().join("inbox");
        std::fs::create_dir_all(&dir).map_err(|e| format!("inbox: {e}"))?;
        Ok(dir)
    }

    /// Never-clobber: `stem.ext`, then `stem-2.ext`, `stem-3.ext`…
    fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
        let first = dir.join(format!("{stem}.{ext}"));
        if !first.exists() {
            return first;
        }
        (2..)
            .map(|n| dir.join(format!("{stem}-{n}.{ext}")))
            .find(|p| !p.exists())
            .expect("unbounded range")
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Harness: open the palette at boot (`--pixvix`).
    pub fn open_now(&mut self) {
        self.open = true;
    }

    /// Harness: fire the Img/Full action through the real handler and
    /// report the status line (`--pixvix-snap` — Law 2 without a mouse).
    pub fn snap_full_now(&mut self, ctx: &egui::Context) -> &str {
        self.snap_full(ctx);
        &self.status
    }

    /// Harness: start a session through the REAL handler
    /// (`--session-drill` — Law 2 without a mouse).
    pub fn session_start_now(&mut self) -> &str {
        self.start_session();
        &self.status
    }

    /// Harness: request the running session's stop.
    pub fn session_stop_now(&mut self) {
        self.stop_session();
    }

    /// True while a session records or its video leg is still closing.
    pub fn session_active(&self) -> bool {
        self.session.is_some()
    }

    /// The honest status line — the drill's verdict source.
    pub fn status_text(&self) -> &str {
        &self.status
    }

    /// The steps hotkey (Ctrl+Shift+9): start when idle, stop+file when
    /// recording — the old toggle, in-app scope for now (ledger: OS-global).
    pub fn toggle_steps(&mut self) {
        if self.steps.status().recording {
            self.stop_steps();
        } else {
            self.start_steps();
        }
    }

    pub fn mark_step(&mut self) {
        if let Err(e) = self.steps.mark() {
            self.status = e;
        }
    }

    /// Blank meta for the auto path — normalize_meta_auto fills the name
    /// (`guide-<timestamp>`) and the default tag; the forced-metadata Save
    /// card is PV3.
    fn blank_meta() -> CaptureMeta {
        CaptureMeta {
            library: false,
            show: String::new(),
            episode: String::new(),
            name: String::new(),
            tags: Vec::new(),
        }
    }

    fn start_steps(&mut self) {
        match Self::inbox().and_then(|inbox| {
            let meta = normalize_meta_auto(Self::blank_meta(), "guide")?;
            self.steps.start(&inbox, meta)
        }) {
            Ok(()) => self.status = "steps: recording".into(),
            Err(e) => self.status = format!("steps: {e}"),
        }
    }

    fn stop_steps(&mut self) {
        match self.steps.stop() {
            Ok(stop) => {
                // Auto-file with generated meta; the forced-metadata Save
                // card arrives with PV3.
                let filed = Self::inbox().and_then(|inbox| {
                    let meta = normalize_meta_auto(Self::blank_meta(), "guide")?;
                    self.steps.save(&inbox, meta)
                });
                self.status = match filed {
                    Ok(folder) => {
                        let warn = Self::inbox()
                            .map(|inbox| Self::index_capture(&inbox.join(&folder), "guide"))
                            .unwrap_or_default();
                        format!("steps: {} steps → {folder}{warn}", stop.count)
                    }
                    Err(e) => format!("steps: stop ok, save failed: {e}"),
                };
            }
            Err(e) => self.status = format!("steps: {e}"),
        }
    }

    /// C1: land the written still on the OS clipboard too — egui's own
    /// image clipboard, no extra dependency. Failure is a status suffix,
    /// never a block.
    fn copy_to_clipboard(ctx: &egui::Context, path: &Path) -> String {
        match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let color = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    &rgba,
                );
                ctx.copy_image(color);
                " · copied".into()
            }
            Err(e) => format!(" · clipboard: {e}"),
        }
    }

    fn snap_full(&mut self, ctx: &egui::Context) {
        let result = Self::inbox().and_then(|inbox| {
            let primary = cap::list_targets()
                .0
                .into_iter()
                .find(|m| m.is_primary)
                .ok_or("no primary monitor")?;
            let out = Self::unique_path(&inbox, "pix", "png");
            cap::still_monitor_to(&out, primary.id)?;
            Ok(out)
        });
        self.status = match result {
            Ok(p) => {
                self.last = Some(LastSnip::Full);
                format!(
                    "saved {}{}{}",
                    p.display(),
                    Self::index_capture(&p, "image"),
                    Self::copy_to_clipboard(ctx, &p)
                )
            }
            Err(e) => e,
        };
    }

    fn snap_window(&mut self, ctx: &egui::Context) {
        let Some(win) = self.windows.get(self.picked_window) else {
            self.status = "pick a window first".into();
            return;
        };
        let (id, title) = (win.id, win.title.clone());
        self.snap_window_id(ctx, id, &title);
    }

    fn snap_window_id(&mut self, ctx: &egui::Context, id: u32, title: &str) {
        let result = Self::inbox().and_then(|inbox| {
            let out = Self::unique_path(&inbox, "pix", "png");
            cap::still_window_to(&out, id)?;
            Ok(out)
        });
        self.status = match result {
            Ok(p) => {
                self.last = Some(LastSnip::Window(id));
                format!(
                    "saved {} ({title}){}{}",
                    p.display(),
                    Self::index_capture(&p, "image"),
                    Self::copy_to_clipboard(ctx, &p)
                )
            }
            Err(e) => e,
        };
    }

    /// Re-snip (C1): re-fire the LAST successful still's shape against a
    /// fresh frame — no re-dragging. Region/Free go straight at the live
    /// screen (the engine's `still_*_to`); no freeze ceremony.
    fn resnip(&mut self, ctx: &egui::Context) {
        let Some(last) = self.last.clone() else {
            self.status = "nothing snapped yet — Re-snip repeats the last shape".into();
            return;
        };
        match last {
            LastSnip::Full => self.snap_full(ctx),
            LastSnip::Window(id) => self.snap_window_id(ctx, id, "re-snip"),
            LastSnip::Region(frac) => {
                let result = Self::inbox().and_then(|inbox| {
                    let out = Self::unique_path(&inbox, "pix", "png");
                    cap::still_region_to(&out, frac)?;
                    Ok(out)
                });
                self.status = match result {
                    Ok(p) => format!(
                        "saved {}{}{}",
                        p.display(),
                        Self::index_capture(&p, "image"),
                        Self::copy_to_clipboard(ctx, &p)
                    ),
                    Err(e) => e,
                };
            }
            LastSnip::Free(points) => {
                let result = Self::inbox().and_then(|inbox| {
                    let out = Self::unique_path(&inbox, "pix", "png");
                    cap::still_freeform_to(&out, &points)?;
                    Ok(out)
                });
                self.status = match result {
                    Ok(p) => format!(
                        "saved {}{}{}",
                        p.display(),
                        Self::index_capture(&p, "image"),
                        Self::copy_to_clipboard(ctx, &p)
                    ),
                    Err(e) => e,
                };
            }
        }
    }

    /// Fire a pending snap NOW, or arm the delay countdown (C1).
    fn arm_or_fire(&mut self, pending: PendingSnip, ctx: &egui::Context) {
        if self.delay == 0 {
            self.fire(pending, ctx);
        } else {
            let deadline = Instant::now() + std::time::Duration::from_secs(u64::from(self.delay));
            self.delay_armed = Some((deadline, pending));
            self.status = format!("snapping in {}s…", self.delay);
        }
    }

    fn fire(&mut self, pending: PendingSnip, ctx: &egui::Context) {
        match pending {
            PendingSnip::Full => self.snap_full(ctx),
            PendingSnip::Window => self.snap_window(ctx),
            PendingSnip::OpenEditor(mode) => self.begin_editor(ctx, mode),
            PendingSnip::Resnip => self.resnip(ctx),
        }
    }

    fn begin_editor(&mut self, ctx: &egui::Context, mode: SnipMode) {
        let result = (|| -> Result<RegionSelect, String> {
            let frozen = std::env::temp_dir().join("dpl-pixvix-freeze.png");
            cap::freeze_primary_to(&frozen)?;
            let img = image::open(&frozen).map_err(|e| format!("freeze decode: {e}"))?;
            let rgba = img.to_rgba8();
            let color = egui::ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                &rgba,
            );
            let tex = ctx.load_texture("pixvix-freeze", color, egui::TextureOptions::LINEAR);
            Ok(RegionSelect {
                tex,
                frozen,
                mode,
                sel: None,
                dragging: None,
                poly: Vec::new(),
            })
        })();
        match result {
            Ok(r) => self.region = Some(r),
            Err(e) => self.status = format!("freeze: {e}"),
        }
    }

    fn start_video(&mut self) {
        let result = Self::inbox().and_then(|inbox| {
            let primary = cap::list_targets()
                .0
                .into_iter()
                .find(|m| m.is_primary)
                .ok_or("no primary monitor")?;
            let out = Self::unique_path(&inbox, "vix", "mp4");
            let stop = Arc::new(AtomicBool::new(false));
            let done = Arc::new(Mutex::new(None));
            let (t_stop, t_done, t_out) = (Arc::clone(&stop), Arc::clone(&done), out.clone());
            std::thread::Builder::new()
                .name("pixvix-video".into())
                .spawn(move || {
                    let opts = cap::VideoOpts {
                        height: 1080,
                        format: "mp4".into(),
                        audio: "none".into(),
                        audio_device: None,
                        max_seconds: None,
                    };
                    let r = cap::record_ctl_to(
                        &t_out.to_string_lossy(),
                        cap::RecTarget::Monitor(primary.id),
                        &opts,
                        &t_stop,
                    );
                    if r.is_ok() {
                        let poster = t_out.with_extension("jpg");
                        cap::poster_for(&t_out.to_string_lossy(), &poster.to_string_lossy());
                    }
                    *t_done.lock().expect("video done lock") = Some(r);
                })
                .map_err(|e| format!("spawn: {e}"))?;
            Ok(VideoRec {
                stop,
                started: Instant::now(),
                out,
                done,
            })
        });
        match result {
            Ok(v) => {
                self.status = format!("recording {}", v.out.display());
                self.video = Some(v);
            }
            Err(e) => self.status = e,
        }
    }

    fn stop_video(&mut self) {
        if let Some(v) = &self.video {
            v.stop.store(true, Ordering::Release);
            self.status = format!("stopping {}", v.out.display());
        }
    }

    /// Start a SESSION (S1): the folder artifact, the video leg on its
    /// thread, and a fresh fabric subscription feeding events.jsonl.
    fn start_session(&mut self) {
        if self.session.is_some() {
            return;
        }
        let rx = self.fabric.subscribe_shell();
        let result = Self::inbox().and_then(|inbox| {
            let dir = session::session_dir(&inbox);
            std::fs::create_dir_all(&dir).map_err(|e| format!("session: {e}"))?;
            let events = session::EventLog::create(&dir.join("events.jsonl"))?;
            let stop = Arc::new(AtomicBool::new(false));
            let done = Arc::new(Mutex::new(None));
            session::spawn_video(&dir, Arc::clone(&stop), Arc::clone(&done))?;
            Ok(session::SessionRec {
                dir,
                stop,
                started: Instant::now(),
                done,
                rx,
                events,
            })
        });
        match result {
            Ok(s) => {
                self.status = format!("session recording {}", s.dir.display());
                self.session = Some(s);
            }
            Err(e) => self.status = e,
        }
    }

    fn stop_session(&mut self) {
        if let Some(s) = &self.session {
            s.stop.store(true, Ordering::Release);
            self.status = format!("stopping {}", s.dir.display());
        }
    }

    /// X1 write-through for a finished (or partial) session FOLDER —
    /// kind "session", poster when the video leg produced one. Same
    /// status-line degrade law as `index_capture`.
    fn index_session(dir: &Path) -> String {
        let root = cap::content_root();
        let mut record = dpl_index::record_for_capture(dir, &root, "session", "pixvix");
        if dir.join("session.jpg").is_file() {
            record.poster_rel_path = Some(format!("{}/session.jpg", record.rel_path));
        }
        let result = dpl_index::sidecar::write(dir, &record)
            .and_then(|()| dpl_index::Index::open_default()?.upsert(&record));
        match result {
            Ok(()) => String::new(),
            Err(e) => format!(" · index: {e}"),
        }
    }

    /// Drive the overlay for this frame: consume summons, then render the
    /// palette viewport (or the REC pill) and the region-select viewport.
    pub fn ui(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.rx.try_recv() {
            if ev == ShellEvent::PixvixSummon {
                self.open = true;
            }
        }

        // A finished video thread reports once.
        if let Some(v) = &self.video {
            let done = v.done.lock().expect("video done lock").take();
            if let Some(result) = done {
                let out = v.out.clone();
                self.status = match result {
                    Ok(frames) => format!(
                        "saved {} ({frames} frames){}",
                        out.display(),
                        Self::index_capture(&out, "video")
                    ),
                    Err(e) => format!("video: {e}"),
                };
                self.video = None;
            }
        }

        // The running session: drain observed fabric traffic into the
        // track every frame; a finished video leg files the folder once.
        // A video ERROR still files — events.jsonl makes it a partial
        // session, and files are the truth.
        if let Some(s) = &mut self.session {
            s.drain();
            let done = s.done.lock().expect("session done lock").take();
            if let Some(result) = done {
                let dir = s.dir.clone();
                let n = s.events.count();
                self.status = match result {
                    Ok(frames) => format!(
                        "session saved {} ({frames} frames · {n} events){}",
                        dir.display(),
                        Self::index_session(&dir)
                    ),
                    Err(e) => format!(
                        "session video: {e} · partial kept ({n} events){}",
                        Self::index_session(&dir)
                    ),
                };
                self.session = None;
            }
        }

        // The delay countdown (C1): fire when it elapses, tick otherwise.
        if let Some((deadline, pending)) = self.delay_armed {
            let now = Instant::now();
            if now >= deadline {
                self.delay_armed = None;
                self.fire(pending, ctx);
            } else {
                let left = (deadline - now).as_secs() + 1;
                self.status = format!("snapping in {left}s…");
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
            }
        }

        if self.region.is_some() {
            self.region_viewport(ctx);
        }

        let recording_steps = self.steps.status().recording;
        if !self.open && !recording_steps {
            return;
        }

        let size = if recording_steps {
            egui::vec2(236.0, 56.0)
        } else {
            egui::vec2(380.0, 240.0)
        };
        let builder = ViewportBuilder::default()
            .with_title("PixVix")
            .with_inner_size(size)
            .with_always_on_top()
            .with_decorations(false)
            .with_resizable(false);

        // Immediate viewport: state lives here, the closure just paints.
        let mut actions = PaletteActions::default();
        let tab = self.tab;
        let status = self.status.clone();
        let steps_state = self.steps.status();
        let video_elapsed = self.video.as_ref().map(|v| v.started.elapsed().as_secs());
        let session_state = self
            .session
            .as_ref()
            .map(|s| (s.started.elapsed().as_secs(), s.events.count()));
        let windows: Vec<String> = self
            .windows
            .iter()
            .map(|w| format!("{} — {}", w.app, w.title))
            .collect();
        let picked = self.picked_window;
        let delay = self.delay;
        let note = self
            .steps_note
            .lock()
            .map(|n| n.clone())
            .unwrap_or_default();

        let mut close_requested = false;
        ctx.pilot_show_viewport_immediate(
            PilotViewportKey::new(PALETTE_KEY),
            ViewportId::from_hash_of("pixvix"),
            builder,
            |ctx, _| {
                close_requested = ctx.input(|input| input.viewport().close_requested());
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(cl::surface::RAISED)
                            .stroke(egui::Stroke::new(1.0, cl::surface::HAIRLINE))
                            .inner_margin(egui::Margin::same(10)),
                    )
                    .show(ctx, |ui| {
                        if steps_state.recording {
                            rec_pill(ui, steps_state.count, &mut actions);
                        } else {
                            palette(
                                ui,
                                &PaletteView {
                                    tab,
                                    status: &status,
                                    windows: &windows,
                                    picked,
                                    video_elapsed,
                                    session_state,
                                    steps_note: &note,
                                    delay,
                                },
                                &mut actions,
                            );
                        }
                    });
            },
        );
        actions.close |= close_requested;

        self.apply(actions, ctx);
    }

    fn apply(&mut self, actions: PaletteActions, ctx: &egui::Context) {
        if let Some(tab) = actions.tab {
            self.tab = tab;
            if tab == Tab::Img && self.windows.is_empty() {
                self.windows = cap::list_targets().1;
            }
        }
        if let Some(idx) = actions.pick_window {
            self.picked_window = idx;
        }
        if actions.close {
            self.open = false;
        }
        if actions.cycle_delay {
            self.delay = match self.delay {
                0 => 3,
                3 => 10,
                _ => 0,
            };
            self.status = if self.delay == 0 {
                "delay off".into()
            } else {
                format!("delay {}s", self.delay)
            };
        }
        if actions.snap_full {
            self.arm_or_fire(PendingSnip::Full, ctx);
        }
        if actions.snap_window {
            self.arm_or_fire(PendingSnip::Window, ctx);
        }
        if actions.begin_region {
            self.arm_or_fire(PendingSnip::OpenEditor(SnipMode::Rect), ctx);
        }
        if actions.begin_free {
            self.arm_or_fire(PendingSnip::OpenEditor(SnipMode::Free), ctx);
        }
        if actions.resnip {
            self.arm_or_fire(PendingSnip::Resnip, ctx);
        }
        if actions.refresh_windows {
            self.windows = cap::list_targets().1;
            self.status = format!("{} windows", self.windows.len());
        }
        if actions.video_start {
            self.start_video();
        }
        if actions.session_start {
            self.start_session();
        }
        if actions.session_stop {
            self.stop_session();
        }
        if actions.video_stop {
            self.stop_video();
        }
        if actions.steps_start {
            self.start_steps();
        }
        if actions.steps_mark {
            self.mark_step();
        }
        if actions.steps_stop {
            self.stop_steps();
        }
    }

    // ── the frozen-frame editor: region rect + freeform polygon (C1) ────
    fn region_viewport(&mut self, ctx: &egui::Context) {
        enum Done {
            Cancel,
            Rect(Rect, egui::Vec2),
            Free(Vec<egui::Pos2>, egui::Vec2),
        }
        let mut finish: Option<Done> = None;
        let region = self.region.as_mut().expect("region state");
        let builder = ViewportBuilder::default()
            .with_title("PixVix region")
            .with_fullscreen(true)
            .with_always_on_top()
            .with_decorations(false);

        let mut close_requested = false;
        ctx.pilot_show_viewport_immediate(
            PilotViewportKey::new(REGION_KEY),
            ViewportId::from_hash_of("pixvix-region"),
            builder,
            |ctx, _| {
                close_requested = ctx.input(|input| input.viewport().close_requested());
                egui::CentralPanel::default()
                    .frame(egui::Frame::new().fill(Color32::BLACK))
                    .show(ctx, |ui| {
                        let full = ui.max_rect();
                        let painter = ui.painter().clone();
                        let tex = region.tex.id();
                        painter.image(
                            tex,
                            full,
                            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );

                        let resp = ui.interact(
                            full,
                            egui::Id::new("pixvix-region-drag"),
                            Sense::click_and_drag(),
                        );
                        let state_word = match region.mode {
                            SnipMode::Rect if region.dragging.is_some() => "dragging".into(),
                            SnipMode::Rect if region.sel.is_some() => "armed".into(),
                            SnipMode::Rect => "drag to select".into(),
                            SnipMode::Free => format!("polygon · {} vertices", region.poly.len()),
                        };
                        let mut metadata = NodeMetadata::new(
                            REGION_DRAG,
                            PilotRole::Region,
                            "PixVix snip selection",
                        )
                        .value(state_word);
                        metadata.actions = vec![PilotActionKind::Drag];
                        ui.pilot_track(&resp, metadata);

                        match region.mode {
                            SnipMode::Rect => {
                                // Drag paints live; release ARMS (the old
                                // pro-snip ceremony) — a new drag replaces.
                                if resp.drag_started() {
                                    if let Some(p) = resp.interact_pointer_pos() {
                                        region.dragging = Some(p);
                                        region.sel = Some(Rect::from_two_pos(p, p));
                                    }
                                }
                                if let (Some(anchor), Some(p)) =
                                    (region.dragging, resp.interact_pointer_pos())
                                {
                                    if resp.dragged() {
                                        region.sel = Some(Rect::from_two_pos(anchor, p));
                                    }
                                }
                                if resp.drag_stopped() {
                                    region.dragging = None;
                                }
                                // The nudge: arrows move the armed rect 1px,
                                // Shift makes it 10.
                                if region.dragging.is_none() {
                                    if let Some(sel) = region.sel.as_mut() {
                                        let step = if ui.input(|i| i.modifiers.shift) {
                                            10.0
                                        } else {
                                            1.0
                                        };
                                        let mut d = egui::Vec2::ZERO;
                                        ui.input(|i| {
                                            if i.key_pressed(egui::Key::ArrowLeft) {
                                                d.x -= step;
                                            }
                                            if i.key_pressed(egui::Key::ArrowRight) {
                                                d.x += step;
                                            }
                                            if i.key_pressed(egui::Key::ArrowUp) {
                                                d.y -= step;
                                            }
                                            if i.key_pressed(egui::Key::ArrowDown) {
                                                d.y += step;
                                            }
                                        });
                                        if d != egui::Vec2::ZERO {
                                            *sel = sel.translate(d);
                                        }
                                    }
                                }
                                // Enter or double-click CAPTURES the armed rect.
                                let confirm = ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    || resp.double_clicked();
                                if confirm {
                                    if let Some(sel) = region.sel {
                                        finish = Some(Done::Rect(sel, full.size()));
                                    }
                                }
                            }
                            SnipMode::Free => {
                                // Every click adds a vertex; a click near the
                                // FIRST vertex (≥3) closes; Enter closes too;
                                // Backspace removes the last.
                                if resp.clicked() {
                                    if let Some(p) = resp.interact_pointer_pos() {
                                        let closes = region.poly.len() >= 3
                                            && (p - region.poly[0]).length() < 8.0;
                                        if closes {
                                            finish = Some(Done::Free(
                                                region.poly.clone(),
                                                full.size(),
                                            ));
                                        } else {
                                            region.poly.push(p);
                                        }
                                    }
                                }
                                if ui.input(|i| i.key_pressed(egui::Key::Backspace)) {
                                    region.poly.pop();
                                }
                                if ui.input(|i| i.key_pressed(egui::Key::Enter))
                                    && region.poly.len() >= 3
                                {
                                    finish =
                                        Some(Done::Free(region.poly.clone(), full.size()));
                                }
                            }
                        }

                        // The veil + guides, per mode.
                        let sel = match region.mode {
                            SnipMode::Rect => region.sel,
                            SnipMode::Free => None,
                        };
                        if region.mode == SnipMode::Free {
                            painter.rect_filled(full, 0, Color32::from_black_alpha(60));
                            for w in region.poly.windows(2) {
                                painter.line_segment(
                                    [w[0], w[1]],
                                    egui::Stroke::new(1.5, TONE.color()),
                                );
                            }
                            if let (Some(&first), Some(p)) =
                                (region.poly.first(), ui.ctx().pointer_latest_pos())
                            {
                                if let Some(&last) = region.poly.last() {
                                    painter.line_segment(
                                        [last, p],
                                        egui::Stroke::new(
                                            1.0,
                                            TONE.color().gamma_multiply(0.5),
                                        ),
                                    );
                                }
                                // The close target glows once it can close.
                                if region.poly.len() >= 3 {
                                    painter.circle_stroke(
                                        first,
                                        8.0,
                                        egui::Stroke::new(1.5, TONE.color()),
                                    );
                                }
                            }
                            for v in &region.poly {
                                painter.circle_filled(*v, 3.0, TONE.color());
                            }
                            painter.text(
                                egui::pos2(full.center().x, full.max.y - 28.0),
                                Align2::CENTER_CENTER,
                                "click to add vertices · click the first (or Enter) closes · Backspace undoes · Esc cancels",
                                FontId::proportional(14.0),
                                cl::ink::STRONG,
                            );
                        }
                        let veil = Color32::from_black_alpha(140);
                        match sel {
                            Some(r) => {
                                for shade in [
                                    Rect::from_min_max(full.min, egui::pos2(full.max.x, r.min.y)),
                                    Rect::from_min_max(egui::pos2(full.min.x, r.max.y), full.max),
                                    Rect::from_min_max(
                                        egui::pos2(full.min.x, r.min.y),
                                        egui::pos2(r.min.x, r.max.y),
                                    ),
                                    Rect::from_min_max(
                                        egui::pos2(r.max.x, r.min.y),
                                        egui::pos2(full.max.x, r.max.y),
                                    ),
                                ] {
                                    painter.rect_filled(shade, 0, veil);
                                }
                                painter.rect(
                                    r,
                                    0,
                                    Color32::TRANSPARENT,
                                    egui::Stroke::new(1.5, TONE.color()),
                                    egui::StrokeKind::Outside,
                                );
                                painter.text(
                                    r.min - egui::vec2(0.0, 6.0),
                                    Align2::LEFT_BOTTOM,
                                    format!("{:.0} × {:.0}", r.width(), r.height()),
                                    FontId::monospace(12.0),
                                    cl::ink::STRONG,
                                );
                                // The ceremony hint, once armed.
                                if region.dragging.is_none() {
                                    painter.text(
                                        egui::pos2(full.center().x, full.max.y - 28.0),
                                        Align2::CENTER_CENTER,
                                        "arrows nudge (Shift = 10px) · Enter or double-click captures · drag again to redo · Esc cancels",
                                        FontId::proportional(14.0),
                                        cl::ink::STRONG,
                                    );
                                }
                            }
                            None if region.mode == SnipMode::Rect => {
                                painter.rect_filled(full, 0, Color32::from_black_alpha(60));
                                painter.text(
                                    full.center(),
                                    Align2::CENTER_CENTER,
                                    "drag to select · Esc cancels",
                                    FontId::proportional(15.0),
                                    cl::ink::STRONG,
                                );
                            }
                            None => {}
                        }

                        // The loupe: 4× zoom around the cursor, ColdLight-framed.
                        if let Some(p) = ui.ctx().pointer_latest_pos() {
                            let zoom = 4.0;
                            let box_size = 120.0;
                            let uv_half = box_size / zoom / 2.0;
                            let uv = Rect::from_min_max(
                                egui::pos2(
                                    ((p.x - uv_half) / full.width()).clamp(0.0, 1.0),
                                    ((p.y - uv_half) / full.height()).clamp(0.0, 1.0),
                                ),
                                egui::pos2(
                                    ((p.x + uv_half) / full.width()).clamp(0.0, 1.0),
                                    ((p.y + uv_half) / full.height()).clamp(0.0, 1.0),
                                ),
                            );
                            let offset = egui::vec2(24.0, 24.0);
                            let mut anchor = p + offset;
                            if anchor.x + box_size > full.max.x {
                                anchor.x = p.x - offset.x - box_size;
                            }
                            if anchor.y + box_size > full.max.y {
                                anchor.y = p.y - offset.y - box_size;
                            }
                            let loupe = Rect::from_min_size(anchor, egui::vec2(box_size, box_size));
                            painter.image(tex, loupe, uv, Color32::WHITE);
                            painter.rect(
                                loupe,
                                CornerRadius::same(cl::RADIUS_SM),
                                Color32::TRANSPARENT,
                                egui::Stroke::new(1.5, TONE.color()),
                                egui::StrokeKind::Outside,
                            );
                            // Crosshair at the loupe center = the cursor pixel.
                            let c = loupe.center();
                            painter.line_segment(
                                [c - egui::vec2(8.0, 0.0), c + egui::vec2(8.0, 0.0)],
                                egui::Stroke::new(1.0, TONE.color()),
                            );
                            painter.line_segment(
                                [c - egui::vec2(0.0, 8.0), c + egui::vec2(0.0, 8.0)],
                                egui::Stroke::new(1.0, TONE.color()),
                            );
                        }

                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            finish = Some(Done::Cancel);
                        }
                    });
            },
        );
        if close_requested {
            finish = Some(Done::Cancel);
        }

        if let Some(done) = finish {
            let state = self.region.take().expect("region state");
            match done {
                Done::Rect(r, screen) => {
                    if r.width() >= 4.0 && r.height() >= 4.0 {
                        let frac = cap::FracRect {
                            x: (r.min.x / screen.x) as f64,
                            y: (r.min.y / screen.y) as f64,
                            w: (r.width() / screen.x) as f64,
                            h: (r.height() / screen.y) as f64,
                        };
                        let result = Self::inbox().and_then(|inbox| {
                            let out = Self::unique_path(&inbox, "pix", "png");
                            cap::still_region_from_png(&state.frozen, &out, frac)?;
                            Ok(out)
                        });
                        self.status = match result {
                            Ok(p) => {
                                self.last = Some(LastSnip::Region(frac));
                                format!(
                                    "saved {}{}{}",
                                    p.display(),
                                    Self::index_capture(&p, "image"),
                                    Self::copy_to_clipboard(ctx, &p)
                                )
                            }
                            Err(e) => e,
                        };
                    } else {
                        self.status = "region too small".into();
                    }
                }
                Done::Free(poly, screen) => {
                    let points: Vec<[f64; 2]> = poly
                        .iter()
                        .map(|p| [(p.x / screen.x) as f64, (p.y / screen.y) as f64])
                        .collect();
                    let result = Self::inbox().and_then(|inbox| {
                        let out = Self::unique_path(&inbox, "pix", "png");
                        cap::still_freeform_from_png(&state.frozen, &out, &points)?;
                        Ok(out)
                    });
                    self.status = match result {
                        Ok(p) => {
                            self.last = Some(LastSnip::Free(points));
                            format!(
                                "saved {}{}{}",
                                p.display(),
                                Self::index_capture(&p, "image"),
                                Self::copy_to_clipboard(ctx, &p)
                            )
                        }
                        Err(e) => e,
                    };
                }
                Done::Cancel => self.status = "snip cancelled".into(),
            }
            let _ = std::fs::remove_file(&state.frozen); // temp freeze only
        }
    }
}

/// What the palette closure asks the owner to do (immediate viewports paint;
/// the owner mutates).
#[derive(Default)]
struct PaletteActions {
    tab: Option<Tab>,
    close: bool,
    snap_full: bool,
    snap_window: bool,
    begin_region: bool,
    begin_free: bool,
    resnip: bool,
    cycle_delay: bool,
    refresh_windows: bool,
    pick_window: Option<usize>,
    video_start: bool,
    video_stop: bool,
    session_start: bool,
    session_stop: bool,
    steps_start: bool,
    steps_mark: bool,
    steps_stop: bool,
}

/// Everything the palette closure READS this frame — bundled so the
/// paint fn stays inside the argument budget without an `#[allow]`
/// (the workspace law; this retires the one grandfathered exception).
struct PaletteView<'a> {
    tab: Tab,
    status: &'a str,
    windows: &'a [String],
    picked: usize,
    video_elapsed: Option<u64>,
    session_state: Option<(u64, usize)>,
    steps_note: &'a str,
    delay: u8,
}

fn palette(ui: &mut egui::Ui, view: &PaletteView<'_>, actions: &mut PaletteActions) {
    let PaletteView {
        tab,
        status,
        windows,
        picked,
        video_elapsed,
        session_state,
        steps_note,
        delay,
    } = *view;
    // Title row: identity + close.
    ui.horizontal(|ui| {
        ui.label(RichText::new(cl::icon::SNIP).color(TONE.color()).size(14.0));
        let title = ui.label(RichText::new("PixVix").strong().color(cl::ink::STRONG));
        ui.pilot_track(
            &title,
            NodeMetadata::new(PALETTE_TITLE, PilotRole::Heading, "PixVix"),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if pilot_chip(ui, PALETTE_CLOSE, cl::icon::CLOSE, "", TONE) {
                actions.close = true;
            }
        });
    });

    // The tab strip.
    ui.horizontal(|ui| {
        for t in Tab::ALL {
            let active = t == tab;
            let text = format!("{} {}", t.icon(), t.label());
            let rich = if active {
                RichText::new(text).color(TONE.color())
            } else {
                RichText::new(text).color(cl::ink::MUTED)
            };
            let response = ui.selectable_label(active, rich);
            let mut metadata =
                NodeMetadata::new(t.pilot_id(), PilotRole::Tab, t.label().to_string());
            metadata.selected = Some(active);
            ui.pilot_track(&response, metadata);
            if response.clicked() {
                actions.tab = Some(t);
            }
        }
    });
    ui.separator();

    match tab {
        Tab::Img => {
            ui.horizontal(|ui| {
                if pilot_chip(ui, REGION_OPEN, cl::icon::REGION, "Region", TONE) {
                    actions.begin_region = true;
                }
                if pilot_chip(ui, SNAP_FULL, cl::icon::FULL, "Full", TONE) {
                    actions.snap_full = true;
                }
                if pilot_chip(ui, FREEFORM, cl::icon::FREE, "Free", TONE) {
                    actions.begin_free = true;
                }
                // The delay chip cycles 0 → 3 → 10 seconds (C1).
                let delay_label = if delay == 0 {
                    "0s".to_string()
                } else {
                    format!("{delay}s")
                };
                if pilot_chip(ui, DELAY_CYCLE, "", &delay_label, TONE) {
                    actions.cycle_delay = true;
                }
                if pilot_chip(ui, RESNIP, "", "Re-snip", TONE) {
                    actions.resnip = true;
                }
            });
            ui.add_space(cl::space::S2);
            ui.horizontal(|ui| {
                ui.label(RichText::new(cl::icon::WINDOW).color(cl::ink::MUTED));
                let current = windows
                    .get(picked)
                    .cloned()
                    .unwrap_or_else(|| "no windows scanned".into());
                egui::ComboBox::from_id_salt("pixvix-window")
                    .width(220.0)
                    .selected_text(RichText::new(truncate(&current, 30)).small())
                    .show_ui(ui, |ui| {
                        for (i, w) in windows.iter().enumerate() {
                            if ui.selectable_label(i == picked, truncate(w, 44)).clicked() {
                                actions.pick_window = Some(i);
                            }
                        }
                    });
                if pilot_chip(ui, SNAP_WINDOW, cl::icon::SNIP, "Snap", TONE) {
                    actions.snap_window = true;
                }
                if ui.pilot_button(REFRESH_WINDOWS, "↻").clicked() {
                    actions.refresh_windows = true;
                }
            });
            ui.label(
                RichText::new(
                    "Region arms · arrows nudge · Enter captures. Free: click a polygon closed.",
                )
                .small()
                .color(cl::ink::SUBTLE),
            );
        }
        Tab::Video => match video_elapsed {
            Some(secs) => {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(cl::icon::REC)
                            .color(Tone::Coral.color())
                            .size(15.0),
                    );
                    ui.label(
                        RichText::new(format!("REC {:02}:{:02}", secs / 60, secs % 60))
                            .monospace()
                            .color(Tone::Coral.color()),
                    );
                    if pilot_chip(ui, VIDEO_STOP, cl::icon::STOP, "Stop", TONE) {
                        actions.video_stop = true;
                    }
                });
            }
            None => {
                ui.horizontal(|ui| {
                    if pilot_chip(ui, VIDEO_START, cl::icon::REC, "Record screen", TONE) {
                        actions.video_start = true;
                    }
                });
                ui.label(
                    RichText::new("Primary monitor · 1080p x264 · silent (audio in PV3).")
                        .small()
                        .color(cl::ink::SUBTLE),
                );
            }
        },
        Tab::Session => match session_state {
            Some((secs, events)) => {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(cl::icon::REC)
                            .color(Tone::Coral.color())
                            .size(15.0),
                    );
                    ui.label(
                        RichText::new(format!("REC {:02}:{:02}", secs / 60, secs % 60))
                            .monospace()
                            .color(Tone::Coral.color()),
                    );
                    ui.label(
                        RichText::new(format!("{events} events"))
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                    if pilot_chip(ui, SESSION_STOP, cl::icon::STOP, "Stop", TONE) {
                        actions.session_stop = true;
                    }
                });
            }
            None => {
                ui.horizontal(|ui| {
                    if pilot_chip(ui, SESSION_START, cl::icon::REC, "Record session", TONE) {
                        actions.session_start = true;
                    }
                });
                ui.label(
                    RichText::new(
                        "Screen video + a typed event track (routes, beats) — one session artifact.",
                    )
                    .small()
                    .color(cl::ink::SUBTLE),
                );
            }
        },
        Tab::Steps => {
            ui.horizontal(|ui| {
                if pilot_chip(ui, STEPS_START, cl::icon::REC, "Start guide", TONE) {
                    actions.steps_start = true;
                }
            });
            ui.label(
                RichText::new("Every click/Enter auto-snaps a step; Ctrl+Shift+9 toggles, Ctrl+Shift+X marks (app focused).")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            if !steps_note.is_empty() {
                ui.label(
                    RichText::new(steps_note)
                        .small()
                        .monospace()
                        .color(cl::ink::MUTED),
                );
            }
        }
    }

    ui.add_space(cl::space::S2);
    if !status.is_empty() {
        ui.label(
            RichText::new(status)
                .small()
                .monospace()
                .color(cl::ink::BASE),
        );
    }
}

/// The REC pill — the palette collapsed while Steps records.
fn rec_pill(ui: &mut egui::Ui, count: u32, actions: &mut PaletteActions) {
    ui.horizontal_centered(|ui| {
        // The pulse: alpha breathes on the motion runtime's loop.
        let t = ui.input(|i| i.time);
        let p = cl::motion::HOTSPOT_PULSE.at(t, 0.0, 0);
        let alpha = 140 + (110.0 * (1.0 - p)) as u8;
        let c = Tone::Coral.color();
        ui.label(
            RichText::new(cl::icon::REC)
                .size(16.0)
                .color(Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)),
        );
        ui.label(
            RichText::new(format!("REC · {count}"))
                .monospace()
                .color(Tone::Coral.color()),
        );
        if pilot_chip(ui, STEPS_MARK, cl::icon::MARK, "Mark", TONE) {
            actions.steps_mark = true;
        }
        if pilot_chip(ui, STEPS_STOP, cl::icon::STOP, "Stop", TONE) {
            actions.steps_stop = true;
        }
        ui.ctx().request_repaint(); // the pulse never rests while recording
    });
}

fn pilot_chip(ui: &mut egui::Ui, id: PilotId, glyph: &str, label: &str, tone: Tone) -> bool {
    let text = if label.is_empty() {
        glyph.to_string()
    } else {
        format!("{glyph} {label}")
    };
    let response = ui.add(
        egui::Button::new(RichText::new(text).color(cl::ink::BASE))
            .fill(cl::surface::OVERLAY)
            .stroke(egui::Stroke::new(1.0, tone.wash()))
            .corner_radius(CornerRadius::same(cl::RADIUS_SM)),
    );
    ui.pilot_track(
        &response,
        NodeMetadata::new(
            id,
            PilotRole::Button,
            if label.is_empty() { glyph } else { label },
        ),
    );
    response.clicked()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_path_never_clobbers() {
        let dir = std::env::temp_dir().join(format!("dpl-pixvix-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let first = Pixvix::unique_path(&dir, "pix", "png");
        assert!(first.ends_with("pix.png"));
        std::fs::write(&first, b"x").unwrap();
        let second = Pixvix::unique_path(&dir, "pix", "png");
        assert!(second.ends_with("pix-2.png"), "{second:?}");
        std::fs::write(&second, b"x").unwrap();
        assert!(Pixvix::unique_path(&dir, "pix", "png").ends_with("pix-3.png"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn truncate_is_char_safe() {
        assert_eq!(truncate("short", 30), "short");
        assert_eq!(truncate("ABCDEFGHIJ", 5), "ABCD…");
    }
}
