# WiredFRONT Workflows — the program map

**Status:** ARCHITECTED — deep-plan complete 2026-07-27; WF1 plan authored
(`wf1-workflow-engine.md`); awaiting operator ratification of the asks in §Ratification
**Date:** 2026-07-27
**Operator's ask (verbatim, 2026-07-27):** "I need this editor to be built
up so it has all the capabilities of After Effects, but then can be AI
driven. Because as long as I have all the capabilities as an operator, it
means that we can then include a really smart system that will integrate
with our fabric, have clear signals, and then allow us to even really have
an integration and an A2UI experience."
**Extends:** `wiredfront-experience.md` (the friction ladder — this program
adds tiers, it does not replace the ladder),
`chat-creation-and-workflows-vision.md` §3, `operator-coach-a2ui.md` (C3).

## Decisions (Fable-owned; evidence may refine, not reverse)

**D1 — Tier order: parity and drive interleave; the engine comes first.**
The operator's own logic — operator capability first, AI-drive second — is
honored *per capability*, not as a monolith: each capability family lands
operator-usable (experience-gated), and only then gains its drive verbs.
But the FIRST pass is the **workflow engine foundation**, because W11
(media→slide), the #97 catalog, the import seam, and the comp/render stack
already support a walking-skeleton storyboard pipeline; the engine makes
every later workflow additive instead of bespoke.

**D2 — "AE-class" is defined by capability families, not by cloning AE.**
In scope: timeline/sequencing · layer transforms with keyframes ·
transitions/effects · editor text/titles · masks/mattes · audio ·
nesting/precomps · captions (CapCut lesson) · render/export. Delegated to
subprocesses per ADR-009: encode/stitch (ffmpeg), PDF (pdftoppm), and the
operator's real `.aep` assets via **aerender.exe as a renderer** — we
render AE projects, we do not reimplement AE's expression language.
Never-goals: AE plugin ecosystem, 3D, expressions. The gap inventory
(evidence pending) turns these families into the ladder's new rows.

**D3 — A workflow is a typed, declarative document, executed by the app.**
New pure crate `dpl-workflows`: a `Workflow` = named, versioned sequence of
typed stages (Generate · Import · MakeSlide · CompMutate · Render ·
Export…), each stage publishing `jobs.*` fabric facts, persisted like any
document. Workflows are ingredients — they live in the one generated
library, never one-off code paths (house law). Rejected: per-workflow
hardcoded functions (unscalable, unteachable to the agent) and an external
orchestrator (breaks pure-Rust and the fabric's signal doctrine).

**D4 — First pass = WF1: engine + storyboarding walking skeleton.** One
workflow proves the engine: outline → slide drafts (catalog templates +
Imagine images per ADR-011) → comp assembly → preview. Experience-gated:
the Workflows panel in WiredFRONT is the deliverable the operator touches.

**D5 — A2UI drives the SAME typed stages.** No second vocabulary: the
agent seam and coach signals drive `dpl-workflows` stages and the existing
comp endpoints. C3's catalog renders workflow state; new agent-only verbs
are forbidden until the operator-parity version of that verb exists
(operator's own sequencing logic, made law).

**D6 — Presets are first-class ingredients (Higgsfield lesson).** Camera
moves, looks, caption styles, and workflow templates are registered,
generated vocabulary objects feeding the one ingredient library — the
access-layer pattern: the operator (and the agent) compose from named
presets, never from loose parameters scattered in code.

## Risk register

- **aerender licensing/availability**: machine-local Adobe install is an
  operator asset, not a shipped dependency; the lane must fail honestly
  when absent (evidence pending on install state).
- **Engine over-design**: WF1 ships ONE workflow end-to-end before any
  second stage type is added speculatively.
- **Experience drift**: every tier passes `wiredfront-experience.md` §5;
  plumbing never ships ahead of the panel the operator touches.
- **Imagine spend in workflows**: generation stages inherit ADR-011 —
  explicit operator start, per-stage cost visibility, never auto-retry.

## The evidence (deep-plan fan-out, 2026-07-27, main @ 09f01fc)

Five Opus sweeps, each spot-checked by Fable before trust (provenance per
the deep-plan law). Verified headlines:

- **Comp document truth** (spot-check: `Layer` struct, comp lib.rs:44-53):
  `Layer {id, source, start, in_pt, out_pt}` — NO transforms, NO keyframes,
  NO opacity, NO audio (every pipe passes `-an`), no editor text, no masks,
  no precomps; preview plays the topmost layer only; render is bare concat.
  `Source` = Artifact | Element, and Element is inert in both preview
  (composer :627) and stitch (:942). The editor's own placeholder blurbs
  already name each missing pass (effects-registry, multi-comp, preset-text,
  3D-later).
- **Gate table re-grounded** (spot-check: element dead legs): the "LAND
  R1x" rows shipped as W1/W1.2/W2 on 07-24 — today 13 DONE, 2 PARTIAL
  (row 6 elements render leg, row 9 chat-edits-comp), 3 NOT STARTED (row 7
  effects — the deepest gap, rows 11/12). Row 11's HTTP blocker is gone
  (`ureq` landed #98). `crates/engine-ae` (aerender orchestrator ph.1, #40)
  and the L2 Lottie branch (first `Source` variant that would actually
  render) are landed-but-unwired assets.
- **Drive surface** (spot-check: silent comp save, seam lib.rs:676-682):
  an agent can ALREADY drive state→comps→doctor-gated POST /comp→render→
  poll over 127.0.0.1:7341 end-to-end. Four breaks, in cost order: the comp
  write publishes no fact and the editor never reloads; the seam vocabulary
  stops at comps/navigate/render (deck absent from SSE); coach (stdio,
  devtools-only) and seam (HTTP, every build) share no identity
  (`Tree::Agents` exists and is empty); C3 UI composition has zero code.
- **Tool lane** (spot-check: paths): aerender PRESENT (`AE 2026`,
  `26.3x87`, full flag matrix recorded in the sweep; matches engine-ae's
  grounding). pdftoppm ABSENT on this box (STATE's drill claim holds only
  on server1) — poppler must be staged into the depot. ffmpeg 8.1.2 full
  build staged — notably with **libass and `--enable-whisper`**: burn-in
  subtitles AND auto-transcription are available as pure ADR-009
  subprocess capability. Depot fragility: only chromium+grok have depot
  copies; codex has no provenance record. The `devpulse labs` reference
  repo is build-output only (no source, no .git) — the port-capture
  premise is dead; salvage = old exes, `artifacts.db`, `content/shows`.
- **Verification toolbox**: fixture+scenario recipe, the slides gallery
  gate (`every_demo_renders_at_start_and_at_rest`) as the census-with-teeth
  precedent, the W11 scratch-`--content-root` headless drill pattern, and
  the `--make-slide` verdict-line precedent for a new `--run-workflow`
  flag. Caught live: composer already has `fn workflow_tabs` (:1898) for
  the Editor/Capture/Annotate/Narrate/Guide tabs — the Workflows panel
  must rename it or "workflow" means two things in one file. CLAUDE.md's
  harness-flag list is stale (`--pixvix`/`--ink-drill` don't exist;
  `--viziwiz`, `--content-root`, chat flags missing).

## Tiers (final)

- **T0 — record hygiene** (rides WF1's PR, zero risk): rewrite gate-table
  rows 2/3/4/5/10 as DONE with their W1/W2 commits; retire ladder rungs
  1–2; row 6 → "packs landed, render leg dead"; row 11 → "ureq landed,
  needs OAuth + Drive Browse"; retire §8 ask #4; fix CLAUDE.md harness
  flags; record the reference-repo truth in the port-capture skill's
  assumptions.
- **T1 — capability parity** (each pass experience-gated, ordered):
  **T1a** land L2 Lottie + wire the `Source::Element` render leg (nearest
  wins; row 6 closes). **T1b** comp schema v2: transforms
  (pos/scale/rot/opacity) + KEYFRAMES + multi-select + split (the
  foundation every AE family needs; schema refusal law carries v1→v2).
  **T1c** effects & transitions registry (xfade/fades/color v1, census
  gate like the slides registry; render honors it). **T1d** editor text +
  captions: `Source::Text`, title presets as ingredients; caption lane via
  staged ffmpeg (whisper transcription + libass burn-in — CapCut parity as
  pure subprocess work). **T1e** audio: drop `-an` — audio tracks,
  waveform, Narrate VO. **T1f** precomp (`Source::Comp`). **T1g** masks
  (ort mini-ADR) + the aerender render lane (wire `engine-ae`: operator's
  real `.aep` → rendered footage via the recorded flag matrix). **T1h**
  Drive Browse (OAuth ask first).
- **T2 — WF1, the first pass**: `dpl-workflows` + Workflows tab +
  storyboarding walking skeleton + `--run-workflow`. Full plan:
  `docs/plans/wf1-workflow-engine.md`.
- **T3 — the ratified workflows**: storyboard→episode full · how-to/
  explainer · LinkedIn PDF carousel (**prereq: poppler staged into the
  depot** — also closes W11's local gap) · episode (stitching/ads/post,
  pulls T1c/T1e).
- **T4 — A2UI drive**, in cost order from the evidence: **T4a** comp
  write-signal + editor reload rider (the silent-save break). **T4b** seam
  workflow verbs (list/run/status) + deck on SSE. **T4c** shared agent
  identity in `Tree::Agents` (one session across seam + coach). **T4d**
  C3 ColdLight catalog + chat-edits-my-comp (R2c) — the agent composes
  surfaces and edits alongside the operator.

## The ComfyUI concepts port (operator directive, 2026-07-27 evening)

> "We want to import and port ComfyUI concepts to our system… if I need to
> allow python, c++, ensure this egui based version gets built methodically
> and truly useful… workflows for all of the key stages… LinkedIn PDF fancy
> carousels… Storyboarding, bibles… create the clips to then stitch and
> enhance with post processing and effects, platform industry aligned."

Grounded against the Aug-2026 state of the art, the concepts worth porting
(the ideas, rebuilt egui-native — never a webview):

1. **Workflow = directed node graph, two serializations.** ComfyUI nodes
   are `{id, class_type, inputs}` where inputs reference literals or other
   nodes' outputs, with a UI-persistence form and an execution form
   stripped of UI metadata. Our mapping: `WorkflowDoc` v1 (WF1) is the
   *linear* degenerate case; **v2 evolves to the graph shape** with the
   same two-form split (panel layout vs executor plan) under the existing
   schema-refusal law.
2. **Subgraphs = workflows packaged as super-nodes** with declared I/O —
   ComfyUI's reuse mechanism, a perfect match for our ingredients law: a
   subgraph IS a library ingredient. Storyboard→clips→stitch→post composes
   as subgraphs; **bible artifacts** (show/character/style bibles) become
   upstream nodes whose outputs feed every downstream stage — identity
   consistency à la Higgsfield's Soul ID, but document-shaped and ours.
3. **Blueprints**: pre-built workflow documents shipped in the registry
   census (LinkedIn fancy carousel, how-to, episode) — one line each, like
   the slides registry.
4. **Node-graph UI, egui-native.** Candidates: `egui-snarl` (serde graphs,
   styled wires, context menus — strongest fit), `egui-graph-edit` /
   `egui_node_editor` (Blackjack lineage). Decision lands in the WF2 plan
   (⚠ new-dependency ask-first; a ColdLight-tuned homegrown canvas is the
   fallback if none meets the trait doctrine).
5. **Local configuration = loopback services.** ComfyUI itself (Python)
   runs as an operator-owned LOCAL server with a JSON `/prompt` API — the
   integration is the pattern we already own: a loopback backend profile
   (the catalog's "Add Local" concept; ADR-009 spirit — external runtimes
   as subprocesses/services, never crate deps). **Operator allowance
   recorded:** Python/C++ are permitted in the estate where they buy
   leverage, as external services/tools DevPULSE orchestrates over
   loopback or subprocess; the egui app itself stays pure Rust,
   methodical, foundation-first.

Sequencing addendum: WF1 unchanged (linear engine, walking skeleton).
**WF2 = graph schema v2 + node panel + subgraph-as-ingredient. WF3 = local
backend profiles (ComfyUI endpoint) as generation stages.** The bible
artifact enters T3's storyboard workflow as its first upstream node.
Sources: ComfyUI workflow-JSON/subgraph docs (deepwiki.com/Comfy-Org),
comfyui-wiki.com subgraph feature, github.com/zakarumych/egui-snarl.

## Ratification record (operator, 2026-07-27, in-session)

1. ✅ WF1 is the next pass (T2 before further T1).
2. ✅ T1 order as listed — T1a nearest-wins first, T1b keyframes second.
3. ✅ Captions via staged ffmpeg (whisper + libass) join T1d.
4. ✅ Poppler (pdftoppm) gets staged into the depot (first step of WF1
   execution, alongside the T0 doc corrections).
