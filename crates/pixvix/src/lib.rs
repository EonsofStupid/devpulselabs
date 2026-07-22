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
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const TONE: Tone = Tone::Magenta; // wiredFRONT's tone — the capture family.

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
}

/// The frozen-frame region flow's state.
struct RegionSelect {
    tex: egui::TextureHandle,
    frozen: PathBuf,
    drag: Option<(egui::Pos2, egui::Pos2)>,
}

/// A running video recording (record_ctl_to on its own thread).
struct VideoRec {
    stop: Arc<AtomicBool>,
    started: Instant,
    out: PathBuf,
    done: Arc<Mutex<Option<Result<u32, String>>>>,
}

pub struct Pixvix {
    open: bool,
    tab: Tab,
    status: String,
    rx: tokio::sync::broadcast::Receiver<ShellEvent>,
    steps: Arc<StepsRecorder>,
    steps_note: Arc<Mutex<String>>,
    region: Option<RegionSelect>,
    video: Option<VideoRec>,
    windows: Vec<cap::WindowInfo>,
    picked_window: usize,
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
            steps: StepsRecorder::new(sink),
            steps_note,
            region: None,
            video: None,
            windows: Vec::new(),
            picked_window: 0,
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
    pub fn snap_full_now(&mut self) -> &str {
        self.snap_full();
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
                    Ok(folder) => format!("steps: {} steps → {folder}", stop.count),
                    Err(e) => format!("steps: stop ok, save failed: {e}"),
                };
            }
            Err(e) => self.status = format!("steps: {e}"),
        }
    }

    fn snap_full(&mut self) {
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
            Ok(p) => format!("saved {}", p.display()),
            Err(e) => e,
        };
    }

    fn snap_window(&mut self) {
        let Some(win) = self.windows.get(self.picked_window) else {
            self.status = "pick a window first".into();
            return;
        };
        let (id, title) = (win.id, win.title.clone());
        let result = Self::inbox().and_then(|inbox| {
            let out = Self::unique_path(&inbox, "pix", "png");
            cap::still_window_to(&out, id)?;
            Ok(out)
        });
        self.status = match result {
            Ok(p) => format!("saved {} ({title})", p.display()),
            Err(e) => e,
        };
    }

    fn begin_region(&mut self, ctx: &egui::Context) {
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
                drag: None,
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
                self.status = match result {
                    Ok(frames) => format!("saved {} ({frames} frames)", v.out.display()),
                    Err(e) => format!("video: {e}"),
                };
                self.video = None;
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
        let windows: Vec<String> = self
            .windows
            .iter()
            .map(|w| format!("{} — {}", w.app, w.title))
            .collect();
        let picked = self.picked_window;
        let note = self
            .steps_note
            .lock()
            .map(|n| n.clone())
            .unwrap_or_default();

        ctx.show_viewport_immediate(ViewportId::from_hash_of("pixvix"), builder, |ctx, _| {
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
                            tab,
                            &status,
                            &windows,
                            picked,
                            video_elapsed,
                            &note,
                            &mut actions,
                        );
                    }
                });
        });

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
        if actions.snap_full {
            self.snap_full();
        }
        if actions.snap_window {
            self.snap_window();
        }
        if actions.begin_region {
            self.begin_region(ctx);
        }
        if actions.free_deferred {
            self.status = "freeform lasso lands with the polygon editor (PV2 slice)".into();
        }
        if actions.refresh_windows {
            self.windows = cap::list_targets().1;
            self.status = format!("{} windows", self.windows.len());
        }
        if actions.video_start {
            self.start_video();
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

    // ── the region-select viewport (frozen frame + loupe) ───────────────
    fn region_viewport(&mut self, ctx: &egui::Context) {
        let mut finish: Option<Option<(egui::Pos2, egui::Pos2, egui::Vec2)>> = None;
        let region = self.region.as_mut().expect("region state");
        let builder = ViewportBuilder::default()
            .with_title("PixVix region")
            .with_fullscreen(true)
            .with_always_on_top()
            .with_decorations(false);

        ctx.show_viewport_immediate(
            ViewportId::from_hash_of("pixvix-region"),
            builder,
            |ctx, _| {
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

                        let resp =
                            ui.interact(full, egui::Id::new("pixvix-region-drag"), Sense::drag());
                        if resp.drag_started() {
                            if let Some(p) = resp.interact_pointer_pos() {
                                region.drag = Some((p, p));
                            }
                        }
                        if let (Some((start, _)), Some(p)) =
                            (region.drag, resp.interact_pointer_pos())
                        {
                            if resp.dragged() {
                                region.drag = Some((start, p));
                            }
                        }

                        // The veil: darken everything OUTSIDE the selection.
                        let sel = region.drag.map(|(a, b)| Rect::from_two_pos(a, b));
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
                            }
                            None => {
                                painter.rect_filled(full, 0, Color32::from_black_alpha(60));
                                painter.text(
                                    full.center(),
                                    Align2::CENTER_CENTER,
                                    "drag to select · Esc cancels",
                                    FontId::proportional(15.0),
                                    cl::ink::STRONG,
                                );
                            }
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

                        if resp.drag_stopped() {
                            finish = Some(region.drag.map(|(a, b)| (a, b, full.size())));
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            finish = Some(None);
                        }
                    });
            },
        );

        if let Some(done) = finish {
            let state = self.region.take().expect("region state");
            if let Some((a, b, screen)) = done {
                let r = Rect::from_two_pos(a, b);
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
                        Ok(p) => format!("saved {}", p.display()),
                        Err(e) => e,
                    };
                } else {
                    self.status = "region too small".into();
                }
            } else {
                self.status = "region cancelled".into();
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
    refresh_windows: bool,
    pick_window: Option<usize>,
    free_deferred: bool,
    video_start: bool,
    video_stop: bool,
    steps_start: bool,
    steps_mark: bool,
    steps_stop: bool,
}

#[allow(clippy::too_many_arguments)]
fn palette(
    ui: &mut egui::Ui,
    tab: Tab,
    status: &str,
    windows: &[String],
    picked: usize,
    video_elapsed: Option<u64>,
    steps_note: &str,
    actions: &mut PaletteActions,
) {
    // Title row: identity + close.
    ui.horizontal(|ui| {
        ui.label(RichText::new(cl::icon::SNIP).color(TONE.color()).size(14.0));
        ui.label(RichText::new("PixVix").strong().color(cl::ink::STRONG));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if chip(ui, cl::icon::CLOSE, "", TONE) {
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
            if ui.selectable_label(active, rich).clicked() {
                actions.tab = Some(t);
            }
        }
    });
    ui.separator();

    match tab {
        Tab::Img => {
            ui.horizontal(|ui| {
                if chip(ui, cl::icon::REGION, "Region", TONE) {
                    actions.begin_region = true;
                }
                if chip(ui, cl::icon::FULL, "Full", TONE) {
                    actions.snap_full = true;
                }
                if chip(ui, cl::icon::FREE, "Free", TONE) {
                    // Honest: the polygon editor is a later slice of PV2.
                    actions.free_deferred = true;
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
                if chip(ui, cl::icon::SNIP, "Snap", TONE) {
                    actions.snap_window = true;
                }
                if ui.small_button("↻").clicked() {
                    actions.refresh_windows = true;
                }
            });
            ui.label(
                RichText::new("Free (lasso) lands with the polygon editor.")
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
                    if chip(ui, cl::icon::STOP, "Stop", TONE) {
                        actions.video_stop = true;
                    }
                });
            }
            None => {
                ui.horizontal(|ui| {
                    if chip(ui, cl::icon::REC, "Record screen", TONE) {
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
        Tab::Session => {
            ui.label(
                RichText::new("Session replay (video-first) lands with PV1c —")
                    .color(cl::ink::MUTED),
            );
            ui.label(
                RichText::new("the inbox/index engine pass. Honest placeholder.")
                    .color(cl::ink::MUTED),
            );
        }
        Tab::Steps => {
            ui.horizontal(|ui| {
                if chip(ui, cl::icon::REC, "Start guide", TONE) {
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
        if chip(ui, cl::icon::MARK, "Mark", TONE) {
            actions.steps_mark = true;
        }
        if chip(ui, cl::icon::STOP, "Stop", TONE) {
            actions.steps_stop = true;
        }
        ui.ctx().request_repaint(); // the pulse never rests while recording
    });
}

fn chip(ui: &mut egui::Ui, glyph: &str, label: &str, tone: Tone) -> bool {
    let text = if label.is_empty() {
        glyph.to_string()
    } else {
        format!("{glyph} {label}")
    };
    ui.add(
        egui::Button::new(RichText::new(text).color(cl::ink::BASE))
            .fill(cl::surface::OVERLAY)
            .stroke(egui::Stroke::new(1.0, tone.wash()))
            .corner_radius(CornerRadius::same(cl::RADIUS_SM)),
    )
    .clicked()
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
