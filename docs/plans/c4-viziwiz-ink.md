# C4 — ViziWiz ink editor (annotate, flatten → inbox)

Parent: roadmap Program C (#108) — the program's LAST pass. After C3
(18af135). Worktree: `C:\localdev\dpl-main` on main.

## 1. Context

Roadmap C4: *"ViziWiz ink editor (wiredFRONT-side, never mixed with
Workbench): ink tones/width/undo, flatten → inbox via forced metadata.
Pilot: stroke + flatten flow, baseline on the composite."*

The old app's `viz-editor.tsx` (273 lines, read in full this session):
pick an image artifact from a thumb strip; strokes draw on a canvas at
natural size (round caps, pointer-captured); SIX fx tones
(cyan/magenta/lime/coral/violet/amber — exactly ColdLight's `Tone`
enum), width 2–24, Undo (last stroke), Clear; **Flatten** composites
base + ink into ONE NEW PNG filed through the forced-metadata inbox
flow (blank name auto-seeds `<src-minus-_vN>-annot`, default tag
"annotated") — the SOURCE IS NEVER MODIFIED (never-mutate law). This is
distinct from the stream-time deck ink (D3) and from the future R1
editor (vello ADR) — ViziWiz is the small, real annotate-and-file tool.

## 2. Current state (ground-truthed this session)

- `crates/hubs/wiredfront/src/lib.rs` (933 lines, read in full): the
  VIZIWIZ SubRoute exists but falls to `placeholder::surface`. The hub
  carries GalleryView (index rows, filter chips, thumb_uri with the
  `file:///` three-slash law, filter_chip) and SessionsView. Deps:
  egui, dpl-hub, dpl-coldlight, dpl-index, dpl-doc, dpl-engine-capture,
  dpl-video, serde_json, egui-pilot-ai — NO image, NO engine-media, NO
  engine-steps.
- `crates/engine-media/src/lib.rs`: the image-ops crate (optimize
  ladder, cutout, anim) — owns the `image` dependency; no ink module.
- `dpl_engine_steps::{CaptureMeta, normalize_meta}` — the ONE
  forced-metadata implementation (C2's Save card uses it).
- `dpl_index::record_for_capture` + tags/show association — the C2/C3
  index write-through pattern.
- `cl::Tone::{Cyan,Lime,Violet,Amber,Magenta,Coral}` + `.color()`.

## 3. Target design

- **Engine tier** — `crates/engine-media/src/ink.rs`:
  `InkStroke{color: [u8;3], width: f32, points: Vec<[f32;2]>}` (IMAGE
  pixel space) and `flatten_ink(src, dest, &[InkStroke])`: decode src →
  RGBA, stamp each stroke as filled discs marched along its segments
  (radius = width/2, spacing ≤ r/2 — round caps and joins for free),
  save dest PNG. Never touches src. Unit-tested on a generated fixture
  (stroked pixels colored, off-path pixels untouched, src bytes
  identical) — the "baseline on the composite" in engine form.
- **Surface tier** — `crates/hubs/wiredfront/src/viziwiz.rs` (the hub
  file is 933 lines; new surfaces get their own module — the C2
  alignment law):
  - `VizView{rows, open: Option<VizEdit>, status}` — rows = index
    images; the picker wall reuses the gallery's card geometry (thumb +
    name, tap opens).
  - `VizEdit{rel, src_name, tex: Option<TextureHandle>, size,
    strokes, drawing, tone: usize, width: f32, name, tags, library,
    show, episode, status}` — strokes in image space; texture loaded
    once via `image::open` → ColorImage.
  - The editor: back row · the CANVAS (image fit to the available
    width, aspect-true; drag draws in image coordinates; stroke widths
    scale with the fit factor; six tone dots + width slider + Undo +
    Clear) · the metadata row (name/tags/General⇄Episode — the C2 Save
    card's shape) · **Flatten & file** → `normalize_meta` (blank name
    seeds `<src>-annot`, blank tags seed "annotated") →
    `flatten_ink` into a unique `inbox/<name>.png` → sidecar+index with
    tags (and the episode association when chosen) → honest status.
    A refusal keeps the strokes and the card.
  - Pilot ids: `viziwiz.pick/<rel>`, `viziwiz.canvas`,
    `viziwiz.tone/<n>`, `viziwiz.undo/clear/flatten/back`,
    `viziwiz.meta.name/tags/dest`.
- **Wiring**: `mod viziwiz;` + `viziwiz: Option<viziwiz::VizView>`
  field + a VIZIWIZ route arm; Cargo += image, dpl-engine-media,
  dpl-engine-steps (all workspace deps already).
- **Harness**: `--ink-drill <rel>` — open the REAL editor state on an
  inbox image, push a corner-to-corner stroke programmatically, fire
  the REAL flatten action (blank name → auto `-annot`), print the
  status, exit 0 iff it reports the flattened file. Screenshot
  `--route wiredfront/viziwiz` for the picker wall proof.

## 4. Changes, in execution order

1. create `crates/engine-media/src/ink.rs` + `pub mod ink;` — the
   rasterizer + tests.
2. create `crates/hubs/wiredfront/src/viziwiz.rs` — view/edit state,
   picker wall, editor, flatten (as §3); `pub(crate) fn surface` +
   `pub(crate) fn drill`.
3. edit `crates/hubs/wiredfront/src/lib.rs` — module + field + route
   arm + `pub fn ink_drill(&mut self, rel: &str) -> String`.
4. edit `crates/hubs/wiredfront/Cargo.toml` — the three deps.
5. edit `crates/app/src/main.rs` — `--ink-drill <rel>` (frame-30 fire,
   print, exit).
6. `cargo green`; drills (§6); roadmap tick (Program C ✅ COMPLETE);
   §8; commit; push.

## 5. Boundaries

🚫 The deck's D3 ink (crates/deck ink.json) untouched — different
system. 🚫 Sources never modified; `/content` never authored into (the
flatten lands in the inbox). 🚫 No new external deps. ⚠️ Cutout (ort/
BiRefNet, the subroute blurb's other half) is NOT this pass — own ADR.
Out of scope: the pilot stroke scenario over the in-app canvas
(scenario-driven per the roadmap — needs the Test Lab runner lane; the
engine unit + drill carry verification meanwhile, recorded).

## 6. Verification

- `cargo green` exits 0 (incl. the ink rasterizer tests).
- `--ink-drill inbox/pix.png` exits 0: status names the flattened
  `…-annot…png` in the inbox; the source file's bytes are unchanged
  (asserted in the drill).
- `--route wiredfront/viziwiz --screenshot docs/proof/c4-viziwiz.png`
  — the picker wall with image thumbs.
- `--pixvix-snap` still exits 0 (regression).

## 7. Commit plan

`feat(wiredfront): C4 — ViziWiz ink editor (tones/width/undo, flatten
→ inbox) — Program C complete`. Push backgrounded.

## 8. Executed — evidence

Executed to plan. Proven: `cargo green` exits 0 (incl. the rasterizer
tests — stroked pixels colored, off-path untouched, SOURCE BYTES
IDENTICAL — and `strip_v`); `--ink-drill inbox/pix.png` exits 0 with
"flattened → inbox/pix-annot_v1.png (source untouched)" — the real
flatten path headless, auto `-annot` + `_v1` metadata, source-bytes
equality asserted in the drill itself;
`docs/proof/c4-viziwiz.png` shows the picker wall listing FIVE images
INCLUDING `pix-annot_v1` whose thumbnail visibly carries the drill's
cyan diagonal — the composite is real, indexed, and browsable;
`--pixvix-snap` still exits 0.

Deviations: the editor preview paints strokes with egui line segments
(flat caps) while the engine flattens round — a preview-only cosmetic
gap; the in-app stroke pilot scenario waits on the Test Lab runner lane
(recorded in §5). **Program C (#108) is COMPLETE: C1 stills, C2 steps,
C3 video, C4 ViziWiz.**
