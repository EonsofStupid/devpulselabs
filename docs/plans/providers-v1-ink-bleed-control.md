# V1 — the Providers control and the ink-bleed preset

Program: `docs/plans/providers-presence-and-the-brain.md` (link V1).
Mock: `docs/design/mocks/providers.html` — the control at the top of it.
Supplies its state from: P1's roster (`docs/plans/chat-providers-become-platform.md`).

## 1. Context

The operator, in their own words:

> "I click on what will be a neat optimized bleed effect that as the mouse
> just overs ever so slowly like ink to paper think of it as a passive
> experience not meant to be instant but literally over the coarse of 5.6 to
> 6.3 seconds before the full color takes over the icon and icon is brain
> and Providers and just equal at the height of P to the left we have a small
> circle with ai inside. The effect will have two color bleed where we have a
> pool of 4-5 colors that can randomly be pulled as this will become an
> elaborate preset that can be used globally for design."

P1 made the platform able to SEE providers and parked a plain text chip
(`AI · Claude Code`) in the status bar as the smallest honest consumer. V1
replaces that placeholder with the control the mock draws, and lands the
bleed as an authored preset any surface can reuse.

## 2. Current state

Ground-truthed 2026-08-02 against `feat/p1-provider-availability`.

### 2.1 The token pipeline is exactly the right shape already

`crates/coldlight/build.rs` reads eight authored JSON sources and GENERATES
Rust into `OUT_DIR`. Relevant generators:

- `generate_motion` (`build.rs:~430`) — `durations` → `DUR_<NAME>: f32`,
  `eases` → `EASE_<NAME>: Ease`, `effects` → `<NAME>: Effect` plus an
  `effect(name)` lookup. An effect referencing an unknown ease **panics the
  build**.
- `generate_icons` (`build.rs:123`) — `tokens/icons.json` entries resolve a
  `lucide` id into `assets/lucide/icons/<id>.svg`, tessellated at build time
  by `dpl_svg::tessellate_icon`. A missing SVG **panics the build**.
- `generate_presets` (`build.rs:629`) — "a preset is DATA over the other
  registries; every ref is cross-checked HERE against the authored sources,
  so an unknown font/mood/effect/ambience fails the build."

So the generated-tokens-only law is enforced by construction, and V1 adds to
it rather than working around it.

### 2.2 What exists

| piece | state |
|---|---|
| `durations` | `fast .12 · base .2 · slow .32 · settle .3` — nothing near six seconds |
| `eases` | 6, none matching the mock's `cubic-bezier(.22,.61,.36,1)` |
| `effects` | 18; the two long ones (`ambient-drift` 12s, `blend-drift` 16s) are **loops**, not transitions |
| `brain` icon | **not in `tokens/icons.json`** — but `assets/lucide/icons/brain.svg` IS in the vendored pool (1756 icons, tracked) and is stroke line-art, which is what the mock draws |
| the five pool hues | all five are **`brand`** hues already: `viridian · cyan · electric · magenta · amber` |
| `MotionPreset` | `{id,label,blurb,enter}` — a single ENTER effect ref. No pool, no hover concept |
| `draw_icon(painter, id, rect, color)` | takes the colour per call, so animating a stroke colour is a lerped `Color32` |
| hover animation | `ui.ctx().animate_bool_with_time(id, on, secs)` — used at `shell/src/lib.rs:752` |
| seeded randomness | `dpl_backdrop::random_mood_index()` — time-seeded xorshift, **no RNG dep**, pinned by `--mood <n>` via `set_mood` |
| the control | **does not exist**; `crates/shell/src/status.rs` has the P1 placeholder text cell |

### 2.3 The one real gap in the vocabulary

A ~6s hover transition is ~18× `slow`. It is not a long `slow`; it is the
category the operator named — *"a passive experience not meant to be
instant."* And `MotionPreset` cannot express it: it carries one `enter`
effect and no colour, while the bleed is a hover effect over a PAIR of hues.

## 3. Target design

### 3.1 A `dwell` duration tier, authored

`tokens/motion.json` `durations` gains:

```json
"dwell": 5.9, "dwell-min": 5.6, "dwell-max": 6.3
```

The window lives in the tokens, not in the widget, so the seeded jitter picks
between two AUTHORED bounds. `DUR_DWELL_MIN` / `DUR_DWELL_MAX` generate for
free from the existing `durations` loop — no generator change.

### 3.2 The ease and the effect, authored

`eases` gains `"dwell": [0.22, 0.61, 0.36, 1.0]` — the mock's curve exactly.
`effects` gains:

```json
"ink-bleed": { "dur": 5.9, "ease": "dwell" }
```

Both generate through the existing paths. The effect referencing `dwell`
proves the ease exists at build time.

### 3.3 The brain icon, authored

`tokens/icons.json` gains
`{"id":"brain","lucide":"brain","label":"Providers","blurb":"the frontier AI on this machine"}`.
It is the 25th entry; `brain.svg` is already in the pool, so the build
tessellates it with no new asset.

### 3.4 A BLEED preset — the "elaborate preset used globally"

`MotionPreset` cannot carry a hue pair, and widening it would change all
three existing motion presets for one caller. So `presets.json` gains a third
kind beside `look` and `motion`:

```json
"bleed": [
  { "id": "providers-ink", "label": "Providers ink",
    "effect": "ink-bleed", "hues": ["cyan","magenta","amber","viridian","electric"],
    "blurb": "Two colours seeping from two origins, ink into paper." }
]
```

`generate_presets` already cross-checks every preset ref against the authored
sources; this extends that to the new kind — **an unknown effect or an
unknown hue id fails the build**, which is what makes the pool authored
rather than a list of hexes in a widget.

Generated as:

```rust
pub struct BleedPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub blurb: &'static str,
    pub effect: &'static str,
    pub hues: &'static [&'static str],
}
```

### 3.5 The seeded pair

`cl::bleed_pair(preset, seed) -> (Color32, Color32)` picks two DISTINCT hues
from the preset's pool by the same xorshift discipline `random_mood_index`
established — no RNG dependency, and the two are never the same hue. The
shell rolls once at construction and pins from `--mood <n>` when it is given,
so a proof shot reproduces exactly. Randomness that a screenshot gate cannot
reproduce is worthless (program law 4).

### 3.6 The control

A new `crates/shell/src/providers.rs`: brain glyph + `Providers` + the `ai`
circle, the circle's DIAMETER equal to the cap-height of the P, laid left of
the brain per the mock. Hover drives one `animate_bool_with_time` at the
seeded dwell; the factor `t` lerps three things together — the two bleed
washes behind the control, the brain's stroke colour, and the label's ink.

It replaces the P1 placeholder cell in `status.rs` — same `Lane::Right`, same
`StatusCell` registry entry, so this is an entry swap, not a new mechanism.
The roster still supplies the truth: the control is present only when a
provider is activated, and its tooltip still names each one's limit.

Clicking it is V2's popup and does nothing yet — V1 ships the control and the
preset, and the click lands with the surface it opens.

## 4. Changes, in execution order

1. `crates/coldlight/tokens/motion.json` — `dwell` tier, `dwell` ease,
   `ink-bleed` effect.
2. `crates/coldlight/tokens/icons.json` — the `brain` entry.
3. `crates/coldlight/tokens/presets.json` — the `bleed` kind + `providers-ink`.
4. `crates/coldlight/build.rs` — `generate_presets` emits `BLEED_PRESETS`,
   cross-checking `effect` against motion.json and every hue against
   hues.json.
5. `crates/coldlight/src/lib.rs` — `BleedPreset`, `bleed_preset(id)`,
   `bleed_pair(preset, seed)`.
6. `crates/shell/src/providers.rs` — the control.
7. `crates/shell/src/status.rs` — swap the placeholder cell for it.
8. Unit pins (§6.3).
9. `tests/eguipilotai/providers-control.toml` — the bleed proved mid-bleed
   AND at full colour.

## 5. Boundaries

🚫 **Never touch**

- No duration, easing curve, hue or icon geometry hand-written into a widget.
  If it is a design value it is authored in `tokens/*.json` and generated.
- The roster. V1 reads `provider_roster()`; it never writes it and never
  changes who is activated.
- `dpl-coldlight` stays egui-only — no fabric dependency. The shell passes
  the roster in.

⚠️ **Ask first**

- Any change to `ThemePreset` / `MotionPreset` shape (three presets depend on
  them).
- Making the control do anything on click (that is V2).

**Out of scope**

- The popup, the tooltip-to-modal promotion, the tenure ticker — V2/V3.
- Presence probes for Claude/Grok/Gemini — A1.

## 6. Verification

1. `cargo test -p dpl-coldlight -p dpl-shell`.
2. `cargo clean -p` the touched crates, then
   `cargo clippy --workspace --all-targets -- -D warnings`.
3. New unit pins:
   - `the_dwell_tier_is_authored_in_the_window_the_operator_named` — 5.6 ≤
     `DUR_DWELL` ≤ 6.3, and min/max bound it.
   - `the_ink_bleed_effect_resolves_and_is_not_a_loop`.
   - `the_brain_icon_tessellated` — `icon_def("brain")` has geometry.
   - `the_bleed_pool_is_five_known_hues`.
   - `a_bleed_pair_is_two_different_hues` — over many seeds, never equal.
   - `the_same_seed_gives_the_same_pair` — the proof-shot guarantee.
   - `the_control_is_absent_when_no_provider_is_activated`.
4. `cargo green` → `green ✔`.
5. **The live proof:** `providers-control` captures the control cold and at
   full colour. A mid-bleed frame is not obtainable — see §8.
6. Regression: `platform-sees-providers` — the control replaces the cell, so
   its text assertions move with it.

## 7. Commit plan

Branch `feat/p1-provider-availability` (V1 stacks on P1, which is unpushed).
One commit:

```
feat(providers): the control and the ink-bleed preset (V1)
```

## 8. Evidence

### The tokens carry it, the widget only reads it

`motion.json` gained the `dwell` tier (5.9, bounded 5.6–6.3), the `dwell`
ease `[0.22,0.61,0.36,1.0]`, and the `ink-bleed` effect. `icons.json` gained
`brain` over the already-vendored `brain.svg`. `presets.json` gained a third
preset kind, `bleed`, and `generate_presets` now cross-checks its effect
against `motion.json` and every hue against `hues.json` — **an unknown hue
fails the build**, proved by hitting exactly that panic during the pass when
the extractor read the wrong shape. `EFFECT_COUNT` moved 18 → 19.

`crates/shell/src/providers.rs` holds no duration, curve, hue or geometry of
its own: the dwell window comes from `cl::motion::DUR_DWELL_MIN/MAX`, the
pair from the authored pool, the glyph from the registry. The `ai` circle's
diameter is expressed as a RATIO of the label size, so the operator's
"equal at the height of the P" survives a font-size change.

### Two bugs the proof caught

1. **The bleed was not animating.** The first run's 1.2s and 6.6s captures
   came back byte-identical. The animation id was derived from `ui.id()`, and
   the status bar builds its lane as a fresh child `Ui` every frame — so the
   id was new each frame, and egui treats an unseen id as already-arrived and
   snaps to the target. A six-second bleed was an instant swap. Fixed with a
   stable `egui::Id::new("dpl_providers_bleed")`, the same shape the rail's
   unfold uses, plus a `request_repaint` while mid-bleed so the wash advances
   on frames that would otherwise never run.

2. **A mid-bleed frame is not obtainable from this harness — at all.** Every
   capture runs under `MotionPolicy::Suppressed`
   (`egui-pilot-ai-runtime/src/lib.rs:1246`), which sets `animation_time = 0`
   and calls `clear_animations()`; the protocol names it a "motion-suppressed
   stable capture". So a hovered capture always lands on the settled end
   state however long the scenario waits. `MotionPolicy::Natural` and
   `FastForward` exist in the enum and nothing selects them.

   **This is a standing limitation of the evidence system, not of V1**, and it
   applies to every future motion pass — the eighteen existing effects have
   the same blind spot. Widening the harness is its own change and is NOT
   done here. What V1 does instead: prove the two ENDS visually, and pin the
   DURATION where it is checkable —
   `the_dwell_tier_is_authored_in_the_window_the_operator_named` on the token
   and `the_roll_is_seeded_and_lands_inside_the_authored_window` on the roll
   the widget actually uses.

### Pins (all green)

`cargo test -p dpl-coldlight -p dpl-shell` — 35 + 26 pass.

| pin | proves |
|---|---|
| `the_dwell_tier_is_authored_in_the_window_the_operator_named` | 5.6 ≤ dwell ≤ 6.3, bounds authored, and > 10× `slow` — a different CLASS of motion |
| `the_ink_bleed_effect_resolves_and_is_not_a_loop` | it settles at full colour; it does not cycle |
| `the_brain_icon_is_tessellated` | the vendored brain became real geometry |
| `the_bleed_pool_is_known_hues` | every pool hue resolves through the authored ramp |
| `a_bleed_pair_is_always_two_different_hues` | 512 seeds, never one colour |
| `the_same_seed_gives_the_same_pair` | the proof-shot guarantee |
| `the_pool_is_actually_drawn_from` | the seed ranges over the pool rather than returning one pair |
| `the_roll_is_seeded_and_lands_inside_the_authored_window` | the widget's own roll obeys the token |
| `the_dwell_is_a_passive_experience_not_a_transition` | nobody can quietly retune it to feel snappy |

### The live proof — PASSED

`tests/eguipilotai/providers-control.toml`, fixture `chat` (live runtime,
`--mood 0` pinning the pair and the dwell). Both captures scoped to `root`:

- `docs/proof/providers-control-cold.png` — muted line-art: the `ai` circle,
  the brain, and "Providers" in subtle ink, no wash.
- `docs/proof/providers-control-full.png` — the two-colour bleed behind the
  control, the brain and the `ai` circle carrying the first hue, the word at
  full strength.

`platform-sees-providers` still passes against the control that replaced the
cell. `cargo green` ✔.
