# ColdLight naming, and the pattern library law (S1)

**Rank:** S1 · **Parents:** [`docs/design/coldlight-design-os.md`](../design/coldlight-design-os.md) (OS),
[`docs/design/theme-vocabulary-law.md`](../design/theme-vocabulary-law.md) (S0 Theme face).
Product face is **Theme / Themes / ThemePreset** (S0). **Looks is forbidden.**
This file owns engineering rename (trait → FramePatch) and pattern→catalog duty.

Two operator corrections from 2026-08-03, ground-truthed against egui 0.35
and the code as it stands. Both are LAWS, not preferences.

> "we are suppose to be having a generic version of these patterns captured
> for the slide library so we properly build up a true library of components
> and patterns that do not use fake terms like traits if not used in egui"

## 1. LAW — no invented term where egui has one

### 1.1 The finding

`trait` is not an egui word. It is a Rust **keyword**, so `traits.json` →
`traits_gen.rs` → `TraitDef` collides with the language's most loaded term
inside a file that is nothing to do with Rust traits.

The system already knows better: `compose()` returns **`FrameStyle`**
(`crates/coldlight/src/lib.rs:463`). Only the input is misnamed.

### 1.2 egui has NO term for a stackable partial Frame

This has to be said plainly, because the fix is not "find the egui word."
egui 0.35 ships:

| egui type | what it is |
|---|---|
| `Style` | the whole configuration |
| `Visuals` | colours and shadows |
| `style::Widgets` | five state slots: `noninteractive` `inactive` `hovered` `active` `open` |
| `style::WidgetVisuals` | `bg_fill` `weak_bg_fill` `bg_stroke` `corner_radius` `fg_stroke` `expansion` |
| `Frame` | the painted container: `fill` `stroke` `corner_radius` `shadow` `inner_margin` `outer_margin` |
| `override_text_style` · `override_font_id` · `override_text_color` | egui's own idiom for PARTIAL application |

A stackable partial Frame that STACKS is a ColdLight invention on top of these.
Adopting an "egui word" for it would be inventing one again.

**The rule an egui developer follows instead: name it for the egui type it
produces.** ColdLight produces an `egui::Frame` —
`crates/coldlight/src/lib.rs:659`:

```rust
.fill(style.fill)
.stroke(style.edge)
.corner_radius(CornerRadius::same(style.radius))
```

### 1.3 The rename

| today | becomes | grounding |
|---|---|---|
| `tokens/traits.json` | `tokens/frame-patches.json` | it authors partial `Frame` values |
| `TraitDef` | `FramePatch` | all-`Option` partial; `patch` is the Rust word for it |
| `ResolvedStyle` | `FrameStyle` | the complete set handed to `egui::Frame` |
| `TraitState` | `WidgetState` | egui's `style::Widgets` axis |
| `Idle` / `Hover` / `Active` | `Inactive` / `Hovered` / `Active` / **`Open`** | egui's exact slot names; `Open` is gained |
| `trait_def(id)` | `frame_patch(id)` | |
| `TRAITS` | `FRAME_PATCHES` | |
| `traits_gen.rs` | `frame_patches_gen.rs` | |
| `role: base \| stack` | `role: base \| overlay` | a stack is the whole; a member is an overlay |
| a NAMED stack | **`recipe`** | already the word in the slide-seed contract — kept, not renamed |

`compose(tone, base, overlays, state) -> FrameStyle` keeps its name. Compose
is the correct verb and needs no change.

### 1.4 The axes — the operator's rule, applied to the right struct

The rule is **an axis name IS the destination field**. The destination is
`egui::Frame`, not `WidgetVisuals`.

| today | becomes | egui destination |
|---|---|---|
| `fill` | `fill` — **unchanged** | `Frame::fill` |
| `edge` | **`stroke`** | `Frame::stroke` |
| `radius` | **`corner_radius`** | `Frame::corner_radius` |
| `shadows` | `shadows` — plural kept | `Frame::shadow` is SINGULAR; stacking is an extension |
| `ink` | **`override_text_color`** | `Visuals::override_text_color` |
| `font` | **`override_font_id`** | `Style::override_font_id` |
| `back_glow` | unchanged — **EXTENSION** | no egui equivalent |
| `tints` | unchanged — **EXTENSION** | no egui equivalent |
| `anim` | unchanged — **EXTENSION** | no egui equivalent |

Three of nine axes have no egui home. The token file must SAY so — an
`"$extension": true` marker per axis — rather than let a reader assume every
axis maps somewhere. That is where ColdLight is genuinely its own system, and
hiding it is how the vocabulary rotted the first time.

### 1.5 The five seed lineages are not patches

**Bauhaus · Art Deco · Swiss · Aero · Brutalist** are ancestry, not
composable layers. They are a `lineage: [...]` provenance field ON a patch,
never entries in the patch list. `neubrutalist` is Brutalist edge over
Bauhaus geometry; `claymorphic` is Aero depth at Bauhaus radius.

### 1.6 Scope

82 sites carry the term across `crates/coldlight`, `crates/slides`,
`crates/composer`, `crates/doc` and the token files. Pre-release law: this is
a rename, executed in one pass, with **no alias and no deprecation shim**.

## 2. LAW — every pattern lands a generic catalog item

### 2.1 The rule

A UI pattern built for one surface is not done when that surface ships. It is
done when a **generic** version exists in the slide catalog, so the Library
accrues a real component and pattern library instead of a pile of one-offs.

This mirrors the rule `slide-catalog-pass` already enforces for ported items:

> "A renderer with no organized seed is only half-ported (this is the gap that
> made the Library feel empty)."

Same gap, other direction: a pattern invented for Chat and never generalized
leaves the Library just as empty.

### 2.2 The two deliverables, per pattern

1. **The generic renderer** — in `crates/slides/src/{elements,blocks,templates}`,
   painting only through `compose(...)` and coldlight paint fns. No ad-hoc hex,
   no bespoke frame helper, no reference to Chat.
2. **The organized seed** — `seeds/slides/<category>/<name>/slide.json`,
   carrying `template`, `category`, stackable `tags`, a `recipe` (the named
   patch stack), and authored content blocks. Registered, so it appears on the
   Workbench Library wall, filterable by category × tags.

Categories in force: `static · animation · interactive · split` (+
`infographics` per the catalog checklist). Today `seeds/slides/` holds four of
those; `infographics/` does not exist yet.

### 2.3 What the providers work owes the library

Each of these is a pattern invented for the Chat providers surface. Each owes
a generic catalog item.

| pattern in Chat | generic catalog item | category | exists? |
|---|---|---|---|
| the `subscriptions` / `API · usage-based` group chip | `elements/group-chip` — an elevated label that introduces a group and stands before the group has entries | static | **no** |
| the subscriptions carousel | `blocks/carousel-row` — one row, horizontal snap, arrow affordances, inertial scroll | interactive | **no** |
| the API grid | `blocks/tile-grid` — N-row-minimum scrolling grid, N authored | interactive | **no** |
| tile elevate-on-focus | `elements/lift-tile` — elevation step + scale, siblings do not reflow | interactive | **no** |
| selection burst | `motion.json` effect 20 — terminal radial shard emission, non-looping | animation | **no** |
| the sporadic chip hover | `motion.json` effect 21 — per-glyph phase offset, hue-step cadence, decay | animation | **no** |
| the `🧠 Providers` cross-link | `elements/bleed-control` — line-art at rest, two-hue dwell wash on hover | static | partial — the effect exists as `ink-bleed`, the element does not |

**Ordering law:** the generic item is authored in the SAME pass as the Chat
use, never "later". A pattern that ships to one surface first and is
generalized afterwards gets generalized to fit that surface.

### 2.4 Where the effects go

The burst and the sporadic wash are **effects**, not renderers, so they land
in `tokens/motion.json` beside the existing 19 and are referenced by id from
the `anim` axis. `tokens/motion.json` currently has no terminal (non-looping)
effect at all — the burst is the first, and the schema has to admit one.

## 3. Order of work

1. **N1** — the rename (§1). One pass, no shims. `cargo green` proves it.
2. **N2** — the `$extension` markers and the `lineage` field (§1.4, §1.5).
3. Then, and only then, the providers links — each carrying its generic
   catalog item per §2.3, because those items must be authored in the new
   vocabulary rather than renamed a second time.

## 4. What this does NOT decide

The generic renderers' composition, the burst's and the sporadic wash's
authored form (counts, spread, cadence, decay), and whether `infographics/`
becomes a real seed category are each decided in their own link's plan,
against the code as it stands then.
