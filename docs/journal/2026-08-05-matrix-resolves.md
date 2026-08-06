# 2026-08-05 — the matrix resolves, and the editor wears a coordinate

## The ask

> "so are you actually ready to show me this working does it truly have
> potential to absorb my presets from rtfx and stuff and get it actually
> working in here as presets"

Answer at the time: **no.** The number that settled it — runtime call sites
reading the generated matrix: **0**. The 25 preset coordinates rendered in a
browser; the app painted with `compose(trait)`.

Operator then chose: **ColdLight → the app** first.

## Done — adoption is no longer zero

### 1. The matrix emits payloads (`8f9d98e`)

`generate_matrix` parsed every authored value for validation and **threw it
away**: `matrix_gen.rs` was **765 bytes** of ids. The matrix could be checked
but never painted. It is now **14,078 bytes** of real payloads.

- `build.rs`: the `ColorRef`/`Axes` emitters are hoisted out of
  `generate_traits` into free functions, so facets and traits are emitted
  through the SAME validated primitives and cannot drift.
- `StructureDef` / `MaterialDef` / `AtmosphereDef` / `AncestorDef`; the id
  pools survive as `*_IDS`.
- `pill`'s authored radius 999 clamps to 255 on the way out — egui's
  `CornerRadius` is `u8` and already treats 255 as fully rounded.
- `Coordinate`, `resolve()`, `Coordinate::legality()`. State-overlay
  semantics are shared with the trait compositor through `merged_axes`, so
  hover/active behave identically on both paths.

### 2. The ruled coordinate did NOT reproduce Vibron — fixed, not papered over

`brutalist · massive · plate · hard-cast` diverged from the `vibron` trait in
two ways that both mattered:

| axis | vibron trait | the coordinate (before) |
|---|---|---|
| `back_glow` | none | **added** a `tone300@60 spread18` halo |
| `active` | fill wash + edge relight + ink STRONG | **only** sank the shadow |

A halo on every quiet plate is the opposite of Vibron's restraint, and losing
the active state loses the "exactly ONE control reads as live" rule.

**The halo is what LOUD looks like, so it does not belong on the shadow.** It
split off as a new atmosphere **`lit-cast`** (cast + pop — for `neo`,
`werkstatt`, `bacterio`), leaving `hard-cast` as the cast alone. `hard-cast`
then took on the interaction convention the vibron trait had proved out:
hover lifts the cast and strengthens the ink; **active** washes the fill in
the surface tone, lights the edge to it, and sinks onto the shadow.

Census: 22 → **23** facet values.

### 3. Roles bind to the matrix (`8918b14`)

- `Dress` is what paints a role: `Trait(id)` or `Coord(Coordinate)`. Both
  resolve to the same `ResolvedStyle`, so `Treated::show` no longer knows
  which path it is on.
- `roles.json` accepts `"coordinate": {…}` beside `"trait"`. The generator
  validates it against the real facet pools AND the ancestor's own rulings —
  **an illegal preset fails the build**, exactly as an unknown trait does.
- `wiredfront.chrome` → `brutalist · massive · plate · hard-cast`.
- `Binding::style()` replaced callers reaching for `.treatment` as a string.

**Not one composer call site changed.** They already asked for an `Intent`;
that is what the role indirection was for.

## Proof

- `vibron_the_coordinate_paints_vibron_the_trait` — byte-identical
  `ResolvedStyle` across **3 tones × 3 states**, with `font` the one
  legitimate difference (law M5 gives typography to the lineage:
  `None → Some("spacegrotesk")`).
- Plus: M2 native fallback, M4 two-facet edge, forbidden-reports-but-paints,
  unknown-facet degrade, every ancestor painting from its own natives.
- 55 coldlight · 62 composer · 55 workbench · 7 viziwiz green; clippy clean.
- **On the real exe:** rebuilt `target/release/devpulselabs.exe` (mtime
  2026-08-05 17:16), `--route wiredfront/editor --window 1400x900`. The
  editor is visually unchanged — which IS the result. The release build's
  `roles_gen.rs` contains `Dress::Coord`, so the shipped binary genuinely
  resolves through the matrix.

## NOT done — be honest about this

- **Runtime switching between the 25 presets.** The binding is data, but
  there is no picker and no `presets.json` registry of the coordinates yet.
  Changing the editor's look today means editing `roles.json` and
  rebuilding. "Switchable at runtime" is the next pass, not this one.
- `compose()` and the 5 traits are still live for workbench/viziwiz chrome.
  Both paths coexist deliberately.
- **RTFX/AE presets are a different axis and were not touched.**
  `crates/composer/src/rtfx.rs` already ingests and tags the 440-element
  pack (prerendered `.mov`); `.ffx`/`.aep`/`.mogrt` are FILED as `element`
  kind but nothing parses them, and `engine-ae` (Phase 1 only) is not wired
  to the app.

## Exact next step

1. `presets.json` gains an `ancestral` kind carrying the 25 coordinates,
   generated + build-checked for legality.
2. A runtime `set_chrome_coord()` + a picker, so the operator can flip the
   editor through all 25 without a rebuild.
3. Then the `.aep` pass for the RTFX side.

## Landmines

- `treated()` now `debug_assert!`s on a non-empty trait STACK — stacks have
  no coordinate form (matrix §8: cyber/retro are material+atmosphere pairs,
  and their geometry cannot survive a single structure).
- `hard-cast` no longer carries a halo. Anything that relied on it must move
  to `lit-cast`.
- The `matrix` module imports `ink`/`surface`/`ColorBase`/`EdgeSpec` etc.
  because the GENERATED payloads are written in those primitives — adding an
  authored axis may require another import there.

## Verify

```
cargo test -p dpl-coldlight                       # 55, incl. the equality proof
cargo build --release -p devpulselabs -p dpl-lottie-render
target\release\devpulselabs.exe --route wiredfront/editor --window 1400x900 --screenshot out.png
```
