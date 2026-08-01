# Pass — effects get motion axes, and the pool becomes generated

**Parent:** `docs/plans/workbench-editor.md` — this is the MOTION layer under
its §3.4 `Bucket::Effects`, whose catalog source is *"`cl::TREATMENTS` +
recipes + motion"*. Treatments and recipes are that plan's; the motion third is
this one's. Its §2.4 ("everything is an ENTITY; presets are entity-scoped") and
§2.2 ("a preset is a REF, not a bake") are RATIFIED and are not reopened here.
**Operator target:** *"I need like 300 total eventually"* — and *"I am copying
all of my effects from After Effects."*
**Sibling law:** `hues.json` L4 — *"a large pool. Adding a palette costs one
object and yields a 13-step ladder + utilities + types."*

## 1. Context — why 18 cannot become 300

Two blockers, and the second is the real one.

**The data says nothing about motion.** An effect today is four timing numbers:

```json
"chart-rise": { "dur": 0.6, "ease": "emphasized", "stagger": 0.12, "delay": 0.3 }
```

No opacity, no offset, no scale, no rotation, no direction, no origin.
`chart-rise` and `enter-rise` differ only in decimals.

**And `Effect::at()` returns a scalar, so every call site invents the motion.**

```rust
// crates/slides/src/templates/callout_card/mod.rs:65
let p     = cl::motion::ENTER_RISE.at(now, t0, 0);
let alpha = p.clamp(0.0, 1.0);          // this caller decides: opacity
let lift  = fluid.sp_6() * (1.0 - p);   // this caller decides: y-lift of sp_6
```

`ENTER_RISE` is not a rise. It is a timing curve, and `callout_card`
independently decided that a rise means *fade + a 6-unit lift*. `bar_compare`,
`donut_share`, `line_trend`, `gauge_dial`, `probability_bars`, `cost_meter_face`,
`media` and `document.rs` each made their own decision at their own call site.

So **adding effect #19 does not add a motion** — it adds a number that some
template must be hand-taught to interpret. That is the ceiling, and no amount of
authoring gets past it.

**Nowhere to see them either.** `LibSection` has ten shelves — Seeds, Templates,
Blocks, Elements, Looks, Colors, Presets, Fonts, Icons, Moods. **Effects is not
one of them.** All 18 are referenced only by `build.rs`, `coldlight/src/lib.rs`
and the composer. The pool has never been visible in the app.

## 2. Current state (ground-truthed, `53b321f`)

`tokens/motion.json`: `durations` (4) · `eases` (6, cubic-bezier control points)
· `effects` (18).

`build.rs` emits, per effect, a const of

```rust
pub struct Effect { dur: f32, delay: f32, stagger: f32, ease: Ease, looped: bool }
```

plus `EFFECT_COUNT` and a `&str -> Option<&Effect>` lookup that degrades to
`None` rather than panicking. `Effect::at(now, t0, index) -> f32` is eased,
clamped 0..1, resting at 1 (or wrapping when `looped`).

**What is good and stays:** the ease pool, the clock contract, the
degrade-never-panic lookup, `in_flight` driving `request_repaint`, and the
presenter law that every state is a rest state.

## 3. Design — axes from After Effects, one resolved transform out

**The axes are AE's own transform properties**, which is the source the operator
is copying from — Position, Scale, Rotation, Opacity — plus mask-driven wipe and
stroke draw for the chart family.

```json
{ "id": "rise",
  "motion":   "translate",         // translate·scale·rotate·opacity·wipe·draw·pulse·bloom
  "axis":     "y",                 // x·y·xy·radial
  "from":     24, "to": 0,         // FLUID units, never px
  "envelope": "emphasized-600",    // named, over the existing durations+eases
  "apply":    "per-child",         // whole·per-child
  "stagger":  0.12,
  "accent":   null }               // glow·bloom·shake·scanline
```

**The generated pool.** motions × axes × envelopes × apply, skipping
combinations that are meaningless (`rotate` has no `x` axis; `stagger` is inert
for `whole`). Roughly **8 × 4 × 5 × 2 ≈ 320 from ~20 authored objects** — the
hues law, pointed at motion. Every entry gets a stable generated id
(`rise-y-emphasized-stagger`), and authored **aliases** keep today's names
(`enter-rise` → that id) so no call site breaks on the rename.

**The API change that actually unlocks it.** `at()` stays — progress is still
useful. Beside it:

```rust
pub struct Motion {
    pub alpha: f32,        // 1.0 = opaque
    pub offset: Vec2,      // in fluid units, caller scales by Fluid
    pub scale: f32,        // 1.0 = natural
    pub rotation: f32,     // radians
    pub clip: f32,         // 0..1 wipe/draw reveal fraction
}
pub fn sample(&self, now: f64, t0: f64, index: usize) -> Motion;
```

A call site stops deciding what the motion is and applies what it is given.
`callout_card` becomes:

```rust
let m = cl::motion::ENTER_RISE.sample(now, t0, 0);
let alpha = m.alpha;
let lift  = fluid.sp_6() * m.offset.y;   // Fluid still owns SIZE; the effect owns SHAPE
```

**Fluid keeps owning size.** `from`/`to` are in fluid units and the caller
multiplies by its own `Fluid` — so an effect never carries a pixel and never
fights the fluid system.

**No new dependency, no new crate.** This is `tokens/motion.json` + `build.rs` +
the `Motion` type, inside `dpl-coldlight` where motion already lives.

## 4. Scope

**In:** the authored axis schema · the generator + skip rules · `Motion` and
`sample()` · aliases for all 18 current ids · migrating the ~12 call sites off
hand-invented math · the census/drift test · `EFFECT_COUNT` becoming the
generated pool size.

**Out, and named:**
- **The Effects shelf.** Its own pass, immediately after (the pool has to exist
  before a wall can render it).
- **`accent` overlays** (glow/bloom/shake/scanline). The field is authored and
  generated, but painting an accent is trait-system work, not motion — it lands
  with the treatment pass.
- **Recipes.** Interaction is design-system §3 and a separate tier.
- **Multi-event slides.** `reveals` stays one linear axis in this pass.
- **Lottie.** `dpl-lottie` plays authored AE comps via the subprocess helper
  (ADR-009) and is untouched — that is playback, this is the parameterized pool.

## 5. Work plan (atomic, in order — commit at each step that compiles)

1. `Motion` + `sample()` over the CURRENT `Effect`, deriving alpha/offset from
   the existing scalar so behaviour is identical. Tests pin the equivalence.
2. Migrate the ~12 call sites to `sample()`. No visual change — a pure
   refactor, provable by unchanged proof shots.
3. Extend `tokens/motion.json` to the axis schema, authoring the ~20 objects and
   the alias table. The 18 existing ids keep resolving.
4. `build.rs` generates the pool + skip rules + alias resolution; `EFFECT_COUNT`
   becomes the pool size; census test fails on drift.
5. `sample()` reads the real axes instead of the derived approximation.
6. `cargo green`; proof shots at 1280×800 and 1100×700; §8; STATE.md.

Steps 1–2 are behaviour-preserving and land first on purpose: the risky part
(re-authoring the data) then has a green baseline to move against.

## 6. Risks

- **A silent visual regression across 12 templates.** This pass rewrites how
  every animated slide moves. Mitigation: step 1–2 are provably identical, and
  the proof shots from before the pass are the comparison for after.
- **A combinatorial pool that is mostly nonsense.** 320 entries where 200 are
  useless is worse than 18. Mitigation: skip rules are authored and tested, and
  the alias table names the ones worth reaching for — the pool is browsable, the
  aliases are what templates use.
- **`EFFECT_COUNT` is asserted somewhere.** Mitigation: it becomes the generated
  count; the census test is updated in the same commit (structure law).
- **Fluid coupling.** If `from`/`to` ever become pixels, the fluid system is
  broken from a data file. Mitigation: the unit is named in the schema and a
  test rejects an effect authored with a pixel unit.
- **Scope drift into the shelf.** Mitigation: §4 names it out; the shelf is the
  next pass.

## 7. Verification / acceptance

- `cargo green` clean, both lanes.
- A test that every one of the 18 current ids still resolves, via alias, to a
  pool entry.
- A test that `sample()` at `t0` and at `t0 + dur` gives the authored `from` and
  `to` for each motion kind.
- A test that a `whole`-apply effect ignores `index` and a `per-child` one does
  not.
- A test that an unknown effect id still returns `None` and never panics.
- Census: `EFFECT_COUNT` equals the generated pool length; drift fails the gate.
- Proof shots at both sizes of an animated slide (callout, bar-compare,
  line-trend), read BY EYE against the pre-pass shots.

  **Not a pixel comparison, and this was measured rather than assumed.** Two
  runs of the *same* binary with identical flags differ by ~1.13M of 3.07M
  subpixels: the animation clock is wall-clock driven and the shot fires at an
  arbitrary phase. `--mood 0` pins the palette, not the clock. Variance by
  region: overlay modal centre 55.8%, backdrop 24.0%, footer 11.6%, topbar
  9.7%, rail 5.6% — so it is the animating SLIDE PREVIEW, not just the backdrop.

  The mechanical proof for steps 1–2 is therefore
  `sample_reproduces_the_hand_math_it_replaces`, which pins alpha/offset/clip
  against the exact expressions the call sites used, at four points in time.

  **Follow-up, worth its own pass:** drive the clock from a virtual time in
  screenshot mode (`t = shot_frame / 60.0`) instead of wall-clock. Two runs at
  one frame would then be byte-identical, and visual regression across this
  whole repo becomes checkable instead of eyeballed — which the "proof on the
  real exe" law has been quietly missing.

## 8. Evidence

_(filled on completion)_
