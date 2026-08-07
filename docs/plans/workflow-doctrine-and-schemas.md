# Workflow doctrine + versioned wire schemas — every workflow PR lands the same shape

## 1 · Context

Operator directive (2026-07-28): workflows are this arc's platform spine —
"get platform, industry bleeding-edge schemas built out for our workflows
to be consistent patterns and properly set us up for scale. All of these
workflow PRs are going to be similar and consistent."

WF1 (`crates/workflows`) shipped a real typed engine: `WorkflowDoc` v1
(id/label/blurb/surface + a `StageKind` census), the slotted
`StoryboardDoc`, validation-before-execution, a registry census, the honest
dry-run, and per-stage jobs facts. What does NOT exist yet is the LAW that
keeps the Nth workflow PR shaped like the 1st — the way
`signal-doctrine.md` did it for signals — and a machine-checkable WIRE
contract: the documents are serde-shaped only, so the agent seam, external
tools, and future platform consumers have nothing to validate against
except reading Rust.

## 2 · Current state (read in-session at `7b1deba`)

- `crates/workflows/src/lib.rs` (975 lines): `SCHEMA_V = 1`;
  `WorkflowDoc { v, id, label, blurb, surface, stages }`;
  `Surface { Wiredfront | Workbench }` (kebab serde);
  `Stage { id, #[serde(flatten)] kind }`;
  `StageKind` internally tagged `kind`, kebab: `outline{beats}` ·
  `slide-from-beat{template}` · `generate-image` · `assemble-comp{fps,w,h}`
  · `render`; `Beat { id, role, title, brief }`;
  `StoryboardDoc { v, workflow, show?, episode?, slots }`;
  `Slot { id, role, brief, constraints (skip-if-empty), fill }`;
  `SlotFill` tagged `kind`: `empty` · `artifact{rel}` · `slide{rel}` ·
  `draft{id}`; `mint_storyboard`, `slide_for_slot`, `comp_for_board`
  (skips named honestly), registry `workflows()`/`find()` (one entry:
  `storyboard`), `validate()` (kebab ids, jail law on refs, per-kind
  checks), executor + dry-run further down, doc header names WF2 = the
  graph, v1 = linear degenerate case, newer-schema-refusal law.
- `crates/composer/src/lib.rs` `run_workflow` (~2040–2125): per-stage jobs
  facts labeled `workflow <id> · <stage>`, ONE `InboxChanged` at
  completion, board saved crash-safe in the run folder, assembled comps
  ride the ONE stitch path (`start_render`).
- Structure audit (`crates/xtask/src/main.rs` ~628): dpl-workflows NORMAL
  deps EXACTLY `[dpl-comp, dpl-doc, serde, serde_json]`; dev-deps are not
  ceiling-counted ("NORMAL direct deps"). egui-free census includes
  dpl-workflows.
- No `schemas/` directory exists; no JSON Schema anywhere in the repo.
- Doctrine precedent: `docs/design/signal-doctrine.md` (LOCKED, numbered
  laws + enforcement + consumers map) is the house doctrine shape.

## 3 · Target design

**The wire contract becomes a first-class, versioned artifact** and the
doctrine makes every future workflow PR mechanical:

- `schemas/workflow.v1.schema.json` + `schemas/storyboard.v1.schema.json`
  — JSON Schema draft 2020-12, hand-authored to the EXACT serde shapes,
  strict (`additionalProperties: false`, `v` is `const 1`, kebab `pattern`
  on every id, jail-safe `rel` pattern). The schema IS the authoring
  contract for agents/external writers; the engine remains the runtime
  gate (`validate()` + newer-refusal). One file per major version —
  `workflow.v2.schema.json` will sit BESIDE v1, never replace it.
- **PINNED BY TEST** (the house law): dpl-workflows gains dev-dep
  `jsonschema`; tests assert (a) every REGISTERED workflow serializes to
  a document the schema accepts, (b) a minted + filled storyboard
  validates, (c) a deliberately wrong document (unknown stage kind, bad
  id, extra field) is REJECTED — the schema is load-bearing, not
  decorative. Rust shape and JSON contract can never drift silently.
- `docs/design/workflow-doctrine.md` — the numbered laws (registry census;
  schema-versioning + newer-refusal + schema-file-per-version;
  validate-before-execute, half-valid never runs; stage census — what a
  new StageKind must touch, in ONE commit; jobs-domain integration —
  per-stage transitions, `workflow <id> · <stage>` labels, batching law,
  the JobBoard rides free; storyboard slots — refs never values, derived
  queries never prose; surface declaration — two-libraries law; dry-run
  never spends, Imagine ADR-011; WF2 graph forward-compat — stage ids are node
  ids) + **THE WORKFLOW-PR CHECKLIST** — the concrete "similar and
  consistent" gate every workflow PR copies into its plan §6.
- `architecture.md` document-crates row for dpl-workflows gains the
  schema pointer (one line), same-commit with the audit table — no audit
  change needed (dev-dep only).

Scale story: the Nth workflow = a `WorkflowDoc` literal in the registry
(census law) — schema + validate + jobs + board + panel all apply with
zero new plumbing. The Nth stage KIND = one enum variant + its checklist
row. The platform convergence rides the same seam signals do: versioned,
validatable wire documents.

## 4 · Changes, in execution order

1. CREATE `schemas/workflow.v1.schema.json` — `$defs`: `kebab` (pattern
   `^[a-z0-9_-]+$`), `beat`, per-kind stage variants (closed objects,
   shared required `id` + `kind` const), `oneOf` stages; root requires
   `v(const 1)/id/label/blurb/surface/stages(minItems 1)`.
2. CREATE `schemas/storyboard.v1.schema.json` — root
   `v(const 1)/workflow/slots` + optional `show`/`episode`; `slot`
   requires `id/role/brief/fill`, optional `constraints`; `fill` `oneOf`
   the four kinds; `rel` pattern refuses `..`, leading `/`, `:`,
   backslashes (the jail law, expressed in schema).
3. EDIT `crates/workflows/Cargo.toml` — `[dev-dependencies] jsonschema`
   (workspace dep added in root `Cargo.toml` `[workspace.dependencies]`).
4. EDIT `crates/workflows/src/lib.rs` tests — new `schema_pin` tests per
   §3 (load schema via `env!("CARGO_MANIFEST_DIR")/../../schemas/…`).
5. CREATE `docs/design/workflow-doctrine.md` per §3.
6. EDIT `docs/design/architecture.md` — the dpl-workflows row's "why"
   gains "wire contract: `schemas/workflow.v1.schema.json` (pinned by
   test)". No xtask change (dev-deps uncounted — verified against the
   audit's NORMAL-deps rule).
7. Gate + PR from the lane via server1 (code PR → both CI lanes run).

## 5 · Boundaries

- 🚫 No engine behavior change — this pass adds contract + law, zero
  runtime semantics. 🚫 No `JobEvent` change. 🚫 The other lanes'
  in-flight PRs (their consistency is enforced FORWARD by the doctrine
  gate, not by me rewriting their branches). 🚫 `/content`.
- ⚠️ Ask first: WF2 graph work (the doctrine documents the contract;
  building it is its own pass); schema for `dpl_doc`/`dpl_comp`
  documents (same pattern, separate pass); a `$id` URL scheme (needs a
  decision on a canonical host).
- Out of scope: CI schema-linting of authored JSON (no authored workflow
  JSON exists yet — documents are minted from the registry).

## 6 · Verification

1. `cargo test -p dpl-workflows` — schema-pin tests pass, including the
   NEGATIVE case.
2. `cargo green` on server1 — fmt/clippy/tests/release/structure all
   green (structure proves the dev-dep stays outside the ceiling).
3. Both CI lanes green on the PR (code change → workflows trigger).
4. Grep-proof: `schemas/*.schema.json` referenced from doctrine +
   architecture; doctrine checklist names every file a workflow PR
   touches.

## 7 · Commit plan

Branch `feat/workflow-doctrine` (worktree lane off `7b1deba`). ONE
commit: "feat(workflows): the wire contract — v1 JSON Schemas pinned by
test + the workflow doctrine". Push via server1 (hook gate), PR,
squash-merge on green CI.

## 8 · Evidence

`crates/workflows/src/lib.rs` read 1–410 in-session at `7b1deba` (shapes
quoted in §2 from that read); composer `run_workflow` jobs integration
read earlier this session (jobs-board pass); xtask structure audit
628–660 read in-session (ceiling = NORMAL deps only); signal-doctrine.md
read in full earlier this session (doctrine shape precedent).
