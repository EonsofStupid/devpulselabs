# Pass — the ColdLight MATRIX, established

**Spec:** `docs/design/coldlight-matrix.md` §§1–8. This pass implements it;
the spec is not reopened.
**Ratified 2026-07-31:** eight seeds (§11.2) · `swiss` not `frutiger` (§11.1) ·
the type pool as a starting point (§11.3) · `aero` no-blur by recommendation
(§11.4) · and the three structural decisions — the axis partition,
preset-by-reference, legality-as-data.

## 1. Context

`refactor(theme)` (`d03a317`) renamed the noun. This gives it its meaning.

Today a look is `compose(tone, base: &str, stack: &[&str], state)`. The axis
maths is already right — override axes (`fill` `edge` `radius` `ink` `font`)
last-wins, additive axes (`shadows` `back_glow` `tints` `anim`) concatenate
(§1). The defect is the **untyped stack**: nothing prevents
`compose(tone, "neubrutalist", &["claymorphic"], …)` — two contradictory bases,
silently resolved last-wins. Four traits, one crude `role` field, no way to say
that `bauhaus + glass + bloom` is a contradiction rather than a choice.

The matrix replaces the list with **one value per facet, exactly**, and makes
illegality a build error.

## 2. Current state (ground-truthed, `d03a317`)

`traits.json` carries four traits with full axis payloads plus per-state
patches:

```jsonc
"neubrutalist": { "role": "base",
  "fill": {"tone": 400}, "edge": {"width": 2.0, "color": {"tone": 700}},
  "radius": 6, "ink": {"surface": "WELL"},
  "shadows": [{"offset": [3,3], "blur": 0, "color": {"black": true, "alpha": 230}}],
  "back_glow": [{"color": {"tone": 300, "alpha": 60}, "spread": 18.0}],
  "hover": { … }, "active": { … } }
```

Under the matrix these four are not four looks — they are **one ancestor, one
material, and two atmospheres wearing the same coat** (§1):
`neubrutalist` = Brutalist · `claymorphic` = a *material* · `cyber` = an
*atmosphere* · `retro` = an *atmosphere*.

`THEME_PRESETS` holds 5 named bundles; `compose()` has 4 callers workspace-wide.

## 3. Design

**The facet payloads are DERIVED, not invented.** Each existing trait's axes
split along facet ownership — that is what makes §8's byte-identity possible:

| trait axis | facet that owns it |
|---|---|
| `radius`, `edge` | STRUCTURE |
| `fill`, `ink` | MATERIAL |
| `shadows`, `back_glow`, `tints`, `anim` | ATMOSPHERE |
| `font` | ANCESTOR (§3 — typography is the most identity-carrying choice a movement makes, so it is never a free knob) |

So `massive` takes neubrutalist's `radius: 6` + `edge`; `raw` takes its `fill` +
`ink`; `hard-cast` takes its `shadows` + `back_glow`. Nothing is authored from
taste for the four migrated values — they are a re-filing of bytes already
ratified.

**The vocabulary** (§3, §4): 8 ancestors × 6 structures × 7 materials ×
6 atmospheres = **2,016 coordinates from 27 authored values.**

**Legality is data** (§5). Each ancestor declares `native` (one value per facet
— an ancestor alone is already a complete look), `allowed` (defensible
departures) and `forbidden` (contradictions). Every facet value must appear in
**exactly one** of the three for **every** ancestor: 8 × 19 = **152 rulings**.
A value left unclassified fails the build, which is what stops the matrix
rotting as values are added.

**The schema** (§7):

```rust
pub struct Coordinate { ancestor, structure, material, atmosphere, tone }
pub fn resolve(c: &Coordinate, state: TraitState) -> ResolvedStyle;
```

Resolution order: ancestor's natives first, then the three facet values against
the existing per-axis rules, then the state overlay. **The compositor's maths
does not change** — only what is allowed to enter it.

## 4. Scope

**In:** `tokens/matrix.json` · the 152 rulings · `build.rs` generation and its
four checks (total-and-disjoint partition · every value classified by every
ancestor · no forbidden reference · kebab ids) · `Coordinate` · `resolve()` ·
the §8 byte-identity test · the three new seeds' instincts (`memphis`,
`constructivist`, `editorial` — ratified in §11.2 but absent from §3's table).

**Out, and named:**
- **Deleting `compose()`.** It keeps working this pass; the four callers move
  in their own pass once `resolve()` is proven. A rename that also moves every
  caller cannot be bisected.
- **The type pool / `xtask fonts`** (§10). Unblocked, but its own pass.
- **Presets as coordinates** (§6). `THEME_PRESETS` keeps its current shape
  until `resolve()` exists to point at.
- **Non-egui targets** (§9). The generator emits Rust here; other targets are
  a later consumer of the same `matrix.json`.
- **`aero` gloss painting.** §11.4's no-blur interpretation is authored as
  data; whether banded gradients read as gloss is a paint pass.

## 5. Work plan (atomic, in order)

1. `tokens/matrix.json` — the 27 values with derived payloads, the 8 ancestors,
   the 152 rulings. No code yet; a schema test proves the file is
   total-and-disjoint.
2. `build.rs` — generate the four id types + the legality table + the four
   build checks. A deliberately illegal fixture proves each check fires.
3. `Coordinate` + `resolve()` beside `compose()`, sharing its axis maths.
4. The §8 byte-identity test: all four legacy calls vs their coordinates.
5. `cargo green`; §8 evidence; STATE.md.

## 6. Risks

- **152 rulings authored by me, not the operator.** Mitigation: §11's working
  rule — a vocabulary is approved as a starting point and revised in flight.
  The rulings are data in one file; changing one is an edit, not a refactor.
- **Byte-identity may fail on float formatting** rather than on real drift.
  Mitigation: compare the `ResolvedStyle` struct, not a serialisation.
- **The three new seeds have no authored instincts** — §3's table still has
  five rows. Mitigation: authored from §3.2's characterisations and flagged in
  §8 evidence as the part most wanting a red pen.
- **2,016 coordinates is a lot of surface for a UI to expose.** Mitigation: not
  this pass's problem — §6 says a preset pins a coordinate and exposes 4–7 free
  axes, which is the whole point of the collapse.
- **`aero` vs the no-blur law** (§3.1). Mitigation: authored as banded
  gradients + hard specular bands per §11.4; recorded so a correction is cheap.

## 7. Verification / acceptance

- `cargo green` clean, both lanes.
- A test that every facet value is classified by every ancestor, exactly once —
  the census law applied to the matrix.
- A build-fixture test per check: unclassified value fails; forbidden reference
  fails; non-kebab id fails; overlapping partition fails.
- **The §8 test — all four legacy `compose()` calls produce a `ResolvedStyle`
  byte-identical to their coordinate's `resolve()`.** This is the acceptance
  criterion that matters: it proves reclassification, not restyle.
- An ancestor alone (natives only) resolves to a complete, non-empty style.
- No proof shots claimed as regression evidence — they are not
  pixel-comparable in this repo (`effects-motion-axes.md` §7).

## 8. Evidence

_(filled on completion)_
