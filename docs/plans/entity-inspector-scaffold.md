# Scaffold — click any entity, adjust anything

**The operator's contract, verbatim:** *"UI user clicks on any entity and at any
given time we at least get the ability to adjust size, color, claymorphism,
neubrutalist, liquid glass, and then adjust with sliders or even presets offered
inside that. We have traits that combine and can add motion and then
interactions are … recipes."*

**Spec:** `docs/design/design-system.md` §2b (semantic treatments), §3 (recipes),
§1 (elements). **Program:** `docs/plans/poam-preset-system.md`.

## 1. Why this is a scaffold and not a feature

"Any entity, at any time" is a hard contract. It forces every paintable thing to
be **addressable**, **self-describing**, and **mutable through one path**. Meet
those three and the inspector panel is *generated*; miss any one and every
surface grows its own bespoke property panel — which is the same failure that
produced six competing control implementations.

The panel is therefore not the deliverable. The seven APIs under it are.

## 2. Current state (ground-truthed, `53b321f`)

**Storage — partially there.** `Block::Text { id, text, font: Option<String>,
size: String, ink: Option<String> }` over `Look { font, ink, background, enter }`
is a real two-level cascade for **font · size · ink**. A block cannot hold a
base treatment, a trait stack, a tone, a motion, or a recipe.

**Resolution — there, one level too low.** `compose(tone, base, &stack, state)
-> ResolvedStyle` works and is good. Four files in 68 crates call it.

**Addressing — two half-systems.** Blocks carry `id: String`. `egui-pilot-ai`
carries `NodeId` / `SemanticNode` / `Role` for the agent seam. Neither knows
about the other.

**Mutation — exists per-surface.** `dpl-doc` has `validate`, `SCHEMA_V = 2` and
newer-refusal; the composer has snapshot undo/redo and debounced autosave. There
is no single `set_axis`.

**The offer — absent.** No manifest. 8 presets (5 look, 3 motion), 4 traits,
53 hues, 18 effects, 6 elements, 28 templates — and nothing enumerates what a
given entity kind may wear.

## 3. The seven APIs

### 3.1 `EntityRef` — addressing (`dpl-doc`)
```rust
pub struct EntityRef { slide: SlideId, block: String, part: Option<String> }
```
Blocks already have ids; parts inside them do not. **Reuse `egui-pilot-ai`'s
`NodeId`** for hit-test rather than minting a second id space — if the operator
and the agent point at different things, the seam mirror in §3.7 is a lie.

### 3.2 `Dress` — what an entity wears (`dpl-doc`, **schema v2 → v3**)
```rust
pub struct Dress {                        // every field optional = inherit
    base:   Option<String>,               // neubrutalist | claymorphic | liquid-glass
    stack:  Vec<String>,                  // ["cyber", "retro"]
    tone:   Option<String>,               // one of 53 hues
    axes:   BTreeMap<String, AxisValue>,  // the slider values
    effect: Option<EffectRef>,            // motion
    recipe: Option<RecipeRef>,            // interaction
}
```
One optional field on every block variant. `skip_serializing_if` keeps every
existing document loading unchanged; the newer-version refusal is already
written and tested.

### 3.3 `axes_for` — what CAN be adjusted (`dpl-coldlight`)
```rust
pub struct AxisSpec { id, label, kind: AxisKind, applies_to: &'static [EntityKind] }
pub enum AxisKind {
    Scalar { min: f32, max: f32, step: f32, unit: Unit },
    Tone, Base, TraitStack, Preset(PresetKind), Effect, Recipe, Toggle,
}
pub fn axes_for(kind: EntityKind) -> &'static [AxisSpec];
```
**The load-bearing API.** The inspector is built by walking this, never by
hand-writing a panel per entity. Add an axis once and every inspector grows it.

This is also the operator's "arktype for Rust": `serde` (have) + `jsonschema`
0.49 (have, already a workspace dep) + **`schemars`** (missing) to derive and
emit the schema, so an agent reads the identical contract the panel is built
from.

### 3.4 `dress()` — resolution (`dpl-coldlight`)
```rust
pub fn dress(look: &Look, d: &Dress, state: TraitState) -> ResolvedStyle
```
Cascade: look → base + stack + tone → per-axis overrides → state. `compose()`
one level up; nothing already working is discarded.

### 3.5 `set_axis` — mutation (`dpl-doc`)
```rust
pub fn set_axis(slide: &mut Slide, e: &EntityRef, axis: &str, v: AxisValue)
    -> Result<(), Refusal>
```
One path. Validated against §3.6 — *you cannot seed a thing with a look that
does not exist*. Undo checkpoints ride the existing snapshot mechanism.

### 3.6 `offers` — the manifest (generated)
```rust
pub fn offers(kind: EntityKind, axis: &str) -> &'static [Offer]
```
Every dropdown, chip and preset row comes from here. A census test fails on
drift. Nothing is hand-listed anywhere in the UI.

### 3.7 The seam mirror (`dpl-seam`)
Exposes §3.3, §3.5 and §3.6 verbatim. An agent adjusts an entity through the
operator's own API — one contract, and AI capability is a consequence of the
design rather than a parallel implementation.

## 4. Scope

**In:** the seven APIs, the schema v3 bump, `schemars`, the generated manifest,
and one inspector panel built entirely from `axes_for`.

**Out, and named:**
- **Multi-event slides.** `reveals` is ONE linear axis (`step_count = reveals+1`).
  A touchscreen prop needs concurrent per-region events. That is a `dpl-doc`
  document-model change and its own plan.
- **Runtime SVG.** `dpl-svg` is a BUILD-dep restricted to the icon subset
  (ADR-015: "runtime never parses SVG"). Live interactive vector charts from
  AE-authored art need a runtime path with fills, gradients and transforms.
  `dpl-vector` already paints charts as meshes and is the right seam. Own plan.
- **The pulse catalog / lesson generation.** The coach capture half exists
  (`coach-overlay`, `coach-inventory`, `desktop-spike`, `coach_signals.rs`).
  Turning observation into an authored lesson is the C-series, not this.
- **The BIG-BANG catalog migration** (design-system §6.3).

## 5. Build order

1. **Effects get motion axes** (`docs/plans/effects-motion-axes.md`) — the pool.
2. **Presets + manifest** — the offer (`AxisKind::Preset`, §3.6).
3. **`Dress` + schema v3** — the storage.
4. **`axes_for` + `dress()` + `schemars`** — the contract.
5. **The inspector panel** — generated, one surface.
6. **The seam mirror** — AI parity.

Effects lead because every tier above composes them, and because the effect pool
is the one ingredient the operator has an explicit target for (~300).

## 6. Risks

- **A second id space.** If hit-test mints ids that `egui-pilot-ai` doesn't
  know, §3.7 silently diverges. Mitigation: `NodeId` is the id; `EntityRef` maps
  to it, and a test asserts round-trip.
- **Schema v3 on real operator documents.** `/content` is operator data and is
  never mutated by us. Mitigation: `Dress` is entirely optional and skipped when
  empty, so a v2 document is a valid v3 document; a test loads a v2 fixture
  unchanged.
- **`axes_for` becoming a god-enum.** Mitigation: `applies_to` filters per kind,
  and the census test fails if an axis claims a kind that cannot honour it.
- **Inspector scope creep into a full editor.** Mitigation: this pass ships ONE
  panel driven by `axes_for`; layout, z-order and grouping are later.
- **Doing 3.1–3.7 as one PR.** They are one scaffold but six passes. Mitigation:
  the build order above, each its own plan + PR + double-green.

## 7. Verification / acceptance

- `cargo green` clean on both lanes at every step.
- A v2 `slide.json` fixture loads under v3 byte-identically on round-trip.
- `axes_for(EntityKind::Text)` drives a panel with no hand-written widget list;
  a test asserts the panel's control count equals `axes_for().len()`.
- `set_axis` with an unlisted preset returns `Refusal`, and a test proves the
  document is unchanged after the refusal.
- `EntityRef` ⇄ `NodeId` round-trips.
- The seam exposes the same `offers()` the panel used, asserted equal.
- Proof shots at 1280×800 and 1100×700 of the inspector over a real card.

## 8. Evidence

_(filled on completion)_
