# WF1 — the workflow engine + storyboarding walking skeleton

**Status:** RATIFIED 2026-07-27 — next pass; poppler staging joins step 1
**Date:** 2026-07-27
**Program:** [`wiredfront-workflows-program.md`](wiredfront-workflows-program.md) (T2; decisions D3/D4/D5/D6 bind)
**Provenance:** §2/§6 draw on the deep-plan Opus sweeps (spot-checked); all
decisions are Fable's.

## 1. Context

The operator commissioned tailored, methodical workflows ("generate
properly and methodically… storyboarding, then the evolution to the full
show/episode") on an editor that is AI-drivable through the fabric. D3:
a workflow is a typed, declarative document executed by the app, each stage
publishing `jobs.*` facts — never bespoke code paths. WF1 builds that
engine and proves it with ONE workflow: **storyboarding** — outline beats →
slide drafts (catalog templates, optional Imagine image per beat under
ADR-011) → comp assembly → preview in the editor.

## 2. Current state (evidence, spot-checked)

- No workflow concept exists anywhere in the workspace.
- `dpl-composer` `Tab` enum = Editor/Capture/Annotate/Narrate/Guide
  (lib.rs:355-361) rendered by `fn workflow_tabs` (:1898) — a live naming
  collision with this pass.
- Slide minting exists: W11's `media_frame` + `make_slide_from_rel`
  (wiredfront hub), `place_slide` (W10a); 28 registered slide designs
  (#97) with the census gate `every_demo_renders_at_start_and_at_rest`
  (crates/slides/src/registry.rs:147).
- Comp assembly exists: `dpl_comp` document + doctor + crash-safe save;
  composer `render_stitch` path; seam `POST /comp` + `render`.
- Imagine generation exists (#98): `dpl-chat-imagine` cache + `import_generated`.
- Fabric `jobs` domain (Queued/Done/Failed) is coach-declared and on the
  seam SSE.
- Harness precedent: `--make-slide` boot-seam flag printing a verdict line
  (no process::exit; paired with `--screenshot` to terminate).
- CLAUDE.md's harness-flag list is stale (T0 fixes ride this PR).

## 3. Target design

New pure crate **`dpl-workflows`** (deps ceiling: serde, serde_json,
dpl-comp, dpl-doc; `publish = false`, workspace lints):

- `WorkflowDoc { schema_v: 1, id, label, blurb, stages: Vec<Stage> }`,
  kebab-case serde, crash-safe save, newer-schema refusal, named
  validation issues (unknown stage kind, dangling refs, jailed paths).
- `StageKind` v1 — exactly what the skeleton needs, registered in ONE
  `stages()` census: `Outline` (typed beats), `SlideFromBeat` (template id
  + text per beat), `GenerateImage` (optional; ADR-011; skips honestly
  when no key/dry-run), `AssembleComp` (slides/media → comp.json layers),
  `Render` (existing stitch).
- Execution: the pure crate owns planning/validation/dry-run; the app
  implements a `StageRunner` seam (trait object) wiring real minting,
  Imagine, comp save, render. One run at a time; every stage publishes
  `JobEvent::Queued/Done/Failed` labeled `workflow <id> · <stage>`;
  artifacts land under the content root with no-clobber minting;
  completion publishes `CaptureEvent::InboxChanged`.
- **Workflows tab** in the composer (sixth `Tab` variant; rename
  `workflow_tabs` → `tab_strip`): list registered workflows (census-read,
  never hardcoded), Run chip, per-stage state chips, honest failure text.
  Experience-gated per wiredfront-experience.md §7 (tooltips, touch, ColdLight).
- Harness: `--run-workflow <id>` boot-seam flag, `--make-slide`-shaped
  verdict line; storyboard workflow ships as a registered default (a
  workflow is an ingredient — D6).

### 3a. The slotted storyboard (operator refinement, 2026-07-27 — binds §3)

> "…truly creating a frictionless/guided experience where we can have AI
> easily understand what slots are remaining within the storyboard
> template, and the storyboard has a system where it gets tailored for
> that project at the time, configurable so we can customize like my raw
> content where I can associate with new or existing episodes/projects."

Four requirements built into the first schema — never bolted on:

1. **Slots are the storyboard's unit.** `StoryboardDoc` = project/episode
   ref + `Vec<Slot { id, role (hook/context/beat/cta…), brief (what this
   slot needs — human- AND agent-readable), constraints, fill }>` with
   typed fill: `Empty | Artifact(rel) | Slide(ref) | Draft(imagine id)`.
   "Slots remaining" is a derived query on the document, never prose.
2. **Machine-legible fill state.** Every slot is an EguiPilot semantic
   node (`wiredfront.storyboard.slot/<id>`, value = fill state,
   description = brief) so chat/coach/A2UI read the same truth the panel
   paints; filling a slot publishes a fabric fact. The seam GET for slot
   inventory is T4b — WF1's job is the document shape that makes it
   trivial.
3. **Tailored at mint.** A storyboard is minted from a blueprint +
   project config (platform target, beat count, bible/style refs) through
   a guided mint step. The blueprint is the ingredient; the minted
   storyboard is the project-specific instance.
4. **Raw content associates in.** A slot accepts existing library/inbox
   artifacts — associated into a NEW or EXISTING episode/project via the
   W10a `place_slide`/never-clobber machinery and the W9 landing policy —
   or a fresh generation. Same gesture, either source.

**The gate:** frictionless means every empty slot names what it needs,
filling one is one gesture, and the agent sees exactly what the operator
sees. The storyboard workflow (`Outline → SlideFromBeat → …`) is framed
accordingly: `Outline` mints the tailored slotted storyboard; subsequent
stages FILL slots; `AssembleComp` consumes the filled set.

**Two workflow surfaces (operator, same session):**

> "…we have wiredfront/workflows and then workbench/workflows where we can
> then even storyboard out the episodes with the fully built slides and
> then get the teleprompter script developed, to be associated with it."

- **One engine, two route surfaces.** `dpl-workflows` is shared; each hub
  mounts a `workflows` sub-route and surfaces the workflows whose
  artifacts belong to its side of the two-libraries law:
  **wiredfront/workflows** = raw-media pipelines (clips, stitch, post,
  carousel media); **workbench/workflows** = slide/episode pipelines —
  storyboarding episodes out of FULLY BUILT catalog slides (the ColdLight
  side), slots filling with registered slides/templates rather than
  footage. `WorkflowDoc` therefore declares its artifact side (a `surface`
  field); the engine and document make NO hub assumptions.
- **Teleprompter script is a first-class associated artifact.** A
  workbench workflow stage develops the per-slide/per-beat script and
  associates it with the episode/storyboard (same association machinery,
  never-clobber), so the presenter/deck can drive it — the script rides
  the episode, slot-aligned, agent-legible like every other slot fill.
  (The Narrate tab stub and deck Beat facts are the existing anchors.)

WF1 impact: the panel lands under wiredfront first; the workbench mount +
teleprompter stage are the immediate follow-on pass. WF1 ships the
`StoryboardDoc` shape + the `Outline` stage minting it (tailoring inputs
carried in the workflow document's stage config), with the EguiPilot slot
nodes on the Workflows panel.

## 4. Changes, in execution order

1. `Cargo.toml`: member + workspace dep `dpl-workflows`.
2. `crates/workflows/` (new): document, registry census, validation,
   dry-run executor, `StageRunner` trait; inline tests per house naming.
3. `crates/composer/src/lib.rs`: rename `workflow_tabs`→`tab_strip`; add
   `Tab::Workflows` + panel; `StageRunner` impl wiring
   `make_slide_from_rel`/doc mint, `dpl_chat_imagine` (via app seam),
   comp save, render path; run-state chips.
   (Exact signatures resolved at execution grounding — the runner wiring
   must re-read the current mint entry points in-session.)
4. `crates/app/src/main.rs`: `--run-workflow <id>` parse + boot seam;
   CLAUDE.md harness list fix (T0).
5. `docs/plans/wiredfront-experience.md`: T0 table/ladder corrections.
6. Fixture `[fixtures.wiredfront-workflows]` + scenario
   `tests/eguipilotai/wiredfront-workflows.toml`.
7. Docs: STATE tip; this §8.

## 5. Boundaries

- 🚫 Never: new fabric variants (jobs/capture suffice); seam workflow
  verbs (T4b); C3 surfaces; auto/retried generation spend (ADR-011 —
  `GenerateImage` runs only inside an operator-started run, dry-run
  default in drills); touching `/content`; `#[allow]`; second workflow
  hardcoded outside the registry.
- ⚠️ Ask first: any new dependency; audio/keyframe schema changes (T1b's
  job); poppler staging.
- Out of scope: how-to/carousel/episode workflows (T3), seam/coach drive
  (T4), Imagine video.

## 6. Verification (adopted from the deep-plan draft, trimmed by Fable)

1. `cargo tree -p dpl-workflows -e normal --depth 1` — deps ⊆ {serde,
   serde_json, dpl-comp, dpl-doc}; FAIL on egui/composer/hub crates.
2. `cargo test -p dpl-workflows` — round-trip, pretty-LF-ref-shaped save,
   newer-schema refusal, named structural faults, jailed refs,
   `the_registry_is_the_census`, `every_stage_executes_dry_and_reports_its_verdict`,
   failed-stage-halts-and-names, rerun-is-no-clobber.
3. `cargo test -p dpl-composer` — ≥ current count; panel lists only
   registered workflows; `grep "fn workflow_tabs"` → 0 hits.
4. `cargo test -p dpl-doc -p dpl-comp -p dpl-slides` — slides census
   intact (28 + any storyboard template added, gate still paints).
5. `cargo green` — all phases (spawn audit covers any new subprocess).
6. Headless E2E on scratch root (W11 pattern): `--content-root
   $TEMP\wf1-scratch\content --run-workflow storyboard --screenshot … --window
   1280x800` → verdict line `run-workflow: storyboard ok · N stage(s) …`;
   per-beat `slide.json` loads via `dpl_doc::load`; `comp.json` validates
   clean; while running, seam SSE shows `jobs.queued/done` per stage.
7. Honest absence: `--run-workflow no-such-workflow` → stderr names the id
   AND lists registered ones; nothing minted; no panic.
8. Panel shots at 1280x800 AND 1600x1000 (never maximized-only); EguiPilot
   scenario passes; the 16 existing scenarios stay green (sequential);
   baseline compare per true-verify.
9. Release exe rebuilt; mtime moved.

## 7. Commit plan

Branch `feat/wf1-workflow-engine` → PR → squash. Commit:
`feat(wiredfront): WF1 — workflow engine + storyboarding walking skeleton`.
Includes T0 doc corrections. Push runs the gate; session-end trio after
merge.

## 8. Evidence

(filled at execution)
