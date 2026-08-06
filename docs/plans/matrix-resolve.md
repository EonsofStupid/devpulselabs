# Pass — the matrix resolves: facet payloads, `Coordinate`, and `resolve()`

## 1. Context

`docs/design/coldlight-matrix.md` defines the ColdLight MATRIX — a
Ranganathan faceted classification (ANCESTOR × STRUCTURE × MATERIAL ×
ATMOSPHERE, + runtime STATE) in which *"a THEME is a COORDINATE, not a
hand-authored bundle."* The operator ruled it the spine on 2026-08-04:

> "for now align to v3 visual then but technically both should be in the
> system in the matrix."

`docs/design/visual-language.md` (the consolidated source of truth)
records the consequence: the WiredFRONT editor's ratified look — v3 mock
`:401`, *"flat wells + liquid-glass/cyber chrome + glow spotlights"* — is
**already expressible** as the coordinate `aero · measured · neon-glass ·
neon`, with no new authored values. It cannot be *painted*, because the
matrix has no resolver and its style data never reaches the runtime.

**This pass builds the machinery.** It does not restyle any surface. The
editor's re-treatment is the pass after this one, so that a styling
change and a compositor change are never in the same diff — the mistake
that produced `2b0dacc`'s wrong base.

Ground truth for this plan was taken with a **SCIP index** built by
`rust-analyzer scip .` (19.8 MB, 3m42s) and queried for exact definition
and reference sets — not grep.

## 2. Current state (all read in-session 2026-08-04)

### 2.1 The compositor — `crates/coldlight/src/lib.rs`

- `ColorRef` (`:362`) — `resolve(self, tone: Tone) -> Color32` over
  `ColorBase::{ToneStep(step), Hue(id, step), Fixed(c)}` + `alpha`.
- `EdgeSpec { width: f32, color: ColorRef }` (`:380`) — **one unit**.
- `ShadowSpec` (`:387`), `Glow` (`:448`), `Tint` (`:455`).
- `Axes { fill, edge, radius, ink, font, shadows, back_glow, tints, anim }`
  — every field `Option`.
- `TraitState { Idle, Hover, Active }` (`:440`).
- `ResolvedStyle { fill, edge, radius, ink, font, shadows, back_glow,
  tints, anim }` (`:463`) — derives `Debug, Clone, Default`. **No
  `PartialEq`.**
- `merged(def, state)` (`:483`) — state patch REPLACES the axes it names.
- `compose(tone, base, stack: &[&str], state)` (`:508`) — iterates
  `once(base).chain(stack)`, applies override axes last-wins and additive
  axes by `extend`. Unknown id → `debug_assert!` + skip.

**The matrix's four facets map exactly onto `Axes`** (matrix §2), which is
why §7 can claim *"the compositor's maths does not change."*

### 2.2 The matrix — data validated, payloads discarded

`crates/coldlight/build.rs:130` `generate_matrix` reads `matrix.json` and
enforces: kebab ids; the ancestor's `font` exists in `fonts.json`;
`native.<facet>` is exactly one value; every facet value classified by
every ancestor (168 rulings).

**Then it emits only:**

```rust
pub static STRUCTURES:  &[&str];   // 6
pub static MATERIALS:   &[&str];   // 9
pub static ATMOSPHERES: &[&str];   // 6
pub static ANCESTORS:   &[&str];   // 8
pub const  RULING_COUNT: usize = 168;
```

Measured: `matrix_gen.rs` is **765 bytes**. Every authored payload in
`matrix.json` — `radius`, `edge_width`, `fill`, `ink`, `edge_color`,
`shadows`, `back_glow`, `tints`, `hover`, `active`, `native`, `allowed`,
`forbidden` — is **parsed for validation and thrown away.**

`lib.rs:1216` `pub mod matrix` includes that file and adds two census
tests. There is **no `Coordinate` type and no `resolve()`**; matrix §7
specifies both and neither exists.

### 2.3 The consumer surface — exact, from SCIP

| symbol | def | external references |
|---|---|---|
| `compose` | `lib.rs:508` | workbench/`author.rs` ×4, workbench/`controls.rs` ×2, workbench/`library.rs` ×1, viziwiz/`palette.rs` ×1 |
| `treated` | `lib.rs:609` | composer/`lib.rs` ×4 |
| `brutal` | `lib.rs:630` | composer/`lib.rs` ×4, composer/`browse.rs` ×1, workbench/`library.rs` ×1 |
| `clay` | `lib.rs:636` | composer/`lib.rs` ×9, `hub/lib.rs` ×1, viziwiz/`lib.rs` ×1 |
| `TraitState` | `lib.rs:440` | composer ×8, workbench/author ×7, workbench/controls ×6, viziwiz/palette ×3, workbench/library ×1 |

**~29 call sites across 8 files.** `card_frame` (`:878`, 12 refs) is a
legacy non-trait frame and is out of scope.

### 2.4 The authored facet data (`tokens/matrix.json`, read in full)

- **structures** carry `radius` + `edge_width`; **materials** carry
  `fill`/`ink`/`edge_color`; **atmospheres** carry
  `shadows`/`back_glow`/`tints` (+ `hover`/`active` on `hard-cast`).
- `glass` (`:32`) fill `surface OVERLAY`, ink `STRONG`, edge `tone 200`.
- `neon-glass` (`:36`) ink `cyan 100`, edge `cyan 400` — **declares no
  fill**, relying on law M2 (unnamed axes fall back to the ancestor's
  natives).
- `neon` (`:56`) two cyan back-glows (spread 14, 28) + a cyan-500 tint.
- Only **`aero`** allows `glass`/`neon-glass` (`:91-95`).

## 3. Target design

### 3.1 `generate_matrix` emits the payloads

Reusing the SAME spec types `traits_gen.rs` already emits (`ColorRef`,
`ShadowSpec`, `GlowSpec`, `TintSpec`), so resolution runs through
identical primitives:

```rust
pub struct StructureDef { pub id, label, blurb: &'static str,
                          pub radius: u8, pub edge_width: f32 }
pub struct MaterialDef  { pub id, label, blurb: &'static str,
                          pub fill: Option<ColorRef>, pub ink: Option<ColorRef>,
                          pub edge_color: Option<ColorRef> }
pub struct AtmosphereDef{ pub id, label, blurb: &'static str,
                          pub base: FacetAxes,
                          pub hover: Option<FacetAxes>,
                          pub active: Option<FacetAxes> }
pub struct AncestorDef  { pub id, label, blurb, font: &'static str,
                          pub native: Natives,
                          pub allowed: Sets, pub forbidden: Sets }
```

`STRUCTURES/MATERIALS/ATMOSPHERES/ANCESTORS` become `&[XDef]`. The `&str`
lists are kept as `*_IDS` so the existing census tests keep compiling.

### 3.2 `Coordinate` + `resolve()`

```rust
pub struct Coordinate {
    pub ancestor: &'static str, pub structure: &'static str,
    pub material: &'static str, pub atmosphere: &'static str,
    pub tone: Tone,
}
pub fn resolve(c: &Coordinate, state: TraitState) -> ResolvedStyle;
```

Resolution order, implementing laws M2 and M4:

1. Look up the ancestor. Unknown → `debug_assert!` + `ResolvedStyle::default()`
   (degrade, never panic — the `compose` precedent).
2. **M2 fallback:** for each facet, resolve the coordinate's chosen value;
   for any axis it does not declare, fall back to the value named by the
   ancestor's `native` for that facet. (`neon-glass` declares no `fill`,
   so on `aero` it inherits `clay`'s `surface RAISED`.)
3. `font` ← ancestor (M5 — never a free knob).
4. `radius` ← structure. **M4 edge split:** `edge = Stroke::new(
   structure.edge_width, material.edge_color.resolve(tone))` — assembled
   from two facets, which is why `EdgeSpec` cannot be carried whole.
5. `fill`, `ink` ← material.
6. `shadows`, `back_glow`, `tints`, `anim` ← atmosphere, **additive**
   (`extend`), matching `compose`.
7. State overlay: the atmosphere's `hover`/`active` patch REPLACES the
   axes it names, reusing `merged()`'s exact semantics.

Legality: authored coordinates (presets) are build-checked. A
runtime-constructed `Coordinate` gets `Coordinate::legality() -> Result<(),
String>` plus a `debug_assert!` — a forbidden combination still paints
(honest degrade) rather than panicking a shipped exe.

### 3.3 What migrates cleanly — and what does NOT

Matrix §8 claims all four legacy calls migrate byte-identically. **Read
against the real data, that is wrong in two specific ways**, and both are
recorded rather than discovered later.

**(a) The two BASES match on every axis except `font`.**

| | trait | coordinate | verdict |
|---|---|---|---|
| fill | `tone 400` | `raw`: `tone 400` | ✔ |
| edge | `2.0, tone 700` | `massive` 2.0 + `raw` `tone 700` | ✔ |
| radius | 6 | `massive` 6 | ✔ |
| ink | `surface WELL` | `raw`: `surface WELL` | ✔ |
| shadows / back_glow | `[3,3] blur0 black@230` / `tone300@60 sp18` | `hard-cast`: identical, incl. hover/active | ✔ |
| **font** | **`None`** (traits.json declares none) | **`Some("spacegrotesk")`** | **✘ differs** |

Same shape for `claymorphic` → `aero · rounded · clay · bloom`
(`font: None` → `Some("exo2")`).

**This is correct, not a bug:** M5 gives typography to the ancestor, so
the matrix legitimately ADDS a font the trait never carried. The test
asserts equality on all axes except `font`, and asserts `font` moves
`None → Some(_)`.

**(b) The STACKS cannot be byte-identical, and §8's table is
incomplete.** `cyber` declares `back_glow, edge(1.0, cyan 400), ink(cyan
100), radius 6, tints`; `retro` declares `edge(2.0, amber 700), ink(amber
200), radius 3, tints`. Under the matrix, `ink`/`edge.color` are
**MATERIAL** and `radius`/`edge.width` are **STRUCTURE** — so neither is
an atmosphere alone. `matrix.json` already authored the material halves:

| legacy | true coordinate mapping |
|---|---|
| `stack=["cyber"]` | `material: neon-glass` + `atmosphere: neon` |
| `stack=["retro"]` | `material: amber-crt` + `atmosphere: scanline` |

And the geometry cannot be preserved: cyber wants `(radius 6, edge_width
1.0)`; no authored structure is that pair (`massive` = 6/2.0, `measured`
= 4/1.0). A coordinate has exactly ONE structure, so a cyber-on-brutalist
surface takes `massive`'s 2.0 edge, not cyber's 1.0.

**Therefore:** stacks migrate as a **deliberate, named restyle**, not a
reclassification. The delta is ≤1px of edge width on the affected
surfaces and is listed in §8 evidence with proof shots. `matrix.md` §8 is
amended in the same commit.

## 4. Changes, in execution order

1. `crates/coldlight/build.rs` — extend `generate_matrix` to emit the §3.1
   payload structs. Keep every existing assertion; add: a material's
   `fill`/`ink`/`edge_color` refs resolve against the pool; an
   atmosphere's shadow/glow/tint refs likewise.
2. `crates/coldlight/src/lib.rs` `mod matrix` — declare the payload
   structs beside the include; keep `*_IDS`; update the two census tests
   to read `.len()` on the new slices.
3. Same file — `Coordinate`, `resolve()`, `Coordinate::legality()` per
   §3.2.
4. Same file — derive `PartialEq` on `ResolvedStyle`, `Glow`, `Tint`
   (egui's `Color32`, `Stroke`, `Shadow` are all `PartialEq`), so the
   migration test can compare whole structs rather than field-by-field.
5. Tests in `mod matrix`:
   - `the_two_bases_reclassify_identically` — §3.3(a), all axes but
     `font`, plus the `None → Some(_)` assertion, across all three
     `TraitState`s and ≥3 tones.
   - `unnamed_axes_fall_back_to_ancestor_natives` — M2: `neon-glass` on
     `aero` inherits `clay`'s fill.
   - `edge_is_assembled_from_two_facets` — M4: width from structure,
     colour from material.
   - `the_editor_coordinate_is_legal` — `aero · measured · neon-glass ·
     neon` passes `legality()`; a forbidden pairing (`swiss` + `glass`)
     fails it.
   - `an_unknown_facet_degrades_without_panicking`.
6. `docs/design/coldlight-matrix.md` — amend §8: the stack rows are
   material+atmosphere pairs; the byte-identical claim is scoped to the
   two bases minus `font`; the geometry delta is stated.
7. `docs/design/visual-language.md` — mark C2b resolved; record the
   editor coordinate as confirmed-legal.

**No surface changes in this pass.** `compose()` stays, every one of the
~29 call sites is untouched, and both paths coexist until the migration
pass.

## 5. Boundaries

**🚫 Never touch**
- `/content` (operator data).
- Any `*_gen.rs` by hand.
- `tokens/matrix.json`'s authored VALUES — this pass emits them, it does
  not retune them. Changing a colour here is a styling decision.
- `compose()`'s behaviour — it must keep resolving byte-identically for
  every existing caller, which the test suite pins.
- The 4 traits in `traits.json` — they remain the live path until the
  migration pass.

**⚠️ Ask first**
- Adding any facet value (a new material/atmosphere costs 8 rulings).
- Changing `ResolvedStyle`'s shape beyond deriving `PartialEq`.
- Retiring `compose()` — that is the migration pass's call, not this one.

**Out of scope**
- Re-treating the editor (the next pass).
- Migrating the ~29 call sites.
- Presets-as-coordinates (`presets.json` → `Coordinate`) — needs §6 of
  the matrix doc, its own pass.
- The editor-mock palette as a second coordinate (visual-language §7.8).
- `xtask fonts` / the ~120-face pool (matrix §10.4).

## 6. Verification

- `cargo test -p dpl-coldlight` — the five new tests plus the two
  existing census tests (8 ancestors × 21 values = 168 rulings) still
  green.
- **Break-it:** set a material's `edge_color` to `{"hue":"nosuchhue"}` →
  the build fails naming the material. Revert.
- **Break-it:** delete one `forbidden` entry from one ancestor → the
  existing coverage assertion fails (a value left unclassified). Revert.
- `cargo green` exits 0 (fmt, clippy, tests, structure audit — proves no
  new crate edge; `dpl-coldlight` has no §2 ceiling and gains no dep).
- **The regression that matters:** every existing surface still paints
  through `compose()` unchanged. Proof shots at `--route wiredfront/editor`
  and `--lib-section icons`, maximized and `--window 1100x700`, compared
  against `docs/proof/editor-icons-*.png` from `2b0dacc` — **pixel-identical
  is the pass condition**, since this pass adds machinery and changes no
  paint.
- Report `matrix_gen.rs`'s new byte size (765 B today) as evidence the
  payloads are actually emitted.

## 7. Commit plan

ONE PR, `branch → gh pr create → double-green → merge`, branch deleted
after. `docs/STATE.md` updated; `target\release\devpulselabs.exe` rebuilt
here with its mtime verified at session end.

`feat(coldlight): the matrix resolves — facet payloads, Coordinate, and resolve()`

Ordered commits inside it: generator payloads → `Coordinate`/`resolve()`
→ tests → doc amendments.

## 8. Evidence

*(filled during execution — real command output, deviations named
explicitly per the execution-discipline law)*
