# R1 PROPOSAL — the WiredFRONT editor (for operator ratification)

Status: PROPOSAL — the operator gave the vision (2026-07-23); this is
the architecture half. Ratify, amend, or redline before any code.
Parents: ADR-011 (rendering-stack.md), ADR-009 (ffmpeg subprocess law),
ADR-010 (ComfyUI first pluggable backend). Ledger #114.

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

**2. The editor surface — ADR-011's picks, realized.**
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
OPEN QUESTION 1: copy into `content/elements/<pack>/` vs index
in-place on the external drive (rec: index in-place + copy-on-use into
the comp's folder, so gigs stay where they live and comps stay
portable).

**5. The agent seam — "chat route aware".**
The route-aware chat (F3 provider mesh / Council) gets TYPED tools:
search the elements/artifact index; propose comp.json edits (validated
by the comp doctor before apply — an agent cannot write an invalid
comp); enqueue generation jobs (ComfyUI first per ADR-010, hosted
APIs as later adapters) whose outputs land as REAL FILES in the inbox
(write-through indexes them — nothing synthesized); enqueue renders.
Generation is an engine job queue (mpsc, not broadcast — the fabric
doc's own rule).

**6. Matting — background removal / transparency.**
ort + BiRefNet as an engine job: images first, video as a batched
frame pipeline. Needs its own mini-ADR (ort runtime + GPU execution
provider choice) — flagged, not smuggled in.

## Proposed passes

R1a comp document + doctor + panes shell + clip-list stitching UI →
R1b stitch render (concat/xfade) + preview playback →
R1c elements ingest + palette + overlay layers →
R1d effect registry (ffmpeg tier) + ViziWiz mode →
R1e matting (after the ort mini-ADR) →
R1f agent tools (needs F3 mesh).
Each pass: plan → green → real-exe proof, as always.

## Open questions for the ratification conversation

1. Elements: index-in-place from your drives (rec) or copy-in? And a
   sample inventory of the formats in the gigs (alpha video? PNG
   sequences? .aep sources needing render-out first?).
2. Preview bar for v1: reduced-res realtime play (rec) or frame-scrub
   only?
3. Comps as inbox artifacts keepable into episodes (rec) — confirm.
4. Generation backends order: ComfyUI local first (ADR-010), then
   which hosted APIs?
