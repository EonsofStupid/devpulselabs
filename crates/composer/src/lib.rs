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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The default trimmed length for a clip whose duration we cannot probe.
const FALLBACK_CLIP_SECS: f32 = 6.0;
/// Timeline row heights: a layer's lane, a twirled property row, the ruler.
const LANE_H: f32 = 26.0;
const PROP_H: f32 = 20.0;
const RULER_H: f32 = 22.0;
/// The layer OUTLINE column's width (timeline left half).
const OUTLINE_W: f32 = 252.0;
/// Undo depth — comp snapshots are refs-only JSON, so 100 stays tiny.
const UNDO_CAP: usize = 100;
/// The autosave debounce: a saved comp persists this long after the last
/// edit settles (mid-drag edits wait for the release).
const AUTOSAVE_SECS: f32 = 2.0;

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
    /// The PACK SHELF — pinned sources' packs, listed live (in place).
    packs: Option<Vec<packs::Pack>>,
    /// The footage row highlighted in the project preview well.
    proj_sel: Option<usize>,
    /// The off-thread stitch render's shared state.
    render: Arc<Mutex<RenderState>>,
    /// The off-thread pack INGEST's shared state (one at a time).
    ingest: Arc<Mutex<IngestState>>,
    /// Files processed by the running ingest (the progress pill).
    ingest_done: Arc<AtomicUsize>,
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
    /// The off-thread guide EXPORT's shared state.
    export: Arc<Mutex<ExportState>>,
    status: String,
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

/// The async guide-export job's state — the Guide tab polls it.
enum ExportState {
    Idle,
    Running,
    Done(PathBuf),
    Failed(String),
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
        let mut c = Self {
            comp: Composition::default_1080p30(),
            dir: None,
            selected: None,
            preview: None,
            palette: None,
            comps: None,
            renders: None,
            elements: None,
            packs: None,
            proj_sel: None,
            render: Arc::new(Mutex::new(RenderState::Idle)),
            ingest: Arc::new(Mutex::new(IngestState::Idle)),
            ingest_done: Arc::new(AtomicUsize::new(0)),
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
            export: Arc::new(Mutex::new(ExportState::Idle)),
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

/// The async pack-ingest job's state — the status line polls it.
enum IngestState {
    Idle,
    Running { pack: String },
    Done { pack: String, copied: usize },
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
        self.checkpoint();
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
        self.checkpoint();
        if let Some(track) = self.comp.tracks.first_mut() {
            track.layers.retain(|l| l.id != id);
        }
        self.expanded.remove(id);
        if self.selected.as_deref() == Some(id) {
            self.selected = None;
            self.preview = None;
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
        state::save(
            &Self::content_root(),
            &state::EditorState {
                last_comp: Some(Self::short(dir)),
            },
        );
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
                    Ok(()) => {
                        // Write-through: the render becomes an ordinary
                        // indexed artifact so Gallery/Keep/agents see it.
                        index_render(&out);
                        RenderState::Done(out)
                    }
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
                self.status = format!("Guide updated — {n} step(s) kept.");
                self.guides = None;
                self.load_review(&rel);
            }
            Err(e) => self.status = format!("Apply failed: {e}"),
        }
        self.apply_armed = false;
    }

    /// Kick off the guide EXPORT off the UI thread; the Guide tab polls
    /// the shared `ExportState`.
    fn start_export(&mut self) {
        if matches!(
            *self.export.lock().expect("export state"),
            ExportState::Running
        ) {
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
        let state = Arc::clone(&self.export);
        *state.lock().expect("export state") = ExportState::Running;
        std::thread::Builder::new()
            .name("dpl-guide-export".into())
            .spawn(move || {
                let result = run_export(
                    &root,
                    &guide_rel,
                    session_rel.as_deref(),
                    name.as_deref(),
                    opts,
                );
                *state.lock().expect("export state") = match result {
                    Ok(e) => ExportState::Done(e.dir),
                    Err(e) => ExportState::Failed(e),
                };
            })
            .expect("spawn export thread");
    }

    /// File a finished export into a show/episode — guide folders keep
    /// wholesale (`Index::keep`).
    fn keep_export(&mut self, show: &str, ep: &str) {
        let dir = match &*self.export.lock().expect("export state") {
            ExportState::Done(d) => d.clone(),
            _ => return,
        };
        let root = Self::content_root();
        let rel = Self::short(&dir);
        self.status =
            match dpl_index::Index::open_default().and_then(|ix| ix.keep(&root, &rel, show, ep)) {
                Ok(new_rel) => {
                    *self.export.lock().expect("export state") = ExportState::Idle;
                    self.guides = None;
                    format!("Filed into {show}/{ep} → {new_rel}")
                }
                Err(e) => format!("Keep failed: {e}"),
            };
    }

    /// Open a Browse peer window rooted at `start`.
    fn open_browser(&mut self, start: PathBuf) {
        self.browsers.push(Arc::new(Mutex::new(browse::Browser::new(
            self.next_browser,
            start,
        ))));
        self.next_browser += 1;
    }

    /// INGEST one pack OFF the UI thread: copy its media into
    /// `content/elements/<pack>/`, indexed and postered — the operator's
    /// "grab the ones I often use". One at a time; sources untouched.
    fn start_ingest(&mut self, pack: &Path) {
        {
            let st = self.ingest.lock().expect("ingest state");
            if matches!(*st, IngestState::Running { .. }) {
                self.status = "One ingest at a time — the current one is still copying.".into();
                return;
            }
        }
        let name = pack
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "pack".into());
        self.ingest_done.store(0, Ordering::Relaxed);
        let state = Arc::clone(&self.ingest);
        let done = Arc::clone(&self.ingest_done);
        let pack = pack.to_path_buf();
        *state.lock().expect("ingest state") = IngestState::Running { pack: name.clone() };
        std::thread::Builder::new()
            .name("dpl-pack-ingest".into())
            .spawn(move || {
                let root = Self::content_root();
                let result = packs::ingest_copy(&root, &pack, &name, &done);
                *state.lock().expect("ingest state") = match result {
                    Ok(copied) => IngestState::Done { pack: name, copied },
                    Err(e) => IngestState::Failed(e),
                };
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

    /// Route one imported file: videos join the timeline, images join
    /// the library, elements file into the elements shelf — honestly.
    fn handle_import(&mut self, src: &Path) {
        match import::import_file(src) {
            Ok(rec) if rec.kind == "video" => self.add_clip(&rec),
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
        let path = match &*self.render.lock().expect("render state") {
            RenderState::Done(p) => p.clone(),
            _ => return,
        };
        let root = Self::content_root();
        let rel = Self::short(&path);
        self.status =
            match dpl_index::Index::open_default().and_then(|ix| ix.keep(&root, &rel, show, ep)) {
                Ok(new_rel) => {
                    *self.render.lock().expect("render state") = RenderState::Idle;
                    self.renders = None;
                    format!("Filed into {show}/{ep} → {new_rel}")
                }
                Err(e) => format!("Keep failed: {e}"),
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
        out
    }

    /// The Composer surface — what the WiredFRONT hub calls for its subroute.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.maybe_restore();
        self.ensure_project();
        self.tick_playback(ui.ctx());
        self.show_browsers(ui.ctx());
        self.drain_media_feeds(ui.ctx());
        // The ingest pill: progress while Running, cache reset on Done.
        let finished = {
            let mut st = self.ingest.lock().expect("ingest state");
            match &*st {
                IngestState::Running { pack } => {
                    self.status = format!(
                        "Ingesting {pack} — {} file(s) processed…",
                        self.ingest_done.load(Ordering::Relaxed)
                    );
                    ui.ctx().request_repaint_after(Duration::from_millis(300));
                    false
                }
                IngestState::Done { pack, copied } => {
                    self.status =
                        format!("Ingested {pack} — {copied} file(s) now local in your library.");
                    *st = IngestState::Idle;
                    true
                }
                IngestState::Failed(e) => {
                    self.status = format!("Ingest failed: {e}");
                    *st = IngestState::Idle;
                    false
                }
                IngestState::Idle => false,
            }
        };
        if finished {
            self.palette = None;
            self.elements = None;
            self.packs = None;
        }
        // A render in flight keeps the frame alive so its status pill updates.
        if matches!(
            *self.render.lock().expect("render state"),
            RenderState::Running
        ) {
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
        self.workflow_tabs(&mut mk(tabbar_rect), &mut act);
        match self.tab {
            Tab::Editor => self.ae_screen(ui, body, &mut act),
            Tab::Annotate => self.annotate_tab(&mut mk(body), &mut act),
            Tab::Guide => self.guide_tab(&mut mk(body)),
            other => tab_stub(&mut mk(body), other),
        }

        match act {
            Some(Act::New) => self.new_comp(),
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
        let status_h = 58.0_f32;
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
    fn status_row(&mut self, ui: &mut egui::Ui, act: &mut Option<Act>) {
        // Snapshot the render state once (don't hold the lock across the UI).
        let (rendering, done, pill) = {
            let st = self.render.lock().expect("render state");
            match &*st {
                RenderState::Idle => (false, false, None),
                RenderState::Running => (
                    true,
                    false,
                    Some(("Rendering...".to_string(), cl::ink::BASE)),
                ),
                RenderState::Done(p) => (
                    false,
                    true,
                    Some((format!("Rendered: {}", Self::short(p)), cl::ink::STRONG)),
                ),
                RenderState::Failed(e) => (
                    false,
                    false,
                    Some((format!("Render failed: {e}"), cl::ink::MUTED)),
                ),
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
            ui.label(
                RichText::new(self.next_hint(rendering, done))
                    .small()
                    .color(Tone::Lime.color()),
            );
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
        let (exporting, pill, done_dir) = {
            let st = self.export.lock().expect("export state");
            match &*st {
                ExportState::Idle => (false, None, None),
                ExportState::Running => (true, Some("Exporting…".to_string()), None),
                ExportState::Done(d) => (
                    false,
                    Some(format!("Exported: {}/guide.html", Self::short(d))),
                    Some(d.clone()),
                ),
                ExportState::Failed(e) => (false, Some(format!("Export failed: {e}")), None),
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

/// Write-through for a finished render: poster + duration + index row
/// (kind video, source "render") — so the Gallery, the Keep seam, and
/// the agents all see it. Runs on the render thread; failures degrade
/// the card, never the render.
fn index_render(out: &Path) {
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
        let _ = ix.upsert(&rec);
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
        let _ = ix.upsert(&rec);
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
        )
        .unwrap_err();
        assert!(err.contains("guide.json"), "{err}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
