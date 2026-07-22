# W2 — the document renderer: mood-blend runtime + seeds rendered live

Parent: docs/plans/authoring-architecture.md (ratified). Follows W1
(`8b8aa14`). This pass makes documents RENDER: the mood-blend background
(the operator's tap-the-background spec), text blocks resolved through the
registries, and every seed live on a Workbench wall.

## 1. Context

W1 gave slides a data form. Nothing renders it yet. The operator's spec:
"click the actual background of the slide and add a mood where two
gradients blend and move subtle and then add the pulse animation" — W2
builds the runtime that W4's taps will drive.

## 2. Current state (ground-truthed this session)

- `cl::mood_gradient` (lib.rs:387) — single-mood 135° per-vertex grid mesh;
  `Mood::at(t)` samples the stop ladder.
- `tokens/motion.json` effects: 17 named; no `blend-drift`;
  `ambient-drift` (12s loop) is the precedent shape.
- `crates/slides/src/blocks/media.rs` — `Ambient::{Rings,Drift,Pulse}`
  painted inside `media_backdrop`, colored by a `Tone` argument.
- `crates/hubs/workbench` — placeholder surface, subroutes
  STUDIO/LIBRARY/TOKENS.
- Unregistered `FontFamily::Name` PANICS at layout — unknown font refs
  must be guarded to Proportional (degrade law).

## 3. Target design

- **`blend-drift`** joins tokens/motion.json (16s, standard, loop) — the
  authored "move subtle" clock.
- **`cl::Mood::blend_at(a, b, t, mix)`** (pure, testable) +
  **`cl::mood_blend(painter, rect, a, b, mix, pan)`** — ONE mesh, blend in
  the SAMPLE (never stacked translucent layers); `pan` shifts the diagonal
  phase (±, clamped by `at`, no wrap seam).
- **`slides::blocks::ambient_backdrop(ui, rect, amb, color, now, t0)`** —
  the three ambience painters extracted from `media_backdrop` (which
  delegates, passing `tone.color()`; callers unchanged). Documents color
  ambiences by the blend's leading-mood ACCENT, not a Tone.
- **`slides::document`** — the ONE renderer:
  `render(ui, rect, &dpl_doc::Slide, now, t0)`:
  - background MoodBlend → look up both moods (unknown → labeled plate),
    drive mix/pan off `BLEND_DRIFT` (cos/sin of the loop phase), half-alpha
    leading scrim for legibility, then the optional ambience;
  - blocks → galley-laid text stack (wrapping), font =
    `cl::font_family` guarded against the pool, size = fs-ref resolved via
    `Fluid::of(rect)` (unknown → body), ink map (tone → leading accent);
  - `enter: rise` → ENTER_RISE staggered per block, like title_card.
- **Workbench → Seeds** subroute: every `dpl_doc::list_seeds()` folder
  rendered LIVE on a 16:9 stage with its id, colocated path, and any
  validator findings (the gallery gate for documents).

## 4. Changes, in execution order

1. tokens/motion.json: add `blend-drift`.
2. coldlight: `Mood::blend_at` + `mood_blend` + tests (endpoints equal the
   single moods, midpoint between).
3. slides/blocks/media.rs: extract `ambient_backdrop` (public), delegate.
4. slides/src/document.rs (new module): resolvers + `render`; tests: all
   10 size names resolve, unknowns degrade, a doc with unknown font/mood
   renders through a real theme'd Context without panic.
5. hubs/workbench: SEEDS subroute + seeds wall (loads once, reload chip);
   deps + dpl-doc, dpl-slides.
6. `cargo green`; exe proof `--route workbench/seeds`; commit.

## 5. Boundaries

- 🚫 No editor interactions (W4). No presets (W3). No changes to existing
  template demos or the app backdrop.
- 🚫 `media_backdrop` behavior for existing callers must not change
  (pure extraction).

## 6. Verification

1. `cargo green` exits 0 (new coldlight blend tests + slides document
   tests included).
2. Real exe: `--route workbench/seeds --shot-frame 40 --screenshot
   docs/proof/w2-seeds.png` — all four seeds visible, each with its own
   fonts and a DIFFERENT two-mood gradient; terminal-retro visibly 8-bit;
   title-neon carries the pulse ambience rings.
3. A doctored seed with a bogus font renders a degraded (not crashed)
   stage — covered by the unit smoke test.

## 7. Commit plan

`feat(slides): W2 — document renderer, mood-blend runtime, Workbench seeds
wall`. No push.
