# Pass 0 — ColdLight treatments: decide + capture neubrutalism/claymorphism (the composable trait system)

Status: **DRAFT — awaiting operator ratification.** No code until §3's specs are
signed off. This plan is the missing gate: it captures the visual language the
first app carried in feel, faithfully, before any more UI ships.

## 1. Context — why this pass exists

The port replaced the original's real design system with an invented one, and the
gate that should have caught it was never built. Ground-truthed this session:

- The original (`C:\localdev\devpulse labs\src\lib\design`, READ-ONLY) ran a full
  **ColdLight/renderForge** pipeline: authored JSON → Style Dictionary → CSS vars,
  a 26-axis contract, and **traits = stackable partial looks** (`neumorphic`,
  `cyber`, `retro`) composed per-axis — additive axes concatenate, override axes
  last-wins. Documented in the original's `src/lib/design/TRUTH.md` (decisions
  D1–D14). Its stated aesthetic specialty was **neumorphic/soft-UI** (D14).
- The port kept a **faithful token half** (`crates/coldlight/build.rs` ports
  `generate-scales.mjs`: OKLCH ladders, fluid, motion, moods, fonts, presets, with
  manifest⇄truth validation) — good, keep it. But it **dropped the trait half**:
  no traits, no compositor, no axis contract, no editor manifest. Styling is
  hand-rolled `brutal_frame`/`clay_frame` helpers over `docs/design/doctrine.md`,
  a thin invented BRUTAL/CLAY language (the words "brutal"/"claymorphism" appear
  nowhere in the original).
- `docs/plans/experience-inventory.md` — which CLAUDE.md AND doctrine.md say gates
  every UI pass — **was never authored**. Sweeps 1–6 had no reference.

Operator ruling (2026-07-24): capture the treatments properly, as composable
authored data, with the same care as the first app. Anchors = **neubrutalism +
claymorphism**; **cyber + retro stack on top**. Neubrutalism is the egui-native
strength trick (glass is weak/expensive in egui): flat saturated fill, crisp hard
edges, hard offset shadow, a painted "lit-from-behind" glow — loud but cheap.

## 2. Current state (all ground-truthed this session)

- `crates/coldlight/build.rs` — generates ramps/fluid/motion/moods/fonts/presets
  from `tokens/*.json`; unknown refs fail the build; `*_gen.rs` marked DO-NOT-EDIT.
- `crates/coldlight/src/lib.rs` (1185 lines) — hardcoded surface/ink `Color32`
  consts, `Tone`, `brutal_frame`/`clay_frame`/`scatter_frame`/`card_frame`, `tip`,
  `Mood`, `Fluid`, `Effect`. NO trait/compose/manifest concept.
- `tokens/` — hues, fluid, motion, moods, fonts, presets. **No `traits.json`.**
- `presets.json` — "look" bundles (font+moods+motion) and "motion" bundles. This
  is a coarse recipe, NOT the per-axis trait stack.
- `docs/design/doctrine.md` — the thin BRUTAL/CLAY doctrine (to be rewritten here).
- Original trait recipes (read this session, `.../presets/trait.tokens.json`):
  - `neumorphic`: shadow `-.5/-.5/1rem light@10%` + `.5/.5/1.25rem void@55%`
    (directional), radius `xl`, hairline edge `light@6%`.
  - `cyber`: shadow `0 0 .75rem cyan400@70%, 0 0 1.75rem cyan600@50%`, gradient
    `cyan500@14%` top, `hue-drift 6s`, radius `sm`, edge `cyan400`, ink `cyan100`.
  - `retro`: inset `0 0 .75rem amber900@60%`, repeating scanline gradient,
    `crt-flicker 4s`, radius `xs`, edge `amber700`, ink amber.

## 3. Target design — RATIFY THIS SECTION

### 3a. The axis contract (egui-resolved struct, not CSS vars)

`ResolvedStyle` — the concrete struct the compositor produces and paint fns read.
Each field is one axis, classed additive or override:

| Axis | Type | Class | Notes |
|---|---|---|---|
| `fill` | `Color32` | override | flat surface fill (from `ramp`/`Tone`) |
| `edge` | `Stroke` | override | color + width, `StrokeKind::Inside` (crisp) |
| `radius` | `u8` | override | `CornerRadius` |
| `ink` | `Color32` | override | text/foreground |
| `font` | `&'static str` | override | font-class id |
| `shadows` | `Vec<Shadow>` | **additive** | `offset:[i8;2] blur:u8 spread:u8 color` |
| `tints` | `Vec<TintLayer>` | **additive** | gradient/wash layers (`gradient_tint`) |
| `back_glow` | `Vec<Glow>` | **additive** | the painted "lit-from-behind" pop |
| `anim` | `Vec<&'static str>` | **additive** | effect ids into the motion registry |

Additive set = `{shadows, tints, back_glow, anim}` (a static table beside the enum,
mirroring the original's `ADDITIVE_AXES`). Everything else = override, last-wins.

### 3b. The compositor

`compose(base: TraitId, stack: &[TraitId], overrides: &StyleOverrides) -> ResolvedStyle`
— a **pure fn**: start from `base`, fold each `stack` trait in order (additive Vecs
extend; override scalars replace if the trait defines that axis), then apply
per-instance `overrides` (an editor hue/font pick) last. Pure ⇒ unit-tested +
cacheable by `(base, stack, overrides)` hash (satisfies "fast despite visually
strong"). Interaction states (`hover`/`active`/`selected`) select a state-variant
of each trait's axis map, resolved in EVERY state (fix the original's B1 defect —
all axes resolve in every state branch).

### 3c. The treatments as authored data (`tokens/traits.json`, generated → Rust)

Proposed specs — **grounded in the original recipes + the egui paint vocabulary,
for the operator's eye.** Values are the ratification target.

- **`neubrutalist`** (base, actionable): `fill` = tone@400 flat; `edge` = tone@700
  2px; `radius` = `RADIUS_SM`; `shadows` = one hard `[3,3] blur:0 spread:0`
  tone@900; `back_glow` = one cheap tone@300 halo behind the rect (the
  "lit-from-behind"); `ink` = ink.strong. Hover = shadow → `[4,4]` + glow up;
  active = fill tone@500 wash, shadow → `[1,1]` (pressed).
- **`claymorphic`** (base, passive): `fill` = surface.RAISED; `edge` = hairline;
  `radius` = `RADIUS_LG`; `shadows` = directional pair (light `[-4,-4] blur:12`
  ink-tint@low + dark `[6,6] blur:16` well@high); `tints` = optional two-color
  diffuse (135°). No back-glow (calm). Content-well tone.
- **`cyber`** (stack): `back_glow` += cyan glow (two-stop); `tints` += cyan@14%
  top wash; `anim` += `hue-drift`; `edge` = cyan@400 thin (override, if last);
  `radius` = `RADIUS_SM`; `ink` = cyan@100.
- **`retro`** (stack): `shadows` += inset amber@900 scanline sim; `tints` +=
  repeating scanline; `anim` += `crt-flicker`; `edge` = amber@700 chunky;
  `radius` = `RADIUS_XS`; ink amber. (Scanline/flicker need a generated
  `tokens/motion.json` entry + a cheap painter, no real blur.)

Stacking example: `base=neubrutalist, stack=[cyber]` → hard brutalist edge/shadow
PLUS cyan back-glow + hue-drift both paint (additive), radius/edge from cyber
(override, last). One data change, not a hand-authored combination.

### 3d. The manifest + the editor seam

`build.rs` emits a `manifest_gen.rs`: every trait (id, label, blurb, which axes it
defines, additive/override) + every element/block/template (id, category, slots).
The Workbench editor reads THIS (only offers what exists). An editor override
writes into the slide's `recipe`/`overrides` in `slide.json`; the compositor reads
it at paint. This is the original's D9/D12 (recipe-as-data + manifest), in egui.

### 3e. The experience inventory (the reference)

Author `docs/plans/experience-inventory.md` — a screen-by-screen + slide-by-slide
capture of the original's look/feel (screenshots of the old app where runnable, or
its component structure) as the acceptance bar every later pass measures against.
This closes the gate CLAUDE.md/doctrine.md require.

## 4. Changes, in execution order (AFTER ratification — each its own pass)

1. Rewrite `docs/design/doctrine.md` from the thin BRUTAL/CLAY to the ratified
   trait doctrine (§3), pointing at this plan.
2. Author `tokens/traits.json` (the 4 treatments, §3c) + extend `build.rs` to
   generate `traits_gen.rs` (with axis-presence + additive tables) and validate
   refs (unknown hue/motion fails the build).
3. Add the axis enum + `ResolvedStyle` + `compose(...)` in `crates/coldlight`
   (pure, unit-tested); migrate `brutal_frame`/`clay_frame` callers onto it;
   retire the bespoke helpers (no shim, pre-release).
4. Emit `manifest_gen.rs`; expose the manifest read API for the Workbench editor.
5. Author `docs/plans/experience-inventory.md`.
6. THEN the `slide-catalog-pass` loop (separate passes) ports the catalog through
   the trait system, each item with an organized seed (category × tags).

## 5. Boundaries

- Slide **categories/tags** are OUT of the styling system — orthogonal (doc schema
  + registry). This pass only defines the styling ingredients + the manifest seam.
- ViziWiz rename (`pixvix`→`viziwiz`) is deferred (operator, 2026-07-24) — not here.
- Keep the whole generated-token half of `build.rs` — it is faithful; only ADD
  traits + compositor + manifest.
- No `#[allow]`, no re-export shims, no blur/feathered borders (egui-build laws).

## 6. Verification

- `cargo green` after each §4 step.
- Compositor unit tests: additive axes concatenate; override last-wins; every axis
  resolves in every state; cache-key stability.
- Proof shots: one neubrutalist + one claymorphic surface, each plain and with a
  cyber/retro stack, at maximized AND `--window 1100x700`. Feel-bar vs the old app.
- Manifest test: an ingredient absent from the manifest cannot be offered (gallery
  gate). Build fails on a recipe naming a missing trait.

## 7. Commit plan

One PR per §4 step (doctrine rewrite · traits+generator · compositor+migration ·
manifest · inventory), each green + proof-shot. Rebuild the operator exe + move
mtime at session end. Update `docs/STATE.md` KNOWN-BROKEN #5 as each lands.

## 8. Executed — evidence

**Ratified by the operator ("go", 2026-07-24).** Foundation landed (§4 steps 1–3
core) on branch `design/coldlight-traits`:

- **Step 1 — doctrine.** `docs/design/doctrine.md` rewritten from the invented
  BRUTAL/CLAY to the composable trait system (concept, treatments, per-axis law,
  laws, category/tag orthogonality). `dpl-style` skill removed; `coldlight-system`
  + `slide-catalog-pass` skills authored; STATE.md KNOWN-BROKEN #5 recorded.
- **Step 2 — traits as data + generator.** `crates/coldlight/tokens/traits.json`
  authored (neubrutalist/claymorphic base + cyber/retro stack; ladder-relative
  color refs). `build.rs::generate_traits` emits `traits_gen.rs`, cross-validating
  every hue/step ref against the pool + `LADDER_STEPS` (unknown → build fails).
- **Step 3 core — compositor.** `ColorRef`/`EdgeSpec`/`ShadowSpec`/`GlowSpec`/
  `TintSpec`/`Axes`/`TraitDef`/`TraitState`/`ResolvedStyle` + the pure
  `compose(tone, base, &[stack], state)` in `crates/coldlight/src/lib.rs`.
  Additive axes concatenate, override axes last-wins, per-trait state overlay.

DEVIATIONS from §3 (corrections law):
- **anim axis deferred.** cyber/retro `hue-drift`/`crt-flicker` are NOT emitted yet
  — they need `motion.json` entries + a cheap painter (scanline/flicker). `anim`
  is carried in the schema, empty in data. Tracked for the next foundation step.
- **State scope.** Interaction states implemented for neubrutalist (hover/active);
  claymorphic/cyber/retro carry no state overlay (correct — passive/stack).
- **Not yet done (own passes, deliberately deferred for care + proof):** the paint
  application API (back-glow + Frame + content), migrating the 9 `brutal_frame`/
  `clay_frame` callers, retiring the bespoke helpers, the generated manifest, and
  the experience inventory (§4 steps 3-migration, 4, 5).

PROOF: `cargo green` ✔ (fmt + clippy + 23 coldlight tests incl. 5 new trait tests:
additive-concatenate, override-last-wins, state-overlay-replaces, tone-relative
fill, every-trait-paints-in-every-state; + full workspace tests + release build).
Operator exe rebuilt this pass.
