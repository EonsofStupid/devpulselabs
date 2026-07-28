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
//! W1 is the 2D slice. R1z added the safety layer: snapshot undo/redo,
//! debounced autosave, reopen-last (`state.rs`), and the editor keyboard
//! map. Ratified LATER phases (honest affordances only): 3D/camera/
//! tracker/motion-blur, the effects registry, elements ingest, transform
//! keyframes (comp schema pass), the ephemeral ✦ + chat seam pass. The
//! comp stays plain validated `comp.json` — the document the agent seam
//! edits.

mod browse;
pub mod import;
mod mogrt;
mod packs;
mod rtfx;
mod state;

use dpl_coldlight::{self as cl, Tone};
use dpl_comp::{Composition, Layer, Source};
use dpl_engine_media::StitchClip;
use dpl_index::Record;
use dpl_video::{clip::ClipPlayer, DECODE_H, DECODE_W};
use egui::{Color32, CornerRadius, Rect, RichText, Sense, UiBuilder};
use egui_pilot_ai::{
    protocol::{PilotViewportKey, Role},
    ContextPilotExt, NodeMetadata, PilotId, PilotKey, UiPilotExt,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The default trimmed length for a clip whose duration we cannot probe.
const FALLBACK_CLIP_SECS: f32 = 6.0;
/// How long an IMAGE layer holds by default (W7) — trim to taste.
const DEFAULT_IMAGE_SECS: f32 = 5.0;
/// Timeline row heights: a layer's lane, a twirled property row, the ruler.
const LANE_H: f32 = 26.0;
const PROP_H: f32 = 20.0;
const RULER_H: f32 = 22.0;
/// The FX section divider row's height (L3).
const FXHDR_H: f32 = 18.0;
/// The layer OUTLINE column's width (timeline left half).
const OUTLINE_W: f32 = 252.0;

/// Responsive clamp — the ALL-RESOLUTIONS law. `f32::clamp` PANICS when
/// `min > max`, which is exactly what happens on a small window: a fixed
/// floor (e.g. 150px) can exceed a proportional ceiling (`height * 0.42`),
/// or a degenerate clip can push a ceiling below its floor. `fit` caps the
/// floor to the ceiling (and the ceiling to ≥0), so layout COLLAPSES to the
/// available space instead of crashing. Every bounded size goes through this,
/// never a bare `.clamp()` with a computed bound.
fn fit(value: f32, floor: f32, ceil: f32) -> f32 {
    let ceil = ceil.max(0.0);
    value.clamp(floor.min(ceil), ceil)
}
/// Undo depth — comp snapshots are refs-only JSON, so 100 stays tiny.
const UNDO_CAP: usize = 100;
/// The autosave debounce: a saved comp persists this long after the last
/// edit settles (mid-drag edits wait for the release).
const AUTOSAVE_SECS: f32 = 2.0;
/// The whole-texture UV rect for painting a Lottie frame straight (no crop).
const FULL_UV: Rect = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

/// The editing session: the comp under edit, where it saves, selection +
/// playback + the preview player, the project-panel caches, and an honest
/// status line.
/// The reference-in-place preference, a sentinel file in Ring 0 `.userdata`
/// (presence = on). No serde needed — one boolean choice that sticks.
fn reference_pref_path() -> PathBuf {
    dpl_engine_capture::userdata_root().join("prefer-reference")
}
fn reference_pref_load() -> bool {
    reference_pref_path().exists()
}
fn reference_pref_save(on: bool) {
    let path = reference_pref_path();
    if on {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, b"1");
    } else {
        let _ = std::fs::remove_file(&path);
    }
}

pub struct Composer {
    comp: Composition,
    /// Where `Save` writes (`inbox/<comp-id>/`); None until first save.
    dir: Option<PathBuf>,
    /// The selected layer id.
    selected: Option<String>,
    /// The preview player, keyed by the layer id it was opened for.
    preview: Option<(String, ClipPlayer)>,
    /// The STILL preview lane (W7): (layer id, file:/// uri) when the
    /// active layer is an image — no ffmpeg pipe for a held frame.
    preview_still: Option<(String, String)>,
    /// The LOTTIE preview lane (L2): (layer id, player) when the active
    /// layer is a native Lottie — rendered by the ThorVG subprocess helper.
    preview_lottie: Option<(String, LottiePlayer)>,
    /// OVERLAY preview lanes (L3): one per live overlay-track layer at the
    /// playhead, keyed by layer id — reconciled every tick, capped by the
    /// machine (`PreviewCaps`). Export is NEVER capped.
    overlay_lanes: Vec<(String, OverlayLane)>,
    /// The machine-probed preview budget (operator-ratified: dynamic,
    /// never a hardcoded laptop limit).
    caps: PreviewCaps,
    /// Overlay layers at the playhead the caps skipped this tick — the
    /// viewer names them honestly, never silently.
    overlays_skipped: usize,
    /// FOOTAGE — video + image artifacts from the index.
    palette: Option<Vec<Record>>,
    /// COMPS — every `inbox/*/comp.json`, as (name, dir).
    comps: Option<Vec<(String, PathBuf)>>,
    /// RENDERS — every `inbox/*/render.mp4`, as (name, rel_path).
    renders: Option<Vec<(String, String)>>,
    /// ELEMENTS — the census count under `content/elements/`.
    elements: Option<usize>,
    /// The PACK SHELF — pinned sources' packs, listed live (in place).
    packs: Option<Vec<packs::Pack>>,
    /// The footage row highlighted in the project preview well.
    proj_sel: Option<usize>,
    /// The FOOTAGE palette's active tag filter (L3 — the enricher's tags
    /// made visible; None = everything).
    palette_tag: Option<String>,
    /// The in-flight/last stitch render — a handle into the fabric's
    /// JobBoard (None = never rendered, or the outcome was acknowledged).
    render_job: Option<dpl_fabric::JobId>,
    /// The in-flight/last pack INGEST (one at a time) + the pack's
    /// display name for the progress pill.
    ingest_job: Option<(dpl_fabric::JobId, String)>,
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
    /// Undo history — comp snapshots pushed BEFORE each edit (one per
    /// completed drag). Per-comp: cleared on New/Open.
    undo_stack: Vec<Composition>,
    /// Redo history — filled by undo, cleared by any fresh edit.
    redo_stack: Vec<Composition>,
    /// Edits not yet on disk (drives the autosave + the honest marker).
    dirty: bool,
    /// When the newest edit settled — the autosave debounce clock.
    edited_at: Option<Instant>,
    /// The one-shot reopen-last guard (runs on the first `ui` frame only,
    /// so tests and drills that stage a comp first stay hermetic).
    restored: bool,
    /// Layer ids with their property rows twirled open.
    expanded: BTreeSet<String>,
    /// The Info panel (mock: optional, default shown, closable).
    info_open: bool,
    /// The Effects & Presets search text.
    fx_search: String,
    /// Open Browse windows (peer viewports); slots never reuse.
    browsers: Vec<Arc<Mutex<browse::Browser>>>,
    /// The shared warm scan cache (W8 proactive-buffer law) — seeded at
    /// first visit for profile roots + pinned packs; every Browse
    /// window paints from it instantly.
    scan_cache: browse::ScanCache,
    /// One-shot warm guard.
    warmed: bool,
    /// REFERENCE-in-place mode: imports symlink the source instead of copying
    /// (the operator's paid AE presets / image folders used where they live).
    /// Persisted in `.userdata` so the choice sticks across launches.
    prefer_reference: bool,
    /// The next Browse window's slot id.
    next_browser: usize,
    /// The seeded comp/guide name the guided Save/Export flows use.
    save_name: String,
    /// Keep targets: every CONTENT episode as (show, episode).
    episodes: Option<Vec<(String, String)>>,
    /// GUIDE tab: the pickable step guides (index kind "guide").
    guides: Option<Vec<Record>>,
    /// GUIDE tab: the pickable session replays (index kind "session").
    session_rows: Option<Vec<Record>>,
    /// The session rel_path bundled into the export (optional).
    sel_session: Option<String>,
    /// The open W5 reviewer: one guide's steps under prune/caption edit.
    review: Option<GuideReview>,
    /// Export toggles (operator-ratified defaults: all ON).
    embed_video: bool,
    make_video: bool,
    make_gif: bool,
    /// Two-tap arm for Apply edits — pruning deletes source frames, and a
    /// destructive action never rides one stray tap.
    apply_armed: bool,
    /// The in-flight/last guide EXPORT — a JobBoard handle like the rest.
    export_job: Option<dpl_fabric::JobId>,
    /// The signal fabric (W8, signal-doctrine.md): every mutation this
    /// editor performs signals. A PRIVATE fabric by default so tests and
    /// drills keep a working JobBoard (publishes go to no subscribers);
    /// `wire_fabric` swaps in the app's ONE fabric.
    fabric: dpl_fabric::Fabric,
    /// Capture-domain subscription — drained each frame; any Saved/
    /// InboxChanged invalidates the project caches (refresh chips stop
    /// being load-bearing).
    capture_rx: dpl_fabric::CaptureReceiver,
    /// Ring-0 estate — dual-writes last-comp beside composer-state.json.
    /// None in tests/drills that never wired one.
    estate: Option<std::sync::Arc<dpl_estate::Estate>>,
    /// The WORKFLOWS tab's last run (WF1): report + minted storyboard +
    /// run folder. One run at a time; a rerun replaces it.
    wf_run: Option<WfRun>,
    status: String,
}

/// One completed workflow run — what the Workflows tab paints and the
/// agent reads (the storyboard's slots are semantic nodes).
struct WfRun {
    workflow: String,
    report: dpl_workflows::RunReport,
    board: dpl_workflows::StoryboardDoc,
    dir: PathBuf,
}

/// The W5 reviewer: the selected guide's steps, editable before export —
/// prune (keep off), reorder (moves), caption (typed in place). Apply
/// writes through `engine-steps::edit::update_guide`.
struct GuideReview {
    rel: String,
    dir: PathBuf,
    steps: Vec<ReviewStep>,
}

/// One reviewable step: the manifest step + the operator's working edits.
struct ReviewStep {
    keep: bool,
    caption: String,
    step: dpl_guide::Step,
}

/// One deferred Guide-tab action (collected during paint, applied after —
/// the tab reads several caches while building its rows).
enum GAct {
    PickGuide(String),
    PickSession(String),
    Move(usize, isize),
    Apply,
    Export,
    KeepTo(String, String),
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
        // A PRIVATE fabric until the app wires the real one — the JobBoard
        // works in tests/drills; publishes simply have no subscribers.
        let fabric = dpl_fabric::Fabric::new();
        let capture_rx = fabric.subscribe_capture();
        let mut c = Self {
            comp: Composition::default_1080p30(),
            dir: None,
            selected: None,
            preview: None,
            preview_still: None,
            preview_lottie: None,
            overlay_lanes: Vec::new(),
            caps: PreviewCaps::probe(),
            overlays_skipped: 0,
            palette: None,
            comps: None,
            renders: None,
            elements: None,
            packs: None,
            proj_sel: None,
            palette_tag: None,
            render_job: None,
            ingest_job: None,
            tab: initial_tab(),
            etool: EditorTool::Select,
            atool: AnnotateTool::Select,
            time: 0.0,
            playing: false,
            px_per_sec: 60.0,
            drag: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
            edited_at: None,
            restored: false,
            expanded: BTreeSet::new(),
            info_open: true,
            fx_search: String::new(),
            browsers: Vec::new(),
            scan_cache: browse::ScanCache::default(),
            warmed: false,
            prefer_reference: reference_pref_load(),
            next_browser: 0,
            save_name: String::new(),
            episodes: None,
            guides: None,
            session_rows: None,
            sel_session: None,
            review: None,
            embed_video: true,
            make_video: true,
            make_gif: true,
            apply_armed: false,
            export_job: None,
            fabric,
            capture_rx,
            estate: None,
            wf_run: None,
            status: "New composition — add footage from the Project panel.".into(),
        };
        // Proof/env override (the DPL_COMPOSER_TAB pattern): open the
        // Guide reviewer on one guide for screenshots; inert in normal use.
        if let Ok(rel) = std::env::var("DPL_GUIDE_PICK") {
            c.load_review(&rel);
        }
        c
    }
}

/// One deferred UI action, applied after the panels release their borrows.
enum Act {
    New,
    Save,
    Render,
    /// Run a registered workflow by id (the Workflows tab's Run chip).
    RunWorkflow(String),
    /// Flip reference-in-place mode (symlink imports instead of copy).
    ToggleReference,
    Tab(Tab),
    ETool(EditorTool),
    ATool(AnnotateTool),
    /// Add a FOOTAGE row to the timeline — videos at their probed
    /// length, images held DEFAULT_IMAGE_SECS (W7); the row also lands
    /// in the preview well.
    Add(usize),
    /// Overlay a FOOTAGE row ON TOP of the timeline at the playhead —
    /// alpha composites over the cuts (L3, the FX track).
    AddOverlay(usize),
    /// Filter the FOOTAGE palette by an index tag (None = everything) —
    /// in-memory over the loaded records, the house pattern.
    PaletteTag(Option<String>),
    /// Edit the selected overlay's transform/tint from the inspector.
    /// `fresh` marks the first change of a gesture (one undo step).
    OverlayEdit {
        id: String,
        x: f32,
        y: f32,
        scale: f32,
        tint: Option<[u8; 4]>,
        fresh: bool,
    },
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
    /// Step back through the undo history (Ctrl+Z).
    Undo,
    /// Step forward again (Ctrl+Shift+Z / Ctrl+Y).
    Redo,
    /// Open a new Browse window (a native peer viewport onto a folder).
    Browse,
    /// Open a Browse window rooted at a specific folder (a shelf pack).
    BrowseAt(PathBuf),
    /// File the finished render into (show, episode) — the tag seam.
    Keep(String, String),
}

/// The tab strip — Editor leads; the others are TAILORED surfaces
/// (operator redline 2026-07-24), never the editor's spine.
#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Editor,
    Capture,
    Annotate,
    Narrate,
    Guide,
    Workflows,
}

impl Tab {
    const ALL: [Tab; 6] = [
        Tab::Editor,
        Tab::Capture,
        Tab::Annotate,
        Tab::Narrate,
        Tab::Guide,
        Tab::Workflows,
    ];
    fn label(self) -> &'static str {
        match self {
            Tab::Editor => "Editor",
            Tab::Capture => "Capture",
            Tab::Annotate => "Annotate",
            Tab::Narrate => "Narrate",
            Tab::Guide => "Guide",
            Tab::Workflows => "Workflows",
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
            Tab::Workflows => {
                "Run a guided pipeline — outline, slides, comp — each step reporting honestly."
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
    /// The FX section divider — overlays composite over the cuts (L3).
    FxHeader,
    /// An FX overlay layer (index into the fx track, tracks[1]).
    Fx(usize),
    FxProp(usize, &'static str),
}

impl Row {
    fn height(&self) -> f32 {
        match self {
            Row::Layer(_) | Row::Fx(_) => LANE_H,
            Row::Prop(..) | Row::FxProp(..) => PROP_H,
            Row::FxHeader => FXHDR_H,
        }
    }
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

    /// The FX overlay track's layers (row 0 = topmost), empty pre-L3.
    fn fx_layers(&self) -> &[Layer] {
        self.comp
            .tracks
            .get(1)
            .map(|t| t.layers.as_slice())
            .unwrap_or(&[])
    }

    /// A layer anywhere in the comp, by its comp-unique id.
    fn layer_mut(&mut self, id: &str) -> Option<&mut Layer> {
        self.comp
            .tracks
            .iter_mut()
            .flat_map(|t| t.layers.iter_mut())
            .find(|l| l.id == id)
    }

    /// Overlay-track layers (tracks 1..) under the playhead, in PAINT order
    /// — earlier tracks first, and within a track the BOTTOM row first
    /// (row 0 = topmost, exactly like the base track). The topmost layer is
    /// therefore LAST, and the caps walk keeps it first.
    fn overlay_layers_at(&self, t: f32) -> Vec<&Layer> {
        let mut out = Vec::new();
        for track in self.comp.tracks.iter().skip(1) {
            for layer in track.layers.iter().rev() {
                if t >= layer.start && t < layer.end().max(layer.start) {
                    out.push(layer);
                }
            }
        }
        out
    }

    /// The overlay ("fx") track, created on first use — track 0 stays the
    /// base cuts; tracks 1.. composite over it (dpl-comp track law).
    fn ensure_fx_track(&mut self) -> usize {
        if self.comp.tracks.len() < 2 {
            self.comp.tracks.push(dpl_comp::Track {
                id: "fx".into(),
                layers: Vec::new(),
            });
        }
        1
    }

    /// Place a footage record as an FX OVERLAY at the playhead — the
    /// element composites over whatever the base is showing there, at its
    /// full trimmed span, untinted and full-frame until the inspector says
    /// otherwise.
    fn add_overlay(&mut self, rec: &Record) {
        self.checkpoint();
        let path = Self::resolve(&rec.rel_path);
        let span = if rec.kind == "image" {
            DEFAULT_IMAGE_SECS
        } else {
            rec.seconds
                .filter(|s| *s > 0)
                .map(|s| s as f32)
                .or_else(|| dpl_video::clip::clip_duration(&path))
                .unwrap_or(FALLBACK_CLIP_SECS)
        };
        let at = self.time;
        let id = self.next_layer_id();
        let track = self.ensure_fx_track();
        self.comp.tracks[track].layers.insert(
            0,
            Layer::plain(
                id.clone(),
                Source::Artifact {
                    rel: rec.rel_path.clone(),
                },
                at,
                0.0,
                span,
            ),
        );
        self.status = format!(
            "{} overlays at {at:.1}s — scale, nudge, tint it below.",
            rec.name
        );
        self.select(&id);
        self.mark_edit();
    }

    /// Keep ONE preview lane per overlay layer under the playhead — open on
    /// enter, drop on exit, re-seek clip lanes on drift (the 0.35s law),
    /// re-render Lottie lanes at layer-local time. The machine's caps bound
    /// the live lanes; TOPMOST wins; the viewer names what's skipped.
    fn reconcile_overlays(&mut self, ctx: &egui::Context) {
        let wanted: Vec<(String, Source, f32)> = self
            .overlay_layers_at(self.time)
            .iter()
            .filter(|l| !matches!(l.source, Source::Element { .. }))
            .map(|l| {
                (
                    l.id.clone(),
                    l.source.clone(),
                    (l.in_pt + (self.time - l.start)).max(0.0),
                )
            })
            .collect();

        // Cap from the END of paint order (= topmost first). Stills are
        // pipe-free and never counted.
        let mut keep: Vec<&(String, Source, f32)> = Vec::new();
        let (mut clips, mut lots) = (0usize, 0usize);
        for entry in wanted.iter().rev() {
            let ok = match &entry.1 {
                Source::Lottie { .. } => {
                    lots += 1;
                    lots <= self.caps.lottie_lanes
                }
                Source::Artifact { rel } => {
                    if dpl_engine_media::is_still(&Self::resolve(rel)) {
                        true
                    } else {
                        clips += 1;
                        clips <= self.caps.clip_lanes
                    }
                }
                Source::Element { .. } => false,
            };
            if ok {
                keep.push(entry);
            }
        }
        self.overlays_skipped = wanted.len() - keep.len();

        self.overlay_lanes
            .retain(|(id, _)| keep.iter().any(|(k, _, _)| k == id));
        for (id, source, want) in keep {
            let have = self.overlay_lanes.iter().position(|(k, _)| k == id);
            match source {
                Source::Artifact { rel } => {
                    let path = Self::resolve(rel);
                    if dpl_engine_media::is_still(&path) {
                        if have.is_none() {
                            let uri =
                                format!("file:///{}", path.to_string_lossy().replace('\\', "/"));
                            self.overlay_lanes
                                .push((id.clone(), OverlayLane::Still(uri)));
                        }
                    } else {
                        let idx = match have {
                            Some(i) => i,
                            None => {
                                let mut player = ClipPlayer::open(path);
                                player.seek(*want);
                                self.overlay_lanes
                                    .push((id.clone(), OverlayLane::Clip(player)));
                                self.overlay_lanes.len() - 1
                            }
                        };
                        if let Some((_, OverlayLane::Clip(player))) =
                            self.overlay_lanes.get_mut(idx)
                        {
                            if (player.position() - want).abs() > 0.35 {
                                player.seek(*want);
                            }
                            if player.playing() != self.playing {
                                player.toggle();
                            }
                        }
                    }
                }
                Source::Lottie { rel } => {
                    let path = Self::resolve(rel);
                    let idx = match have {
                        Some(i) => i,
                        None => {
                            let Some(lp) = LottiePlayer::open(&path) else {
                                continue;
                            };
                            self.overlay_lanes
                                .push((id.clone(), OverlayLane::Lottie(lp)));
                            self.overlay_lanes.len() - 1
                        }
                    };
                    if let Some((_, OverlayLane::Lottie(lp))) = self.overlay_lanes.get_mut(idx) {
                        lp.render(ctx, *want);
                    }
                }
                Source::Element { .. } => {}
            }
        }
        if !self.overlay_lanes.is_empty() && self.playing {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    /// Paint the live overlay lanes into the viewer `frame`, back-to-front,
    /// honoring each layer's scale/offset/tint (the tint rides egui's
    /// per-draw multiply — the 4th `painter.image` arg). Overlays
    /// LETTERBOX inside their box (never cover-crop): a full-frame element
    /// must never lose edges. Straight-alpha textures stack correctly.
    fn paint_overlays(&self, ui: &egui::Ui, frame: Rect) {
        let px = frame.width() / (self.comp.w.max(1) as f32);
        for layer in self.overlay_layers_at(self.time) {
            let Some((_, lane)) = self.overlay_lanes.iter().find(|(k, _)| k == &layer.id) else {
                continue;
            };
            let tint = layer
                .tint
                .map(|[r, g, b, a]| Color32::from_rgba_unmultiplied(r, g, b, a))
                .unwrap_or(Color32::WHITE);
            let center = frame.center() + egui::vec2(layer.x * px, layer.y * px);
            let bx = Rect::from_center_size(center, frame.size() * layer.scale);
            match lane {
                OverlayLane::Clip(player) => {
                    if let Some(tex) = player.texture(ui.ctx()) {
                        // The clip decode is 16:9 cover — a 16:9 box keeps it true.
                        let r = fit_aspect(bx, DECODE_W as f32 / DECODE_H as f32);
                        ui.painter().image(tex, r, FULL_UV, tint);
                    }
                }
                OverlayLane::Lottie(lp) => {
                    if let Some(tex) = lp.id() {
                        let r = fit_aspect(bx, lp.aspect());
                        ui.painter().image(tex, r, FULL_UV, tint);
                    }
                }
                OverlayLane::Still(uri) => {
                    egui::Image::new(uri.as_str()).tint(tint).paint_at(ui, bx);
                }
            }
        }
        if self.overlays_skipped > 0 {
            ui.painter().text(
                frame.left_bottom() + egui::vec2(cl::space::S2, -cl::space::S2),
                egui::Align2::LEFT_BOTTOM,
                format!(
                    "{} overlay(s) past this machine's preview budget — all render in export",
                    self.overlays_skipped
                ),
                egui::TextStyle::Small.resolve(ui.style()),
                cl::ink::MUTED,
            );
        }
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
    /// IMAGES are layers too (W7): a still holds DEFAULT_IMAGE_SECS,
    /// trimmable like any bar — no probe, no special schema.
    fn add_clip(&mut self, rec: &Record) {
        self.checkpoint();
        let path = Self::resolve(&rec.rel_path);
        let out_pt = if rec.kind == "image" {
            DEFAULT_IMAGE_SECS
        } else {
            rec.seconds
                .filter(|s| *s > 0)
                .map(|s| s as f32)
                .or_else(|| dpl_video::clip::clip_duration(&path).filter(|d| *d > 0.0))
                .unwrap_or(FALLBACK_CLIP_SECS)
        };
        let start = self.comp.duration();
        let id = self.next_layer_id();
        // default_1080p30 guarantees a "main" track exists.
        if let Some(track) = self.comp.tracks.first_mut() {
            track.layers.push(Layer::plain(
                id.clone(),
                Source::Artifact {
                    rel: rec.rel_path.clone(),
                },
                start,
                0.0,
                out_pt,
            ));
        }
        self.status = format!(
            "Added {} — {} layer(s), {:.1}s total.",
            rec.name,
            self.layers().len(),
            self.comp.duration()
        );
        self.select(&id);
        self.mark_edit();
    }

    /// Append a native Lottie artifact as a `Source::Lottie` layer, timed by
    /// its OWN Bodymovin duration (`op`/`fr`), stitched onto the end.
    fn add_lottie(&mut self, rec: &Record) {
        self.checkpoint();
        let path = Self::resolve(&rec.rel_path);
        let out_pt = dpl_lottie::lottie_meta(&path)
            .map(|m| m.duration_seconds())
            .filter(|d| *d > 0.0)
            .unwrap_or(FALLBACK_CLIP_SECS);
        let start = self.comp.duration();
        let id = self.next_layer_id();
        if let Some(track) = self.comp.tracks.first_mut() {
            track.layers.push(Layer::plain(
                id.clone(),
                Source::Lottie {
                    rel: rec.rel_path.clone(),
                },
                start,
                0.0,
                out_pt,
            ));
        }
        self.status = format!("Added {} — native Lottie, {out_pt:.1}s.", rec.name);
        self.select(&id);
        self.mark_edit();
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
        // L3: overlay lanes reconcile FIRST — they live regardless of what
        // (or whether anything) is under the playhead on the base track.
        self.reconcile_overlays(ctx);
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
        let lid = layer.id.clone();
        let want = layer.in_pt + (self.time - layer.start);
        let source = layer.source.clone(); // owned — releases the layer borrow

        // L2: a native Lottie layer previews on its OWN lane (the ThorVG
        // helper), rendered here so the immutable viewer just reads the tex.
        if let Source::Lottie { rel } = &source {
            let path = Self::resolve(rel);
            if self.preview_lottie.as_ref().map(|(k, _)| k.as_str()) != Some(lid.as_str()) {
                self.preview_lottie = LottiePlayer::open(&path).map(|p| (lid.clone(), p));
            }
            if let Some((_, lp)) = self.preview_lottie.as_mut() {
                lp.render(ctx, want.max(0.0));
            }
            self.preview_still = None;
            if let Some((_, player)) = self.preview.as_mut() {
                if player.playing() {
                    player.toggle();
                }
            }
            if self.playing {
                ctx.request_repaint_after(Duration::from_millis(16));
            }
            return;
        }
        // Not a Lottie — drop that lane.
        self.preview_lottie = None;

        let (id, rel) = match &source {
            Source::Artifact { rel } => (lid, rel.clone()),
            // Elements have no captured file yet (the ingest pass).
            Source::Element { .. } => return,
            Source::Lottie { .. } => return, // handled above
        };
        let path = Self::resolve(&rel);
        if dpl_engine_media::is_still(&path) {
            // A held frame previews as itself — no ffmpeg pipe.
            if self.preview_still.as_ref().map(|(k, _)| k.as_str()) != Some(id.as_str()) {
                let uri = format!("file:///{}", path.to_string_lossy().replace('\\', "/"));
                self.preview_still = Some((id, uri));
            }
            if let Some((_, player)) = self.preview.as_mut() {
                if player.playing() {
                    player.toggle();
                }
            }
            return;
        }
        self.preview_still = None;
        if self.preview.as_ref().map(|(k, _)| k.as_str()) != Some(id.as_str()) {
            let mut player = ClipPlayer::open(path);
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
        self.checkpoint();
        // EVERY track — an FX overlay removes exactly like a base cut.
        for track in &mut self.comp.tracks {
            track.layers.retain(|l| l.id != id);
        }
        self.overlay_lanes.retain(|(k, _)| k != id);
        self.expanded.remove(id);
        if self.selected.as_deref() == Some(id) {
            self.selected = None;
            self.preview = None;
            self.preview_still = None;
            self.preview_lottie = None;
        }
        self.status = format!("Removed {id} — {} layer(s) left.", self.layers().len());
        self.mark_edit();
    }

    /// Push the pre-edit comp onto the undo history (deduped — a no-op
    /// "edit" never stacks twice) and invalidate redo. Every comp
    /// mutation calls this FIRST; a drag calls it once, at grab.
    fn checkpoint(&mut self) {
        if self.undo_stack.last() != Some(&self.comp) {
            self.undo_stack.push(self.comp.clone());
            if self.undo_stack.len() > UNDO_CAP {
                self.undo_stack.remove(0);
            }
        }
        self.redo_stack.clear();
    }

    /// An edit settled: note it for the autosave debounce.
    fn mark_edit(&mut self) {
        self.dirty = true;
        self.edited_at = Some(Instant::now());
    }

    /// Undo — swap the comp back one snapshot (skipping no-op entries),
    /// current comp onto redo.
    fn undo_now(&mut self) {
        while let Some(prev) = self.undo_stack.pop() {
            if prev != self.comp {
                self.redo_stack
                    .push(std::mem::replace(&mut self.comp, prev));
                self.after_history_jump("Undone");
                return;
            }
        }
        self.status = "Nothing to undo.".into();
    }

    /// Redo — the inverse swap.
    fn redo_now(&mut self) {
        while let Some(next) = self.redo_stack.pop() {
            if next != self.comp {
                self.undo_stack
                    .push(std::mem::replace(&mut self.comp, next));
                self.after_history_jump("Redone");
                return;
            }
        }
        self.status = "Nothing to redo.".into();
    }

    /// After a history swap the derived state must match the restored
    /// comp: clamp the playhead, drop dangling selection/twirls, and let
    /// the autosave persist the jump.
    fn after_history_jump(&mut self, verb: &str) {
        let ids: BTreeSet<String> = self.layers().iter().map(|l| l.id.clone()).collect();
        self.expanded.retain(|id| ids.contains(id));
        if let Some(sel) = &self.selected {
            if !ids.contains(sel) {
                self.selected = None;
            }
        }
        self.time = self.time.min(self.comp.duration());
        self.status = format!(
            "{verb} — {} layer(s), {:.1}s.",
            self.layers().len(),
            self.comp.duration()
        );
        self.mark_edit();
    }

    /// Reset to a fresh comp WITHOUT touching the rest of the session —
    /// caches, Browse windows, and the in-flight render/ingest/export
    /// jobs all stay alive (the old `*self = default()` severed them).
    fn new_comp(&mut self) {
        self.flush_save();
        self.comp = Composition::default_1080p30();
        self.dir = None;
        self.selected = None;
        self.preview = None;
        self.preview_still = None;
        self.preview_lottie = None;
        self.overlay_lanes.clear();
        self.proj_sel = None;
        self.time = 0.0;
        self.playing = false;
        self.drag = None;
        self.expanded.clear();
        self.save_name.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.dirty = false;
        self.edited_at = None;
        self.status = "New composition — add footage from the Project panel.".into();
    }

    /// Reopen the last comp, ONCE, on the first frame — and only when the
    /// editor would otherwise open empty-handed (a drill-staged or
    /// restored comp is never stomped). Experience gate row 1.
    fn maybe_restore(&mut self) {
        if self.restored {
            return;
        }
        self.restored = true;
        if self.dir.is_some() || !self.layers().is_empty() {
            return;
        }
        let Some(rel) = state::load(&Self::content_root()).last_comp else {
            return;
        };
        let dir = Self::resolve(&rel);
        if dir.join("comp.json").is_file() {
            self.open_comp(dir);
            if self.dir.is_some() {
                self.status = format!("Reopened {rel} — your last comp.");
            }
        }
    }

    /// The debounced autosave: a SAVED comp persists 2s after the last
    /// edit settles. Unsaved comps wait for the guided Save (the NEXT
    /// hint names it) — autosave never mints folders behind your back.
    fn maybe_autosave(&mut self, ctx: &egui::Context) {
        if !self.dirty || self.dir.is_none() {
            return;
        }
        let elapsed = self
            .edited_at
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(f32::MAX);
        if elapsed >= AUTOSAVE_SECS {
            self.flush_save();
        } else {
            // No input arrives while idle — keep one frame queued so the
            // debounce actually fires.
            ctx.request_repaint_after(Duration::from_secs_f32((AUTOSAVE_SECS - elapsed).max(0.05)));
        }
    }

    /// Write pending edits to the comp's folder NOW (autosave fire,
    /// leaving the comp, app exit). Failures stay dirty and retry on the
    /// next debounce instead of dying silently.
    fn flush_save(&mut self) {
        if !self.dirty {
            return;
        }
        let Some(dir) = self.dir.clone() else {
            return;
        };
        match dpl_comp::save(&dir, &self.comp) {
            Ok(()) => {
                self.dirty = false;
                self.edited_at = None;
            }
            Err(e) => {
                self.status = format!("Autosave failed: {e:?}");
                self.edited_at = Some(Instant::now());
            }
        }
    }

    /// Remember this comp as the one to reopen next run.
    fn remember_last(&self, dir: &Path) {
        let rel = Self::short(dir);
        state::save(
            &Self::content_root(),
            &state::EditorState {
                last_comp: Some(rel.clone()),
            },
        );
        // Dual-write into the estate (seam GET /state lastComp). Best-effort —
        // composer-state.json remains the reopen path this pass.
        if let Some(est) = &self.estate {
            let _ = est.put_json(
                dpl_estate::Tree::State,
                dpl_estate::KEY_WIREDFRONT_LAST_COMP,
                &rel,
            );
            let _ = est.persist();
        }
    }

    /// Share the Ring-0 estate so Save dual-writes last-comp.
    pub fn wire_estate(&mut self, estate: std::sync::Arc<dpl_estate::Estate>) {
        self.estate = Some(estate);
    }

    /// Persist `comp.json` to an inbox folder (inbox-born per R1 §1).
    /// The GUIDED path: a typed name (sanitized to kebab, no-clobber)
    /// names the folder; blank falls back to the seeded `comp-N`.
    fn save(&mut self) {
        if self.dir.is_none() {
            self.dir = Some(match import::sanitize_name(&self.save_name) {
                Some(name) => {
                    let inbox = Self::content_root().join("inbox");
                    let first = inbox.join(&name);
                    if first.exists() {
                        (2..)
                            .map(|n| inbox.join(format!("{name}-{n}")))
                            .find(|p| !p.exists())
                            .unwrap()
                    } else {
                        first
                    }
                }
                None => Self::fresh_comp_dir(),
            });
        }
        let dir = self.dir.clone().unwrap();
        self.status = match dpl_comp::save(&dir, &self.comp) {
            Ok(()) => {
                self.dirty = false;
                self.edited_at = None;
                self.remember_last(&dir);
                format!("Saved to {}", dir.display())
            }
            Err(e) => format!("Save failed: {e:?}"),
        };
        self.comps = None;
    }

    /// Open an existing comp document folder — the COMPS section's tap.
    /// Pending edits on the outgoing comp flush first; history is
    /// per-comp, so it resets with the switch.
    fn open_comp(&mut self, dir: PathBuf) {
        self.flush_save();
        match dpl_comp::load(&dir) {
            Ok(comp) => {
                self.comp = comp;
                self.dir = Some(dir.clone());
                self.selected = None;
                self.preview = None;
                self.preview_still = None;
                self.preview_lottie = None;
                self.overlay_lanes.clear();
                self.expanded.clear();
                self.time = 0.0;
                self.playing = false;
                self.undo_stack.clear();
                self.redo_stack.clear();
                self.dirty = false;
                self.edited_at = None;
                self.remember_last(&dir);
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

    /// Gather the L3 render inputs from the WHOLE comp (all tracks), pure.
    /// Base track: artifacts become concat legs; a Lottie span becomes a
    /// BLACK leg + a same-timed overlay (the native render composites on
    /// top, timeline-true). Overlay tracks: bottom row first within a
    /// track (row 0 = topmost), tracks in order — stack order. The base is
    /// padded black to the comp's full duration so overlays past the last
    /// cut stay visible (output length = concat length).
    fn render_parts(&self) -> RenderParts {
        use dpl_engine_media::{CompBase, OverlayClip, OverlayInput};
        let mut base = Vec::new();
        let mut overlays = Vec::new();
        let mut elements_skipped = 0usize;
        let mut base_end = 0.0f32;
        for l in self.layers() {
            match &l.source {
                Source::Artifact { rel } => {
                    let path = Self::resolve(rel);
                    base.push(CompBase::Clip(StitchClip {
                        still: dpl_engine_media::is_still(&path),
                        path,
                        in_pt: l.in_pt,
                        out_pt: l.out_pt,
                    }));
                    base_end = base_end.max(l.end());
                }
                Source::Lottie { rel } => {
                    base.push(CompBase::Black {
                        dur: l.span().max(0.05),
                    });
                    overlays.push(OverlaySlot::Lottie(PendingLottie {
                        path: Self::resolve(rel),
                        in_pt: l.in_pt,
                        out_pt: l.out_pt,
                        at: l.start,
                        x: l.x,
                        y: l.y,
                        scale: l.scale,
                        tint: l.tint,
                    }));
                    base_end = base_end.max(l.end());
                }
                Source::Element { .. } => elements_skipped += 1,
            }
        }
        for track in self.comp.tracks.iter().skip(1) {
            for l in track.layers.iter().rev() {
                match &l.source {
                    Source::Artifact { rel } => {
                        let path = Self::resolve(rel);
                        let hold = dpl_engine_media::is_still(&path);
                        overlays.push(OverlaySlot::Ready(OverlayClip {
                            input: OverlayInput::File(path),
                            in_pt: l.in_pt,
                            out_pt: l.out_pt,
                            at: l.start,
                            x: l.x,
                            y: l.y,
                            scale: l.scale,
                            tint: l.tint,
                            hold,
                        }));
                    }
                    Source::Lottie { rel } => {
                        overlays.push(OverlaySlot::Lottie(PendingLottie {
                            path: Self::resolve(rel),
                            in_pt: l.in_pt,
                            out_pt: l.out_pt,
                            at: l.start,
                            x: l.x,
                            y: l.y,
                            scale: l.scale,
                            tint: l.tint,
                        }));
                    }
                    Source::Element { .. } => elements_skipped += 1,
                }
            }
        }
        let comp_dur = self.comp.duration();
        if comp_dur > base_end + 0.05 {
            base.push(CompBase::Black {
                dur: comp_dur - base_end,
            });
        }
        if base.is_empty() && !overlays.is_empty() {
            base.push(CompBase::Black {
                dur: comp_dur.max(0.05),
            });
        }
        RenderParts {
            base,
            overlays,
            elements_skipped,
        }
    }

    /// Kick off the ffmpeg stitch render OFF the UI thread; the status row
    /// reads this job's row on the fabric's JobBoard. Output colocates with
    /// the comp as `<comp-dir>/render.mp4` (the comp folder is minted if
    /// unsaved).
    fn start_render(&mut self) {
        let parts = self.render_parts();
        // Honesty: Element layers have no playable file (dormant R1c
        // placeholder) — the render never silently drops them unnamed.
        if parts.elements_skipped > 0 {
            self.status = format!(
                "Rendering — {} element layer(s) have no playable file yet and are skipped.",
                parts.elements_skipped
            );
        }
        // ONE render slot: acknowledging the previous outcome keeps the
        // board tidy — the new row is the pill from here on.
        let job = self.fabric.mint_job();
        if let Some(old) = self.render_job.replace(job) {
            self.fabric.ack_job(old);
        }
        self.fabric.publish_job(dpl_fabric::JobEvent::Queued {
            job,
            label: "render".into(),
        });
        if parts.base.is_empty() {
            // The refusal is an honest fact — queued, failed, on the record.
            self.fabric.publish_job(dpl_fabric::JobEvent::Failed {
                job,
                error: "no playable clips to render yet".into(),
            });
            return;
        }
        if self.dir.is_none() {
            self.dir = Some(Self::fresh_comp_dir());
        }
        let out = self.dir.clone().unwrap().join("render.mp4");
        let (w, h, fps) = (self.comp.w, self.comp.h, self.comp.fps);
        let fabric = self.fabric.clone();
        std::thread::Builder::new()
            .name("dpl-composer-render".into())
            .spawn(
                move || match render_comp_resolving(parts, w, h, fps, &out) {
                    Ok(()) => {
                        // Write-through: the render becomes an ordinary
                        // indexed artifact so Gallery/Keep/agents see it.
                        index_render(&out, Some(&fabric));
                        fabric.publish_job(dpl_fabric::JobEvent::Done {
                            job,
                            out: out.display().to_string(),
                        });
                    }
                    Err(e) => {
                        fabric.publish_job(dpl_fabric::JobEvent::Failed { job, error: e });
                    }
                },
            )
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
        if self.episodes.is_none() {
            self.episodes = Some(
                dpl_doc::list_content_episodes(&Self::content_root())
                    .into_iter()
                    .map(|(show, ep, _)| (show, ep))
                    .collect(),
            );
        }
        if self.packs.is_none() {
            self.packs = Some(packs::list_packs(&Self::content_root()));
        }
        if !self.warmed {
            self.warmed = true;
            // Proactive buffering: the folders a Browse window opens on
            // first are already listed before the first click. Roots are
            // probed INSIDE the worker — never here on the paint thread.
            let pinned: Vec<PathBuf> = self
                .packs
                .iter()
                .flatten()
                .map(|p| p.path.clone())
                .collect();
            browse::warm_cache(&self.scan_cache, pinned);
        }
        if self.guides.is_none() {
            self.guides = Some(
                dpl_index::Index::open_default()
                    .and_then(|ix| ix.list(Some("guide"), None))
                    .unwrap_or_default(),
            );
        }
        if self.session_rows.is_none() {
            self.session_rows = Some(
                dpl_index::Index::open_default()
                    .and_then(|ix| ix.list(Some("session"), None))
                    .unwrap_or_default(),
            );
        }
    }

    /// Open one guide in the reviewer — the Guide tab's pick path.
    fn load_review(&mut self, rel: &str) {
        let dir = Self::resolve(rel);
        match dpl_guide::load_guide(&dir) {
            Ok(m) => {
                self.review = Some(GuideReview {
                    rel: rel.to_string(),
                    dir,
                    steps: m
                        .steps
                        .into_iter()
                        .map(|s| ReviewStep {
                            keep: true,
                            caption: s.caption.clone().unwrap_or_default(),
                            step: s,
                        })
                        .collect(),
                });
                self.apply_armed = false;
            }
            Err(e) => self.status = format!("Open guide failed: {e}"),
        }
    }

    /// Apply the reviewer's edits to the SOURCE guide through the ONE
    /// edit engine (`engine-steps::edit::update_guide`): kept steps in
    /// their new order, captions written, pruned frames deleted.
    fn apply_review(&mut self) {
        let Some(rev) = &self.review else {
            return;
        };
        let kept: Vec<_> = rev
            .steps
            .iter()
            .filter(|s| s.keep)
            .map(review_edit)
            .collect();
        let rel = rev.rel.clone();
        match dpl_engine_steps::edit::update_guide(&rev.dir, kept) {
            Ok(n) => {
                // W8 gap-fix: the edit is a MUTATION — index + signal it
                // (the sweep found update_guide wrote no index row).
                let root = Self::content_root();
                let rec = dpl_index::record_for_capture(&rev.dir.clone(), &root, "guide", "edit");
                if let Ok(ix) = dpl_index::Index::open_default() {
                    if ix.upsert(&rec).is_ok() {
                        Self::signal_saved(&self.fabric, &rec);
                    }
                }
                self.status = format!("Guide updated — {n} step(s) kept.");
                self.guides = None;
                self.load_review(&rel);
            }
            Err(e) => {
                self.signal(dpl_fabric::CaptureEvent::Failed { error: e.clone() });
                self.status = format!("Apply failed: {e}");
            }
        }
        self.apply_armed = false;
    }

    /// Kick off the guide EXPORT off the UI thread; the Guide tab reads
    /// this job's row on the fabric's JobBoard.
    fn start_export(&mut self) {
        if self
            .export_job
            .and_then(|j| self.fabric.job(j))
            .is_some_and(|r| r.running())
        {
            return;
        }
        let Some(rev) = &self.review else {
            self.status = "Pick a step guide first — the export ships its steps.".into();
            return;
        };
        let root = Self::content_root();
        let guide_rel = rev.rel.clone();
        let session_rel = self.sel_session.clone();
        let name = import::sanitize_name(&self.save_name);
        let opts = dpl_guide::ExportOpts {
            embed_video: self.embed_video,
            make_guide_video: self.make_video,
            make_gif: self.make_gif,
            ..Default::default()
        };
        let job = self.fabric.mint_job();
        if let Some(old) = self.export_job.replace(job) {
            self.fabric.ack_job(old);
        }
        self.fabric.publish_job(dpl_fabric::JobEvent::Queued {
            job,
            label: "guide export".into(),
        });
        let fabric = self.fabric.clone();
        std::thread::Builder::new()
            .name("dpl-guide-export".into())
            .spawn(move || {
                match run_export(
                    &root,
                    &guide_rel,
                    session_rel.as_deref(),
                    name.as_deref(),
                    opts,
                    Some(&fabric),
                ) {
                    Ok(e) => {
                        fabric.publish_job(dpl_fabric::JobEvent::Done {
                            job,
                            out: e.dir.display().to_string(),
                        });
                    }
                    Err(e) => {
                        fabric.publish_job(dpl_fabric::JobEvent::Failed { job, error: e });
                    }
                }
            })
            .expect("spawn export thread");
    }

    /// File a finished export into a show/episode — guide folders keep
    /// wholesale (`Index::keep`).
    fn keep_export(&mut self, show: &str, ep: &str) {
        let dir = match self.export_job.and_then(|j| self.fabric.job(j)) {
            Some(row) => match &row.state {
                dpl_fabric::JobState::Done { out } => PathBuf::from(out),
                _ => return,
            },
            None => return,
        };
        let root = Self::content_root();
        let rel = Self::short(&dir);
        self.status =
            match dpl_index::Index::open_default().and_then(|ix| ix.keep(&root, &rel, show, ep)) {
                Ok(new_rel) => {
                    if let Some(job) = self.export_job.take() {
                        self.fabric.ack_job(job);
                    }
                    self.guides = None;
                    self.signal(dpl_fabric::CaptureEvent::InboxChanged);
                    format!("Filed into {show}/{ep} → {new_rel}")
                }
                Err(e) => {
                    self.signal(dpl_fabric::CaptureEvent::Failed { error: e.clone() });
                    format!("Keep failed: {e}")
                }
            };
    }

    /// Open a Browse peer window rooted at `start`.
    fn open_browser(&mut self, start: PathBuf) {
        self.browsers.push(Arc::new(Mutex::new(browse::Browser::new(
            self.next_browser,
            start,
            Arc::clone(&self.scan_cache),
        ))));
        self.next_browser += 1;
    }

    /// INGEST one pack OFF the UI thread: copy its media into
    /// `content/elements/<pack>/`, indexed and postered — the operator's
    /// "grab the ones I often use". One at a time; sources untouched.
    fn start_ingest(&mut self, pack: &Path) {
        if self
            .ingest_job
            .as_ref()
            .and_then(|(j, _)| self.fabric.job(*j))
            .is_some_and(|r| r.running())
        {
            self.status = "One ingest at a time — the current one is still copying.".into();
            return;
        }
        let name = pack
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "pack".into());
        let pack = pack.to_path_buf();
        // The BATCHING law (doctrine law 8): a 9k-file ingest speaks the
        // jobs domain + ONE InboxChanged at the end — never a Saved storm.
        let job = self.fabric.mint_job();
        if let Some((old, _)) = self.ingest_job.replace((job, name.clone())) {
            self.fabric.ack_job(old);
        }
        self.fabric.publish_job(dpl_fabric::JobEvent::Queued {
            job,
            label: format!("ingest {name}"),
        });
        // The board row's counter IS the progress pill (doctrine law 7) —
        // the worker bumps it per file, lock-free.
        let done = self.fabric.job(job).map(|r| r.progress).unwrap_or_default();
        let fabric = self.fabric.clone();
        std::thread::Builder::new()
            .name("dpl-pack-ingest".into())
            .spawn(move || {
                let root = Self::content_root();
                // The ADR-014 index (.userdata/index.db) — the one every
                // read surface opens. The old ad-hoc root/artifacts.db
                // open wrote a RETIRED legacy DB nothing read: ingested
                // packs were invisible in the library on a migrated
                // install (verified 2026-07-27, plan §2).
                let result = dpl_index::Index::open_default()
                    .and_then(|ix| packs::ingest_copy(&root, &pack, &name, &ix, &done));
                match result {
                    Ok(copied) => {
                        fabric.publish_job(dpl_fabric::JobEvent::Done {
                            job,
                            out: format!("{copied} file(s)"),
                        });
                        fabric.publish_capture(dpl_fabric::CaptureEvent::InboxChanged);
                    }
                    Err(e) => {
                        fabric.publish_job(dpl_fabric::JobEvent::Failed { job, error: e });
                    }
                }
            })
            .expect("spawn ingest thread");
    }

    /// Pin one folder as a pack source — the shelf lists it in place;
    /// nothing is copied. The Browse-window chip and the
    /// `--pin-pack <dir>` harness seam both land here.
    pub fn pin_pack(&mut self, dir: &Path) {
        self.status = match packs::add_source(&Self::content_root(), dir) {
            Ok(()) => format!("Pinned {} to your Elements shelf.", dir.display()),
            Err(e) => format!("Pin failed: {e}"),
        };
        self.packs = None;
    }

    /// Show every open Browse window as a DEFERRED peer viewport (the
    /// chat window's law: a real window or an honest failure, never a
    /// silent embed). Closed windows are dropped.
    fn show_browsers(&mut self, ctx: &egui::Context) {
        for b in &self.browsers {
            let (slot, title, open) = {
                let b = lock(b);
                (b.slot, b.title(), b.open)
            };
            if !open {
                continue;
            }
            let builder = egui::ViewportBuilder::default()
                .with_title(title)
                .with_inner_size([560.0, 640.0])
                .with_min_inner_size([380.0, 420.0])
                .with_resizable(true)
                .with_decorations(true)
                .with_taskbar(true);
            let shared = Arc::clone(b);
            ctx.pilot_show_viewport_deferred(
                PilotViewportKey::new(format!("devpulse.browse.{slot}")),
                egui::ViewportId::from_hash_of(("dpl.browse", slot)),
                builder,
                move |ui, class| {
                    let mut b = lock(&shared);
                    if class == egui::ViewportClass::EmbeddedWindow {
                        ui.colored_label(
                            Tone::Coral.color(),
                            "Browse needs native multi-viewport support; this backend \
                             embedded the window.",
                        );
                        b.open = false;
                        return;
                    }
                    if ui.ctx().input(|i| i.viewport().close_requested()) {
                        b.open = false;
                    }
                    egui::CentralPanel::default()
                        .frame(
                            egui::Frame::new()
                                .fill(cl::surface::WELL)
                                .inner_margin(egui::Margin::same(8)),
                        )
                        .show(ui, |ui| browse::window_ui(ui, &mut b));
                },
            );
        }
        self.browsers.retain(|b| lock(b).open);
    }

    /// ONE funnel for both feeders: Browse-window picks and files
    /// dropped from Explorer onto the main window. Imports copy into
    /// the content tree and write through the index; videos also land
    /// on the timeline. Any import refreshes the project caches.
    fn drain_media_feeds(&mut self, ctx: &egui::Context) {
        let mut incoming: Vec<PathBuf> = Vec::new();
        let mut pins: Vec<PathBuf> = Vec::new();
        let mut ingests: Vec<PathBuf> = Vec::new();
        for b in &self.browsers {
            let mut b = lock(b);
            incoming.append(&mut b.picked);
            pins.append(&mut b.pinned);
            if let Some(dir) = b.ingest_req.take() {
                ingests.push(dir);
            }
        }
        for dir in pins {
            self.pin_pack(&dir);
        }
        for dir in ingests {
            self.start_ingest(&dir);
        }
        incoming.extend(
            ctx.input(|i| i.raw.dropped_files.clone())
                .into_iter()
                .filter_map(|f| f.path),
        );
        if incoming.is_empty() {
            return;
        }
        for src in incoming {
            self.handle_import(&src);
        }
        self.palette = None;
        self.elements = None;
    }

    /// The harness seam (`--save-comp <name>`): save the booted comp
    /// under `name` through the SAME path the Save chip takes — paired
    /// with `--import-drill`, the reopen-last drill's first half.
    pub fn save_comp(&mut self, name: &str) {
        self.save_name = name.to_string();
        self.save();
    }

    /// The `--lottie-import <file>` harness seam (L2): import ONE Lottie
    /// (`.json`/`.lottie`) through the SAME path a drop takes — copy +
    /// index + add a native `Source::Lottie` layer to the timeline.
    pub fn import_lottie(&mut self, path: &Path) {
        self.handle_import(path);
        self.palette = None;
    }

    /// The `--fx-drill <clip-rel> <fx-rel>` harness seam (L3): one base
    /// cut + one FX overlay at 1.0s — half-size, nudged off-center,
    /// pool-tinted (Coral) — through the EXACT app paths (add_clip,
    /// add_overlay, inspector-shaped edits), saved, then rendered
    /// SYNCHRONOUSLY so the headless proof can ffprobe `render.mp4` the
    /// moment the process exits. Verdict on stdout.
    pub fn fx_drill(&mut self, clip_rel: &str, fx_rel: &str) {
        let base = Record {
            rel_path: clip_rel.into(),
            kind: "video".into(),
            name: "drill base".into(),
            ..Record::default()
        };
        self.add_clip(&base);
        self.time = 1.0;
        let fx = Record {
            rel_path: fx_rel.into(),
            kind: "video".into(),
            name: "drill fx".into(),
            ..Record::default()
        };
        self.add_overlay(&fx);
        if let Some(id) = self.selected.clone() {
            let coral = Tone::Coral.color();
            let tint = Some([coral.r(), coral.g(), coral.b(), 230]);
            if let Some(l) = self.layer_mut(&id) {
                l.scale = 0.5;
                l.x = -120.0;
                l.y = 40.0;
                l.tint = tint;
            }
        }
        if self.dir.is_none() {
            self.dir = Some(Self::fresh_comp_dir());
        }
        let dir = self.dir.clone().expect("comp dir");
        if let Err(e) = dpl_comp::save(&dir, &self.comp) {
            println!("fx-drill: comp save failed: {e}");
            return;
        }
        let out = dir.join("render.mp4");
        let parts = self.render_parts();
        match render_comp_resolving(parts, self.comp.w, self.comp.h, self.comp.fps, &out) {
            Ok(()) => {
                index_render(&out, Some(&self.fabric));
                self.status = format!("FX drill rendered {}", out.display());
                println!("fx-drill: rendered {}", out.display());
            }
            Err(e) => {
                self.status = format!("FX drill failed: {e}");
                println!("fx-drill: FAILED: {e}");
            }
        }
    }

    /// One indexed media record sent from an OUTSIDE surface — the
    /// Library shelf's double-click (W6). Videos AND images (W7) join
    /// the timeline through the same path a FOOTAGE tap takes —
    /// checkpoint, undo, autosave all ride. The Nth feeding surface
    /// calls this same one seam.
    pub fn add_from_library(&mut self, rec: &Record) {
        self.add_clip(rec);
    }

    /// Turn a library artifact into a standalone MEDIA SLIDE (W11) — the
    /// SIBLING of `add_from_library` (that feeds the timeline; this makes a
    /// slide). Mints an `inbox/<name>/` folder holding a `slide.json` whose
    /// media block refs the artifact IN PLACE (the asset already lives in
    /// content — never a copy). A session replay is just a video artifact, so
    /// it rides this same path. Returns the new slide dir and signals the
    /// inbox changed so pickers refresh.
    pub fn make_slide_from_library(&self, rec: &Record) -> Result<PathBuf, String> {
        let dir = Self::mint_media_slide(rec)?;
        self.signal(dpl_fabric::CaptureEvent::InboxChanged);
        Ok(dir)
    }

    /// The pure mint (no signal) — the harness seam and tests call this.
    /// Validates the document BEFORE it touches disk (invalid → Err, nothing
    /// written), then crash-safe writes `slide.json`.
    pub fn mint_media_slide(rec: &Record) -> Result<PathBuf, String> {
        let slide = Self::media_slide_for(rec);
        if let Some(i) = dpl_doc::validate(&slide).first() {
            return Err(format!("media slide invalid at {}: {}", i.path, i.message));
        }
        let dir = Self::fresh_slide_dir(&rec.name);
        dpl_doc::save(&dir, &slide).map_err(|e| format!("save slide: {e}"))?;
        Ok(dir)
    }

    /// The media-slide document for an artifact record: `template:"media"`,
    /// bare background, one full-bleed media block refing `rec.rel_path`. A
    /// video carries its poster for the degradation ladder.
    fn media_slide_for(rec: &Record) -> dpl_doc::Slide {
        let show = if rec.kind == "video" {
            "video"
        } else {
            "image"
        };
        dpl_doc::Slide {
            v: dpl_doc::SCHEMA_V,
            template: "media".into(),
            background: Some(dpl_doc::Background::None),
            blocks: vec![dpl_doc::Block::Media {
                id: "media".into(),
                show: show.into(),
                src: rec.rel_path.clone(),
                fit: Some("cover".into()),
                poster: rec.poster_rel_path.clone(),
            }],
            enter: None,
            reveals: Vec::new(),
        }
    }

    /// The next free `inbox/<name>/` slide folder — the artifact's name
    /// kebab'd (fallback "media-slide"), no-clobber with a `-N` suffix.
    fn fresh_slide_dir(name_hint: &str) -> PathBuf {
        let inbox = Self::content_root().join("inbox");
        let base = crate::import::sanitize_name(name_hint).unwrap_or_else(|| "media-slide".into());
        let first = inbox.join(&base);
        if !first.exists() {
            return first;
        }
        (2..)
            .map(|n| inbox.join(format!("{base}-{n}")))
            .find(|p| !p.exists())
            .unwrap()
    }

    /// Explode a PDF into per-page media slides (W11 "explode + reuse"): each
    /// page rasterizes to a PNG via a SUBPROCESS (ADR-009 — never a crate PDF
    /// parser), imports as an ordinary image artifact, and mints its own
    /// media slide. One tap → a deck-ready run of page slides, in order.
    pub fn make_slides_from_pdf(&self, pdf: &Path) -> Result<Vec<PathBuf>, String> {
        let stem =
            crate::import::sanitize_name(pdf.file_stem().and_then(|s| s.to_str()).unwrap_or("pdf"))
                .unwrap_or_else(|| "pdf".into());
        // Rasterize into a hidden staging dir; import copies each page into
        // the inbox proper, so the stage is disposable.
        let stage = Self::content_root()
            .join("inbox")
            .join(format!(".pdf-stage-{stem}"));
        let pages = crate::import::rasterize_pdf(pdf, &stage, &stem)?;
        let mut dirs = Vec::with_capacity(pages.len());
        for page in &pages {
            let rec = crate::import::import_file(page).map_err(|e| format!("import page: {e}"))?;
            dirs.push(Self::mint_media_slide(&rec)?);
        }
        let _ = std::fs::remove_dir_all(&stage);
        self.signal(dpl_fabric::CaptureEvent::InboxChanged);
        Ok(dirs)
    }

    /// ONE entry for the harness `--make-slide <rel>` seam and any caller
    /// holding just a content-relative path: a `.pdf` explodes to page
    /// slides; an image/video mints a single media slide; anything else is an
    /// honest error. Returns every slide dir minted (page order for a PDF).
    pub fn make_slide_from_rel(&self, rel: &str) -> Result<Vec<PathBuf>, String> {
        let path = Path::new(rel);
        if rel.to_ascii_lowercase().ends_with(".pdf") {
            return self.make_slides_from_pdf(&Self::resolve(rel));
        }
        let kind = crate::import::media_kind(path)
            .filter(|k| *k == "image" || *k == "video")
            .ok_or_else(|| format!("not an image, video, or pdf: {rel}"))?;
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("media")
            .to_string();
        let rec = Record {
            rel_path: rel.to_string(),
            kind: kind.to_string(),
            name,
            ..Record::default()
        };
        Ok(vec![self.make_slide_from_library(&rec)?])
    }

    /// Run a registered workflow end-to-end (the Workflows tab's Run chip
    /// AND the `--run-workflow` harness seam — ONE entry, the
    /// `make_slide_from_rel` law). Synchronous: the skeleton's stages are
    /// milliseconds of minting; generation NEVER spends here (ADR-011 —
    /// the honest skip). Every stage publishes a jobs fact; artifacts land
    /// under a no-clobber `inbox/workflow-<id>/` run folder; completion
    /// signals ONE `InboxChanged` (the batching law). Returns the verdict
    /// line, or an error naming the registered workflows.
    pub fn run_workflow(&mut self, id: &str) -> Result<String, String> {
        let Some(doc) = dpl_workflows::find(id) else {
            let all = dpl_workflows::workflows()
                .iter()
                .map(|w| w.id.clone())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!("unknown workflow {id:?} — registered: {all}"));
        };
        let inbox = Self::content_root().join("inbox");
        let dir = dpl_workflows::fresh_dir(&inbox, &format!("workflow-{id}"));
        let rel = format!(
            "inbox/{}",
            dir.file_name().and_then(|n| n.to_str()).unwrap_or("run")
        );
        let mut runner = WfRunner {
            dir: dir.clone(),
            rel,
            assembled: 0,
        };
        // Jobs domain (doctrine law 7): one fact per stage TRANSITION,
        // labeled `workflow <id> · <stage>`; a skip is a Done that says so.
        let fabric = self.fabric.clone();
        let mut pending: std::collections::HashMap<String, dpl_fabric::JobId> =
            std::collections::HashMap::new();
        let (report, board) = dpl_workflows::execute(&doc, &mut runner, &mut |ev| {
            let f = &fabric;
            match ev {
                dpl_workflows::StageEvent::Started(stage) => {
                    let job = f.mint_job();
                    f.publish_job(dpl_fabric::JobEvent::Queued {
                        job,
                        label: format!("workflow {id} · {}", stage.id),
                    });
                    pending.insert(stage.id.clone(), job);
                }
                dpl_workflows::StageEvent::Finished(stage, verdict) => {
                    let Some(job) = pending.remove(&stage.id) else {
                        return;
                    };
                    f.publish_job(match verdict {
                        dpl_workflows::StageVerdict::Done { note } => dpl_fabric::JobEvent::Done {
                            job,
                            out: note.clone(),
                        },
                        dpl_workflows::StageVerdict::Skipped { reason } => {
                            dpl_fabric::JobEvent::Done {
                                job,
                                out: format!("skipped — {reason}"),
                            }
                        }
                        dpl_workflows::StageVerdict::Failed { error } => {
                            dpl_fabric::JobEvent::Failed {
                                job,
                                error: error.clone(),
                            }
                        }
                    });
                }
            }
        })?;
        // The minted board is a DOCUMENT — crash-safe in the run folder so
        // chat/coach/seam read the same truth the panel paints.
        dpl_workflows::save_board(&dir, &board).map_err(|e| format!("save storyboard: {e}"))?;
        let verdict = report.verdict_line();
        let assembled = runner.assembled;
        self.status = verdict.clone();
        self.wf_run = Some(WfRun {
            workflow: id.to_string(),
            report,
            board,
            dir: dir.clone(),
        });
        self.signal(dpl_fabric::CaptureEvent::InboxChanged);
        // The render leg: an assembled comp opens in the Editor and rides
        // the EXISTING stitch job — one render path, never a second.
        if assembled > 0 {
            self.open_comp(dir.join("comp"));
            self.start_render();
        }
        Ok(verdict)
    }

    /// Wire the signal fabric (called by the hub's own wire_fabric —
    /// ONE fabric end to end). Also opens this editor's capture ears.
    pub fn wire_fabric(&mut self, fabric: &dpl_fabric::Fabric) {
        self.capture_rx = fabric.subscribe_capture();
        self.fabric = fabric.clone();
    }

    /// Publish a capture signal (the default private fabric publishes into
    /// silence — drills and tests never panic).
    fn signal(&self, ev: dpl_fabric::CaptureEvent) {
        self.fabric.publish_capture(ev);
    }

    /// Saved for an indexed record — the emit half of
    /// "index + signal identically".
    fn signal_saved(fabric: &dpl_fabric::Fabric, rec: &Record) {
        fabric.publish_capture(dpl_fabric::CaptureEvent::Saved {
            rel_path: rec.rel_path.clone(),
            kind: rec.kind.clone(),
            name: rec.name.clone(),
            location: rec.location.clone(),
            source: rec.source.clone(),
        });
    }

    /// Drain capture signals (W8): any artifact change — ours or another
    /// surface's — invalidates the project caches so FOOTAGE/renders/
    /// pickers reload lazily. Cheap: caches refill on next need.
    fn drain_signals(&mut self) {
        let mut dirty = false;
        while let Ok(stamped) = self.capture_rx.try_recv() {
            dirty |= matches!(
                stamped.ev,
                dpl_fabric::CaptureEvent::Saved { .. } | dpl_fabric::CaptureEvent::InboxChanged
            );
        }
        if dirty {
            self.palette = None;
            self.renders = None;
            self.comps = None;
            self.elements = None;
            self.guides = None;
            self.session_rows = None;
        }
    }

    /// The harness seam (`--import-drill <dir>`): import every media
    /// file in `dir` through the SAME path an Explorer drop takes —
    /// the real-exe proof of the import seam.
    pub fn import_dir(&mut self, dir: &Path) {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        files.sort();
        for f in files {
            self.handle_import(&f);
        }
        self.palette = None;
        self.elements = None;
    }

    /// Route one imported file: videos AND images join the timeline
    /// (W7), elements file into the elements shelf — honestly.
    fn handle_import(&mut self, src: &Path) {
        // REFERENCE in-place (symlink) when the operator has that mode on —
        // their paid AE presets / image folders are used where they live, not
        // copied into the app. Copy stays the default (safe, self-contained).
        let outcome = if self.prefer_reference {
            import::import_reference(src)
        } else {
            import::import_file(src)
        };
        match &outcome {
            Ok(rec) => Self::signal_saved(&self.fabric, rec),
            Err(e) => self.signal(dpl_fabric::CaptureEvent::Failed { error: e.clone() }),
        }
        match outcome {
            Ok(rec) if rec.kind == "video" || rec.kind == "image" => self.add_clip(&rec),
            Ok(rec) if rec.kind == "lottie" => self.add_lottie(&rec),
            Ok(rec) if rec.kind == "element" => {
                let mogrt = src
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("mogrt"));
                self.status = if mogrt {
                    // R2a.2: the template was actually READ, say so.
                    format!(
                        "{} on your elements shelf — preview and tags read from the template.",
                        rec.name
                    )
                } else {
                    format!(
                        "{} filed into your elements — parsing lands with the .aep pass.",
                        rec.name
                    )
                };
            }
            Ok(rec) if rec.kind == "audio" => {
                self.status = format!(
                    "{} in your library — timeline audio lands with the Narrate pass.",
                    rec.name
                );
            }
            Ok(rec) => self.status = format!("Added {} to your library.", rec.name),
            Err(e) => self.status = format!("Import failed: {e}"),
        }
    }

    /// The guided workflow's NEXT-step suggestion, from real state.
    fn next_hint(&self, rendering: bool, done: bool) -> &'static str {
        if self.layers().is_empty() {
            "NEXT: add footage — tap a clip, Browse… a folder, or drop files from Explorer"
        } else if self.dir.is_none() {
            "NEXT: name it (optional) and Save — comp.json lands in your inbox"
        } else if rendering {
            "Rendering — the pill here updates when it finishes"
        } else if done {
            "NEXT: file the render into a show, or keep editing"
        } else {
            "NEXT: Render stitches your layers into one video"
        }
    }

    /// File the finished render into a show/episode (the tag seam) —
    /// `Index::keep` moves the artifact and its index row together.
    fn keep_render(&mut self, show: &str, ep: &str) {
        let path = match self.render_job.and_then(|j| self.fabric.job(j)) {
            Some(row) => match &row.state {
                dpl_fabric::JobState::Done { out } => PathBuf::from(out),
                _ => return,
            },
            None => return,
        };
        let root = Self::content_root();
        let rel = Self::short(&path);
        self.status =
            match dpl_index::Index::open_default().and_then(|ix| ix.keep(&root, &rel, show, ep)) {
                Ok(new_rel) => {
                    if let Some(job) = self.render_job.take() {
                        self.fabric.ack_job(job);
                    }
                    self.renders = None;
                    self.signal(dpl_fabric::CaptureEvent::InboxChanged);
                    format!("Filed into {show}/{ep} → {new_rel}")
                }
                Err(e) => {
                    self.signal(dpl_fabric::CaptureEvent::Failed { error: e.clone() });
                    format!("Keep failed: {e}")
                }
            };
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
        // L3: the FX section — overlays composite over the cuts above.
        if !self.fx_layers().is_empty() {
            out.push(Row::FxHeader);
            for (i, l) in self.fx_layers().iter().enumerate() {
                out.push(Row::Fx(i));
                if self.expanded.contains(&l.id) {
                    out.push(Row::FxProp(i, "Start"));
                    out.push(Row::FxProp(i, "Trim in"));
                    out.push(Row::FxProp(i, "Trim out"));
                }
            }
        }
        out
    }

    /// The Composer surface — what the WiredFRONT hub calls for its subroute.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.drain_signals();
        self.maybe_restore();
        self.ensure_project();
        self.tick_playback(ui.ctx());
        self.show_browsers(ui.ctx());
        self.drain_media_feeds(ui.ctx());
        // The ingest pill: progress while Running (the board row's counter),
        // one-shot status + cache reset when the row resolves.
        let finished = match self.ingest_job.clone() {
            Some((job, pack)) => match self.fabric.job(job) {
                Some(row) if row.running() => {
                    self.status = format!(
                        "Ingesting {pack} — {} file(s) processed…",
                        row.progress.load(Ordering::Relaxed)
                    );
                    ui.ctx().request_repaint_after(Duration::from_millis(300));
                    false
                }
                Some(row) => {
                    let done = match &row.state {
                        dpl_fabric::JobState::Done { out } => {
                            self.status =
                                format!("Ingested {pack} — {out} now local in your library.");
                            true
                        }
                        _ => {
                            if let dpl_fabric::JobState::Failed { error } = &row.state {
                                self.status = format!("Ingest failed: {error}");
                            }
                            false
                        }
                    };
                    self.fabric.ack_job(job);
                    self.ingest_job = None;
                    done
                }
                None => {
                    self.ingest_job = None;
                    false
                }
            },
            None => false,
        };
        if finished {
            self.palette = None;
            self.elements = None;
            self.packs = None;
        }
        // A render in flight keeps the frame alive so its status pill updates.
        if self
            .render_job
            .and_then(|j| self.fabric.job(j))
            .is_some_and(|r| r.running())
        {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        let mut act: Option<Act> = None;

        // The editor keyboard map — Editor tab only, and NEVER while a
        // text field (name, search, caption) owns the keys.
        if self.tab == Tab::Editor && !ui.ctx().egui_wants_keyboard_input() {
            let mods = ui.input(|i| i.modifiers);
            let pressed: Vec<egui::Key> = ui.input(|i| {
                [
                    egui::Key::Space,
                    egui::Key::Delete,
                    egui::Key::Backspace,
                    egui::Key::Home,
                    egui::Key::End,
                    egui::Key::ArrowLeft,
                    egui::Key::ArrowRight,
                    egui::Key::Z,
                    egui::Key::Y,
                ]
                .into_iter()
                .filter(|k| i.key_pressed(*k))
                .collect()
            });
            for k in pressed {
                if let Some(a) = key_act(k, mods, self.selected.as_deref()) {
                    act = Some(a);
                }
            }
        }

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
        self.tab_strip(&mut mk(tabbar_rect), &mut act);
        match self.tab {
            Tab::Editor => self.ae_screen(ui, body, &mut act),
            Tab::Annotate => self.annotate_tab(&mut mk(body), &mut act),
            Tab::Guide => self.guide_tab(&mut mk(body)),
            Tab::Workflows => self.workflows_tab(&mut mk(body), &mut act),
            other => tab_stub(&mut mk(body), other),
        }

        match act {
            Some(Act::New) => self.new_comp(),
            Some(Act::Save) => self.save(),
            Some(Act::Render) => self.start_render(),
            Some(Act::RunWorkflow(id)) => {
                // The Err (unknown id) cannot happen from the census-read
                // panel, but stays honest in the status line if it ever does.
                if let Err(e) = self.run_workflow(&id) {
                    self.status = format!("Workflow refused: {e}");
                }
            }
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
                    self.add_clip(&rec);
                    self.proj_sel = Some(i);
                }
            }
            Some(Act::AddOverlay(i)) => {
                if let Some(rec) = self.palette.as_ref().and_then(|p| p.get(i)).cloned() {
                    self.add_overlay(&rec);
                    self.proj_sel = Some(i);
                }
            }
            Some(Act::PaletteTag(tag)) => {
                self.palette_tag = tag;
                self.proj_sel = None;
            }
            Some(Act::OverlayEdit {
                id,
                x,
                y,
                scale,
                tint,
                fresh,
            }) => {
                if fresh {
                    self.checkpoint();
                }
                if let Some(l) = self.layer_mut(&id) {
                    l.x = x;
                    l.y = y;
                    l.scale = scale.max(0.05);
                    l.tint = tint;
                }
                self.mark_edit();
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
            Some(Act::ToggleReference) => {
                self.prefer_reference = !self.prefer_reference;
                reference_pref_save(self.prefer_reference);
                self.status = if self.prefer_reference {
                    "reference mode ON — pulled files are symlinked in place, not copied".into()
                } else {
                    "reference mode off — pulled files are copied into the app".into()
                };
            }
            Some(Act::Zoom(f)) => self.px_per_sec = (self.px_per_sec * f).clamp(20.0, 240.0),
            Some(Act::Undo) => self.undo_now(),
            Some(Act::Redo) => self.redo_now(),
            Some(Act::Browse) => {
                let start = std::env::var("USERPROFILE")
                    .map(|p| Path::new(&p).join("Videos"))
                    .ok()
                    .filter(|p| p.is_dir())
                    .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
                    .unwrap_or_else(|| PathBuf::from("C:\\"));
                self.open_browser(start);
            }
            Some(Act::BrowseAt(dir)) => self.open_browser(dir),
            Some(Act::Keep(show, ep)) => self.keep_render(&show, &ep),
            None => {}
        }
        self.maybe_autosave(ui.ctx());
    }

    /// The tab strip — tailored siblings; active glows voltage.
    fn tab_strip(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
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

    /// The WORKFLOWS tab (WF1): registered pipelines (census-read, never
    /// hardcoded), a Run chip each, and the last run's honest story —
    /// per-stage verdicts plus the slotted storyboard, every slot a
    /// semantic node so the agent reads exactly what the operator sees.
    fn workflows_tab(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for wf in dpl_workflows::workflows() {
                cl::clay().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&wf.label).heading());
                        ui.label(
                            RichText::new(match wf.surface {
                                dpl_workflows::Surface::Wiredfront => "wiredFRONT · raw media",
                                dpl_workflows::Surface::Workbench => "Workbench · slides",
                            })
                            .color(cl::ink::SUBTLE),
                        );
                        let run = cl::tip(
                            chip(ui, Tone::Lime, "Run", ("wf-run", &wf.id)),
                            "Run this pipeline now — every step reports honestly below. \
                             Nothing is ever generated or spent without a key you provisioned.",
                        );
                        ui.pilot_track(
                            &run,
                            NodeMetadata::new(
                                PilotId::new("composer.workflows.run")
                                    .child(&PilotKey::new(&wf.id)),
                                Role::Button,
                                format!("Run {}", wf.label),
                            )
                            .value("ready"),
                        );
                        if run.clicked() {
                            *act = Some(Act::RunWorkflow(wf.id.clone()));
                        }
                    });
                    ui.label(RichText::new(&wf.blurb).color(cl::ink::SUBTLE));
                    ui.label(
                        RichText::new(
                            wf.stages
                                .iter()
                                .map(|s| s.id.as_str())
                                .collect::<Vec<_>>()
                                .join(" → "),
                        )
                        .color(cl::ink::SUBTLE)
                        .small(),
                    );
                });
                ui.add_space(cl::space::S2);
            }

            let Some(run) = &self.wf_run else {
                plate(
                    ui,
                    "No run yet",
                    "Press Run on a pipeline — its steps and storyboard land here.",
                );
                return;
            };

            ui.add_space(cl::space::S2);
            cl::clay().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Last run — {}", run.workflow)).heading());
                    // Where the run landed — the operator can find it in Explorer.
                    ui.label(
                        RichText::new(run.dir.display().to_string())
                            .small()
                            .color(cl::ink::SUBTLE),
                    );
                });
                for row in &run.report.stages {
                    use dpl_workflows::StageVerdict as V;
                    let (tone, word, detail) = match &row.verdict {
                        V::Done { note } => (Tone::Lime, "done", note.as_str()),
                        V::Skipped { reason } => (Tone::Amber, "skipped", reason.as_str()),
                        V::Failed { error } => (Tone::Coral, "FAILED", error.as_str()),
                    };
                    ui.horizontal(|ui| {
                        let badge = chip(ui, tone, word, ("wf-stage", &run.workflow, &row.stage));
                        ui.pilot_track(
                            &badge,
                            NodeMetadata::new(
                                PilotId::new("composer.workflows.stage")
                                    .child(&PilotKey::new(&row.stage)),
                                Role::Label,
                                row.stage.clone(),
                            )
                            .value(word),
                        );
                        ui.label(RichText::new(&row.stage).strong());
                        ui.label(RichText::new(detail).color(cl::ink::SUBTLE));
                    });
                }
            });

            ui.add_space(cl::space::S2);
            cl::clay().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Storyboard").heading());
                    ui.label(
                        RichText::new(format!(
                            "{} of {} slots filled",
                            run.board.filled().len(),
                            run.board.slots.len()
                        ))
                        .color(cl::ink::SUBTLE),
                    );
                });
                for slot in &run.board.slots {
                    use dpl_workflows::SlotFill as F;
                    let (tone, state) = match &slot.fill {
                        F::Empty => (Tone::Amber, "empty"),
                        F::Slide { .. } => (Tone::Lime, "slide"),
                        F::Artifact { .. } => (Tone::Cyan, "artifact"),
                        F::Draft { .. } => (Tone::Violet, "draft"),
                    };
                    ui.horizontal(|ui| {
                        let badge = chip(ui, tone, state, ("wf-slot", &run.workflow, &slot.id));
                        let resp = cl::tip(
                            badge,
                            "A storyboard slot — its brief says what it needs; filling one \
                             is one gesture, from the library or a fresh creation.",
                        );
                        let mut meta = NodeMetadata::new(
                            PilotId::new("wiredfront.storyboard.slot")
                                .child(&PilotKey::new(&slot.id)),
                            Role::ListItem,
                            format!("{} ({})", slot.id, slot.role),
                        )
                        .value(state);
                        meta.description = Some(slot.brief.clone());
                        ui.pilot_track(&resp, meta);
                        ui.label(RichText::new(&slot.id).strong());
                        ui.label(RichText::new(&slot.brief).color(cl::ink::SUBTLE));
                    });
                }
            });
        });
    }

    /// The AE screen: tool strip · project | viewer | info+fx · timeline ·
    /// status row. Manual rects + `new_child` (the house idiom).
    fn ae_screen(&mut self, ui: &mut egui::Ui, body: Rect, act: &mut Option<Act>) {
        let pad = cl::space::S2;
        let tools_h = 40.0_f32;
        let status_h = 58.0_f32;
        // The timeline grows with twirled rows, within honest bounds.
        let rows_h: f32 = self.rows().iter().map(Row::height).sum();
        let timeline_h = fit(RULER_H + rows_h + 40.0, 150.0, body.height() * 0.42);
        // Fit-to-available (RR2): the side panels shrink on a narrow body but
        // never below a usable min, so the center viewer keeps real width
        // instead of being starved into text overlap. Preferred widths hold
        // once the body is wide enough (~≥1280).
        let project_w = fit(250.0, 160.0, body.width() * 0.26);
        let right_w = fit(232.0, 150.0, body.width() * 0.24);

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
        // The right stack: Info (when open) above Effects & Presets. A
        // selected OVERLAY grows Info for the L3 inspector (size/nudge/
        // tint/opacity) — the sections must never paint over each other.
        let (info_rect, fx_rect) = if self.info_open {
            let overlay_selected = self
                .selected
                .as_deref()
                .is_some_and(|id| self.fx_layers().iter().any(|l| l.id == id));
            let info_h = if overlay_selected {
                310.0_f32.min(right_rect.height() * 0.66)
            } else {
                120.0_f32.min(right_rect.height() * 0.4)
            };
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
        // Explorer drag-drop affordance: hovering files lights the whole
        // editor as the drop target; the drop itself drains next frame.
        if ui.ctx().input(|i| !i.raw.hovered_files.is_empty()) {
            ui.painter()
                .rect_filled(body, CornerRadius::same(cl::RADIUS), Tone::Magenta.wash());
            ui.painter().text(
                body.center(),
                egui::Align2::CENTER_CENTER,
                "Drop to import into your library",
                egui::FontId::proportional(22.0),
                cl::ink::STRONG,
            );
        }
    }

    /// The AE tool strip: the operator's set + the later-phase affordances,
    /// every one honestly blurbed.
    fn tool_strip(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        // Scroll, don't overflow (RR2): the strip stays one 40px line and
        // scrolls horizontally when the tools exceed a narrow width, instead
        // of spilling under the inspector. Hides nothing.
        egui::ScrollArea::horizontal()
            .auto_shrink([false, true])
            .show(ui, |ui| {
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
                            .value(if active {
                                "active"
                            } else {
                                "available"
                            }),
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
            });
    }

    /// The PROJECT panel: preview well + COMPS / FOOTAGE / ELEMENTS /
    /// RENDERS sections. Footage taps add videos to the timeline.
    fn project_panel(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("PROJECT").small().strong());
            let resp = cl::tip(
                small_btn(ui, "Browse…", "browse"),
                "Open a separate window onto one of your folders — click a file there \
                 to pull it in. You can also drop files here straight from Explorer.",
            );
            ui.pilot_track(
                &resp,
                NodeMetadata::new(
                    PilotId::new("composer.project").child(&PilotKey::new("browse")),
                    Role::Button,
                    "Browse folders",
                ),
            );
            if resp.clicked() {
                *act = Some(Act::Browse);
            }
            let (rlabel, rtip) = if self.prefer_reference {
                (
                    "⌁ in place",
                    "Reference mode ON — pulled files are SYMLINKED where they live, so your \
                     paid AE presets / image folders are used without copying them into the app. \
                     Tap to switch back to copy.",
                )
            } else {
                (
                    "⧉ copy",
                    "Copy mode — pulled files are copied in. Tap to REFERENCE in place (symlink) \
                     instead: use your AE presets / folders directly, no duplication.",
                )
            };
            if cl::tip(small_btn(ui, rlabel, "ref-toggle"), rtip).clicked() {
                *act = Some(Act::ToggleReference);
            }
        });
        // The preview well appears WITH a selection — an empty well is
        // dead space the pack shelf needs more.
        let root = Self::content_root();
        let sel_rec = self
            .proj_sel
            .and_then(|i| self.palette.as_ref().and_then(|p| p.get(i)));
        if let Some(r) = sel_rec {
            let w = ui.available_width();
            let (well, _) = ui.allocate_exact_size(egui::vec2(w, w * 9.0 / 16.0), Sense::hover());
            ui.painter()
                .rect_filled(well, cl::RADIUS_SM, cl::surface::WELL);
            match media_thumb_uri(r, &root) {
                Some(uri) => {
                    egui::Image::new(uri)
                        .corner_radius(cl::RADIUS_SM)
                        .paint_at(ui, well);
                }
                None => painter_label(ui, well, &r.kind),
            }
            ui.label(
                RichText::new(format!("{} · {} · {}", r.name, r.kind, fmt_secs(r.seconds)))
                    .small()
                    .color(cl::ink::SUBTLE),
            );
        }
        ui.add_space(cl::space::S1);
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
                section_header(ui, "ELEMENTS — your packs");
                let shelf = self.packs.as_deref().unwrap_or(&[]);
                if shelf.is_empty() {
                    empty_line(
                        ui,
                        "none pinned — open Browse…, go to a pack folder, tap Pin pack",
                    );
                }
                for p in shelf {
                    let count = if !p.online {
                        "offline".to_string()
                    } else if p.ingested {
                        "local".to_string()
                    } else if p.files >= 1000 {
                        "1000+".to_string()
                    } else {
                        format!("{}", p.files)
                    };
                    let resp = cl::tip(
                        row_line(ui, "pak", &p.name, &count, ("pk", p.name.as_str())),
                        if p.online {
                            "Open this pack in its own window — the files stay in place; \
                             using one copies just that file"
                        } else {
                            "This pack's drive isn't connected — plug it back in and the \
                             pack returns; anything you ingested stays local"
                        },
                    );
                    ui.pilot_track(
                        &resp,
                        NodeMetadata::new(
                            PilotId::new("composer.project.pack").child(&PilotKey::new(&p.name)),
                            Role::Button,
                            &p.name,
                        )
                        .value(if p.online { "online" } else { "offline" }),
                    );
                    if resp.clicked() && p.online {
                        *act = Some(Act::BrowseAt(p.path.clone()));
                    }
                }
                if let Some(n) = self.elements.filter(|n| *n > 0) {
                    empty_line(ui, &format!("{n} imported element file(s) in your library"));
                }
                section_header(ui, "FOOTAGE");
                let footage = self.palette.as_deref().unwrap_or(&[]);
                if footage.is_empty() {
                    empty_line(ui, "no clips or images yet — capture or import some");
                }
                // L3: tag chips — the enricher's tags made tappable. Top
                // tags by count, in memory, `dir:*` noise excluded.
                let mut tag_counts: Vec<(&str, usize)> = Vec::new();
                for rec in footage {
                    for t in &rec.tags {
                        if t.starts_with("dir:") || t == "rtfx" {
                            continue;
                        }
                        match tag_counts.iter_mut().find(|(k, _)| k == t) {
                            Some((_, n)) => *n += 1,
                            None => tag_counts.push((t, 1)),
                        }
                    }
                }
                if !tag_counts.is_empty() {
                    tag_counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
                    ui.horizontal_wrapped(|ui| {
                        let all = self.palette_tag.is_none();
                        let resp = cl::tip(
                            tag_chip(ui, "all", all, "tagchip-all"),
                            "Show every clip and image",
                        );
                        if resp.clicked() && !all {
                            *act = Some(Act::PaletteTag(None));
                        }
                        for (tag, n) in tag_counts.iter().take(8) {
                            let active = self.palette_tag.as_deref() == Some(*tag);
                            let resp = cl::tip(
                                tag_chip(ui, &format!("{tag} {n}"), active, tag),
                                "Show only footage carrying this tag",
                            );
                            if resp.clicked() {
                                *act = Some(Act::PaletteTag(if active {
                                    None
                                } else {
                                    Some(tag.to_string())
                                }));
                            }
                        }
                    });
                }
                let wanted_tag = self.palette_tag.clone();
                for (i, rec) in footage.iter().enumerate().filter(|(_, r)| {
                    wanted_tag
                        .as_deref()
                        .is_none_or(|t| r.tags.iter().any(|x| x == t))
                }) {
                    let glyph = if rec.kind == "video" { "vid" } else { "img" };
                    // Trailing spaces shift the duration clear of the fx
                    // corner painted below (right-anchored text).
                    let trail = format!("{}     ", fmt_secs(rec.seconds));
                    let resp = cl::tip(
                        row_line(ui, glyph, &rec.name, &trail, ("ft", i)),
                        if rec.kind == "video" {
                            "Add this clip to the timeline"
                        } else {
                            "Add this image to the timeline (held 5s, trim to taste)"
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
                    // L3: the fx corner — overlay this ON TOP at the playhead
                    // instead of cutting it onto the end.
                    let fxr = Rect::from_min_max(
                        egui::pos2(resp.rect.max.x - 26.0, resp.rect.min.y),
                        resp.rect.max,
                    );
                    let fx_resp = cl::tip(
                        ui.interact(fxr, egui::Id::new(("ft-fx", i)), Sense::click()),
                        "Overlay this ON TOP at the playhead — alpha composites over the cuts",
                    );
                    ui.painter().text(
                        fxr.center(),
                        egui::Align2::CENTER_CENTER,
                        "fx",
                        egui::FontId::monospace(9.0),
                        if fx_resp.hovered() {
                            Tone::Violet.color()
                        } else {
                            cl::ink::SUBTLE
                        },
                    );
                    if fx_resp.clicked() {
                        *act = Some(Act::AddOverlay(i));
                    } else if resp.clicked() && !fx_resp.hovered() {
                        *act = Some(Act::Add(i));
                    }
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
        if let Some((_, lp)) = &self.preview_lottie {
            match lp.id() {
                Some(tex) => {
                    // Native Lottie: fit its own aspect (letterbox), full UV
                    // (straight alpha composites over the WELL).
                    let lr = fit_aspect(frame, lp.aspect());
                    ui.painter().image(tex, lr, FULL_UV, Color32::WHITE);
                    if self.playing {
                        ui.ctx().request_repaint_after(Duration::from_millis(16));
                    }
                }
                None => painter_label(ui, frame, "Rendering the Lottie…"),
            }
        } else if let Some((_, uri)) = &self.preview_still {
            egui::Image::new(uri.as_str())
                .corner_radius(cl::RADIUS_SM)
                .paint_at(ui, frame);
        } else {
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
        }
        // L3: FX overlays composite over whatever the base showed.
        self.paint_overlays(ui, frame);
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

    /// INFO (mock: default shown, closable): the selected layer's facts,
    /// plus the L3 OVERLAY inspector for an fx selection. Scroll-and-clip
    /// (RR2): the pane must never paint over Effects & Presets below.
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
        egui::ScrollArea::vertical()
            .id_salt("composer-info-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| self.info_body(ui, act));
    }

    /// The Info pane's scrollable body (facts + the overlay inspector).
    fn info_body(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        let on_fx = self
            .selected
            .as_deref()
            .is_some_and(|id| self.fx_layers().iter().any(|l| l.id == id));
        let Some(layer) = self.selected.as_deref().and_then(|id| {
            self.layers()
                .iter()
                .chain(self.fx_layers().iter())
                .find(|l| l.id == id)
        }) else {
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
        if on_fx {
            self.overlay_inspector(ui, layer, act);
        }
    }

    /// The OVERLAY inspector (L3): size, position, tint — plain-words
    /// controls for the selected FX layer. Emits `Act::OverlayEdit`; the
    /// first change of a gesture (`fresh`) takes the undo checkpoint.
    fn overlay_inspector(&self, ui: &mut egui::Ui, layer: &Layer, act: &mut Option<Act>) {
        ui.add_space(cl::space::S1);
        ui.label(RichText::new("OVERLAY").small().strong());
        let (mut x, mut y, mut scale, mut tint) = (layer.x, layer.y, layer.scale, layer.tint);
        let mut fresh = false;
        let mut changed = false;

        let rs = cl::tip(
            ui.add(
                egui::Slider::new(&mut scale, 0.1..=3.0)
                    .text("size")
                    .fixed_decimals(2),
            ),
            "How big the overlay sits — 1.0 fills the frame",
        );
        fresh |= rs.drag_started();
        changed |= rs.changed();

        ui.horizontal(|ui| {
            let rx = cl::tip(
                ui.add(egui::DragValue::new(&mut x).speed(2.0).prefix("x ")),
                "Slide the overlay left/right (pixels from center)",
            );
            let ry = cl::tip(
                ui.add(egui::DragValue::new(&mut y).speed(2.0).prefix("y ")),
                "Slide the overlay up/down (pixels from center)",
            );
            let rc = cl::tip(
                ui.label(RichText::new("center").small().color(cl::ink::SUBTLE))
                    .interact(Sense::click()),
                "Snap the overlay back to dead center",
            );
            fresh |= rx.drag_started() || ry.drag_started() || rc.clicked();
            changed |= rx.changed() || ry.changed();
            if rc.clicked() && (x, y) != (0.0, 0.0) {
                x = 0.0;
                y = 0.0;
                changed = true;
            }
        });

        // TINT: the ColdLight tone pool as tap-size swatches + none/white.
        // Content color, chosen from the system's own hues — no alien
        // picker popup. The alpha ride is the opacity slider below.
        ui.horizontal(|ui| {
            ui.label(RichText::new("tint").small().color(cl::ink::SUBTLE));
            let a = tint.map(|t| t[3]).unwrap_or(255);
            let none_resp = cl::tip(
                swatch(ui, None, tint.is_none(), "tint-none"),
                "No tint — the element's own colors",
            );
            if none_resp.clicked() && tint.is_some() {
                tint = None;
                fresh = true;
                changed = true;
            }
            for tone in [
                Tone::Cyan,
                Tone::Lime,
                Tone::Violet,
                Tone::Amber,
                Tone::Magenta,
                Tone::Coral,
            ] {
                let c = tone.color();
                let target = [c.r(), c.g(), c.b(), a];
                let active = tint.is_some_and(|t| t[..3] == target[..3]);
                let resp = cl::tip(
                    swatch(ui, Some(c), active, tone.hue_id()),
                    "Recolor the overlay with this hue",
                );
                if resp.clicked() && !active {
                    tint = Some(target);
                    fresh = true;
                    changed = true;
                }
            }
        });
        let mut opacity = tint.map(|t| t[3] as f32 / 255.0).unwrap_or(1.0);
        let ro = cl::tip(
            ui.add(
                egui::Slider::new(&mut opacity, 0.05..=1.0)
                    .text("opacity")
                    .show_value(false),
            ),
            "Fade the whole overlay (rides the tint's alpha)",
        );
        fresh |= ro.drag_started();
        if ro.changed() {
            let a = (opacity * 255.0).round() as u8;
            let base = tint.unwrap_or([255, 255, 255, 255]);
            tint = if a == 255 && base[..3] == [255, 255, 255] {
                None
            } else {
                Some([base[0], base[1], base[2], a])
            };
            changed = true;
        }
        // The census marks noCT elements as designed-untinted — advise,
        // never block (the operator outranks the pack's label).
        if tint.is_some()
            && self
                .rec_tags_for(layer.source.reference())
                .contains(&"no-tint")
        {
            ui.label(
                RichText::new("This element is designed untinted (noCT) — tint may look off.")
                    .small()
                    .color(cl::ink::MUTED),
            );
        }
        if changed {
            *act = Some(Act::OverlayEdit {
                id: layer.id.clone(),
                x,
                y,
                scale,
                tint,
                fresh,
            });
        }
    }

    /// The indexed tags for a rel path, from the loaded palette (best
    /// effort — an unindexed ref simply has none).
    fn rec_tags_for(&self, rel: &str) -> Vec<&str> {
        self.palette
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .find(|r| r.rel_path == rel)
            .map(|r| r.tags.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// EFFECTS & PRESETS: search + honest stock (the registry and the
    /// elements ingest are their own ratified passes).
    fn fx_pane(&mut self, ui: &mut egui::Ui) {
        // Scroll, don't overflow (RR2): at a narrow right column the copy
        // wraps to many lines; clip-and-scroll it within the panel instead of
        // spilling into the timeline below.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(RichText::new("EFFECTS & PRESETS").small().strong());
                let search = egui::TextEdit::singleline(&mut self.fx_search)
                    .hint_text("search effects…")
                    .desired_width(f32::INFINITY);
                cl::tip(ui.add(search), "Filter effects and presets by name");
                ui.add_space(cl::space::S1);
                ui.label(
                    RichText::new(
                        "Transitions · Color · Keying land with the effects-registry pass.",
                    )
                    .small()
                    .color(cl::ink::SUBTLE),
                );
                ui.label(
                    RichText::new(
                        "Your element presets (color · font · text · ease editable) arrive with \
                         the elements ingest.",
                    )
                    .small()
                    .color(cl::ink::SUBTLE),
                );
            });
    }

    /// The timeline: the layer OUTLINE (left) beside per-layer LANES with
    /// bars + the CTI (right). Trim/move/scrub live on the lanes.
    fn timeline_panel(&mut self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("TIMELINE").small().strong());
            ui.label(
                RichText::new(
                    "tap + for properties · drag to trim/move · Space plays · Del removes \
                     · Ctrl+Z undoes",
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
        if self.layers().is_empty() && self.fx_layers().is_empty() {
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
                Row::FxHeader => {
                    painter.text(
                        egui::pos2(r.min.x + 6.0, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        "FX — composites over the cuts",
                        egui::FontId::proportional(10.0),
                        cl::ink::MUTED,
                    );
                }
                Row::Fx(i) => {
                    let layer = &self.fx_layers()[*i];
                    let selected = self.selected.as_deref() == Some(layer.id.as_str());
                    if selected {
                        painter.rect_filled(r, CornerRadius::ZERO, Tone::Violet.wash());
                    }
                    painter.text(
                        egui::pos2(r.min.x + 6.0, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        "fx",
                        egui::FontId::monospace(10.0),
                        Tone::Violet.color(),
                    );
                    let chip_r = Rect::from_center_size(
                        egui::pos2(r.min.x + 26.0, r.center().y),
                        egui::vec2(9.0, 9.0),
                    );
                    painter.rect_filled(chip_r, CornerRadius::same(2), Tone::Violet.color());
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
                    let twirl = Rect::from_min_max(r.min, egui::pos2(r.min.x + 52.0, r.max.y));
                    let tw_resp =
                        ui.interact(twirl, egui::Id::new(("tl-fx-twirl", row_i)), Sense::click());
                    let tw_resp = cl::tip(tw_resp, "Show this overlay's timing properties");
                    if tw_resp.clicked() {
                        *act = Some(Act::ToggleExpand(layer.id.clone()));
                    }
                    let row_resp = ui.interact(
                        Rect::from_min_max(egui::pos2(r.min.x + 52.0, r.min.y), r.max),
                        egui::Id::new(("tl-fx-row", row_i)),
                        Sense::click(),
                    );
                    let row_resp = cl::tip(
                        row_resp,
                        "Select this overlay — size, position and tint live in Info",
                    );
                    ui.pilot_track(
                        &row_resp,
                        NodeMetadata::new(
                            PilotId::new("composer.outline.fx").child(&PilotKey::new(&layer.id)),
                            Role::Button,
                            layer.source.reference(),
                        ),
                    );
                    if row_resp.clicked() {
                        *act = Some(Act::Select(layer.id.clone()));
                    }
                }
                Row::FxProp(i, name) => {
                    let layer = &self.fx_layers()[*i];
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
                        Tone::Violet.color(),
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
            // Layer and Fx rows paint bars the same way — only the source
            // slice and idle tone differ (FX rides the Violet family).
            let bar_layer = match row {
                Row::Layer(i) => Some((&self.layers()[i], row_tone(i))),
                Row::Fx(i) => Some((&self.fx_layers()[i], Tone::Violet)),
                _ => None,
            };
            if let Some((layer, idle_tone)) = bar_layer {
                let selected = self.selected.as_deref() == Some(layer.id.as_str());
                let bar = Rect::from_min_max(
                    egui::pos2(to_x(layer.start), y + 3.0),
                    egui::pos2(to_x(layer.end()).min(rect.max.x), y + h - 3.0),
                );
                let tone = if selected { Tone::Magenta } else { idle_tone };
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
                let hit = Self::hit_test(&bars, p, pps);
                if !matches!(hit, Drag::Scrub) {
                    // One undo step per drag: the pre-drag comp, at grab.
                    self.checkpoint();
                }
                if let Drag::Move(id, _) | Drag::TrimIn(id) | Drag::TrimOut(id) = &hit {
                    self.selected = Some(id.clone());
                }
                self.drag = Some(hit);
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
            if matches!(
                self.drag,
                Some(Drag::TrimIn(_) | Drag::TrimOut(_) | Drag::Move(..))
            ) {
                // The edit settles on release — mid-drag never autosaves.
                self.mark_edit();
            }
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
    /// snap to a 0.1s grid. Ids are comp-unique, so the same drags serve
    /// base cuts AND FX overlays (L3).
    fn apply_drag(&mut self, drag: &Drag, pt: f32) {
        match drag {
            Drag::TrimIn(id) => {
                if let Some(l) = self.layer_mut(id) {
                    let delta = pt - l.start;
                    let new_in = fit(l.in_pt + delta, 0.0, l.out_pt - 0.25);
                    l.start = (l.start + (new_in - l.in_pt)).max(0.0);
                    l.in_pt = new_in;
                }
            }
            Drag::TrimOut(id) => {
                if let Some(l) = self.layer_mut(id) {
                    l.out_pt = (l.in_pt + (pt - l.start)).max(l.in_pt + 0.25);
                }
            }
            Drag::Move(id, grab) => {
                if let Some(l) = self.layer_mut(id) {
                    l.start = snap_tenth((pt - grab).max(0.0));
                }
            }
            Drag::Scrub => {}
        }
    }

    /// The status row: New · Save · Render + the render pill, then the
    /// saved crumb + comp facts + the Info reopen toggle.
    fn status_row(&mut self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        // Snapshot the render job's board row once for this frame.
        let row = self.render_job.and_then(|j| self.fabric.job(j));
        let (rendering, done, pill) = match row.as_ref().map(|r| &r.state) {
            None => (false, false, None),
            Some(dpl_fabric::JobState::Running) => (
                true,
                false,
                Some(("Rendering...".to_string(), cl::ink::BASE)),
            ),
            Some(dpl_fabric::JobState::Done { out }) => (
                false,
                true,
                Some((
                    format!("Rendered: {}", Self::short(&PathBuf::from(out))),
                    cl::ink::STRONG,
                )),
            ),
            Some(dpl_fabric::JobState::Failed { error }) => (
                false,
                false,
                Some((format!("Render failed: {error}"), cl::ink::MUTED)),
            ),
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
            let field = egui::TextEdit::singleline(&mut self.save_name)
                .hint_text("name this comp")
                .desired_width(120.0);
            cl::tip(
                ui.add(field),
                "Names the comp folder on Save (kebab-cased); blank uses comp-N",
            );
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
            if done {
                self.keep_chips(ui, act);
            }
        });
        // Line 2: the guided NEXT step left, the comp facts right —
        // never fighting line 1's actions for width.
        ui.horizontal(|ui| {
            // Right-align the comp facts FIRST so they reserve the right edge;
            // the NEXT hint then TRUNCATES into whatever's left, never
            // overrunning the facts at a narrow width (RR2).
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
                        "{} layer(s) · {:.1}s · {}×{} @{}fps · {}{}",
                        self.layers().len(),
                        self.comp.duration(),
                        self.comp.w,
                        self.comp.h,
                        self.comp.fps,
                        self.dir
                            .as_deref()
                            .map(Self::short)
                            .unwrap_or_else(|| "unsaved".into()),
                        // Honest while the autosave debounce runs.
                        if self.dirty && self.dir.is_some() {
                            " · unsaved edits"
                        } else {
                            ""
                        },
                    ))
                    .small()
                    .color(cl::ink::SUBTLE),
                );
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(self.next_hint(rendering, done))
                                .small()
                                .color(Tone::Lime.color()),
                        )
                        .truncate(),
                    );
                });
            });
        });
    }

    /// The show KEEP chips — tag the finished render into any show,
    /// one tap. No shows yet says so honestly.
    fn keep_chips(&self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        let episodes = self.episodes.as_deref().unwrap_or(&[]);
        if episodes.is_empty() {
            ui.label(
                RichText::new("no shows yet — create one in Workbench to file this")
                    .small()
                    .color(cl::ink::SUBTLE),
            );
            return;
        }
        for (show, ep) in episodes.iter().take(4) {
            let label = format!("→ {show}/{ep}");
            let resp = cl::tip(
                small_btn(ui, &label, ("keep", label.as_str())),
                "File the finished render into this show",
            );
            ui.pilot_track(
                &resp,
                NodeMetadata::new(
                    PilotId::new("composer.keep").child(&PilotKey::new(&label)),
                    Role::Button,
                    &label,
                ),
            );
            if resp.clicked() {
                *act = Some(Act::Keep(show.clone(), ep.clone()));
            }
        }
    }

    /// GUIDE — the real .guide flow (W5): pick a step guide (+ optional
    /// session replay) · review & tune (prune/reorder/caption through
    /// the ONE edit engine) · name it · export the standalone folder
    /// (guide.html + light replay + guide-video.mp4) · file it.
    fn guide_tab(&mut self, ui: &mut egui::Ui) {
        let row = self.export_job.and_then(|j| self.fabric.job(j));
        let (exporting, pill, done_dir) = match row.as_ref().map(|r| &r.state) {
            None => (false, None, None),
            Some(dpl_fabric::JobState::Running) => (true, Some("Exporting…".to_string()), None),
            Some(dpl_fabric::JobState::Done { out }) => {
                let dir = PathBuf::from(out);
                (
                    false,
                    Some(format!("Exported: {}/guide.html", Self::short(&dir))),
                    Some(dir),
                )
            }
            Some(dpl_fabric::JobState::Failed { error }) => {
                (false, Some(format!("Export failed: {error}")), None)
            }
        };
        if exporting {
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        }
        let mut gact: Option<GAct> = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width(640.0);
                    ui.add_space(cl::space::S3);
                    ui.label(
                        RichText::new("Export a standalone guide")
                            .heading()
                            .color(cl::ink::STRONG),
                    );
                    ui.label(
                        RichText::new(
                            "Your recorded steps + the session replay, in one double-click \
                             file — works offline, easy to load on a phone.",
                        )
                        .small()
                        .color(cl::ink::SUBTLE),
                    );
                    ui.add_space(cl::space::S3);

                    self.guide_pick_section(ui, &mut gact);
                    ui.add_space(cl::space::S2);
                    self.guide_review_section(ui, &mut gact);
                    ui.add_space(cl::space::S2);

                    cl::clay().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("3 · NAME IT").small().strong());
                        let field = egui::TextEdit::singleline(&mut self.save_name)
                            .hint_text("guide name")
                            .desired_width(f32::INFINITY);
                        cl::tip(ui.add(field), "Names the folder everything ships under");
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("suggestions:").small().color(cl::ink::MUTED));
                            for seed in name_seeds() {
                                let resp = cl::tip(
                                    small_btn(ui, &seed, ("seed", seed.as_str())),
                                    "Use this name",
                                );
                                if resp.clicked() {
                                    self.save_name = seed.clone();
                                }
                            }
                        });
                    });
                    ui.add_space(cl::space::S2);

                    cl::clay().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("4 · EXPORT").small().strong());
                        ui.horizontal_wrapped(|ui| {
                            let resp = cl::tip(
                                small_btn(
                                    ui,
                                    if self.embed_video {
                                        "Replay: inside the file"
                                    } else {
                                        "Replay: beside the file"
                                    },
                                    "gembed",
                                ),
                                "Inside = ONE guide.html carries everything (best for \
                                 phones). Beside = a smaller page plus session.mp4 next \
                                 to it.",
                            );
                            if resp.clicked() {
                                self.embed_video = !self.embed_video;
                            }
                            let resp = cl::tip(
                                small_btn(
                                    ui,
                                    if self.make_video {
                                        "Guide video: ON"
                                    } else {
                                        "Guide video: OFF"
                                    },
                                    "gvideo",
                                ),
                                "Also render guide-video.mp4 — each still held 3 seconds, \
                                 then the replay, as one standalone video",
                            );
                            if resp.clicked() {
                                self.make_video = !self.make_video;
                            }
                            let resp = cl::tip(
                                small_btn(
                                    ui,
                                    if self.make_gif {
                                        "Social GIF: ON"
                                    } else {
                                        "Social GIF: OFF"
                                    },
                                    "ggif",
                                ),
                                "Also render replay.gif — the replay as a small                                  animated GIF you can drop anywhere; for LinkedIn                                  video posts, upload the bundled session.mp4",
                            );
                            if resp.clicked() {
                                self.make_gif = !self.make_gif;
                            }
                        });
                        ui.horizontal(|ui| {
                            if self.review.is_some() && !exporting {
                                let resp = cl::tip(
                                    chip(ui, Tone::Magenta, "Export guide", "guide-export"),
                                    "Write the standalone guide folder into your inbox",
                                );
                                ui.pilot_track(
                                    &resp,
                                    NodeMetadata::new(
                                        PilotId::new("composer.guide")
                                            .child(&PilotKey::new("export")),
                                        Role::Button,
                                        "Export guide",
                                    ),
                                );
                                if resp.clicked() {
                                    gact = Some(GAct::Export);
                                }
                            }
                            if let Some(text) = &pill {
                                ui.label(RichText::new(text).small().color(cl::ink::BASE));
                            }
                        });
                    });
                    ui.add_space(cl::space::S2);

                    cl::clay().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new("5 · FILE IT INTO A SHOW").small().strong());
                        if done_dir.is_some() {
                            let episodes = self.episodes.clone().unwrap_or_default();
                            if episodes.is_empty() {
                                ui.label(
                                    RichText::new(
                                        "no shows yet — create one in Workbench to file this",
                                    )
                                    .small()
                                    .color(cl::ink::SUBTLE),
                                );
                            }
                            ui.horizontal_wrapped(|ui| {
                                for (show, ep) in episodes.iter().take(4) {
                                    let label = format!("→ {show}/{ep}");
                                    let resp = cl::tip(
                                        small_btn(ui, &label, ("gkeepto", label.as_str())),
                                        "File the exported guide into this show",
                                    );
                                    if resp.clicked() {
                                        gact = Some(GAct::KeepTo(show.clone(), ep.clone()));
                                    }
                                }
                            });
                        } else {
                            ui.label(
                                RichText::new("Export first — the show chips appear here.")
                                    .small()
                                    .color(cl::ink::SUBTLE),
                            );
                        }
                    });
                    ui.add_space(cl::space::S3);
                    ui.label(
                        RichText::new(
                            "Double-click guide.html to use it anywhere — it works offline. \
                             Copy the folder to your phone (or just the file when the \
                             replay rides inside it).",
                        )
                        .small()
                        .color(cl::ink::MUTED),
                    );
                });
            });
        match gact {
            Some(GAct::PickGuide(rel)) => {
                self.load_review(&rel);
                if let Some(rev) = &self.review {
                    self.status = format!("Reviewing {} — {} step(s).", rev.rel, rev.steps.len());
                }
            }
            Some(GAct::PickSession(rel)) => {
                self.sel_session =
                    (self.sel_session.as_deref() != Some(rel.as_str())).then_some(rel);
            }
            Some(GAct::Move(i, d)) => {
                if let Some(rev) = self.review.as_mut() {
                    let j = i.saturating_add_signed(d);
                    if j < rev.steps.len() && i != j {
                        rev.steps.swap(i, j);
                    }
                }
            }
            Some(GAct::Apply) => {
                if self.apply_armed {
                    self.apply_review();
                } else {
                    self.apply_armed = true;
                }
            }
            Some(GAct::Export) => self.start_export(),
            Some(GAct::KeepTo(show, ep)) => self.keep_export(&show, &ep),
            None => {}
        }
    }

    /// Guide step 1 · PICK — the guides and sessions walls, tappable.
    fn guide_pick_section(&mut self, ui: &mut egui::Ui, gact: &mut Option<GAct>) {
        cl::clay().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("1 · PICK WHAT SHIPS").small().strong());
            ui.label(
                RichText::new("STEP GUIDES")
                    .small()
                    .strong()
                    .color(cl::ink::MUTED),
            );
            let picked_guide = self.review.as_ref().map(|r| r.rel.clone());
            let guides = self.guides.as_deref().unwrap_or(&[]);
            if guides.is_empty() {
                ui.label(
                    RichText::new(
                        "no step guides yet — record one with ViziWiz Steps (Ctrl+Shift+9)",
                    )
                    .small()
                    .color(cl::ink::SUBTLE),
                );
            }
            for r in guides {
                let on = picked_guide.as_deref() == Some(r.rel_path.as_str());
                let resp = cl::tip(
                    row_line(
                        ui,
                        "gde",
                        &r.name,
                        if on { "picked" } else { "" },
                        ("gg", r.rel_path.as_str()),
                    ),
                    "Review and export this step guide",
                );
                ui.pilot_track(
                    &resp,
                    NodeMetadata::new(
                        PilotId::new("composer.guide.pick").child(&PilotKey::new(&r.rel_path)),
                        Role::Button,
                        &r.name,
                    )
                    .value(if on { "picked" } else { "row" }),
                );
                if resp.clicked() {
                    *gact = Some(GAct::PickGuide(r.rel_path.clone()));
                }
            }
            ui.add_space(cl::space::S1);
            ui.label(
                RichText::new("SESSION REPLAY — optional")
                    .small()
                    .strong()
                    .color(cl::ink::MUTED),
            );
            let sessions = self.session_rows.as_deref().unwrap_or(&[]);
            if sessions.is_empty() {
                ui.label(
                    RichText::new("no sessions yet — record one from ViziWiz's Session tab")
                        .small()
                        .color(cl::ink::SUBTLE),
                );
            }
            for r in sessions {
                let on = self.sel_session.as_deref() == Some(r.rel_path.as_str());
                let resp = cl::tip(
                    row_line(
                        ui,
                        "ses",
                        &r.name,
                        if on { "picked" } else { "" },
                        ("gs", r.rel_path.as_str()),
                    ),
                    "Bundle this session's replay video and its event chapters \
                     — tap again to leave it out",
                );
                ui.pilot_track(
                    &resp,
                    NodeMetadata::new(
                        PilotId::new("composer.guide.session").child(&PilotKey::new(&r.rel_path)),
                        Role::Button,
                        &r.name,
                    )
                    .value(if on { "picked" } else { "row" }),
                );
                if resp.clicked() {
                    *gact = Some(GAct::PickSession(r.rel_path.clone()));
                }
            }
        });
    }

    /// Guide step 2 · REVIEW & TUNE — the W5 reviewer rows: thumb, keep,
    /// reorder, caption; Apply is two-tap (pruning deletes source frames).
    fn guide_review_section(&mut self, ui: &mut egui::Ui, gact: &mut Option<GAct>) {
        cl::clay().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("2 · REVIEW & TUNE").small().strong());
            let Some(rev) = self.review.as_mut() else {
                ui.label(
                    RichText::new("Pick a step guide above to prune and caption its steps.")
                        .small()
                        .color(cl::ink::SUBTLE),
                );
                return;
            };
            let dir = rev.dir.clone();
            let total = rev.steps.len();
            for (i, rs) in rev.steps.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let (well, _) = ui.allocate_exact_size(egui::vec2(64.0, 36.0), Sense::hover());
                    ui.painter()
                        .rect_filled(well, cl::RADIUS_SM, cl::surface::WELL);
                    let abs = dir.join(&rs.step.file);
                    if abs.is_file() {
                        // file:/// with THREE slashes — the media block's law.
                        let uri = format!("file:///{}", abs.to_string_lossy().replace('\\', "/"));
                        egui::Image::new(uri)
                            .corner_radius(cl::RADIUS_SM)
                            .paint_at(ui, well);
                    }
                    ui.label(
                        RichText::new(format!("{}", i + 1))
                            .monospace()
                            .small()
                            .color(cl::ink::MUTED),
                    );
                    let resp = cl::tip(
                        small_btn(ui, if rs.keep { "Keep" } else { "Skip" }, ("gk", i)),
                        if rs.keep {
                            "This step ships — tap to skip it"
                        } else {
                            "Skipped — tap to keep it (Apply deletes skipped frames \
                             from the guide)"
                        },
                    );
                    if resp.clicked() {
                        rs.keep = !rs.keep;
                    }
                    if i > 0
                        && cl::tip(small_btn(ui, "^", ("gu", i)), "Move this step earlier")
                            .clicked()
                    {
                        *gact = Some(GAct::Move(i, -1));
                    }
                    if i + 1 < total
                        && cl::tip(small_btn(ui, "v", ("gd", i)), "Move this step later").clicked()
                    {
                        *gact = Some(GAct::Move(i, 1));
                    }
                    let field = egui::TextEdit::singleline(&mut rs.caption)
                        .hint_text("caption — your words for this step")
                        .desired_width(f32::INFINITY);
                    cl::tip(
                        ui.add(field),
                        "Shown under this still in the exported guide",
                    );
                });
            }
            let kept = rev.steps.iter().filter(|s| s.keep).count();
            ui.horizontal(|ui| {
                let label = if self.apply_armed {
                    "Really apply?"
                } else {
                    "Apply edits"
                };
                let resp = cl::tip(
                    small_btn(ui, label, "gapply"),
                    if self.apply_armed {
                        "Tap again to write the edits — skipped steps and their \
                         frames are removed from this guide"
                    } else {
                        "Write prune/reorder/captions into the guide — it takes \
                         two taps, so a stray tap never deletes"
                    },
                );
                ui.pilot_track(
                    &resp,
                    NodeMetadata::new(
                        PilotId::new("composer.guide").child(&PilotKey::new("apply")),
                        Role::Button,
                        "Apply edits",
                    )
                    .value(if self.apply_armed { "armed" } else { "idle" }),
                );
                if resp.clicked() {
                    *gact = Some(GAct::Apply);
                }
                ui.label(
                    RichText::new(format!("{kept} of {total} step(s) ship"))
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
        if let Some((_, lp)) = &self.preview_lottie {
            if let Some(tex) = lp.id() {
                let lr = fit_aspect(canvas.shrink(cl::space::S3), lp.aspect());
                ui.painter().image(tex, lr, FULL_UV, Color32::WHITE);
                if self.playing {
                    ui.ctx().request_repaint_after(Duration::from_millis(16));
                }
            } else {
                painter_label(ui, canvas, "Rendering the Lottie…");
            }
        } else if let Some((_, uri)) = &self.preview_still {
            let frame = fit_aspect(canvas.shrink(cl::space::S3), 16.0 / 9.0);
            egui::Image::new(uri.as_str())
                .corner_radius(cl::RADIUS_SM)
                .paint_at(ui, frame);
        } else {
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
        // L3: FX overlays composite over the annotate frame too — what you
        // ink on is what exports.
        self.paint_overlays(ui, fit_aspect(canvas.shrink(cl::space::S3), 16.0 / 9.0));
    }
}

/// Pending edits never die with the process — the last flush rides the
/// drop (app exit; New/Open flush explicitly on their own paths).
impl Drop for Composer {
    fn drop(&mut self) {
        self.flush_save();
    }
}

/// The editor keyboard map — ONE pure path (the deck's idiom), so the
/// bindings are unit-provable. Ctrl chords first; bare keys only reach
/// here when no text field wants them.
fn key_act(key: egui::Key, mods: egui::Modifiers, selected: Option<&str>) -> Option<Act> {
    if mods.command {
        return match key {
            egui::Key::Z if mods.shift => Some(Act::Redo),
            egui::Key::Z => Some(Act::Undo),
            egui::Key::Y => Some(Act::Redo),
            _ => None,
        };
    }
    match key {
        egui::Key::Space => Some(Act::TogglePlay),
        egui::Key::Delete | egui::Key::Backspace => selected.map(|id| Act::Remove(id.to_string())),
        egui::Key::Home => Some(Act::ToStart),
        egui::Key::End => Some(Act::ToEnd),
        egui::Key::ArrowLeft => Some(Act::Step(-1.0)),
        egui::Key::ArrowRight => Some(Act::Step(1.0)),
        _ => None,
    }
}

/// The tone cycle for layer rows/bars (Magenta is the selection voice).
fn row_tone(i: usize) -> Tone {
    [Tone::Cyan, Tone::Lime, Tone::Coral][i % 3]
}

/// A poison-proof lock on one Browse window's shared state.
fn lock(b: &Mutex<browse::Browser>) -> std::sync::MutexGuard<'_, browse::Browser> {
    b.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Resolve every pending Lottie to a PNG frame-sequence (the ThorVG helper,
/// off the UI thread — this RUNS ON THE RENDER THREAD), then run the one-
/// pass comp render. Temp sequences live under the OS temp dir (NEVER the
/// comp folder — its contract reserves comp.json + assets) and are cleaned
/// best-effort after the encode.
fn render_comp_resolving(
    parts: RenderParts,
    w: u32,
    h: u32,
    fps: u32,
    out: &Path,
) -> Result<(), String> {
    use dpl_engine_media::{OverlayClip, OverlayInput};
    static SEQ_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let seq_root = std::env::temp_dir().join(format!(
        "dpl-comp-render-{}-{}",
        std::process::id(),
        SEQ_N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut overlays: Vec<OverlayClip> = Vec::with_capacity(parts.overlays.len());
    for (n, slot) in parts.overlays.into_iter().enumerate() {
        match slot {
            OverlaySlot::Ready(o) => overlays.push(o),
            OverlaySlot::Lottie(p) => {
                let meta = dpl_lottie::lottie_meta(&p.path)
                    .ok_or_else(|| format!("unreadable lottie: {}", p.path.display()))?;
                // Export at the comp's fps, aspect-fit inside the comp
                // frame (the graph's scale leg only ever shrinks further).
                let (ew, eh) = fit_box(meta.width, meta.height, w, h);
                let dir = seq_root.join(format!("seq-{n}"));
                dpl_lottie::export_sequence(&p.path, ew, eh, fps as f32, &dir)?;
                overlays.push(OverlayClip {
                    input: OverlayInput::PngSeq {
                        pattern: dir.join("frame-%05d.png"),
                        fps: fps as f32,
                        // The helper numbers from frame-00000.
                        start: 0,
                    },
                    in_pt: p.in_pt,
                    out_pt: p.out_pt,
                    at: p.at,
                    x: p.x,
                    y: p.y,
                    scale: p.scale,
                    tint: p.tint,
                    hold: false,
                });
            }
        }
    }
    let result = dpl_engine_media::render_comp(&parts.base, &overlays, w, h, fps, out);
    let _ = std::fs::remove_dir_all(&seq_root);
    result
}

/// The largest w×h box of the source's aspect that fits inside the target.
fn fit_box(src_w: u32, src_h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let (sw, sh) = (src_w.max(1) as f32, src_h.max(1) as f32);
    let s = (max_w as f32 / sw).min(max_h as f32 / sh).min(1.0);
    (
        ((sw * s).round() as u32).max(1),
        ((sh * s).round() as u32).max(1),
    )
}

/// Write-through for a finished render: poster + duration + index row
/// (kind video, source "render") — so the Gallery, the Keep seam, and
/// the agents all see it. Runs on the render thread; failures degrade
/// the card, never the render.
fn index_render(out: &Path, fabric: Option<&dpl_fabric::Fabric>) {
    let root = dpl_engine_capture::content_root();
    let mut rec = dpl_index::record_for_capture(out, &root, "video", "render");
    let poster = out.with_file_name("render-poster.jpg");
    if dpl_engine_media::write_poster(out, &poster).is_ok() {
        rec.poster_rel_path = poster
            .strip_prefix(&root)
            .ok()
            .map(|p| p.to_string_lossy().replace('\\', "/"));
    }
    rec.seconds = dpl_video::clip::clip_duration(out)
        .filter(|d| *d > 0.0)
        .map(|d| d.round() as u32);
    if let Ok(ix) = dpl_index::Index::open_default() {
        if ix.upsert(&rec).is_ok() {
            if let Some(f) = fabric {
                f.publish_capture(dpl_fabric::CaptureEvent::Saved {
                    rel_path: rec.rel_path.clone(),
                    kind: rec.kind.clone(),
                    name: rec.name.clone(),
                    location: rec.location.clone(),
                    source: rec.source.clone(),
                });
            }
        }
    }
}

/// One reviewer row → the edit engine's shape. Blank captions clear.
fn review_edit(s: &ReviewStep) -> dpl_engine_steps::edit::GuideStepEdit {
    let caption = s.caption.trim();
    dpl_engine_steps::edit::GuideStepEdit {
        order: s.step.order,
        file: s.step.file.clone(),
        result_file: s.step.result_file.clone(),
        action: s.step.action.clone(),
        click: s.step.click,
        app: s.step.app.clone(),
        title: s.step.title.clone(),
        width: s.step.width,
        height: s.step.height,
        bytes: s.step.bytes,
        caption: (!caption.is_empty()).then(|| caption.to_string()),
        // DG1: the clip identity rides the review round-trip untouched —
        // a caption edit can never drop a replay step's media.
        kind: s.step.kind.clone(),
        clip: s.step.clip.clone(),
        clip_in: s.step.clip_in,
        clip_out: s.step.clip_out,
    }
}

/// The ONE guide-export path — the Guide tab and the `--export-guide`
/// harness seam both land here: jail + resolve the rels, mint the
/// no-clobber `inbox/<name>/` folder, run `dpl_guide::export_guide`,
/// write the artifact through the index (kind "guide", source "export").
pub fn run_export(
    root: &Path,
    guide_rel: &str,
    session_rel: Option<&str>,
    out_name: Option<&str>,
    opts: dpl_guide::ExportOpts,
    fabric: Option<&dpl_fabric::Fabric>,
) -> Result<dpl_guide::Export, String> {
    let source = jailed(root, guide_rel)?;
    if !source.join("guide.json").is_file() {
        return Err(format!("{guide_rel} is not a guide folder (no guide.json)"));
    }
    let session = match session_rel {
        Some(rel) => {
            let dir = jailed(root, rel)?;
            if !dir.is_dir() {
                return Err(format!("{rel} is not a session folder"));
            }
            Some(dir)
        }
        None => None,
    };
    let base = out_name.map(str::to_string).unwrap_or_else(|| {
        let src = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "guide".into());
        format!("{src}-share")
    });
    let out_dir = mint_export_dir(&root.join("inbox"), &base);
    let export = dpl_guide::export_guide(&source, session.as_deref(), &out_dir, opts)?;
    // Write-through: the export is an ordinary indexed artifact, so the
    // Gallery, the Keep seam, and the agents all see it.
    let folder = out_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or(base);
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let rec = Record {
        rel_path: format!("inbox/{folder}"),
        location: "inbox".into(),
        kind: "guide".into(),
        name: folder,
        source: "export".into(),
        captured_at: dpl_index::iso_from_ms(ms),
        modified_ms: Some(ms),
        ..Record::default()
    };
    if let Ok(ix) = dpl_index::Index::open_default() {
        if ix.upsert(&rec).is_ok() {
            if let Some(f) = fabric {
                f.publish_capture(dpl_fabric::CaptureEvent::Saved {
                    rel_path: rec.rel_path.clone(),
                    kind: rec.kind.clone(),
                    name: rec.name.clone(),
                    location: rec.location.clone(),
                    source: rec.source.clone(),
                });
            }
        }
    }
    Ok(export)
}

/// A forward-slash rel path joined under the root — absolute paths, drive
/// letters, and `..` are refused BY NAME (the seam's jail law).
fn jailed(root: &Path, rel: &str) -> Result<PathBuf, String> {
    if rel.is_empty()
        || rel.contains("..")
        || rel.contains(':')
        || rel.starts_with('/')
        || rel.starts_with('\\')
    {
        return Err(format!("refusing a path outside the content root: {rel:?}"));
    }
    Ok(rel.split('/').fold(root.to_path_buf(), |a, s| a.join(s)))
}

/// The next free `<inbox>/<base>[-N]` folder — deterministic, no clobber.
fn mint_export_dir(inbox: &Path, base: &str) -> PathBuf {
    let first = inbox.join(base);
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| inbox.join(format!("{base}-{n}")))
        .find(|p| !p.exists())
        .unwrap()
}

/// Seeded name suggestions: the next free comp folder + a date-stamped
/// guide name (the index's own clock — no new deps).
fn name_seeds() -> Vec<String> {
    let mut out = Vec::new();
    if let Some(n) = Composer::fresh_comp_dir()
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
    {
        out.push(n);
    }
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let iso = dpl_index::iso_from_ms(ms);
    if iso.len() >= 10 {
        out.push(format!("guide-{}", &iso[..10]));
    }
    out
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

/// A small tag-filter chip (L3): treated neubrutal like `tool_btn`, the
/// active filter presses in — composed from the trait system, never
/// hand-painted chrome.
fn tag_chip(ui: &mut egui::Ui, label: &str, active: bool, key: &str) -> egui::Response {
    let tone = if active { Tone::Magenta } else { Tone::Cyan };
    let state = if active {
        cl::TraitState::Active
    } else {
        cl::TraitState::Idle
    };
    let inner = cl::treated(tone, "neubrutalist", &[], state)
        .margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(label).small());
        });
    ui.interact(
        inner.response.rect,
        egui::Id::new(("tag-chip", key)),
        Sense::click(),
    )
}

/// A tap-size tint swatch (L3 inspector): crisp square, hard edge, strong
/// ring when active/hovered. `None` paints the no-tint slash.
fn swatch(ui: &mut egui::Ui, color: Option<Color32>, active: bool, key: &str) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::hover());
    let resp = ui.interact(rect, egui::Id::new(("tint-swatch", key)), Sense::click());
    let p = ui.painter();
    let inner = rect.shrink(2.0);
    match color {
        Some(c) => {
            p.rect_filled(inner, CornerRadius::same(2), c);
        }
        None => {
            p.rect_stroke(
                inner,
                CornerRadius::same(2),
                egui::Stroke::new(1.0, cl::ink::SUBTLE),
                egui::StrokeKind::Inside,
            );
            p.line_segment(
                [inner.left_bottom(), inner.right_top()],
                egui::Stroke::new(1.0, cl::ink::SUBTLE),
            );
        }
    }
    if active || resp.hovered() {
        p.rect_stroke(
            rect,
            CornerRadius::same(2),
            egui::Stroke::new(2.0, cl::ink::STRONG),
            egui::StrokeKind::Inside,
        );
    }
    resp
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
    // Everything paints CLIPPED to the row — pack names run long.
    let painter = ui.painter_at(rect);
    let hovered = resp.hovered();
    if hovered {
        painter.rect_filled(rect, cl::RADIUS_SM, cl::surface::OVERLAY);
    }
    painter.text(
        egui::pos2(rect.min.x + 4.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        glyph,
        egui::FontId::monospace(10.0),
        Tone::Cyan.color(),
    );
    let trail_w = if trail.is_empty() { 4.0 } else { 44.0 };
    let name_clip = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + 32.0, rect.min.y),
        egui::pos2(rect.max.x - trail_w, rect.max.y),
    );
    ui.painter_at(name_clip).text(
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
    painter.text(
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
        Some("workflows") => Tab::Workflows,
        _ => Tab::Editor,
    }
}

/// An honest stub for a tailored tab not yet built — names what it will do.
fn tab_stub(ui: &mut egui::Ui, tab: Tab) {
    plate(ui, tab.label(), tab.blurb());
}

/// The app-side [`dpl_workflows::StageRunner`] (WF1): materializes stages
/// with the editor's own seams — slide docs via `dpl_doc`, the comp via
/// `dpl_comp` — under the no-clobber run folder. Fabric-free: the executor
/// events map to jobs facts in `run_workflow`.
struct WfRunner {
    /// The run folder (absolute) and its content-relative prefix.
    dir: PathBuf,
    rel: String,
    /// Layers the assemble stage produced — the render leg's honesty check.
    assembled: usize,
}

impl dpl_workflows::StageRunner for WfRunner {
    fn run_stage(
        &mut self,
        _workflow: &dpl_workflows::WorkflowDoc,
        stage: &dpl_workflows::Stage,
        board: &mut dpl_workflows::StoryboardDoc,
    ) -> dpl_workflows::StageVerdict {
        use dpl_workflows::{SlotFill, StageKind, StageVerdict};
        match &stage.kind {
            StageKind::Outline { .. } => unreachable!("the executor mints the board"),
            StageKind::SlideFromBeat { template } => {
                let mut minted = 0usize;
                for slot in &mut board.slots {
                    if !matches!(slot.fill, SlotFill::Empty) {
                        continue;
                    }
                    let slide = dpl_workflows::slide_for_slot(template, slot);
                    if let Some(i) = dpl_doc::validate(&slide).first() {
                        return StageVerdict::Failed {
                            error: format!(
                                "slide for {} invalid at {}: {}",
                                slot.id, i.path, i.message
                            ),
                        };
                    }
                    let slide_dir = self.dir.join("slides").join(&slot.id);
                    if let Err(e) = dpl_doc::save(&slide_dir, &slide) {
                        return StageVerdict::Failed {
                            error: format!("save slide {}: {e}", slot.id),
                        };
                    }
                    slot.fill = SlotFill::Slide {
                        rel: format!("{}/slides/{}", self.rel, slot.id),
                    };
                    minted += 1;
                }
                StageVerdict::Done {
                    note: format!("minted {minted} `{template}` slide(s)"),
                }
            }
            StageKind::GenerateImage => {
                // Presence CHECK only — the key VALUE is never read, logged,
                // or carried here (ADR-011 custody).
                if std::env::var_os("XAI_API_KEY").is_none() {
                    StageVerdict::Skipped {
                        reason:
                            "no key provisioned (ADR-011) — set XAI_API_KEY to enable image art"
                                .into(),
                    }
                } else {
                    StageVerdict::Skipped {
                        reason: "in-run generation lands with the async-run slice — \
                                 create in Chat's panel and save to the library meanwhile"
                            .into(),
                    }
                }
            }
            StageKind::AssembleComp { fps, w, h } => {
                let (comp, notes) = dpl_workflows::comp_for_board(board, *fps, *w, *h);
                if let Some(i) = dpl_comp::validate(&comp).first() {
                    return StageVerdict::Failed {
                        error: format!("assembled comp invalid at {}: {}", i.path, i.message),
                    };
                }
                if let Err(e) = dpl_comp::save(&self.dir.join("comp"), &comp) {
                    return StageVerdict::Failed {
                        error: format!("save comp: {e}"),
                    };
                }
                self.assembled = comp.tracks.iter().map(|t| t.layers.len()).sum();
                StageVerdict::Done {
                    note: format!(
                        "assembled {} layer(s) → {}/comp ({} slot(s) noted, not layered)",
                        self.assembled,
                        self.rel,
                        notes.len()
                    ),
                }
            }
            StageKind::Render => {
                if self.assembled == 0 {
                    StageVerdict::Skipped {
                        reason:
                            "nothing assembled to render — fill slots with media artifacts first"
                                .into(),
                    }
                } else {
                    StageVerdict::Done {
                        note: "assembled comp opens in the Editor and rides the render job".into(),
                    }
                }
            }
        }
    }
}

/// The machine-probed preview budget (L3, operator-ratified DYNAMIC): how
/// many simultaneous overlay lanes this box affords. Each clip lane is its
/// own ffmpeg pipe + reader thread; each Lottie lane is a synchronous
/// helper round-trip on the UI thread — so RAM and cores set the caps,
/// never a hardcoded laptop number. Export is NEVER capped.
struct PreviewCaps {
    clip_lanes: usize,
    lottie_lanes: usize,
}

impl PreviewCaps {
    /// Total RAM via sysinfo + logical CPUs via std (the
    /// `dpl_engine_ae::Caps::probe` pattern, replicated per the repo's
    /// small-utility philosophy — no engine-ae coupling for 12 lines).
    fn probe() -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        let ram_gb = sys.total_memory() as f32 / (1024.0 * 1024.0 * 1024.0);
        let cpus = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        Self::for_machine(ram_gb, cpus)
    }

    /// The policy, pure for tests: clip lanes = cpus/4 clamped 2..=8, and a
    /// sub-24GB box (this laptop) never exceeds 3; Lottie lanes = 2 on a
    /// 32GB+ box, else 1 (each is a BLOCKING UI-thread round-trip).
    fn for_machine(ram_gb: f32, cpus: usize) -> Self {
        let mut clip_lanes = (cpus / 4).clamp(2, 8);
        if ram_gb < 24.0 {
            clip_lanes = clip_lanes.min(3);
        }
        Self {
            clip_lanes,
            lottie_lanes: if ram_gb >= 32.0 { 2 } else { 1 },
        }
    }
}

/// One live overlay preview lane (L3): the player kind feeding an
/// overlay-track layer's pixels at the playhead.
enum OverlayLane {
    Clip(ClipPlayer),
    Lottie(LottiePlayer),
    /// A held frame: the file:/// uri (no pipe).
    Still(String),
}

/// A Lottie layer awaiting its frame-sequence export (the render thread
/// resolves it — a helper subprocess run never happens on the UI thread).
struct PendingLottie {
    path: PathBuf,
    in_pt: f32,
    out_pt: f32,
    at: f32,
    x: f32,
    y: f32,
    scale: f32,
    tint: Option<[u8; 4]>,
}

/// One overlay in stack order: ready for ffmpeg, or pending a Lottie
/// export. Order IS stacking order — the chain composites in sequence.
enum OverlaySlot {
    Ready(dpl_engine_media::OverlayClip),
    Lottie(PendingLottie),
}

/// Everything the L3 render needs from the comp, gathered PURE (no
/// subprocess): ordered base legs (black stands in for base-track Lottie
/// spans and the tail overlays hang past), overlays in stack order, and
/// the honest count of unplayable Element layers.
struct RenderParts {
    base: Vec<dpl_engine_media::CompBase>,
    overlays: Vec<OverlaySlot>,
    elements_skipped: usize,
}

/// A native Lottie layer's live preview — the `ClipPlayer` analog, backed
/// by the `dpl-lottie` subprocess helper (ThorVG). The FRAME render +
/// texture upload happen in `tick_playback` (`&mut`); the viewer only
/// reads the cached texture id + aspect (`&self`), mirroring how the video
/// player's interior mutability keeps the paint path immutable.
struct LottiePlayer {
    player: dpl_lottie::Player,
    handle: Option<egui::TextureHandle>,
    /// The last rendered time — re-render (a subprocess round-trip) only
    /// when the playhead actually moved.
    last_t: f32,
    render_w: u32,
    render_h: u32,
}

impl LottiePlayer {
    /// Open at a preview size derived from the Lottie's NATIVE aspect,
    /// capped so a scrub frame stays a cheap pipe read. None if the file
    /// isn't a readable Lottie or the helper won't start.
    fn open(path: &Path) -> Option<Self> {
        let meta = dpl_lottie::lottie_meta(path)?;
        let (render_w, render_h) = preview_size(meta.width, meta.height);
        let player = dpl_lottie::Player::open(path, render_w, render_h).ok()?;
        Some(Self {
            player,
            handle: None,
            last_t: f32::NAN,
            render_w,
            render_h,
        })
    }

    /// Ensure the frame at `t` is rendered and uploaded (a no-op when `t`
    /// is unchanged). Called from `tick_playback`.
    fn render(&mut self, ctx: &egui::Context, t: f32) {
        if self.handle.is_some() && (t - self.last_t).abs() < 1e-4 {
            return;
        }
        if let Ok(frame) = self.player.frame_at(t) {
            let img = egui::ColorImage::from_rgba_unmultiplied(
                [frame.width as usize, frame.height as usize],
                &frame.rgba,
            );
            match &mut self.handle {
                Some(h) => h.set(img, egui::TextureOptions::LINEAR),
                None => {
                    self.handle = Some(ctx.load_texture(
                        "dpl-lottie-preview",
                        img,
                        egui::TextureOptions::LINEAR,
                    ))
                }
            }
            self.last_t = t;
        }
    }

    fn id(&self) -> Option<egui::TextureId> {
        self.handle.as_ref().map(|h| h.id())
    }

    fn aspect(&self) -> f32 {
        self.render_w as f32 / self.render_h.max(1) as f32
    }
}

/// A preview render size at the Lottie's native aspect, longest edge capped
/// (a scrub frame is w*h*4 bytes over a pipe — keep it modest).
fn preview_size(w: u32, h: u32) -> (u32, u32) {
    const MAX: u32 = 720;
    let (w, h) = (w.max(1), h.max(1));
    let long = w.max(h);
    if long <= MAX {
        return (w, h);
    }
    let s = MAX as f32 / long as f32;
    (
        ((w as f32 * s).round() as u32).max(1),
        ((h as f32 * s).round() as u32).max(1),
    )
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
/// The pack REGISTRY (`sources.json`, W3) lives in this tree but is not
/// an element — counting it told a pinned-but-empty library it had one.
fn count_files(dir: &Path) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    rd.flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                count_files(&p)
            } else if e.file_name() == "sources.json" {
                0
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

    /// WF1: the app-side runner materializes the storyboard workflow into
    /// a scratch run folder — slides minted per slot, board filled, comp
    /// saved, generation and render honestly skipped. No content root, no
    /// fabric: the runner is jailed to the dir it is handed.
    #[test]
    fn the_workflow_runner_materializes_the_storyboard() {
        let dir = std::env::temp_dir()
            .join("dpl-composer-tests")
            .join(format!("wf-run-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let doc = dpl_workflows::find("storyboard").expect("registered");
        let mut runner = WfRunner {
            dir: dir.clone(),
            rel: "inbox/workflow-storyboard".into(),
            assembled: 0,
        };
        let (report, board) =
            dpl_workflows::execute(&doc, &mut runner, &mut |_| {}).expect("valid doc runs");
        assert!(!report.failed(), "{:?}", report);
        assert_eq!(board.remaining().len(), 0, "every slot filled with a slide");
        for slot in &board.slots {
            let slide_dir = dir.join("slides").join(&slot.id);
            let slide = dpl_doc::load(&slide_dir).expect("minted slide loads");
            assert!(dpl_doc::validate(&slide).is_empty());
        }
        let comp = dpl_comp::load(&dir.join("comp")).expect("assembled comp loads");
        assert!(dpl_comp::validate(&comp).is_empty());
        assert_eq!(runner.assembled, 0, "text slides are not comp layers yet");
        let render = report.stages.iter().find(|s| s.stage == "render").unwrap();
        assert!(
            matches!(&render.verdict, dpl_workflows::StageVerdict::Skipped { .. }),
            "render skips honestly with nothing assembled"
        );
        assert!(report
            .verdict_line()
            .starts_with("run-workflow: storyboard ok"));
    }

    fn video_rec(name: &str, rel: &str, secs: Option<u32>) -> Record {
        Record {
            rel_path: rel.into(),
            kind: "video".into(),
            name: name.into(),
            seconds: secs,
            ..Default::default()
        }
    }

    /// L3: an overlay lands on the FX track at the playhead, the comp
    /// stays valid, and the base track is untouched.
    #[test]
    fn add_overlay_places_at_the_playhead_on_the_fx_track() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("Base", "inbox/base.mp4", Some(10)));
        c.time = 3.2;
        c.add_overlay(&video_rec("Fire", "elements/rtfx/fire.mov", Some(2)));
        assert_eq!(c.comp.tracks.len(), 2, "fx track minted on first use");
        assert_eq!(c.comp.tracks[0].layers.len(), 1, "base untouched");
        let fx = &c.comp.tracks[1].layers[0];
        assert!((fx.start - 3.2).abs() < 1e-6, "placed at the playhead");
        assert_eq!(fx.scale, 1.0);
        assert_eq!(fx.tint, None);
        assert!(dpl_comp::validate(&c.comp).is_empty());
    }

    /// L3: only overlay-track layers under the playhead are live, bottom
    /// row first within a track (row 0 = topmost paints last).
    #[test]
    fn overlay_layers_at_selects_by_time_in_paint_order() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("Base", "inbox/base.mp4", Some(10)));
        c.time = 0.0;
        c.add_overlay(&video_rec("A", "inbox/a.mov", Some(4)));
        c.time = 1.0;
        c.add_overlay(&video_rec("B", "inbox/b.mov", Some(4)));
        // At t=2 both cover; row 0 (B, newest) is topmost → LAST in paint order.
        let order: Vec<&str> = c
            .overlay_layers_at(2.0)
            .iter()
            .map(|l| l.source.reference())
            .collect();
        assert_eq!(order, vec!["inbox/a.mov", "inbox/b.mov"]);
        assert!(c.overlay_layers_at(0.5).len() == 1, "B not yet started");
        assert!(c.overlay_layers_at(9.0).is_empty(), "both ended");
    }

    /// L3: render_parts maps the whole comp — base clips as concat legs, a
    /// base-track Lottie as black + a same-timed overlay, overlay-track
    /// artifacts as file overlays (stills hold), and black tail padding so
    /// an overlay hanging past the last cut stays visible.
    #[test]
    fn render_parts_map_tracks_black_gaps_and_the_tail() {
        use dpl_engine_media::CompBase;
        let mut c = Composer::default();
        c.add_clip(&video_rec("Base", "inbox/base.mp4", Some(4)));
        // A base-track Lottie span (added directly — add_lottie probes disk).
        if let Some(t) = c.comp.tracks.first_mut() {
            t.layers.push(Layer::plain(
                "lot",
                Source::Lottie {
                    rel: "inbox/title.json".into(),
                },
                4.0,
                0.0,
                2.0,
            ));
        }
        c.time = 5.0;
        c.add_overlay(&video_rec("Fire", "elements/fire.mov", Some(3)));
        let parts = c.render_parts();
        assert_eq!(parts.base.len(), 3, "clip + black lottie span + tail");
        assert!(matches!(parts.base[0], CompBase::Clip(_)));
        assert!(
            matches!(parts.base[1], CompBase::Black { dur } if (dur - 2.0).abs() < 1e-3),
            "lottie span plays black under its overlay"
        );
        assert!(
            matches!(parts.base[2], CompBase::Black { dur } if (dur - 2.0).abs() < 1e-3),
            "tail pad: overlay ends at 8s, base at 6s"
        );
        assert_eq!(parts.overlays.len(), 2, "lottie + fx overlay");
        assert!(matches!(parts.overlays[0], OverlaySlot::Lottie(_)));
        match &parts.overlays[1] {
            OverlaySlot::Ready(o) => {
                assert!((o.at - 5.0).abs() < 1e-6);
                assert!(!o.hold, "a video overlay is not held");
            }
            _ => panic!("fx overlay should be ready"),
        }
        assert_eq!(parts.elements_skipped, 0);
    }

    /// L3: the caps policy is machine-derived, never a constant — a big box
    /// earns more lanes, the 16GB laptop stays clamped.
    #[test]
    fn preview_caps_scale_with_the_machine() {
        let laptop = PreviewCaps::for_machine(15.3, 16);
        assert_eq!((laptop.clip_lanes, laptop.lottie_lanes), (3, 1));
        let rig = PreviewCaps::for_machine(64.0, 24);
        assert_eq!((rig.clip_lanes, rig.lottie_lanes), (6, 2));
        let tiny = PreviewCaps::for_machine(8.0, 4);
        assert_eq!((tiny.clip_lanes, tiny.lottie_lanes), (2, 1));
        let monster = PreviewCaps::for_machine(128.0, 88);
        assert_eq!((monster.clip_lanes, monster.lottie_lanes), (8, 2));
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
    fn add_lottie_appends_a_native_lottie_layer_and_validates() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("Intro", "inbox/intro.mp4", Some(3)));
        let rec = Record {
            rel_path: "inbox/title.json".into(),
            kind: "lottie".into(),
            name: "Title".into(),
            ..Default::default()
        };
        c.add_lottie(&rec);
        assert_eq!(c.layers().len(), 2);
        assert!(
            matches!(c.layers()[1].source, Source::Lottie { .. }),
            "the second layer is a native Lottie source"
        );
        // Stitched onto the end (the first clip runs 0..3).
        assert!((c.layers()[1].start - 3.0).abs() < 1e-6);
        // Missing file → duration falls back, never zero (comp stays valid).
        assert!(c.layers()[1].out_pt > 0.0);
        assert!(dpl_comp::validate(&c.comp).is_empty());
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
    fn render_parts_map_in_order_skipping_elements() {
        use dpl_engine_media::CompBase;
        let mut c = Composer::default();
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(3)));
        c.add_clip(&video_rec("B", "inbox/b.mp4", Some(4)));
        if let Some(t) = c.comp.tracks.first_mut() {
            t.layers.push(Layer::plain(
                "el",
                Source::Element { id: "spark".into() },
                0.0,
                0.0,
                2.0,
            ));
        }
        let parts = c.render_parts();
        assert_eq!(parts.base.len(), 2, "element source is skipped");
        assert_eq!(parts.elements_skipped, 1, "and counted honestly");
        match (&parts.base[0], &parts.base[1]) {
            (CompBase::Clip(a), CompBase::Clip(b)) => {
                assert!(a.path.to_string_lossy().contains("a.mp4"));
                assert!((b.out_pt - 4.0).abs() < 1e-6);
            }
            _ => panic!("both base legs are clips"),
        }
    }

    #[test]
    fn next_hint_walks_the_guided_flow() {
        let mut c = Composer::default();
        assert!(c.next_hint(false, false).contains("add footage"));
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(3)));
        assert!(c.next_hint(false, false).contains("Save"));
        c.dir = Some(PathBuf::from("inbox/comp-1"));
        assert!(c.next_hint(false, false).contains("Render"));
        assert!(c.next_hint(true, false).contains("Rendering"));
        assert!(c.next_hint(false, true).contains("file the render"));
        // The dir is a dummy — never let the Drop flush write it.
        c.dirty = false;
    }

    #[test]
    fn undo_redo_round_trip_restores_spans() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(4)));
        let id = c.layers()[0].id.clone();
        // A completed drag = one checkpoint at grab + the live edits.
        c.checkpoint();
        c.apply_drag(&Drag::TrimOut(id.clone()), 2.0);
        assert!((c.layers()[0].span() - 2.0).abs() < 1e-6);
        c.undo_now();
        assert!((c.layers()[0].span() - 4.0).abs() < 1e-6, "trim undone");
        c.undo_now();
        assert!(c.layers().is_empty(), "add undone");
        c.redo_now();
        assert_eq!(c.layers().len(), 1, "add redone");
        assert!((c.layers()[0].span() - 4.0).abs() < 1e-6);
        c.redo_now();
        assert!((c.layers()[0].span() - 2.0).abs() < 1e-6, "trim redone");
        assert!(dpl_comp::validate(&c.comp).is_empty());
    }

    #[test]
    fn checkpoint_dedups_noops_and_caps_depth() {
        let mut c = Composer::default();
        c.checkpoint();
        c.checkpoint();
        assert_eq!(c.undo_stack.len(), 1, "identical snapshots dedup");
        for i in 0..(UNDO_CAP + 20) {
            c.add_clip(&video_rec("X", &format!("inbox/x{i}.mp4"), Some(1)));
        }
        assert!(c.undo_stack.len() <= UNDO_CAP, "history stays bounded");
    }

    #[test]
    fn undo_prunes_dangling_selection_and_clamps_time() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(3)));
        let id = c.layers()[0].id.clone();
        c.expanded.insert(id.clone());
        c.time = 3.0;
        assert_eq!(c.selected.as_deref(), Some(id.as_str()));
        c.undo_now(); // back to the empty comp
        assert!(c.layers().is_empty());
        assert!(c.selected.is_none(), "dangling selection dropped");
        assert!(c.expanded.is_empty(), "dangling twirl dropped");
        assert_eq!(c.time, 0.0, "playhead clamped to the restored comp");
        assert!(c.dirty, "a history jump is an edit the autosave persists");
    }

    #[test]
    fn a_fresh_edit_clears_redo() {
        let mut c = Composer::default();
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(3)));
        c.undo_now();
        assert!(!c.redo_stack.is_empty());
        c.add_clip(&video_rec("B", "inbox/b.mp4", Some(2)));
        assert!(c.redo_stack.is_empty(), "a new edit invalidates redo");
    }

    #[test]
    fn new_comp_resets_the_edit_and_keeps_the_session() {
        let mut c = Composer::default();
        c.packs = Some(Vec::new());
        c.palette = Some(Vec::new());
        c.add_clip(&video_rec("A", "inbox/a.mp4", Some(3)));
        c.save_name = "typed".into();
        c.new_comp();
        assert!(c.layers().is_empty());
        assert!(c.dir.is_none());
        assert!(c.undo_stack.is_empty() && c.redo_stack.is_empty());
        assert!(!c.dirty);
        assert!(c.save_name.is_empty());
        assert!(
            c.packs.is_some() && c.palette.is_some(),
            "session caches survive New — only the comp resets"
        );
    }

    #[test]
    fn drop_flushes_pending_edits_to_disk() {
        let dir =
            std::env::temp_dir().join(format!("dpl-composer-r1z-flush-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        {
            let mut c = Composer::default();
            c.add_clip(&video_rec("A", "inbox/a.mp4", Some(3)));
            c.dir = Some(dir.clone());
        } // drop = app exit
        let saved = dpl_comp::load(&dir).unwrap();
        assert_eq!(
            saved.tracks[0].layers.len(),
            1,
            "the drop flush wrote comp.json"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn key_act_maps_the_editor_keys() {
        use egui::{Key, Modifiers};
        let none = Modifiers::default();
        let ctrl = Modifiers {
            command: true,
            ..Default::default()
        };
        let ctrl_shift = Modifiers {
            command: true,
            shift: true,
            ..Default::default()
        };
        assert!(matches!(
            key_act(Key::Space, none, None),
            Some(Act::TogglePlay)
        ));
        assert!(
            matches!(key_act(Key::Delete, none, Some("clip-1")), Some(Act::Remove(id)) if id == "clip-1")
        );
        assert!(
            key_act(Key::Delete, none, None).is_none(),
            "no selection, nothing removed"
        );
        assert!(matches!(
            key_act(Key::Backspace, none, Some("x")),
            Some(Act::Remove(_))
        ));
        assert!(matches!(key_act(Key::Z, ctrl, None), Some(Act::Undo)));
        assert!(matches!(key_act(Key::Z, ctrl_shift, None), Some(Act::Redo)));
        assert!(matches!(key_act(Key::Y, ctrl, None), Some(Act::Redo)));
        assert!(key_act(Key::Z, none, None).is_none(), "bare Z is unbound");
        assert!(matches!(key_act(Key::Home, none, None), Some(Act::ToStart)));
        assert!(matches!(key_act(Key::End, none, None), Some(Act::ToEnd)));
        assert!(matches!(key_act(Key::ArrowLeft, none, None), Some(Act::Step(d)) if d < 0.0));
        assert!(matches!(key_act(Key::ArrowRight, none, None), Some(Act::Step(d)) if d > 0.0));
        assert!(
            key_act(Key::Space, ctrl, None).is_none(),
            "ctrl+space is unbound"
        );
    }

    #[test]
    fn the_elements_census_never_counts_the_registry() {
        let dir = std::env::temp_dir().join(format!("dpl-composer-census-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("pack")).unwrap();
        std::fs::write(dir.join("sources.json"), b"[]").unwrap();
        std::fs::write(dir.join("pack").join("spark.mogrt"), b"x").unwrap();
        assert_eq!(count_files(&dir), 1, "the registry is not an element");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// W11: the media-slide mapping — a video record becomes a `video` media
    /// block carrying its poster; an image record an `image` block. Both are
    /// valid `template:"media"` documents. Pure (no disk): mint/PDF that write
    /// the content root are proven by the harness seam, not a unit test.
    #[test]
    fn media_slide_for_maps_records_to_valid_media_slides() {
        let mut vid = video_rec("Screen Clip", "inbox/clip.mp4", Some(4));
        vid.poster_rel_path = Some("inbox/clip-poster.jpg".into());
        let s = Composer::media_slide_for(&vid);
        assert_eq!(s.template, "media");
        assert_eq!(s.background, Some(dpl_doc::Background::None));
        assert!(
            dpl_doc::validate(&s).is_empty(),
            "video media slide is valid"
        );
        let dpl_doc::Block::Media {
            show, src, poster, ..
        } = &s.blocks[0]
        else {
            panic!("a media block");
        };
        assert_eq!(show, "video");
        assert_eq!(src, "inbox/clip.mp4");
        assert_eq!(poster.as_deref(), Some("inbox/clip-poster.jpg"));

        let img = Record {
            rel_path: "inbox/pix.png".into(),
            kind: "image".into(),
            name: "Pix".into(),
            ..Record::default()
        };
        let s2 = Composer::media_slide_for(&img);
        let dpl_doc::Block::Media { show, .. } = &s2.blocks[0] else {
            panic!("a media block");
        };
        assert_eq!(show, "image");
        assert!(
            dpl_doc::validate(&s2).is_empty(),
            "image media slide is valid"
        );
        let replay = video_rec("Session replay", "inbox/sess/light.mp4", Some(8));
        assert_eq!(
            Composer::media_slide_for(&replay).template,
            "media",
            "replay rides the video path"
        );
    }

    #[test]
    fn add_from_library_is_the_one_outside_seam() {
        let mut c = Composer::default();
        // A video rides the full add_clip path: layer + undo snapshot.
        c.add_from_library(&video_rec("Clip", "inbox/clip.mp4", Some(4)));
        assert_eq!(c.layers().len(), 1, "video joins the timeline");
        assert!(!c.undo_stack.is_empty(), "checkpoint rode the add");
        assert!(c.dirty, "autosave sees the edit");
        // W7: an image is a first-class layer — held 5s, valid comp.
        let img = Record {
            rel_path: "inbox/pix.png".into(),
            kind: "image".into(),
            name: "Pix".into(),
            ..Record::default()
        };
        c.add_from_library(&img);
        assert_eq!(c.layers().len(), 2, "the image joined the timeline");
        let l = &c.layers()[1];
        assert!((l.span() - 5.0).abs() < 1e-6, "held DEFAULT_IMAGE_SECS");
        assert!((l.start - 4.0).abs() < 1e-6, "appended after the video");
        assert!(dpl_comp::validate(&c.comp).is_empty());
        // The render maps it as a STILL leg.
        let parts = c.render_parts();
        match (&parts.base[0], &parts.base[1]) {
            (dpl_engine_media::CompBase::Clip(a), dpl_engine_media::CompBase::Clip(b)) => {
                assert!(!a.still && b.still, "still flags mapped")
            }
            _ => panic!("both base legs are clips"),
        }
    }

    #[test]
    fn fmt_secs_reads_in_plain_words() {
        assert_eq!(fmt_secs(None), "—");
        assert_eq!(fmt_secs(Some(0)), "—");
        assert_eq!(fmt_secs(Some(6)), "6s");
        assert_eq!(fmt_secs(Some(64)), "1:04");
    }

    #[test]
    fn review_rows_map_losslessly_and_blank_captions_clear() {
        let step = dpl_guide::Step {
            order: 3,
            file: "step-003.webp".into(),
            result_file: Some("step-003-done.webp".into()),
            action: "click".into(),
            click: Some([0.5, 0.25]),
            caption: Some("old words".into()),
            app: "Test".into(),
            title: "Test - App".into(),
            width: 100,
            height: 80,
            bytes: 12,
            kind: String::new(),
            clip: None,
            clip_in: None,
            clip_out: None,
        };
        let typed = review_edit(&ReviewStep {
            keep: true,
            caption: "  new words  ".into(),
            step: step.clone(),
        });
        assert_eq!(typed.caption.as_deref(), Some("new words"), "trimmed");
        assert_eq!(typed.file, "step-003.webp");
        assert_eq!(typed.result_file.as_deref(), Some("step-003-done.webp"));
        assert_eq!(typed.click, Some([0.5, 0.25]));
        assert_eq!(typed.app, "Test", "recorder facts survive the round trip");
        let blank = review_edit(&ReviewStep {
            keep: true,
            caption: "   ".into(),
            step,
        });
        assert!(blank.caption.is_none(), "blank caption clears");
    }

    #[test]
    fn export_paths_are_jailed_and_minted_no_clobber() {
        let root = std::env::temp_dir().join(format!("dpl-composer-jail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("inbox").join("g-share")).unwrap();
        assert!(jailed(&root, "inbox/g").is_ok());
        assert!(jailed(&root, "../escape").is_err(), "dotdot refused");
        assert!(jailed(&root, "C:/windows").is_err(), "drive letter refused");
        assert!(jailed(&root, "/abs").is_err(), "absolute refused");
        assert!(jailed(&root, "").is_err(), "empty refused");
        let minted = mint_export_dir(&root.join("inbox"), "g-share");
        assert!(
            minted.ends_with("g-share-2"),
            "existing folder never clobbered: {}",
            minted.display()
        );
        // run_export refuses a non-guide folder by name.
        std::fs::create_dir_all(root.join("inbox").join("not-a-guide")).unwrap();
        let err = run_export(
            &root,
            "inbox/not-a-guide",
            None,
            None,
            dpl_guide::ExportOpts::default(),
            None,
        )
        .unwrap_err();
        assert!(err.contains("guide.json"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn remember_last_dual_writes_into_the_estate() {
        let edir = std::env::temp_dir().join(format!("dpl-composer-estate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&edir);
        let est = std::sync::Arc::new(dpl_estate::Estate::open(&edir).unwrap());
        let mut c = Composer::default();
        c.wire_estate(std::sync::Arc::clone(&est));
        let comp = Composer::content_root().join("inbox").join("estate-comp");
        let _ = std::fs::create_dir_all(&comp);
        c.remember_last(&comp);
        let got: Option<String> = est
            .get_json(
                dpl_estate::Tree::State,
                dpl_estate::KEY_WIREDFRONT_LAST_COMP,
            )
            .unwrap();
        assert_eq!(got.as_deref(), Some("inbox/estate-comp"));
        let _ = std::fs::remove_dir_all(&edir);
    }
}
