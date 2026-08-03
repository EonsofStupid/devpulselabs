# DevPULSE Design System — the composable architecture

**Status: RATIFIED 2026-07-27** (operator, with §6 decisions settled). The
overarching contract for the catalog: how UI is *built*, not just how it looks.
Composition metal is owned by [`doctrine.md`](doctrine.md) (FramePatch compositor; Theme face is S0) and is **not re-specified here** — this document defines the
layers around it and how they compose. Supersedes the ad-hoc "hand-painted
template" model (the W-series `title_card`/`quote`/… monoliths were a stopgap,
re-authored onto this in the §7.6 pass).

Ratified decisions: **element = the UI primitive** (the composer's imported
pack files rename `ELEMENTS → pack assets` gradually, copy-only, no churn now);
**big-bang catalog re-author** once the system stands (§7.6, one concentrated
consistency pass, not opportunistic).

Gate: every UI pass invokes `coldlight-system` and leaves surfaces clean
against BOTH this doc and doctrine.md.

---

## 0. The one idea

> **A template is a layout of ELEMENTS. Each element wears a TREATMENT
> (semantic or composite). Any interaction is a RECIPE. All of it paints
> through one 2D pipeline governed by a FIDELITY budget.**

Nothing is hand-painted that could be composed. A new slide design should be
*authored data* (elements + treatments + recipes) over *new paint code* — the
same law the slide document layer (`dpl_doc::Slide`) already follows for text.

This is the mature-design-system stack, mapped to a native egui app:

```
tokens        (generated pool: hues, fluid, motion, radius)      — design tokens
  └─ ELEMENTS       (primitives / atoms — organized, standalone) — Radix primitives
       └─ TEMPLATES     (compositions / organisms)               — components
   dressed by TREATMENTS  (semantic | composite)                 — variants / recipes (CVA, Panda)
   behaving via RECIPES   (interaction patterns)                 — interaction patterns
   painted under a FIDELITY budget (i5 baseline → scales up)     — progressive enhancement / graphics tiers
```

Industry parallels are exact: **design tokens** (Style Dictionary / Material's
core→semantic split), **atomic design** (atoms→organisms), **variant recipes**
(Stitches/CVA, Panda CSS literally calls them "recipes"), and **adaptive
fidelity** (game-engine graphics presets). The one place we go past a web
design system — native 2D rendering with a hardware cost budget — is correct
for egui and is the whole point of §5.

---

## 1. Elements (primitives)

An **element** is the smallest composable unit that paints or lays out — the
tier the current `crates/slides/src/blocks/` holds (`eyebrow`, `badge`, `stat`,
`bar`, `divider`, `media`), elevated to a real, organized, browsable library.

**Contract.**
- **Standalone.** An element reads only its arguments — never a route, never
  global state (carried from the blocks doctrine). It is usable on its own and
  inside any template.
- **Data in, paint out.** `element(ui/painter, Fluid, props, treatment,
  fidelity)`. It never hand-rolls a treatment — it *receives* a `FrameStyle`
  (or a treatment handle) and paints through it.
- **Categorized.** Elements live in named categories so the Library can shelve
  them: `type` (eyebrow/heading/body/mono), `mark` (badge/chip/tag/avatar/
  glyph), `meter` (bar/gauge/ring/spark), `divider`, `media`, `container`
  (plate/card/well), `control` (button/toggle/stepper). The category is the
  shelf, not a style.
- **Promotion rule (kept).** A pattern becomes an element only after it recurs
  in ≥2 templates. Inline first, extract when it repeats.
- **Cost-tiered (§5).** Each element declares its baseline tier and, if it has
  an enhanced form, the cheaper fallback it degrades to.

**Registry.** One census like `slides::registry::demos()` — an element that
isn't registered can't exist (the gallery gate), and a headless render test
paints every element at rest + mid-state without panicking.

---

## 2. Treatments — semantic vs composite

A **treatment** is the dressing an element wears. It always resolves to a
`FrameStyle` (doctrine.md's axis contract) — the two *kinds* differ only in
how they're named and chosen:

### 2a. Composite treatment (the explicit form)
A `base` + a `stack[]` + a `state`, resolved by `compose(tone, base, &stack,
state)` — exactly today's system: `base ∈ {neubrutalist, claymorphic}`,
`stack ⊆ {cyber, retro, …}`. This is the low-level, fully-explicit knob. "Wear
neubrutalist + cyber in Amber."

### 2b. Semantic treatment (the intent alias) — NEW
A treatment named by **role/intent**, not by construction:
`primary · secondary · danger · success · info · muted · ghost · surface`.
A semantic treatment is a **named binding** to `(base, stack, tone, state
behavior)` — the alias-token tier over the composite tier, exactly Material's
core→semantic split. "Wear `danger`" → resolves to (neubrutalist, [], Coral)
today; re-point the binding once and every `danger` element in the app moves.

**The law:** authored surfaces reach for **semantic** treatments
(`primary`, `danger`, …); only the design system itself binds semantics to
composites. This keeps intent decoupled from look — the whole reason semantic
tokens exist.

**Schema** (`tokens/treatments.json`, generated like FramePatches):
```
{ "id": "danger", "kind": "semantic",
  "base": "neubrutalist", "stack": [], "tone": "coral",
  "on": { "hover": "lift", "press": "inset" } }   // state behavior ref
```
Composites stay in `frame-patches.json`; semantics are the new binding layer that
*names* combinations of them.

---

## 3. Recipes — interaction as data

If an element or template **reacts**, that behavior is a **recipe**: a named,
reusable interaction pattern — never hand-coded per template again
(`reveal_list`, `checklist`, `drag_match` currently each reinvent tap logic).
"Recipe" is deliberate: it's the term of art (Panda/Vanilla Extract) *and* it
reads right for "a repeatable procedure for a behavior."

**A recipe = trigger → transition → rest**, all pure functions of a clock and
a small state — honoring the presenter law (every state is a rest state;
nothing auto-advances; motion is a clock function that settles once).

**Recipe kinds (starter set):**
- `advance` — a tap reveals/steps the next item (reveal_list).
- `toggle` — a tap flips one item on/off (checklist).
- `flip` — a tap swaps front↔back of a face (before/after).
- `match` — drag-and-drop pairing with lock/shake feedback (drag_match).
- `poll` / `quiz` — commit-once selection with a scored rest (choice_poll, quiz).
- `peek` — hover/press to momentarily reveal, releases back to rest.

**Contract.**
```
trait Recipe {
    fn state(&self) -> &RecipeState;              // the small serializable model
    fn on(&mut self, ev: Trigger, now: f64);      // press/hover/appear/value
    fn progress(&self, now: f64) -> f32;           // eased, clamped, resting
    fn restart(&mut self);                         // replay from zero
}
```
- **Declarative attach.** A template names its recipe by id + params
  (`{"recipe":"advance","items":4}`), the way a slide names a `template`.
- **Motion via the pool.** A recipe's feedback rides a generated `Effect`
  (`REVEAL_POP`, `REWARD_PULSE`, `DRAG_SHAKE`) — never a bespoke tween.
- **Testable.** The state machine is pure and unit-tested (advance clamps,
  restart clears) independent of paint — the pattern the current interactive
  templates already prove per-template, lifted to one place.

This also wires straight into the coach/agent seam (C-series): a recipe's
triggers are exactly the `press`/`hover`/`appeared`/`value-changed` edges the
touchpoint router already speaks.

---

## 4. Templates — composition, not paint

A **template** becomes a **layout of placed elements**, each carrying a
treatment ref and an optional recipe ref — authored data, resolved and painted
by one shared renderer (mirroring `slides::document::render`, which already
does this for `Text`/`Media` blocks).

```
Template { category, layout, slots: [ Slot { element, treatment, recipe?, props } ] }
```
- `category ∈ {static, animation, interactive, split, infographics}` — the
  slide's KIND, orthogonal to treatment (doctrine.md §"categories are not
  styling"). `interactive` ⇔ has a recipe.
- The renderer is the ONE path: measure → treat → paint, per element, under the
  fidelity budget. No template hand-paints.
- **Migration.** The W-series hand-painted templates are re-expressed as
  slot-lists incrementally; each migration deletes bespoke paint. Until a
  template migrates it stays as-is (working), just outside the new law.

---

## 5. Fidelity — the performance budget (first-class, day one)

**Hard constraints (operator-ratified 2026-07-27):**
- **2D only. Never 3D.** No 3D transforms, no perspective, no mesh depth.
- **The i5 baseline runs the whole app** — an old 4-core i5 with integrated
  graphics paints every surface at a smooth frame. That is Tier 0 and it is the
  *default*, not a degraded mode.
- **Better hardware only adds.** A capable GPU turns on richer layers — it never
  changes layout or meaning, only fidelity. "It screams and is better and
  better" = strictly additive enhancement.

**Tiers.**
| Tier | Target | What paints |
|---|---|---|
| **0 — Baseline** | old i5 / iGPU | flat fills, HARD offset shadows (blur 0), ONE painted back-glow, gradient tints, single-clock motion. The doctrine "no blur, depth from hard shadows + back-glow" law IS Tier 0. |
| **1 — Enhanced** | modern dGPU | extra back-glow layers, cyber/retro animated washes (hue-drift, scanline), more scatter/particles, per-item stagger depth. |
| **2 — Max** | high-end | the above at higher density + longer motion, ambient loops that never rest-block. |

**Mechanics.**
- **Every treatment and recipe declares a `cost_tier` and a cheap-path
  fallback.** An enhanced back-glow stack degrades to the single Tier-0 glow;
  an animated wash degrades to a static tint. Degrade is authored, not
  emergent.
- **One capability probe at boot** (extend the `sysinfo`/`Caps` precedent from
  `dpl-engine-ae`) picks the active tier: core count, RAM, GPU/adapter name,
  and honor `prefers-reduced-motion`-style overrides. `--fidelity <0|1|2>`
  forces it for proofs.
- **The no-blur law (doctrine #6) is permanent and tier-independent** — blur is
  never the answer at any tier; higher tiers add *more cheap shapes*, not
  feathering.
- **Budget, not guesswork.** A tier has a per-frame shape/mesh budget; the
  compositor caches `FrameStyle` by (tone, base, stack, state, tier) so
  resolution is free after first paint.

---

## 6. Decisions (ratified 2026-07-27)

1. **The word "element" — SETTLED.** UI primitive = **element** (this doc's
   layer 1). The composer's imported pack files (mogrt/aep/… in
   `content/elements/`) rename to **pack assets** in copy gradually — no code
   churn now, flagged so the Library never shelves two "elements."
2. **Where treatments+recipes live as data — SETTLED (proposal stands).**
   `tokens/treatments.json` (semantic bindings) + a `recipes` registry in
   `dpl-slides` (code-authored state machines, params as data), both
   generated/registered like `frame-patches.json`.
3. **Migration appetite — SETTLED: BIG-BANG.** Once the system stands
   (elements → treatments → recipes → fidelity + the composed renderer), the
   catalog is re-authored onto it in ONE concentrated consistency pass (§7.6),
   not opportunistically. The reward is a uniformly composable catalog; the
   cost is one large, well-gated migration PR.

---

## 7. Build order (spec → system → migrate)

1. **This spec, ratified.** ← we are here.
2. **Element library** — reorganize `blocks/` into categories, add the census +
   render gate + `cost_tier` field; make each standalone-usable in the Library.
3. **Treatment layer** — `tokens/treatments.json` semantic bindings over the
   existing composite `frame-patches.json`; the `treated`/`compose` paint API already
   exists. Expand ColdLight bases/stacks here (still clay/brutalist default).
4. **Recipe layer** — the `Recipe` trait + registry; lift `advance`/`toggle`/
   `flip`/`match`/`poll` out of the interactive templates.
5. **Fidelity probe** — the `Caps→Tier` boot probe + `--fidelity` + the
   `cost_tier`/fallback fields honored by the paint pipeline.
6. **Composed-template renderer + BIG-BANG migration** — the slot-list template
   model + the ONE renderer; then the one concentrated pass that re-authors the
   whole hand-painted catalog onto it, deleting every bespoke `ui()` (§6.3).

Each step is its own plan doc + PR + `cargo green`, gated on this spec.
Steps 2–5 land the system alongside the working (still hand-painted) catalog;
step 6 flips the catalog over in one pass.
