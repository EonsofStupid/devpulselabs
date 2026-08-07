# R1 PROPOSAL — the WiredFRONT editor (for operator ratification)

Status: PARTIALLY RATIFIED — the operator gave the vision (2026-07-23)
and, in the 2026-07-24 ratification conversation, resolved the four
gating forks (see "Ratified decisions" below). This is the architecture
half; the passes may now begin, R1a first. Parents: ADR-018
(`docs/design/adr-018-rendering-stack.md`, filed at the time as ADR-011 —
RS4 names the R1 stack: egui_tiles docking + egui_wgpu/vello compositor
behind our own Layer/Compositor traits),
ADR-009 (ffmpeg subprocess law), ADR-010 (ComfyUI first pluggable
backend). Ledger #114.

## Ratified decisions (operator, 2026-07-24)

1. **Generation/editing backend = HYBRID from day one.** A
   backend-agnostic job queue: each media op (image gen/edit, video,
   effects) routes to whichever backend — LOCAL (ComfyUI, ADR-010) or
   HOSTED (media-model APIs) — is configured. The chat mesh
   (Claude/Codex/Grok) ORCHESTRATES either way; the pixels come from a
   backend, never from the chat channel itself. CAPABILITY REALITY
   recorded: Claude/Codex subscriptions are text/vision (they instruct
   and edit-by-description); actual image/video/matte pixels need a media
   backend (Grok image gen, dedicated image/video keys, or local
   ComfyUI). The ffmpeg/effect tier stays local regardless. This
   SUPERSEDES §5's "ComfyUI first, hosted later" — both from the start,
   behind one `GenBackend` trait.
2. **Background removal = LOCAL ort + BiRefNet** (§6 confirmed). Needs
   the ort-runtime mini-ADR (GPU execution provider) authored before its
   pass (R1e).
3. **Elements = COPY ALL into `content/elements/<pack>/`.** This
   OVERRIDES §4's "index in place, copy-on-use" recommendation: the gigs
   are ingested into the content tree up front (operator data, gitignored
   — never committed), then X1-indexed from there. Self-contained; the
   palette and agents search one root.
4. **Scope = amend R1 (this edit), then plan + build R1a** (comp
   document + doctor + editor panes shell + clip-list stitching UI).

## The vision (operator, 2026-07-23, their words compressed)

"After Effects but with our chat route aware" — with a ViziWiz mode
for editing/post-processing/special-effects elements. The main focus:
**stitch series of 8–15 second AI-generated clips**, then properly
post-process, enhance, apply effects. Support the operator's custom
After Effects elements — "gigs and gigs" — that AI can leverage for
animations and access for editing. Background removal / transparent
background. An environment where Codex/Claude can generate images and
video. Personal use — licensing is a non-constraint. Output: individual
videos usable as a single slide or in a presentation.

## Architecture (the boring, industry-aligned shapes)

**1. The composition document — data, never code (the document law).**
A comp is a folder artifact (inbox-born, keepable into episodes like
everything else): `comp.json` = `Composition { fps, size, duration,
tracks[] }` → `Track { layers[] }` → `Layer { source (an artifact
rel_path OR an element id — refs never values), in/out, transform
keyframes, effects: [effect refs] }`. Keyframe curves use the authored
motion grammar (coldlight motion tokens + params). A comp doctor
validates every ref. **This is what makes it AI-editable: agents read
and write plain validated JSON — the same trick as slides.**

**2. The editor surface — ADR-018's picks, realized.**
`egui_tiles` panes: preview · timeline · palette (elements + artifacts
from the index) · inspector. `vello` for vector overlays and effect UI,
behind our own Layer/Compositor traits (exit honesty). ViziWiz mode =
a mode of this editor operating on the selected layer/frame (ink,
quick-post, callouts) — C4's ViziWiz grows up into it; never mixed
with Workbench (the two-editors law).

**3. Preview vs render — two tiers, honestly separated.**
- PREVIEW: dpl-video decode → GPU textures, transforms/opacity
  composited in wgpu at reduced resolution — scrub + play for editing
  decisions, not pixel-final.
- FINAL RENDER: ffmpeg subprocess (ADR-009, never a crate dep) driven
  by an authored EFFECT REGISTRY — effect id → filtergraph template
  (concat/xfade for stitching; overlay-with-alpha for elements;
  eq/curves/sharpen for enhancement). What ffmpeg can't express
  pre-renders per layer. Registry = another token source: authored
  JSON, build-validated, zero code per effect.

**4. The elements library — the gigs, made queryable.**
An INGEST pass walks the operator's element stores (rendered
transparent media: alpha WebM/MOV, PNG sequences; Lottie where
exported), writes X1 sidecars (kind "element", pack + tags), and
indexes them — the palette and the AGENTS then search the same index.
RESOLVED (ratified decision 3): COPY ALL into
`content/elements/<pack>/` up front, then X1-index from there — one
root the palette and agents both search. (The earlier index-in-place
recommendation is retired.)

**5. The agent seam — "chat route aware".**
The route-aware chat (F3 provider mesh / Council) gets TYPED tools:
search the elements/artifact index; propose comp.json edits (validated
by the comp doctor before apply — an agent cannot write an invalid
comp); enqueue generation jobs (RATIFIED decision 1: HYBRID — local ComfyUI
per ADR-010 AND hosted media APIs from day one, behind one `GenBackend`
trait; the chat mesh orchestrates, a backend renders) whose outputs
land as REAL FILES in the inbox
(write-through indexes them — nothing synthesized); enqueue renders.
Generation is an engine job queue (mpsc, not broadcast — the fabric
doc's own rule).

**6. Matting — background removal / transparency.**
ort + BiRefNet as an engine job: images first, video as a batched
frame pipeline. CONFIRMED (ratified decision 2): local ort + BiRefNet
is the matting path. Needs its own mini-ADR (ort runtime + GPU
execution provider choice) authored before R1e — flagged, not smuggled
in.

## Proposed passes

R1a comp document + doctor + panes shell + clip-list stitching UI →
R1b stitch render (concat/xfade) + preview playback →
R1c elements ingest + palette + overlay layers →
R1d effect registry (ffmpeg tier) + ViziWiz mode →
R1e matting (after the ort mini-ADR) →
R1f agent tools (needs F3 mesh).
Each pass: plan → green → real-exe proof, as always.

## Open questions

RESOLVED 2026-07-24: #1 elements storage (→ copy-all, decision 3) and
#4 backend order (→ hybrid, decision 1). STILL OPEN, needed before the
passes they gate — not before R1a:

- **Element format inventory** (gates R1c): what actually lives in the
  gigs — alpha WebM/MOV? PNG/EXR sequences? Lottie exports? `.aep`
  sources that must be rendered out first (the app never runs After
  Effects)? A sample listing sets the ingest matrix.
- **Preview tier for R1b** (gates R1b): reduced-res realtime play (rec)
  or frame-scrub only for v1?
- **Comps as inbox artifacts keepable into episodes** (rec) — confirm at
  R1's keep seam (leans yes; matches the X3 doctrine).
- **Hosted backend roster** (gates the hybrid adapters): which media
  APIs first — Grok image gen + which image/video/matte services?
