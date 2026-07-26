//! ViziWiz — the capture overlay (PV2), rebuilt in egui on the ported
//! engines. The palette is a compact ALWAYS-ON-TOP viewport with the old
//! four tabs (Img · Video · Session · Steps); region select is a borderless
//! fullscreen viewport over the FROZEN frame (`freeze_primary_to`) with the
//! magnifier loupe — the old app's own pattern, no window transparency
//! needed. While Steps records, the palette collapses to the REC pill.
//!
//! Every action calls an engine function and reports honestly in the
//! status line. Filing: `content_root()/inbox`, never-clobber names.
//!
//! Modules (the C2 alignment split): `palette` paints the palette + Save
//! card, `editor` is the frozen-frame region/polygon editor, `session`
//! the S1 session vehicle. State and every engine call live HERE.

use dpl_coldlight::Tone;
use dpl_engine_capture as cap;
use dpl_engine_steps::{normalize_meta_auto, CaptureMeta, Hotkey, StepsRecorder};
use dpl_fabric::{Fabric, ShellEvent};
use egui::{ViewportBuilder, ViewportId};
use egui_pilot_ai::protocol::PilotViewportKey;
use egui_pilot_ai::ContextPilotExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub(crate) const TONE: Tone = Tone::Magenta; // wiredFRONT's tone — the capture family.
const PALETTE_KEY: &str = "devpulse.viziwiz.palette";

mod editor;
mod palette;
pub mod session;

use editor::{RegionSelect, SnipMode};
use palette::{PaletteActions, PaletteView, Tab};

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

/// The C3 video matrix — engine ids + chip labels. AV1 is offered but
/// mp4 stays the default (CPU-heavy encode, the engine's own note).
pub(crate) const FORMATS: [(&str, &str); 3] =
    [("mp4", "MP4"), ("webm-vp9", "VP9"), ("webm-av1", "AV1")];
/// Audio modes — "system" needs a loopback device (Stereo Mix/virtual
/// cable on Windows); the engine errors honestly without one.
pub(crate) const AUDIO: [(&str, &str); 3] =
    [("none", "Silent"), ("system", "System"), ("mic", "Mic")];
/// Max-cap chip stops the recording automatically (touch law: a cycling
/// chip, not the old free-text seconds field — recorded deviation).
pub(crate) const CAPS: [(Option<u32>, &str); 4] = [
    (None, "no cap"),
    (Some(60), "1m"),
    (Some(300), "5m"),
    (Some(900), "15m"),
];

/// A stopped-but-unfiled guide waiting on the Save card (C2). The
/// engine keeps the session PENDING until `save` or `discard` decides.
pub(crate) struct PendingGuide {
    pub(crate) count: u32,
    pub(crate) name: String,
    /// Space/comma-separated entry; split on save.
    pub(crate) tags: String,
    /// General library ⇄ episode destination (the forced-metadata law).
    pub(crate) library: bool,
    pub(crate) show: String,
    pub(crate) episode: String,
}

pub struct Viziwiz {
    open: bool,
    tab: Tab,
    status: String,
    rx: tokio::sync::broadcast::Receiver<ShellEvent>,
    /// Kept whole so a session can take a FRESH subscription at start —
    /// a broadcast receiver only sees traffic after it subscribes.
    fabric: Fabric,
    steps: Arc<StepsRecorder>,
    steps_note: Arc<Mutex<String>>,
    /// OS-global chords, queued on the hook thread, drained each frame.
    hotkeys: Arc<Mutex<Vec<Hotkey>>>,
    hook_wired: bool,
    /// The Save card's state — Some between Stop and Save/Discard.
    pending: Option<PendingGuide>,
    region: Option<RegionSelect>,
    video: Option<VideoRec>,
    /// The running session recording (S1), if any.
    session: Option<session::SessionRec>,
    windows: Vec<cap::WindowInfo>,
    picked_window: usize,
    /// C3 video matrix state — indices into FORMATS / AUDIO / CAPS.
    video_format: usize,
    video_audio: usize,
    video_cap: usize,
    /// false = primary screen, true = the picked window.
    video_window: bool,
    /// ffmpeg-enumerated audio devices; filled when audio leaves Silent.
    audio_devices: Vec<String>,
    picked_audio: usize,
    /// The capture delay in seconds — one chip cycles 0 → 3 → 5 → 10 (C1).
    delay: u8,
    /// A snap waiting out the delay; the status line counts it down.
    delay_armed: Option<(Instant, PendingSnip)>,
    /// The last successful still's shape — Re-snip's memory.
    last: Option<LastSnip>,
}

impl Viziwiz {
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
            hotkeys: Arc::new(Mutex::new(Vec::new())),
            hook_wired: false,
            pending: None,
            region: None,
            video: None,
            session: None,
            windows: Vec::new(),
            picked_window: 0,
            video_format: 0,
            video_audio: 0,
            video_cap: 0,
            video_window: false,
            audio_devices: Vec::new(),
            picked_audio: 0,
            delay: 0,
            delay_armed: None,
            last: None,
        }
    }

    /// THE one write-through seam (signal-doctrine law 3): sidecar +
    /// index upsert + the `Saved` signal, identically for every vehicle.
    /// Capture NEVER fails because indexing did — a problem comes back
    /// as a status-line suffix (and a `Failed` signal); `Saved` fires
    /// ONLY after the write succeeded (signals report truth).
    fn index_and_signal(
        fabric: &dpl_fabric::Fabric,
        target: &Path,
        record: dpl_index::Record,
    ) -> String {
        let result = dpl_index::sidecar::write(target, &record)
            .and_then(|()| dpl_index::Index::open_default()?.upsert(&record));
        match result {
            Ok(()) => {
                fabric.publish_capture(dpl_fabric::CaptureEvent::Saved {
                    rel_path: record.rel_path,
                    kind: record.kind,
                    name: record.name,
                    location: record.location,
                    source: record.source,
                });
                String::new()
            }
            Err(e) => {
                fabric.publish_capture(dpl_fabric::CaptureEvent::Failed { error: e.clone() });
                format!(" · index: {e}")
            }
        }
    }

    /// X1 write-through for a fresh capture — builds the row, then the
    /// one seam above indexes + signals.
    fn index_capture(fabric: &dpl_fabric::Fabric, path: &Path, kind: &str) -> String {
        let root = cap::content_root();
        let record = dpl_index::record_for_capture(path, &root, kind, "viziwiz");
        Self::index_and_signal(fabric, path, record)
    }

    /// The Save card's write-through (C2): the guide row carries the
    /// card's tags and — for an episode destination — the association,
    /// like the old `index_saved`. Same degrade law as `index_capture`.
    fn index_guide(fabric: &dpl_fabric::Fabric, path: &Path, meta: &CaptureMeta) -> String {
        let root = cap::content_root();
        let mut record = dpl_index::record_for_capture(path, &root, "guide", "viziwiz");
        record.tags = meta.tags.clone();
        if !meta.library {
            record.show_id = Some(meta.show.clone());
            record.episode_folder = Some(meta.episode.clone());
        }
        Self::index_and_signal(fabric, path, record)
    }

    /// X1 write-through for a finished VIDEO — the row carries the
    /// convention poster when it exists (C3; same degrade law).
    fn index_video(fabric: &dpl_fabric::Fabric, out: &Path) -> String {
        let root = cap::content_root();
        let mut record = dpl_index::record_for_capture(out, &root, "video", "viziwiz");
        let poster = Self::poster_path(out);
        if poster.is_file() {
            if let Some(name) = poster.file_name() {
                let name = name.to_string_lossy();
                record.poster_rel_path = Some(match record.rel_path.rsplit_once('/') {
                    Some((parent, _)) => format!("{parent}/{name}"),
                    None => name.to_string(),
                });
            }
        }
        Self::index_and_signal(fabric, out, record)
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

    /// The repo's poster convention: `x.mp4` → `x-poster.jpg`.
    fn poster_path(video: &Path) -> PathBuf {
        let stem = video
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "vix".into());
        video.with_file_name(format!("{stem}-poster.jpg"))
    }

    /// Video stems must be unique across CONTAINERS too — `vix.mp4` and
    /// `vix.webm` would otherwise share one `vix-poster.jpg` and the
    /// later recording's poster clobbers the earlier's (caught live by
    /// the C3 matrix drill).
    fn unique_video_path(dir: &Path, ext: &str) -> PathBuf {
        let free = |stem: &str| {
            let vid = dir.join(format!("{stem}.{ext}"));
            !vid.exists() && !Self::poster_path(&vid).exists()
        };
        if free("vix") {
            return dir.join(format!("vix.{ext}"));
        }
        (2..)
            .map(|n| format!("vix-{n}"))
            .find(|s| free(s))
            .map(|s| dir.join(format!("{s}.{ext}")))
            .expect("unbounded range")
    }

    /// THE one palette-visibility write (W8): every open/close funnels
    /// here so `PaletteVisibility` fires exactly once per real change.
    fn set_open(&mut self, open: bool) {
        if self.open != open {
            self.open = open;
            self.fabric
                .publish_capture(dpl_fabric::CaptureEvent::PaletteVisibility { visible: open });
        }
    }

    pub fn toggle(&mut self) {
        self.set_open(!self.open);
    }

    /// Harness: open the palette at boot (`--viziwiz`).
    pub fn open_now(&mut self) {
        self.set_open(true);
    }

    /// Harness: fire the Img/Full action through the real handler and
    /// report the status line (`--viziwiz-snap` — Law 2 without a mouse).
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

    /// True while the Steps recorder runs (the hotkey drill's verdict).
    pub fn steps_recording(&self) -> bool {
        self.steps.status().recording
    }

    /// The honest status line — the drill's verdict source.
    pub fn status_text(&self) -> &str {
        &self.status
    }

    /// The Steps toggle (Ctrl+Shift+9, OS-global since C2): start when
    /// idle, stop into the Save card when recording.
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

    /// Blank meta for the recording-folder auto path — the operator's
    /// real name/tags/destination arrive via the Save card on stop (C2).
    /// General library here: the auto path must never block on a
    /// destination (the first hotkey drill caught `library: false`
    /// refusing every start).
    fn blank_meta() -> CaptureMeta {
        CaptureMeta {
            library: true,
            show: String::new(),
            episode: String::new(),
            name: String::new(),
            tags: Vec::new(),
        }
    }

    /// The `--steps-drill` seam (W5.3): start the Steps recorder through
    /// the REAL handler (the palette button / Ctrl+Shift+9 path), with
    /// self-capture allowed so the drill can demonstrate this very app.
    pub fn steps_start_drill_now(&mut self) -> &str {
        self.steps.set_allow_self(true);
        self.start_steps();
        self.status_text()
    }

    /// Drill: mark THIS moment through the real Mark path (always kept).
    pub fn steps_mark_now(&mut self) {
        self.mark_step();
    }

    /// Drill: captured step count, straight from the recorder.
    pub fn steps_count(&self) -> u32 {
        self.steps.status().count
    }

    /// Drill: stop through the real handler — the guide goes PENDING on
    /// the Save card exactly like the operator's flow.
    pub fn steps_stop_now(&mut self) -> &str {
        self.stop_steps();
        self.status_text()
    }

    /// Drill: file the pending guide through the REAL Save-card path
    /// (seeded name + write-through index), like any capture. The strict
    /// meta law still holds — the drill only fills what the operator
    /// would have typed (a fallback name, a "drill" tag).
    pub fn steps_save_now(&mut self) -> &str {
        if let Some(p) = self.pending.as_mut() {
            if p.name.trim().is_empty() {
                p.name = "drill-guide".into();
            }
            if p.tags.trim().is_empty() {
                p.tags = "drill".into();
            }
        }
        self.save_pending();
        self.status_text()
    }

    fn start_steps(&mut self) {
        if self.pending.is_some() {
            // Starting would drop the unfiled session — the card decides.
            self.status = "a guide is waiting on the Save card — Save or Discard first".into();
            self.tab = Tab::Steps;
            self.set_open(true);
            return;
        }
        match Self::inbox().and_then(|inbox| {
            let meta = normalize_meta_auto(Self::blank_meta(), "guide")?;
            self.steps.start(&inbox, meta)
        }) {
            Ok(()) => {
                self.status = "steps: recording".into();
                self.fabric
                    .publish_capture(dpl_fabric::CaptureEvent::Started {
                        mode: "steps".into(),
                        target: String::new(),
                    });
            }
            Err(e) => self.status = format!("steps: {e}"),
        }
    }

    /// Stop keeps the session PENDING and opens the Save card seeded
    /// from the first captured window's title (C2 — the old flow).
    fn stop_steps(&mut self) {
        match self.steps.stop() {
            Ok(stop) => {
                self.pending = Some(PendingGuide {
                    count: stop.count,
                    name: seed_name(&stop.seed_title),
                    tags: String::new(),
                    library: true,
                    show: String::new(),
                    episode: String::new(),
                });
                self.tab = Tab::Steps;
                self.set_open(true);
                self.status = format!("{} steps pending — name your guide to file it", stop.count);
            }
            Err(e) => self.status = format!("steps: {e}"),
        }
    }

    /// The Save card's Save: STRICT metadata (kebab name + auto `_v1`,
    /// ≥1 tag, episode/General) — a refusal keeps the pending guide and
    /// lands in the status line. Filing happens exactly once, in the
    /// engine.
    fn save_pending(&mut self) {
        let Some(p) = self.pending.as_ref() else {
            return;
        };
        let tags: Vec<String> = p
            .tags
            .split([' ', ','])
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect();
        let meta = CaptureMeta {
            library: p.library,
            show: p.show.trim().to_string(),
            episode: p.episode.trim().to_string(),
            name: p.name.trim().to_string(),
            tags,
        };
        let saved = Self::inbox().and_then(|inbox| self.steps.save(&inbox, meta.clone()));
        match saved {
            Ok(folder) => {
                let count = self.pending.take().map(|p| p.count).unwrap_or(0);
                let warn = Self::inbox()
                    .map(|inbox| Self::index_guide(&self.fabric, &inbox.join(&folder), &meta))
                    .unwrap_or_default();
                self.status = format!("guide: {count} steps → {folder}{warn}");
            }
            Err(e) => self.status = format!("guide: {e}"),
        }
    }

    fn discard_pending(&mut self) {
        self.steps.discard();
        self.pending = None;
        self.status = "guide discarded".into();
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
                    Self::index_capture(&self.fabric, &p, "image"),
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
                    Self::index_capture(&self.fabric, &p, "image"),
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
                        Self::index_capture(&self.fabric, &p, "image"),
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
                        Self::index_capture(&self.fabric, &p, "image"),
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

    /// Start▸Stop recording with the C3 matrix: screen or the picked
    /// window, 720p (the old default; the engine downscales), format +
    /// audio + cap from the chips. The engine owns every refusal.
    fn start_video(&mut self) {
        let (format_id, format_label) = FORMATS[self.video_format];
        let (audio_id, audio_label) = AUDIO[self.video_audio];
        let (cap_secs, _) = CAPS[self.video_cap];
        let target = if self.video_window {
            match self.windows.get(self.picked_window) {
                Some(w) => cap::RecTarget::Window(w.id),
                None => {
                    self.status = "pick a window first (Img tab scans them)".into();
                    return;
                }
            }
        } else {
            match cap::list_targets().0.into_iter().find(|m| m.is_primary) {
                Some(m) => cap::RecTarget::Monitor(m.id),
                None => {
                    self.status = "no primary monitor".into();
                    return;
                }
            }
        };
        // A mode that records audio needs a device: the picked one,
        // else the first enumerated — the engine refuses honestly when
        // "system" has none at all.
        let device = if audio_id == "none" {
            None
        } else {
            self.audio_devices
                .get(self.picked_audio)
                .or_else(|| self.audio_devices.first())
                .cloned()
        };
        let result = Self::inbox().and_then(|inbox| {
            let out = Self::unique_video_path(&inbox, cap::ext_for_format(format_id));
            let stop = Arc::new(AtomicBool::new(false));
            let done = Arc::new(Mutex::new(None));
            let (t_stop, t_done, t_out) = (Arc::clone(&stop), Arc::clone(&done), out.clone());
            let opts = cap::VideoOpts {
                height: 720,
                format: format_id.into(),
                audio: audio_id.into(),
                audio_device: device,
                max_seconds: cap_secs,
            };
            std::thread::Builder::new()
                .name("viziwiz-video".into())
                .spawn(move || {
                    let r = cap::record_ctl_to(&t_out.to_string_lossy(), target, &opts, &t_stop);
                    if r.is_ok() {
                        let poster = Self::poster_path(&t_out);
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
                self.fabric
                    .publish_capture(dpl_fabric::CaptureEvent::Started {
                        mode: "video".into(),
                        target: v.out.display().to_string(),
                    });
                self.status = format!(
                    "recording {} · {format_label} · {audio_label}",
                    v.out.display()
                );
                self.video = Some(v);
            }
            Err(e) => self.status = e,
        }
    }

    /// Harness: start a recording through the REAL handler, optionally
    /// pinning the format (`--video-drill`).
    pub fn video_start_now(&mut self, format: Option<&str>) -> &str {
        if let Some(f) = format {
            match FORMATS.iter().position(|(id, _)| *id == f) {
                Some(i) => self.video_format = i,
                None => {
                    self.status = format!("unknown video format {f:?}");
                    return &self.status;
                }
            }
        }
        self.start_video();
        &self.status
    }

    /// True while a recording runs or its thread is still closing.
    pub fn video_active(&self) -> bool {
        self.video.is_some()
    }

    /// Harness: request the running recording's stop.
    pub fn video_stop_now(&mut self) {
        self.stop_video();
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
        let rx_capture = self.fabric.subscribe_capture();
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
                rx_capture,
                events,
            })
        });
        match result {
            Ok(s) => {
                self.fabric
                    .publish_capture(dpl_fabric::CaptureEvent::Started {
                        mode: "session".into(),
                        target: s.dir.display().to_string(),
                    });
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
    fn index_session(fabric: &dpl_fabric::Fabric, dir: &Path) -> String {
        let root = cap::content_root();
        let mut record = dpl_index::record_for_capture(dir, &root, "session", "viziwiz");
        if dir.join("session.jpg").is_file() {
            record.poster_rel_path = Some(format!("{}/session.jpg", record.rel_path));
        }
        Self::index_and_signal(fabric, dir, record)
    }

    /// Install the OS-global hook ONCE (first frame): chords queue on the
    /// hook thread and wake the UI — never any work over there (C2).
    fn wire_hotkeys(&mut self, ctx: &egui::Context) {
        if self.hook_wired {
            return;
        }
        self.hook_wired = true;
        let queue = Arc::clone(&self.hotkeys);
        let repaint = ctx.clone();
        let sink: dpl_engine_steps::HotkeySink = Arc::new(move |hotkey| {
            if let Ok(mut q) = queue.lock() {
                q.push(hotkey);
            }
            repaint.request_repaint();
        });
        dpl_engine_steps::install_hook(&self.steps, Some(sink));
    }

    /// Drive the overlay for this frame: consume summons + chords, then
    /// render the palette viewport (or the REC pill) and the editor.
    pub fn ui(&mut self, ctx: &egui::Context) {
        self.wire_hotkeys(ctx);
        while let Ok(ev) = self.rx.try_recv() {
            if ev == ShellEvent::ViziwizSummon {
                self.set_open(true);
            }
        }
        let fired: Vec<Hotkey> = self
            .hotkeys
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default();
        for hotkey in fired {
            match hotkey {
                Hotkey::Palette => self.toggle(),
                Hotkey::ToggleSteps => self.toggle_steps(),
                // No error spam when idle — the old hotkey semantics.
                Hotkey::MarkStep => {
                    if self.steps.status().recording {
                        self.mark_step();
                    }
                }
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
                        Self::index_video(&self.fabric, &out)
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
                        Self::index_session(&self.fabric, &dir)
                    ),
                    Err(e) => format!(
                        "session video: {e} · partial kept ({n} events){}",
                        Self::index_session(&self.fabric, &dir)
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

        // Sizes carry the claymorphic plate's chrome: a thin WELL gutter (6)
        // for the plate's directional shadow to fall into + the clay inner
        // margin (10) on each side.
        let size = if recording_steps {
            egui::vec2(248.0, 76.0)
        } else if self.pending.is_some() {
            // The Save card needs the extra rows.
            egui::vec2(396.0, 330.0)
        } else {
            egui::vec2(396.0, 260.0)
        };
        let builder = ViewportBuilder::default()
            .with_title("ViziWiz")
            .with_inner_size(size)
            .with_always_on_top()
            .with_decorations(false)
            .with_resizable(false);

        // Immediate viewport: state lives here, the closure just paints.
        // The pending guide is LENT to the closure — its text fields edit
        // in place; Save/Discard come back as actions.
        let mut actions = PaletteActions::default();
        let tab = self.tab;
        let status = self.status.clone();
        let steps_state = self.steps.status();
        let mut pending = self.pending.take();
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
        let video_window = self.video_window;
        let format_label = FORMATS[self.video_format].1;
        let audio_label = AUDIO[self.video_audio].1;
        let audio_silent = AUDIO[self.video_audio].0 == "none";
        let cap_label = CAPS[self.video_cap].1;
        let audio_devices = self.audio_devices.clone();
        let picked_audio = self.picked_audio;
        let note = self
            .steps_note
            .lock()
            .map(|n| n.clone())
            .unwrap_or_default();

        let mut close_requested = false;
        ctx.pilot_show_viewport_immediate(
            PilotViewportKey::new(PALETTE_KEY),
            ViewportId::from_hash_of("viziwiz"),
            builder,
            |ctx, _| {
                close_requested = ctx.input(|input| input.viewport().close_requested());
                egui::CentralPanel::default()
                    // The recessed WELL behind the plate — a thin gutter the
                    // claymorphic plate's directional shadow falls into.
                    .frame(
                        egui::Frame::new()
                            .fill(dpl_coldlight::surface::WELL)
                            .inner_margin(egui::Margin::same(6)),
                    )
                    .show(ctx, |ui| {
                        // The palette rides a claymorphic passive plane (the
                        // trait system, not a bespoke frame) — a soft extruded
                        // plate the dense capture chrome sits IN.
                        dpl_coldlight::clay()
                            .margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                if steps_state.recording {
                                    palette::rec_pill(ui, steps_state.count, &mut actions);
                                } else {
                                    palette::palette(
                                        ui,
                                        &PaletteView {
                                            tab,
                                            status: &status,
                                            windows: &windows,
                                            picked,
                                            video_elapsed,
                                            video_window,
                                            format_label,
                                            audio_label,
                                            audio_silent,
                                            cap_label,
                                            audio_devices: &audio_devices,
                                            picked_audio,
                                            session_state,
                                            steps_note: &note,
                                            delay,
                                        },
                                        pending.as_mut(),
                                        &mut actions,
                                    );
                                }
                            });
                    });
            },
        );
        actions.close |= close_requested;
        self.pending = pending;

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
            self.set_open(false);
        }
        if actions.cycle_delay {
            self.delay = match self.delay {
                0 => 3,
                3 => 5,
                5 => 10,
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
        if actions.cycle_format {
            self.video_format = (self.video_format + 1) % FORMATS.len();
            self.status = format!("format {}", FORMATS[self.video_format].1);
        }
        if actions.cycle_audio {
            self.video_audio = (self.video_audio + 1) % AUDIO.len();
            self.status = format!("audio {}", AUDIO[self.video_audio].1);
            // Leaving Silent needs the device list; enumerate once.
            if AUDIO[self.video_audio].0 != "none" && self.audio_devices.is_empty() {
                self.audio_devices = cap::list_audio_devices();
                if self.audio_devices.is_empty() {
                    self.status = format!(
                        "audio {} · no devices found (system needs a loopback device)",
                        AUDIO[self.video_audio].1
                    );
                }
            }
        }
        if actions.refresh_audio {
            self.audio_devices = cap::list_audio_devices();
            self.status = format!("{} audio devices", self.audio_devices.len());
        }
        if let Some(idx) = actions.pick_audio {
            self.picked_audio = idx;
        }
        if actions.cycle_cap {
            self.video_cap = (self.video_cap + 1) % CAPS.len();
            self.status = format!("cap {}", CAPS[self.video_cap].1);
        }
        if actions.toggle_video_target {
            self.video_window = !self.video_window;
            if self.video_window && self.windows.is_empty() {
                self.windows = cap::list_targets().1;
            }
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
        if actions.guide_save {
            self.save_pending();
        }
        if actions.guide_discard {
            self.discard_pending();
        }
    }
}

/// Seed a Save-card name from the first captured window's title —
/// Scribe-style: "Downloads - File Explorer" suggests "downloads". The
/// trailing " - App" segment drops; the rest kebab-ifies. The operator
/// edits from there; the STRICT law applies on save.
fn seed_name(title: &str) -> String {
    let base = title
        .rsplit_once(" - ")
        .map(|(head, _)| head)
        .unwrap_or(title);
    let mut out = String::new();
    for c in base.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    // Window titles can be paths hundreds of chars long — an unclamped
    // seed mints folders that break Win32 cwd limits (seen live in the
    // W5.3 drill). 48 chars keeps names readable and filesystem-safe.
    out.truncate(48);
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_path_never_clobbers() {
        let dir = std::env::temp_dir().join(format!("dpl-viziwiz-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let first = Viziwiz::unique_path(&dir, "pix", "png");
        assert!(first.ends_with("pix.png"));
        std::fs::write(&first, b"x").unwrap();
        let second = Viziwiz::unique_path(&dir, "pix", "png");
        assert!(second.ends_with("pix-2.png"), "{second:?}");
        std::fs::write(&second, b"x").unwrap();
        assert!(Viziwiz::unique_path(&dir, "pix", "png").ends_with("pix-3.png"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn video_stems_never_share_a_poster() {
        let dir = std::env::temp_dir().join(format!("dpl-vix-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let first = Viziwiz::unique_video_path(&dir, "mp4");
        assert!(first.ends_with("vix.mp4"));
        std::fs::write(&first, b"x").unwrap();
        std::fs::write(Viziwiz::poster_path(&first), b"x").unwrap();
        let second = Viziwiz::unique_video_path(&dir, "webm");
        assert!(
            second.ends_with("vix-2.webm"),
            "a webm must not reuse the mp4's stem (poster collision): {second:?}"
        );
        assert!(Viziwiz::poster_path(&second).ends_with("vix-2-poster.jpg"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn truncate_is_char_safe() {
        assert_eq!(palette::truncate("short", 30), "short");
        assert_eq!(palette::truncate("ABCDEFGHIJ", 5), "ABCD…");
    }

    #[test]
    fn seed_name_strips_the_app_and_kebabs() {
        assert_eq!(seed_name("Downloads - File Explorer"), "downloads");
        assert_eq!(
            seed_name("Users & Auth — Setup - Google Chrome"),
            "users-auth-setup"
        );
        assert_eq!(seed_name("Notepad"), "notepad");
        assert_eq!(seed_name(""), "", "empty seeds stay empty — the law asks");
        let monster = format!("C:\\{} - Console", "very\\deep\\path\\".repeat(20));
        assert!(
            seed_name(&monster).len() <= 48,
            "path-length titles clamp to filesystem-safe names"
        );
    }
}
