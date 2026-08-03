# DS2 — Element library (design-system build order §7.2)

**Status: PLAN — awaiting operator ratification.** No code until §4 scope +
§3's one open decision are signed off. Gated on the ratified
[`docs/design/design-system.md`](../design/design-system.md) (§1 Elements,
§5 Fidelity, §7 build order). This is step **②** of six: it lays the element
foundation the treatment/recipe/fidelity/renderer steps compose over.

---

## 1. Context — why this pass exists

The design-system spec §1 elevates today's `crates/slides/src/blocks/` from an
ad-hoc bag of free functions into "a real, organized, browsable library." Three
concrete gaps, ground-truthed this session:

- **No census.** Templates have one (`slides::registry::demos()` — every
  registered template constructed live, "a template that exists but never shows
  is impossible"). Elements have **none** — nothing enumerates them.
- **The Library element wall is hand-authored and already stale.**
  `workbench/src/library.rs::blocks_wall` (line 376) hard-lists only
  `eyebrow · badge · stat · ambient`. Two shipped elements — `bar` (added for
  compare_vs/checklist) and `divider` (section_divider/quote) — **do not appear
  on the wall at all.** A hand-maintained list drifts; that is the bug the
  template registry's gallery gate was built to kill, not yet applied here.
- **No `cost_tier`.** Spec §5 makes fidelity first-class and requires every
  element (and treatment/recipe) to declare a baseline tier. Elements carry no
  such field, so §5's boot probe has nothing to honor when it lands.

This pass closes all three, and only these three (§4 draws the line hard).

## 2. Current state (all ground-truthed this session)

- **`blocks/mod.rs`** — 6 elements, flat, exported as free functions:
  | element | signature | reuse (≥2-template promotion) |
  |---|---|---|
  | `eyebrow` / `tracked` | `(ui, fluid, text, Option<Color32>)` | recurred 9× (pre-promoted) |
  | `badge` | `(ui, fluid, text, live: Option<Tone>)` | callout / on-air chip |
  | `stat` | `(ui, fluid, value, label, Color32)` | big_number, stats_grid |
  | `bar` | `(painter, rect, frac, Tone, alpha)` | compare_vs, checklist |
  | `rule` (divider) | `(painter, left, y, width, Tone, alpha)` | section_divider, quote |
  | `media` | `Ambient`/`Media` enums + `*_backdrop(...)` fns | media_frame, backdrops |
- **The contract already half-holds.** Every element reads **only its
  arguments** — no route, no global state (the blocks doctrine's "standalone"
  law). Signatures split into two honest shapes: **`ui`-driven** (eyebrow,
  badge, stat — allocate + lay out) and **`painter`-driven** (bar, divider,
  media — paint into a caller-owned rect). The census must accommodate both.
- **The census exemplar to mirror** — `slides::registry.rs`: `TemplateId`
  (never a bare string), `TemplateSpec {id, label, class, blurb}`,
  `SlideTemplate` trait (`spec` / `ui` / `restart`), `demos()` (one live
  instance per template, one line each), and three teeth:
  `the_registry_is_the_census` (count + unique ids + non-empty labels),
  `the_folder_law_matches_the_registry_taxonomy`, and the gallery gate
  `every_demo_renders_at_start_and_at_rest` (paints each at t=0.05 and t=3.0
  without panicking).
- **`library.rs`** already shelves the template census **by class** into section
  headers (`TEMPLATE_CLASSES`, `by_class` bucketing, "the class stays honest and
  empty" for empty buckets) — the exact shelving idiom the element wall will
  adopt, keyed by category instead of class.

## 3. Design — the element census

Mirror the template registry, adapted for elements. **New module
`crates/slides/src/blocks/census.rs`**, re-exported from `blocks/mod.rs`.

### 3a. Types
```rust
pub struct ElementId(pub &'static str);          // never a bare string (registry idiom)

pub enum ElementCategory {                        // the SHELF, not a style (spec §1)
    Type, Mark, Meter, Divider, Media, Container, Control,
}

pub enum CostTier { Baseline, Enhanced, Max }     // = fidelity 0 / 1 / 2 (spec §5)

pub struct ElementSpec {
    pub id: ElementId,
    pub label: &'static str,
    pub category: ElementCategory,
    pub cost_tier: CostTier,                       // baseline tier this element paints at
    pub blurb: &'static str,                        // one line, Library shelf copy
}
```

### 3b. Uniform enumeration without disturbing call sites
Elements stay ergonomic free functions — templates keep calling
`blocks::stat(ui, fluid, …)` unchanged. The census adds a thin **demo wrapper**
per element (exactly as templates pair a `'static` spec with a live instance):
```rust
pub trait ElementDemo {
    fn spec(&self) -> &'static ElementSpec;
    fn paint(&self, ui: &mut egui::Ui);   // self-contained: calls the free fn with authored sample args
}
```
Each element ships a zero-size `…Demo` struct whose `paint` reproduces what
`blocks_wall` hand-wrote today (e.g. `StatDemo` calls `stat(ui, fluid, "128",
"tokens", tone)`). `painter`-driven elements allocate their own plate rect
inside `paint` (as the current ambient plate already does).

```rust
pub fn elements() -> Vec<Box<dyn ElementDemo>> {   // the census — one line per element
    vec![
        Box::new(EyebrowDemo), Box::new(StatDemo),          // Type
        Box::new(BadgeDemo),                                 // Mark
        Box::new(BarDemo),                                   // Meter
        Box::new(DividerDemo),                               // Divider
        Box::new(MediaDemo),                                 // Media
    ]
}
```

### 3c. Category population (today)
| Category | Elements today |
|---|---|
| Type | eyebrow, stat |
| Mark | badge |
| Meter | bar |
| Divider | divider |
| Media | media |
| Container | *(empty — honest shelf, plate/card/well arrive by promotion)* |
| Control | *(empty — button/toggle/stepper arrive with recipes, §④)* |

Empty shelves render honestly empty, matching `library.rs`'s existing
"the class stays honest and empty" idiom. All 6 elements are **`CostTier::
Baseline`** today — none paints an animated wash or multi-layer glow; `media`'s
ambient rings are cheap vector math, still Tier 0. `Enhanced`/`Max` variants and
authored fallbacks are **step ⑤'s** work (§4 out-of-scope).

### 3d. Library becomes census-driven
`blocks_wall` is rewritten to walk `dpl_slides::blocks::elements()`, bucketed by
`ElementCategory` into section headers — reusing the `templates_gallery`
shelving pattern verbatim. The hand-list dies; `bar` and `divider` appear
automatically; every future element appears the day it registers.

### 3e. ONE OPEN DECISION (needs your call before code)
The spec says "reorganize `blocks/` **into categories**." Two readings:

- **(A) Category-as-data (recommended).** Files stay flat; `category` on the
  spec drives all shelving. Zero import churn, honors "no re-export shims on
  moves" (CLAUDE.md), least blast radius. The taxonomy lives where it's used —
  the census + the wall.
- **(B) Physical subdirs** (`blocks/type/eyebrow.rs`, `blocks/mark/badge.rs`,
  …). Matches the words literally but moves every file → rewrites every
  `blocks::eyebrow` import across templates + `library.rs`, and the no-shim law
  forbids the soft landing. Cosmetic gain for real churn at 6 files.

I recommend **(A)** — the *substance* of "categories" is the shelving taxonomy
(data), which A delivers; physical layout is deferrable if the element count
ever outgrows a flat dir. Flagging because the spec's wording admits B.

## 4. Scope

**In:** `blocks/census.rs` (types + `ElementDemo` + per-element demos +
`elements()`); `cost_tier` + `category` metadata on all 6 elements; census tests
(count/unique-id/non-empty + the element gallery gate); `blocks_wall` rewritten
census-driven; the empty Container/Control shelves.

**Out (explicitly deferred to their own steps):**
- Treatments / `FrameStyle` threading into elements — **step ③** (elements
  keep their current `Tone`/`Color32` args this pass; the census doesn't force
  the treatment API yet).
- Recipes / interaction — **step ④** (Control category stays empty).
- The actual `Caps→Tier` boot probe, `--fidelity` flag, cheap-path fallbacks,
  and the paint pipeline honoring `cost_tier` — **step ⑤** (we only *declare*
  the field this pass).
- Composed-template renderer + catalog migration — **step ⑥**.
- New elements. This pass organizes the **existing 6**; it does not add
  plate/card/button. (Promotion rule unchanged: extract on ≥2-template recurrence.)

**Blast radius:** `crates/slides` (new module + 6 files gain a spec/demo) and
one function in `crates/hubs/workbench/src/library.rs`. No template touched, no
call site changed, no public element fn signature changed → the catalog keeps
working byte-for-byte.

## 5. Work plan (atomic)

1. `blocks/census.rs`: `ElementId`, `ElementCategory`, `CostTier`,
   `ElementSpec`, `ElementDemo` trait. Re-export from `mod.rs`.
2. Per element: a `static SPEC: ElementSpec` + a `…Demo` unit struct with
   `paint` (lift the sample-arg paint from today's `blocks_wall`, plus the two
   uncovered ones for `bar`/`divider`).
3. `elements()` census (§3b), registry order = category order.
4. Census tests (mirror `registry.rs`): count == 6, unique ids, non-empty
   label/blurb, **element gallery gate** (each `paint` runs at t=0.05 and t=3.0
   under a headless `egui::Context` without panicking).
5. Rewrite `library.rs::blocks_wall` to walk `elements()` bucketed by category
   (reuse `block_plate`; add category section headers per the class-section
   idiom). Delete the hand-list.
6. `cargo green` (fmt + clippy no-`#[allow]` + tests + release build).

## 6. Risks

- **`painter`- vs `ui`-driven split.** The gallery gate must give
  `painter`-driven demos a real rect. Mitigation: each `…Demo::paint` allocates
  its own plate (the current ambient plate proves the pattern) — the trait only
  ever hands out a `ui`.
- **Category assignment is a judgment call** (`stat` → Type vs a future
  Container). Low stakes: category is data, re-shelving is a one-line edit, and
  the ≥2-template promotion rule already governs what *is* an element.
- **Decision A vs B churn** (§3e) — resolved by ratifying A up front; picking B
  later is a mechanical move-pass, not a redesign.
- **Scope creep toward treatments.** The census tempts threading `FrameStyle`
  now; §4 forbids it — elements keep current args, step ③ owns that seam.

## 7. Verification / acceptance

- `cargo green` clean on the real tree (fmt, clippy no-allow, tests, release
  build) — the wired pre-push gate.
- New tests pass: element census (count/unique/non-empty) + the element gallery
  gate paints all 6 at start and at rest without panic.
- **Proof on the real exe** (CLAUDE.md law): rebuild `devpulselabs.exe`, then a
  screenshot of the Library element wall at a **real** window size showing all 6
  elements shelved by category — `bar` and `divider` now present (they are
  absent today). Acceptance = the wall is census-driven (grep confirms no
  element is named by hand in `blocks_wall`).
- Regression guard: the 28-template census test and every template render
  unchanged (this pass touches no template).

## 8. Evidence

*(filled at proof time — census test output, `cargo green` tail, and the
real-size Library element-wall screenshot with all 6 elements shelved.)*
