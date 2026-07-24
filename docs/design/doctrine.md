# Design doctrine — the composable trait system (ColdLight, ported)

This is the contract the first app carried in feel. It is no longer a two-word
invention: it is the original's **stackable trait system** faithfully ported to
egui. Ratified in `docs/plans/pass0-coldlight-treatments.md`; gated by the
`coldlight-system` skill (formerly `dpl-style`). The ONLY implementation is
ColdLight — surfaces never hand-roll treatments.

## The concept, in egui

The original ran: author JSON → Style Dictionary → CSS vars → cascade. egui has no
cascade, so it lands as: **author JSON → `build.rs` generates Rust → a pure
compositor resolves a `ResolvedStyle` struct → immediate-mode paint**. Same
architecture, better fit — the compositor is one pure, testable, cacheable
function.

```
tokens/traits.json (AUTHORED)  ──build.rs──▶  traits_gen.rs (GENERATED, do-not-edit)
        │
compose(tone, base, &[stack], state) -> ResolvedStyle   (pure, per-frame, cacheable)
        │
paint fns take ResolvedStyle → back-glow + Frame + content
        │
manifest (generated) ← the Workbench editor reads it ("only offer what exists")
```

## The treatments (traits)

Two BASE treatments + two STACKABLE traits (operator-ratified 2026-07-24). Authored
as data in `crates/coldlight/tokens/traits.json`; every color is a **ladder-relative
ref** resolved against the surface's runtime `Tone`, so one trait dresses any hue.

- **NEUBRUTALIST** (base, *actionable*) — the egui-native strength move. Flat
  saturated fill (tone 400), crisp 2px hard edge (tone 700), HARD offset shadow
  `[3,3]` zero blur (a shape, not a glow), tight radius. The "lit-from-behind" pop
  is a CHEAP painted back-glow (tone 300 halo behind the rect) — **never a real
  blur**. Loud, honest, unmistakably tappable, fast. Hover lifts the shadow +
  glow; active presses the shadow in.
- **CLAYMORPHIC** (base, *passive*) — soft extruded plane (the original's neumorphic
  specialty). Directional double-shadow (light top-left, dark bottom-right),
  generous radius, hairline edge, optional two-color diffuse tint. Content sits IN
  it; chrome sits ON it.
- **CYBER** (stack) — cyan back-glow + a cyan top wash + hue-drift, thin bright
  edge. Rides on top of a base.
- **RETRO** (stack) — scanline texture + chunky amber edge + sharp corners +
  flicker. Rides on top of a base.

**Never a bespoke frame.** `brutal_frame`/`clay_frame` are legacy bridges being
migrated onto `compose(...)`; do not extend them.

## The composition law (per-axis)

The axis contract is a `ResolvedStyle` struct. Each axis is **additive** or
**override** (a static table, mirroring the original's `ADDITIVE_AXES`):

- **Additive** — `shadows`, `tints`, `back_glow`, `anim`. Every trait in the stack
  CONCATENATES its contribution (retro scanline AND cyber glow both paint).
- **Override** — `fill`, `edge`, `radius`, `ink`, `font`. Last trait in the stack
  wins.

Within one trait, a `hover`/`active` state overlay REPLACES that trait's axes for
that state; across traits, additive axes concatenate. Every axis resolves in every
state (fixes the original's B1 defect). `base=neubrutalist, stack=[cyber]` →
hard brutalist edge/shadow PLUS a cyan glow, from one data change, not a
hand-authored combination.

## The laws

1. **Tooltip law** — every interactive control ships `cl::tip(resp, "…")` in plain
   layman words. A control without a tip fails review.
2. **Generated tokens only** — color via the pool (`ramp`/`Tone`/trait refs); size
   via `Fluid::of(rect)`; radius/space/motion via generated consts. No raw hex, no
   magic paddings, no bespoke treatment code.
3. **Any-shape rendering** — every surface renders at any window shape
   (`--window <w>x<h>` proofs; the fit guard owns the window). Fixed dims need a
   stated reason.
4. **Motion respects PilotClock** — no uncontrolled animation on new surfaces.
5. **Proof includes feel** — a UI pass's §8 evidence includes at least one shot the
   operator could mistake for the old app's polish. That is the acceptance bar.
6. **No blur, ever** — egui backdrop blur is weak and expensive. Depth comes from
   hard shadows, painted back-glow, gradient tint, and scatter — never a feathered
   border or a real blur.

## Slide categories are NOT styling

`static · animation · interactive · split · infographics` are the slide's
**category** (a document field + registry key), with open stackable **tags**
(`topic:*`, `episode:*`, …) for the Workbench Library filter. Orthogonal to traits —
the trait system dresses whatever the slide is.

## Rollout

Foundation (traits data + generator + compositor) lands first (Pass 0 §4). Then the
`slide-catalog-pass` loop ports the catalog through the trait system. Every UI/slide
pass after this doc MUST invoke `coldlight-system` and leave touched surfaces
doctrine-clean.
