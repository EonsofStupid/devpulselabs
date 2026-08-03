# 2026-08-02 — V1: the Providers control and the ink-bleed preset

Plan: `docs/plans/providers-v1-ink-bleed-control.md`.
Program: `docs/plans/providers-presence-and-the-brain.md` (link V1).
Mock: `docs/design/mocks/providers.html`.

## What the operator asked for

> "I click on what will be a neat optimized bleed effect that as the mouse
> just overs ever so slowly like ink to paper think of it as a passive
> experience not meant to be instant but literally over the coarse of 5.6 to
> 6.3 seconds before the full color takes over the icon and icon is brain and
> Providers and just equal at the height of P to the left we have a small
> circle with ai inside. The effect will have two color bleed where we have a
> pool of 4-5 colors that can randomly be pulled as this will become an
> elaborate preset that can be used globally for design."

## Done

Everything the operator named is now an AUTHORED token, and the widget reads
it:

- `tokens/motion.json` — a **`dwell` duration tier** (5.9, bounded 5.6–6.3),
  a `dwell` ease `[0.22,0.61,0.36,1.0]`, and the `ink-bleed` effect.
  `EFFECT_COUNT` 18 → 19.
- `tokens/icons.json` — `brain`, over the `brain.svg` already in the vendored
  Lucide pool, tessellated at build time like every other icon.
- `tokens/presets.json` — a **third preset kind**, `bleed`. `MotionPreset`
  carries one `enter` effect and no colour, and widening it for one caller
  would have changed all three existing motion presets. `generate_presets`
  cross-checks the bleed's effect against `motion.json` and every hue in its
  pool against `hues.json`, so **an unknown hue fails the build** — that is
  what makes "a preset used globally" real instead of a list of hexes in a
  widget. Proved by hitting exactly that panic mid-pass.
- `crates/shell/src/providers.rs` — the control: the `ai` circle whose
  diameter is a RATIO of the label size (so "equal at the height of the P"
  survives a font change), the brain, the word, and two washes seeping from
  two origins. It holds no duration, curve, hue or geometry of its own.

The seeded pair and the dwell jitter follow `random_mood_index`'s discipline
exactly — xorshift, no RNG dependency — and pin through `set_harness_mood`,
so `--mood 0` fixes the whole look of the bleed for a capture.

The control replaces P1's placeholder text chip as the same `StatusCell`
registry entry. Clicking it does nothing yet; the popup is V2.

## Two bugs the proof caught

**The bleed was not animating at all.** The first run's 1.2s and 6.6s
captures came back byte-identical. The animation id was derived from
`ui.id()`, and the status bar builds its lane as a fresh child `Ui` every
frame — so the id was new every frame, and egui treats an unseen id as
already-arrived and snaps to the target. A six-second passive bleed was an
instant colour swap. Fixed with a stable `egui::Id::new(...)`, the shape the
rail's unfold already uses, plus a `request_repaint` while mid-bleed.

**A mid-animation capture is impossible in this harness.** Every capture runs
under `MotionPolicy::Suppressed`
(`egui-pilot-ai-runtime/src/lib.rs:1246`) — `animation_time = 0` and
`clear_animations()`; the protocol calls it a "motion-suppressed stable
capture". `MotionPolicy::Natural` and `FastForward` exist in the enum and
nothing selects them. So no scenario can photograph any animation part-way
through, and that applies to all nineteen effects, not just this one. It is
recorded as a harness open; widening it is its own pass and was not done
here.

So V1 proves the two ENDS visually and the DURATION by pin.

## Proof

`tests/eguipilotai/providers-control.toml`, fixture `chat`, `--mood 0`:

- `docs/proof/providers-control-cold.png` — the `ai` circle, the brain and
  "Providers" as muted line-art, no wash.
- `docs/proof/providers-control-full.png` — the two-colour bleed behind the
  control, the brain and circle carrying the first hue, the word at full
  strength.

Nine new pins across `dpl-coldlight` and `dpl-shell` (35 + 26 green),
including `the_dwell_tier_is_authored_in_the_window_the_operator_named`,
`a_bleed_pair_is_always_two_different_hues` over 512 seeds, and
`the_same_seed_gives_the_same_pair`. `platform-sees-providers` still passes
against the control that replaced the cell. `cargo green` ✔.
