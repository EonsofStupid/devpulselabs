# Pass — thumbnail mode for the document renderer

A slide drawn into a card-sized stage crops. This pass makes a small stage
render a true miniature instead, in `dpl-slides` where the sizing happens —
not in each surface that happens to want a small one.

Lane journal: `docs/journal/2026-07-29-authoring-lane-resume.md`, NEXT item 4
(named in #136, evidenced again in `docs/plans/author-filmstrip.md` §8.3).

## 1. Context — why this pass exists

Every surface that shows a slide small shows it broken:

- the authoring **carousel** (#136) — at 1100×700 the seed cards read
  "all y?", "4", "G", "ba" (`docs/proof/author-filmstrip-1100x700.png`)
- the Library's **seeds wall**
- the **filmstrip**, which had to ship labelled plates instead of miniatures
  because a real render would have looked worse than no render

It has been deferred twice as "the seed-thumbnail crop". It is not a surface
bug and no surface can fix it: three of them would each need the same
workaround, which is the definition of a fix belonging one layer down.

## 2. Current state (ground-truthed in-session, `d71d5bc`)

**The cause is the clamp, and it is deliberate.** `cl::Fluid` sizes content
to whatever rect it is given — the doctrine's upgrade over the web original,
"so a slide sizes to its STAGE … with identical code and zero
ScaledStage/transform hacks" (`coldlight/src/lib.rs`). But every token is
evaluated through:

```rust
fn eval(&self, min: f32, max: f32, slope: f32, slope_y: f32, intercept: f32) -> f32 {
    (intercept + slope * self.w_rem + slope_y * self.h_rem).clamp(min, max) * 16.0
}
```

The `.clamp(min, max)` is a **floor**. `fs-display` has `min: 3.0` rem — 48
points — so below roughly a 600-point-wide stage a display headline stops
shrinking while the stage keeps going. At a 300-point card the headline is
still 48pt: it fills, wraps, and crops. The floor is right for a real stage
(text must stay legible) and wrong for a miniature (nobody reads a
thumbnail).

**The scope is unusually clean.** `crates/slides/src/document.rs` contains
**zero** references to the fixed `cl::space::S*` constants — checked, the
count is 0. Every size in a document render, type and padding alike
(`pad = fluid.sp_7()`, `wrap`, every `size_of` ref), flows through one
`Fluid`. One scale factor on `Fluid` therefore scales an entire document
proportionally, with no second mechanism and nothing left behind at full
size.

**Entry points** (`document.rs`): `render_step(ui, rect, slide, look, step,
now, t0)` and `DocSlide::ui(&mut self, ui, rect)`, both funnelling into
`render_inner`. `DocSlide` carries its own `Clock`, which is why the seeds
wall and the carousel animate independently.

**Callers**: `crates/deck` (full stage), `hubs/workbench` seeds wall and
episode wall, `author.rs` carousel. Only the small-stage callers want the
new behaviour.

## 3. Design — a miniature is a scaled full stage, not a small stage

Add a scale to `Fluid` and one constructor that sets it:

```rust
/// Sizing for a stage rendered as a MINIATURE: evaluated at the nominal
/// stage it stands for, then scaled down to the rect it is drawn in.
pub fn thumbnail(rect: Rect, nominal_width: f32) -> Self
```

`eval` gains a single trailing `* self.scale`; `Fluid::of` sets `scale: 1.0`
and is unchanged in behaviour. Because `document.rs` sources every size from
`Fluid`, this is the whole mechanism.

This keeps the doctrine's promise rather than bending it. "A slide sizes to
its stage" still holds for every real stage. A thumbnail is declared to be a
different thing — a picture *of* the full stage — so the honest way to make
one is to evaluate at full size and scale, which is exactly what the rest of
the world means by "thumbnail". No transform layer, no ScaledStage, no
second render path.

**What "nominal" is.** The reference stage a miniature stands for. It is a
16:9 stage at the width where the ramp stops clamping — derived from the
tokens, not chosen: the smallest width at which no type token is still
pinned to its `min`. Computing it from the generated table keeps it honest
and self-updating when the ramp is re-authored.

**The opt-in is per-call**, so nothing changes for a caller that does not ask:
`DocSlide::thumbnail_ui(ui, rect)` beside `ui`, and a `render_thumbnail`
beside `render_step`.

## 4. Scope

**In:** `Fluid::thumbnail` + the `scale` field · the nominal width derived
from the generated ramp · thumbnail entry points on `DocSlide` and the free
renderer · the three small-stage callers switched over (carousel, seeds wall,
filmstrip tiles) · tests · proof shots at both window sizes.

**Out, and named:**
- **Caching a rendered thumbnail to a texture.** Painting is cheap here and
  premature caching would add invalidation. If a wall of 40 miniatures ever
  measures slow, that is its own pass with its own evidence.
- **Media/video blocks inside a miniature.** A backdrop already paints to the
  rect; a video does not belong in a thumbnail. Left exactly as-is.
- Changing the ramp's authored `min` values. The floor is correct for real
  stages; this pass does not touch `tokens/fluid.json`.

## 5. Work plan (atomic, in order — commit at each step that compiles)

1. `Fluid::scale` + `Fluid::thumbnail` + the derived nominal width, with
   tests. `coldlight` alone; no caller changes.
2. `render_thumbnail` + `DocSlide::thumbnail_ui` in `dpl-slides`.
3. The carousel switches over; proof shot at 1100×700 against the one in
   `author-filmstrip` §8.2 as the before.
4. The filmstrip tiles switch over — they stop being labelled plates and
   become real miniatures, closing the note in `film_tile`.
5. Proof shots, §8, `docs/STATE.md`.

**Correction to §1, ground-truthed after this plan was written.** The seeds
wall is NOT a caller: `library.rs` sizes its stage to the full available
width (`avail_w - S4*2`) and the episode wall caps at 720 points. Both are
real stages well clear of the ramp's floor, and neither crops. The only
genuinely card-sized stages are the authoring **carousel** cards and the
**filmstrip tiles** — both in `author.rs`. Scope shrinks accordingly; the
"three surfaces each needing the same workaround" argument in §1 rests on
two, which is still one more than belongs in a surface.

## 6. Risks

- **A miniature that is now legible-but-tiny may read as worse than a crop
  to the operator.** Mitigation: the before/after shots at 1100×700 are the
  evidence, and the feel bar (coldlight-system §5) decides — not the
  arithmetic being correct.
- **`scale` silently applying where it should not.** Mitigation: `Fluid::of`
  keeps `scale: 1.0`, and a test asserts `of()` and `thumbnail()` at the same
  rect differ only by the expected factor.
- **Wrap width interacts with scale.** `wrap` is derived from `rect.width()`
  and real padding, not from `Fluid` — check it scales coherently, or derive
  it from the nominal too. This is the one place the "everything flows
  through Fluid" claim needs verifying against the code rather than assumed.

## 7. Verification / acceptance

- `cargo green` clean.
- A test pins that no type token is clamped at the derived nominal width —
  the definition of "nominal" made executable.
- A test renders the same slide at full and thumbnail scale and asserts the
  block heights are proportional.
- Real-exe proof at **1280×800** and **1100×700**, sequential, scratch
  content root: the carousel cards readable as whole cards, not crops.
- The feel bar: the shot has to look better than
  `docs/proof/author-filmstrip-1100x700.png`, or the pass is not done.

## 8. Evidence

_Filled during the pass._
