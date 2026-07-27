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
/// trim WITHIN the source. Transform keyframes + effect refs are a later
/// pass — a bare layer is a straight cut, which is all stitching needs.
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
}

impl Layer {
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
            fps: 30,
            w: 1920,
            h: 1080,
            tracks: vec![Track {
                id: "main".into(),
                layers: vec![
                    Layer {
                        id: "clip-a".into(),
                        source: Source::Artifact {
                            rel: "inbox/a.mp4".into(),
                        },
                        start: 0.0,
                        in_pt: 0.0,
                        out_pt: 6.0,
                    },
                    Layer {
                        id: "clip-b".into(),
                        source: Source::Artifact {
                            rel: "inbox/b.mp4".into(),
                        },
                        start: 6.0,
                        in_pt: 1.0,
                        out_pt: 9.0,
                    },
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
            fps: 30,
            w: 1920,
            h: 1080,
            tracks: vec![Track {
                id: "main".into(),
                layers: vec![Layer {
                    id: "lot-1".into(),
                    source: Source::Lottie {
                        rel: "inbox/title.json".into(),
                    },
                    start: 0.0,
                    in_pt: 0.0,
                    out_pt: 3.0,
                }],
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
            fps: 0,
            w: 0,
            h: 1080,
            tracks: vec![
                Track {
                    id: "dup".into(),
                    layers: vec![
                        Layer {
                            id: "x".into(),
                            source: Source::Element { id: "  ".into() },
                            start: -1.0,
                            in_pt: -0.5,
                            out_pt: -0.5,
                        },
                        Layer {
                            id: "x".into(),
                            source: Source::Artifact {
                                rel: "ok.mp4".into(),
                            },
                            start: 0.0,
                            in_pt: 2.0,
                            out_pt: 1.0,
                        },
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
