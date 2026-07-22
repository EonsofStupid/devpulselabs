//! The DOCUMENT layer (authoring architecture, layer 3): each slide is its
//! own FOLDER holding a colocated `slide.json` — structure + content with
//! style REFS, NEVER VALUES. A slide names registry ids ("orbitron",
//! "ember-drift", "display"); it never contains a hex color, a pixel size,
//! or a font path. The editor (W4) is a view over this data: it swaps a
//! ref and saves. Pure data — no egui in this crate.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Current schema version. Bump on breaking shape changes; `load` refuses
/// newer versions honestly instead of misreading them.
pub const SCHEMA_V: u32 = 1;

/// One slide document — the editable truth of `<slide-dir>/slide.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slide {
    pub v: u32,
    /// Which registered template renders this document (W2 resolves;
    /// unknown degrades to a labeled plate, never a crash).
    pub template: String,
    pub background: Background,
    pub blocks: Vec<Block>,
    /// Entrance motion preset ref (e.g. "rise"). None = static.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enter: Option<String>,
}

/// The slide's backdrop — the operator's tap-the-background target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Background {
    /// TWO moods, gradients crossfaded + panned by an authored motion,
    /// with an optional ambience (pulse/drift/rings) layered above.
    MoodBlend {
        moods: Vec<String>,
        motion: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ambience: Option<String>,
    },
    /// One mood, static gradient.
    Mood { mood: String },
    /// The bare well.
    None,
}

/// One content block. Additive by design — new kinds join as W-passes
/// need them (media lands with the first media seed, W2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Block {
    Text {
        id: String,
        text: String,
        /// Font pool id (`cl::FONTS`).
        font: String,
        /// Fluid type ref, `fs-` stripped: "display", "h3", "eyebrow"…
        size: String,
        /// Semantic ink: strong | base | muted | subtle | tone.
        ink: String,
    },
}

/// A validation finding: the path names WHERE, the message WHAT. Findings
/// are warnings — the renderer degrades unknown refs to labeled plates
/// (never a crash); the validator exists so the doctor can NAME them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub path: String,
    pub message: String,
}

#[derive(Debug)]
pub enum DocError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    /// The file's `v` is newer than this build understands.
    Version(u32),
}

impl std::fmt::Display for DocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocError::Io(e) => write!(f, "io: {e}"),
            DocError::Parse(e) => write!(f, "parse: {e}"),
            DocError::Version(v) => {
                write!(f, "slide.json is schema v{v}; this build reads v{SCHEMA_V}")
            }
        }
    }
}

impl std::error::Error for DocError {}

const FILE: &str = "slide.json";

/// Load `<dir>/slide.json`.
pub fn load(dir: &Path) -> Result<Slide, DocError> {
    let raw = std::fs::read_to_string(dir.join(FILE)).map_err(DocError::Io)?;
    let slide: Slide = serde_json::from_str(&raw).map_err(DocError::Parse)?;
    if slide.v > SCHEMA_V {
        return Err(DocError::Version(slide.v));
    }
    Ok(slide)
}

/// Save to `<dir>/slide.json` — pretty JSON + trailing LF, crash-safe:
/// the full content lands in a sibling tmp first, then swaps in (Windows
/// cannot rename over an existing file, so the old file steps aside as
/// `.bak` for the swap and the bak is removed on success).
pub fn save(dir: &Path, slide: &Slide) -> Result<(), DocError> {
    std::fs::create_dir_all(dir).map_err(DocError::Io)?;
    let target = dir.join(FILE);
    let tmp = dir.join("slide.json.tmp");
    let bak = dir.join("slide.json.bak");

    let mut json = serde_json::to_string_pretty(slide).map_err(DocError::Parse)?;
    json.push('\n');
    std::fs::write(&tmp, json).map_err(DocError::Io)?;

    if target.exists() {
        let _ = std::fs::remove_file(&bak);
        std::fs::rename(&target, &bak).map_err(DocError::Io)?;
    }
    std::fs::rename(&tmp, &target).map_err(DocError::Io)?;
    let _ = std::fs::remove_file(&bak);
    Ok(())
}

// W1 allowlists — replaced by registry checks as later passes wire the
// runtime: ambience/motion/enter become authored-motion refs in W2/W3.
const INKS: &[&str] = &["strong", "base", "muted", "subtle", "tone"];
const AMBIENCES: &[&str] = &["pulse", "drift", "rings"];
const BLEND_MOTIONS: &[&str] = &["blend-drift"];
const ENTERS: &[&str] = &["rise"];

/// Name every unknown ref in the document. Empty = clean.
pub fn validate(slide: &Slide) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |path: &str, message: String| {
        issues.push(Issue {
            path: path.to_string(),
            message,
        })
    };

    let font_ok = |id: &str| dpl_coldlight::FONTS.iter().any(|f| f.id == id);
    let mood_ok = |name: &str| dpl_coldlight::MOODS.iter().any(|m| m.name == name);
    let size_ok = |name: &str| {
        dpl_coldlight::FLUID_TOKEN_NAMES
            .iter()
            .any(|n| *n == format!("fs-{name}"))
    };

    match &slide.background {
        Background::MoodBlend {
            moods,
            motion,
            ambience,
        } => {
            if moods.len() != 2 {
                push(
                    "background.moods",
                    format!("mood-blend takes exactly 2 moods, got {}", moods.len()),
                );
            }
            for m in moods {
                if !mood_ok(m) {
                    push("background.moods", format!("unknown mood {m:?}"));
                }
            }
            if !BLEND_MOTIONS.contains(&motion.as_str()) {
                push(
                    "background.motion",
                    format!("unknown blend motion {motion:?}"),
                );
            }
            if let Some(a) = ambience {
                if !AMBIENCES.contains(&a.as_str()) {
                    push("background.ambience", format!("unknown ambience {a:?}"));
                }
            }
        }
        Background::Mood { mood } => {
            if !mood_ok(mood) {
                push("background.mood", format!("unknown mood {mood:?}"));
            }
        }
        Background::None => {}
    }

    for block in &slide.blocks {
        match block {
            Block::Text {
                id,
                font,
                size,
                ink,
                ..
            } => {
                let at = format!("blocks.{id}");
                if !font_ok(font) {
                    push(&at, format!("unknown font {font:?}"));
                }
                if !size_ok(size) {
                    push(&at, format!("unknown size {size:?} (fluid fs-* ref)"));
                }
                if !INKS.contains(&ink.as_str()) {
                    push(&at, format!("unknown ink {ink:?}"));
                }
            }
        }
    }

    if let Some(e) = &slide.enter {
        if !ENTERS.contains(&e.as_str()) {
            push("enter", format!("unknown enter {e:?}"));
        }
    }

    issues
}

/// Walk from the exe's directory upward, then from the CWD upward, for a
/// relative path — the same discovery pattern the asset loaders use
/// (deliberately reimplemented: the document layer must not depend on the
/// video crate for a path walk).
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

/// The repo/install seed set: `seeds/slides/`.
pub fn seeds_root() -> Option<PathBuf> {
    find_upward("seeds/slides")
}

/// Every seed folder holding a slide.json, as (id, dir), sorted by id.
pub fn list_seeds() -> Vec<(String, PathBuf)> {
    let Some(root) = seeds_root() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut seeds: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().join(FILE).is_file())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect();
    seeds.sort();
    seeds
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Slide {
        Slide {
            v: SCHEMA_V,
            template: "title".into(),
            background: Background::MoodBlend {
                moods: vec!["ember-drift".into(), "deep-lagoon".into()],
                motion: "blend-drift".into(),
                ambience: Some("pulse".into()),
            },
            blocks: vec![Block::Text {
                id: "title".into(),
                text: "The stack, laid bare".into(),
                font: "orbitron".into(),
                size: "display".into(),
                ink: "strong".into(),
            }],
            enter: Some("rise".into()),
        }
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("dpl-doc-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn save_load_round_trips_identically() {
        let dir = scratch("roundtrip");
        let slide = sample();
        save(&dir, &slide).unwrap();
        assert_eq!(load(&dir).unwrap(), slide);
        // Crash-safety droppings are cleaned up.
        assert!(!dir.join("slide.json.tmp").exists());
        assert!(!dir.join("slide.json.bak").exists());
        // Second save swaps over the existing file cleanly.
        save(&dir, &slide).unwrap();
        assert_eq!(load(&dir).unwrap(), slide);
    }

    #[test]
    fn saved_json_is_pretty_lf_and_ref_shaped() {
        let dir = scratch("shape");
        save(&dir, &sample()).unwrap();
        let raw = std::fs::read_to_string(dir.join("slide.json")).unwrap();
        assert!(raw.ends_with('\n'));
        assert!(raw.contains("\"kind\": \"mood-blend\""));
        assert!(raw.contains("\"font\": \"orbitron\""));
        // Refs, never values: no hex colors, no pixel numbers for style.
        assert!(!raw.contains('#'));
    }

    #[test]
    fn a_newer_schema_version_is_refused_honestly() {
        let dir = scratch("version");
        let mut slide = sample();
        slide.v = SCHEMA_V + 1;
        save(&dir, &slide).unwrap();
        assert!(matches!(load(&dir), Err(DocError::Version(v)) if v == SCHEMA_V + 1));
    }

    #[test]
    fn the_sample_validates_clean_against_the_real_registries() {
        assert_eq!(validate(&sample()), Vec::new());
    }

    #[test]
    fn unknown_refs_are_named_not_swallowed() {
        let mut slide = sample();
        slide.background = Background::MoodBlend {
            moods: vec!["not-a-mood".into()],
            motion: "warp".into(),
            ambience: Some("strobe".into()),
        };
        slide.blocks = vec![Block::Text {
            id: "t".into(),
            text: "x".into(),
            font: "comic-sans".into(),
            size: "gigantic".into(),
            ink: "chartreuse".into(),
        }];
        slide.enter = Some("teleport".into());
        let issues = validate(&slide);
        let all = issues
            .iter()
            .map(|i| i.message.as_str())
            .collect::<Vec<_>>()
            .join(" | ");
        for needle in [
            "exactly 2 moods",
            "not-a-mood",
            "warp",
            "strobe",
            "comic-sans",
            "gigantic",
            "chartreuse",
            "teleport",
        ] {
            assert!(all.contains(needle), "missing finding for {needle}: {all}");
        }
    }

    #[test]
    fn every_seed_loads_and_validates_clean() {
        let seeds = list_seeds();
        assert!(
            seeds.len() >= 4,
            "expected the seed set, found {}: {seeds:?}",
            seeds.len()
        );
        for (id, dir) in seeds {
            let slide = load(&dir).unwrap_or_else(|e| panic!("seed {id}: {e}"));
            let issues = validate(&slide);
            assert!(issues.is_empty(), "seed {id} has unknown refs: {issues:?}");
        }
    }
}
