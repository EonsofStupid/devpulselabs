# EX1 — the guided explainer-video workflow (one PR, POAM-tracked)

## 1 · Context

Operator's ask (verbatim, 2026-07-28): "I want to create a explainer
video. That video explainer system will literally allow me to work with
chat that will have a guided system where the first prompt captures the
necessary context that would allow us to then create the bible and
storyboard. it would allow us then to create the storyboards to complete
that vision that would allow us to then produce the full video properly
suppporting short to long format videos even."

This is T3's explainer workflow from the ratified program map
(`wiredfront-workflows-program.md` — D3 documents-not-code-paths, D5
operator-parity, D6 presets-as-ingredients; "bible artifacts become
upstream nodes… enters T3's storyboard workflow as its first upstream
node"), landing per `workflow-doctrine.md` (census law, wire-contract
law, THE WORKFLOW-PR CHECKLIST) on the platform this arc built (JobBoard
`d367390`, pinned schemas `c803e51`).

**Ratified decisions (operator, 2026-07-29):**
- Format axis v1 = duration + beat count, ALL 16:9 1920×1080; vertical
  9:16 is a later one-line preset.
- Delivery = ONE PR on `feat/explainer-guided-workflow`, worked as a
  POAM (§9) with milestones + journal entries — objective vs done vs
  remaining stays legible at every point.
- Slide look v1 = the honest current text-stack document rendering;
  template-true rendering is its own later pass (upgrades every video
  retroactively).
- Bible = PRODUCTION + SCRIPT bible: canon/vision/constraints AND
  per-beat narration authored during intake.

## 2 · Current state (five spot-checked deep-plan sweeps, 2026-07-28, re-verified drift-free at `0a21ad1`)

Full evidence in the session record; the load-bearing facts:

- **Engine**: `execute(doc: &WorkflowDoc, …)` takes any document
  (workflows/src/lib.rs:682) — but `Composer::run_workflow` gates on
  `dpl_workflows::find(id)` (composer:2187), there is NO
  `load_workflow`/`save_workflow`/`WORKFLOW_FILE`, run folders are
  id-derived (composer:2196), `--run-workflow` is id-only.
- **The render leg is dead in practice**: SlideFromBeat mints run-folder
  slides (composer:5415, fill `SlotFill::Slide`), `comp_for_board` lays
  ONLY `Artifact` fills (workflows:252-266), so fresh runs assemble 0
  layers and render never fires (test asserts it, composer:5798).
  GenerateImage NEVER spends (both branches Skipped, composer:5430-5446).
- **No slot-filling gesture exists** (panel tooltip promises it,
  composer:2841; nothing implements it); `load_board` has zero
  production callers — boards are write-only; `wf_run` dies on restart.
- **Slide→pixels**: the ONLY egui rasterizer is the root-viewport
  screenshot (main.rs:1010→1033, real window mandatory); `render_step`
  paints any slide into any Rect (slides/document.rs:191-232, deck
  proves it full-bleed at :224-232); PNG stills are first-class stitch
  inputs with ZERO engine change (IMAGE_EXTS engine-media:1529, still
  legs :331-348, composer auto-detects :1359-1365). `dpl-lottie-render`
  is the sanctioned self-subprocess-rasterizer pattern.
- **Chat**: typed intent chain `ChatWindowIntent → HostIntent → app
  coordinator` (window:747, app/chat.rs:794); the imagine flow is the
  drive precedent (job mint app/chat.rs:1495, ONE composer call
  import_generated :1587); `InteractionContent` is a closed 4-variant
  text-only vocabulary — NO form/choice/answer machinery (interactions
  :58-113); the composer's "⊞ Component" button is the pre-declared
  disabled entry point (window:2100-2106); `GuidedValue<T>` (core:76)
  is the typed field-with-provenance primitive, unused for flows.
- **Registries**: slides registry census 28 (registry.rs:127-133) but
  `Slide.template` is INERT — the document renderer never reads it
  (doc:31, slides/document.rs:233-320); coldlight presets.json +
  build.rs cross-validation is the preset archetype; NO format/aspect/
  duration preset exists anywhere; `Slot.constraints` (workflows:156)
  is the pre-cut seam for format constraints;
  `AssembleComp{fps,w,h}` (workflows:83) is the geometry insertion
  point. Beat lists exist ONLY in the storyboard blueprint (:314-320).
- **Drills**: pre-window blocks exit by verdict (--clone-seed
  main.rs:284, --export-guide :314); boot-time drills do NOT
  (--run-workflow exits 0 via the screenshot frame). Sync render + ffprobe
  proofs ride `Composer::drill_render` (composer:2028-2055).
  `--video-format` is the screen-recording container, NOT a format axis.

## 3 · Target design

**One system, four layers, all doctrine-shaped:**

1. **Documents (dpl-workflows, egui-free).** `BibleDoc v1` — the
   production+script bible: `{ v, id, workflow, subject, audience,
   tone, style_refs[], key_points[], avoid[], format: <profile-id>,
   beats: [BibleBeat { id, role, title, brief, narration }] }` — the
   intake's captured context AND the per-beat script, one document,
   `schemas/bible.v1.schema.json` beside the other two (law 3), pinned
   by test with negative cases. `IntakeSpec` — the declarative guided
   Q&A: ordered `IntakeField { id, ask, kind: Text|Choice{options}|
   FormatPick|BeatsDraft, required }` carried by the workflow blueprint
   (additive optional field, stays v1). `FormatProfile` registry census
   (`format_profiles()`: `short` ~45s/4 beats · `standard` ~120s/7 ·
   `long` ~360s/12 + section beats; each `{ id, label, blurb, fps:30,
   w:1920, h:1080, beat_budget, per_beat_secs }`) — D6 ingredients, the
   coldlight-census shape. `tailor(&BibleDoc) -> WorkflowDoc`: pure,
   deterministic, schema-valid output; beats from the bible (narration
   → slot brief tail), `AssembleComp` geometry + per-beat span from the
   profile; refuses an unsatisfiable bible with named issues.
   `save_workflow/load_workflow` + `WORKFLOW_FILE="workflow.json"` +
   newer-refusal (the missing halves of the document family).
2. **The run becomes real (composer/app).** `run_workflow_doc(&doc,
   dir)` executes a minted instance (run_workflow(id) = find +
   delegate); the run folder carries `bible.json` + `workflow.json` +
   `storyboard.json`. NEW StageKind `RasterizeSlides` (census law: one
   commit touches enum+label+validate+executor+schema+pins): after
   SlideFromBeat, the app renders every run slide to
   `<run>/stills/<slot>.png` at profile geometry by SPAWNING ITSELF
   (the dpl-lottie-render pattern, quiet-spawn law): a new pre-window
   mode `--slides-still <run-rel>` boots, paints each slide full-bleed
   via `render_step` on successive frames, captures via the existing
   screenshot machinery per frame, writes stills, exits by verdict —
   no index writes (sequential-runs law respected: pure files).
   The runner then flips each slot `Slide→Artifact{stills/<slot>.png}`
   … no: fills STAY `Slide` (truth); `comp_for_board` learns to lay a
   Slide fill whose sibling still exists as a still layer with the
   profile's per-beat span (honest note when the still is missing).
   Render goes SYNCHRONOUS for drills (`drill_render` tail) and stays
   jobs-async in-session; ffprobe-checkable `render.mp4` in the run
   folder, duration = profile budget ±0.5s.
3. **The guided intake (chat).** New `InteractionContent::GuidedPrompt
   { intake_id, field, prior: Vec<(field-id, answer)> }` rendered
   INLINE in the transcript (new renderer beside compact_interaction):
   the ask, a typed input (text box or choice chips — the first
   input-carrying interaction, mounted at the ⊞ Component entry
   point), Skip where !required. Answers ride new
   `ChatWindowIntent::SubmitIntakeAnswer → HostIntent` into a
   host-side `IntakeSession` store; provider assist may PRE-FILL a
   `GuidedValue` suggestion (source Ai, operator-editable) but the
   OPERATOR submits (D5 parity: the same spec renders as a plain form
   in the Workflows panel — panel form ships in this pass, minimal).
   Completed intake → mint bible → `tailor` → save docs → run — the
   imagine-precedent drive chain (direct calls + jobs facts;
   the R2c mpsc stays reserved for the external seam).
4. **Registration.** ONE new registry entry: `explainer()` blueprint
   (Surface::Wiredfront — video out; run-folder slides are
   intermediates, not library ingredients) with intake spec + stage
   list `outline(placeholder beats) · slide-from-beat(title) ·
   rasterize-slides · generate-image · assemble-comp · render`.
   Panel: the Workflows tab lists it; its Run without a bible opens
   the intake form (parity path).

Scale story: workflow #3 = one blueprint + intake spec; format #4 =
one census line; the bible/intake/tailor machinery is paid for once.

## 4 · Changes, in execution order — see §9 POAM

The POAM is the execution order; each milestone names its exact files.

## 5 · Boundaries

- 🚫 `/content`; the other lanes' branches; `JobEvent` wire shape;
  template-true slide rendering (named later pass); vertical 9:16
  (later preset); the external-seam R2c mpsc (reserved); Slide.template
  resolution (inert today, stays inert this pass); GenerateImage
  spending (stays honest-skip; Imagine art arrives via chat drafts +
  slot-fill).
- ⚠️ Ask first: any new crate; any new non-dev dependency; touching
  `dpl_comp::Source` variants (the Slide-still leg deliberately avoids
  a new Source variant in v1).
- One-giant-PR risk management: rebase onto origin/main at every
  milestone boundary (active parallel lanes), journal entry per
  milestone, double-green before the final merge (the 07-29 law).

## 6 · Verification

The ratified draft from the verification sweep, condensed; each
milestone's exit criteria in §9 cite these:
V1 schema pins (`bible.v1` + tailored-doc revalidates against
workflow.v1; negative drift cases). V2 `cargo test -p dpl-workflows`
(tailor determinism, short/long differ only on the declared axis,
census gates incl. explainer validates clean). V3 `--slides-still`
pre-window drill: verdict line + stills exist at 1920×1080. V4
`--intake explainer --intake-answers <fixture>` pre-window drill:
verdict + bible.json validates + no-clobber on rerun + honest-absence
for unknown ids. V5 `--run-workflow-doc <rel> --render-sync`: verdict
+ storyboard all slots filled + render.mp4 ffprobe h264 1920×1080,
duration within profile budget ±0.5s, short AND long fixtures. V6 chat
proofs: `--chat-proof intake` (devtools) screenshots at 1280x800 AND
1600x1000; Workflows-panel intake form + run verdicts same sizes; PNGs
READ and baseline-compared per true-verify. V7 `cargo green` + both CI
lanes DOUBLE-GREEN before merge. V8 EguiPilotAI scenario
`wiredfront-intake.toml` (semantic intake nodes). V9 release exe
rebuilt in THE checkout, mtime moved; jobs facts observed on the
SIGNALS feed during a run.

## 7 · Commit plan

ONE branch `feat/explainer-guided-workflow` (this lane), commits per
milestone slice (each gate-clean: fmt+clippy+tests in-lane before
push — the server1 gate-hole law), pushed via server1, PR opened as
DRAFT at M1-end and kept current; squash-merge ONLY on double-green
after M4. Journal entries `docs/journal/explainer-m<N>.md` per
milestone; STATE.md close-out rides the final merge.

## 8 · Evidence & provenance

Deep-plan fan-out 2026-07-28 (five Opus sweeps: chat estate, slide
raster, executor surface, registries, verification draft), EVERY sweep
spot-checked by Fable against the real files before trust (composer
2186/5415-5423, window 2100-2106 + interactions 58-66, main.rs
284-290/1030-1036 + chat.rs 48-56, engine-media 1529 + comp Source,
registry.rs 127-133 + workflows 324-328 + document.rs template greps —
all confirmed verbatim). Drift-checked 2026-07-29 at `0a21ad1`:
workflows/interactions/composer byte-identical since the sweeps.
Operator ratifications recorded in §1.

## 9 · POAM (the living tracker — updated every work session)

**Objective:** operator opens chat, is guided through the explainer
intake, a production+script bible is minted, a tailored storyboard is
completed against it, and a real 16:9 video renders — short or long —
with every step visible on the JobBoard and in the Workflows panel.

| M | Scope (files) | Exit criteria | Status |
|---|---|---|---|
| M1 · Documents | dpl-workflows: BibleDoc+IntakeSpec+FormatProfile census+tailor+save/load_workflow; schemas/bible.v1.schema.json; pin tests | V1+V2 green in-lane | **DONE 2026-07-29** — 24/24 tests, fmt+clippy clean (journal: explainer-m1) |
| M2 · Run real | composer run_workflow_doc; StageKind::RasterizeSlides; --slides-still pre-window mode; comp_for_board slide-still legs; --run-workflow-doc + --render-sync; schema oneOf + pins | V3+V5 green; ffprobed short+long fixtures | not started |
| M3 · Guided intake | chat: InteractionContent::GuidedPrompt + inline renderer + intents + IntakeSession store; panel intake form (parity); mint→tailor→run chain; --intake/--intake-answers; --chat-proof intake | V4+V6 green | not started |
| M4 · Close-out | rebase, full V7-V9, baselines, STATE+journal, double-green merge | PR merged, exe rebuilt HERE | not started |

### §9.1 Deviations log
(empty — every deviation from the plan gets a dated line here)
