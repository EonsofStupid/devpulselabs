# ColdLight naming, and the pattern library law

Two operator corrections from 2026-08-03. The second is a law. The first was
written as a rename program and is **withdrawn** — the matrix already solves
it, better. This document keeps the parts that survive and records why.

> "we are suppose to be having a generic version of these patterns captured
> for the slide library so we properly build up a true library of components
> and patterns that do not use fake terms like traits if not used in egui"

## 1. WITHDRAWN — the `trait` → `FramePatch` rename

### 1.1 What was wrong

The proposal renamed `traits.json` → `frame-patches.json`, `TraitDef` →
`FramePatch`, and the nine axes onto their `egui::Frame` destinations, because
`trait` is a Rust keyword and not an egui word.

The premise was right. **The conclusion was wrong, because the answer already
existed on `main` and nobody looked.**

`docs/design/coldlight-matrix.md` (2026-07-31) and
`crates/coldlight/tokens/matrix.json` land **THE MATRIX** — a faceted
classification after Ranganathan's Colon Classification, generated and
build-validated. Traits are not being renamed; they are being **replaced** by
four facets:

| Ranganathan | facet | owns |
|---|---|---|
| Personality | **ANCESTOR** | `font` + the default coordinate for everything below |
| Space | **STRUCTURE** | `radius`, `edge.width` |
| Matter | **MATERIAL** | `fill`, `ink`, `edge.color` |
| Energy | **ATMOSPHERE** | `shadows`, `back_glow`, `tints`, `anim` |
| Time | **STATE** | the state overlay — already exists |

Ratified and generated on `main`:

- **8 ancestors** — `bauhaus · art-deco · brutalist · swiss · aero · memphis ·
  constructivist · editorial`
- **6 structures** · **9 materials** · **6 atmospheres**
- ~22 authored values → ~900 valid coordinates, with legality as DATA: every
  value is `native`/`allowed`/`forbidden` for every ancestor, and an
  unclassified value **fails the build**.

### 1.2 Two claims that were false

Both were stated as findings. Both are corrected here rather than quietly
edited away.

1. **"The matrix has four traits and no seed lineages."** False. Eight
   ancestors were ratified 2026-07-31 — before the operator described five on
   2026-08-02, and main already carried three more.
2. **"The five ancestrals are not in it yet."** False. Five of them are, plus
   `memphis`, `constructivist` and `editorial`.

### 1.3 Why the rename must not land

Its destination contradicts the matrix on the point the matrix cares most
about. **Matrix law M4:**

> "`edge` is SPLIT: width is geometry (STRUCTURE), colour is substance
> (MATERIAL). Without this an atmosphere has to reach outside its lane to
> recolour an edge, which is the exact defect the matrix exists to close."

The rename collapsed `edge` into a single `stroke` axis — the opposite move.
Landing it would force the matrix to undo a rename to do its own job.

**A rename on the way to a replacement is a migration nobody asked for.**

### 1.4 What survives, as amendments to the matrix

Three findings hold, and are handed to whoever finishes the matrix migration:

1. **The STATE facet should take egui's own slot names.**
   `egui::style::Widgets` has exactly `noninteractive · inactive · hovered ·
   active · open`. `Idle` is not an egui word, and `open` is a slot ColdLight
   lacks. Free to get right while that code is being touched anyway.
2. **Three axes have no egui equivalent.** `back_glow`, `tints` and `anim`
   appear nowhere in `egui::Frame`, `Visuals` or `WidgetVisuals`. ATMOSPHERE
   owns all three, so that facet is *by definition* the non-egui one — worth
   saying in `matrix.json` so a reader does not hunt for the field it maps to.
3. **The composed output is already named right.** `compose()` returns
   `ResolvedStyle` and it lands on `egui::Frame` — `.fill`, `.stroke`,
   `.corner_radius` in `crates/coldlight/src/lib.rs`. Whatever the input
   becomes, the output name should not change.

### 1.5 The lesson, recorded

`compose(tone, base, stack, state)` takes an **untyped `&[&str]`**, and matrix
law M1 exists precisely because of it: nothing stops
`compose(tone, "neubrutalist", &["claymorphic"], …)` — two contradictory
bases, silently last-wins. A pass was spent renaming the thing instead of
reading the doctrine that had already diagnosed it.

**Read `docs/design/` before proposing a change to the design system.** The
matrix doc opens with a section headed *"What already exists (ground truth, do
not rebuild)"*.

## 2. LAW — every pattern lands a generic catalog item

This one stands.

### 2.1 The rule

A UI pattern built for one surface is not done when that surface ships. It is
done when a **generic** version exists, so the library accrues real components
instead of a pile of one-offs.

This mirrors what `slide-catalog-pass` already enforces for ported items:

> "A renderer with no organized seed is only half-ported (this is the gap that
> made the Library feel empty)."

Same gap, other direction: a pattern invented for Chat and never generalized
leaves the Library just as empty.

### 2.2 Where a pattern lives

**`crates/coldlight/src/patterns.rs`** — the shared interaction primitives,
because every crate that paints already depends on ColdLight and several need
these. `crates/slides` registers thin demos into the element census so the
Library wall renders them: one implementation, one place it is shown from.

`crates/slides/src/elements/census.rs` already reserves
`ElementCategory::Control` and `Container` for exactly this.

### 2.3 What has needed generalizing so far

Three surfaces, three primitives — which is the signal this law was written
for.

| primitive | built for | now shared with | state |
|---|---|---|---|
| `bleed_control` | the shell status bar (V1) | the chat rail (C1) | **done** — one implementation |
| `group_chip` · `lift_tile` · `carousel_row` · `tile_grid` | the chat carousel (C1) | the Library wall | renderers **done**, census registration **owed** |
| `accordion` | the shell hub rail (L1a) | the chat provider rail (F1) | **owed** — still one implementation, in the shell |

### 2.4 Still owed by C1

- register the four patterns into the slides element census
- `motion.json`: the **selection burst** — the first terminal (non-looping)
  effect; the schema has to admit one
- `motion.json`: the **sporadic** multi-hue chip hover

## 3. Order

The matrix migration and the chat surface are independent lanes and stay that
way. Nothing in §2 waits on §1.
