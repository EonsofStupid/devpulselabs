# DevPULSE Design System — the composable architecture

**Status: RATIFIED 2026-07-27 · REVISED 2026-08-09 (§0.5, §0.6, §2, §4, §7).**
The overarching contract for the catalog: how UI is *built*, not just how it
looks. The look layer's ingredients are owned by
[`doctrine.md`](doctrine.md) and [`coldlight-matrix.md`](coldlight-matrix.md);
this document defines the layers around them, the **platform contract** we hold
with egui (§0.5), the **naming law** every term in the codebase answers to
(§0.6), and how it all composes. Supersedes the ad-hoc "hand-painted template"
model (the W-series `title_card`/`quote`/… monoliths were a stopgap,
re-authored onto this in the §7.6 pass).

Ratified decisions: **element = the UI primitive** (the composer's imported
pack files rename `ELEMENTS → pack assets` gradually, copy-only, no churn now);
**big-bang catalog re-author** once the system stands (§7.6, one concentrated
consistency pass, not opportunistic).

**The 2026-08-09 revision** answers a standing operator instruction — *"ensure
platform industry terminology is being used… if these terms are wrong we need
to merge them to their correct code file that aligns with how an SME in Rust
would author it"* — and records the ground-truthed audit that prompted it. It
does not soften any 2026-07-27 ratification; §6 stands.

Gate: every UI pass invokes `coldlight-system` and leaves surfaces clean
against BOTH this doc and doctrine.md.

---

## 0. The one idea

> **A slide is a tree of NODES. A node is placed in a SLOT, wears a
> COORDINATE (explicit) or an INTENT (semantic), and moves on a TRACK. Any
> interaction is a RECIPE. All of it paints through one 2D pipeline governed
> by a FIDELITY budget.**

*(Revised 2026-08-09: "template → layout of elements" became "slide → tree of
nodes" once §4 specified the cascade; "treatment" retired to "coordinate" per
§0.6 when the matrix superseded traits. The layering is unchanged.)*

Nothing is hand-painted that could be composed. A new slide design should be
*authored data* (elements + treatments + recipes) over *new paint code* — the
same law the slide document layer (`dpl_doc::Slide`) already follows for text.

This is the mature-design-system stack, mapped to a native egui app:

```
tokens        (generated pool: hues, fluid, motion, radius)      — design tokens
  └─ ELEMENTS       (primitives / atoms — organized, standalone) — Radix primitives
       └─ NODE TREE     (compositions / organisms)               — the retained tree (React owns; egui does NOT — §0.5)
   dressed by COORDINATES (explicit) via INTENTS (semantic)      — variants + alias tokens (CVA, Material core→semantic)
   moved by TRACKS        (keyframes, per-channel)               — WAAPI / CSS @keyframes
   behaving via RECIPES   (interaction patterns)                 — interaction patterns (Panda)
   painted under a FIDELITY budget (i5 baseline → scales up)     — progressive enhancement / graphics tiers
```

Industry parallels are exact: **design tokens** (Style Dictionary / Material's
core→semantic split), **atomic design** (atoms→organisms), **variant recipes**
(Stitches/CVA, Panda CSS literally calls them "recipes"), and **adaptive
fidelity** (game-engine graphics presets). The one place we go past a web
design system — native 2D rendering with a hardware cost budget — is correct
for egui and is the whole point of §5.

---

## 0.5 The platform contract — what egui 0.35 actually gives us

*Added 2026-08-09. Ground-truthed against the vendored source at
`~/.cargo/registry/src/*/egui-0.35.0/src`. Version pinned in `Cargo.toml`.*

You cannot design a system on a platform you are guessing about. This section
is the honest inventory, and it is written as a **React → egui map** because
that is the vocabulary available to the operator.

### 0.5.1 The one structural fact

**egui is immediate-mode: there is no retained widget tree.** Every frame
re-runs your function top to bottom and paints from scratch. Nothing survives
between frames except `egui::Memory` (a keyed blob store) and whatever *your*
app owns.

React's retained tree — the thing a `<Slide>` component lives in — **has no
counterpart in egui**. There is no node to hold a style on, no parent to
inherit from, no element to query.

The consequence is the single most important design decision in this repo:

> **`dpl_doc` IS our retained tree. egui is only the painter.**
> Any document model, cascade, provenance, or parent→child inheritance must be
> built in `dpl-doc` and resolved to a flat paint list *before* egui is called.
> egui will never do it for us, and no amount of egui API will change that.

### 0.5.2 The map

| React / CSS | egui 0.35 | source file |
|---|---|---|
| component | a **function** `fn(&mut Ui, …) -> Response`. No class, no lifecycle, no props type | `ui.rs` |
| props | function arguments | — |
| `useState` | `Memory`, keyed by `Id`: `ui.data_mut(\|d\| d.get_temp_mut_or_default::<T>(id))` | `memory/` |
| `key` | `Id` / `IdSalt` — hashed from a source **plus the parent's id** | `id.rs`, `id_salt.rs` |
| the CSS box | **`Frame`** — `fill`, `stroke`, `corner_radius`, `shadow`, `inner_margin`, `outer_margin` | `containers/frame.rs` |
| `flex-direction` / `align-items` / `justify-content` | **`Layout`** — `top_down(Align)`, `left_to_right(Align)`, `bottom_up`, `centered_and_justified`, `with_main_wrap`, `with_cross_align`, `with_main_justify` | `layout.rs` |
| CSS Grid | `Grid`, or explicit `Rect` math | `grid.rs` |
| `position: absolute` / a portal | **`Area`** on its own layer | `containers/area.rs` |
| `z-index` | `LayerId` + `Order` (`Background → Middle → Foreground → Tooltip → Debug`) | `layers.rs` |
| `overflow: scroll` | `ScrollArea` | `containers/scroll_area.rs` |
| a pan/zoom canvas (the Figma surface) | **`Scene`** — wraps content in a `TSTransform`, with `fit_to_rect_in_scene` and a `zoom_range` | `containers/scene.rs` |
| the transform matrix | `emath::TSTransform` — **translate + uniform scale only.** No rotation, no shear, no perspective | emath |
| `pointer-events` | `Sense` (`hover` / `click` / `drag`) | `sense.rs` |
| `onClick` / `onHover` / drag | `Response::clicked()`, `.hovered()`, `.dragged()`, `.drag_delta()` | `response.rs` |
| the paint call | `Painter` → `Shape` (`Rect`, `Circle`, `Path`, **`Mesh`**, `Text`, `CubicBezier`, `QuadraticBezier`) | `painter.rs`, epaint |
| `clip-path` / `overflow: hidden` | `Painter::with_clip_rect` / `Ui::set_clip_rect` | `painter.rs` |
| the theme object | `Style` → `Visuals`, `TextStyle → FontId` | `style.rs` |
| CSS `transition` | `Context::animate_bool_with_time` / `animate_value_with_time` — a built-in per-`Id` tween | `animation_manager.rs` |
| composable widget content **(new in 0.35)** | **`egui::atomics`** — `Atom`, `AtomKind`, `AtomLayout`: text + icon + image composed inside ONE widget in ONE layout pass | `atomics/` |
| a custom container | `Ui::scope_builder(UiBuilder)` / `ui.new_child(UiBuilder::new().max_rect(r))` | `ui_builder.rs` |
| paint *behind* something you haven't measured yet | `painter.add(Shape::Noop) -> PaintIdx`, measure, then `painter.set(idx, shape)` | `painter.rs` |

### 0.5.3 What egui does NOT have — build it or drop it

- **No gradients.** A gradient is a `Mesh` with per-vertex colour. This is why
  `mood_blend` / `aurora` build meshes by hand, and it is the correct
  technique — not a workaround.
- **No blur.** Confirms doctrine law 6 from the platform side: the no-blur law
  is not taste, it is what the renderer is good at.
- **No rotation in the scene transform.** `TSTransform` is translate+scale.
  Rotation exists only per-`Shape` (galley rotation, mesh vertex math). Any
  design that assumes a rotating container is unbuildable as specified.
- **No document model, no cascade, no selectors.** §0.5.1.
- **No "slot" and no "axis."** Neither word appears in egui. They are **ours**,
  and §0.6 defines them before §4 uses them.

### 0.5.4 Two capabilities we have and do not use

Recorded because both are load-bearing for the roadmap and their absence is a
gap, not a decision:

1. **`Scene`** — the pan/zoom container an editor canvas is built on. The
   Workbench editor currently does its own rect math. ⬜
2. **`egui::atomics`** — 0.35's composable widget content. Our element library
   (§1) is the layer that should be sitting on it. ⬜

---

## 0.6 The naming law, and the merge table

*Added 2026-08-09.*

**THE LAW.** Every identifier in this codebase is exactly one of:

- **(a) the platform's** — Rust's or egui's, used with *precisely* that
  meaning (`Frame`, `Layout`, `Response`, `Sense`, `Id`, `Painter`, `Mesh`;
  and every Rust keyword). Never shadowed, never redefined, never given a
  second meaning. *(Rust and egui are listed together deliberately: a keyword
  collision is a Rust concern, not an egui one — see the `trait` row, where
  conflating the two produced a misattributed argument.)*
- **(b) the industry's** — used with *precisely* the industry's meaning
  (design tokens, variant, recipe, cascade, intent, keyframe/`Track`/`Stop`,
  WAAPI channel names). If we use the word, we owe the semantics.
- **(c) ours** — permitted, but it **must be defined in this document before
  any code uses it**, and it must not collide with (a) or (b).

A term that is none of the three is drift and is fixed at sight. A term that
is *retired* is **merged into its correct owner**, never left as an alias, a
shim, or a deprecation layer (pre-release law).

### 0.6.1 The merge table

Ground-truthed 2026-08-09 against `crates/coldlight/src/lib.rs` (3,342 lines,
one file), `crates/doc/src/lib.rs`, and every call site in `crates/`.
"Sites" is the real churn.

| today | why it is wrong | correct name | owner file | sites |
|---|---|---|---|---|
| `Tone` (6-variant enum) | **collides with (b).** Material 3 defines *tone* as a **lightness step**; ours picks a **hue family**. Our lightness step is separately called `step`. Two industries' meanings crossed in one type. | **`Accent`** — six semantic accent roles selected from the 53-hue pool; `HueId` stays the pool index | `coldlight/src/hue.rs` | **358** |
| `TraitDef`, `tokens/traits.json`, `compose()` | **PRIMARY: it is a second compositor.** The matrix superseded it, and `Dress` exists only to bridge the two. **SECONDARY (naming, and only that):** `trait` is a strict Rust keyword — not an egui concern, egui has no opinion on the word. It compiles fine as `TraitDef`, but it forced four prefixed identifiers (`trait_ids` `build.rs:169`, `trait_def` `lib.rs:487`, `trait_id` `lib.rs:844`, `trait_name` fabric), and `TraitState` misreads in a Rust file. The rename comes **free with the deletion**; it would not justify one. | **deleted.** The 5 traits become 5 `Preset`s (named `Coordinate`s) — the `vibron_the_coordinate_paints_vibron_the_trait` test already proves this is a reclassification | `coldlight/tokens/presets.json` | 7 |
| `Dress` (`Trait \| Coord`) | a bridge that exists only because two compositors do | **deleted** with the trait path | — | — |
| `AncestralPreset` | qualified by a facet name, leaking the taxonomy into the type | **`Preset`** — the one preset in the design system: a named `Coordinate` + `Accent` | `coldlight/src/preset.rs` | — |
| `ThemePreset` | **layering violation, not just a name clash.** All four fields (`font`, `moods`, `motion`, `ambience`) are `dpl_doc::Theme` fields. The design system was holding document defaults. | **moves to `dpl-doc`**, keeping the name, beside the `Theme` it presets. (`dpl-doc` already depends on `dpl-coldlight`; the direction is legal.) | `doc/src/theme.rs` | — |
| `Treated`, `treated()`, "treatment" | pre-matrix vocabulary; the matrix superseded it (§2) | **`Styled`** — `Styled::new(coord, accent).show(ui, \|ui\| …)`, mirroring `egui::Frame::show` exactly | `coldlight/src/paint.rs` | 1 |
| `scatter_frame()`, `card_frame()` | hand-built `egui::Frame` with literal shadow numbers — the exact bespoke-paint defect | **deleted.** `ResolvedStyle::frame() -> egui::Frame` is the one converter | `coldlight/src/paint.rs` | 16 |
| `Chan` | needless abbreviation; in Rust `chan` reads as `mpsc` | **`Channel`** (WAAPI's own word) | `coldlight/src/motion.rs` | — |
| `Block` (flat, text-only) | not a tree, so no cascade, no provenance, no ripple (§4) | **`Node`** + `NodeKind`. **Not `Entity`** — in Rust GUI, `Entity` means an ECS id (Bevy) and implies components+systems we do not have. A tree node is a `Node`. | `doc/src/node.rs` | schema |
| `Axes` / "axis" | **ours, undefined** — used before it was specified | **kept, now defined:** an *axis* is one styling property channel carrying a `MergeRule::{Override, Additive}` | `coldlight/src/style.rs` | — |
| "slot" | **ours, undefined** | **kept, now defined:** `Slot(&'static str)` names a `Rect` region a layout produces (§4.2) | `doc/src/layout.rs` | — |
| everything in one `lib.rs` | 3,342 lines in a single module is not how an SME ships a crate | **file split** — `hue · fluid · matrix · style · paint · motion · preset · icon`, each `pub use`d from a thin `lib.rs`. That facade is the canonical path, not a shim. | `coldlight/src/*.rs` | — |

### 0.6.2 Merged, NOT killed — one authored source, zero call-site churn

These three were duplicated data, not wrong names. The fix is to give them one
authored source and **generate** the existing consts from it, so ~1,150 call
sites do not move and chrome does not start jittering:

| today | the duplication | the merge |
|---|---|---|
| `cl::space::S1..S6` (hardcoded `f32`) | parallel to `Fluid::sp_1..sp_8` | **generated from `tokens/fluid.json` at its nominal rem.** One authored ramp. Law: **fixed `space::*` for chrome, `Fluid::sp_*` for stage content** — chrome must not resize with the slide. (260 sites unchanged) |
| `cl::RADIUS`, `RADIUS_SM`, `RADIUS_LG` | parallel to `StructureDef.radius` | **generated from `tokens/fluid.json`**, same law. (105 sites unchanged) |
| `cl::surface::*`, `cl::ink::*` (raw hex) | the ONLY colours in the app outside the 53-hue pool — and the reason `paper`/`plate` materials are hue-independent | authored as a **`neutral` ramp in `tokens/hues.json`**; `build.rs` generates the same const names from pool steps. (792 sites unchanged) |

### 0.6.3 Terms that are already right — do not touch

`Intent` (semantic intent — industry-correct) · `Fluid` (ours, defined:
container-relative scale from any `Rect`) · `Coordinate` / `Ancestor` /
`Structure` / `Material` / `Atmosphere` (ours, defined in
`coldlight-matrix.md` §2 from Ranganathan's PMEST) · `ResolvedStyle`
(unambiguous; deliberately not `Style`, which is egui's) · `Ease`, `Motion`,
`Stop`, `Track`, `Effect` (WAAPI — `keyframe-engine.md`) · `Recipe` (Panda /
Vanilla Extract — §3) · `element`, `template` (atomic design — §1, §4).

---

## 1. Elements (primitives)

An **element** is the smallest composable unit that paints or lays out — the
tier the current `crates/slides/src/blocks/` holds (`eyebrow`, `badge`, `stat`,
`bar`, `divider`, `media`), elevated to a real, organized, browsable library.

**Contract.**
- **Standalone.** An element reads only its arguments — never a route, never
  global state (carried from the blocks doctrine). It is usable on its own and
  inside any template.
- **Data in, paint out.** `element(painter, Fluid, props, &ResolvedStyle,
  &Ctx, tier)`. It never resolves its own dressing — it *receives* a
  `ResolvedStyle` and paints through it. *(Signature updated 2026-08-09:
  "treatment" retired per §0.6; `&Ctx` added per §4.4 — an element cannot
  stagger or ripple without knowing its index and its parent's motion.)*
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

## 2. Dressing — coordinates, and the semantic layer over them

**REVISED 2026-08-09.** The 2026-07-27 text specified a two-kind "treatment"
system over `compose(tone, base, &stack, state)`. The **ColdLight matrix**
([`coldlight-matrix.md`](coldlight-matrix.md), 2026-07-31) superseded the
composite tier: a dressing is no longer a base+stack of traits, it is a
**`Coordinate`** — one point in a four-facet space, with legality as data. The
word "treatment" retires per §0.6; the *idea* of a two-tier explicit/semantic
split is kept, because it was correct.

### 2a. The explicit tier — a `Coordinate`

```rust
Coordinate { ancestor, structure, material, atmosphere }   // + an Accent
```
Resolved by the ONE compositor, `matrix::resolve(&Coordinate, hue, state) ->
ResolvedStyle`. Facet ownership (from `coldlight-matrix.md` §2):

| facet | PMEST | owns |
|---|---|---|
| **ancestor** | Personality | the font |
| **structure** | Space | radius, edge width |
| **material** | Matter | fill, ink, edge colour |
| **atmosphere** | Energy | shadows, back-glow, tints, anim |
| *state* | Time | runtime only — never authored into a coordinate |

**Legality is data, not convention.** Every facet value is in exactly one of
`native` / `allowed` / `forbidden` per ancestor — 8 ancestors × 23 values =
184 rulings, and an unclassified pair **fails the build**. This is the
guarantee that replaces "don't stack retro on claymorphic, it looks bad."

### 2b. The semantic tier — `Intent` bound per surface

Named by **role**, not construction: `Intent::{Chrome, Hero, Quiet, Danger}`,
bound to coordinates in **`tokens/roles.json`**.

> **Correction of record, 2026-08-09:** §2b's 2026-07-27 plan called for
> `tokens/treatments.json`. That file was never authored — but the layer
> **was** built, as `roles.json`. `treatments.json` is therefore **cancelled,
> not pending**; `roles.json` is the semantic binding layer, and §7 is
> corrected to match.

```jsonc
// tokens/roles.json — a binding is a coordinate (or, today, still a trait ref)
"surfaces": {
  "wiredfront": { "chrome": { "coordinate": {
      "ancestor": "brutalist", "structure": "massive",
      "material": "plate", "atmosphere": "hard-cast" } } }
}
```

**The law (unchanged and still right):** authored surfaces reach for an
**`Intent`**; only the design system binds an intent to a coordinate. Re-point
one binding and every `Danger` control in the app moves.

**Adoption, ground-truthed 2026-08-09:** `roles.json` binds **one** role to a
coordinate — `wiredfront.chrome`. All four defaults and the other two surfaces
are still trait refs. **The matrix paints exactly one thing in the whole app.**
Completing that migration is §7.3.

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

## 4. The slide schema — the source of truth

**REWRITTEN 2026-08-09.** The 2026-07-27 text sketched
`Template { category, layout, slots: [Slot { … }] }` in one line. This is that
line, specified — and it is the **source of truth** the operator asked for:
*"a schema that defines the slide clearly so we can then have the complexity
behind the scenes. When we want to extend the properties and capabilities of
that slide it has a source of truth."*

### 4.0 Why it must change — the ground truth

`crates/slides/src/document.rs:309-463`, `render_inner`, is the ONE path every
surface renders a document through. Read in full 2026-08-09:

- **Layout is a single centred vertical stack** — `y += heights[i] + gap`
  (`:460`). No axis, no slot, no grid, no columns.
- **A `Block::Text` resolves exactly three properties:** `size` → fluid ramp
  (`:443`), `font` → family (`:447`), `ink` → four semantic names (`:444`).
  No fill, edge, radius, shadow, glow, material, or ancestor.
- **`matrix::resolve` is never called on this path.** Slides never touch the
  matrix. §2's whole apparatus is unreachable from a slide.
- **Motion is hardcoded to one effect** — `cl::motion::ENTER_RISE` (`:394`),
  gated on the literal string `"rise"` (`:393`). Nineteen effects exist.
- **Two of `Motion`'s five channels are read** — `alpha` and `offset.y`.
  `scale`, `rotation`, and `clip` never reach a pixel from a slide.
- **`ResolvedStyle.anim` has zero readers app-wide.** "Atmosphere carries
  motion" is currently data with nothing on the other end.

This is the concrete answer to *"the plans never show up in the work."* The
engine is genuinely built; the renderer consumes roughly a tenth of it. The
missing piece is not more engine — it is **a node to hang the engine on**.

### 4.1 `Node` — the styleable, animatable unit

A flat `Vec<Block>` cannot cascade, cannot carry provenance, and cannot
ripple, because none of those mean anything without a parent. The schema
becomes a **tree**:

```rust
// crates/doc/src/node.rs
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    /// Where the parent's layout places this node. None = next in flow.
    pub slot: Option<Slot>,
    /// Explicit dressing. None = INHERIT the parent's (§4.3).
    pub coord: Option<Coordinate>,
    /// Accent override. None = inherit.
    pub accent: Option<Accent>,
    /// Effect pool ref driving this node's `Motion`. None = inherit.
    pub motion: Option<String>,
    pub children: Vec<Node>,
}

pub enum NodeKind {
    Text { text: String, size: String },
    Media { show: String, src: String, fit: Option<String>, poster: Option<String> },
    /// A layout-only node: paints its own dressing, arranges children.
    Group { layout: LayoutSpec },
    /// A registered element from §1, by census id, with authored props.
    Element { element: String, props: Props },
}
```

**Naming, per §0.6:** this is `Node`, not `Entity`. In Rust GUI, `Entity`
means an ECS handle (Bevy) and implies components + systems we do not have and
do not want. A node in a retained tree is a `Node` — the same word Figma,
Graphite, and the DOM use. *(Where the operator says "entity," this is the
type.)*

**Every styling field is `Option`.** Absent = inherit. This is the sparse-field
law `Slide` already follows, extended down the tree — and it is what makes a
slide file stay small and legible while the complexity sits behind it.

### 4.2 `Slot` — defined, because egui has none

> **`Slot(&'static str)` names a `Rect` region that a parent's layout
> produces.** It is a *label for a position*, nothing more. A `LayoutSpec`
> maps slot names to rects; a child either names a slot or falls into flow
> order.

```rust
pub enum LayoutSpec {
    /// Today's behaviour, kept: one centred column. Slots ignored.
    Stack { gap: String, align: Align },
    /// Named columns with weights — the `split` category's real home.
    Split { slots: Vec<(Slot, f32)> },
    /// Rows × cols; slots address cells "r1c2".
    Grid { rows: u8, cols: u8 },
    /// Slots carry normalized rects — the free-placement canvas.
    Free { slots: Vec<(Slot, [f32; 4])> },
}
```

`Stack` is exactly what `render_inner` does today, so the migration has a
provable identity case: **a v3 tree of `Stack` + `Text` nodes must paint
pixel-identical to the v2 flat blocks.** That is the same reclassification
proof `keyframe-engine.md` §5 used, and it is what makes this safe.

### 4.3 The cascade — four levels

The operator's mental model is CSS, and CSS is the right model. Resolution is
a pure function, top-down, one pass:

```
1. Theme      (doc/theme.json)   — the document root. "material as :root".
2. Slide      (slide.json)       — the document override.
3. Parent Node                   — NEW. inheritance down the tree.
4. Node                          — the leaf's own override.
```

Levels 1–2 exist today (`doc::resolve`, `lib.rs:764`) and are explicitly
documented as *"exactly two cascade levels… deliberately without a third
tier."* **Level 3 is the addition**, and it is the whole unlock: without it a
`Group` cannot dress its children, which is what "the complexity behind the
scenes" means in practice.

Resolution stays pure and returns a fully-specified tree — a fixed point,
same as today. The renderer paints what it is handed and decides nothing.

### 4.4 `Ctx` — provenance, surrounding context, and ripple

The operator's requirement: *"provenance, and surrounding context and impacts
so that motion would provide a ripple effect one way on the surface of the
background."* A node cannot know any of that from its own fields. It is
carried **down** the resolve walk:

```rust
pub struct Ctx<'a> {
    /// PROVENANCE: which ancestor node supplied each inherited facet.
    /// Answers "why is this red?" — the editor's inspector reads this.
    pub source: &'a Provenance,
    /// SURROUNDING CONTEXT: where the parent put us, and how big.
    pub parent_rect: Rect,
    /// IMPACT / RIPPLE: the parent's sampled Motion this frame.
    pub parent_motion: Motion,
    /// Position among siblings — drives per-index stagger.
    pub index: usize,
    pub sibling_count: usize,
    pub depth: u8,
}
```

- **Provenance** is not decoration: it is the only honest way an inspector can
  say *"inherited from the theme"* vs *"set on this node."* Every inherited
  field records the node id it came from.
- **Ripple** is `parent_motion` plus a `Shape::Noop` reservation
  (§0.5.2) — the parent reserves its paint index, children resolve and report,
  the parent then paints its backdrop *knowing what happened above it*. The
  technique is already used in `Treated::show` (`coldlight/src/lib.rs:962`);
  this generalises it. **No new rendering machinery.**
- **Ripple is one-way, by law.** Parent → child, plus the single deferred
  backdrop write. No child→parent relayout, no second pass, no cycles. That
  keeps resolve O(n) and immediate-mode honest.

### 4.5 Worked example — the five rings

The operator's acceptance test: *"a small effect of like 4 or 5 rings that grow
in size and have each entity capable of being styled while performing and
optimized for any system."*

```jsonc
{ "id": "rings", "kind": "group", "layout": { "kind": "free" },
  "coord": { "ancestor": "vibron", "structure": "airy",
             "material": "glass", "atmosphere": "soft-bloom" },
  "motion": "enter-bloom",
  "children": [
    { "id": "r1", "kind": "element", "element": "ring", "props": { "r": 0.20 } },
    { "id": "r2", "kind": "element", "element": "ring", "props": { "r": 0.35 } },
    { "id": "r3", "kind": "element", "element": "ring", "props": { "r": 0.50 },
      "accent": "coral" },
    { "id": "r4", "kind": "element", "element": "ring", "props": { "r": 0.65 } },
    { "id": "r5", "kind": "element", "element": "ring", "props": { "r": 0.80 } }
  ]
}
```

Every requirement, against what exists today:

| requirement | mechanism | status |
|---|---|---|
| five entities, staggered | `Effect::sample(now, t0, ctx.index)` | ✅ built |
| "grow" | `Channel::Scale` via `Track::sample` | ✅ built, **renderer never reads it** |
| each individually styled | `r3`'s `accent` override; the rest inherit | ✅ engine built, **unreachable from a slide** |
| styled *while performing* | resolve is pure; `Motion` multiplies the resolved style at paint | ✅ built |
| ripple onto the background | `ctx.parent_motion` + deferred backdrop write | ⬜ §4.4 |
| **optimized for any system** | 5 `Shape::Circle` in one `Painter` call is negligible. The real per-frame cost is `ResolvedStyle` allocating **four `Vec`s per resolve** (`shadows`, `back_glow`, `tints`, `anim`). The fix is the §5 cache, keyed `(coord, accent, state, tier)` — free after first paint. | ⬜ **NOT YET BENCHED.** The frame budget is 16.6 ms at 60 fps (arithmetic). The resolve cost is *not* measured — step 3′ owes a `criterion` bench, and no tier claim may be made from an unbenched number. |

**Nothing in this example needs a new engine.** It needs `Node`, `Slot`, level
3 of the cascade, and `Ctx`. That is the entire gap.

### 4.6 Templates, after the schema

A **template** is then what §0's one idea always said: a `Node` tree with
holes. `category ∈ {static, animation, interactive, split, infographics}` stays
the slide's KIND, orthogonal to dressing; `interactive` ⇔ carries a recipe
(§3). The renderer is the ONE path — resolve → measure → paint, under the
fidelity budget (§5). No template hand-paints.

**Migration.** `SCHEMA_V` 2 → 3. A v2 flat `blocks` array loads as a v3 tree of
one `Stack` group, proven pixel-identical (§4.2). The W-series hand-painted
templates are re-expressed as node trees in the §6.3 big-bang pass; each
migration deletes bespoke paint.

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
  compositor caches `ResolvedStyle` by **`(coord, accent, state, tier)`** so
  resolution is free after first paint. *(Key updated 2026-08-09 — the old
  `(tone, base, stack, …)` key named the retired trait compositor, §0.6.)*

---

## 6. Decisions (ratified 2026-07-27)

1. **The word "element" — SETTLED.** UI primitive = **element** (this doc's
   layer 1). The composer's imported pack files (mogrt/aep/… in
   `content/elements/`) rename to **pack assets** in copy gradually — no code
   churn now, flagged so the Library never shelves two "elements."
2. **Where treatments+recipes live as data — SETTLED (proposal stands).**
   `tokens/treatments.json` (semantic bindings) + a `recipes` registry in
   `dpl-slides` (code-authored state machines, params as data), both
   generated/registered like `traits.json`.
   > **Superseded in part, 2026-08-09 — the DECISION holds, the FILENAMES
   > moved.** "Semantic bindings live as generated token data" is exactly what
   > shipped; it shipped as **`tokens/roles.json`** over the matrix, not
   > `treatments.json` over `traits.json` (§2b). `traits.json` is retired
   > entirely (§0.6). The recipes half is unchanged and still owed (§7.4).
3. **Migration appetite — SETTLED: BIG-BANG.** Once the system stands
   (elements → treatments → recipes → fidelity + the composed renderer), the
   catalog is re-authored onto it in ONE concentrated consistency pass (§7.6),
   not opportunistically. The reward is a uniformly composable catalog; the
   cost is one large, well-gated migration PR.

---

## 7. Build order (spec → system → migrate)

**REVISED 2026-08-09** with ground-truthed status. The 2026-07-27 order was
right about the layers and wrong about two things: it did not know the matrix
was coming (which cancelled step 3's file), and it assumed the schema could
carry the system (§4.0 proves it cannot). Status is evidence, not intent.

| # | step | status | evidence |
|---|---|---|---|
| 1 | **This spec** | ✅ ratified 2026-07-27, revised 2026-08-09 | this file |
| 2 | **Element library** — categories, census, render gate, `cost_tier` | 🟡 **partial** — `slides/src/elements/census.rs` has `ElementSpec`, `CostTier`, `ElementDemo`, and **7 registered elements, all `Baseline`**. No probe honours the tier (that is step 5). | read 2026-08-09 |
| 3 | ~~Treatment layer (`tokens/treatments.json`)~~ | ⛔ **CANCELLED** — superseded by the matrix + `roles.json` (§2b). The file was never authored and never will be. | `coldlight-matrix.md`, `tokens/roles.json` |
| **3′** | **Naming merge + module split** — §0.6's table, in one pass: `Tone→Accent`, kill traits/`Dress`/`compose`, `Treated→Styled`, delete the bespoke frames, generate `space`/`RADIUS`/`surface`/`ink` from one authored source, split `lib.rs` into modules. | ⬜ **NEW, and it goes first** — every later step writes call sites, and writing them twice is the drift this revision exists to stop. | §0.6 |
| **3″** | **Complete the matrix binding** — every default and every surface in `roles.json` becomes a coordinate. | ⬜ **NEW** — today the matrix paints exactly one role (`wiredfront.chrome`). | §2b |
| **3‴** | **Migrate the 19 effects to tracks** — each becomes a 2-stop `Track`, proven identical at `t ∈ {0,.25,.5,.75,1}`. | ⬜ **NEW** — 1 of 19 done (`enter-bloom`). | `keyframe-engine.md` §5 |
| 4 | **Recipe layer** — the `Recipe` trait + registry; lift `advance`/`toggle`/`flip`/`match`/`poll` out of the interactive templates | ⬜ not started — `trait Recipe` / `RecipeState` are 0 files | grep 2026-08-09 |
| 5 | **Fidelity probe** — `Caps→Tier` at boot, `--fidelity`, `cost_tier` + fallbacks honoured | ⬜ not started — `Fidelity` is 0 files; `cost_tier` is authored but unread | grep 2026-08-09 |
| **6a** | **The schema** — `SCHEMA_V` 2→3: `Node` tree, `Slot`, cascade level 3, `Ctx`. Gated on the v2→v3 pixel-identity proof (§4.2). | ⬜ **the real §4** | §4 |
| **6b** | **The renderer + BIG-BANG migration** — `render_inner` becomes the resolve→measure→paint walk over the node tree; then the one concentrated pass re-authoring the hand-painted catalog, deleting every bespoke `ui()` (§6.3). | ⬜ | §4.6 |

**Ordering law.** 3′ before everything, because it is the only step that
*removes* surface area; every step after it is cheaper for having run. 6a
before 6b, because a renderer for a schema that does not exist is how we got
`render_inner`.

Each step is its own plan doc (7 sections + §8 evidence) + PR + `cargo green`,
gated on this spec. Steps 2–5 land the system alongside the working (still
hand-painted) catalog; step 6b flips the catalog over in one pass.

---

## 8. Reference — what a senior egui/Rust engineer reads

*Added 2026-08-09, per the operator's "we just need to find a strong reference
project to utilize."*

| project | what to take | what NOT to take |
|---|---|---|
| **egui 0.35 source** — vendored at `~/.cargo/registry/src/*/egui-0.35.0/src` | the platform truth (§0.5). Read `containers/scene.rs` and `atomics/` first — both are capabilities we own and do not use. | — |
| **`rerun`'s `re_ui` crate** | the largest production egui app there is, and it carries a real design-token layer over egui (tokens loaded from JSON, list items, docking). Closest existing thing to ColdLight. | ⚠️ **Not yet read in-session.** Ground-truth it before leaning on any specific claim about its internals — plan-before-code law. |
| **Graphite** (Rust 2D vector editor) | the *document model*: retained node tree, per-node properties, transform stack. Exactly §4's shape. | it is not egui — take the model, never the rendering. |
| **WAAPI keyframe format** (MDN) | already adopted — `keyframe-engine.md` §3. | the CSS emitter leg is deferred (§7 ruling, 2026-08-05). |
| **Style Dictionary / Material 3 core→semantic** | already adopted — the token pipeline and §2b's explicit/semantic split. | Material's *word* "tone" means lightness; ours meant hue. That collision is §0.6's first row. |
