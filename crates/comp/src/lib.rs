//! The COMPOSITION document (R1a) — a video comp is its own FOLDER holding
//! a colocated `comp.json`: tracks → layers → sources, with style/media as
//! REFS, NEVER VALUES (a layer names an artifact rel_path or an element id;
//! it never embeds bytes). This is the trick that makes a comp AI-editable —
//! an agent reads and writes plain, validated JSON, exactly like a slide
//! (`dpl-doc`). Pure data: no egui, no ffmpeg, no compositor in this crate.
//!
//! R1a is the FOUNDATION. Transform keyframes, the effect registry, and the
//! `GenBackend` seam land in later R1 passes; the schema grows additively.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Current schema version. Bump on breaking shape changes; `load` refuses a
/// newer version honestly instead of misreading it.
pub const SCHEMA_V: u32 = 1;

/// One composition — the editable truth of `<comp-dir>/comp.json`. A comp is
/// an ordered stack of tracks; each track an ordered run of layers placed on
/// a timeline. `fps`/`w`/`h` are the render intent (the final ffmpeg tier,
/// R1b, honors them); preview is reduced-res regardless.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Composition {
    pub v: u32,
    pub fps: u32,
    pub w: u32,
    pub h: u32,
    pub tracks: Vec<Track>,

    // ── comp-level switches (mock ② calls m.blur "a comp switch"). Additive
    // and skipped at default, exactly like the layer fields. ──────────────
    /// The motion-blur MASTER. A layer blurs only when this AND its own
    /// switch are on — AE's rule, so one toggle can kill it comp-wide.
    #[serde(default, skip_serializing_if = "is_no")]
    pub motion_blur: bool,
    /// Shutter angle in degrees (AE's default is 180). Drives how many frames
    /// the render averages.
    #[serde(default = "shutter", skip_serializing_if = "is_shutter")]
    pub shutter: u32,
    /// Camera focal length in comp px. `None` = NO CAMERA: `z` is inert and
    /// the comp is flat, which is byte-identical to every comp authored
    /// before this field existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focal: Option<f32>,
}

/// One track — a named, ordered run of layers. Tracks stack back-to-front in
/// authored order (the compositor, R1b, paints track 0 first).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub layers: Vec<Layer>,
}

/// One layer — a source clip placed on its track's timeline and trimmed.
/// Times are seconds. `start` is the placement on the track; `in_pt`/`out_pt`
/// trim WITHIN the source. L3 adds a STATIC transform + tint (overlay tracks
/// honor them; the base track ignores them). Keyframes are a later pass —
/// a bare layer is a straight cut, which is all stitching needs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub source: Source,
    /// Placement on the track timeline (seconds from comp start).
    pub start: f32,
    /// Trim in — seconds into the source where this layer begins.
    pub in_pt: f32,
    /// Trim out — seconds into the source where this layer ends.
    pub out_pt: f32,
    /// Center-relative horizontal offset in comp px (0 = centered).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x: f32,
    /// Center-relative vertical offset in comp px (0 = centered).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub y: f32,
    /// Size factor: 1.0 = aspect-fit the comp frame (full-frame native).
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub scale: f32,
    /// RGBA multiply as VALUES 0-255 (`[r,g,b,a]`; `a` is layer opacity).
    /// None = untinted. An array, never a hex string — the shape law.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tint: Option<[u8; 4]>,
    /// How this layer composites (L4). Overlay tracks honor it; the base
    /// track ignores it. Normal = alpha-over (skipped on disk).
    #[serde(default, skip_serializing_if = "Blend::is_normal")]
    pub blend: Blend,
    /// A luma-matte video's rel path — its LUMINANCE becomes this layer's
    /// alpha (the estate's matte-wipe packs). A REF, never bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matte: Option<String>,
    /// A `.cube` LUT's rel path applied to this layer (base cuts grade
    /// too — the 860-cube use case). A REF, never bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lut: Option<String>,

    // ── the timeline switches (mock ⑦). Every one is ADDITIVE and skipped at
    // its default, so an existing comp.json round-trips byte-identically and
    // SCHEMA_V does not move. ─────────────────────────────────────────────
    /// Hidden layers render as if absent. A hidden BASE cut still holds its
    /// time as black, so hiding never re-times everything after it.
    #[serde(default = "yes", skip_serializing_if = "is_yes")]
    pub visible: bool,
    /// Editor-side only: a locked layer cannot be selected, dragged, trimmed
    /// or removed. Never reaches the render.
    #[serde(default, skip_serializing_if = "is_no")]
    pub locked: bool,
    /// Per-layer motion blur, gated by the comp master switch — AE's rule:
    /// both must be on.
    #[serde(default, skip_serializing_if = "is_no")]
    pub motion_blur: bool,
    /// Another layer's id whose transform this one rides. Cycles are refused
    /// at author time and survived at resolve time ([`resolved_transform`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Depth in comp px. 0 = the camera's focal plane (unchanged). Positive
    /// is further away (smaller), negative is nearer (larger). Inert unless
    /// the comp has a `focal` — so a comp without a camera is exactly flat.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub z: f32,
}

/// The L4 blend modes — screen and add cover the estate's light-leak/
/// dust/glow overlays (black is their neutral, so black-padded legs
/// composite invisibly). More modes join additively as passes need them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Blend {
    #[default]
    Normal,
    Screen,
    Add,
}

impl Blend {
    fn is_normal(&self) -> bool {
        *self == Blend::Normal
    }

    /// Every mode, in the order the Mode column steps through them.
    pub const ALL: [Blend; 3] = [Blend::Normal, Blend::Screen, Blend::Add];

    /// What the operator reads in the timeline's Mode column.
    pub fn label(self) -> &'static str {
        match self {
            Blend::Normal => "Normal",
            Blend::Screen => "Screen",
            Blend::Add => "Add",
        }
    }

    /// Plain words for what the mode DOES — the tooltip law, and the reason
    /// these live here rather than in the surface: one vocabulary, wherever
    /// a mode is shown.
    pub fn blurb(self) -> &'static str {
        match self {
            Blend::Normal => "Lays on top as-is, using its own transparency.",
            Blend::Screen => "Only lightens — black in this layer disappears.                               Light leaks, glows, dust.",
            Blend::Add => "Piles light on light. Brighter and harsher than Screen.",
        }
    }

    /// The next mode, wrapping — the Mode column is a tap-through, so there
    /// is always a way back round to Normal.
    pub fn next(self) -> Blend {
        match self {
            Blend::Normal => Blend::Screen,
            Blend::Screen => Blend::Add,
            Blend::Add => Blend::Normal,
        }
    }
}

fn one() -> f32 {
    1.0
}

fn is_one(v: &f32) -> bool {
    *v == 1.0
}

fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

fn yes() -> bool {
    true
}

fn is_yes(v: &bool) -> bool {
    *v
}

fn is_no(v: &bool) -> bool {
    !*v
}

/// AE's default shutter angle.
fn shutter() -> u32 {
    180
}

fn is_shutter(v: &u32) -> bool {
    *v == 180
}

/// How deep a parent chain may go before [`resolved_transform`] stops
/// walking. A hand-edited `comp.json` can contain a cycle; the paint thread
/// and the render must survive one, so the walk is BOUNDED rather than
/// trusting.
pub const PARENT_MAX_DEPTH: usize = 16;

/// A layer's resolved placement — the three values both the preview and the
/// ffmpeg graph already consume. Parenting and depth resolve INTO this, which
/// is why neither needs an engine change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
}

impl Transform {
    pub const IDENTITY: Transform = Transform {
        x: 0.0,
        y: 0.0,
        scale: 1.0,
    };
}

/// Resolve a layer's transform, walking its parent chain.
///
/// A child rides its parent: `scale` multiplies, and the child's offset is
/// expressed in the parent's SCALED space — which is what makes a parented
/// layer track its parent when the parent scales rather than drifting off it.
///
/// **Cycle-safe by construction.** The walk carries a visited set and stops at
/// [`PARENT_MAX_DEPTH`], returning the chain resolved so far. A cycle in a
/// hand-edited file can therefore never hang a frame or a render.
pub fn resolved_transform(layers: &[Layer], id: &str) -> Option<Transform> {
    let find = |lid: &str| layers.iter().find(|l| l.id == lid);
    let start = find(id)?;

    // Collect the chain child→root first, so the compose below reads in the
    // same direction a reader thinks about it.
    let mut chain: Vec<&Layer> = vec![start];
    let mut seen: Vec<&str> = vec![start.id.as_str()];
    let mut cursor = start;
    while let Some(pid) = cursor.parent.as_deref() {
        if seen.contains(&pid) || chain.len() >= PARENT_MAX_DEPTH {
            break; // a cycle, or too deep — keep what we have, never spin
        }
        let Some(p) = find(pid) else { break };
        chain.push(p);
        seen.push(p.id.as_str());
        cursor = p;
    }

    // Apply root→child so each child lands inside its parent's scaled space.
    let mut t = Transform::IDENTITY;
    for l in chain.iter().rev() {
        t = Transform {
            x: t.x + l.x * t.scale,
            y: t.y + l.y * t.scale,
            scale: t.scale * l.scale,
        };
    }
    Some(t)
}

/// Would setting `child.parent = Some(parent)` close a cycle?
///
/// The UI asks BEFORE authoring, so a cycle never reaches the document. The
/// depth cap in [`resolved_transform`] is the second line of defence, for
/// files edited by hand or by an agent.
pub fn parent_would_cycle(layers: &[Layer], child: &str, parent: &str) -> bool {
    if child == parent {
        return true;
    }
    let find = |lid: &str| layers.iter().find(|l| l.id == lid);
    let mut cursor = parent;
    for _ in 0..PARENT_MAX_DEPTH {
        let Some(l) = find(cursor) else { return false };
        match l.parent.as_deref() {
            Some(p) if p == child => return true,
            Some(p) => cursor = p,
            None => return false,
        }
    }
    true // ran out of depth — treat as a cycle rather than authoring one
}

/// Project a transform through a pinhole camera at depth `z`.
///
/// `k = focal / (focal + z)` — the whole of it. `focal: None` means no camera
/// and returns the transform UNCHANGED, so a comp without one is exactly as
/// flat as it was before cameras existed. `z = 0` is likewise identity.
///
/// This is 3D **position**, not rotation: rotation needs a perspective warp
/// and is a later pass. Because the output is `(x, y, scale)`, it feeds the
/// preview and the ffmpeg graph with no engine change at all.
pub fn project(t: Transform, z: f32, focal: Option<f32>) -> Transform {
    let Some(f) = focal else { return t };
    if f <= 0.0 || z == 0.0 {
        return t;
    }
    // Clamp rather than invert or divide by zero: a layer at or behind the
    // camera collapses toward nothing instead of flipping inside out.
    let k = (f / (f + z)).max(0.01);
    Transform {
        x: t.x * k,
        y: t.y * k,
        scale: t.scale * k,
    }
}

impl Layer {
    /// A bare layer: full-frame, centered, unscaled, untinted — the straight
    /// cut every pre-L3 comp is made of, and the default new layers start as.
    pub fn plain(
        id: impl Into<String>,
        source: Source,
        start: f32,
        in_pt: f32,
        out_pt: f32,
    ) -> Self {
        Self {
            id: id.into(),
            source,
            start,
            in_pt,
            out_pt,
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            tint: None,
            blend: Blend::Normal,
            matte: None,
            lut: None,
            visible: true,
            locked: false,
            motion_blur: false,
            parent: None,
            z: 0.0,
        }
    }

    /// The layer's on-timeline length: the trimmed span.
    pub fn span(&self) -> f32 {
        (self.out_pt - self.in_pt).max(0.0)
    }

    /// The timeline position where this layer ends.
    pub fn end(&self) -> f32 {
        self.start + self.span()
    }
}

/// Where a layer's pixels come from — a REF, never a value. Additive by
/// design: new source kinds (generated jobs, nested comps) join as later
/// passes need them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Source {
    /// An artifact `rel_path` under the content root (a captured/generated
    /// clip in the index).
    Artifact { rel: String },
    /// A library element id (the operator's transparent elements; R1c
    /// ingests + indexes them).
    Element { id: String },
    /// A native Lottie/Bodymovin artifact `rel_path` under the content
    /// root — rendered in-app by the ThorVG helper (L2), no Adobe.
    Lottie { rel: String },
}

impl Source {
    /// The ref string, whatever the kind — used by the doctor and pickers.
    pub fn reference(&self) -> &str {
        match self {
            Source::Artifact { rel } => rel,
            Source::Element { id } => id,
            Source::Lottie { rel } => rel,
        }
    }
}

impl Composition {
    /// An empty comp at the given render intent.
    pub fn new(fps: u32, w: u32, h: u32) -> Self {
        Self {
            v: SCHEMA_V,
            fps,
            w,
            h,
            tracks: Vec::new(),
            // A fresh comp is flat and unblurred — the switches are opt-in,
            // and these defaults are what make the field additions invisible
            // on disk.
            motion_blur: false,
            shutter: shutter(),
            focal: None,
        }
    }

    /// The default working comp: 1080p at 30fps, one empty track. This is the
    /// canvas the Composer opens on "New".
    pub fn default_1080p30() -> Self {
        Self {
            tracks: vec![Track {
                id: "main".into(),
                layers: Vec::new(),
            }],
            ..Self::new(30, 1920, 1080)
        }
    }

    /// The comp's duration: the furthest layer end across every track. Empty
    /// = 0.0.
    pub fn duration(&self) -> f32 {
        self.tracks
            .iter()
            .flat_map(|t| t.layers.iter())
            .map(Layer::end)
            .fold(0.0f32, f32::max)
    }
}

/// A validation finding: `path` names WHERE, `message` WHAT — the same shape
/// as `dpl_doc::Issue`. Findings are warnings the doctor NAMES; the runtime
/// degrades unknown refs honestly rather than crashing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub path: String,
    pub message: String,
}

#[derive(Debug)]
pub enum CompError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    /// The file's `v` is newer than this build understands.
    Version(u32),
}

impl std::fmt::Display for CompError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompError::Io(e) => write!(f, "io: {e}"),
            CompError::Parse(e) => write!(f, "parse: {e}"),
            CompError::Version(v) => {
                write!(f, "comp.json is schema v{v}; this build reads v{SCHEMA_V}")
            }
        }
    }
}

impl std::error::Error for CompError {}

const FILE: &str = "comp.json";

/// Load `<dir>/comp.json`.
pub fn load(dir: &Path) -> Result<Composition, CompError> {
    let raw = std::fs::read_to_string(dir.join(FILE)).map_err(CompError::Io)?;
    let comp: Composition = serde_json::from_str(&raw).map_err(CompError::Parse)?;
    if comp.v > SCHEMA_V {
        return Err(CompError::Version(comp.v));
    }
    Ok(comp)
}

/// Crash-safe pretty-JSON+LF write: content lands in a sibling tmp first,
/// then swaps in (Windows cannot rename over an existing file, so the old
/// file steps aside as `.bak` for the swap and the bak is removed on
/// success). The `dpl-doc` writer, replicated — the document layer depends on
/// no other crate for a file swap.
pub fn save(dir: &Path, comp: &Composition) -> Result<(), CompError> {
    std::fs::create_dir_all(dir).map_err(CompError::Io)?;
    let target = dir.join(FILE);
    let tmp = dir.join(format!("{FILE}.tmp"));
    let bak = dir.join(format!("{FILE}.bak"));

    let mut json = serde_json::to_string_pretty(comp).map_err(CompError::Parse)?;
    json.push('\n');
    std::fs::write(&tmp, json).map_err(CompError::Io)?;

    if target.exists() {
        let _ = std::fs::remove_file(&bak);
        std::fs::rename(&target, &bak).map_err(CompError::Io)?;
    }
    std::fs::rename(&tmp, &target).map_err(CompError::Io)?;
    let _ = std::fs::remove_file(&bak);
    Ok(())
}

/// Name every structural fault in a comp. Empty = clean. R1a validates
/// SHAPE, not media existence: whether an artifact rel or element id actually
/// resolves is a SOFT concern for R1c (the index/ingest pass) — the doctor
/// never flags a ref it cannot yet resolve, only a ref that is malformed.
pub fn validate(comp: &Composition) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |path: String, message: String| issues.push(Issue { path, message });

    if comp.fps == 0 {
        push("fps".into(), "fps must be > 0".into());
    }
    if comp.w == 0 || comp.h == 0 {
        push(
            "size".into(),
            format!("w and h must be > 0 (got {}x{})", comp.w, comp.h),
        );
    }

    let mut seen_track = std::collections::BTreeSet::new();
    let mut seen_layer = std::collections::BTreeSet::new();
    for track in &comp.tracks {
        if !seen_track.insert(track.id.clone()) {
            push(format!("tracks.{}", track.id), "duplicate track id".into());
        }
        for layer in &track.layers {
            let at = format!("tracks.{}.{}", track.id, layer.id);
            // Layer ids are unique across the WHOLE comp (reveals/refs name
            // them flat, like slide block ids).
            if !seen_layer.insert(layer.id.clone()) {
                push(at.clone(), "duplicate layer id".into());
            }
            if layer.source.reference().trim().is_empty() {
                push(format!("{at}.source"), "empty source ref".into());
            }
            if layer.in_pt < 0.0 {
                push(
                    format!("{at}.in_pt"),
                    format!("in_pt must be >= 0 (got {})", layer.in_pt),
                );
            }
            if layer.out_pt <= layer.in_pt {
                push(
                    format!("{at}.out_pt"),
                    format!(
                        "out_pt must be > in_pt (got in={} out={})",
                        layer.in_pt, layer.out_pt
                    ),
                );
            }
            if layer.start < 0.0 {
                push(
                    format!("{at}.start"),
                    format!("start must be >= 0 (got {})", layer.start),
                );
            }
            if !(layer.scale.is_finite() && layer.scale > 0.0) {
                push(
                    format!("{at}.scale"),
                    format!("scale must be finite and > 0 (got {})", layer.scale),
                );
            }
            if !layer.x.is_finite() || !layer.y.is_finite() {
                push(
                    format!("{at}.offset"),
                    format!("x/y must be finite (got x={} y={})", layer.x, layer.y),
                );
            }
            if layer.matte.as_deref().is_some_and(|m| m.trim().is_empty()) {
                push(format!("{at}.matte"), "matte ref must not be empty".into());
            }
            if layer.lut.as_deref().is_some_and(|l| l.trim().is_empty()) {
                push(format!("{at}.lut"), "lut ref must not be empty".into());
            }
        }
    }

    issues
}

/// Walk from the exe's directory upward, then from the CWD upward, for a
/// relative path — the asset-loader discovery pattern, reimplemented here so
/// the document layer depends on no engine crate for a path walk (the repo's
/// stated small-utility philosophy, matching `dpl_doc::find_upward`).
pub fn find_upward(rel: &str) -> Option<PathBuf> {
    let mut starts: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            starts.push(dir.to_path_buf());
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    for start in starts {
        let mut here = Some(start.as_path());
        while let Some(dir) = here {
            let candidate = dir.join(rel);
            if candidate.exists() {
                return Some(candidate);
            }
            here = dir.parent();
        }
    }
    None
}

// The FOLDER CONTRACT: a comp folder holds reserved names only — `comp.json`
// and a reserved `assets/` (copy-on-use media, R1c). The doctor flags
// everything else, including interrupted-save transients.
const COMP_RESERVED: &[&str] = &["comp.json", "assets"];

/// Name every entry of the comp folder that the contract does not reserve —
/// strays and leftover `.tmp`/`.bak` transients.
pub fn folder_issues(dir: &Path) -> Vec<Issue> {
    let mut issues = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return issues;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if COMP_RESERVED.contains(&name.as_str()) {
            continue;
        }
        let is_dir = entry.path().is_dir();
        let message = if name.ends_with(".tmp") || name.ends_with(".bak") {
            format!("leftover transient {name:?} from an interrupted save")
        } else {
            format!(
                "unexpected {} {name:?} (contract: {})",
                if is_dir { "folder" } else { "file" },
                COMP_RESERVED.join(", ")
            )
        };
        issues.push(Issue {
            path: "folder".into(),
            message,
        });
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Composition {
        Composition {
            v: SCHEMA_V,
            motion_blur: false,
            shutter: 180,
            focal: None,
            fps: 30,
            w: 1920,
            h: 1080,
            tracks: vec![Track {
                id: "main".into(),
                layers: vec![
                    Layer::plain(
                        "clip-a",
                        Source::Artifact {
                            rel: "inbox/a.mp4".into(),
                        },
                        0.0,
                        0.0,
                        6.0,
                    ),
                    Layer::plain(
                        "clip-b",
                        Source::Artifact {
                            rel: "inbox/b.mp4".into(),
                        },
                        6.0,
                        1.0,
                        9.0,
                    ),
                ],
            }],
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("dpl-comp-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// THE additive claim, proven rather than asserted: a comp that uses none
    /// of the new switches serialises with NO trace of them, so every
    /// comp.json written before this pass is byte-identical after it and
    /// SCHEMA_V never had to move.
    #[test]
    fn the_switches_are_invisible_on_disk_until_used() {
        let mut c = Composition::default_1080p30();
        c.tracks[0].layers.push(Layer::plain(
            "clip-1",
            Source::Artifact {
                rel: "inbox/a.mp4".into(),
            },
            0.0,
            0.0,
            4.0,
        ));
        let json = serde_json::to_string_pretty(&c).unwrap();
        for absent in [
            "visible",
            "locked",
            "motion_blur",
            "parent",
            "\"z\"",
            "shutter",
            "focal",
        ] {
            assert!(
                !json.contains(absent),
                "{absent} must not appear until it is used:\n{json}"
            );
        }
        // `json` above provably contains NONE of the new fields, so it IS an
        // old-format file. Reading it back is therefore the real
        // backward-compatibility test — and it cannot drift from the schema
        // the way a hand-written fixture would.
        let back: Composition = serde_json::from_str(&json).expect("an old comp still loads");
        let l = &back.tracks[0].layers[0];
        assert!(
            l.visible,
            "a layer with no `visible` is VISIBLE, not hidden"
        );
        assert!(!l.locked && !l.motion_blur && l.parent.is_none());
        assert_eq!(l.z, 0.0);
        assert_eq!(back.shutter, 180, "shutter defaults to AE's 180");
        assert!(back.focal.is_none(), "no camera unless one is authored");
    }

    /// A child rides its parent, and its offset is expressed in the parent's
    /// SCALED space — which is what makes a parented layer track its parent
    /// instead of drifting off it when the parent scales.
    #[test]
    fn a_child_rides_its_parents_scaled_space() {
        let mk = |id: &str, x: f32, scale: f32, parent: Option<&str>| {
            let mut l = Layer::plain(
                id,
                Source::Artifact {
                    rel: "inbox/a.mp4".into(),
                },
                0.0,
                0.0,
                1.0,
            );
            l.x = x;
            l.scale = scale;
            l.parent = parent.map(str::to_string);
            l
        };
        let layers = vec![
            mk("rig", 100.0, 2.0, None),
            mk("child", 10.0, 0.5, Some("rig")),
        ];
        let t = resolved_transform(&layers, "child").unwrap();
        assert_eq!(t.scale, 1.0, "0.5 inside a 2x rig is natural size");
        assert_eq!(t.x, 120.0, "the child's 10 is 10 IN THE RIG'S 2x space");
        // The rig itself is unaffected by its child.
        assert_eq!(resolved_transform(&layers, "rig").unwrap().x, 100.0);
        // An unparented layer is just itself.
        assert_eq!(
            resolved_transform(&[mk("solo", 5.0, 1.0, None)], "solo").unwrap(),
            Transform {
                x: 5.0,
                y: 0.0,
                scale: 1.0
            }
        );
    }

    /// A cycle in a hand-edited file must never hang a frame or a render.
    /// Authoring one is refused up front; surviving one is the second line.
    #[test]
    fn a_parent_cycle_is_refused_and_survived() {
        let mk = |id: &str, parent: Option<&str>| {
            let mut l = Layer::plain(
                id,
                Source::Artifact {
                    rel: "inbox/a.mp4".into(),
                },
                0.0,
                0.0,
                1.0,
            );
            l.parent = parent.map(str::to_string);
            l
        };
        // Authoring: b already rides a, so a may not ride b.
        let sane = vec![mk("a", None), mk("b", Some("a"))];
        assert!(
            parent_would_cycle(&sane, "a", "b"),
            "a->b would close a loop"
        );
        assert!(
            parent_would_cycle(&sane, "a", "a"),
            "self-parenting is a loop"
        );
        assert!(
            !parent_would_cycle(&sane, "b", "a"),
            "b->a is already true and fine"
        );

        // Surviving: a file that already contains a->b->a must still resolve.
        let looped = vec![mk("a", Some("b")), mk("b", Some("a"))];
        let t = resolved_transform(&looped, "a").expect("a cycle resolves, bounded");
        assert!(t.scale.is_finite(), "and yields finite numbers");
    }

    /// 3D position is a pinhole projection, and it is INERT without a camera —
    /// so every comp authored before cameras existed is exactly as flat.
    #[test]
    fn depth_is_inert_without_a_camera_and_shrinks_with_one() {
        let t = Transform {
            x: 100.0,
            y: 50.0,
            scale: 1.0,
        };
        assert_eq!(project(t, 500.0, None), t, "no camera: z does nothing");
        assert_eq!(
            project(t, 0.0, Some(1000.0)),
            t,
            "on the focal plane: identity"
        );

        let far = project(t, 1000.0, Some(1000.0));
        assert_eq!(far.scale, 0.5, "twice the focal length is half the size");
        assert_eq!(far.x, 50.0, "and it pulls toward centre");

        let near = project(t, -500.0, Some(1000.0));
        assert!(near.scale > 1.0, "nearer than the plane is LARGER");

        // At or behind the camera it collapses rather than inverting.
        let behind = project(t, -2000.0, Some(1000.0));
        assert!(
            behind.scale > 0.0 && behind.scale.is_finite(),
            "never negative, never NaN: {behind:?}"
        );
    }

    #[test]
    fn save_load_round_trips_identically() {
        let dir = scratch("roundtrip");
        let comp = sample();
        save(&dir, &comp).unwrap();
        assert_eq!(load(&dir).unwrap(), comp);
        // Crash-safety droppings are cleaned up.
        assert!(!dir.join("comp.json.tmp").exists());
        assert!(!dir.join("comp.json.bak").exists());
        // Second save swaps over the existing file cleanly.
        save(&dir, &comp).unwrap();
        assert_eq!(load(&dir).unwrap(), comp);
    }

    #[test]
    fn saved_json_is_pretty_lf_and_ref_shaped() {
        let dir = scratch("shape");
        save(&dir, &sample()).unwrap();
        let raw = std::fs::read_to_string(dir.join("comp.json")).unwrap();
        assert!(raw.ends_with('\n'));
        assert!(raw.contains("\"kind\": \"artifact\""));
        assert!(raw.contains("\"rel\": \"inbox/a.mp4\""));
        // Refs, never values: no embedded hex/colors/bytes.
        assert!(!raw.contains('#'));
    }

    #[test]
    fn a_newer_schema_version_is_refused_honestly() {
        let dir = scratch("version");
        let mut comp = sample();
        comp.v = SCHEMA_V + 1;
        save(&dir, &comp).unwrap();
        assert!(matches!(load(&dir), Err(CompError::Version(v)) if v == SCHEMA_V + 1));
    }

    #[test]
    fn the_sample_and_defaults_validate_clean() {
        assert_eq!(validate(&sample()), Vec::new());
        assert_eq!(validate(&Composition::default_1080p30()), Vec::new());
    }

    #[test]
    fn a_lottie_source_round_trips_and_validates() {
        let dir = scratch("lottie");
        let comp = Composition {
            v: SCHEMA_V,
            motion_blur: false,
            shutter: 180,
            focal: None,
            fps: 30,
            w: 1920,
            h: 1080,
            tracks: vec![Track {
                id: "main".into(),
                layers: vec![Layer::plain(
                    "lot-1",
                    Source::Lottie {
                        rel: "inbox/title.json".into(),
                    },
                    0.0,
                    0.0,
                    3.0,
                )],
            }],
        };
        assert!(validate(&comp).is_empty(), "a Lottie layer validates");
        save(&dir, &comp).unwrap();
        let raw = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(raw.contains("\"kind\": \"lottie\""), "kebab-cased tag");
        assert_eq!(load(&dir).unwrap(), comp, "round-trips identically");
    }

    #[test]
    fn structural_faults_are_named_not_swallowed() {
        let comp = Composition {
            v: SCHEMA_V,
            motion_blur: false,
            shutter: 180,
            focal: None,
            fps: 0,
            w: 0,
            h: 1080,
            tracks: vec![
                Track {
                    id: "dup".into(),
                    layers: vec![
                        Layer {
                            scale: 0.0,
                            x: f32::NAN,
                            ..Layer::plain(
                                "x",
                                Source::Element { id: "  ".into() },
                                -1.0,
                                -0.5,
                                -0.5,
                            )
                        },
                        Layer::plain(
                            "x",
                            Source::Artifact {
                                rel: "ok.mp4".into(),
                            },
                            0.0,
                            2.0,
                            1.0,
                        ),
                    ],
                },
                Track {
                    id: "dup".into(),
                    layers: vec![],
                },
            ],
        };
        let all = validate(&comp)
            .iter()
            .map(|i| format!("{} {}", i.path, i.message))
            .collect::<Vec<_>>()
            .join(" | ");
        for needle in [
            "fps must be > 0",
            "w and h must be > 0",
            "duplicate track id",
            "duplicate layer id",
            "empty source ref",
            "in_pt must be >= 0",
            "out_pt must be > in_pt",
            "start must be >= 0",
            "scale must be finite and > 0",
            "x/y must be finite",
        ] {
            assert!(
                all.contains(needle),
                "missing finding for {needle:?}: {all}"
            );
        }
    }

    #[test]
    fn duration_is_the_furthest_layer_end() {
        // clip-a: 0..6, clip-b: start 6 + span (9-1)=8 → ends at 14.
        assert!((sample().duration() - 14.0).abs() < 1e-6);
        assert_eq!(Composition::default_1080p30().duration(), 0.0);
    }

    #[test]
    fn the_folder_contract_names_strays_and_transients() {
        let dir = scratch("contract");
        save(&dir, &sample()).unwrap();
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        assert!(folder_issues(&dir).is_empty(), "reserved names are clean");

        std::fs::write(dir.join("notes.txt"), "nope").unwrap();
        std::fs::write(dir.join("comp.json.tmp"), "{}").unwrap();
        let all = folder_issues(&dir)
            .iter()
            .map(|i| i.message.clone())
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(all.contains("\"notes.txt\""), "stray named: {all}");
        assert!(all.contains("leftover transient"), "transient named: {all}");
    }

    /// The L3 back-compat law: a pre-L3 comp.json (no x/y/scale/tint) loads
    /// with the defaults — full-frame, centered, unscaled, untinted.
    #[test]
    fn a_pre_l3_comp_loads_with_transform_defaults() {
        let dir = scratch("backcompat");
        std::fs::create_dir_all(&dir).unwrap();
        let raw = r#"{
  "v": 1, "fps": 30, "w": 1920, "h": 1080,
  "tracks": [{ "id": "main", "layers": [{
    "id": "clip-a",
    "source": { "kind": "artifact", "rel": "inbox/a.mp4" },
    "start": 0.0, "in_pt": 0.0, "out_pt": 6.0
  }]}]
}"#;
        std::fs::write(dir.join(FILE), raw).unwrap();
        let comp = load(&dir).unwrap();
        let layer = &comp.tracks[0].layers[0];
        assert_eq!(layer.x, 0.0);
        assert_eq!(layer.y, 0.0);
        assert_eq!(layer.scale, 1.0);
        assert_eq!(layer.tint, None);
        assert!(validate(&comp).is_empty());
    }

    /// A transformed + tinted overlay layer round-trips identically, its tint
    /// serialized as an rgba VALUE array (never a hex string — the shape law),
    /// and default-valued transforms stay OUT of the JSON (old shape holds).
    #[test]
    fn transform_and_tint_round_trip_as_value_arrays() {
        let dir = scratch("transform");
        let mut comp = sample();
        comp.tracks[0].layers[0].x = -120.0;
        comp.tracks[0].layers[0].y = 40.0;
        comp.tracks[0].layers[0].scale = 0.5;
        comp.tracks[0].layers[0].tint = Some([255, 64, 0, 230]);
        assert!(validate(&comp).is_empty());
        save(&dir, &comp).unwrap();
        let raw = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(raw.contains("\"tint\""), "tint serialized: {raw}");
        assert!(!raw.contains('#'), "values, never hex");
        // The UNTOUCHED second layer keeps the pre-L3 shape on disk.
        assert!(
            !raw.contains("\"scale\": 1.0"),
            "default transform is skipped: {raw}"
        );
        assert_eq!(load(&dir).unwrap(), comp, "round-trips identically");
    }

    /// L4: blend/matte/lut round-trip as kebab data + refs; defaults stay
    /// OFF the disk (a pre-L4 comp is byte-identical), and empty refs are
    /// named faults.
    #[test]
    fn blend_matte_lut_round_trip_and_validate() {
        let dir = scratch("l4");
        let mut comp = sample();
        comp.tracks[0].layers[0].blend = Blend::Screen;
        comp.tracks[0].layers[0].matte = Some("elements/brush-matte-01.mov".into());
        comp.tracks[0].layers[0].lut = Some("elements/cc-basic-1.cube".into());
        assert!(validate(&comp).is_empty());
        save(&dir, &comp).unwrap();
        let raw = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(raw.contains("\"blend\": \"screen\""), "kebab blend: {raw}");
        assert!(raw.contains("\"matte\""), "{raw}");
        // The UNTOUCHED second layer stays pre-L4-shaped on disk.
        assert!(
            !raw.contains("\"blend\": \"normal\""),
            "default skipped: {raw}"
        );
        assert_eq!(load(&dir).unwrap(), comp);

        comp.tracks[0].layers[0].matte = Some("  ".into());
        comp.tracks[0].layers[0].lut = Some("".into());
        let all = validate(&comp)
            .iter()
            .map(|i| i.message.clone())
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(all.contains("matte ref must not be empty"), "{all}");
        assert!(all.contains("lut ref must not be empty"), "{all}");
    }

    /// The operator's requirement, made concrete: two stitched clips persist
    /// as ONE validated, AI-editable comp.json and reload identically.
    #[test]
    fn two_stitched_clips_persist_as_one_validated_comp() {
        let dir = scratch("stitch");
        let comp = sample();
        assert_eq!(comp.tracks[0].layers.len(), 2);
        assert!(validate(&comp).is_empty());
        save(&dir, &comp).unwrap();
        let reloaded = load(&dir).unwrap();
        assert_eq!(reloaded, comp);
        assert!(validate(&reloaded).is_empty());
    }
}
