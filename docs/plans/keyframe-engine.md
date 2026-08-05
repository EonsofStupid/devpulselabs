# Determination — the keyframe engine: one authored format, three consumers

**Status:** RESEARCH + DETERMINATION, 2026-08-05. Operator: *"you need to
research and determine how you are going to generate keyframes for the system
to pull from reliably. This should be a sophisticated pattern that will also be
used for all of our design."*

Not an implementation plan yet — this fixes the FORMAT, because the format is
the expensive decision. Implementation follows once it is ratified.

## 1. Why the current motion system cannot carry this

Ground-truthed in-session 2026-08-05.

`tokens/motion.json` authors **19 effects**, and every one has the same shape:

```jsonc
"enter-bloom": { "dur": 0.5, "ease": "spring-soft", "delay": 0.0 }
```

`cl::motion::Effect` (`lib.rs:1331`) is `{ dur, delay, stagger, ease, looped }`
and `Effect::at()` returns **one eased scalar 0→1**. `Effect::sample()` then
turns that scalar into a `Motion { alpha, offset, scale, rotation, clip }` —
and the mapping is **hardcoded in Rust**, per its own doc-comment:

> *"The interpretation here is the one every call site was already applying by
> hand, written down once: fade in, rise from below, reveal forward."*

That comment also records exactly why this matters:

> *"That is why the pool could not grow: a new effect was a new number somebody
> had to be hand-taught to interpret, not a new motion."*

**Three consequences:**

1. **All 19 effects share ONE curve shape.** You cannot author "alpha 0→1 over
   the first 40%, scale 0.8→1.05→1.0 with an overshoot at 60%" — different
   channels on different schedules is unexpressible.
2. **The meaning lives in Rust, so the web targets cannot see it.** This
   violates `coldlight-matrix.md` §9.1 head-on: *"Nothing may be authored in
   Rust… if it lives in Rust code rather than `tokens/*.json`, the web targets
   cannot see it and the three renderers diverge silently."* A Svelte build
   would have to re-implement `sample()` by hand.
3. **Two other keyframe debts exist and would each invent their own.**
   `dpl-comp` (`lib.rs:8`): *"Transform keyframes … land in later R1 passes."*
   `composer` (`lib.rs:16`) owes the same. Without one model we get three.

## 2. What already works and must be preserved

Not a rewrite — the good parts are load-bearing:

- **`Motion` is a resolved struct, not a scalar** — the fix for the "bare
  number" defect. It stays the output type.
- **Normalised, never pixels.** `offset` is *a fraction of the caller's
  amplitude*, so `Fluid` keeps owning size and the effect owns shape. One
  effect then reads correctly at every window size. **This law is kept.**
- **`Ease` is a real cubic-bézier with a Newton solve** — *"the same math the
  browser ran"*. Identical curves across targets come free.
- **Sampling is a pure function of time**, which is what lets `PilotClock` pin
  a frame for deterministic proof shots (doctrine law 4).

## 3. The format — WAAPI-shaped, because that is the standard

Researched: the [Web Animations API keyframe format](https://developer.mozilla.org/en-US/docs/Web/API/Web_Animations_API/Keyframe_Formats)
is the right model and maps 1:1 onto CSS `@keyframes`:

- offsets are **0.0–1.0, ascending**; an omitted offset is **evenly spaced**
  between its neighbours;
- **easing is per-keyframe and applies FROM that keyframe until the next** —
  segment easing, not one curve for the whole animation;
- overshoot is expressible, because a cubic-bézier may leave `[0,1]` — which
  the authored `spring-soft [0.34, 1.56, 0.64, 1.0]` already relies on.

Adopting it means the format is a standard rather than a house invention, and
the CSS emitter is close to a transcription.

### The authored shape

```jsonc
// tokens/motion.json — effects gain an optional `track`.
// Channels are EXACTLY Motion's fields. Omitted channel = REST, untouched.
"enter-bloom": {
  "dur": 0.5, "delay": 0.0, "stagger": 0.0, "looped": false,
  "track": [
    { "offset": 0.0, "ease": "spring-soft", "alpha": 0.0, "scale": 0.86 },
    { "offset": 0.62, "ease": "decelerate",  "alpha": 1.0, "scale": 1.04 },
    { "offset": 1.0,                          "scale": 1.0 }
  ]
}
```

**Laws the generator enforces** (same shape as `generate_icons` /
`generate_matrix`, which already fail the build on bad refs):

1. offsets in `[0,1]`, strictly ascending, first `0.0`, last `1.0`;
2. every `ease` resolves in `eases` — unknown ref fails the build;
3. every channel key is a real `Motion` field — a typo fails the build;
4. a channel that appears in any stop must appear in the **first and last**
   stop, so its start and end are never implicit;
5. an effect with **no** `track` keeps today's behaviour exactly (§5).

## 4. Three consumers, one definition

```
tokens/motion.json  ← THE ONLY AUTHORED THING
        │
   ┌────┴─────────────────┬──────────────────────┐
   ▼                      ▼                      ▼
build.rs               gen:css                dpl-comp
KeyframeTrack consts   @keyframes blocks      layer transform tracks
+ sample(t) -> Motion  + cubic-bezier()       (same Track type)
   │                      │                      │
 egui                  Svelte / RAC           the editor
```

**egui** — generated const data, zero per-frame allocation:

```rust
pub struct Stop { pub offset: f32, pub ease: Ease, pub ch: &'static [(Chan, f32)] }
pub struct Track { pub stops: &'static [Stop] }
impl Track { pub fn sample(&self, p: f32) -> Motion }   // pure, PilotClock-pinnable
```

`sample` finds the bracketing pair, eases the local `p` with the **left** stop's
ease (WAAPI's rule), and lerps each channel. Channels absent from a stop hold
their previous value; channels absent everywhere stay at `Motion::REST`.

**CSS** — a near-transcription:

```css
@keyframes enter-bloom {
  0%   { opacity: 0; transform: scale(.86); animation-timing-function: cubic-bezier(.34,1.56,.64,1) }
  62%  { opacity: 1; transform: scale(1.04); animation-timing-function: cubic-bezier(0,0,.2,1) }
  100% { transform: scale(1) }
}
```

The one thing needing an authored decision is the **channel → CSS property
map**, since it is not 1:1: `alpha → opacity`, `offset → translate()`,
`scale → scale()`, `rotation → rotate()`, and **`clip`** has no single
equivalent (`clip-path: inset()` vs a mask). That map is data, not code, and
lives beside the channel enum.

**dpl-comp** — the editor's transform keyframes become the same `Track` over
its own channels (`x`, `y`, `scale`, `opacity`), so the editor's timeline and
the design system share one evaluator and one authoring format. **Lottie stays
out**: ThorVG owns Bodymovin keyframes as an ADR-009 subprocess, and pulling
that in would be a second engine.

## 5. Migration — provably a reclassification

Today's 19 effects each become a **2-stop track** carrying the interpretation
`sample()` currently hardcodes. `enter-rise`'s hand-written "fade in, rise from
below" becomes:

```jsonc
"track": [ { "offset": 0, "ease": "decelerate", "alpha": 0, "offset_y": 1 },
           { "offset": 1, "alpha": 1, "offset_y": 0 } ]
```

**The proof:** a test samples every one of the 19 effects at
`t ∈ {0, .25, .5, .75, 1}` through BOTH paths — legacy `sample()` and the new
`Track::sample()` — and asserts the `Motion` values are identical. Migration is
then a reclassification, not a restyle, and the same test guards the port.

An effect with no `track` falls through to the legacy path, so the two coexist
during migration and nothing is big-bang.

## 6. What this does NOT do

- **No new runtime dependency.** Sampling is lerp + the existing Newton solve.
- **No uncontrolled animation.** Doctrine law 4 holds — `sample` is a pure
  function of `t`; the caller supplies the clock.
- **No pixel values.** The normalisation law is unchanged.
- **No Lottie merge.** ThorVG keeps Bodymovin (ADR-009).
- **No spring solver.** Springs stay cubic-bézier approximations, which is what
  is authored today and what CSS can express. A real spring integrator would
  not survive the CSS leg.

## 7. Open questions for the operator

1. **`clip` → CSS.** `clip-path: inset()` or a mask? It is the one channel
   without a clean web equivalent, and picking wrong means the web leg fakes it.
2. **Per-channel easing.** WAAPI eases a whole keyframe, not one channel. Do we
   want alpha and scale on *different* curves within one segment? It is
   expressible (a track per channel), but it doubles the format's surface and
   leaves the WAAPI standard behind.
3. **Does the editor share `Track` now or later?** Sharing it now shapes the
   channel enum; deferring risks a second model.

## 8. Evidence

*(filled during implementation)*
