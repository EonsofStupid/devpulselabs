# THE WORKFLOW DOCTRINE — documents, not code paths; one shape for every workflow PR

Status: LAW for every workflow the platform grows (operator directive
2026-07-28: consistent patterns, industry-grade schemas, built for scale).
A workflow PR that doesn't fit this shape is wrong by definition — the
same standing `signal-doctrine.md` has for signals.

## Where this comes from

WF1 (`docs-history-2026-10-12:docs/plans/wf1-workflow-engine.md`, `crates/workflows`) established
the founding decision: **a workflow is a typed, declarative DOCUMENT
executed by one engine — never a bespoke code path.** This doctrine
ratifies the patterns WF1 proved and adds the wire contract that keeps
the Nth workflow shaped like the 1st.

## The laws

1. **The registry census.** Every workflow is one `WorkflowDoc` literal
   in `dpl_workflows::workflows()` — nothing else enumerates workflows,
   anywhere (the slides-registry law). Adding the Nth workflow is adding
   ONE entry there; panels, `--run-workflow`, agents, and jobs facts all
   see it with zero new plumbing.

2. **Versioned documents, refusal over misreading.** Both documents
   carry `v` (`SCHEMA_V`); `load` REFUSES a newer `v` honestly
   (`DocError::Version`) instead of misreading it. Breaking a shape
   bumps `v`; additive fields ride the current `v` with serde defaults.

3. **THE WIRE CONTRACT IS A FIRST-CLASS ARTIFACT.** Each document major
   version has a JSON Schema in `schemas/` —
   `workflow.v1.schema.json`, `storyboard.v1.schema.json`,
   `bible.v1.schema.json` (EX1) — strict
   (`additionalProperties: false`, `v` pinned by `const`), draft
   2020-12. A new major version adds `*.v2.schema.json` BESIDE v1;
   schema files are append-only history, never edited after their
   version ships (fixes to a shipped schema are a new version) — with
   ONE carve-out: a compatible ADDITIVE amendment (a new OPTIONAL
   property that leaves every previously-valid document valid) amends
   the current version's file in the SAME commit as its Rust field,
   mirroring law 2's additive rule. Anything that could invalidate an
   existing document is a new version, no exceptions.
   **PINNED BY TEST**: `crates/workflows` tests serialize the real Rust
   types and validate them against these files — including a NEGATIVE
   case, so the schema is proven load-bearing. Rust and wire cannot
   drift silently. External writers (agents, platform tools) author
   against the schema; the engine remains the runtime gate.

4. **Validate before execute; half-valid never runs.** `validate()` /
   `validate_board()` name every structural fault (`Issue{path,message}`
   — WHERE and WHAT); the executor refuses a document with issues.
   Semantic laws that regex can't carry (stage ordering, the full `..`
   jail on refs) live in the engine validators; the schema carries
   everything it can express. BOTH layers are law — weakening either to
   make a document pass is the bug.

5. **The stage census.** `StageKind` is the ONLY enumeration of stage
   kinds. A new kind lands in ONE commit touching exactly: the enum
   variant + `label()` · its `validate()` arm · its executor arm (with
   an honest `Skipped` where it cannot run) · the schema's `oneOf`
   variant · the pin tests · the plan doc naming it. Nothing else
   branches on stage kinds.

6. **Jobs-domain integration (signal-doctrine law 6/7).** Every stage
   TRANSITION publishes a jobs fact labeled `workflow <id> · <stage>`
   (Queued at start; Done/Failed at finish; a skip is a Done that says
   so). Progress inside a stage is the JobBoard row's latest-wins
   counter — never a broadcast storm. Completion signals ONE
   `InboxChanged` (the batching law). The status bar's `working…` cell,
   the activity feed, `/signals`, and `GET /render` all ride for free —
   a workflow PR adds NO new signal plumbing.

7. **Slots hold REFS, never values.** A `SlotFill` names an artifact
   rel, a slide rel, or a draft id — content lives in the content root
   under the jail law; the board is small, diffable, crash-safe
   (tmp/bak swap writer). Board state questions are DERIVED QUERIES
   (`remaining()`, `filled()`), never prose an agent has to parse.

8. **Surface declaration (two-libraries law).** Every workflow declares
   `Surface::Wiredfront` (raw media) or `Surface::Workbench` (ColdLight
   slides/episodes); hubs mount by it, the ENGINE never branches on it.
   A workflow that needs both sides is two workflows.

9. **Dry-run never spends (Imagine ADR-011).** Generation runs only inside an
   operator-started run; keyless or dry runs produce an honest
   `Skipped{reason}` row — visible in the report, the jobs feed, and
   the panel. A silent skip is a bug.

10. **Runs are no-clobber.** Every run mints a fresh
    `inbox/workflow-<id>[-N]/` (`fresh_dir`); a rerun NEVER writes over
    a prior run; the minted board is saved crash-safe in the run folder
    so panel, chat, coach, and seam read the same truth.

11. **WF2 forward-compat: v1 is the degenerate graph.** Stage `id`s are
    node ids; order-in-vec is the only edge in v1. WF2 adds explicit
    edges and the two-form split (panel layout vs executor plan) as a
    `v` bump with its own schema file, under law 2/3 — v1 documents
    keep loading forever.

## THE WORKFLOW-PR CHECKLIST (the "similar and consistent" gate)

Every PR that adds a workflow, a stage kind, or changes a document shape
copies this into its plan §6 and checks each row with evidence:

- [ ] Registry: the workflow is ONE `workflows()` entry (law 1); no
      parallel enumeration anywhere (grep-proof).
- [ ] Schema: shape change → schema change in the SAME commit; new
      major `v` → new `schemas/*.vN.schema.json` file beside the old.
- [ ] Pin tests green: registered docs validate; the negative case
      still refuses; `cargo test -p dpl-workflows`.
- [ ] Validation: every new field/kind has its `validate()` arm; the
      executor refuses issues (law 4/5).
- [ ] Jobs facts: stage transitions labeled `workflow <id> · <stage>`;
      ONE `InboxChanged`; no per-item signal storms (law 6).
- [ ] Honest skips: every can't-run path is a named `Skipped` (law 9).
- [ ] Surface declared; artifacts land on the declared side (law 8).
- [ ] Real-exe proof: `--run-workflow <id>` verdict line + the run
      folder's `storyboard.json` (law 10), per the repo's Law 2.

## Enforcement

- The pin tests in `crates/workflows` (law 3) run in `cargo green` and
  both CI lanes — schema drift fails the gate.
- `validate()`/`validate_board()` refusal is exercised by unit tests;
  the executor's refusal path is part of the WF1 suite.
- This document is the review standard for workflow PRs; the checklist
  above is copied, not paraphrased.

## Consumers (current map)

- The Workflows tab (composer): registry census → panel rows; run
  report + minted board painted from the run folder.
- `--run-workflow <id>`: the headless drill — verdict line on stdout.
- The agent seam: reads the same run-folder documents; the storyboard's
  `remaining()` is the agent's first question.
- Jobs/status surfaces: per-stage facts on the fabric → activity feed,
  `working…` cell, `/signals` SSE (envelope law).
- Future platform consumers: validate against `schemas/*.schema.json`
  without reading a line of Rust — the same convergence seam the signal
  envelope rides.
