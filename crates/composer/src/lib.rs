//! The Composer (R1a) — the WiredFRONT video editor surface: a four-pane
//! shell whose timeline is a CLIP LIST you build by appending source clips
//! from the artifact index, with single-source preview playback. It writes
//! through `dpl-comp` — the composition is plain, validated `comp.json`, the
//! same AI-editable document the later agent seam edits.
//!
//! R1a is the FOUNDATION: NO stitch render, NO generation, NO matting, NO
//! compositor. The Preview plays the SELECTED layer's source (via the S2
//! `ClipPlayer`); the COMPOSITED preview + ffmpeg stitch render land in R1b.
//! The two-editors law holds: this is never the Workbench slide editor.
//!
//! Layout (R1u, per the reference images): workflow tabs on top, SCENES/
//! ASSETS rail far left, then per-mode — a LEFT tool rail, the canvas/
//! preview center, inspector right, timeline above a full-width BOTTOM
//! toolbar (the transport bar). Manual rects + `new_child` (the house
//! idiom); `egui_tiles` drag-docking stays a later rider.

mod library;
pub use library::library_ui;

use dpl_coldlight::{self as cl, Tone};
use dpl_comp::{Composition, Layer, Source};
use dpl_engine_media::StitchClip;
use dpl_index::Record;
use dpl_video::{clip::ClipPlayer, DECODE_H, DECODE_W};
use egui::{Color32, CornerRadius, Rect, RichText, Sense, UiBuilder};
use egui_pilot_ai::{protocol::Role, NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The default trimmed length for a clip whose duration we cannot probe.
const FALLBACK_CLIP_SECS: f32 = 6.0;

/// The editing session: the comp under edit, where it saves, the current
/// selection + its preview player, the lazily-loaded palette, and an honest
/// status line.
pub struct Composer {
    comp: Composition,
    /// Where `Save` writes (`inbox/<comp-id>/`); None until first save.
    dir: Option<PathBuf>,
    /// The selected layer id.
    selected: Option<String>,
    /// The preview player, keyed by the layer id it was opened for.
    preview: Option<(String, ClipPlayer)>,
    /// Video artifacts from the index — the clips you can append.
    palette: Option<Vec<Record>>,
    /// The off-thread stitch render's shared state.
    render: Arc<Mutex<RenderState>>,
    /// The active workflow tab (the editor's current mode).
    tab: Tab,
    /// The selected canvas tool — the LEFT tool rail's persisted choice,
    /// shared by the Edit and Annotate modes.
    tool: Tool,
    status: String,
}

impl Default for Composer {
    fn default() -> Self {
        Self {
            comp: Composition::default_1080p30(),
            dir: None,
            selected: None,
            preview: None,
            palette: None,
            render: Arc::new(Mutex::new(RenderState::Idle)),
            tab: initial_tab(),
            tool: Tool::Select,
            status: "New composition — add clips from the palette on the left.".into(),
        }
    }
}

/// One deferred UI action, applied after the panels release their borrows.
enum Act {
    New,
    Save,
    Render,
    Tab(Tab),
    Tool(Tool),
    Add(usize),
    Select(String),
    Remove(String),
    MoveUp(String),
    MoveDown(String),
}

/// The editor's workflow modes — the top-tab spine (WiredFRONT inventory).
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Capture,
    Annotate,
    Edit,
    Narrate,
    Preview,
    Export,
}

impl Tab {
    const ALL: [Tab; 6] = [
        Tab::Capture,
        Tab::Annotate,
        Tab::Edit,
        Tab::Narrate,
        Tab::Preview,
        Tab::Export,
    ];
    fn label(self) -> &'static str {
        match self {
            Tab::Capture => "Capture",
            Tab::Annotate => "Annotate",
            Tab::Edit => "Edit",
            Tab::Narrate => "Narrate",
            Tab::Preview => "Preview",
            Tab::Export => "Export",
        }
    }
    fn blurb(self) -> &'static str {
        match self {
            Tab::Capture => {
                "Grab screens, windows, and recordings — the ViziWiz overlay feeds here."
            }
            Tab::Annotate => {
                "Holographic arrows, numbered callouts, and spotlights over each scene."
            }
            Tab::Edit => "Arrange and stitch your clips into one video.",
            Tab::Narrate => "Record or drop a voiceover; the waveform rides the timeline.",
            Tab::Preview => "Play the guide as your audience will — Touch, Mouse, or Keyboard.",
            Tab::Export => "Ship a self-contained .guide, or render the video.",
        }
    }
}

/// The canvas tools — the LEFT tool rail (WiredFRONT reference: tool
/// selection lives on the left edge, like every serious editor). Selection
/// is real and persisted; each tool's canvas behaviour lands with the
/// annotation model.
#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Arrow,
    Callout,
    Box,
    Spotlight,
    Text,
}

impl Tool {
    const ALL: [Tool; 6] = [
        Tool::Select,
        Tool::Arrow,
        Tool::Callout,
        Tool::Box,
        Tool::Spotlight,
        Tool::Text,
    ];
    fn label(self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Arrow => "Arrow",
            Tool::Callout => "Callout",
            Tool::Box => "Box",
            Tool::Spotlight => "Spot",
            Tool::Text => "Text",
        }
    }
    fn blurb(self) -> &'static str {
        match self {
            Tool::Select => "Pick and move things on the canvas.",
            Tool::Arrow => "Draw a holographic arrow pointing at what matters.",
            Tool::Callout => "Drop a numbered callout bubble.",
            Tool::Box => "Draw a highlight box around a region.",
            Tool::Spotlight => "Dim everything except the spot you choose.",
            Tool::Text => "Place styled text over the scene.",
        }
    }
}

/// The async stitch-render job's state — the toolbar polls it each frame.
enum RenderState {
    Idle,
    Running,
    Done(PathBuf),
    Failed(String),
}

impl Composer {
    /// The content root, resolved from the same law the Gallery uses.
    fn content_root() -> PathBuf {
        dpl_engine_capture::content_root()
    }

    /// Resolve a forward-slash artifact rel_path to a real file path under the
    /// content root (component-wise, so Windows separators stay correct).
    fn resolve(rel: &str) -> PathBuf {
        rel.split('/')
            .fold(Self::content_root(), |acc, seg| acc.join(seg))
    }

    /// The main track's layers, in order — R1a edits the first track.
    fn layers(&self) -> &[Layer] {
        self.comp
            .tracks
            .first()
            .map(|t| t.layers.as_slice())
            .unwrap_or(&[])
    }

    /// A layer id unique across the comp: the first free `clip-N`.
    fn next_layer_id(&self) -> String {
        let used: std::collections::BTreeSet<&str> = self
            .comp
            .tracks
            .iter()
            .flat_map(|t| t.layers.iter())
            .map(|l| l.id.as_str())
            .collect();
        (1..)
            .map(|n| format!("clip-{n}"))
            .find(|id| !used.contains(id.as_str()))
            .unwrap()
    }

    /// Append a palette record as a new layer, stitched onto the end.
    fn add_clip(&mut self, rec: &Record) {
        let path = Self::resolve(&rec.rel_path);
        let out_pt = rec
            .seconds
            .filter(|s| *s > 0)
            .map(|s| s as f32)
            .or_else(|| dpl_video::clip::clip_duration(&path).filter(|d| *d > 0.0))
            .unwrap_or(FALLBACK_CLIP_SECS);
        let start = self.comp.duration();
        let id = self.next_layer_id();
        // default_1080p30 guarantees a "main" track exists.
        if let Some(track) = self.comp.tracks.first_mut() {
            track.layers.push(Layer {
                id: id.clone(),
                source: Source::Artifact {
                    rel: rec.rel_path.clone(),
                },
                start,
                in_pt: 0.0,
                out_pt,
            });
        }
        self.status = format!(
            "Added {} — {} clip(s), {:.1}s total.",
            rec.name,
            self.layers().len(),
            self.comp.duration()
        );
        self.select(&id);
    }

    /// Select a layer and (re)open its preview if it is a playable artifact.
    fn select(&mut self, id: &str) {
        self.selected = Some(id.to_string());
        let source = self
            .comp
            .tracks
            .iter()
            .flat_map(|t| t.layers.iter())
            .find(|l| l.id == id)
            .map(|l| l.source.clone());
        match source {
            Some(Source::Artifact { rel }) => {
                let path = Self::resolve(&rel);
                self.preview = Some((id.to_string(), ClipPlayer::open(path)));
            }
            // Elements have no captured file yet (R1c ingests them).
            _ => self.preview = None,
        }
    }

    fn remove(&mut self, id: &str) {
        if let Some(track) = self.comp.tracks.first_mut() {
            track.layers.retain(|l| l.id != id);
        }
        if self.selected.as_deref() == Some(id) {
            self.selected = None;
            self.preview = None;
        }
        self.status = format!("Removed {id} — {} clip(s) left.", self.layers().len());
    }

    fn move_by(&mut self, id: &str, delta: isize) {
        if let Some(track) = self.comp.tracks.first_mut() {
            if let Some(i) = track.layers.iter().position(|l| l.id == id) {
                let j = i as isize + delta;
                if j >= 0 && (j as usize) < track.layers.len() {
                    track.layers.swap(i, j as usize);
                }
            }
        }
    }

    /// Persist `comp.json` to an inbox folder (inbox-born per R1 §1). The
    /// index write-through + keep-into-episode are R1c.
    fn save(&mut self) {
        if self.dir.is_none() {
            self.dir = Some(Self::fresh_comp_dir());
        }
        let dir = self.dir.clone().unwrap();
        self.status = match dpl_comp::save(&dir, &self.comp) {
            Ok(()) => format!("Saved to {}", dir.display()),
            Err(e) => format!("Save failed: {e}"),
        };
    }

    /// The next free `inbox/comp-N/` directory — deterministic, no clobber.
    fn fresh_comp_dir() -> PathBuf {
        let inbox = Self::content_root().join("inbox");
        (1..)
            .map(|n| inbox.join(format!("comp-{n}")))
            .find(|p| !p.exists())
            .unwrap()
    }

    /// The main track's playable (artifact) layers → stitch clips, in order.
    /// Element sources have no file yet (R1c) — skipped.
    fn stitch_clips(&self) -> Vec<StitchClip> {
        self.layers()
            .iter()
            .filter_map(|l| match &l.source {
                Source::Artifact { rel } => Some(StitchClip {
                    path: Self::resolve(rel),
                    in_pt: l.in_pt,
                    out_pt: l.out_pt,
                }),
                Source::Element { .. } => None,
            })
            .collect()
    }

    /// Kick off the ffmpeg stitch render OFF the UI thread; the toolbar polls
    /// the shared `RenderState`. Output colocates with the comp as
    /// `<comp-dir>/render.mp4` (the comp folder is minted if unsaved).
    fn start_render(&mut self) {
        let clips = self.stitch_clips();
        if clips.is_empty() {
            *self.render.lock().expect("render state") =
                RenderState::Failed("no playable clips to render yet".into());
            return;
        }
        if self.dir.is_none() {
            self.dir = Some(Self::fresh_comp_dir());
        }
        let out = self.dir.clone().unwrap().join("render.mp4");
        let (w, h, fps) = (self.comp.w, self.comp.h, self.comp.fps);
        let state = Arc::clone(&self.render);
        *state.lock().expect("render state") = RenderState::Running;
        std::thread::Builder::new()
            .name("dpl-composer-render".into())
            .spawn(move || {
                let result = dpl_engine_media::render_stitch(&clips, w, h, fps, &out);
                *state.lock().expect("render state") = match result {
                    Ok(()) => RenderState::Done(out),
                    Err(e) => RenderState::Failed(e),
                };
            })
            .expect("spawn render thread");
    }

    /// A short display path: relative to the content root when possible.
    fn short(p: &Path) -> String {
        p.strip_prefix(Self::content_root())
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/")
    }

    fn ensure_palette(&mut self) {
        if self.palette.is_some() {
            return;
        }
        let rows = dpl_index::Index::open_default()
            .and_then(|ix| ix.recent(500, None))
            .map(|rows| {
                rows.into_iter()
                    .filter(|r| r.kind == "video")
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.palette = Some(rows);
    }

    /// The Composer surface — what the WiredFRONT hub calls for its subroute.
    /// The editor SHELL: a workflow-tab spine (top) + a SCENES/ASSETS rail
    /// (left) + the active mode's surface (WiredFRONT editor inventory).
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_palette();
        // A render in flight keeps the frame alive so its status pill updates.
        if matches!(
            *self.render.lock().expect("render state"),
            RenderState::Running
        ) {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        let mut act: Option<Act> = None;

        let full = ui.max_rect();
        let tabbar_h = 44.0_f32;
        let rail_w = 196.0_f32;
        let pad = cl::space::S2;
        let tabbar_rect =
            Rect::from_min_max(full.min, egui::pos2(full.max.x, full.min.y + tabbar_h));
        let body_top = full.min.y + tabbar_h + pad;
        let rail_rect = Rect::from_min_max(
            egui::pos2(full.min.x, body_top),
            egui::pos2(full.min.x + rail_w, full.max.y),
        );
        let content_rect =
            Rect::from_min_max(egui::pos2(full.min.x + rail_w + pad, body_top), full.max);

        // Recessed plates behind the rail + content (voltage WELL).
        for r in [rail_rect, content_rect] {
            ui.painter()
                .rect_filled(r, CornerRadius::same(cl::RADIUS), cl::surface::WELL);
        }

        let mut mk = |rect: Rect| ui.new_child(UiBuilder::new().max_rect(rect.shrink(pad)));
        self.workflow_tabs(&mut mk(tabbar_rect), &mut act);
        self.left_rail(&mut mk(rail_rect), &mut act);
        match self.tab {
            Tab::Edit => self.edit_tab(&mut mk(content_rect), &mut act),
            Tab::Preview => self.preview_tab(&mut mk(content_rect)),
            Tab::Annotate => self.annotate_tab(&mut mk(content_rect), &mut act),
            other => tab_stub(&mut mk(content_rect), other),
        }

        match act {
            Some(Act::New) => *self = Composer::default(),
            Some(Act::Save) => self.save(),
            Some(Act::Render) => self.start_render(),
            Some(Act::Tab(t)) => self.tab = t,
            Some(Act::Tool(t)) => self.tool = t,
            Some(Act::Add(i)) => {
                if let Some(rec) = self.palette.as_ref().and_then(|p| p.get(i)).cloned() {
                    self.add_clip(&rec);
                }
            }
            Some(Act::Select(id)) => self.select(&id),
            Some(Act::Remove(id)) => self.remove(&id),
            Some(Act::MoveUp(id)) => self.move_by(&id, -1),
            Some(Act::MoveDown(id)) => self.move_by(&id, 1),
            None => {}
        }
    }

    /// The workflow-tab spine — the editor's modes; active glows voltage.
    fn workflow_tabs(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.horizontal(|ui| {
            for t in Tab::ALL {
                let active = self.tab == t;
                let tone = if active { Tone::Magenta } else { Tone::Cyan };
                let resp = cl::tip(chip(ui, tone, t.label(), ("tab", t.label())), t.blurb());
                ui.pilot_track(
                    &resp,
                    NodeMetadata::new(
                        PilotId::new("composer.tab").child(&PilotKey::new(t.label())),
                        Role::Button,
                        t.label(),
                    )
                    .value(if active { "active" } else { "available" }),
                );
                if resp.clicked() && !active {
                    *act = Some(Act::Tab(t));
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if cl::tip(
                    chip(ui, Tone::Magenta, "Export .guide", "export-guide"),
                    "Ship a self-contained guide (the export mode)",
                )
                .clicked()
                {
                    *act = Some(Act::Tab(Tab::Export));
                }
            });
        });
    }

    /// The left rail — SCENES (single-composition stub for now) over the
    /// ASSETS clip list.
    fn left_rail(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.label(
            RichText::new("SCENES")
                .small()
                .strong()
                .color(cl::ink::MUTED),
        );
        cl::clay().show(ui, |ui| {
            ui.label(RichText::new("Full composition").strong());
            ui.label(
                RichText::new(format!(
                    "{} clip(s) · {:.1}s",
                    self.layers().len(),
                    self.comp.duration()
                ))
                .small()
                .color(cl::ink::SUBTLE),
            );
        });
        ui.label(
            RichText::new("Multiple scenes land in the next pass.")
                .small()
                .color(cl::ink::SUBTLE),
        );
        ui.add_space(cl::space::S3);
        self.palette_pane(ui, act);
    }

    /// The EDIT mode — the working editor laid out like the reference:
    /// TOOLS on the LEFT edge, preview center, inspector right, the
    /// timeline above a full-width BOTTOM toolbar (the transport bar).
    fn edit_tab(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        let rect = ui.max_rect();
        let toolbar_h = 44.0_f32;
        let tools_w = 76.0_f32;
        // The inspector yields at small windows so the preview never
        // starves (the 1100x700 fit-guard minimum is a real size).
        let inspector_w = (rect.width() * 0.25).clamp(170.0, 240.0);
        let timeline_h = 132.0_f32;
        let pad = cl::space::S2;
        // The BOTTOM toolbar spans the full width; everything works above it.
        let work_bottom = (rect.max.y - toolbar_h - pad).max(rect.min.y + 1.0);
        let toolbar_rect =
            Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - toolbar_h), rect.max);
        // The LEFT tool strip runs from the top down to the toolbar.
        let tools_rect =
            Rect::from_min_max(rect.min, egui::pos2(rect.min.x + tools_w, work_bottom));
        let body_left = rect.min.x + tools_w + pad;
        let body_bottom = (work_bottom - timeline_h - pad).max(rect.min.y + 1.0);
        let timeline_rect = Rect::from_min_max(
            egui::pos2(body_left, body_bottom + pad),
            egui::pos2(rect.max.x, work_bottom),
        );
        let inspector_rect = Rect::from_min_max(
            egui::pos2(rect.max.x - inspector_w, rect.min.y),
            egui::pos2(rect.max.x, body_bottom),
        );
        let preview_rect = Rect::from_min_max(
            egui::pos2(body_left, rect.min.y),
            egui::pos2(rect.max.x - inspector_w - pad, body_bottom),
        );
        for r in [
            tools_rect,
            preview_rect,
            inspector_rect,
            timeline_rect,
            toolbar_rect,
        ] {
            ui.painter()
                .rect_filled(r, CornerRadius::same(cl::RADIUS), cl::surface::BASE);
        }
        let mut mk = |r: Rect| ui.new_child(UiBuilder::new().max_rect(r.shrink(pad)));
        self.tool_rail(&mut mk(tools_rect), act);
        self.preview_pane(&mut mk(preview_rect));
        self.inspector_pane(&mut mk(inspector_rect));
        self.timeline_pane(&mut mk(timeline_rect), act);
        self.toolbar(&mut mk(toolbar_rect), act);
    }

    /// The LEFT tool rail — the persisted canvas-tool selection, one
    /// full-width button per tool; the active tool presses in (the
    /// neubrutal Active state's [1,1] shadow).
    fn tool_rail(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.label(
            RichText::new("TOOLS")
                .small()
                .strong()
                .color(cl::ink::MUTED),
        );
        ui.add_space(cl::space::S1);
        for t in Tool::ALL {
            let active = self.tool == t;
            let resp = cl::tip(tool_btn(ui, t.label(), active), t.blurb());
            ui.pilot_track(
                &resp,
                NodeMetadata::new(
                    PilotId::new("composer.tool").child(&PilotKey::new(t.label())),
                    Role::Button,
                    t.label(),
                )
                .value(if active { "active" } else { "available" }),
            );
            if resp.clicked() && !active {
                *act = Some(Act::Tool(t));
            }
            ui.add_space(cl::space::S1);
        }
    }

    /// PREVIEW mode — the composition in a device frame. Plays the selected
    /// clip for now; the composited SEQUENCE preview lands with R1b.2.
    fn preview_tab(&self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let controls_h = 40.0_f32;
        let stage = Rect::from_min_max(
            rect.min,
            egui::pos2(rect.max.x, (rect.max.y - controls_h).max(rect.min.y + 1.0)),
        );
        let frame = fit_aspect(stage.shrink(cl::space::S3), 16.0 / 9.0);
        ui.painter()
            .rect_filled(frame, CornerRadius::same(cl::RADIUS), cl::surface::WELL);
        match &self.preview {
            Some((_, player)) => match player.texture(ui.ctx()) {
                Some(tex) => {
                    let uv = dpl_video::cover_uv(
                        DECODE_W as f32 / DECODE_H as f32,
                        frame.aspect_ratio(),
                    );
                    ui.painter().image(tex, frame, uv, Color32::WHITE);
                    ui.ctx().request_repaint_after(Duration::from_millis(16));
                }
                None => painter_label(ui, frame, "Loading the frame..."),
            },
            None => painter_label(ui, frame, "Select a clip in Edit to preview it here"),
        }
        let bar = Rect::from_min_max(
            egui::pos2(rect.min.x, (rect.max.y - controls_h).max(rect.min.y + 1.0)),
            rect.max,
        );
        let mut c = ui.new_child(UiBuilder::new().max_rect(bar));
        c.horizontal(|ui| {
            ui.label(
                RichText::new("Simulate input:")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            for (i, name) in ["Touch", "Mouse", "Keyboard"].iter().enumerate() {
                let tone = if i == 0 { Tone::Magenta } else { Tone::Cyan };
                cl::tip(
                    chip(ui, tone, name, ("input", *name)),
                    "How the preview simulates your audience (selection lands next)",
                );
            }
        });
    }

    /// ANNOTATE mode — the SAME left tool rail as Edit (one selection, one
    /// muscle memory) beside the scene canvas. Tool behaviour + the
    /// annotation model land in the annotation pass; the canvas shows the
    /// selected frame today.
    fn annotate_tab(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        let rect = ui.max_rect();
        let tools_w = 76.0_f32;
        let pad = cl::space::S2;
        let tools_rect = Rect::from_min_max(rect.min, egui::pos2(rect.min.x + tools_w, rect.max.y));
        let canvas =
            Rect::from_min_max(egui::pos2(rect.min.x + tools_w + pad, rect.min.y), rect.max);
        ui.painter().rect_filled(
            tools_rect,
            CornerRadius::same(cl::RADIUS),
            cl::surface::BASE,
        );
        let mut tr = ui.new_child(UiBuilder::new().max_rect(tools_rect.shrink(pad)));
        self.tool_rail(&mut tr, act);
        ui.painter()
            .rect_filled(canvas, CornerRadius::same(cl::RADIUS), cl::surface::WELL);
        match &self.preview {
            Some((_, player)) => match player.texture(ui.ctx()) {
                Some(tex) => {
                    let frame = fit_aspect(canvas.shrink(cl::space::S3), 16.0 / 9.0);
                    let uv = dpl_video::cover_uv(
                        DECODE_W as f32 / DECODE_H as f32,
                        frame.aspect_ratio(),
                    );
                    ui.painter().image(tex, frame, uv, Color32::WHITE);
                    ui.ctx().request_repaint_after(Duration::from_millis(16));
                }
                None => painter_label(ui, canvas, "Loading the frame..."),
            },
            None => painter_label(
                ui,
                canvas,
                "Select a clip in Edit; annotate it here (arrows, callouts, spotlights next)",
            ),
        }
    }

    fn toolbar(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        // Snapshot the render state once (don't hold the lock across the UI).
        let (rendering, pill) = {
            let st = self.render.lock().expect("render state");
            match &*st {
                RenderState::Idle => (false, None),
                RenderState::Running => (true, Some(("Rendering...".to_string(), cl::ink::BASE))),
                RenderState::Done(p) => (
                    false,
                    Some((format!("Rendered: {}", Self::short(p)), cl::ink::STRONG)),
                ),
                RenderState::Failed(e) => {
                    (false, Some((format!("Render failed: {e}"), cl::ink::MUTED)))
                }
            }
        };
        ui.horizontal(|ui| {
            if cl::tip(
                chip(ui, Tone::Lime, "New", "new"),
                "Start a fresh composition",
            )
            .clicked()
            {
                *act = Some(Act::New);
            }
            if cl::tip(
                chip(ui, Tone::Cyan, "Save", "save"),
                "Write this composition to your inbox as comp.json",
            )
            .clicked()
            {
                *act = Some(Act::Save);
            }
            // Render is offered only with clips to stitch and no job running.
            if !self.layers().is_empty()
                && !rendering
                && cl::tip(
                    chip(ui, Tone::Magenta, "Render", "render"),
                    "Stitch these clips into one video file in your inbox",
                )
                .clicked()
            {
                *act = Some(Act::Render);
            }
            if let Some((text, color)) = pill {
                ui.label(RichText::new(text).small().color(color));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} clip(s) · {:.1}s · {}×{} @{}fps",
                        self.layers().len(),
                        self.comp.duration(),
                        self.comp.w,
                        self.comp.h,
                        self.comp.fps
                    ))
                    .small()
                    .color(cl::ink::SUBTLE),
                );
            });
        });
    }

    fn palette_pane(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.label(RichText::new("PALETTE").small().strong());
        ui.label(
            RichText::new("Video clips in your library — tap to add.")
                .small()
                .color(cl::ink::SUBTLE),
        );
        ui.add_space(cl::space::S2);
        let clips = self.palette.as_deref().unwrap_or(&[]);
        if clips.is_empty() {
            ui.label(
                RichText::new("No video clips yet — capture or generate some.")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, rec) in clips.iter().enumerate() {
                let label = format!("{}  ·  {}", rec.name, fmt_secs(rec.seconds));
                let resp = cl::tip(
                    chip(ui, Tone::Magenta, &label, ("palette", i)),
                    "Add this clip to the end of your composition",
                );
                ui.pilot_track(
                    &resp,
                    NodeMetadata::new(
                        PilotId::new("composer.palette.add").child(&PilotKey::new(&rec.rel_path)),
                        Role::Button,
                        &rec.name,
                    ),
                );
                if resp.clicked() {
                    *act = Some(Act::Add(i));
                }
                ui.add_space(cl::space::S1);
            }
        });
    }

    fn timeline_pane(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.label(RichText::new("TIMELINE — the clip list").small().strong());
        ui.label(
            RichText::new("Clips play in this order. Tap to select; ^ v to reorder.")
                .small()
                .color(cl::ink::SUBTLE),
        );
        ui.add_space(cl::space::S1);
        let layers = self.layers();
        if layers.is_empty() {
            ui.label(
                RichText::new("Empty — add a clip from the palette.")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            return;
        }
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for layer in layers {
                    let selected = self.selected.as_deref() == Some(layer.id.as_str());
                    let tone = if selected { Tone::Lime } else { Tone::Cyan };
                    let title = format!("{}\n{:.1}s", layer.source.reference(), layer.span());
                    let resp = cl::tip(
                        chip(ui, tone, &title, ("clip", layer.id.as_str())),
                        "Select this clip to preview and edit it",
                    );
                    ui.pilot_track(
                        &resp,
                        NodeMetadata::new(
                            PilotId::new("composer.clip").child(&PilotKey::new(&layer.id)),
                            Role::Button,
                            &layer.id,
                        )
                        .value(if selected {
                            "selected"
                        } else {
                            "available"
                        }),
                    );
                    if resp.clicked() {
                        *act = Some(Act::Select(layer.id.clone()));
                    }
                    if selected {
                        ui.vertical(|ui| {
                            if cl::tip(
                                small_btn(ui, "^", ("up", layer.id.as_str())),
                                "Move this clip earlier",
                            )
                            .clicked()
                            {
                                *act = Some(Act::MoveUp(layer.id.clone()));
                            }
                            if cl::tip(
                                small_btn(ui, "v", ("down", layer.id.as_str())),
                                "Move this clip later",
                            )
                            .clicked()
                            {
                                *act = Some(Act::MoveDown(layer.id.clone()));
                            }
                            if cl::tip(
                                small_btn(ui, "x", ("rm", layer.id.as_str())),
                                "Remove this clip from the composition",
                            )
                            .clicked()
                            {
                                *act = Some(Act::Remove(layer.id.clone()));
                            }
                        });
                    }
                    ui.add_space(cl::space::S2);
                }
            });
        });
    }

    fn inspector_pane(&self, ui: &mut egui::Ui) {
        ui.label(RichText::new("INSPECTOR").small().strong());
        let Some(id) = self.selected.as_deref() else {
            ui.label(
                RichText::new("Select a clip to see its details.")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            return;
        };
        let Some(layer) = self.layers().iter().find(|l| l.id == id) else {
            return;
        };
        ui.add_space(cl::space::S2);
        cl::clay().show(ui, |ui| {
            ui.label(RichText::new(&layer.id).strong());
            ui.label(
                RichText::new(layer.source.reference())
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            ui.add_space(cl::space::S1);
            ui.label(RichText::new(format!("Starts at {:.2}s", layer.start)).small());
            ui.label(RichText::new(format!("Trim in {:.2}s", layer.in_pt)).small());
            ui.label(RichText::new(format!("Trim out {:.2}s", layer.out_pt)).small());
            ui.label(RichText::new(format!("Plays for {:.2}s", layer.span())).small());
        });
        ui.add_space(cl::space::S2);
        ui.label(
            RichText::new("Trim + transform editing lands in R1b.")
                .small()
                .color(cl::ink::SUBTLE),
        );
    }

    fn preview_pane(&self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        match &self.preview {
            Some((_, player)) => match player.texture(ui.ctx()) {
                Some(tex) => {
                    let uv =
                        dpl_video::cover_uv(DECODE_W as f32 / DECODE_H as f32, rect.aspect_ratio());
                    ui.painter().image(tex, rect, uv, Color32::WHITE);
                    ui.ctx().request_repaint_after(Duration::from_millis(16));
                }
                None => {
                    plate(
                        ui,
                        "Loading preview…",
                        "The clip is decoding (needs ffmpeg on PATH).",
                    );
                    ui.ctx().request_repaint_after(Duration::from_millis(120));
                }
            },
            None => plate(
                ui,
                "Preview",
                "Select a clip to preview it. The composited preview lands in R1b.",
            ),
        }
    }
}

/// A full-rect tappable brutal chip (touch law: the whole surface is the hit
/// target), painted only via ColdLight.
fn chip(
    ui: &mut egui::Ui,
    tone: Tone,
    text: &str,
    key: impl std::hash::Hash + std::fmt::Debug,
) -> egui::Response {
    let inner = cl::brutal(tone).show(ui, |ui| {
        ui.label(RichText::new(text).strong());
    });
    ui.interact(
        inner.response.rect,
        egui::Id::new(("composer-chip", key)),
        Sense::click(),
    )
}

/// One LEFT-rail tool button: full rail width, treated neubrutal; the
/// active tool composes in the Active state, so it visibly presses in.
fn tool_btn(ui: &mut egui::Ui, label: &'static str, active: bool) -> egui::Response {
    let tone = if active { Tone::Magenta } else { Tone::Cyan };
    let state = if active {
        cl::TraitState::Active
    } else {
        cl::TraitState::Idle
    };
    let inner = cl::treated(tone, "neubrutalist", &[], state)
        .margin(egui::Margin::symmetric(4, 7))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(label).small().strong());
            });
        });
    ui.interact(
        inner.response.rect,
        egui::Id::new(("composer-tool", label)),
        Sense::click(),
    )
}

/// A compact square button for the per-clip reorder/remove controls.
fn small_btn(
    ui: &mut egui::Ui,
    glyph: &str,
    key: impl std::hash::Hash + std::fmt::Debug,
) -> egui::Response {
    let inner = cl::brutal(Tone::Magenta)
        .margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(glyph).strong());
        });
    ui.interact(
        inner.response.rect,
        egui::Id::new(("composer-sbtn", key)),
        Sense::click(),
    )
}

/// The initial workflow tab — Edit by default. A proof/env override lets
/// screenshots open a specific mode (`DPL_COMPOSER_TAB=preview|annotate|…`),
/// same spirit as the app's other proof flags; inert in normal use.
fn initial_tab() -> Tab {
    match std::env::var("DPL_COMPOSER_TAB").ok().as_deref() {
        Some("capture") => Tab::Capture,
        Some("annotate") => Tab::Annotate,
        Some("narrate") => Tab::Narrate,
        Some("preview") => Tab::Preview,
        Some("export") => Tab::Export,
        _ => Tab::Edit,
    }
}

/// An honest stub for a workflow mode not yet built — names what it will do.
fn tab_stub(ui: &mut egui::Ui, tab: Tab) {
    plate(ui, tab.label(), tab.blurb());
}

/// The largest rect of the given aspect ratio, centered inside `r`.
fn fit_aspect(r: Rect, aspect: f32) -> Rect {
    let (w, h) = (r.width().max(1.0), r.height().max(1.0));
    let (fw, fh) = if w / h > aspect {
        (h * aspect, h)
    } else {
        (w, w / aspect)
    };
    Rect::from_center_size(r.center(), egui::vec2(fw, fh))
}

/// Centered muted text painted directly (for canvas/stage placeholders).
fn painter_label(ui: &egui::Ui, rect: Rect, text: &str) {
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(15.0),
        cl::ink::SUBTLE,
    );
}

/// A passive claymorphic plate with a title + hint — the empty/loading state.
fn plate(ui: &mut egui::Ui, title: &str, hint: &str) {
    cl::clay().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(cl::space::S3);
            ui.label(RichText::new(title).heading());
            ui.label(RichText::new(hint).color(cl::ink::SUBTLE));
            ui.add_space(cl::space::S3);
        });
    });
}

/// Seconds → a compact "6s" / "1:04" / "—" label.
fn fmt_secs(secs: Option<u32>) -> String {
    match secs {
        None | Some(0) => "—".into(),
        Some(s) if s < 60 => format!("{s}s"),
        Some(s) => format!("{}:{:02}", s / 60, s % 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video_rec(name: &str, rel: &str, secs: Option<u32>) -> Record {
        Record {
            rel_path: rel.into(),
            kind: "video".into(),
            name: name.into(),
            seconds: secs,
            ..Default::default()
        }
    }

    #[test]
    fn add_clip_stitches_onto_the_end_and_stays_valid() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("Intro", "inbox/intro.mp4", Some(5)));
        c.add_clip(&video_rec("Body", "inbox/body.mp4", Some(8)));
        assert_eq!(c.layers().len(), 2);
        // Second clip starts where the first ends (end-to-end stitch).
        assert!((c.layers()[0].start - 0.0).abs() < 1e-6);
        assert!((c.layers()[1].start - 5.0).abs() < 1e-6);
        assert!((c.comp.duration() - 13.0).abs() < 1e-6);
        assert!(dpl_comp::validate(&c.comp).is_empty());
        // Layer ids are unique.
        assert_ne!(c.layers()[0].id, c.layers()[1].id);
    }

    #[test]
    fn seconds_missing_falls_back_not_zero() {
        // No probe possible in the test env → the fallback keeps the comp valid
        // (out_pt > in_pt), never a zero-length layer.
        let mut c = Composer::default();
        c.add_clip(&video_rec("Mystery", "inbox/x.mp4", None));
        assert!(c.layers()[0].out_pt > 0.0);
        assert!(dpl_comp::validate(&c.comp).is_empty());
    }

    #[test]
    fn remove_and_reorder_keep_the_comp_valid() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(3)));
        c.add_clip(&video_rec("B", "inbox/b.mp4", Some(4)));
        c.add_clip(&video_rec("C", "inbox/c.mp4", Some(5)));
        let first = c.layers()[0].id.clone();
        c.move_by(&first, 1);
        assert_eq!(c.layers()[1].id, first, "moved later");
        c.remove(&c.layers()[0].id.clone());
        assert_eq!(c.layers().len(), 2);
        assert!(dpl_comp::validate(&c.comp).is_empty());
    }

    #[test]
    fn fmt_secs_reads_in_plain_words() {
        assert_eq!(fmt_secs(None), "—");
        assert_eq!(fmt_secs(Some(0)), "—");
        assert_eq!(fmt_secs(Some(6)), "6s");
        assert_eq!(fmt_secs(Some(64)), "1:04");
    }

    #[test]
    fn stitch_clips_map_in_order_skipping_elements() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(3)));
        c.add_clip(&video_rec("B", "inbox/b.mp4", Some(4)));
        if let Some(t) = c.comp.tracks.first_mut() {
            t.layers.push(Layer {
                id: "el".into(),
                source: Source::Element { id: "spark".into() },
                start: 0.0,
                in_pt: 0.0,
                out_pt: 2.0,
            });
        }
        let clips = c.stitch_clips();
        assert_eq!(clips.len(), 2, "element source is skipped");
        assert!(clips[0].path.to_string_lossy().contains("a.mp4"));
        assert!((clips[1].out_pt - 4.0).abs() < 1e-6);
    }
}
