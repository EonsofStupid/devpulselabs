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
///
/// Styling fields are SPARSE (W3b): absent means "inherit the show's
/// [`Look`]"; present means a deliberate override. Exactly two cascade
/// levels — episode look, then slide — resolved by [`resolve`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slide {
    pub v: u32,
    /// Which registered template renders this document (W2 resolves;
    /// unknown degrades to a labeled plate, never a crash).
    pub template: String,
    /// None = inherit the show look's background. An explicit
    /// `{"kind": "none"}` is the deliberately bare well.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Background>,
    pub blocks: Vec<Block>,
    /// Entrance ref: None = inherit the look; "none" = explicitly
    /// static; "rise" = the entrance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enter: Option<String>,
    /// The beat grammar's reveals (D2): an ORDERED list of block ids
    /// revealed one per step. Step 0 is the SETUP (only unlisted blocks
    /// visible — arriving never pre-reveals); step k shows reveals[0..k].
    /// Empty = a single-step slide; callers never branch.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reveals: Vec<String>,
}

impl Slide {
    /// The deck's step count for this slide: `reveals + 1` (step 0 =
    /// setup). Never zero.
    pub fn step_count(&self) -> usize {
        self.reveals.len() + 1
    }
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
        /// Font pool id (`cl::FONTS`). None = inherit the look's font.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        font: Option<String>,
        /// Fluid type ref, `fs-` stripped: "display", "h3", "eyebrow"…
        /// REQUIRED — size is structural (an eyebrow is not an h1),
        /// never a show-wide default.
        size: String,
        /// Semantic ink: strong | base | muted | subtle | tone.
        /// None = inherit the look's ink.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ink: Option<String>,
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

/// Crash-safe pretty-JSON+LF write: the full content lands in a sibling
/// tmp first, then swaps in (Windows cannot rename over an existing file,
/// so the old file steps aside as `.bak` for the swap and the bak is
/// removed on success). One writer for every document kind.
fn save_json<T: serde::Serialize>(dir: &Path, file: &str, value: &T) -> Result<(), DocError> {
    std::fs::create_dir_all(dir).map_err(DocError::Io)?;
    let target = dir.join(file);
    let tmp = dir.join(format!("{file}.tmp"));
    let bak = dir.join(format!("{file}.bak"));

    let mut json = serde_json::to_string_pretty(value).map_err(DocError::Parse)?;
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

/// Save to `<dir>/slide.json`.
pub fn save(dir: &Path, slide: &Slide) -> Result<(), DocError> {
    save_json(dir, FILE, slide)
}

// W1 allowlists — replaced by registry checks as later passes wire the
// runtime: ambience/motion/enter become authored-motion refs in W2/W3.
const INKS: &[&str] = &["strong", "base", "muted", "subtle", "tone"];
const AMBIENCES: &[&str] = &["pulse", "drift", "rings"];
const BLEND_MOTIONS: &[&str] = &["blend-drift"];
/// "none" is the explicit static override — a slide opting OUT of the
/// show look's entrance (absent would mean inherit, not static).
const ENTERS: &[&str] = &["rise", "none"];

fn font_ok(id: &str) -> bool {
    dpl_coldlight::FONTS.iter().any(|f| f.id == id)
}
fn mood_ok(name: &str) -> bool {
    dpl_coldlight::MOODS.iter().any(|m| m.name == name)
}

/// The Background ref checks — shared by the slide and look validators.
fn validate_background(bg: &Background, at: &str, issues: &mut Vec<Issue>) {
    let mut push = |path: String, message: String| issues.push(Issue { path, message });
    match bg {
        Background::MoodBlend {
            moods,
            motion,
            ambience,
        } => {
            if moods.len() != 2 {
                push(
                    format!("{at}.moods"),
                    format!("mood-blend takes exactly 2 moods, got {}", moods.len()),
                );
            }
            for m in moods {
                if !mood_ok(m) {
                    push(format!("{at}.moods"), format!("unknown mood {m:?}"));
                }
            }
            if !BLEND_MOTIONS.contains(&motion.as_str()) {
                push(
                    format!("{at}.motion"),
                    format!("unknown blend motion {motion:?}"),
                );
            }
            if let Some(a) = ambience {
                if !AMBIENCES.contains(&a.as_str()) {
                    push(format!("{at}.ambience"), format!("unknown ambience {a:?}"));
                }
            }
        }
        Background::Mood { mood } => {
            if !mood_ok(mood) {
                push(format!("{at}.mood"), format!("unknown mood {mood:?}"));
            }
        }
        Background::None => {}
    }
}

/// Name every unknown ref in the document. Empty = clean. Absent styling
/// fields are LEGAL (they inherit the show look) — only present refs are
/// checked.
pub fn validate(slide: &Slide) -> Vec<Issue> {
    let mut issues = Vec::new();

    if let Some(bg) = &slide.background {
        validate_background(bg, "background", &mut issues);
    }

    let size_ok = |name: &str| {
        dpl_coldlight::FLUID_TOKEN_NAMES
            .iter()
            .any(|n| *n == format!("fs-{name}"))
    };
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
                let mut push = |message: String| {
                    issues.push(Issue {
                        path: at.clone(),
                        message,
                    })
                };
                if let Some(font) = font {
                    if !font_ok(font) {
                        push(format!("unknown font {font:?}"));
                    }
                }
                if !size_ok(size) {
                    push(format!("unknown size {size:?} (fluid fs-* ref)"));
                }
                if let Some(ink) = ink {
                    if !INKS.contains(&ink.as_str()) {
                        push(format!("unknown ink {ink:?}"));
                    }
                }
            }
        }
    }

    if let Some(e) = &slide.enter {
        if !ENTERS.contains(&e.as_str()) {
            issues.push(Issue {
                path: "enter".into(),
                message: format!("unknown enter {e:?}"),
            });
        }
    }

    // Reveals (D2): every entry names an existing block, exactly once.
    let mut seen_reveal = std::collections::BTreeSet::new();
    for r in &slide.reveals {
        let exists = slide.blocks.iter().any(|b| {
            let Block::Text { id, .. } = b;
            id == r
        });
        if !exists {
            issues.push(Issue {
                path: "reveals".into(),
                message: format!("reveal names unknown block {r:?}"),
            });
        }
        if !seen_reveal.insert(r.clone()) {
            issues.push(Issue {
                path: "reveals".into(),
                message: format!("duplicate reveal {r:?}"),
            });
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

// ── the EPISODE tier (D1) — a show over slide folders ───────────────────

const EPISODE_FILE: &str = "episode.json";

/// An episode document — the editable truth of `<episode-dir>/episode.json`:
/// an ordered chapter→slide structure over slide SUBFOLDERS (each a Slide
/// document). Refs never values here too: a chapter's slides are subfolder
/// names, resolved to `<episode-dir>/<name>/slide.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Episode {
    pub v: u32,
    pub id: String,
    pub title: String,
    pub chapters: Vec<Chapter>,
}

/// One chapter — a titled ordered run of slide-subfolder names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub slides: Vec<String>,
}

/// One resolved slide in an episode's flat ordered walk — what the deck
/// (D2) steps through. `index` is the episode-global slide position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideRef {
    pub chapter: String,
    pub name: String,
    pub dir: PathBuf,
    pub index: usize,
}

/// Load `<dir>/episode.json`.
pub fn load_episode(dir: &Path) -> Result<Episode, DocError> {
    let raw = std::fs::read_to_string(dir.join(EPISODE_FILE)).map_err(DocError::Io)?;
    let ep: Episode = serde_json::from_str(&raw).map_err(DocError::Parse)?;
    if ep.v > SCHEMA_V {
        return Err(DocError::Version(ep.v));
    }
    Ok(ep)
}

/// Save `<dir>/episode.json` — same crash-safe pretty+LF write as slides.
pub fn save_episode(dir: &Path, ep: &Episode) -> Result<(), DocError> {
    save_json(dir, EPISODE_FILE, ep)
}

/// The flat, ordered slide walk the deck consumes — one SlideRef per slide
/// in chapter order. Does not validate; pair with `validate_episode`.
pub fn episode_slides(dir: &Path, ep: &Episode) -> Vec<SlideRef> {
    let mut out = Vec::new();
    let mut index = 0;
    for chapter in &ep.chapters {
        for name in &chapter.slides {
            out.push(SlideRef {
                chapter: chapter.id.clone(),
                name: name.clone(),
                dir: dir.join(name),
                index,
            });
            index += 1;
        }
    }
    out
}

// The FOLDER CONTRACT (W3b): each folder holds reserved names only, one
// concern per file, so "auto-growing" stays strict — a new concern claims
// a reserved name here, it never squats. The doctor flags everything else.
const SLIDE_RESERVED: &[&str] = &["slide.json", "ink.json", "notes.md", "assets"];
const EPISODE_RESERVED: &[&str] = &["episode.json", "look.json", "assets"];

/// Contract-check one folder's entries: leftover save transients and
/// unreserved names are flagged; `dirs_are_slides` marks subfolders as
/// slide folders (legal in an episode) instead of strays.
fn check_folder_contract(
    dir: &Path,
    at: &str,
    reserved: &[&str],
    dirs_are_slides: bool,
    issues: &mut Vec<Issue>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.path().is_dir();
        if is_dir && (dirs_are_slides || reserved.contains(&name.as_str())) {
            continue;
        }
        if reserved.contains(&name.as_str()) {
            continue;
        }
        let message = if name.ends_with(".tmp") || name.ends_with(".bak") {
            format!("leftover transient {name:?} from an interrupted save")
        } else {
            format!(
                "unexpected {} {name:?} (contract: {})",
                if is_dir { "folder" } else { "file" },
                reserved.join(", ")
            )
        };
        issues.push(Issue {
            path: at.to_string(),
            message,
        });
    }
}

/// Structural doctrine (platform-free, the old doctor's core): duplicate
/// chapter ids, empty episode/chapter, each slide ref resolving to a
/// slide.json that loads AND validates clean (its issues bubble up,
/// path-prefixed), and no slide ref used twice. Plus the W3b contract:
/// the authored look's refs, stray/transient files, and orphan slide
/// folders no chapter references. Show-kind doctrine (chapter-lock,
/// interaction budgets, slide bands) is a later authored-doctrine pass —
/// TODO(doctrine).
pub fn validate_episode(dir: &Path, ep: &Episode) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |path: String, message: String| issues.push(Issue { path, message });

    if ep.chapters.is_empty() {
        push("chapters".into(), "episode has no chapters".into());
    }

    let mut seen_chapter = std::collections::BTreeSet::new();
    let mut seen_slide = std::collections::BTreeSet::new();
    for chapter in &ep.chapters {
        if !seen_chapter.insert(chapter.id.clone()) {
            push(
                format!("chapters.{}", chapter.id),
                "duplicate chapter id".into(),
            );
        }
        if chapter.slides.is_empty() {
            push(
                format!("chapters.{}", chapter.id),
                "chapter has no slides".into(),
            );
        }
        for name in &chapter.slides {
            let at = format!("chapters.{}.{name}", chapter.id);
            if !seen_slide.insert(name.clone()) {
                push(at.clone(), "slide referenced more than once".into());
            }
            let slide_dir = dir.join(name);
            match load(&slide_dir) {
                Ok(slide) => {
                    for sub in validate(&slide) {
                        push(format!("{at} → {}", sub.path), sub.message);
                    }
                }
                Err(e) => push(at, format!("slide does not load: {e}")),
            }
        }
    }

    // The authored look (when present) must itself resolve.
    match load_look(dir) {
        Ok(Some(look)) => {
            for sub in validate_look(&look) {
                push(format!("look.{}", sub.path), sub.message);
            }
        }
        Ok(None) => {}
        Err(e) => push("look".into(), format!("look.json does not load: {e}")),
    }

    // The folder contract, episode level + each referenced slide folder.
    check_folder_contract(dir, "folder", EPISODE_RESERVED, true, &mut issues);
    for chapter in &ep.chapters {
        for name in &chapter.slides {
            let slide_dir = dir.join(name);
            if slide_dir.is_dir() {
                check_folder_contract(
                    &slide_dir,
                    &format!("chapters.{}.{name} → folder", chapter.id),
                    SLIDE_RESERVED,
                    false,
                    &mut issues,
                );
            }
        }
    }

    // Orphans: a subfolder holding a slide.json that no chapter names.
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().join(FILE).is_file() && !seen_slide.contains(&name) {
                issues.push(Issue {
                    path: "folder".into(),
                    message: format!("orphan slide folder {name:?} — not in any chapter"),
                });
            }
        }
    }

    issues
}

// ── the LOOK tier (W3b) — the show's authored trait ─────────────────────

const LOOK_FILE: &str = "look.json";

/// The show-level styling trait — `<episode-dir>/look.json`. Refs only,
/// like every document: a look names registry ids, never values. Slides
/// inherit these unless they carry an explicit override; [`resolve`] is
/// the whole cascade (exactly two levels — this is the slide-master
/// pattern, deliberately without a third tier).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Look {
    pub v: u32,
    /// Default font pool id for every text block.
    pub font: String,
    /// Default semantic ink for every text block.
    pub ink: String,
    pub background: Background,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enter: Option<String>,
}

impl Look {
    /// The built-in default — used wherever no look.json is authored
    /// (standalone seeds, episodes without one). Known-good refs; a test
    /// proves they resolve through the live registries.
    pub fn fallback() -> Self {
        Self {
            v: SCHEMA_V,
            font: "spacegrotesk".into(),
            ink: "base".into(),
            background: Background::MoodBlend {
                moods: vec!["ember-drift".into(), "deep-lagoon".into()],
                motion: "blend-drift".into(),
                ambience: None,
            },
            enter: Some("rise".into()),
        }
    }
}

/// Load `<dir>/look.json`; Ok(None) when none is authored (= fallback).
pub fn load_look(dir: &Path) -> Result<Option<Look>, DocError> {
    let path = dir.join(LOOK_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(DocError::Io)?;
    let look: Look = serde_json::from_str(&raw).map_err(DocError::Parse)?;
    if look.v > SCHEMA_V {
        return Err(DocError::Version(look.v));
    }
    Ok(Some(look))
}

/// Save `<dir>/look.json` — same crash-safe pretty+LF write.
pub fn save_look(dir: &Path, look: &Look) -> Result<(), DocError> {
    save_json(dir, LOOK_FILE, look)
}

/// The look a slide folder resolves against: its parent episode's
/// authored look, else the fallback. (A slide is "in a show" when its
/// PARENT folder holds an episode.json.)
pub fn look_for_slide(slide_dir: &Path) -> Look {
    slide_dir
        .parent()
        .filter(|p| p.join(EPISODE_FILE).is_file())
        .and_then(|p| load_look(p).ok().flatten())
        .unwrap_or_else(Look::fallback)
}

/// THE CASCADE — slide override wins, else the look fills in. Pure;
/// returns a fully-specified document the renderer paints as-is. A
/// fully-specified slide is a fixed point.
pub fn resolve(slide: &Slide, look: &Look) -> Slide {
    let mut out = slide.clone();
    if out.background.is_none() {
        out.background = Some(look.background.clone());
    }
    if out.enter.is_none() {
        out.enter = look.enter.clone();
    }
    for b in &mut out.blocks {
        let Block::Text { font, ink, .. } = b;
        if font.is_none() {
            *font = Some(look.font.clone());
        }
        if ink.is_none() {
            *ink = Some(look.ink.clone());
        }
    }
    out
}

/// Name every unknown ref in a look. Empty = clean.
pub fn validate_look(look: &Look) -> Vec<Issue> {
    let mut issues = Vec::new();
    if !font_ok(&look.font) {
        issues.push(Issue {
            path: "font".into(),
            message: format!("unknown font {:?}", look.font),
        });
    }
    if !INKS.contains(&look.ink.as_str()) {
        issues.push(Issue {
            path: "ink".into(),
            message: format!("unknown ink {:?}", look.ink),
        });
    }
    validate_background(&look.background, "background", &mut issues);
    if let Some(e) = &look.enter {
        if !ENTERS.contains(&e.as_str()) {
            issues.push(Issue {
                path: "enter".into(),
                message: format!("unknown enter {e:?}"),
            });
        }
    }
    issues
}

/// The repo/install seed set: `seeds/slides/`.
pub fn seeds_root() -> Option<PathBuf> {
    find_upward("seeds/slides")
}

/// The seed episodes root: `seeds/episodes/`.
pub fn episodes_root() -> Option<PathBuf> {
    find_upward("seeds/episodes")
}

/// Every seed episode folder holding an episode.json, as (id, dir), sorted.
pub fn list_episodes() -> Vec<(String, PathBuf)> {
    let Some(root) = episodes_root() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut eps: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().join(EPISODE_FILE).is_file())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect();
    eps.sort();
    eps
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

// ── the CONTENT tier (X3) — the operator's real shows ───────────────────
//
// The RATIFIED tree (operator, 2026-07-23): `content/<show>/<episode>/`,
// the show name IS the stream category. Seeds stay pristine templates;
// a new show COPIES a seed into content. Episodes are NOT guides — the
// guide system files through the inbox and only enters an episode via an
// explicit associate (dpl-index's lane).

/// One entry in the seeds+content UNION every picker lists: `show` names
/// the content category, or None for a seed template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeEntry {
    pub show: Option<String>,
    pub id: String,
    pub dir: PathBuf,
}

/// The content root's shows: subfolders holding at least one episode
/// folder. The machine tier (`inbox/`, `artifacts.db*`) is skipped by
/// name; anything else without an episode inside simply isn't a show yet.
pub fn list_shows(content_root: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(content_root) else {
        return Vec::new();
    };
    let mut shows: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let dir = e.path();
            name != "inbox"
                && !name.starts_with("artifacts.db")
                && dir.is_dir()
                && holds_an_episode(&dir)
        })
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect();
    shows.sort();
    shows
}

/// A show is a folder with at least one episode inside — a guide folder
/// (guide.json, no episode.json) never qualifies.
fn holds_an_episode(show_dir: &Path) -> bool {
    std::fs::read_dir(show_dir)
        .map(|entries| {
            entries
                .flatten()
                .any(|e| e.path().join(EPISODE_FILE).is_file())
        })
        .unwrap_or(false)
}

/// Every content episode as (show, episode, dir), sorted.
pub fn list_content_episodes(content_root: &Path) -> Vec<(String, String, PathBuf)> {
    let mut out = Vec::new();
    for (show, show_dir) in list_shows(content_root) {
        let Ok(entries) = std::fs::read_dir(&show_dir) else {
            continue;
        };
        for e in entries.flatten() {
            if e.path().join(EPISODE_FILE).is_file() {
                out.push((
                    show.clone(),
                    e.file_name().to_string_lossy().into_owned(),
                    e.path(),
                ));
            }
        }
    }
    out.sort();
    out
}

/// The UNION every picker lists: content episodes FIRST (they are the
/// point), then the seed templates.
pub fn all_episodes(content_root: &Path) -> Vec<EpisodeEntry> {
    let mut out: Vec<EpisodeEntry> = list_content_episodes(content_root)
        .into_iter()
        .map(|(show, id, dir)| EpisodeEntry {
            show: Some(show),
            id,
            dir,
        })
        .collect();
    out.extend(list_episodes().into_iter().map(|(id, dir)| EpisodeEntry {
        show: None,
        id,
        dir,
    }));
    out
}

/// The kebab name law — the capture metadata law's predicate, restated
/// here like `find_upward`: the document layer must not depend on the
/// capture engine for a character check.
fn kebab_ok(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_'))
}

/// "New show from a seed": copy a seed episode into
/// `content/<show>/<episode>/`. COPY never move; an existing destination
/// is refused; seeds are never written. Creates the episode-level
/// `assets/` home (reserved by the folder contract).
pub fn clone_episode(
    seed_dir: &Path,
    content_root: &Path,
    show: &str,
    episode: &str,
) -> Result<PathBuf, String> {
    if !kebab_ok(show) || !kebab_ok(episode) {
        return Err("show and episode names are kebab-case (a-z, 0-9, -, _)".into());
    }
    if !seed_dir.join(EPISODE_FILE).is_file() {
        return Err(format!("not an episode folder: {}", seed_dir.display()));
    }
    let dest = content_root.join(show).join(episode);
    if dest.exists() {
        return Err(format!("{show}/{episode} already exists — clone refused"));
    }
    copy_dir(seed_dir, &dest)?;
    std::fs::create_dir_all(dest.join("assets")).map_err(|e| format!("assets: {e}"))?;
    Ok(dest)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("copy {}: {e}", to.display()))?;
    let entries = std::fs::read_dir(from).map_err(|e| format!("copy {}: {e}", from.display()))?;
    for entry in entries.flatten() {
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst).map_err(|e| format!("copy {}: {e}", src.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fully specified on purpose — every field `Some` so the round-trip
    /// and validation tests keep their pre-W3b meaning.
    fn sample() -> Slide {
        Slide {
            v: SCHEMA_V,
            template: "title".into(),
            background: Some(Background::MoodBlend {
                moods: vec!["ember-drift".into(), "deep-lagoon".into()],
                motion: "blend-drift".into(),
                ambience: Some("pulse".into()),
            }),
            blocks: vec![Block::Text {
                id: "title".into(),
                text: "The stack, laid bare".into(),
                font: Some("orbitron".into()),
                size: "display".into(),
                ink: Some("strong".into()),
            }],
            enter: Some("rise".into()),
            reveals: Vec::new(),
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
        slide.background = Some(Background::MoodBlend {
            moods: vec!["not-a-mood".into()],
            motion: "warp".into(),
            ambience: Some("strobe".into()),
        });
        slide.blocks = vec![Block::Text {
            id: "t".into(),
            text: "x".into(),
            font: Some("comic-sans".into()),
            size: "gigantic".into(),
            ink: Some("chartreuse".into()),
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

    // ── episode tier (D1) ───────────────────────────────────────────────

    fn ep_scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir()
            .join("dpl-episode-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn write_ok_slide(dir: &Path) {
        save(dir, &sample()).unwrap();
    }

    fn ep_sample() -> Episode {
        Episode {
            v: SCHEMA_V,
            id: "demo".into(),
            title: "Demo show".into(),
            chapters: vec![
                Chapter {
                    id: "intro".into(),
                    title: "Setup".into(),
                    slides: vec!["a".into(), "b".into()],
                },
                Chapter {
                    id: "end".into(),
                    title: "Wrap".into(),
                    slides: vec!["c".into()],
                },
            ],
        }
    }

    #[test]
    fn episode_round_trips_and_flattens_in_authored_order() {
        let dir = ep_scratch("flat");
        for s in ["a", "b", "c"] {
            write_ok_slide(&dir.join(s));
        }
        let ep = ep_sample();
        save_episode(&dir, &ep).unwrap();
        assert_eq!(load_episode(&dir).unwrap(), ep);
        assert!(!dir.join("episode.json.tmp").exists());

        let refs = episode_slides(&dir, &ep);
        let order: Vec<&str> = refs.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(order, ["a", "b", "c"]);
        assert_eq!(refs[2].index, 2);
        assert_eq!(refs[0].chapter, "intro");
        assert_eq!(refs[2].chapter, "end");
        assert!(validate_episode(&dir, &ep).is_empty());
    }

    #[test]
    fn a_newer_episode_version_is_refused() {
        let dir = ep_scratch("epver");
        let mut ep = ep_sample();
        ep.v = SCHEMA_V + 1;
        save_episode(&dir, &ep).unwrap();
        assert!(matches!(load_episode(&dir), Err(DocError::Version(_))));
    }

    #[test]
    fn episode_doctrine_names_dangling_dup_and_empty() {
        let dir = ep_scratch("doctrine");
        write_ok_slide(&dir.join("a"));
        // 'b' folder intentionally missing → dangling ref.
        let ep = Episode {
            v: SCHEMA_V,
            id: "bad".into(),
            title: "Bad".into(),
            chapters: vec![
                Chapter {
                    id: "one".into(),
                    title: "One".into(),
                    slides: vec!["a".into(), "b".into(), "a".into()],
                },
                Chapter {
                    id: "one".into(),
                    title: "Dup".into(),
                    slides: vec![],
                },
            ],
        };
        let all = validate_episode(&dir, &ep)
            .iter()
            .map(|i| format!("{} {}", i.path, i.message))
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(all.contains("duplicate chapter id"), "{all}");
        assert!(all.contains("does not load"), "dangling: {all}");
        assert!(
            all.contains("referenced more than once"),
            "dup slide: {all}"
        );
        assert!(all.contains("no slides"), "empty chapter: {all}");
    }

    #[test]
    fn a_nested_bad_slide_bubbles_up_path_prefixed() {
        let dir = ep_scratch("nested");
        let mut bad = sample();
        bad.blocks[0] = Block::Text {
            id: "t".into(),
            text: "x".into(),
            font: Some("comic-sans".into()),
            size: "display".into(),
            ink: Some("strong".into()),
        };
        save(&dir.join("a"), &bad).unwrap();
        let ep = Episode {
            v: SCHEMA_V,
            id: "n".into(),
            title: "N".into(),
            chapters: vec![Chapter {
                id: "c".into(),
                title: "C".into(),
                slides: vec!["a".into()],
            }],
        };
        let issues = validate_episode(&dir, &ep);
        assert!(
            issues
                .iter()
                .any(|i| i.path.contains("chapters.c.a →") && i.message.contains("comic-sans")),
            "expected path-prefixed nested finding: {issues:?}"
        );
    }

    #[test]
    fn the_seed_episode_loads_validates_and_flattens() {
        let eps = list_episodes();
        assert!(!eps.is_empty(), "expected a seed episode");
        for (id, dir) in eps {
            let ep = load_episode(&dir).unwrap_or_else(|e| panic!("episode {id}: {e}"));
            let issues = validate_episode(&dir, &ep);
            assert!(issues.is_empty(), "episode {id} doctrine: {issues:?}");
            assert!(
                !episode_slides(&dir, &ep).is_empty(),
                "episode {id} has no slides"
            );
        }
    }

    // ── the look tier + cascade (W3b) ───────────────────────────────────

    #[test]
    fn the_fallback_look_resolves_through_the_live_registries() {
        assert_eq!(validate_look(&Look::fallback()), Vec::new());
    }

    #[test]
    fn look_round_trips_and_absent_look_is_ok_none() {
        let dir = ep_scratch("look");
        assert!(load_look(&dir).unwrap().is_none(), "no folder yet");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(load_look(&dir).unwrap().is_none(), "no look.json");
        let look = Look::fallback();
        save_look(&dir, &look).unwrap();
        assert_eq!(load_look(&dir).unwrap(), Some(look));
        assert!(!dir.join("look.json.tmp").exists());
        assert!(!dir.join("look.json.bak").exists());
    }

    #[test]
    fn resolve_prefers_the_slide_override_and_fills_the_rest() {
        let look = Look::fallback();
        let sparse = Slide {
            v: SCHEMA_V,
            template: "title".into(),
            background: None,
            blocks: vec![
                Block::Text {
                    id: "a".into(),
                    text: "override".into(),
                    font: Some("orbitron".into()),
                    size: "h1".into(),
                    ink: None,
                },
                Block::Text {
                    id: "b".into(),
                    text: "inherit".into(),
                    font: None,
                    size: "body".into(),
                    ink: None,
                },
            ],
            enter: None,
            reveals: Vec::new(),
        };
        let r = resolve(&sparse, &look);
        assert_eq!(r.background, Some(look.background.clone()));
        assert_eq!(r.enter, look.enter);
        let Block::Text { font, ink, .. } = &r.blocks[0];
        assert_eq!(font.as_deref(), Some("orbitron"), "the override wins");
        assert_eq!(ink.as_deref(), Some(look.ink.as_str()));
        let Block::Text { font, .. } = &r.blocks[1];
        assert_eq!(font.as_deref(), Some(look.font.as_str()), "inherited");
        // A fully-specified slide is a fixed point.
        assert_eq!(resolve(&sample(), &look), sample());
        // The resolved sparse slide validates clean.
        assert_eq!(validate(&r), Vec::new());
    }

    #[test]
    fn a_sparse_slide_saves_without_the_absent_fields() {
        let dir = scratch("sparse");
        let sparse = Slide {
            v: SCHEMA_V,
            template: "title".into(),
            background: None,
            blocks: vec![Block::Text {
                id: "t".into(),
                text: "words only".into(),
                font: None,
                size: "h2".into(),
                ink: None,
            }],
            enter: None,
            reveals: Vec::new(),
        };
        save(&dir, &sparse).unwrap();
        let raw = std::fs::read_to_string(dir.join("slide.json")).unwrap();
        for absent in ["\"background\"", "\"font\"", "\"ink\"", "\"enter\""] {
            assert!(!raw.contains(absent), "{absent} must be absent: {raw}");
        }
        assert_eq!(load(&dir).unwrap(), sparse);
        // Absent styling is legal — the validator has nothing to say.
        assert_eq!(validate(&sparse), Vec::new());
    }

    #[test]
    fn a_v1_fully_specified_file_still_loads() {
        // The pre-W3b shape: every styling field present. No version bump.
        let dir = scratch("compat");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("slide.json"),
            r#"{ "v": 1, "template": "title",
                 "background": { "kind": "mood-blend",
                   "moods": ["ember-drift", "deep-lagoon"],
                   "motion": "blend-drift" },
                 "blocks": [ { "kind": "text", "id": "t", "text": "old",
                   "font": "orbitron", "size": "h1", "ink": "strong" } ],
                 "enter": "rise" }"#,
        )
        .unwrap();
        let slide = load(&dir).unwrap();
        let Block::Text { font, ink, .. } = &slide.blocks[0];
        assert_eq!(font.as_deref(), Some("orbitron"));
        assert_eq!(ink.as_deref(), Some("strong"));
        assert!(slide.background.is_some());
        assert_eq!(validate(&slide), Vec::new());
    }

    #[test]
    fn the_folder_contract_names_strays_orphans_and_transients() {
        let dir = ep_scratch("contract");
        for s in ["a", "b", "c"] {
            write_ok_slide(&dir.join(s));
        }
        let ep = ep_sample();
        save_episode(&dir, &ep).unwrap();
        assert!(validate_episode(&dir, &ep).is_empty(), "clean to start");

        // A stray in a slide folder, a transient, an episode-level stray,
        // and an orphan slide folder no chapter names.
        std::fs::write(dir.join("a").join("styles.css"), "nope").unwrap();
        std::fs::write(dir.join("b").join("slide.json.tmp"), "{}").unwrap();
        std::fs::write(dir.join("readme.txt"), "hi").unwrap();
        write_ok_slide(&dir.join("zz-unused"));
        // Reserved names are NOT strays.
        std::fs::write(dir.join("a").join("notes.md"), "presenter notes").unwrap();
        save_look(&dir, &Look::fallback()).unwrap();

        let all = validate_episode(&dir, &ep)
            .iter()
            .map(|i| format!("{} {}", i.path, i.message))
            .collect::<Vec<_>>()
            .join(" | ");
        // Flagged names are quoted ({name:?}); the contract enumeration in
        // the message is not — assert on the quoted form.
        assert!(all.contains("\"styles.css\""), "slide stray: {all}");
        assert!(all.contains("leftover transient"), "transient: {all}");
        assert!(all.contains("\"readme.txt\""), "episode stray: {all}");
        assert!(all.contains("orphan slide folder"), "orphan: {all}");
        assert!(!all.contains("\"notes.md\""), "reserved flagged: {all}");
        assert!(
            !all.contains("\"look.json\""),
            "look flagged as stray: {all}"
        );
    }

    // ── reveals (D2) ────────────────────────────────────────────────────

    #[test]
    fn reveals_validate_against_block_ids_and_count_steps() {
        let mut slide = sample();
        assert_eq!(slide.step_count(), 1, "no reveals = single step");
        slide.reveals = vec!["title".into()];
        assert_eq!(slide.step_count(), 2);
        assert_eq!(validate(&slide), Vec::new(), "a real block id is clean");

        slide.reveals = vec!["ghost".into(), "title".into(), "title".into()];
        let all = validate(&slide)
            .iter()
            .map(|i| i.message.clone())
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(all.contains("unknown block \"ghost\""), "{all}");
        assert!(all.contains("duplicate reveal \"title\""), "{all}");
    }

    #[test]
    fn empty_reveals_stay_absent_on_disk() {
        let dir = scratch("reveals");
        save(&dir, &sample()).unwrap();
        let raw = std::fs::read_to_string(dir.join("slide.json")).unwrap();
        assert!(
            !raw.contains("\"reveals\""),
            "empty reveals crept in: {raw}"
        );
    }

    #[test]
    fn a_bad_look_ref_is_named_via_the_episode_doctor() {
        let dir = ep_scratch("badlook");
        for s in ["a", "b", "c"] {
            write_ok_slide(&dir.join(s));
        }
        let ep = ep_sample();
        save_episode(&dir, &ep).unwrap();
        let mut look = Look::fallback();
        look.font = "comic-sans".into();
        save_look(&dir, &look).unwrap();
        let issues = validate_episode(&dir, &ep);
        assert!(
            issues
                .iter()
                .any(|i| i.path == "look.font" && i.message.contains("comic-sans")),
            "expected the look's bad font named: {issues:?}"
        );
    }

    // ── the content tier (X3) ───────────────────────────────────────────

    /// A scratch pair: a seed episode OUTSIDE the content root (so the
    /// tree listings never see it) and an empty content root.
    fn content_scratch(name: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir()
            .join("dpl-content-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let seed = base.join("seed-src");
        for s in ["a", "b", "c"] {
            write_ok_slide(&seed.join(s));
        }
        save_episode(&seed, &ep_sample()).unwrap();
        save_look(&seed, &Look::fallback()).unwrap();
        (seed, base.join("content"))
    }

    #[test]
    fn clone_copies_the_seed_never_moves_and_refuses_repeats() {
        let (seed, content) = content_scratch("clone");
        let dest = clone_episode(&seed, &content, "devpulse-news", "ep1").unwrap();
        assert_eq!(dest, content.join("devpulse-news").join("ep1"));
        assert!(dest.join("episode.json").is_file());
        assert!(dest.join("look.json").is_file());
        assert!(dest.join("a").join("slide.json").is_file());
        assert!(dest.join("assets").is_dir(), "the assets home is created");
        // The seed is untouched — copy, never move.
        assert!(seed.join("episode.json").is_file());
        assert!(seed.join("a").join("slide.json").is_file());
        // The clone validates clean under the contract (assets/ reserved).
        let ep = load_episode(&dest).unwrap();
        assert_eq!(validate_episode(&dest, &ep), Vec::new());
        // An existing destination is refused, and nothing was clobbered.
        let err = clone_episode(&seed, &content, "devpulse-news", "ep1").unwrap_err();
        assert!(err.contains("refused"), "{err}");
        // The kebab law names bad show/episode names.
        assert!(clone_episode(&seed, &content, "DevPULSE News", "ep2").is_err());
        assert!(clone_episode(&seed, &content, "effingai", "Ep Two").is_err());
        // A non-episode source is refused.
        assert!(clone_episode(&seed.join("a"), &content, "effingai", "ep2").is_err());
    }

    #[test]
    fn the_content_tree_lists_shows_and_skips_the_machine_tier() {
        let (seed, content) = content_scratch("tree");
        clone_episode(&seed, &content, "devpulse-news", "ep1").unwrap();
        clone_episode(&seed, &content, "devpulse-news", "ep2").unwrap();
        clone_episode(&seed, &content, "effingai", "pilot").unwrap();
        // The machine tier and non-show folders never list.
        std::fs::create_dir_all(content.join("inbox")).unwrap();
        std::fs::write(content.join("inbox").join("pix.png"), b"img").unwrap();
        std::fs::write(content.join("artifacts.db"), b"db").unwrap();
        std::fs::create_dir_all(content.join("empty-folder")).unwrap();
        // A guide folder inside a show is NOT an episode.
        let guide = content.join("devpulse-news").join("guide-20260723");
        std::fs::create_dir_all(&guide).unwrap();
        std::fs::write(guide.join("guide.json"), "{}").unwrap();

        let shows: Vec<String> = list_shows(&content).into_iter().map(|(s, _)| s).collect();
        assert_eq!(shows, ["devpulse-news", "effingai"]);
        let eps: Vec<(String, String)> = list_content_episodes(&content)
            .into_iter()
            .map(|(s, e, _)| (s, e))
            .collect();
        assert_eq!(
            eps,
            [
                ("devpulse-news".into(), "ep1".into()),
                ("devpulse-news".into(), "ep2".into()),
                ("effingai".to_string(), "pilot".to_string()),
            ]
        );
    }

    #[test]
    fn the_union_lists_content_first_then_the_seed_templates() {
        let (seed, content) = content_scratch("union");
        clone_episode(&seed, &content, "eons-of-stupidity", "ep1").unwrap();
        let all = all_episodes(&content);
        assert!(
            all.first().is_some_and(|e| e.show.is_some()),
            "content leads: {all:?}"
        );
        let content_count = all.iter().filter(|e| e.show.is_some()).count();
        assert_eq!(content_count, 1);
        assert!(
            all.iter().any(|e| e.show.is_none()),
            "the seed templates still list: {all:?}"
        );
        assert!(
            all.iter().take(content_count).all(|e| e.show.is_some()),
            "no template sorts above content: {all:?}"
        );
    }
}
