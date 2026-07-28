//! The WORKFLOW engine (WF1, D3) — a workflow is a typed, declarative
//! document executed by the app, never a bespoke code path. This crate owns
//! the documents (`WorkflowDoc`, the slotted `StoryboardDoc`), the registry
//! census, validation, the executor, and an honest dry-run; the app
//! implements [`StageRunner`] to wire real minting, Imagine, comp save, and
//! render. Pure data + sequencing: no egui, no fabric, no subprocesses.
//!
//! Forward-compat (WF2 is the graph): v1 is deliberately the LINEAR
//! degenerate case of a stage graph — every stage already carries a unique
//! `id`, and order-in-vec is the only edge. v2 grows explicit edges and a
//! two-form split (panel layout vs executor plan) under the same
//! newer-schema-refusal law that guards `dpl_doc` and `dpl_comp`.
//!
//! Two surfaces, one engine (§3a of the WF1 plan): a workflow DECLARES which
//! side of the two-libraries law its artifacts belong to (`Surface`), and
//! the hubs mount accordingly — the engine makes no hub assumptions.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Current schema version for both documents. Bump on breaking shape
/// changes; `load` refuses a newer version honestly instead of misreading.
pub const SCHEMA_V: u32 = 1;

/// The default on-timeline span for an assembled layer (seconds) — the W7
/// still-hold convention; real durations are a later refinement.
pub const DEFAULT_SPAN: f32 = 5.0;

// ── the workflow document ───────────────────────────────────────────────

/// One workflow — a named, versioned, ordered run of typed stages. A
/// workflow is an INGREDIENT (D6): registered in [`workflows`], persisted
/// like any document, never a one-off code path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowDoc {
    pub v: u32,
    /// Kebab-case identity — the `--run-workflow <id>` handle.
    pub id: String,
    pub label: String,
    /// One line — what this workflow produces, in plain words.
    pub blurb: String,
    /// Which side of the two-libraries law the artifacts belong to; the
    /// hubs mount by this, the engine never branches on it.
    pub surface: Surface,
    pub stages: Vec<Stage>,
}

/// The artifact side a workflow declares (two-libraries law): WiredFRONT =
/// raw media (clips, stitch, post), Workbench = slides/episodes (ColdLight).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Surface {
    Wiredfront,
    Workbench,
}

/// One stage — a unique id plus its typed kind. The id is what jobs facts,
/// reports, and (in WF2) graph edges name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stage {
    pub id: String,
    #[serde(flatten)]
    pub kind: StageKind,
}

/// The v1 stage census — exactly what the storyboarding walking skeleton
/// needs. New kinds join HERE (and in every exhaustive match) as later
/// passes need them; nothing else enumerates stages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum StageKind {
    /// Mint the tailored slotted storyboard from this stage's typed beats
    /// (the blueprint's tailoring inputs — §3a: tailored at mint).
    Outline { beats: Vec<Beat> },
    /// Mint one text slide per storyboard slot from a registered template
    /// id, filling each slot with the minted slide ref.
    SlideFromBeat { template: String },
    /// Optionally generate one image per slot (ADR-011): runs ONLY inside
    /// an operator-started run, skips honestly without a key, and a
    /// dry-run never spends.
    GenerateImage,
    /// Assemble the filled slots' artifacts into a comp document.
    AssembleComp { fps: u32, w: u32, h: u32 },
    /// Render the assembled comp through the existing stitch path.
    Render,
}

impl StageKind {
    /// The kind's kebab label — report rows and jobs labels speak this.
    pub fn label(&self) -> &'static str {
        match self {
            StageKind::Outline { .. } => "outline",
            StageKind::SlideFromBeat { .. } => "slide-from-beat",
            StageKind::GenerateImage => "generate-image",
            StageKind::AssembleComp { .. } => "assemble-comp",
            StageKind::Render => "render",
        }
    }
}

/// One authored beat — the tailoring unit `Outline` mints a slot from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Beat {
    pub id: String,
    /// The beat's role in the arc: hook, context, beat, cta…
    pub role: String,
    pub title: String,
    /// What this beat needs — human- AND agent-readable.
    pub brief: String,
}

// ── the slotted storyboard ──────────────────────────────────────────────

/// The minted, project-specific storyboard: slots whose fill state is the
/// machine-legible truth ("slots remaining" is a DERIVED QUERY, never
/// prose). Minted by `Outline`, filled by later stages or by the operator
/// associating raw content in (§3a).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryboardDoc {
    pub v: u32,
    /// The workflow that minted this board.
    pub workflow: String,
    /// Optional association: an existing show/episode this board rides
    /// (the W10a place seam associates; this ref never creates).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub episode: Option<String>,
    pub slots: Vec<Slot>,
}

impl StoryboardDoc {
    /// The slots still empty — what the agent (and the panel) ask first.
    pub fn remaining(&self) -> Vec<&Slot> {
        self.slots
            .iter()
            .filter(|s| matches!(s.fill, SlotFill::Empty))
            .collect()
    }

    /// The slots carrying a fill.
    pub fn filled(&self) -> Vec<&Slot> {
        self.slots
            .iter()
            .filter(|s| !matches!(s.fill, SlotFill::Empty))
            .collect()
    }
}

/// One slot — brief says what it needs; fill says what it holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Slot {
    pub id: String,
    pub role: String,
    pub brief: String,
    /// Named constraints the fill must honor (platform, tone, length…).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<String>,
    pub fill: SlotFill,
}

/// A slot's typed fill state — REFS, never values (the comp/doc law).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum SlotFill {
    Empty,
    /// A content-relative media artifact (raw-media side).
    Artifact {
        rel: String,
    },
    /// A content-relative slide folder (ColdLight side).
    Slide {
        rel: String,
    },
    /// An unsaved Imagine draft id (ADR-011 cache custody).
    Draft {
        id: String,
    },
}

/// Mint the tailored storyboard from an `Outline` stage's beats — one slot
/// per beat, all empty. Pure: the caller decides where (and whether) it
/// lands on disk.
pub fn mint_storyboard(workflow: &str, beats: &[Beat]) -> StoryboardDoc {
    StoryboardDoc {
        v: SCHEMA_V,
        workflow: workflow.to_string(),
        show: None,
        episode: None,
        slots: beats
            .iter()
            .map(|b| Slot {
                id: b.id.clone(),
                role: b.role.clone(),
                brief: format!("{} — {}", b.title, b.brief),
                constraints: Vec::new(),
                fill: SlotFill::Empty,
            })
            .collect(),
    }
}

/// The slide document for one slot — template + title/brief text blocks
/// (the deck's own proven text-slide shape). Pure value; the runner writes.
pub fn slide_for_slot(template: &str, slot: &Slot) -> dpl_doc::Slide {
    let (title, brief) = match slot.brief.split_once(" — ") {
        Some((t, b)) => (t.to_string(), b.to_string()),
        None => (slot.brief.clone(), String::new()),
    };
    let mut blocks = vec![dpl_doc::Block::Text {
        id: "title".into(),
        text: title,
        font: None,
        size: "h1".into(),
        ink: None,
    }];
    if !brief.is_empty() {
        blocks.push(dpl_doc::Block::Text {
            id: "brief".into(),
            text: brief,
            font: None,
            size: "h3".into(),
            ink: None,
        });
    }
    dpl_doc::Slide {
        v: dpl_doc::SCHEMA_V,
        template: template.to_string(),
        background: None,
        blocks,
        enter: None,
        reveals: Vec::new(),
    }
}

/// Assemble the filled slots into a comp: every `Artifact` fill becomes a
/// sequential layer on one track (stills-hold span, the W7 convention);
/// `Slide`/`Draft` fills are honestly SKIPPED (a slide is not yet a
/// renderable source — named in the returned notes, never dropped silently).
pub fn comp_for_board(
    board: &StoryboardDoc,
    fps: u32,
    w: u32,
    h: u32,
) -> (dpl_comp::Composition, Vec<String>) {
    let mut comp = dpl_comp::Composition::new(fps, w, h);
    let mut layers = Vec::new();
    let mut notes = Vec::new();
    let mut start = 0.0_f32;
    for slot in &board.slots {
        match &slot.fill {
            SlotFill::Artifact { rel } => {
                layers.push(dpl_comp::Layer::plain(
                    slot.id.clone(),
                    dpl_comp::Source::Artifact { rel: rel.clone() },
                    start,
                    0.0,
                    DEFAULT_SPAN,
                ));
                start += DEFAULT_SPAN;
            }
            SlotFill::Empty => notes.push(format!("slot {} empty — no layer", slot.id)),
            SlotFill::Slide { .. } => notes.push(format!(
                "slot {} holds a slide — not a renderable comp source yet",
                slot.id
            )),
            SlotFill::Draft { .. } => notes.push(format!(
                "slot {} holds an unsaved draft — save it to the library first",
                slot.id
            )),
        }
    }
    comp.tracks.push(dpl_comp::Track {
        id: "main".into(),
        layers,
    });
    (comp, notes)
}

// ── the registry census ─────────────────────────────────────────────────

/// Every registered workflow, one line each — nothing else enumerates
/// workflows anywhere (the slides-registry census law). The storyboard
/// blueprint ships as the WF1 walking skeleton.
pub fn workflows() -> Vec<WorkflowDoc> {
    vec![storyboard()]
}

/// Look one up by id — the `--run-workflow` and panel entry.
pub fn find(id: &str) -> Option<WorkflowDoc> {
    workflows().into_iter().find(|w| w.id == id)
}

/// The storyboard blueprint: outline → slides → optional images → comp →
/// render. Beats here are the DEFAULT tailoring; a minted board is the
/// project-specific instance.
fn storyboard() -> WorkflowDoc {
    let beat = |id: &str, role: &str, title: &str, brief: &str| Beat {
        id: id.into(),
        role: role.into(),
        title: title.into(),
        brief: brief.into(),
    };
    WorkflowDoc {
        v: SCHEMA_V,
        id: "storyboard".into(),
        label: "Storyboard".into(),
        blurb: "Outline beats, mint slide drafts, optionally imagine art, assemble a comp.".into(),
        surface: Surface::Wiredfront,
        stages: vec![
            Stage {
                id: "outline".into(),
                kind: StageKind::Outline {
                    beats: vec![
                        beat("hook", "hook", "The hook", "Grab attention in one line."),
                        beat("context", "context", "The setup", "Why this matters, fast."),
                        beat("beat-1", "beat", "First beat", "The core idea, shown."),
                        beat("beat-2", "beat", "Second beat", "The payoff or proof."),
                        beat("cta", "cta", "Call to action", "What to do next."),
                    ],
                },
            },
            Stage {
                id: "slides".into(),
                kind: StageKind::SlideFromBeat {
                    template: "title".into(),
                },
            },
            Stage {
                id: "imagine".into(),
                kind: StageKind::GenerateImage,
            },
            Stage {
                id: "assemble".into(),
                kind: StageKind::AssembleComp {
                    fps: 30,
                    w: 1920,
                    h: 1080,
                },
            },
            Stage {
                id: "render".into(),
                kind: StageKind::Render,
            },
        ],
    }
}

// ── validation ──────────────────────────────────────────────────────────

/// A validation finding — `path` names WHERE, `message` WHAT (the
/// `dpl_doc::Issue` shape). Empty = clean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub path: String,
    pub message: String,
}

fn kebab_ok(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// A content-relative ref stays in jail: never absolute, never a drive
/// letter, never escaping upward (the `dpl_doc` media-src law).
fn jailed_ok(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.contains(':')
        && !rel.starts_with('/')
        && !rel.starts_with('\\')
        && !rel.split(['/', '\\']).any(|part| part == "..")
}

/// Name every structural fault in a workflow document. The executor
/// refuses a document with issues — half-valid never runs.
pub fn validate(doc: &WorkflowDoc) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |path: String, message: String| issues.push(Issue { path, message });

    if !kebab_ok(&doc.id) {
        push(
            "id".into(),
            format!("workflow id {:?} is not kebab-case", doc.id),
        );
    }
    if doc.label.trim().is_empty() {
        push("label".into(), "label is empty".into());
    }
    if doc.blurb.trim().is_empty() {
        push("blurb".into(), "blurb is empty".into());
    }
    if doc.stages.is_empty() {
        push(
            "stages".into(),
            "a workflow needs at least one stage".into(),
        );
    }

    let mut seen_stage = std::collections::BTreeSet::new();
    let mut outlined = false;
    for stage in &doc.stages {
        let at = format!("stages.{}", stage.id);
        if !kebab_ok(&stage.id) {
            push(
                at.clone(),
                format!("stage id {:?} is not kebab-case", stage.id),
            );
        }
        if !seen_stage.insert(stage.id.clone()) {
            push(at.clone(), "duplicate stage id".into());
        }
        match &stage.kind {
            StageKind::Outline { beats } => {
                if beats.is_empty() {
                    push(format!("{at}.beats"), "outline has no beats".into());
                }
                let mut seen_beat = std::collections::BTreeSet::new();
                for b in beats {
                    if !kebab_ok(&b.id) {
                        push(
                            format!("{at}.beats.{}", b.id),
                            format!("beat id {:?} is not kebab-case", b.id),
                        );
                    }
                    if !seen_beat.insert(b.id.clone()) {
                        push(format!("{at}.beats.{}", b.id), "duplicate beat id".into());
                    }
                    if b.title.trim().is_empty() {
                        push(
                            format!("{at}.beats.{}.title", b.id),
                            "title is empty".into(),
                        );
                    }
                }
                outlined = true;
            }
            StageKind::SlideFromBeat { template } => {
                if template.trim().is_empty() {
                    push(format!("{at}.template"), "template id is empty".into());
                }
                if !outlined {
                    push(
                        at.clone(),
                        "slide-from-beat needs an outline stage before it".into(),
                    );
                }
            }
            StageKind::GenerateImage | StageKind::Render => {
                if !outlined {
                    push(
                        at.clone(),
                        format!("{} needs an outline stage before it", stage.kind.label()),
                    );
                }
            }
            StageKind::AssembleComp { fps, w, h } => {
                if *fps == 0 {
                    push(format!("{at}.fps"), "fps must be > 0".into());
                }
                if *w == 0 || *h == 0 {
                    push(
                        format!("{at}.size"),
                        format!("w and h must be > 0 (got {w}x{h})"),
                    );
                }
                if !outlined {
                    push(
                        at.clone(),
                        "assemble-comp needs an outline stage before it".into(),
                    );
                }
            }
        }
    }
    issues
}

/// Name every structural fault in a storyboard — including jailbreak refs.
pub fn validate_board(board: &StoryboardDoc) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |path: String, message: String| issues.push(Issue { path, message });

    let mut seen = std::collections::BTreeSet::new();
    for slot in &board.slots {
        let at = format!("slots.{}", slot.id);
        if !kebab_ok(&slot.id) {
            push(
                at.clone(),
                format!("slot id {:?} is not kebab-case", slot.id),
            );
        }
        if !seen.insert(slot.id.clone()) {
            push(at.clone(), "duplicate slot id".into());
        }
        if slot.brief.trim().is_empty() {
            push(
                format!("{at}.brief"),
                "brief is empty — a slot names what it needs".into(),
            );
        }
        match &slot.fill {
            SlotFill::Artifact { rel } | SlotFill::Slide { rel } => {
                if !jailed_ok(rel) {
                    push(
                        format!("{at}.fill"),
                        format!("ref {rel:?} escapes the content jail"),
                    );
                }
            }
            SlotFill::Draft { id } => {
                if id.trim().is_empty() {
                    push(format!("{at}.fill"), "draft id is empty".into());
                }
            }
            SlotFill::Empty => {}
        }
    }
    issues
}

// ── load/save (crash-safe, the dpl-doc writer shape) ────────────────────

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
                write!(f, "document is schema v{v}; this build reads v{SCHEMA_V}")
            }
        }
    }
}

impl std::error::Error for DocError {}

const BOARD_FILE: &str = "storyboard.json";

/// Load `<dir>/storyboard.json`, refusing a newer schema honestly.
pub fn load_board(dir: &Path) -> Result<StoryboardDoc, DocError> {
    let raw = std::fs::read_to_string(dir.join(BOARD_FILE)).map_err(DocError::Io)?;
    let board: StoryboardDoc = serde_json::from_str(&raw).map_err(DocError::Parse)?;
    if board.v > SCHEMA_V {
        return Err(DocError::Version(board.v));
    }
    Ok(board)
}

/// Crash-safe pretty-JSON+LF write into `<dir>/storyboard.json` (tmp →
/// bak-step-aside → swap; the `dpl-doc` writer, replicated so the document
/// layer depends on no other crate for a file swap).
pub fn save_board(dir: &Path, board: &StoryboardDoc) -> Result<(), DocError> {
    std::fs::create_dir_all(dir).map_err(DocError::Io)?;
    let target = dir.join(BOARD_FILE);
    let tmp = dir.join(format!("{BOARD_FILE}.tmp"));
    let bak = dir.join(format!("{BOARD_FILE}.bak"));

    let mut json = serde_json::to_string_pretty(board).map_err(DocError::Parse)?;
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

/// The next free `<parent>/<base>[-N]/` — the composer's no-clobber mint,
/// shared so a rerun NEVER writes over a prior run.
pub fn fresh_dir(parent: &Path, base: &str) -> PathBuf {
    let first = parent.join(base);
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| parent.join(format!("{base}-{n}")))
        .find(|p| !p.exists())
        .unwrap()
}

// ── execution ───────────────────────────────────────────────────────────

/// What one stage reported. `Skipped` is a first-class HONEST outcome
/// (dry-run generation, keyless generation) — never a silent pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageVerdict {
    Done { note: String },
    Skipped { reason: String },
    Failed { error: String },
}

/// One stage's row in the run report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageReport {
    pub stage: String,
    pub kind: &'static str,
    pub verdict: StageVerdict,
}

/// The whole run: stages in execution order; a failed stage is the LAST
/// row (the run halts and names it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    pub workflow: String,
    pub stages: Vec<StageReport>,
}

impl RunReport {
    pub fn failed(&self) -> bool {
        self.stages
            .iter()
            .any(|s| matches!(s.verdict, StageVerdict::Failed { .. }))
    }

    /// The `--run-workflow` verdict line (the `--make-slide` shape).
    pub fn verdict_line(&self) -> String {
        if let Some(f) = self
            .stages
            .iter()
            .find(|s| matches!(s.verdict, StageVerdict::Failed { .. }))
        {
            let StageVerdict::Failed { error } = &f.verdict else {
                unreachable!()
            };
            return format!(
                "run-workflow: {} FAILED at {} · {error}",
                self.workflow, f.stage
            );
        }
        let done = self
            .stages
            .iter()
            .filter(|s| matches!(s.verdict, StageVerdict::Done { .. }))
            .count();
        let skipped = self.stages.len() - done;
        let mut line = format!(
            "run-workflow: {} ok · {} stage(s) done",
            self.workflow, done
        );
        if skipped > 0 {
            line.push_str(&format!(" · {skipped} skipped"));
        }
        line
    }
}

/// A lifecycle event the executor emits around every stage — the app maps
/// these onto `jobs.*` fabric facts; this crate stays fabric-free.
#[derive(Debug)]
pub enum StageEvent<'a> {
    Started(&'a Stage),
    Finished(&'a Stage, &'a StageVerdict),
}

/// The app-side seam: materialize one stage (mint slides, spend Imagine,
/// save the comp, render). The minted board arrives mutable — fill stages
/// write their fills into it.
pub trait StageRunner {
    fn run_stage(
        &mut self,
        workflow: &WorkflowDoc,
        stage: &Stage,
        board: &mut StoryboardDoc,
    ) -> StageVerdict;
}

/// Execute a workflow: validate (half-valid never runs), mint the board at
/// `Outline` (pure), hand every stage to the runner in order, halt on the
/// first failure. `on` fires around every stage for jobs-fact wiring.
pub fn execute(
    doc: &WorkflowDoc,
    runner: &mut dyn StageRunner,
    on: &mut dyn FnMut(StageEvent<'_>),
) -> Result<(RunReport, StoryboardDoc), String> {
    let issues = validate(doc);
    if let Some(i) = issues.first() {
        return Err(format!("workflow invalid at {}: {}", i.path, i.message));
    }
    let mut board = mint_storyboard(&doc.id, &[]);
    let mut report = RunReport {
        workflow: doc.id.clone(),
        stages: Vec::new(),
    };
    for stage in &doc.stages {
        on(StageEvent::Started(stage));
        let verdict = if let StageKind::Outline { beats } = &stage.kind {
            board = mint_storyboard(&doc.id, beats);
            StageVerdict::Done {
                note: format!("minted {} slot(s)", board.slots.len()),
            }
        } else {
            runner.run_stage(doc, stage, &mut board)
        };
        on(StageEvent::Finished(stage, &verdict));
        let failed = matches!(verdict, StageVerdict::Failed { .. });
        report.stages.push(StageReport {
            stage: stage.id.clone(),
            kind: stage.kind.label(),
            verdict,
        });
        if failed {
            break;
        }
    }
    Ok((report, board))
}

/// The honest dry-run: reports what every stage WOULD do; nothing touches
/// disk and generation NEVER spends (ADR-011).
pub struct DryRunner;

impl StageRunner for DryRunner {
    fn run_stage(
        &mut self,
        _workflow: &WorkflowDoc,
        stage: &Stage,
        board: &mut StoryboardDoc,
    ) -> StageVerdict {
        match &stage.kind {
            StageKind::Outline { .. } => unreachable!("the executor mints the board"),
            StageKind::SlideFromBeat { template } => StageVerdict::Done {
                note: format!(
                    "would mint {} `{template}` slide(s)",
                    board.remaining().len()
                ),
            },
            StageKind::GenerateImage => StageVerdict::Skipped {
                reason: "dry-run — generation never spends (ADR-011)".into(),
            },
            StageKind::AssembleComp { fps, w, h } => {
                let (comp, notes) = comp_for_board(board, *fps, *w, *h);
                StageVerdict::Done {
                    note: format!(
                        "would assemble {} layer(s), {} note(s)",
                        comp.tracks.iter().map(|t| t.layers.len()).sum::<usize>(),
                        notes.len()
                    ),
                }
            }
            StageKind::Render => StageVerdict::Skipped {
                reason: "dry-run — nothing assembled to render".into(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("dpl-workflows-tests")
            .join(format!("{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn the_registry_is_the_census() {
        let all = workflows();
        assert_eq!(all.len(), 1, "WF1 ships exactly the storyboard skeleton");
        let mut ids = std::collections::BTreeSet::new();
        for w in &all {
            assert!(ids.insert(w.id.clone()), "workflow ids must be unique");
            assert!(!w.label.is_empty());
            assert!(!w.blurb.is_empty());
            assert!(
                validate(w).is_empty(),
                "registered workflow {} must validate clean: {:?}",
                w.id,
                validate(w)
            );
        }
        assert!(find("storyboard").is_some());
        assert!(find("no-such-workflow").is_none());
    }

    #[test]
    fn documents_round_trip() {
        let doc = workflows().remove(0);
        let json = serde_json::to_string(&doc).unwrap();
        assert_eq!(doc, serde_json::from_str::<WorkflowDoc>(&json).unwrap());

        let board = mint_storyboard(
            "storyboard",
            &[Beat {
                id: "hook".into(),
                role: "hook".into(),
                title: "T".into(),
                brief: "B".into(),
            }],
        );
        let json = serde_json::to_string(&board).unwrap();
        assert_eq!(board, serde_json::from_str::<StoryboardDoc>(&json).unwrap());
    }

    #[test]
    fn board_save_is_pretty_lf_and_reloads() {
        let dir = scratch("save");
        let mut board = mint_storyboard(
            "storyboard",
            &[Beat {
                id: "hook".into(),
                role: "hook".into(),
                title: "T".into(),
                brief: "B".into(),
            }],
        );
        save_board(&dir, &board).unwrap();
        let raw = std::fs::read_to_string(dir.join("storyboard.json")).unwrap();
        assert!(raw.ends_with('\n'), "save writes a trailing LF");
        assert!(!raw.contains('\r'), "save never writes CRLF");
        assert!(raw.contains("  \"v\": 1"), "save is pretty-printed");
        assert_eq!(load_board(&dir).unwrap(), board);

        // Overwrite is crash-safe (bak swap) and leaves no transients.
        board.slots[0].fill = SlotFill::Slide {
            rel: "inbox/hook-slide".into(),
        };
        save_board(&dir, &board).unwrap();
        assert_eq!(load_board(&dir).unwrap(), board);
        assert!(!dir.join("storyboard.json.tmp").exists());
        assert!(!dir.join("storyboard.json.bak").exists());
    }

    #[test]
    fn a_newer_schema_is_refused() {
        let dir = scratch("newer");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("storyboard.json"),
            format!("{{\"v\":{},\"workflow\":\"x\",\"slots\":[]}}", SCHEMA_V + 1),
        )
        .unwrap();
        match load_board(&dir) {
            Err(DocError::Version(v)) => assert_eq!(v, SCHEMA_V + 1),
            other => panic!("expected version refusal, got {other:?}"),
        }
    }

    #[test]
    fn validation_names_structural_faults() {
        let mut doc = workflows().remove(0);
        doc.stages[1].id = "outline".into(); // duplicate of stage 0
        let issues = validate(&doc);
        assert!(
            issues
                .iter()
                .any(|i| i.path == "stages.outline" && i.message.contains("duplicate")),
            "duplicate stage id must be named: {issues:?}"
        );

        // A fill stage with no outline before it is a dangling ref.
        let orphan = WorkflowDoc {
            v: SCHEMA_V,
            id: "orphan".into(),
            label: "Orphan".into(),
            blurb: "b".into(),
            surface: Surface::Wiredfront,
            stages: vec![Stage {
                id: "slides".into(),
                kind: StageKind::SlideFromBeat {
                    template: String::new(),
                },
            }],
        };
        let issues = validate(&orphan);
        assert!(issues.iter().any(|i| i.message.contains("outline")));
        assert!(issues.iter().any(|i| i.path == "stages.slides.template"));
    }

    #[test]
    fn board_refs_stay_jailed() {
        let mut board = mint_storyboard(
            "storyboard",
            &[Beat {
                id: "hook".into(),
                role: "hook".into(),
                title: "T".into(),
                brief: "B".into(),
            }],
        );
        for bad in ["C:/evil.png", "../up.png", "/abs.png", "a/../b.png"] {
            board.slots[0].fill = SlotFill::Artifact { rel: bad.into() };
            let issues = validate_board(&board);
            assert!(
                issues.iter().any(|i| i.path == "slots.hook.fill"),
                "{bad} must be named as a jailbreak: {issues:?}"
            );
        }
        board.slots[0].fill = SlotFill::Artifact {
            rel: "inbox/fine.png".into(),
        };
        assert!(validate_board(&board).is_empty());
    }

    #[test]
    fn every_stage_executes_dry_and_reports_its_verdict() {
        let doc = find("storyboard").unwrap();
        let mut events = 0usize;
        let (report, board) = execute(&doc, &mut DryRunner, &mut |_| events += 1).unwrap();
        assert_eq!(report.stages.len(), doc.stages.len());
        assert_eq!(events, doc.stages.len() * 2, "started+finished per stage");
        assert!(!report.failed());
        assert_eq!(board.slots.len(), 5);
        assert_eq!(board.remaining().len(), 5, "dry-run fills nothing");
        // Generation is honestly skipped, never spent.
        let imagine = &report.stages[2];
        assert_eq!(imagine.kind, "generate-image");
        assert!(matches!(
            &imagine.verdict,
            StageVerdict::Skipped { reason } if reason.contains("ADR-011")
        ));
        assert!(report
            .verdict_line()
            .starts_with("run-workflow: storyboard ok"));
    }

    #[test]
    fn a_failed_stage_halts_and_names_itself() {
        struct FailAt(&'static str);
        impl StageRunner for FailAt {
            fn run_stage(
                &mut self,
                _w: &WorkflowDoc,
                stage: &Stage,
                _b: &mut StoryboardDoc,
            ) -> StageVerdict {
                if stage.id == self.0 {
                    StageVerdict::Failed {
                        error: "template registry offline".into(),
                    }
                } else {
                    StageVerdict::Done { note: "ok".into() }
                }
            }
        }
        let doc = find("storyboard").unwrap();
        let (report, _) = execute(&doc, &mut FailAt("slides"), &mut |_| {}).unwrap();
        assert_eq!(report.stages.len(), 2, "halts at the failure");
        assert!(report.failed());
        let line = report.verdict_line();
        assert!(
            line.contains("FAILED at slides") && line.contains("template registry offline"),
            "the verdict names stage and error: {line}"
        );
    }

    #[test]
    fn an_invalid_workflow_never_runs() {
        let mut doc = find("storyboard").unwrap();
        doc.stages.clear();
        let err = execute(&doc, &mut DryRunner, &mut |_| {}).unwrap_err();
        assert!(err.contains("workflow invalid at stages"));
    }

    #[test]
    fn rerun_is_no_clobber() {
        let parent = scratch("mint");
        std::fs::create_dir_all(&parent).unwrap();
        let first = fresh_dir(&parent, "storyboard");
        std::fs::create_dir_all(&first).unwrap();
        let second = fresh_dir(&parent, "storyboard");
        assert_ne!(first, second);
        assert!(second.ends_with("storyboard-2"));
        std::fs::create_dir_all(&second).unwrap();
        assert!(fresh_dir(&parent, "storyboard").ends_with("storyboard-3"));
    }

    #[test]
    fn slide_for_slot_builds_the_deck_shape() {
        let board = mint_storyboard(
            "storyboard",
            &[Beat {
                id: "hook".into(),
                role: "hook".into(),
                title: "The hook".into(),
                brief: "Grab attention.".into(),
            }],
        );
        let slide = slide_for_slot("title", &board.slots[0]);
        assert_eq!(slide.template, "title");
        assert_eq!(slide.blocks.len(), 2);
        assert!(
            dpl_doc::validate(&slide).is_empty(),
            "minted slide validates clean"
        );
    }

    #[test]
    fn comp_for_board_assembles_artifacts_and_names_skips() {
        let mut board = mint_storyboard(
            "storyboard",
            &[
                Beat {
                    id: "a".into(),
                    role: "beat".into(),
                    title: "A".into(),
                    brief: "a".into(),
                },
                Beat {
                    id: "b".into(),
                    role: "beat".into(),
                    title: "B".into(),
                    brief: "b".into(),
                },
                Beat {
                    id: "c".into(),
                    role: "beat".into(),
                    title: "C".into(),
                    brief: "c".into(),
                },
            ],
        );
        board.slots[0].fill = SlotFill::Artifact {
            rel: "inbox/a.png".into(),
        };
        board.slots[1].fill = SlotFill::Slide {
            rel: "inbox/b-slide".into(),
        };
        let (comp, notes) = comp_for_board(&board, 30, 1920, 1080);
        assert!(dpl_comp::validate(&comp).is_empty());
        assert_eq!(comp.tracks[0].layers.len(), 1);
        assert_eq!(comp.tracks[0].layers[0].start, 0.0);
        assert_eq!(
            notes.len(),
            2,
            "the slide and the empty slot are both named"
        );
        assert!(notes.iter().any(|n| n.contains("not a renderable")));
    }
}

/// THE SCHEMA PIN (workflow-doctrine law 3): the wire contract in
/// `schemas/` and the serde shapes here are ONE thing — these tests fail
/// the gate the moment either drifts. Includes a negative case so the
/// schema is proven load-bearing, never decorative.
#[cfg(test)]
mod schema_pin {
    use super::*;

    fn validator(file: &str) -> jsonschema::Validator {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../schemas")
            .join(file);
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let schema: serde_json::Value = serde_json::from_str(&raw).expect("schema is JSON");
        jsonschema::validator_for(&schema).expect("schema compiles")
    }

    fn assert_valid(v: &jsonschema::Validator, doc: &serde_json::Value, what: &str) {
        let errors: Vec<String> = v.iter_errors(doc).map(|e| e.to_string()).collect();
        assert!(errors.is_empty(), "{what} violates the schema: {errors:?}");
    }

    #[test]
    fn every_registered_workflow_matches_the_v1_wire_schema() {
        let v = validator("workflow.v1.schema.json");
        for wf in workflows() {
            let doc = serde_json::to_value(&wf).expect("serialize");
            assert_valid(&v, &doc, &format!("registered workflow {:?}", wf.id));
        }
    }

    #[test]
    fn a_minted_and_filled_storyboard_matches_the_v1_wire_schema() {
        let v = validator("storyboard.v1.schema.json");
        let wf = find("storyboard").expect("registered");
        let StageKind::Outline { beats } = &wf.stages[0].kind else {
            panic!("storyboard opens with an outline");
        };
        let mut board = mint_storyboard(&wf.id, beats);
        board.show = Some("devpulse-news".into());
        board.episode = Some("ep1".into());
        board.slots[0].fill = SlotFill::Artifact {
            rel: "inbox/comp-1/render.mp4".into(),
        };
        board.slots[1].fill = SlotFill::Slide {
            rel: "inbox/slide-1".into(),
        };
        board.slots[2].fill = SlotFill::Draft { id: "d-42".into() };
        board.slots[3].constraints = vec!["16:9".into()];
        let doc = serde_json::to_value(&board).expect("serialize");
        assert_valid(&v, &doc, "minted+filled storyboard");
    }

    #[test]
    fn the_workflow_schema_refuses_drift() {
        let v = validator("workflow.v1.schema.json");
        let good = serde_json::to_value(find("storyboard").unwrap()).unwrap();

        // An unknown stage kind is drift, not tolerated input.
        let mut doc = good.clone();
        doc["stages"][0] = serde_json::json!({"id": "teleport", "kind": "teleport"});
        assert!(!v.is_valid(&doc), "unknown stage kind must be refused");

        // A non-kebab id breaks the identity law at the contract.
        let mut doc = good.clone();
        doc["id"] = serde_json::json!("Not Kebab");
        assert!(!v.is_valid(&doc), "non-kebab workflow id must be refused");

        // Unknown root fields are drift (additionalProperties: false).
        let mut doc = good.clone();
        doc["vibes"] = serde_json::json!("immaculate");
        assert!(!v.is_valid(&doc), "unknown fields must be refused");

        // A future major version never validates against the v1 file.
        let mut doc = good;
        doc["v"] = serde_json::json!(2);
        assert!(!v.is_valid(&doc), "v2 must not pass the v1 schema");
    }

    #[test]
    fn the_storyboard_schema_refuses_drift() {
        let v = validator("storyboard.v1.schema.json");
        let board = mint_storyboard(
            "storyboard",
            &[Beat {
                id: "hook".into(),
                role: "hook".into(),
                title: "The hook".into(),
                brief: "Grab attention.".into(),
            }],
        );
        let good = serde_json::to_value(&board).unwrap();

        // Absolute and drive-lettered refs are refused at the contract
        // (the full `..` jail is engine-enforced — both layers are law).
        for bad_rel in ["/etc/passwd", "C:\\loot", "a\\b"] {
            let mut doc = good.clone();
            doc["slots"][0]["fill"] =
                serde_json::json!({"kind": "artifact", "rel": bad_rel});
            assert!(!v.is_valid(&doc), "jailbreak rel {bad_rel:?} must be refused");
        }

        // An unknown fill kind is drift.
        let mut doc = good.clone();
        doc["slots"][0]["fill"] = serde_json::json!({"kind": "hologram"});
        assert!(!v.is_valid(&doc), "unknown fill kind must be refused");

        // Extra slot fields are drift.
        let mut doc = good;
        doc["slots"][0]["mood"] = serde_json::json!("neon");
        assert!(!v.is_valid(&doc), "unknown slot fields must be refused");
    }
}
