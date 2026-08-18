# BS1 — the brainstorm surface (one PR, POAM-tracked)

## 1 · Context

Operator's ask (verbatim, 2026-08-06): "when i type in chat brainstorming
it literally will trigger a2ui addition components that will populate
with list that I can edit they will be visualized. So one will be a
storyboard yes but it might have a template to make it brainstorming so
its taking the massive idea and hten breaking down."

Vision + researched patterns: `brainstorm-and-clyffy-drive-capture.md`
(A2UI = UI-as-data against a trusted catalog; template-first
diverge→converge; structure-as-context breakdown; editable peer items).
Brainstorming sits UPSTREAM of EX1 (intake → bible → storyboard →
production): the stage where the massive idea breaks down before intake
captures it. Sessions/counterpoints and Clyffy drive are LATER arcs —
BS1 is documents + chat surface + verbs only.

**Decisions (stated inline per the 2026-07-29 no-dialogs ruling; each
marked RECOMMENDED was mine to make from evidence):**
- D1 Trigger = whole-buffer recognition at Ai-composer submit:
  `brainstorming` (case-insensitive) or `/brainstorm`. The
  `parse_operator_slash` provenance law verbatim (window:409-415):
  pattern-in-string never escalates. RECOMMENDED.
- D2 The document IS the data model: `BrainstormDoc v1` in
  `dpl-workflows` (serde-only — NO ceiling amendment), mutated ONLY
  through a typed `BrainstormOp` census via one pure
  `apply(doc, op) -> Result<(), Vec<Issue>>`. Operator taps and assist
  proposals emit the SAME ops (D5 parity by construction). RECOMMENDED.
- D3 Undo = host-side snapshot stack (composer `checkpoint` pattern:
  dedupe, cap 100, redo cleared on new edit). NO document-layer
  op-journal — none exists anywhere in the estate; snapshots are the
  house shape (editor:232, composer:1304). RECOMMENDED.
- D4 Promote = `distill(&BrainstormDoc) -> Result<IntakePrefill,
  Vec<Issue>>` — pre-fills the EX1 explainer intake (subject from seed,
  key_points from converged items, beats-draft from breakdown). The
  bible mint stays single-sited at intake; resolves capture-doc §4 Q3.
  RECOMMENDED.
- D5 v1 composites (editable card, lane, phase header) are
  WINDOW-LOCAL painters composed from the `cl::` vocabulary + the
  quote_tab editable pattern (author.rs:453-476). The slides block
  tier stays empty: "Promotion, not authorship, fills this — recurs in
  ≥2 templates" (blocks/census.rs:59,77). Block promotion is a named
  later pass. RECOMMENDED.
- D6 Chat-inline first (capture-doc §4 Q1): the board renders in the
  Ai transcript (the operator's words: "when i type in chat"); a
  peer-window board is a later arc riding the same document.
  RECOMMENDED.

## 2 · Current state (four Opus sweeps 2026-08-07, EVERY sweep
spot-checked by Fable against the real files — §8)

- **Interactions** (chat/interactions/src/lib.rs): closed 4-variant
  declarative vocabulary ("Providers cannot supply egui callbacks…"),
  8KB text cap, lifecycle registry, Approval-is-overlay law. Notice is
  never minted in production; RouteResult/ArtifactPreview mint only
  from the authored testkit fixture.
- **Render sites**: `inline_interactions` (window:2600, called ONCE
  from `ai_conversation` :2482 — Ai domain only); approval modal
  (:3981); utility viewport (app/chat.rs:4195). The ⊞ Component
  button EXISTS DISABLED at window:2756 — "The governed Interaction
  Library picker is not connected in this slice" — the pre-declared
  mount BS1 connects.
- **Intent chain**: `ChatWindowIntent` (37 variants, window:282) →
  pure `apply()` (:975, exhaustive) → `Option<HostIntent>` (30
  variants, :418) → `apply_host_intent` (app/chat.rs:1305). Slash law
  at window:409: exact whole-buffer at submit.
- **Read-model precedent (H1)**: `ContentView` — window holds the
  view (observe_content :864, focus-preserving), host owns IO
  (read_content_view app/chat.rs:1242, LoadContentSpine :1526). "A
  surface does not do IO."
- **Assist is a boolean with NO behavior**: `assist_enabled` read at
  exactly 2 sites (chip label + pilot metadata). The tooltip promise
  ("may offer pop-up suggestions you approve or dismiss") has zero
  implementation. BS1's assist populate is assist's FIRST teeth.
- **Jobs**: imagine precedent — mint BEFORE worker (app/chat.rs:1994),
  Queued label, one transition per terminal path, honest no-key error
  without a job (:1988). Workflow-stage labels `workflow <id> ·
  <stage>` (composer:2461).
- **Documents** (workflows/src/lib.rs, 1818 lines): family =
  WorkflowDoc/StoryboardDoc/BibleDoc + `save_json` crash-safe writer
  (:1054, tmp→bak→rename) + newer-refusal + `fresh_dir` no-clobber
  (:1073). A new doc type = FILE const + load/save pair, 3 lines each.
  `tailor` (:507) is the distill archetype: validate-first →
  `Vec<Issue>`, `expect("validated")` after the gate, pure +
  deterministic, pinned. Ceiling EXACT-SET `["dpl-comp","dpl-doc",
  "serde","serde_json"]` (xtask:664); `dpl-workflows` is EGUI_FREE
  (xtask:682) — the surface CANNOT live there.
- **Undo**: grep-proof NONE in chat estate or any document crate.
  Editor = snapshot per mutate, cap 50, no redo (editor:232-250);
  composer = checkpoint/undo/redo, cap 100, dedupe (:1304-1345).
- **ColdLight/slides split**: coldlight = tokens + paint primitives
  (12 generated files, `cl::tip` law :139); slides = the ingredient
  ladder — elements 6 / **blocks 0 (empty by design)** / templates 28.
  `badge` (chips) and `eyebrow` (micro-label) exist; editable cards
  exist ONLY as hub-local TextEdit-in-styled-panel (author.rs:453).
  chat/window already deps dpl-coldlight, NOT dpl-slides/dpl-doc.
- **Proof harness**: `--chat-proof` is a devtools boot PRESET; the
  driver is `chat_native_proof.rs` (out-of-process, Windows-only,
  paints-check + retry, exit by verdict; duplicated mode list to keep
  in sync). Pre-window drills (--clone-seed :415, --export-guide :444)
  sit above eframe boot and exit by verdict. House proof sizes:
  1280x800 universal + 1100x700 short-fit (1600x1000 is prose-only).
  Baseline compare = the true-verify READ protocol, not a harness.
  EguiPilotAI lane is workflow_dispatch only — NOT in double-green.

## 3 · Target design

**One document, one op vocabulary, one surface — doctrine-shaped:**

1. **Documents (dpl-workflows, egui-free).** `BrainstormDoc v1`:
   `{ v, id, title, seed, template: <scaffold-id>, phase:
   Diverge|Cluster|Converge, items: [BrainstormItem { id, text,
   parent: Option<id>, cluster: Option<id>, origin: Operator|Assist }],
   clusters: [BrainstormCluster { id, label }] }` —
   `schemas/brainstorm.v1.schema.json` beside the other three (law 3),
   pinned with negative cases. `BrainstormOp` census (enum + apply arm
   + schema + pins): `Seed · AddItem · EditItem · RemoveItem ·
   BreakDown{parent} · AssignCluster · RenameCluster · AdvancePhase`.
   `apply` is pure, deterministic, all-or-nothing (refuses with named
   Issues: cluster before diverge has items, advance past Converge,
   unknown ids). `BrainstormScaffold` registry census
   (`brainstorm_scaffolds()`: v1 = `breakdown` — seed → explode →
   cluster → order; the operator's "storyboard with a brainstorming
   template"), each scaffold mints a valid doc.
   `distill(&BrainstormDoc) -> Result<IntakePrefill, Vec<Issue>>`
   where `IntakePrefill = Vec<(field-id, suggested answer)>` keyed to
   the explainer intake's 8 field ids; refuses an unconverged board.
   `BRAINSTORM_FILE="brainstorm.json"` + save/load via `save_json` +
   newer-refusal.
2. **The chat surface (dpl-chat-window).** NEW dep window→dpl-workflows
   (surface→document, the editor→dpl-doc direction; no ceiling row
   exists for window — architecture.md §1 gains the sentence, audit
   unchanged). Trigger: `parse_brainstorm_trigger` beside
   parse_operator_slash — whole-buffer `brainstorming`/`/brainstorm`
   at Ai submit → `ChatWindowIntent::StartBrainstorm`. The board
   renders INLINE in `ai_conversation` from a window-held
   `BrainstormView` (the ContentView projection shape:
   `observe_brainstorm(doc)`), painted with cl:: vocabulary:
   phase header (eyebrow formula + count + rule), item cards
   (quote_tab editable pattern — TextEdit in a cl-styled plate, tip'd,
   widget_info'd, pilot-tracked), cluster lanes (DragMatch column
   formula), origin distinction (Assist items get the badge live-tone
   until operator-touched), breakdown affordance per item, promote bar
   visible only at Converge. Every edit gesture emits
   `ChatWindowIntent::BrainstormOp(op)` → `HostIntent::BrainstormOp`.
   The ⊞ Component button ENABLES for exactly this surface (its
   tooltip finally honest).
3. **The host (app/chat.rs).** `Shared.brainstorm: Option<
   BrainstormSession { dir, doc, undo: Vec<BrainstormDoc>, redo }>`
   (checkpoint pattern, cap 100). StartBrainstorm → `fresh_dir(inbox,
   "brainstorm")` mint from scaffold → save → project view.
   BrainstormOp → checkpoint → `dpl_workflows::apply` → save →
   re-project (refused ops surface their Issues as an honest status
   line, never a panic). Undo/redo intents ride the same seam.
   PromoteBrainstorm → `distill` → (M3 wiring) the explainer intake
   opens pre-filled; v1 exit = the IntakePrefill lands beside the doc
   as `intake-prefill.json` + an ArtifactPreview interaction announces
   it (the honest slice until EX1-M3 builds the intake surface).
   Assist populate (assist ON only): a job (imagine shape — mint
   before worker, label `brainstorm · populate`, one terminal
   transition) asks the active provider for candidate items against
   the seed + existing structure; results arrive as PROPOSED ops
   applied through the same `apply` with origin Assist — visible,
   editable, deletable peer items. No key/provider → honest skip.
4. **Registration.** `--brainstorm-drill` pre-window flag (clone-seed
   shape, exit by verdict); `ChatProofMode::Brainstorm` threaded
   through enum + parse + arrangement + chat_native_proof's list;
   harness-flag census in xtask updated same commit.

Scale story: scaffold #2 = one census line; the doc/ops/apply/distill
machinery is paid for once; the counterpoint arc later rides the same
ops; EX1-M3's intake reuses the trigger + inline-input patterns.

## 4 · Changes, in execution order — see §9 POAM

Each milestone names its exact files in §9.

## 5 · Boundaries

- 🚫 `/content`; multi-session spawning + provider counterpoints (the
  capture doc's LATER arcs); Clyffy drive; block-tier promotion
  (named later pass); peer-window board (later, same document);
  EX1 M2 (render leg) untouched; `JobEvent` wire shape; the R2c mpsc.
- ⚠️ Ask first: any new crate; any new non-dev dependency anywhere
  except the declared window→dpl-workflows edge; touching
  `IntakeSpec`/bible shapes beyond reading field ids.
- Rebase onto origin/main at every milestone boundary; journal per
  milestone; double-green before merge.

## 6 · Verification

V1 schema pins positive (`every_scaffolded_brainstorm_matches…`, every
op + phase in a validated fixture) · V2 negative drift (unknown op tag,
non-kebab id, unknown root field, v=2, jailbreak refs — all refused) ·
V3 doc hygiene (round-trip, pretty-LF, tmp/bak absent, newer-refusal) ·
V4 op census + determinism + refusal (`apply` twice = identical bytes;
refused op mutates nothing, names Issues) · V5 distill (pure/
deterministic; refuses unconverged; field ids resolve against the
explainer intake) · V6 [NEW HARNESS] `--brainstorm-drill <scaffold>
--content-root target\bs1-scratch`: verdict line, doc schema-valid on
reload, rerun mints `-2` (no-clobber), `/content` untouched · V7 [NEW]
`ChatProofMode::Brainstorm` + chat_native_proof at 1280x800 AND
1100x700 — PNGs READ per true-verify (phases visible, assist item
distinct from operator item, promote bar at Converge, no clipping at
the short size), committed as baselines · V8 `cargo xtask structure`
(workflows ceiling EXACT-SET unchanged, EGUI_FREE holds) · V9 `cargo
green` all phases · V10 both CI lanes DOUBLE-GREEN (`gh pr checks
--watch`; a running check is a red check) · V11 [NEW] EguiPilotAI
`chat-brainstorm.toml` (semantic phase/item nodes, no sleeps; manual
lane — result pasted into evidence) · V12 operator exe: plain release
rebuild HERE, mtime moved, STATE updated.

**THE WORKFLOW-PR CHECKLIST** (doctrine, copied per law):
- [ ] Registry: scaffolds are ONE `brainstorm_scaffolds()` census; no
      parallel enumeration (grep-proof).
- [ ] Schema: shape change → schema change SAME commit; new doc = new
      `schemas/brainstorm.v1.schema.json` beside the three.
- [ ] Pin tests green incl. negatives; `cargo test -p dpl-workflows`.
- [ ] Validation: every op/field has its arm; `apply` refuses issues.
- [ ] Jobs facts: assist populate labeled `brainstorm · populate`;
      ONE transition per terminal; no per-item storms.
- [ ] Honest skips: no provider/key/assist-off = named skip.
- [ ] Surface declared (board doc lives run-side, Workbench-adjacent;
      artifacts stay put).
- [ ] Real-exe proof: `--brainstorm-drill` verdict + run folder's
      `brainstorm.json`.

## 7 · Commit plan

Same lane branch `feat/explainer-guided-workflow` (BS1 extends the
EX1 program's one-PR POAM — brainstorm is the pipeline's upstream
stage; the operator may split it at review). Commits per milestone
slice, each gate-clean in-lane; PR opened DRAFT at BS-M1 end; journal
`docs/journal/brainstorm-m<N>.md` per milestone; squash-merge on
double-green after BS-M3.

## 8 · Evidence & provenance

Deep-plan fan-out 2026-08-07, four Opus sweeps (chat estate; ColdLight/
slides catalog; documents/undo/architecture; verification draft), run
against HEAD after rebasing the lane onto origin/main (97 commits —
the EX1-era evidence was stale and was NOT trusted). Fable spot-checks
before trust, all confirmed verbatim: editor snapshot-mutate
(editor:232-239), xtask CEILINGS (xtask:661-674), ⊞ Component disabled
mount (window:2756-2762), sweep SIZES incl. no-1600x1000
(xtask:1020-1028), empty blocks census + promotion rule
(blocks/census.rs:59-80). Sweep contradictions adopted into §2:
assist-no-behavior, proof-size prose drift, no automated baseline
diff, chat_native_proof duplicated mode list (BS-M2 must update both).
Operator ratifications: vision verbatim §1 (2026-08-06); "go"
(2026-08-07); decisions D1-D6 are RECOMMENDED-inline per the
2026-07-29 no-dialogs ruling and stand unless overruled.

## 9 · POAM (the living tracker)

**Objective:** the operator types "brainstorming" in chat; a phased
board appears inline and populates; every item is editable/movable/
deletable with undo; assist (when on) proposes visible peer items;
Converge offers Promote, which distills the board into an explainer
intake pre-fill — proven headless, on-screen at both house sizes, and
double-green.

| M | Scope (files) | Exit criteria | Status |
|---|---|---|---|
| BS-M1 · Documents | workflows lib.rs: BrainstormDoc+ops+apply+scaffold census+distill+save/load; schemas/brainstorm.v1.schema.json; pins | V1-V5 green in-lane | **DONE 2026-08-07** — 34/34 tests, fmt+clippy clean (journal: brainstorm-m1) |
| BS-M2 · Surface | window: trigger+view+inline board painters+intents; app/chat.rs: session store+undo+host arm; --brainstorm-drill; ChatProofMode::Brainstorm (+native-proof list); xtask flag census | V6-V9 green; both PNGs read | **DONE 2026-08-07** — 44+34+37 tests, drill verdicts live, baseline PNG read (journal: brainstorm-m2) |
| BS-M3 · Assist + promote + close-out | assist populate job; promote/distill wiring + ArtifactPreview; pilot scenario; rebase, V10-V12, baselines, STATE+journal, double-green | PR merged, exe rebuilt HERE | **DONE 2026-08-07** — SuggestClient + populate job + announce + pilot 6/6 (journal: brainstorm-m3); merge rides double-green |

### §9.1 Deviations log
- 2026-08-07 (M1): V2's "unknown op tag" negative case moved from the
  wire schema to the `apply` unit tests — ops are not persisted in v1,
  so `brainstorm.v1.schema.json` covers the DOC only; the op census is
  pinned Rust-side (refusal ladder + determinism tests).
- 2026-08-07 (M2): V7's "both house sizes" replaced by ONE capture at
  the chat peer window's own geometry (1300x800 via chat-native-proof)
  — the two-size law governs ROUTE screenshots; the peer window sizes
  itself, and compact is a separate proof state. `IntakePrefillDoc`
  added to dpl-workflows (the distill's persistence half M1 left
  implicit).
