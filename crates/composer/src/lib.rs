//! The WiredFRONT editor — rebuilt to the operator-approved mock
//! (`docs/design/mocks/wiredfront-editor-mock.html`, v2.1): **After
//! Effects, panel for panel**, inside the `wiredfront/editor` subroute.
//!
//! The skeleton: workflow tabs (Editor · Capture · Annotate · Narrate ·
//! Guide — tailored siblings, never the editor's spine) over the AE
//! screen: tool strip · PROJECT panel (comps / footage / elements /
//! renders) · COMPOSITION viewer + transport · Info (closable) +
//! Effects & Presets · the layer-OUTLINE timeline (one row per layer,
//! twirl-down properties, per-layer lanes, CTI) · status row.
//!
//! W1 is the 2D slice. Ratified LATER phases (honest affordances only):
//! 3D/camera/tracker/motion-blur, the effects registry, elements ingest,
//! transform keyframes (comp schema pass), browse windows (R1y),
//! autosave/undo (R1z), the ephemeral ✦ + chat seam pass. The comp stays
//! plain validated `comp.json` — the document the agent seam edits.

use dpl_coldlight::{self as cl, Tone};
use dpl_comp::{Composition, Layer, Source};
use dpl_engine_media::StitchClip;
use dpl_index::Record;
use dpl_video::{clip::ClipPlayer, DECODE_H, DECODE_W};
use egui::{Color32, CornerRadius, Rect, RichText, Sense, UiBuilder};
use egui_pilot_ai::{protocol::Role, NodeMetadata, PilotId, PilotKey, UiPilotExt};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The default trimmed length for a clip whose duration we cannot probe.
const FALLBACK_CLIP_SECS: f32 = 6.0;
/// Timeline row heights: a layer's lane, a twirled property row, the ruler.
const LANE_H: f32 = 26.0;
const PROP_H: f32 = 20.0;
const RULER_H: f32 = 22.0;
/// The layer OUTLINE column's width (timeline left half).
const OUTLINE_W: f32 = 252.0;

/// The editing session: the comp under edit, where it saves, selection +
/// playback + the preview player, the project-panel caches, and an honest
/// status line.
pub struct Composer {
    comp: Composition,
    /// Where `Save` writes (`inbox/<comp-id>/`); None until first save.
    dir: Option<PathBuf>,
    /// The selected layer id.
    selected: Option<String>,
    /// The preview player, keyed by the layer id it was opened for.
    preview: Option<(String, ClipPlayer)>,
    /// FOOTAGE — video + image artifacts from the index.
    palette: Option<Vec<Record>>,
    /// COMPS — every `inbox/*/comp.json`, as (name, dir).
    comps: Option<Vec<(String, PathBuf)>>,
    /// RENDERS — every `inbox/*/render.mp4`, as (name, rel_path).
    renders: Option<Vec<(String, String)>>,
    /// ELEMENTS — the census count under `content/elements/`.
    elements: Option<usize>,
    /// The footage row highlighted in the project preview well.
    proj_sel: Option<usize>,
    /// The off-thread stitch render's shared state.
    render: Arc<Mutex<RenderState>>,
    /// The active workflow tab (Editor leads; the rest are tailored).
    tab: Tab,
    /// The tool-strip selection (Selection edits today; the rest are
    /// honest placeholders for their ratified passes).
    etool: EditorTool,
    /// The ANNOTATE tab's tool (its own set, its own muscle memory).
    atool: AnnotateTool,
    /// The PLAYHEAD — comp time in seconds. The preview follows it.
    time: f32,
    /// Sequence playback: `time` advances by real dt while true.
    playing: bool,
    /// Timeline zoom — pixels per second (clamped 20–240).
    px_per_sec: f32,
    /// A live timeline drag (trim/move/scrub), cleared on release.
    drag: Option<Drag>,
    /// Layer ids with their property rows twirled open.
    expanded: BTreeSet<String>,
    /// The Info panel (mock: optional, default shown, closable).
    info_open: bool,
    /// The Effects & Presets search text.
    fx_search: String,
    status: String,
}

/// One in-flight timeline drag.
enum Drag {
    /// Trimming the layer's head (left edge) — tail stays anchored.
    TrimIn(String),
    /// Trimming the layer's tail (right edge).
    TrimOut(String),
    /// Moving the layer along its lane; f32 = grab offset in seconds.
    Move(String, f32),
    /// Scrubbing the playhead.
    Scrub,
}

impl Default for Composer {
    fn default() -> Self {
        Self {
            comp: Composition::default_1080p30(),
            dir: None,
            selected: None,
            preview: None,
            palette: None,
            comps: None,
            renders: None,
            elements: None,
            proj_sel: None,
            render: Arc::new(Mutex::new(RenderState::Idle)),
            tab: initial_tab(),
            etool: EditorTool::Select,
            atool: AnnotateTool::Select,
            time: 0.0,
            playing: false,
            px_per_sec: 60.0,
            drag: None,
            expanded: BTreeSet::new(),
            info_open: true,
            fx_search: String::new(),
            status: "New composition — add footage from the Project panel.".into(),
        }
    }
}

/// One deferred UI action, applied after the panels release their borrows.
enum Act {
    New,
    Save,
    Render,
    Tab(Tab),
    ETool(EditorTool),
    ATool(AnnotateTool),
    /// Add a FOOTAGE row (videos join the timeline; images preview only,
    /// honestly, until the image-layers pass) — either way the row lands
    /// in the preview well.
    Add(usize),
    /// Append a RENDER (name, rel_path) as a new layer.
    AddRender(String, String),
    /// Open a comp document folder.
    OpenComp(PathBuf),
    Select(String),
    Remove(String),
    ToggleExpand(String),
    TogglePlay,
    ToStart,
    ToEnd,
    Step(f32),
    ToggleInfo,
    Zoom(f32),
}

/// The workflow tabs — Editor leads; the others are TAILORED surfaces
/// (operator redline 2026-07-24), never the editor's spine.
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Editor,
    Capture,
    Annotate,
    Narrate,
    Guide,
}

impl Tab {
    const ALL: [Tab; 5] = [
        Tab::Editor,
        Tab::Capture,
        Tab::Annotate,
        Tab::Narrate,
        Tab::Guide,
    ];
    fn label(self) -> &'static str {
        match self {
            Tab::Editor => "Editor",
            Tab::Capture => "Capture",
            Tab::Annotate => "Annotate",
            Tab::Narrate => "Narrate",
            Tab::Guide => "Guide",
        }
    }
    fn blurb(self) -> &'static str {
        match self {
            Tab::Editor => "The editor — After Effects-shaped: project, composition, timeline.",
            Tab::Capture => {
                "Grab screens, windows, and recordings — the ViziWiz overlay feeds here."
            }
            Tab::Annotate => {
                "Ink, arrows, callouts over a frame — also live on the presenting toolbar."
            }
            Tab::Narrate => "Record or drop a voiceover; the waveform rides the timeline.",
            Tab::Guide => {
                "Export a standalone double-click guide — tailored, phone-friendly, its own flow."
            }
        }
    }
}

/// The AE tool strip (operator's set). Selection is live; the rest are
/// selectable with honest blurbs naming their ratified passes.
#[derive(Clone, Copy, PartialEq)]
enum EditorTool {
    Select,
    Hand,
    Pen,
    Camera,
    Text,
}

impl EditorTool {
    const ALL: [EditorTool; 5] = [
        EditorTool::Select,
        EditorTool::Hand,
        EditorTool::Pen,
        EditorTool::Camera,
        EditorTool::Text,
    ];
    fn label(self) -> &'static str {
        match self {
            EditorTool::Select => "Select",
            EditorTool::Hand => "Hand",
            EditorTool::Pen => "Pen",
            EditorTool::Camera => "Camera",
            EditorTool::Text => "Text",
        }
    }
    fn blurb(self) -> &'static str {
        match self {
            EditorTool::Select => "Pick and move things — the working tool today.",
            EditorTool::Hand => "Pan the view — lands with the composited-viewer pass.",
            EditorTool::Pen => "Masks and paths — land with the effects pass.",
            EditorTool::Camera => "Orbit the 3D view — 3D is a ratified later phase.",
            EditorTool::Text => "Type on the comp — lands with the preset-text pass.",
        }
    }
}

/// The ANNOTATE tab's tools (its own set — arrows, callouts, spotlights).
#[derive(Clone, Copy, PartialEq)]
enum AnnotateTool {
    Select,
    Arrow,
    Callout,
    Box,
    Spotlight,
    Text,
}

impl AnnotateTool {
    const ALL: [AnnotateTool; 6] = [
        AnnotateTool::Select,
        AnnotateTool::Arrow,
        AnnotateTool::Callout,
        AnnotateTool::Box,
        AnnotateTool::Spotlight,
        AnnotateTool::Text,
    ];
    fn label(self) -> &'static str {
        match self {
            AnnotateTool::Select => "Select",
            AnnotateTool::Arrow => "Arrow",
            AnnotateTool::Callout => "Callout",
            AnnotateTool::Box => "Box",
            AnnotateTool::Spotlight => "Spot",
            AnnotateTool::Text => "Text",
        }
    }
    fn blurb(self) -> &'static str {
        match self {
            AnnotateTool::Select => "Pick and move things on the canvas.",
            AnnotateTool::Arrow => "Draw a holographic arrow pointing at what matters.",
            AnnotateTool::Callout => "Drop a numbered callout bubble.",
            AnnotateTool::Box => "Draw a highlight box around a region.",
            AnnotateTool::Spotlight => "Dim everything except the spot you choose.",
            AnnotateTool::Text => "Place styled text over the scene.",
        }
    }
}

/// One timeline row: a layer's lane, or a twirled-open property line.
enum Row {
    Layer(usize),
    Prop(usize, &'static str),
}

impl Row {
    fn height(&self) -> f32 {
        match self {
            Row::Layer(_) => LANE_H,
            Row::Prop(..) => PROP_H,
        }
    }
}

/// The async stitch-render job's state — the status row polls it.
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

    /// The main track's layers, in row order (row 0 = topmost).
    fn layers(&self) -> &[Layer] {
        self.comp
            .tracks
            .first()
            .map(|t| t.layers.as_slice())
            .unwrap_or(&[])
    }

    /// A layer id unique across the comp: the first free `clip-N`.
    fn next_layer_id(&self) -> String {
        let used: BTreeSet<&str> = self
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

    /// Append a footage record as a new layer, stitched onto the end.
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
            "Added {} — {} layer(s), {:.1}s total.",
            rec.name,
            self.layers().len(),
            self.comp.duration()
        );
        self.select(&id);
    }

    /// Select a layer (outline + Info focus). The PREVIEW follows the
    /// playhead, not the selection — like any real editor.
    fn select(&mut self, id: &str) {
        self.selected = Some(id.to_string());
    }

    /// The layer under the playhead: the FIRST row whose [start, end)
    /// contains `t` (row 0 = topmost wins). Gaps return None.
    fn active_layer_at(layers: &[Layer], t: f32) -> Option<usize> {
        layers
            .iter()
            .position(|l| t >= l.start && t < l.end().max(l.start))
    }

    /// Advance playback and keep the preview player FOLLOWING the
    /// playhead: the active layer's source at `in_pt + (time - start)`.
    /// Layer switches reopen the player; scrubs and drift re-seek it.
    fn tick_playback(&mut self, ctx: &egui::Context) {
        if self.playing {
            let dt = ctx.input(|i| i.stable_dt).min(0.1);
            self.time += dt;
            let dur = self.comp.duration();
            if self.time >= dur {
                self.time = dur;
                self.playing = false;
            }
            ctx.request_repaint_after(Duration::from_millis(16));
        }
        let Some(i) = Self::active_layer_at(self.layers(), self.time) else {
            // A gap (or an empty comp): nothing to follow. Keep the last
            // player around — re-entering its layer resumes cheaply.
            if let Some((_, player)) = self.preview.as_mut() {
                if player.playing() {
                    player.toggle();
                }
            }
            return;
        };
        let layer = &self.layers()[i];
        let (id, rel) = match &layer.source {
            Source::Artifact { rel } => (layer.id.clone(), rel.clone()),
            // Elements have no captured file yet (the ingest pass).
            Source::Element { .. } => return,
        };
        let want = layer.in_pt + (self.time - layer.start);
        if self.preview.as_ref().map(|(k, _)| k.as_str()) != Some(id.as_str()) {
            let mut player = ClipPlayer::open(Self::resolve(&rel));
            player.seek(want);
            self.preview = Some((id, player));
        }
        let playing = self.playing;
        if let Some((_, player)) = self.preview.as_mut() {
            // Re-seek on real divergence only — every seek restarts the
            // ffmpeg pipe, so tolerance beats frame-perfect chasing.
            if (player.position() - want).abs() > 0.35 {
                player.seek(want);
            }
            if player.playing() != playing {
                player.toggle();
            }
        }
    }

    fn remove(&mut self, id: &str) {
        if let Some(track) = self.comp.tracks.first_mut() {
            track.layers.retain(|l| l.id != id);
        }
        self.expanded.remove(id);
        if self.selected.as_deref() == Some(id) {
            self.selected = None;
            self.preview = None;
        }
        self.status = format!("Removed {id} — {} layer(s) left.", self.layers().len());
    }

    /// Persist `comp.json` to an inbox folder (inbox-born per R1 §1).
    fn save(&mut self) {
        if self.dir.is_none() {
            self.dir = Some(Self::fresh_comp_dir());
        }
        let dir = self.dir.clone().unwrap();
        self.status = match dpl_comp::save(&dir, &self.comp) {
            Ok(()) => format!("Saved to {}", dir.display()),
            Err(e) => format!("Save failed: {e:?}"),
        };
        self.comps = None;
    }

    /// Open an existing comp document folder — the COMPS section's tap.
    fn open_comp(&mut self, dir: PathBuf) {
        match dpl_comp::load(&dir) {
            Ok(comp) => {
                self.comp = comp;
                self.dir = Some(dir.clone());
                self.selected = None;
                self.preview = None;
                self.expanded.clear();
                self.time = 0.0;
                self.playing = false;
                self.status = format!("Opened {}", dir.display());
            }
            Err(e) => self.status = format!("Open failed: {e:?}"),
        }
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
    /// Element sources have no file yet (the ingest pass) — skipped.
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

    /// Kick off the ffmpeg stitch render OFF the UI thread; the status row
    /// polls the shared `RenderState`. Output colocates with the comp as
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
        self.renders = None;
    }

    /// A short display path: relative to the content root when possible.
    fn short(p: &Path) -> String {
        p.strip_prefix(Self::content_root())
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/")
    }

    /// Load the PROJECT panel's caches once per visit.
    fn ensure_project(&mut self) {
        if self.palette.is_none() {
            let rows = dpl_index::Index::open_default()
                .and_then(|ix| ix.recent(500, None))
                .map(|rows| {
                    rows.into_iter()
                        .filter(|r| r.kind == "video" || r.kind == "image")
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            self.palette = Some(rows);
        }
        if self.comps.is_none() {
            self.comps = Some(scan_inbox_for(&Self::content_root(), "comp.json"));
        }
        if self.renders.is_none() {
            let root = Self::content_root();
            self.renders = Some(
                scan_inbox_for(&root, "render.mp4")
                    .into_iter()
                    .map(|(name, dir)| {
                        let rel = format!(
                            "inbox/{}/render.mp4",
                            dir.file_name()
                                .map(|n| n.to_string_lossy())
                                .unwrap_or_default()
                        );
                        (name, rel)
                    })
                    .collect(),
            );
        }
        if self.elements.is_none() {
            self.elements = Some(count_files(&Self::content_root().join("elements")));
        }
    }

    /// The timeline's row model: one row per layer, plus the timing
    /// property rows for layers twirled open.
    fn rows(&self) -> Vec<Row> {
        let mut out = Vec::new();
        for (i, l) in self.layers().iter().enumerate() {
            out.push(Row::Layer(i));
            if self.expanded.contains(&l.id) {
                out.push(Row::Prop(i, "Start"));
                out.push(Row::Prop(i, "Trim in"));
                out.push(Row::Prop(i, "Trim out"));
            }
        }
        out
    }

    /// The Composer surface — what the WiredFRONT hub calls for its subroute.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.ensure_project();
        self.tick_playback(ui.ctx());
        // A render in flight keeps the frame alive so its status pill updates.
        if matches!(
            *self.render.lock().expect("render state"),
            RenderState::Running
        ) {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        let mut act: Option<Act> = None;

        let full = ui.max_rect();
        let tabbar_h = 42.0_f32;
        let pad = cl::space::S2;
        let tabbar_rect =
            Rect::from_min_max(full.min, egui::pos2(full.max.x, full.min.y + tabbar_h));
        let body = Rect::from_min_max(
            egui::pos2(full.min.x, full.min.y + tabbar_h + pad),
            full.max,
        );

        if self.tab != Tab::Editor {
            ui.painter()
                .rect_filled(body, CornerRadius::same(cl::RADIUS), cl::surface::WELL);
        }
        let mut mk = |rect: Rect| ui.new_child(UiBuilder::new().max_rect(rect.shrink(pad)));
        self.workflow_tabs(&mut mk(tabbar_rect), &mut act);
        match self.tab {
            Tab::Editor => self.ae_screen(ui, body, &mut act),
            Tab::Annotate => self.annotate_tab(&mut mk(body), &mut act),
            other => tab_stub(&mut mk(body), other),
        }

        match act {
            Some(Act::New) => *self = Composer::default(),
            Some(Act::Save) => self.save(),
            Some(Act::Render) => self.start_render(),
            Some(Act::Tab(t)) => self.tab = t,
            Some(Act::ETool(t)) => {
                self.etool = t;
                if t != EditorTool::Select {
                    self.status = format!("{} — {}", t.label(), t.blurb());
                }
            }
            Some(Act::ATool(t)) => self.atool = t,
            Some(Act::Add(i)) => {
                if let Some(rec) = self.palette.as_ref().and_then(|p| p.get(i)).cloned() {
                    if rec.kind == "video" {
                        self.add_clip(&rec);
                    } else {
                        self.status = format!(
                            "{} — images join the timeline with the image-layers pass.",
                            rec.name
                        );
                    }
                    self.proj_sel = Some(i);
                }
            }
            Some(Act::AddRender(name, rel)) => {
                let rec = Record {
                    rel_path: rel,
                    kind: "video".into(),
                    name,
                    ..Record::default()
                };
                self.add_clip(&rec);
            }
            Some(Act::OpenComp(dir)) => self.open_comp(dir),
            Some(Act::Select(id)) => self.select(&id),
            Some(Act::Remove(id)) => self.remove(&id),
            Some(Act::ToggleExpand(id)) => {
                if !self.expanded.remove(&id) {
                    self.expanded.insert(id);
                }
            }
            Some(Act::TogglePlay) => {
                // Play from the top when the playhead rests at the end.
                if !self.playing && self.time >= self.comp.duration() - 0.01 {
                    self.time = 0.0;
                }
                self.playing = !self.playing;
            }
            Some(Act::ToStart) => {
                self.time = 0.0;
                self.playing = false;
            }
            Some(Act::ToEnd) => {
                self.time = self.comp.duration();
                self.playing = false;
            }
            Some(Act::Step(dt)) => {
                self.time = (self.time + dt).clamp(0.0, self.comp.duration());
                self.playing = false;
            }
            Some(Act::ToggleInfo) => self.info_open = !self.info_open,
            Some(Act::Zoom(f)) => self.px_per_sec = (self.px_per_sec * f).clamp(20.0, 240.0),
            None => {}
        }
    }

    /// The workflow tabs — tailored siblings; active glows voltage.
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
        });
    }

    /// The AE screen: tool strip · project | viewer | info+fx · timeline ·
    /// status row. Manual rects + `new_child` (the house idiom).
    fn ae_screen(&mut self, ui: &mut egui::Ui, body: Rect, act: &mut Option<Act>) {
        let pad = cl::space::S2;
        let tools_h = 40.0_f32;
        let status_h = 32.0_f32;
        // The timeline grows with twirled rows, within honest bounds.
        let rows_h: f32 = self.rows().iter().map(Row::height).sum();
        let timeline_h = (RULER_H + rows_h + 40.0).clamp(150.0, body.height() * 0.42);
        let project_w = 250.0_f32;
        let right_w = 232.0_f32;

        let tools_rect = Rect::from_min_max(body.min, egui::pos2(body.max.x, body.min.y + tools_h));
        let status_rect =
            Rect::from_min_max(egui::pos2(body.min.x, body.max.y - status_h), body.max);
        let tl_top = status_rect.min.y - pad - timeline_h;
        let timeline_rect = Rect::from_min_max(
            egui::pos2(body.min.x, tl_top),
            egui::pos2(body.max.x, status_rect.min.y - pad),
        );
        let main_top = tools_rect.max.y + pad;
        let project_rect = Rect::from_min_max(
            egui::pos2(body.min.x, main_top),
            egui::pos2(body.min.x + project_w, tl_top - pad),
        );
        let right_rect = Rect::from_min_max(
            egui::pos2(body.max.x - right_w, main_top),
            egui::pos2(body.max.x, tl_top - pad),
        );
        let viewer_rect = Rect::from_min_max(
            egui::pos2(project_rect.max.x + pad, main_top),
            egui::pos2(right_rect.min.x - pad, tl_top - pad),
        );
        // The right stack: Info (when open) above Effects & Presets.
        let (info_rect, fx_rect) = if self.info_open {
            let info_h = 120.0_f32.min(right_rect.height() * 0.4);
            let info = Rect::from_min_max(
                right_rect.min,
                egui::pos2(right_rect.max.x, right_rect.min.y + info_h),
            );
            let fx = Rect::from_min_max(
                egui::pos2(right_rect.min.x, info.max.y + pad),
                right_rect.max,
            );
            (Some(info), fx)
        } else {
            (None, right_rect)
        };
        for r in [
            tools_rect,
            project_rect,
            viewer_rect,
            fx_rect,
            timeline_rect,
            status_rect,
        ]
        .into_iter()
        .chain(info_rect)
        {
            ui.painter()
                .rect_filled(r, CornerRadius::same(cl::RADIUS), cl::surface::BASE);
        }
        let mut mk = |r: Rect| ui.new_child(UiBuilder::new().max_rect(r.shrink(pad)));
        self.tool_strip(&mut mk(tools_rect), act);
        self.project_panel(&mut mk(project_rect), act);
        self.viewer_pane(&mut mk(viewer_rect), act);
        if let Some(info) = info_rect {
            self.info_pane(&mut mk(info), act);
        }
        self.fx_pane(&mut mk(fx_rect));
        self.timeline_panel(&mut mk(timeline_rect), act);
        self.status_row(&mut mk(status_rect), act);
    }

    /// The AE tool strip: the operator's set + the later-phase affordances,
    /// every one honestly blurbed.
    fn tool_strip(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.horizontal(|ui| {
            for t in EditorTool::ALL {
                let active = self.etool == t;
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
                    *act = Some(Act::ETool(t));
                }
            }
            ui.add_space(cl::space::S3);
            for (label, blurb) in [
                (
                    "null",
                    "A parenting rig handle — lands with the keyframes pass.",
                ),
                (
                    "pre-comp",
                    "Nest layers as one — lands with the multi-comp pass.",
                ),
                ("track", "Motion tracking — a ratified later phase."),
                ("m.blur", "Motion blur — a ratified later phase."),
                ("3D", "3D layers and cameras — a ratified later phase."),
            ] {
                let resp = cl::tip(small_btn(ui, label, ("strip", label)), blurb);
                if resp.clicked() {
                    // Honest: name the phase instead of pretending.
                    *act = Some(Act::ETool(self.etool));
                }
            }
        });
    }

    /// The PROJECT panel: preview well + COMPS / FOOTAGE / ELEMENTS /
    /// RENDERS sections. Footage taps add videos to the timeline.
    fn project_panel(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.label(RichText::new("PROJECT").small().strong());
        // The preview well: the highlighted footage row's thumbnail.
        let w = ui.available_width();
        let (well, _) = ui.allocate_exact_size(egui::vec2(w, w * 9.0 / 16.0), Sense::hover());
        ui.painter()
            .rect_filled(well, cl::RADIUS_SM, cl::surface::WELL);
        let root = Self::content_root();
        let sel_rec = self
            .proj_sel
            .and_then(|i| self.palette.as_ref().and_then(|p| p.get(i)));
        match sel_rec.and_then(|r| media_thumb_uri(r, &root)) {
            Some(uri) => {
                egui::Image::new(uri)
                    .corner_radius(cl::RADIUS_SM)
                    .paint_at(ui, well);
            }
            None => painter_label(ui, well, "Tap footage below to preview it"),
        }
        if let Some(r) = sel_rec {
            ui.label(
                RichText::new(format!("{} · {} · {}", r.name, r.kind, fmt_secs(r.seconds)))
                    .small()
                    .color(cl::ink::SUBTLE),
            );
        }
        ui.add_space(cl::space::S2);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                section_header(ui, "COMPS");
                let comps = self.comps.as_deref().unwrap_or(&[]);
                if comps.is_empty() {
                    empty_line(ui, "none yet — Save writes comp.json to your inbox");
                }
                for (name, dir) in comps {
                    let resp = cl::tip(
                        row_line(ui, "cmp", name, "", ("comp", name.as_str())),
                        "Open this composition",
                    );
                    ui.pilot_track(
                        &resp,
                        NodeMetadata::new(
                            PilotId::new("composer.project.comp").child(&PilotKey::new(name)),
                            Role::Button,
                            name,
                        ),
                    );
                    if resp.clicked() {
                        *act = Some(Act::OpenComp(dir.clone()));
                    }
                }
                section_header(ui, "FOOTAGE");
                let footage = self.palette.as_deref().unwrap_or(&[]);
                if footage.is_empty() {
                    empty_line(ui, "no clips or images yet — capture or import some");
                }
                for (i, rec) in footage.iter().enumerate() {
                    let glyph = if rec.kind == "video" { "vid" } else { "img" };
                    let resp = cl::tip(
                        row_line(ui, glyph, &rec.name, &fmt_secs(rec.seconds), ("ft", i)),
                        if rec.kind == "video" {
                            "Add this clip to the timeline"
                        } else {
                            "Preview this image (image layers land in a later pass)"
                        },
                    );
                    ui.pilot_track(
                        &resp,
                        NodeMetadata::new(
                            PilotId::new("composer.project.footage")
                                .child(&PilotKey::new(&rec.rel_path)),
                            Role::Button,
                            &rec.name,
                        ),
                    );
                    if resp.clicked() {
                        *act = Some(Act::Add(i));
                    }
                }
                section_header(ui, "ELEMENTS");
                match self.elements {
                    Some(0) | None => empty_line(
                        ui,
                        "your AE elements appear here once the ingest pass lands",
                    ),
                    Some(n) => empty_line(ui, &format!("{n} element file(s) — ingest pass next")),
                }
                section_header(ui, "RENDERS");
                let renders = self.renders.as_deref().unwrap_or(&[]);
                if renders.is_empty() {
                    empty_line(ui, "finished renders land here");
                }
                for (name, rel) in renders {
                    let resp = cl::tip(
                        row_line(ui, "rnd", name, "", ("rd", name.as_str())),
                        "Add this render to the timeline as footage",
                    );
                    if resp.clicked() {
                        *act = Some(Act::AddRender(name.clone(), rel.clone()));
                    }
                }
            });
    }

    /// The COMPOSITION viewer: the stage + the full transport.
    fn viewer_pane(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        let rect = ui.max_rect();
        let transport_h = 36.0_f32;
        let stage_rect = Rect::from_min_max(
            rect.min,
            egui::pos2(rect.max.x, (rect.max.y - transport_h).max(rect.min.y + 1.0)),
        );
        let frame = fit_aspect(stage_rect.shrink(cl::space::S1), 16.0 / 9.0);
        ui.painter()
            .rect_filled(frame, CornerRadius::same(cl::RADIUS_SM), cl::surface::WELL);
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
                None => painter_label(ui, frame, "Loading the frame… (needs ffmpeg on PATH)"),
            },
            None => painter_label(ui, frame, "Add footage, then press play"),
        }
        let bar = Rect::from_min_max(egui::pos2(rect.min.x, stage_rect.max.y), rect.max);
        let mut c = ui.new_child(UiBuilder::new().max_rect(bar));
        c.horizontal(|ui| {
            let dur = self.comp.duration();
            for (glyph, tip_text, a) in [
                ("⏮", "Jump to the start", Act::ToStart),
                ("◀", "Step back one second", Act::Step(-1.0)),
                (
                    if self.playing { "⏸" } else { "▶" },
                    "Play or pause the sequence (the playhead drives the preview)",
                    Act::TogglePlay,
                ),
                ("▶|", "Step forward one second", Act::Step(1.0)),
                ("⏭", "Jump to the end", Act::ToEnd),
            ] {
                let resp = cl::tip(small_btn(ui, glyph, ("tr", glyph)), tip_text);
                ui.pilot_track(
                    &resp,
                    NodeMetadata::new(
                        PilotId::new("composer.transport").child(&PilotKey::new(glyph)),
                        Role::Button,
                        tip_text,
                    ),
                );
                if resp.clicked() {
                    *act = Some(a);
                }
            }
            ui.label(
                RichText::new(format!("{} / {}", mmss(self.time), mmss(dur)))
                    .monospace()
                    .color(cl::ink::STRONG),
            );
        });
    }

    /// INFO (mock: default shown, closable): the selected layer's facts.
    fn info_pane(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("INFO").small().strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let resp = cl::tip(
                    ui.label(RichText::new("x").small().color(cl::ink::SUBTLE))
                        .interact(Sense::click()),
                    "Close Info (reopen from the status row)",
                );
                if resp.clicked() {
                    *act = Some(Act::ToggleInfo);
                }
            });
        });
        let Some(layer) = self
            .selected
            .as_deref()
            .and_then(|id| self.layers().iter().find(|l| l.id == id))
        else {
            ui.label(
                RichText::new("Select a layer to see its facts.")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            return;
        };
        ui.label(RichText::new(&layer.id).strong());
        ui.label(
            RichText::new(layer.source.reference())
                .small()
                .color(cl::ink::SUBTLE),
        );
        ui.label(
            RichText::new(format!(
                "start {:.2}s · in {:.2}s · out {:.2}s · plays {:.2}s",
                layer.start,
                layer.in_pt,
                layer.out_pt,
                layer.span()
            ))
            .small()
            .color(cl::ink::BASE),
        );
    }

    /// EFFECTS & PRESETS: search + honest stock (the registry and the
    /// elements ingest are their own ratified passes).
    fn fx_pane(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("EFFECTS & PRESETS").small().strong());
        let search = egui::TextEdit::singleline(&mut self.fx_search)
            .hint_text("search effects…")
            .desired_width(f32::INFINITY);
        cl::tip(ui.add(search), "Filter effects and presets by name");
        ui.add_space(cl::space::S1);
        ui.label(
            RichText::new("Transitions · Color · Keying land with the effects-registry pass.")
                .small()
                .color(cl::ink::SUBTLE),
        );
        ui.label(
            RichText::new(
                "Your element presets (color · font · text · ease editable) arrive with the \
                 elements ingest.",
            )
            .small()
            .color(cl::ink::SUBTLE),
        );
    }

    /// The timeline: the layer OUTLINE (left) beside per-layer LANES with
    /// bars + the CTI (right). Trim/move/scrub live on the lanes.
    fn timeline_panel(&mut self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("TIMELINE").small().strong());
            ui.label(
                RichText::new(
                    "tap + for properties · drag edges to trim · body to move · lane to scrub",
                )
                .small()
                .color(cl::ink::SUBTLE),
            );
            if cl::tip(small_btn(ui, "-", "zoom-out"), "Zoom the timeline out").clicked() {
                *act = Some(Act::Zoom(1.0 / 1.4));
            }
            if cl::tip(small_btn(ui, "+", "zoom-in"), "Zoom the timeline in").clicked() {
                *act = Some(Act::Zoom(1.4));
            }
            if let Some(id) = self.selected.clone() {
                if cl::tip(
                    small_btn(ui, "x", ("rm", id.as_str())),
                    "Remove the selected layer from the composition",
                )
                .clicked()
                {
                    *act = Some(Act::Remove(id));
                }
            }
        });
        ui.add_space(cl::space::S1);
        if self.layers().is_empty() {
            ui.label(
                RichText::new("Empty — add footage from the Project panel.")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            return;
        }
        let area = ui.available_rect_before_wrap();
        let outline_rect =
            Rect::from_min_max(area.min, egui::pos2(area.min.x + OUTLINE_W, area.max.y));
        let lanes_rect = Rect::from_min_max(
            egui::pos2(outline_rect.max.x + cl::space::S1, area.min.y),
            area.max,
        );
        self.outline(ui, outline_rect, act);
        self.lanes(ui, lanes_rect);
    }

    /// The layer OUTLINE: #, color chip, twirl, name, in-time — one row
    /// per layer, property lines when twirled open.
    fn outline(&self, ui: &mut egui::Ui, rect: Rect, act: &mut Option<Act>) {
        let painter = ui.painter().clone();
        painter.rect_filled(rect, CornerRadius::same(cl::RADIUS_SM), cl::surface::WELL);
        let mut y = rect.min.y + RULER_H;
        for (row_i, row) in self.rows().iter().enumerate() {
            let h = row.height();
            let r = Rect::from_min_max(
                egui::pos2(rect.min.x, y),
                egui::pos2(rect.max.x, (y + h).min(rect.max.y)),
            );
            if r.min.y >= rect.max.y {
                break;
            }
            match row {
                Row::Layer(i) => {
                    let layer = &self.layers()[*i];
                    let selected = self.selected.as_deref() == Some(layer.id.as_str());
                    if selected {
                        painter.rect_filled(r, CornerRadius::ZERO, Tone::Magenta.wash());
                    }
                    painter.text(
                        egui::pos2(r.min.x + 6.0, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        format!("{}", i + 1),
                        egui::FontId::monospace(10.0),
                        cl::ink::MUTED,
                    );
                    let chip_r = Rect::from_center_size(
                        egui::pos2(r.min.x + 26.0, r.center().y),
                        egui::vec2(9.0, 9.0),
                    );
                    painter.rect_filled(chip_r, CornerRadius::same(2), row_tone(*i).color());
                    let open = self.expanded.contains(&layer.id);
                    painter.text(
                        egui::pos2(r.min.x + 40.0, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        if open { "-" } else { "+" },
                        egui::FontId::monospace(11.0),
                        cl::ink::SUBTLE,
                    );
                    painter.text(
                        egui::pos2(r.min.x + 56.0, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        layer.source.reference(),
                        egui::FontId::proportional(12.0),
                        if selected {
                            cl::ink::STRONG
                        } else {
                            cl::ink::BASE
                        },
                    );
                    painter.text(
                        egui::pos2(r.max.x - 6.0, r.center().y),
                        egui::Align2::RIGHT_CENTER,
                        mmss(layer.start),
                        egui::FontId::monospace(10.0),
                        cl::ink::SUBTLE,
                    );
                    // Two hit zones: the twirl (left 52px) and the row.
                    let twirl = Rect::from_min_max(r.min, egui::pos2(r.min.x + 52.0, r.max.y));
                    let tw_resp =
                        ui.interact(twirl, egui::Id::new(("tl-twirl", row_i)), Sense::click());
                    let tw_resp = cl::tip(tw_resp, "Show this layer's timing properties");
                    if tw_resp.clicked() {
                        *act = Some(Act::ToggleExpand(layer.id.clone()));
                    }
                    let row_resp = ui.interact(
                        Rect::from_min_max(egui::pos2(r.min.x + 52.0, r.min.y), r.max),
                        egui::Id::new(("tl-row", row_i)),
                        Sense::click(),
                    );
                    let row_resp = cl::tip(row_resp, "Select this layer");
                    ui.pilot_track(
                        &row_resp,
                        NodeMetadata::new(
                            PilotId::new("composer.outline").child(&PilotKey::new(&layer.id)),
                            Role::Button,
                            layer.source.reference(),
                        )
                        .value(if selected { "selected" } else { "row" }),
                    );
                    if row_resp.clicked() {
                        *act = Some(Act::Select(layer.id.clone()));
                    }
                }
                Row::Prop(i, name) => {
                    let layer = &self.layers()[*i];
                    let value = match *name {
                        "Start" => format!("{:.2}s", layer.start),
                        "Trim in" => format!("{:.2}s", layer.in_pt),
                        _ => format!("{:.2}s", layer.out_pt),
                    };
                    painter.text(
                        egui::pos2(r.min.x + 56.0, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        *name,
                        egui::FontId::proportional(11.0),
                        cl::ink::SUBTLE,
                    );
                    painter.text(
                        egui::pos2(r.max.x - 6.0, r.center().y),
                        egui::Align2::RIGHT_CENTER,
                        value,
                        egui::FontId::monospace(10.0),
                        Tone::Cyan.color(),
                    );
                }
            }
            y += h;
        }
    }

    /// The LANES: ruler on top, one lane per row, bars for layer rows,
    /// the voltage CTI over everything. One response, manual hit-testing.
    fn lanes(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let pps = self.px_per_sec;
        let dur = self.comp.duration();
        let painter = ui.painter().clone();
        painter.rect_filled(rect, CornerRadius::same(cl::RADIUS_SM), cl::surface::WELL);
        let to_x = |t: f32| rect.min.x + 8.0 + t * pps;
        let to_t = |x: f32| ((x - rect.min.x - 8.0) / pps).max(0.0);

        // ── the RULER ──
        let step = if pps >= 60.0 {
            1.0
        } else if pps >= 30.0 {
            2.0
        } else {
            5.0
        };
        let ruler = Rect::from_min_size(rect.min, egui::vec2(rect.width(), RULER_H));
        let mut t = 0.0_f32;
        while t <= dur + step {
            let x = to_x(t);
            if x > rect.max.x {
                break;
            }
            painter.line_segment(
                [
                    egui::pos2(x, ruler.bottom() - 6.0),
                    egui::pos2(x, ruler.bottom()),
                ],
                egui::Stroke::new(1.0, cl::surface::HAIRLINE),
            );
            painter.text(
                egui::pos2(x + 3.0, ruler.top() + 2.0),
                egui::Align2::LEFT_TOP,
                mmss(t),
                egui::FontId::monospace(9.0),
                cl::ink::SUBTLE,
            );
            t += step;
        }

        // ── the lanes + bars (aligned to the outline's row model) ──
        let mut bars: Vec<(String, Rect)> = Vec::new();
        let mut y = rect.min.y + RULER_H;
        for row in self.rows() {
            let h = row.height();
            if y >= rect.max.y {
                break;
            }
            if let Row::Layer(i) = row {
                let layer = &self.layers()[i];
                let selected = self.selected.as_deref() == Some(layer.id.as_str());
                let bar = Rect::from_min_max(
                    egui::pos2(to_x(layer.start), y + 3.0),
                    egui::pos2(to_x(layer.end()).min(rect.max.x), y + h - 3.0),
                );
                let tone = if selected { Tone::Magenta } else { row_tone(i) };
                painter.rect_filled(bar, CornerRadius::same(cl::RADIUS_SM), tone.wash());
                painter.rect_stroke(
                    bar,
                    CornerRadius::same(cl::RADIUS_SM),
                    egui::Stroke::new(if selected { 2.0 } else { 1.0 }, tone.color()),
                    egui::StrokeKind::Inside,
                );
                if selected {
                    for x in [bar.min.x + 2.0, bar.max.x - 2.0] {
                        painter.line_segment(
                            [
                                egui::pos2(x, bar.min.y + 3.0),
                                egui::pos2(x, bar.max.y - 3.0),
                            ],
                            egui::Stroke::new(2.0, cl::ink::STRONG),
                        );
                    }
                }
                if bar.width() > 48.0 {
                    painter.text(
                        egui::pos2(bar.min.x + 6.0, bar.center().y),
                        egui::Align2::LEFT_CENTER,
                        format!("{}  {:.1}s", layer.source.reference(), layer.span()),
                        egui::FontId::proportional(11.0),
                        cl::ink::STRONG,
                    );
                }
                bars.push((layer.id.clone(), bar));
            }
            y += h;
        }

        // ── the CTI ──
        let px = to_x(self.time);
        if px <= rect.max.x {
            painter.line_segment(
                [egui::pos2(px, rect.min.y), egui::pos2(px, rect.max.y)],
                egui::Stroke::new(2.0, Tone::Magenta.color()),
            );
        }

        // ── interactions ──
        let resp = ui.interact(
            rect,
            egui::Id::new("composer-lanes"),
            Sense::click_and_drag(),
        );
        let resp = cl::tip(
            resp,
            "Tap to scrub · drag a bar's edge to trim it · drag its body to move it",
        );
        ui.pilot_track(
            &resp,
            NodeMetadata::new(PilotId::new("composer.timeline"), Role::Slider, "Timeline")
                .value(format!("{:.1}/{dur:.1}s", self.time)),
        );
        let pointer = resp.interact_pointer_pos();
        if resp.drag_started() {
            if let Some(p) = pointer {
                self.drag = Some(Self::hit_test(&bars, p, pps));
                if let Some(Drag::Move(id, _) | Drag::TrimIn(id) | Drag::TrimOut(id)) = &self.drag {
                    self.selected = Some(id.clone());
                }
            }
        } else if resp.clicked() {
            if let Some(p) = pointer {
                match Self::hit_test(&bars, p, pps) {
                    Drag::Move(id, _) => self.selected = Some(id),
                    _ => {
                        self.time = to_t(p.x).min(dur);
                        self.playing = false;
                    }
                }
            }
        }
        if resp.dragged() {
            if let Some(p) = pointer {
                let pt = to_t(p.x);
                match self.drag.take() {
                    Some(Drag::Scrub) | None => {
                        self.time = pt.min(dur);
                        self.playing = false;
                        self.drag = Some(Drag::Scrub);
                    }
                    Some(d) => {
                        self.apply_drag(&d, pt);
                        self.drag = Some(d);
                    }
                }
            }
        }
        if resp.drag_stopped() {
            self.drag = None;
        }
    }

    /// What a pointer position grabs: a bar edge (trim), a bar body
    /// (move — offset remembered so the layer doesn't jump), or the scrub.
    fn hit_test(bars: &[(String, Rect)], p: egui::Pos2, pps: f32) -> Drag {
        const EDGE: f32 = 8.0;
        for (id, bar) in bars {
            if bar.expand2(egui::vec2(EDGE, 0.0)).contains(p) {
                if (p.x - bar.min.x).abs() <= EDGE {
                    return Drag::TrimIn(id.clone());
                }
                if (p.x - bar.max.x).abs() <= EDGE {
                    return Drag::TrimOut(id.clone());
                }
                if bar.contains(p) {
                    // Grab offset in SECONDS from the bar's start, so the
                    // layer moves with the hand instead of jumping.
                    return Drag::Move(id.clone(), (p.x - bar.min.x) / pps.max(1.0));
                }
            }
        }
        Drag::Scrub
    }

    /// Apply a live drag at pointer-time `pt` (seconds). All edits clamp:
    /// spans never collapse below 0.25s, nothing goes negative, moves
    /// snap to a 0.1s grid.
    fn apply_drag(&mut self, drag: &Drag, pt: f32) {
        let Some(track) = self.comp.tracks.first_mut() else {
            return;
        };
        match drag {
            Drag::TrimIn(id) => {
                if let Some(l) = track.layers.iter_mut().find(|l| &l.id == id) {
                    let delta = pt - l.start;
                    let new_in = (l.in_pt + delta).clamp(0.0, l.out_pt - 0.25);
                    l.start = (l.start + (new_in - l.in_pt)).max(0.0);
                    l.in_pt = new_in;
                }
            }
            Drag::TrimOut(id) => {
                if let Some(l) = track.layers.iter_mut().find(|l| &l.id == id) {
                    l.out_pt = (l.in_pt + (pt - l.start)).max(l.in_pt + 0.25);
                }
            }
            Drag::Move(id, grab) => {
                if let Some(l) = track.layers.iter_mut().find(|l| &l.id == id) {
                    l.start = snap_tenth((pt - grab).max(0.0));
                }
            }
            Drag::Scrub => {}
        }
    }

    /// The status row: New · Save · Render + the render pill, then the
    /// saved crumb + comp facts + the Info reopen toggle.
    fn status_row(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
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
            if !self.layers().is_empty()
                && !rendering
                && cl::tip(
                    chip(ui, Tone::Magenta, "Render", "render"),
                    "Stitch these layers into one video file beside the comp",
                )
                .clicked()
            {
                *act = Some(Act::Render);
            }
            if let Some((text, color)) = pill {
                ui.label(RichText::new(text).small().color(color));
            } else {
                ui.label(RichText::new(&self.status).small().color(cl::ink::SUBTLE));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.info_open {
                    let resp = cl::tip(
                        small_btn(ui, "info", "info-reopen"),
                        "Reopen the Info panel",
                    );
                    if resp.clicked() {
                        *act = Some(Act::ToggleInfo);
                    }
                }
                ui.label(
                    RichText::new(format!(
                        "{} layer(s) · {:.1}s · {}×{} @{}fps · {}",
                        self.layers().len(),
                        self.comp.duration(),
                        self.comp.w,
                        self.comp.h,
                        self.comp.fps,
                        self.dir
                            .as_deref()
                            .map(Self::short)
                            .unwrap_or_else(|| "unsaved".into()),
                    ))
                    .small()
                    .color(cl::ink::SUBTLE),
                );
            });
        });
    }

    /// ANNOTATE — its own tool set beside the scene canvas. Tool behaviour
    /// and the annotation model land in the annotation pass; the canvas
    /// shows the playhead frame today. Ink also rides the presenting toolbar.
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
        tr.label(
            RichText::new("TOOLS")
                .small()
                .strong()
                .color(cl::ink::MUTED),
        );
        tr.add_space(cl::space::S1);
        for t in AnnotateTool::ALL {
            let active = self.atool == t;
            let resp = cl::tip(tool_btn(&mut tr, t.label(), active), t.blurb());
            tr.pilot_track(
                &resp,
                NodeMetadata::new(
                    PilotId::new("composer.annotate.tool").child(&PilotKey::new(t.label())),
                    Role::Button,
                    t.label(),
                )
                .value(if active { "active" } else { "available" }),
            );
            if resp.clicked() && !active {
                *act = Some(Act::ATool(t));
            }
            tr.add_space(cl::space::S1);
        }
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
                "Scrub to a frame in the Editor; annotate it here (the model lands next)",
            ),
        }
    }
}

/// The tone cycle for layer rows/bars (Magenta is the selection voice).
fn row_tone(i: usize) -> Tone {
    [Tone::Cyan, Tone::Lime, Tone::Coral][i % 3]
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

/// One tool button: treated neubrutal; the active tool presses in.
fn tool_btn(ui: &mut egui::Ui, label: &'static str, active: bool) -> egui::Response {
    let tone = if active { Tone::Magenta } else { Tone::Cyan };
    let state = if active {
        cl::TraitState::Active
    } else {
        cl::TraitState::Idle
    };
    let inner = cl::treated(tone, "neubrutalist", &[], state)
        .margin(egui::Margin::symmetric(8, 5))
        .show(ui, |ui| {
            ui.label(RichText::new(label).small().strong());
        });
    ui.interact(
        inner.response.rect,
        egui::Id::new(("composer-tool", label)),
        Sense::click(),
    )
}

/// A compact square button (transport, zoom, strip affordances).
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

/// A PROJECT section header line.
fn section_header(ui: &mut egui::Ui, label: &str) {
    ui.add_space(cl::space::S2);
    ui.label(RichText::new(label).small().strong().color(cl::ink::MUTED));
}

/// An honest muted line for empty sections.
fn empty_line(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).small().color(cl::ink::SUBTLE));
}

/// One tappable project row: glyph · name · trailing value; whole-row hit.
fn row_line(
    ui: &mut egui::Ui,
    glyph: &str,
    name: &str,
    trail: &str,
    key: impl std::hash::Hash + std::fmt::Debug,
) -> egui::Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 22.0), Sense::click());
    let hovered = resp.hovered();
    if hovered {
        ui.painter()
            .rect_filled(rect, cl::RADIUS_SM, cl::surface::OVERLAY);
    }
    ui.painter().text(
        egui::pos2(rect.min.x + 4.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        glyph,
        egui::FontId::monospace(10.0),
        Tone::Cyan.color(),
    );
    ui.painter().text(
        egui::pos2(rect.min.x + 32.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        name,
        egui::FontId::proportional(12.0),
        if hovered {
            cl::ink::STRONG
        } else {
            cl::ink::BASE
        },
    );
    ui.painter().text(
        egui::pos2(rect.max.x - 4.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        trail,
        egui::FontId::monospace(10.0),
        cl::ink::SUBTLE,
    );
    let _ = key;
    resp
}

/// The initial workflow tab — Editor by default. A proof/env override lets
/// screenshots open a specific mode (`DPL_COMPOSER_TAB=capture|annotate|
/// narrate|guide`); inert in normal use.
fn initial_tab() -> Tab {
    match std::env::var("DPL_COMPOSER_TAB").ok().as_deref() {
        Some("capture") => Tab::Capture,
        Some("annotate") => Tab::Annotate,
        Some("narrate") => Tab::Narrate,
        Some("guide") => Tab::Guide,
        _ => Tab::Editor,
    }
}

/// An honest stub for a tailored tab not yet built — names what it will do.
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
        egui::FontId::proportional(14.0),
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

/// A media record's thumbnail source: an image is itself, a video is its
/// poster; None paints the honest kind-label fallback. file:/// with THREE
/// slashes — two reads as a UNC host (the media block's hard-won law).
fn media_thumb_uri(rec: &Record, root: &Path) -> Option<String> {
    let rel = match rec.kind.as_str() {
        "image" => Some(rec.rel_path.clone()),
        "video" => rec.poster_rel_path.clone(),
        _ => None,
    }?;
    let abs = rel.split('/').fold(root.to_path_buf(), |a, s| a.join(s));
    abs.is_file()
        .then(|| format!("file:///{}", abs.to_string_lossy().replace('\\', "/")))
}

/// Every `inbox/*/<marker>` hit, as (folder name, folder path), sorted.
fn scan_inbox_for(root: &Path, marker: &str) -> Vec<(String, PathBuf)> {
    let inbox = root.join("inbox");
    let Ok(rd) = std::fs::read_dir(&inbox) else {
        return Vec::new();
    };
    let mut out: Vec<(String, PathBuf)> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && p.join(marker).is_file())
        .filter_map(|p| {
            p.file_name()
                .map(|n| (n.to_string_lossy().into_owned(), p.clone()))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Count files under a directory tree (the elements census). Absent → 0.
fn count_files(dir: &Path) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    rd.flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                count_files(&p)
            } else {
                1
            }
        })
        .sum()
}

/// Snap to the 0.1s edit grid.
fn snap_tenth(t: f32) -> f32 {
    (t * 10.0).round() / 10.0
}

/// Seconds → "m:ss" (the transport/ruler clock).
fn mmss(t: f32) -> String {
    let s = t.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
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
    fn remove_keeps_the_comp_valid_and_clears_expansion() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(3)));
        c.add_clip(&video_rec("B", "inbox/b.mp4", Some(4)));
        let first = c.layers()[0].id.clone();
        c.expanded.insert(first.clone());
        c.remove(&first);
        assert_eq!(c.layers().len(), 1);
        assert!(c.expanded.is_empty());
        assert!(dpl_comp::validate(&c.comp).is_empty());
    }

    #[test]
    fn rows_model_counts_twirled_properties() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(3)));
        c.add_clip(&video_rec("B", "inbox/b.mp4", Some(4)));
        assert_eq!(c.rows().len(), 2, "collapsed: one row per layer");
        let id = c.layers()[0].id.clone();
        c.expanded.insert(id);
        assert_eq!(c.rows().len(), 5, "twirled: +3 timing property rows");
    }

    #[test]
    fn trim_and_move_clamp_and_snap() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(4)));
        let id = c.layers()[0].id.clone();
        // Trim the tail below the minimum span — clamps to 0.25s.
        c.apply_drag(&Drag::TrimOut(id.clone()), 0.05);
        assert!((c.layers()[0].span() - 0.25).abs() < 1e-6);
        // Move snaps to the 0.1s grid and never goes negative.
        c.apply_drag(&Drag::Move(id.clone(), 0.0), 1.234);
        assert!((c.layers()[0].start - 1.2).abs() < 1e-6);
        c.apply_drag(&Drag::Move(id, 5.0), 1.0);
        assert!(c.layers()[0].start >= 0.0);
        assert!(dpl_comp::validate(&c.comp).is_empty());
    }

    #[test]
    fn active_layer_is_topmost_row_and_gaps_are_none() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(3)));
        c.add_clip(&video_rec("B", "inbox/b.mp4", Some(4)));
        let layers = c.layers();
        assert_eq!(Composer::active_layer_at(layers, 1.0), Some(0));
        assert_eq!(Composer::active_layer_at(layers, 5.0), Some(1));
        assert_eq!(Composer::active_layer_at(layers, 99.0), None);
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

    #[test]
    fn fmt_secs_reads_in_plain_words() {
        assert_eq!(fmt_secs(None), "—");
        assert_eq!(fmt_secs(Some(0)), "—");
        assert_eq!(fmt_secs(Some(6)), "6s");
        assert_eq!(fmt_secs(Some(64)), "1:04");
    }
}
