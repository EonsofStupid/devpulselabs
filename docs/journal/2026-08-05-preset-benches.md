# 2026-08-05 — the preset benches, and the three defects they exposed

## The ask

> "others get presets now but we need to mock them first to truly capture 3
> unqiue diverse presets for that ancestral that truly captures that material
> and that seed"

Eight ancestors × three presets. `brutalist` already had four ratified
(`neo · hacker · retro-os · vibron`); the other seven had none.

## Done

**`docs/design/mocks/preset-bench.html` — 8 ancestors × 3 presets, GENERATED.**
Not transcribed. `docs/design/mocks/_preset-bench.gen.py` reads
`tokens/matrix.json` + `tokens/hues.json` and fills `_preset-bench.tpl.html`,
so the mock cannot drift from what `build.rs` generates. The OKLCH ladders are
computed with the real ColdLight lightness/chroma curves.

The generator is a **gate, not a renderer**. It refuses to emit:
- a coordinate its ancestor forbids (checked against `native` + `allowed`),
- an unknown hue,
- a set of three where two resolve to **identical paint** (radius, edge width,
  fill, ink, edge, shadows, back-glow, tints all compared).

It also *reports* — see the material cap below — rather than quietly shipping
a weaker set.

### Three defects in the authored data, found by building it

1. **`bloom` shipped a blur.** Aero's native atmosphere authored `blur: 12`
   and `blur: 16`, violating doctrine law 6 ("No blur, ever") *and*
   `coldlight-matrix.md` §11.4, which had already ruled aero no-blur —
   "gloss via banded gradients and hard specular bands". Re-authored as a
   painted **ring falloff** (concentric hard rings at falling alpha) plus two
   hard directional shadows. No atmosphere in the matrix carries a non-zero
   blur any more; a `python` sweep over `atmospheres[].shadows[].blur`
   confirms it.

2. **The gloss band was invisible.** `sheen` and `bloom` painted their
   specular band as `{surface: OVERLAY, alpha: 90}` — a *dark* surface over a
   *dark* fill. Measured: `OVERLAY@90` over `RAISED` lands 4/255 from the
   fill. Aero's entire gloss story was dead data. Both now use
   `{ink: STRONG}` (30 for sheen, 20 for bloom), and the band stops sharp at
   46% because a specular band is a band, not a wash.

3. **A third of the material axis is duplicate data — UNFIXED, needs a
   ruling.** `matte ≡ raw`, `glass ≡ metallic`, `paper ≡ clay` are
   byte-identical in `fill`/`ink`/`edge_color`. Nine named materials paint
   six. Consequence: **`bauhaus`, `swiss` and `constructivist` cannot produce
   three materially-distinct presets at all** — their whole material palette
   is `matte`/`paper`/`raw`, which is two things wearing three names. Their
   sets differentiate by structure and atmosphere instead, which works (see
   the shots) but is a workaround.

### Two mock-side bugs, fixed

- `pill`'s radius 999 was applied to the panel, turning the whole shell into
  an ellipse. `pill` means pill-shaped **controls**; a container radius is
  clamped (`min(radius, 14)`).
- The specular tint sat on a `::before` at `z-index:-1`, i.e. **behind** the
  element's own `background` — painted, then covered. It is now composited
  into the background stack, which is also how the egui painter stacks it.

## The 24 coordinates

| ancestor | the three |
|---|---|
| bauhaus | Primary · Werkstatt · Vorkurs |
| art-deco | Chrysler · Machine Age · Salon |
| brutalist | Neo · Hacker · Retro-OS *(the ratified ones, restated as coordinates)* |
| swiss | Grid · Akzidenz · Offset |
| aero | Hacker · Gloss · Tactile |
| memphis | Bacterio · Tawaraya · Carlton |
| constructivist | Rodchenko · Agitprop · Lissitzky |
| editorial | Broadsheet · Cover · Masthead |

`aero/Hacker` is the operator-named `aero + hacker` for Workbench.
**Names are PROPOSED, not ratified** — per the clay-is-a-button ruling, a port
inference stays proposed until the operator rules on it.

## Exact next step

1. **Operator ruling on the material collision** (finding 3). Proposal:
   separate `raw` from `matte` (unfinished/béton — darker, desaturated, not a
   saturated tone-400 fill) and `clay` from `paper` (clay is extruded and
   takes a directional double-shadow; paper is fibrous and flat). That is a
   vocabulary change, so it is not being made unilaterally.
2. **Operator ruling on the 21 proposed preset names.**
3. Once ruled: author them into `tokens/presets.json` and bind through
   `tokens/roles.json`, which already resolves role → treatment per surface.

## Landmines

- `docs/design/mocks/preset-bench.html` is **GENERATED — never hand-edit.**
  Edit `_preset-bench.tpl.html` or `_preset-bench.gen.py` and re-run
  `python docs/design/mocks/_preset-bench.gen.py`.
- The generator resolves the repo root from `__file__`, so it works from any
  clone, but it must stay three levels under the root.
- `hues.json` has two entries (`white`, `black`) that are component literals
  with no `hue` key — any sweep over the hue pool must skip them.
- Raising `bloom`'s halo alphas (16/8 → 34/18) was needed because 6%/3%
  before falloff is under the visible floor on the dark base. If the surface
  ramp is ever lightened, re-check.

## Verify

```
python docs/design/mocks/_preset-bench.gen.py     # gate passes, 24 coordinates
cargo test -p dpl-coldlight                       # 48 passed
cargo clippy -p dpl-coldlight --all-targets       # clean
```
Then open `docs/design/mocks/preset-bench.html` and look at it — the shell is
identical in every column, so any two columns that look alike are a defect.
