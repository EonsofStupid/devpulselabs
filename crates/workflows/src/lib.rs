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
    /// The guided intake this blueprint opens with (EX1): a declarative
    /// Q&A whose answers mint the bible. ADDITIVE-optional — pre-EX1
    /// documents stay valid v1 (doctrine law 2/3 additive amendment).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intake: Option<IntakeSpec>,
    pub stages: Vec<Stage>,
}

/// The declarative guided intake (EX1): ordered typed questions both
/// surfaces render — the chat guided flow AND the panel form paint THIS
/// spec (D5 operator-parity by construction). Nothing else enumerates
/// intake questions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntakeSpec {
    pub fields: Vec<IntakeField>,
}

/// One intake question: what to ask, what kind of answer it takes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntakeField {
    pub id: String,
    /// The question, in the operator's language.
    pub ask: String,
    #[serde(flatten)]
    pub kind: IntakeFieldKind,
    /// Optional questions may be skipped; answers to required ones gate
    /// the bible mint.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

/// serde's `skip_serializing_if` contract requires `&bool`.
fn is_true(b: &bool) -> bool {
    *b
}

/// The v1 intake answer kinds — exactly what the explainer needs. New
/// kinds join HERE (census law: enum + validate arm + schema + pins).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum IntakeFieldKind {
    /// Free text (subject, audience, tone…).
    Text,
    /// One of a named set.
    Choice { options: Vec<String> },
    /// Pick a registered FormatProfile (the short↔long axis).
    FormatPick,
    /// Draft/confirm the beat list (title + brief + narration per beat);
    /// the assist may propose, the operator disposes.
    BeatsDraft,
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

/// The slide document for one slot (`dpl.slide/3`): the slot's title in
/// the component's `title` slot and its brief in `subtitle` (the title
/// component's own slots, sized as the deck's proven text slide). Pure
/// value with a fresh id; the runner writes it.
pub fn slide_for_slot(template: &str, slot: &Slot) -> dpl_doc::Slide {
    let (title, brief) = match slot.brief.split_once(" — ") {
        Some((t, b)) => (t.to_string(), b.to_string()),
        None => (slot.brief.clone(), String::new()),
    };
    let sized = |id: &str, text: String, size: &str| dpl_doc::Block::Text {
        id: id.into(),
        preset: None,
        tone: None,
        text,
        font: None,
        size: Some(size.into()),
        ink: None,
        enter: None,
        unlinked_from: None,
    };
    let mut slide = dpl_doc::Slide::new(format!("{template}@1"));
    slide.title = Some(title.clone());
    slide.blocks.push(sized("title", title, "h1"));
    if !brief.is_empty() {
        slide.blocks.push(sized("subtitle", brief, "h3"));
    }
    slide
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

// ── the bible (EX1: production + script) ────────────────────────────────

/// The PRODUCTION + SCRIPT bible — the canon every downstream stage
/// answers to (program map: "bible artifacts become upstream nodes whose
/// outputs feed every downstream stage"). Minted from a completed intake;
/// the storyboard is tailored FROM it; narration is the script half the
/// VO/caption passes will consume. Wire contract:
/// `schemas/bible.v1.schema.json` (doctrine law 3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BibleDoc {
    pub v: u32,
    /// Kebab identity of THIS bible instance (its folder name stem).
    pub id: String,
    /// The blueprint this bible answers (registry id, e.g. "explainer").
    pub workflow: String,
    pub subject: String,
    pub audience: String,
    /// Tone/voice in plain words ("confident, plain-spoken, no hype").
    pub tone: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub style_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_points: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub avoid: Vec<String>,
    /// A registered [`FormatProfile`] id — the short↔long axis.
    pub format: String,
    /// The SCRIPT half: authored beats, narration required per beat.
    pub beats: Vec<BibleBeat>,
}

/// One authored bible beat — a storyboard [`Beat`] plus its narration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BibleBeat {
    pub id: String,
    pub role: String,
    pub title: String,
    pub brief: String,
    /// What is SAID over this beat — the script-bible law: no beat
    /// without its narration.
    pub narration: String,
}

/// Name every structural fault in a bible. The tailor refuses a bible
/// with issues — half-valid never mints.
pub fn validate_bible(bible: &BibleDoc) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |path: String, message: String| issues.push(Issue { path, message });
    if !kebab_ok(&bible.id) {
        push(
            "id".into(),
            format!("bible id {:?} is not kebab-case", bible.id),
        );
    }
    if !kebab_ok(&bible.workflow) {
        push(
            "workflow".into(),
            format!("workflow ref {:?} is not kebab-case", bible.workflow),
        );
    }
    for (field, value) in [
        ("subject", &bible.subject),
        ("audience", &bible.audience),
        ("tone", &bible.tone),
    ] {
        if value.trim().is_empty() {
            push(field.into(), format!("{field} is empty"));
        }
    }
    if format_profile(&bible.format).is_none() {
        let all = format_profiles()
            .iter()
            .map(|p| p.id)
            .collect::<Vec<_>>()
            .join(", ");
        push(
            "format".into(),
            format!("unknown format {:?} — registered: {all}", bible.format),
        );
    }
    if bible.beats.is_empty() {
        push("beats".into(), "a bible needs at least one beat".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for b in &bible.beats {
        let at = format!("beats.{}", b.id);
        if !kebab_ok(&b.id) {
            push(at.clone(), format!("beat id {:?} is not kebab-case", b.id));
        }
        if !seen.insert(b.id.clone()) {
            push(at.clone(), "duplicate beat id".into());
        }
        if b.title.trim().is_empty() {
            push(format!("{at}.title"), "title is empty".into());
        }
        if b.narration.trim().is_empty() {
            push(
                format!("{at}.narration"),
                "narration is empty — the script bible has no silent beats".into(),
            );
        }
    }
    issues
}

// ── format profiles (the short↔long axis, D6 ingredients) ───────────────

/// One registered video format — duration + beat shape at 16:9 (v1;
/// vertical joins as a later census line). The tailor reads geometry and
/// pacing from HERE; nothing else hardcodes them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormatProfile {
    pub id: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    pub fps: u32,
    pub w: u32,
    pub h: u32,
    /// The beat count the intake's BeatsDraft proposes (a target, not a
    /// cage — the operator's authored count wins).
    pub beat_budget: usize,
    /// Seconds each beat holds on screen in the assembled comp.
    pub per_beat_secs: f32,
}

/// THE format census — one line per profile, nothing else enumerates.
pub fn format_profiles() -> Vec<FormatProfile> {
    vec![
        FormatProfile {
            id: "short",
            label: "Short",
            blurb: "Under a minute — hook, one idea, call to action.",
            fps: 30,
            w: 1920,
            h: 1080,
            beat_budget: 4,
            per_beat_secs: 11.0,
        },
        FormatProfile {
            id: "standard",
            label: "Standard",
            blurb: "A couple of minutes — room to show, not just tell.",
            fps: 30,
            w: 1920,
            h: 1080,
            beat_budget: 7,
            per_beat_secs: 17.0,
        },
        FormatProfile {
            id: "long",
            label: "Long-form",
            blurb: "Five minutes and up — sections, depth, worked examples.",
            fps: 30,
            w: 1920,
            h: 1080,
            beat_budget: 12,
            per_beat_secs: 30.0,
        },
    ]
}

/// Look one profile up by id.
pub fn format_profile(id: &str) -> Option<FormatProfile> {
    format_profiles().into_iter().find(|p| p.id == id)
}

// ── the tailor (bible → per-run workflow instance) ──────────────────────

/// Mint the tailored, per-run [`WorkflowDoc`] from a bible: the bible's
/// beats become the outline, the format profile supplies geometry.
/// Pure and DETERMINISTIC — the same bible always mints the same doc.
/// Narration stays in the bible (the upstream node downstream stages
/// read); it never leaks into slide text. Refuses an invalid bible with
/// every issue named.
pub fn tailor(bible: &BibleDoc) -> Result<WorkflowDoc, Vec<Issue>> {
    let issues = validate_bible(bible);
    if !issues.is_empty() {
        return Err(issues);
    }
    let profile = format_profile(&bible.format).expect("validated");
    let beats = bible
        .beats
        .iter()
        .map(|b| Beat {
            id: b.id.clone(),
            role: b.role.clone(),
            title: b.title.clone(),
            brief: b.brief.clone(),
        })
        .collect();
    Ok(WorkflowDoc {
        v: SCHEMA_V,
        id: format!("{}-{}", bible.workflow, bible.id),
        label: format!("{} — {}", bible.subject, profile.label),
        blurb: format!("Tailored {} run of {:?}.", profile.id, bible.workflow),
        surface: Surface::Wiredfront,
        intake: None,
        stages: vec![
            Stage {
                id: "outline".into(),
                kind: StageKind::Outline { beats },
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
                    fps: profile.fps,
                    w: profile.w,
                    h: profile.h,
                },
            },
            Stage {
                id: "render".into(),
                kind: StageKind::Render,
            },
        ],
    })
}

// ── the brainstorm board (BS1: the massive idea, breaking down) ─────────

/// The brainstorm phases, in order — the template-first scaffold the
/// surface walks: explode the idea, group what landed, order the
/// survivors. Promotion (distill) is only offered from the last phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BrainstormPhase {
    Diverge,
    Cluster,
    Converge,
}

impl BrainstormPhase {
    /// The phase's kebab label — surface headers and pilot nodes speak
    /// this.
    pub fn label(self) -> &'static str {
        match self {
            BrainstormPhase::Diverge => "diverge",
            BrainstormPhase::Cluster => "cluster",
            BrainstormPhase::Converge => "converge",
        }
    }

    /// The phase after this one; `None` past the end — advancing from
    /// Converge is a refusal, not a wrap.
    pub fn next(self) -> Option<Self> {
        match self {
            BrainstormPhase::Diverge => Some(BrainstormPhase::Cluster),
            BrainstormPhase::Cluster => Some(BrainstormPhase::Converge),
            BrainstormPhase::Converge => None,
        }
    }
}

/// Who put an item on the board. Assist proposals stay visibly ASSIST
/// until the operator edits them — same ops, disclosed origin (D5
/// parity: the assist may propose, the operator disposes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BrainstormOrigin {
    Operator,
    Assist,
}

/// One idea on the board — an editable peer object whatever its origin.
/// `parent` makes it a breakdown of another item; `cluster` groups it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrainstormItem {
    pub id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cluster: Option<String>,
    pub origin: BrainstormOrigin,
}

/// One named group the Cluster phase sorts items into.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrainstormCluster {
    pub id: String,
    pub label: String,
}

/// The brainstorm board — the massive idea and its breakdown, one
/// document. The chat surface renders THIS; every gesture (operator tap
/// or assist proposal) mutates it only through [`apply`], so the board
/// on disk is always the board on screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrainstormDoc {
    pub v: u32,
    /// Kebab identity of THIS board instance (its folder name stem).
    pub id: String,
    /// A registered [`BrainstormScaffold`] id.
    pub template: String,
    /// The massive idea, in the operator's words. Empty until the first
    /// [`BrainstormOp::Seed`] — the board opens before the idea lands.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub seed: String,
    pub phase: BrainstormPhase,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<BrainstormItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clusters: Vec<BrainstormCluster>,
}

/// Name every structural fault in a board. [`apply`] refuses to touch a
/// board with issues — half-valid never mutates.
pub fn validate_brainstorm(doc: &BrainstormDoc) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |path: String, message: String| issues.push(Issue { path, message });
    if !kebab_ok(&doc.id) {
        push(
            "id".into(),
            format!("board id {:?} is not kebab-case", doc.id),
        );
    }
    if brainstorm_scaffold(&doc.template).is_none() {
        let all = brainstorm_scaffolds()
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>()
            .join(", ");
        push(
            "template".into(),
            format!("unknown scaffold {:?} — registered: {all}", doc.template),
        );
    }
    let mut item_ids = std::collections::BTreeSet::new();
    for item in &doc.items {
        let at = format!("items.{}", item.id);
        if !kebab_ok(&item.id) {
            push(
                at.clone(),
                format!("item id {:?} is not kebab-case", item.id),
            );
        }
        if !item_ids.insert(item.id.clone()) {
            push(at.clone(), "duplicate item id".into());
        }
        if item.text.trim().is_empty() {
            push(format!("{at}.text"), "item text is empty".into());
        }
    }
    let mut cluster_ids = std::collections::BTreeSet::new();
    for cluster in &doc.clusters {
        let at = format!("clusters.{}", cluster.id);
        if !kebab_ok(&cluster.id) {
            push(
                at.clone(),
                format!("cluster id {:?} is not kebab-case", cluster.id),
            );
        }
        if !cluster_ids.insert(cluster.id.clone()) {
            push(at.clone(), "duplicate cluster id".into());
        }
        if cluster.label.trim().is_empty() {
            push(format!("{at}.label"), "cluster label is empty".into());
        }
    }
    for item in &doc.items {
        let at = format!("items.{}", item.id);
        if let Some(parent) = &item.parent {
            if !item_ids.contains(parent) {
                push(
                    format!("{at}.parent"),
                    format!("parent {parent:?} does not exist"),
                );
            }
        }
        if let Some(cluster) = &item.cluster {
            if !cluster_ids.contains(cluster) {
                push(
                    format!("{at}.cluster"),
                    format!("cluster {cluster:?} does not exist"),
                );
            }
        }
    }
    // A parent chain must terminate — a breakdown is a tree, not a loop.
    for item in &doc.items {
        let mut seen = std::collections::BTreeSet::new();
        let mut cursor = item.parent.as_deref();
        while let Some(id) = cursor {
            if !seen.insert(id) || id == item.id {
                push(
                    format!("items.{}.parent", item.id),
                    "breakdown chain loops".into(),
                );
                break;
            }
            cursor = doc
                .items
                .iter()
                .find(|i| i.id == id)
                .and_then(|i| i.parent.as_deref());
        }
    }
    issues
}

/// The typed board verbs — the ONE mutation vocabulary. The operator's
/// gestures and the assist's proposals both speak this; nothing else
/// edits a board (census law: variant + [`apply`] arm + pins).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum BrainstormOp {
    /// Set (or restate) the massive idea.
    Seed { seed: String },
    /// Put an idea on the board, optionally as a breakdown of `parent`.
    AddItem {
        id: String,
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent: Option<String>,
        origin: BrainstormOrigin,
    },
    /// Rewrite an item's text. An operator edit CLAIMS an assist item —
    /// touched work is the operator's work.
    EditItem { id: String, text: String },
    /// Remove an item AND its whole breakdown subtree — nothing dangles.
    RemoveItem { id: String },
    /// Add a named cluster to sort into.
    AddCluster { id: String, label: String },
    /// Rename a cluster.
    RenameCluster { id: String, label: String },
    /// Move an item into a cluster (`None` = unsorted).
    AssignCluster {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cluster: Option<String>,
    },
    /// Step the board to the next phase. Gated: leaving Diverge needs a
    /// seed and at least two items; leaving Cluster needs every root
    /// item sorted; Converge is the end.
    AdvancePhase,
}

/// Apply one verb to the board — pure in effect (same board + same op =
/// same result), all-or-nothing (a refused op mutates NOTHING), every
/// refusal a named issue. This is the single write path the surface,
/// the assist, and the drills all share.
pub fn apply(doc: &mut BrainstormDoc, op: &BrainstormOp) -> Result<(), Vec<Issue>> {
    let refuse = |path: &str, message: String| {
        Err(vec![Issue {
            path: path.into(),
            message,
        }])
    };
    match op {
        BrainstormOp::Seed { seed } => {
            if seed.trim().is_empty() {
                return refuse("seed", "a seed needs words".into());
            }
            doc.seed = seed.trim().to_owned();
        }
        BrainstormOp::AddItem {
            id,
            text,
            parent,
            origin,
        } => {
            if !kebab_ok(id) {
                return refuse("id", format!("item id {id:?} is not kebab-case"));
            }
            if doc.items.iter().any(|i| &i.id == id) {
                return refuse("id", format!("item {id:?} already exists"));
            }
            if text.trim().is_empty() {
                return refuse("text", "item text is empty".into());
            }
            if let Some(parent) = parent {
                if !doc.items.iter().any(|i| &i.id == parent) {
                    return refuse("parent", format!("parent {parent:?} does not exist"));
                }
            }
            doc.items.push(BrainstormItem {
                id: id.clone(),
                text: text.trim().to_owned(),
                parent: parent.clone(),
                cluster: None,
                origin: *origin,
            });
        }
        BrainstormOp::EditItem { id, text } => {
            if text.trim().is_empty() {
                return refuse("text", "item text is empty".into());
            }
            let Some(item) = doc.items.iter_mut().find(|i| &i.id == id) else {
                return refuse("id", format!("item {id:?} does not exist"));
            };
            item.text = text.trim().to_owned();
            item.origin = BrainstormOrigin::Operator;
        }
        BrainstormOp::RemoveItem { id } => {
            if !doc.items.iter().any(|i| &i.id == id) {
                return refuse("id", format!("item {id:?} does not exist"));
            }
            // Collect the subtree first, then remove — nothing dangles.
            let mut doomed: Vec<String> = vec![id.clone()];
            let mut cursor = 0;
            while cursor < doomed.len() {
                let parent = doomed[cursor].clone();
                for child in doc
                    .items
                    .iter()
                    .filter(|i| i.parent.as_deref() == Some(parent.as_str()))
                {
                    doomed.push(child.id.clone());
                }
                cursor += 1;
            }
            doc.items.retain(|i| !doomed.contains(&i.id));
        }
        BrainstormOp::AddCluster { id, label } => {
            if !kebab_ok(id) {
                return refuse("id", format!("cluster id {id:?} is not kebab-case"));
            }
            if doc.clusters.iter().any(|c| &c.id == id) {
                return refuse("id", format!("cluster {id:?} already exists"));
            }
            if label.trim().is_empty() {
                return refuse("label", "cluster label is empty".into());
            }
            doc.clusters.push(BrainstormCluster {
                id: id.clone(),
                label: label.trim().to_owned(),
            });
        }
        BrainstormOp::RenameCluster { id, label } => {
            if label.trim().is_empty() {
                return refuse("label", "cluster label is empty".into());
            }
            let Some(cluster) = doc.clusters.iter_mut().find(|c| &c.id == id) else {
                return refuse("id", format!("cluster {id:?} does not exist"));
            };
            cluster.label = label.trim().to_owned();
        }
        BrainstormOp::AssignCluster { id, cluster } => {
            if let Some(cluster) = cluster {
                if !doc.clusters.iter().any(|c| &c.id == cluster) {
                    return refuse("cluster", format!("cluster {cluster:?} does not exist"));
                }
            }
            let Some(item) = doc.items.iter_mut().find(|i| &i.id == id) else {
                return refuse("id", format!("item {id:?} does not exist"));
            };
            item.cluster = cluster.clone();
        }
        BrainstormOp::AdvancePhase => {
            let Some(next) = doc.phase.next() else {
                return refuse(
                    "phase",
                    "the board is at Converge — promote it, don't advance it".into(),
                );
            };
            match doc.phase {
                BrainstormPhase::Diverge => {
                    if doc.seed.trim().is_empty() {
                        return refuse("seed", "seed the idea before clustering".into());
                    }
                    if doc.items.len() < 2 {
                        return refuse(
                            "items",
                            "clustering needs at least two ideas on the board".into(),
                        );
                    }
                }
                BrainstormPhase::Cluster => {
                    let unsorted: Vec<&str> = doc
                        .items
                        .iter()
                        .filter(|i| i.parent.is_none() && i.cluster.is_none())
                        .map(|i| i.id.as_str())
                        .collect();
                    if doc.clusters.is_empty() {
                        return refuse("clusters", "converging needs at least one cluster".into());
                    }
                    if !unsorted.is_empty() {
                        return refuse(
                            "items",
                            format!("unsorted ideas remain: {}", unsorted.join(", ")),
                        );
                    }
                }
                BrainstormPhase::Converge => unreachable!("next() gated"),
            }
            doc.phase = next;
        }
    }
    Ok(())
}

// ── brainstorm scaffolds (the template that shapes the brainstorm) ──────

/// One registered brainstorm scaffold — the operator's "template to make
/// it brainstorming". v1 ships the breakdown arc; a new scaffold is one
/// census line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrainstormScaffold {
    pub id: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
}

/// THE scaffold census — one line per scaffold, nothing else enumerates.
pub fn brainstorm_scaffolds() -> Vec<BrainstormScaffold> {
    vec![BrainstormScaffold {
        id: "breakdown",
        label: "Breakdown",
        blurb: "Seed the massive idea, explode it, group what landed, order the survivors.",
    }]
}

/// Look one scaffold up by id.
pub fn brainstorm_scaffold(id: &str) -> Option<BrainstormScaffold> {
    brainstorm_scaffolds().into_iter().find(|s| s.id == id)
}

/// Mint a fresh board from a scaffold. The board opens at Diverge with
/// no seed — the first gesture IS the idea landing.
pub fn mint_brainstorm(scaffold: &BrainstormScaffold, id: &str) -> BrainstormDoc {
    BrainstormDoc {
        v: SCHEMA_V,
        id: id.to_owned(),
        template: scaffold.id.to_owned(),
        seed: String::new(),
        phase: BrainstormPhase::Diverge,
        items: Vec::new(),
        clusters: Vec::new(),
    }
}

// ── the distill (board → intake pre-fill; promotion feeds EX1) ──────────

/// One suggested intake answer — which field it pre-fills and the text
/// the operator will see (and may overrule) in the guided intake.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntakePrefill {
    pub field: String,
    pub answer: String,
}

/// Distill a CONVERGED board into explainer-intake pre-fill. Pure and
/// DETERMINISTIC — same board, same pre-fill. The bible mint stays
/// single-sited at the intake: brainstorm feeds intake, intake mints
/// bible. Refuses an unfinished board with every issue named.
pub fn distill(doc: &BrainstormDoc) -> Result<Vec<IntakePrefill>, Vec<Issue>> {
    let mut issues = validate_brainstorm(doc);
    if doc.phase != BrainstormPhase::Converge {
        issues.push(Issue {
            path: "phase".into(),
            message: format!(
                "promote from converge — the board is still at {}",
                doc.phase.label()
            ),
        });
    }
    if doc.seed.trim().is_empty() {
        issues.push(Issue {
            path: "seed".into(),
            message: "an unseeded board has nothing to promote".into(),
        });
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let key_points: Vec<String> = doc
        .clusters
        .iter()
        .flat_map(|cluster| {
            doc.items
                .iter()
                .filter(move |i| i.cluster.as_deref() == Some(cluster.id.as_str()))
                .map(|i| i.text.clone())
        })
        .collect();
    let beats: Vec<String> = doc.clusters.iter().map(|c| c.label.clone()).collect();
    Ok(vec![
        IntakePrefill {
            field: "subject".into(),
            answer: doc.seed.clone(),
        },
        IntakePrefill {
            field: "key-points".into(),
            answer: key_points.join("\n"),
        },
        IntakePrefill {
            field: "beats".into(),
            answer: beats.join("\n"),
        },
    ])
}

/// The staged pre-fill on disk (document law: persisted JSON carries
/// `v`). The guided intake (EX1-M3) reads this beside the board.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntakePrefillDoc {
    pub v: u32,
    pub answers: Vec<IntakePrefill>,
}

// ── the registry census ─────────────────────────────────────────────────

/// Every registered workflow, one line each — nothing else enumerates
/// workflows anywhere (the slides-registry census law). The storyboard
/// blueprint ships as the WF1 walking skeleton.
pub fn workflows() -> Vec<WorkflowDoc> {
    vec![storyboard(), explainer()]
}

/// Theme one up by id — the `--run-workflow` and panel entry.
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
        intake: None,
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

/// The EXPLAINER blueprint (EX1, T3): the guided one — its intake mints
/// the bible, the bible tailors the run. The default beats here are the
/// standard-format arc the BeatsDraft question opens with.
fn explainer() -> WorkflowDoc {
    let beat = |id: &str, role: &str, title: &str, brief: &str| Beat {
        id: id.into(),
        role: role.into(),
        title: title.into(),
        brief: brief.into(),
    };
    let ask = |id: &str, ask: &str, kind: IntakeFieldKind, required: bool| IntakeField {
        id: id.into(),
        ask: ask.into(),
        kind,
        required,
    };
    WorkflowDoc {
        v: SCHEMA_V,
        id: "explainer".into(),
        label: "Explainer video".into(),
        blurb: "Guided intake → production+script bible → tailored storyboard → video.".into(),
        surface: Surface::Wiredfront,
        intake: Some(IntakeSpec {
            fields: vec![
                ask(
                    "subject",
                    "What is this explainer about? One or two sentences.",
                    IntakeFieldKind::Text,
                    true,
                ),
                ask(
                    "audience",
                    "Who is it for? What do they already know?",
                    IntakeFieldKind::Text,
                    true,
                ),
                ask(
                    "tone",
                    "What tone and voice should it carry?",
                    IntakeFieldKind::Text,
                    true,
                ),
                ask(
                    "style-refs",
                    "Any style references worth naming? (optional)",
                    IntakeFieldKind::Text,
                    false,
                ),
                ask(
                    "key-points",
                    "Key points that MUST land, one per line. (optional)",
                    IntakeFieldKind::Text,
                    false,
                ),
                ask(
                    "avoid",
                    "Anything to avoid — claims, jargon, topics? (optional)",
                    IntakeFieldKind::Text,
                    false,
                ),
                ask(
                    "format",
                    "How long should it run?",
                    IntakeFieldKind::FormatPick,
                    true,
                ),
                ask(
                    "beats",
                    "Here's a draft arc — tune the beats and their narration.",
                    IntakeFieldKind::BeatsDraft,
                    true,
                ),
            ],
        }),
        stages: vec![
            Stage {
                id: "outline".into(),
                kind: StageKind::Outline {
                    beats: vec![
                        beat("hook", "hook", "The hook", "Name the pain in one line."),
                        beat(
                            "promise",
                            "context",
                            "The promise",
                            "What they'll be able to do.",
                        ),
                        beat("context", "context", "The setup", "Why this matters, fast."),
                        beat(
                            "step-1",
                            "beat",
                            "First move",
                            "The first concrete step, shown.",
                        ),
                        beat(
                            "step-2",
                            "beat",
                            "Second move",
                            "The next step, building on it.",
                        ),
                        beat(
                            "payoff",
                            "beat",
                            "The payoff",
                            "The result, proven on screen.",
                        ),
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

    if let Some(intake) = &doc.intake {
        if intake.fields.is_empty() {
            push("intake".into(), "an intake needs at least one field".into());
        }
        let mut seen_field = std::collections::BTreeSet::new();
        let mut format_picks = 0usize;
        let mut beats_drafts = 0usize;
        for f in &intake.fields {
            let at = format!("intake.{}", f.id);
            if !kebab_ok(&f.id) {
                push(at.clone(), format!("field id {:?} is not kebab-case", f.id));
            }
            if !seen_field.insert(f.id.clone()) {
                push(at.clone(), "duplicate field id".into());
            }
            if f.ask.trim().is_empty() {
                push(format!("{at}.ask"), "ask is empty".into());
            }
            match &f.kind {
                IntakeFieldKind::Choice { options } => {
                    if options.is_empty() {
                        push(format!("{at}.options"), "choice has no options".into());
                    }
                }
                IntakeFieldKind::FormatPick => format_picks += 1,
                IntakeFieldKind::BeatsDraft => beats_drafts += 1,
                IntakeFieldKind::Text => {}
            }
        }
        if format_picks > 1 {
            push("intake".into(), "at most one format-pick field".into());
        }
        if beats_drafts > 1 {
            push("intake".into(), "at most one beats-draft field".into());
        }
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
    /// The write guard refused: the library is unplugged (ST2b).
    Unplugged(String),
    Parse(serde_json::Error),
    /// The file's `v` is newer than this build understands.
    Version(u32),
}

impl std::fmt::Display for DocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocError::Io(e) => write!(f, "io: {e}"),
            DocError::Unplugged(why) => f.write_str(why),
            DocError::Parse(e) => write!(f, "parse: {e}"),
            DocError::Version(v) => {
                write!(f, "document is schema v{v}; this build reads v{SCHEMA_V}")
            }
        }
    }
}

impl std::error::Error for DocError {}

const BOARD_FILE: &str = "storyboard.json";
const WORKFLOW_FILE: &str = "workflow.json";
const BIBLE_FILE: &str = "bible.json";
const BRAINSTORM_FILE: &str = "brainstorm.json";
const PREFILL_FILE: &str = "intake-prefill.json";

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
    save_json(dir, BOARD_FILE, board)
}

/// Load `<dir>/workflow.json` (a tailored per-run instance), refusing a
/// newer schema honestly.
pub fn load_workflow(dir: &Path) -> Result<WorkflowDoc, DocError> {
    let raw = std::fs::read_to_string(dir.join(WORKFLOW_FILE)).map_err(DocError::Io)?;
    let doc: WorkflowDoc = serde_json::from_str(&raw).map_err(DocError::Parse)?;
    if doc.v > SCHEMA_V {
        return Err(DocError::Version(doc.v));
    }
    Ok(doc)
}

/// Crash-safe write of a tailored instance into `<dir>/workflow.json`.
pub fn save_workflow(dir: &Path, doc: &WorkflowDoc) -> Result<(), DocError> {
    save_json(dir, WORKFLOW_FILE, doc)
}

/// Load `<dir>/bible.json`, refusing a newer schema honestly.
pub fn load_bible(dir: &Path) -> Result<BibleDoc, DocError> {
    let raw = std::fs::read_to_string(dir.join(BIBLE_FILE)).map_err(DocError::Io)?;
    let bible: BibleDoc = serde_json::from_str(&raw).map_err(DocError::Parse)?;
    if bible.v > SCHEMA_V {
        return Err(DocError::Version(bible.v));
    }
    Ok(bible)
}

/// Crash-safe write of the bible into `<dir>/bible.json`.
pub fn save_bible(dir: &Path, bible: &BibleDoc) -> Result<(), DocError> {
    save_json(dir, BIBLE_FILE, bible)
}

/// Load `<dir>/brainstorm.json`, refusing a newer schema honestly.
pub fn load_brainstorm(dir: &Path) -> Result<BrainstormDoc, DocError> {
    let raw = std::fs::read_to_string(dir.join(BRAINSTORM_FILE)).map_err(DocError::Io)?;
    let doc: BrainstormDoc = serde_json::from_str(&raw).map_err(DocError::Parse)?;
    if doc.v > SCHEMA_V {
        return Err(DocError::Version(doc.v));
    }
    Ok(doc)
}

/// Crash-safe write of the board into `<dir>/brainstorm.json`.
pub fn save_brainstorm(dir: &Path, doc: &BrainstormDoc) -> Result<(), DocError> {
    save_json(dir, BRAINSTORM_FILE, doc)
}

/// Load `<dir>/intake-prefill.json`, refusing a newer schema honestly.
pub fn load_intake_prefill(dir: &Path) -> Result<IntakePrefillDoc, DocError> {
    let raw = std::fs::read_to_string(dir.join(PREFILL_FILE)).map_err(DocError::Io)?;
    let doc: IntakePrefillDoc = serde_json::from_str(&raw).map_err(DocError::Parse)?;
    if doc.v > SCHEMA_V {
        return Err(DocError::Version(doc.v));
    }
    Ok(doc)
}

/// Crash-safe write of the staged intake pre-fill beside the board.
pub fn save_intake_prefill(dir: &Path, answers: &[IntakePrefill]) -> Result<(), DocError> {
    let doc = IntakePrefillDoc {
        v: SCHEMA_V,
        answers: answers.to_vec(),
    };
    save_json(dir, PREFILL_FILE, &doc)
}

/// The ONE crash-safe pretty-JSON+LF writer (tmp → bak-step-aside →
/// swap) all four run documents share.
fn save_json<T: Serialize>(dir: &Path, file: &str, value: &T) -> Result<(), DocError> {
    let dir = &dpl_doc::writable(dir).map_err(|e| DocError::Unplugged(e.to_string()))?;
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
        assert_eq!(all.len(), 2, "WF1 storyboard + EX1 explainer");
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
            intake: None,
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
        assert_eq!(slide.component, "title@1");
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

    /// Build a board the way the surface will: mint, then ops through
    /// every phase — the fixture IS an integration proof of `apply`.
    fn worked_board() -> BrainstormDoc {
        let scaffold = brainstorm_scaffold("breakdown").expect("registered");
        let mut doc = mint_brainstorm(&scaffold, "launch-video");
        let ops = [
            BrainstormOp::Seed {
                seed: "One video that explains the whole product".into(),
            },
            BrainstormOp::AddItem {
                id: "pain".into(),
                text: "Name the pain".into(),
                parent: None,
                origin: BrainstormOrigin::Operator,
            },
            BrainstormOp::AddItem {
                id: "demo".into(),
                text: "Show it working".into(),
                parent: None,
                origin: BrainstormOrigin::Assist,
            },
            BrainstormOp::AddItem {
                id: "demo-fail".into(),
                text: "Show the old way failing first".into(),
                parent: Some("demo".into()),
                origin: BrainstormOrigin::Assist,
            },
            BrainstormOp::AdvancePhase,
            BrainstormOp::AddCluster {
                id: "open".into(),
                label: "The opening".into(),
            },
            BrainstormOp::AddCluster {
                id: "proof".into(),
                label: "The proof".into(),
            },
            BrainstormOp::AssignCluster {
                id: "pain".into(),
                cluster: Some("open".into()),
            },
            BrainstormOp::AssignCluster {
                id: "demo".into(),
                cluster: Some("proof".into()),
            },
            BrainstormOp::AdvancePhase,
        ];
        for op in &ops {
            apply(&mut doc, op).expect("fixture ops are legal");
        }
        doc
    }

    #[test]
    fn the_brainstorm_scaffold_registry_is_the_census() {
        let all = brainstorm_scaffolds();
        assert_eq!(all.len(), 1, "v1 ships the breakdown scaffold");
        let mut ids = std::collections::BTreeSet::new();
        for s in &all {
            assert!(ids.insert(s.id), "duplicate scaffold id {}", s.id);
            assert!(!s.label.is_empty() && !s.blurb.is_empty());
            let minted = mint_brainstorm(s, "census-proof");
            assert!(
                validate_brainstorm(&minted).is_empty(),
                "a scaffold must mint a clean board"
            );
        }
        assert!(brainstorm_scaffold("ghost").is_none());
    }

    #[test]
    fn a_worked_board_is_deterministic_and_ops_are_all_or_nothing() {
        let a = serde_json::to_string(&worked_board()).unwrap();
        let b = serde_json::to_string(&worked_board()).unwrap();
        assert_eq!(a, b, "same ops, same board — bytes included");
        let mut doc = worked_board();
        let before = serde_json::to_string(&doc).unwrap();
        let refused = apply(
            &mut doc,
            &BrainstormOp::AssignCluster {
                id: "pain".into(),
                cluster: Some("ghost".into()),
            },
        );
        assert!(refused.is_err());
        assert_eq!(
            serde_json::to_string(&doc).unwrap(),
            before,
            "a refused op mutates NOTHING"
        );
    }

    #[test]
    fn apply_refuses_what_it_cannot_do() {
        let scaffold = brainstorm_scaffold("breakdown").unwrap();
        let mut doc = mint_brainstorm(&scaffold, "refusals");
        // Leaving Diverge before the idea landed.
        let err = apply(&mut doc, &BrainstormOp::AdvancePhase).unwrap_err();
        assert!(err.iter().any(|i| i.path == "seed"));
        apply(
            &mut doc,
            &BrainstormOp::Seed {
                seed: "the idea".into(),
            },
        )
        .unwrap();
        let err = apply(&mut doc, &BrainstormOp::AdvancePhase).unwrap_err();
        assert!(err.iter().any(|i| i.path == "items"), "needs two ideas");
        // Unknown refs and duplicates name themselves.
        let add = |id: &str| BrainstormOp::AddItem {
            id: id.into(),
            text: format!("idea {id}"),
            parent: None,
            origin: BrainstormOrigin::Operator,
        };
        apply(&mut doc, &add("one")).unwrap();
        assert!(apply(&mut doc, &add("one")).is_err(), "duplicate id");
        assert!(apply(
            &mut doc,
            &BrainstormOp::AddItem {
                id: "orphan".into(),
                text: "x".into(),
                parent: Some("ghost".into()),
                origin: BrainstormOrigin::Operator,
            }
        )
        .is_err());
        assert!(apply(
            &mut doc,
            &BrainstormOp::EditItem {
                id: "ghost".into(),
                text: "x".into()
            }
        )
        .is_err());
        apply(&mut doc, &add("two")).unwrap();
        apply(&mut doc, &BrainstormOp::AdvancePhase).unwrap();
        // Leaving Cluster with unsorted ideas, then without any cluster.
        let err = apply(&mut doc, &BrainstormOp::AdvancePhase).unwrap_err();
        assert!(err.iter().any(|i| i.path == "clusters"));
        apply(
            &mut doc,
            &BrainstormOp::AddCluster {
                id: "only".into(),
                label: "Only".into(),
            },
        )
        .unwrap();
        let err = apply(&mut doc, &BrainstormOp::AdvancePhase).unwrap_err();
        assert!(
            err.iter().any(|i| i.message.contains("unsorted")),
            "unsorted ideas are named"
        );
        for id in ["one", "two"] {
            apply(
                &mut doc,
                &BrainstormOp::AssignCluster {
                    id: id.into(),
                    cluster: Some("only".into()),
                },
            )
            .unwrap();
        }
        apply(&mut doc, &BrainstormOp::AdvancePhase).unwrap();
        // Past Converge is a refusal, not a wrap.
        let err = apply(&mut doc, &BrainstormOp::AdvancePhase).unwrap_err();
        assert!(err.iter().any(|i| i.message.contains("promote")));
    }

    #[test]
    fn an_operator_edit_claims_assist_work() {
        let mut doc = worked_board();
        assert_eq!(
            doc.items.iter().find(|i| i.id == "demo").unwrap().origin,
            BrainstormOrigin::Assist
        );
        apply(
            &mut doc,
            &BrainstormOp::EditItem {
                id: "demo".into(),
                text: "Show it working, live".into(),
            },
        )
        .unwrap();
        assert_eq!(
            doc.items.iter().find(|i| i.id == "demo").unwrap().origin,
            BrainstormOrigin::Operator,
            "touched work is the operator's work"
        );
    }

    #[test]
    fn removal_cascades_the_breakdown() {
        let mut doc = worked_board();
        apply(&mut doc, &BrainstormOp::RemoveItem { id: "demo".into() }).unwrap();
        assert!(
            !doc.items.iter().any(|i| i.id == "demo-fail"),
            "the subtree goes with its root — nothing dangles"
        );
        assert!(validate_brainstorm(&doc).is_empty());
    }

    #[test]
    fn brainstorm_round_trips_crash_safe() {
        let dir = scratch("brainstorm-roundtrip");
        let doc = worked_board();
        save_brainstorm(&dir, &doc).unwrap();
        assert_eq!(load_brainstorm(&dir).unwrap(), doc);
        assert!(!dir.join("brainstorm.json.tmp").exists());
        assert!(!dir.join("brainstorm.json.bak").exists());
        let mut newer = doc.clone();
        newer.v = SCHEMA_V + 1;
        save_brainstorm(&dir, &newer).unwrap();
        assert!(matches!(load_brainstorm(&dir), Err(DocError::Version(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn distill_fills_the_explainer_intake() {
        let prefill = distill(&worked_board()).unwrap();
        let intake = explainer().intake.expect("explainer has an intake");
        for p in &prefill {
            assert!(
                intake.fields.iter().any(|f| f.id == p.field),
                "prefill field {:?} is not an explainer intake field",
                p.field
            );
            assert!(!p.answer.trim().is_empty());
        }
        let subject = prefill.iter().find(|p| p.field == "subject").unwrap();
        assert_eq!(subject.answer, "One video that explains the whole product");
        let beats = prefill.iter().find(|p| p.field == "beats").unwrap();
        assert_eq!(
            beats.answer, "The opening\nThe proof",
            "beats ride the cluster order"
        );
        assert_eq!(distill(&worked_board()).unwrap(), prefill, "deterministic");
    }

    #[test]
    fn intake_prefill_round_trips_crash_safe() {
        let dir = scratch("prefill-roundtrip");
        let prefill = distill(&worked_board()).unwrap();
        save_intake_prefill(&dir, &prefill).unwrap();
        assert_eq!(load_intake_prefill(&dir).unwrap().answers, prefill);
        assert!(!dir.join("intake-prefill.json.tmp").exists());
        let newer = serde_json::json!({ "v": SCHEMA_V + 1, "answers": [] });
        std::fs::write(
            dir.join("intake-prefill.json"),
            serde_json::to_string_pretty(&newer).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            load_intake_prefill(&dir),
            Err(DocError::Version(_))
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn distill_refuses_an_unconverged_board() {
        let scaffold = brainstorm_scaffold("breakdown").unwrap();
        let doc = mint_brainstorm(&scaffold, "too-early");
        let err = distill(&doc).unwrap_err();
        assert!(err.iter().any(|i| i.path == "phase"));
        assert!(err.iter().any(|i| i.path == "seed"));
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
            doc["slots"][0]["fill"] = serde_json::json!({"kind": "artifact", "rel": bad_rel});
            assert!(
                !v.is_valid(&doc),
                "jailbreak rel {bad_rel:?} must be refused"
            );
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

    fn fixture_bible() -> BibleDoc {
        let beat = |id: &str, title: &str, narration: &str| BibleBeat {
            id: id.into(),
            role: "beat".into(),
            title: title.into(),
            brief: "shown on screen".into(),
            narration: narration.into(),
        };
        BibleDoc {
            v: SCHEMA_V,
            id: "rust-traits".into(),
            workflow: "explainer".into(),
            subject: "What Rust traits are and when to reach for them".into(),
            audience: "Working devs new to Rust".into(),
            tone: "Confident, plain-spoken, no hype".into(),
            style_refs: vec!["fireship pacing".into()],
            key_points: vec!["traits are contracts".into()],
            avoid: vec!["monad jokes".into()],
            format: "short".into(),
            beats: vec![
                beat(
                    "hook",
                    "The hook",
                    "You keep writing the same function five times.",
                ),
                beat(
                    "idea",
                    "One contract",
                    "A trait is the contract; types opt in.",
                ),
                beat(
                    "payoff",
                    "The payoff",
                    "One impl block later, it just works.",
                ),
                beat("cta", "Try it", "Take one duplicated function and lift it."),
            ],
        }
    }

    #[test]
    fn a_minted_bible_matches_the_v1_wire_schema() {
        let v = validator("bible.v1.schema.json");
        let doc = serde_json::to_value(fixture_bible()).expect("serialize");
        assert_valid(&v, &doc, "fixture bible");
    }

    #[test]
    fn the_bible_schema_refuses_drift() {
        let v = validator("bible.v1.schema.json");
        let good = serde_json::to_value(fixture_bible()).unwrap();

        // A silent beat breaks the script-bible law at the contract.
        let mut doc = good.clone();
        doc["beats"][0]["narration"] = serde_json::json!("");
        assert!(!v.is_valid(&doc), "empty narration must be refused");

        let mut doc = good.clone();
        doc["id"] = serde_json::json!("Not Kebab");
        assert!(!v.is_valid(&doc), "non-kebab id must be refused");

        let mut doc = good.clone();
        doc["vibes"] = serde_json::json!("immaculate");
        assert!(!v.is_valid(&doc), "unknown fields must be refused");

        let mut doc = good;
        doc["v"] = serde_json::json!(2);
        assert!(!v.is_valid(&doc), "v2 must not pass the v1 schema");
    }

    #[test]
    fn the_explainer_intake_rides_the_amended_v1_schema() {
        // The ADDITIVE amendment: blueprints WITH intake validate, and
        // pre-EX1 documents (no intake) stay valid — both proven by
        // every_registered_workflow_matches_the_v1_wire_schema covering
        // storyboard (no intake) and explainer (intake) alike. Here:
        // the intake vocabulary itself refuses drift.
        let v = validator("workflow.v1.schema.json");
        let good = serde_json::to_value(find("explainer").unwrap()).unwrap();
        assert_valid(&v, &good, "explainer blueprint with intake");

        let mut doc = good.clone();
        doc["intake"]["fields"][0]["kind"] = serde_json::json!("mind-reading");
        assert!(!v.is_valid(&doc), "unknown intake kind must be refused");

        let mut doc = good;
        doc["intake"]["fields"][0]["mood"] = serde_json::json!("neon");
        assert!(
            !v.is_valid(&doc),
            "unknown intake field props must be refused"
        );
    }

    #[test]
    fn tailor_mints_a_schema_valid_deterministic_doc() {
        let v = validator("workflow.v1.schema.json");
        let bible = fixture_bible();
        let a = tailor(&bible).expect("valid bible tailors");
        let b = tailor(&bible).expect("valid bible tailors");
        assert_eq!(a, b, "tailor is deterministic");
        assert_valid(&v, &serde_json::to_value(&a).unwrap(), "tailored instance");
        assert!(validate(&a).is_empty(), "tailored doc validates clean");
        assert_eq!(a.id, "explainer-rust-traits");
    }

    #[test]
    fn tailor_short_and_long_differ_only_on_the_declared_axis() {
        let bible = fixture_bible();
        let mut long = bible.clone();
        long.format = "long".into();
        let a = tailor(&bible).unwrap();
        let b = tailor(&long).unwrap();
        // Same beats, same stage ids, same geometry (both 16:9 in v1) —
        // the axis lives in the profile the run reads (per-beat span,
        // M2) and the label.
        let ids = |d: &WorkflowDoc| d.stages.iter().map(|s| s.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&a), ids(&b));
        assert_ne!(a.label, b.label, "the format names itself");
    }

    #[test]
    fn tailor_refuses_an_invalid_bible_naming_issues() {
        let mut bible = fixture_bible();
        bible.format = "imax".into();
        bible.beats[0].narration = String::new();
        let issues = tailor(&bible).unwrap_err();
        assert!(issues.iter().any(|i| i.path == "format"));
        assert!(issues.iter().any(|i| i.path.ends_with(".narration")));
    }

    #[test]
    fn bible_and_workflow_docs_round_trip_crash_safe() {
        let dir = std::env::temp_dir()
            .join("dpl-workflows-tests")
            .join(format!("ex1-docs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let bible = fixture_bible();
        save_bible(&dir, &bible).unwrap();
        assert_eq!(load_bible(&dir).unwrap(), bible);
        let doc = tailor(&bible).unwrap();
        save_workflow(&dir, &doc).unwrap();
        assert_eq!(load_workflow(&dir).unwrap(), doc);
        // Newer-schema refusal, both documents.
        let mut newer = bible.clone();
        newer.v = SCHEMA_V + 1;
        save_bible(&dir, &newer).unwrap();
        assert!(matches!(load_bible(&dir), Err(DocError::Version(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_format_census_is_the_census() {
        let all = format_profiles();
        assert_eq!(all.len(), 3, "short + standard + long (v1, all 16:9)");
        let mut ids = std::collections::BTreeSet::new();
        for p in &all {
            assert!(ids.insert(p.id), "duplicate profile id {}", p.id);
            assert!(!p.label.is_empty() && !p.blurb.is_empty());
            assert!(p.fps >= 1 && p.w >= 1 && p.h >= 1 && p.per_beat_secs > 0.0);
            assert!(p.beat_budget >= 1);
        }
        assert!(format_profile("ghost").is_none());
    }

    fn fixture_board(phase: BrainstormPhase) -> BrainstormDoc {
        BrainstormDoc {
            v: SCHEMA_V,
            id: "launch-video".into(),
            template: "breakdown".into(),
            seed: "One video that explains the whole product".into(),
            phase,
            items: vec![
                BrainstormItem {
                    id: "pain".into(),
                    text: "Name the pain".into(),
                    parent: None,
                    cluster: Some("open".into()),
                    origin: BrainstormOrigin::Operator,
                },
                BrainstormItem {
                    id: "demo-fail".into(),
                    text: "Show the old way failing first".into(),
                    parent: Some("pain".into()),
                    cluster: None,
                    origin: BrainstormOrigin::Assist,
                },
            ],
            clusters: vec![BrainstormCluster {
                id: "open".into(),
                label: "The opening".into(),
            }],
        }
    }

    #[test]
    fn a_minted_and_worked_brainstorm_matches_the_v1_wire_schema() {
        let v = validator("brainstorm.v1.schema.json");
        let scaffold = brainstorm_scaffold("breakdown").expect("registered");
        let minted = serde_json::to_value(mint_brainstorm(&scaffold, "fresh")).unwrap();
        assert_valid(&v, &minted, "a freshly minted board");
        for phase in [
            BrainstormPhase::Diverge,
            BrainstormPhase::Cluster,
            BrainstormPhase::Converge,
        ] {
            let doc = serde_json::to_value(fixture_board(phase)).unwrap();
            assert_valid(&v, &doc, "a worked board at every phase");
        }
    }

    #[test]
    fn the_brainstorm_schema_refuses_drift() {
        let v = validator("brainstorm.v1.schema.json");
        let good = serde_json::to_value(fixture_board(BrainstormPhase::Cluster)).unwrap();
        assert_valid(&v, &good, "the drift fixture starts valid");

        let mut doc = good.clone();
        doc["phase"] = serde_json::json!("vibing");
        assert!(!v.is_valid(&doc), "an unknown phase must be refused");

        let mut doc = good.clone();
        doc["items"][0]["origin"] = serde_json::json!("gremlin");
        assert!(!v.is_valid(&doc), "an unknown origin must be refused");

        let mut doc = good.clone();
        doc["id"] = serde_json::json!("Not Kebab");
        assert!(!v.is_valid(&doc), "a non-kebab id must be refused");

        let mut doc = good.clone();
        doc["vibes"] = serde_json::json!("immaculate");
        assert!(!v.is_valid(&doc), "an unknown root field is drift");

        let mut doc = good.clone();
        doc["items"][0]["text"] = serde_json::json!("");
        assert!(!v.is_valid(&doc), "an empty item text must be refused");

        let mut doc = good;
        doc["v"] = serde_json::json!(2);
        assert!(!v.is_valid(&doc), "v2 must not pass the v1 contract");
    }
}
