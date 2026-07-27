# Catalog — a diverse library of options

Operator ask (2026-07-27): *"create some elements and templates and blocks for
me to have a true diverse library of options."* Additive authoring against the
established catalog architecture (`crates/slides`): blocks → templates →
infographics, each registered ONE line in `registry::demos`, each rendered
live by the Library gallery (the gallery gate).

## What landed

**Blocks (2 new reusable units)** — both meet the ≥2-template promotion rule:
- `divider::rule` — a toned rule with a bright accent segment. Used by
  `quote` + `section_divider`.
- `bar::bar` — a pill track with a toned proportional fill. Used by
  `compare_vs` + `checklist`.

**Templates (7 new, spanning all four classes):**
- `quote` (static) — pull-quote: oversized mark, display line, divider,
  attribution.
- `section_divider` (static) — giant tone section number + display title + rule.
- `agenda` (static) — numbered running order, tone chips, staggered fade.
- `compare_vs` (split) — head-to-head A/B: strength meters + points + a VS chip.
- `feature_grid` (split) — 2×2 feature cards, glyph badge + title + one-liner,
  diagonal fade.
- `timeline` (animation) — milestones on a baseline that draws in, dots pop in
  order, labels alternate above/below.
- `checklist` (interactive) — tap to tick items one at a time; each pops on
  `reward-pulse`; a meter fills.

**Infographics (2 new, on the vector engine):**
- `gauge_dial` (animation) — a 180° dial, needle sweeps to a value over a toned
  arc, big center readout.
- `radial_progress` (animation) — a progress ring that sweeps to a percent, big
  center number.

Census: **19 → 28** (20 templates + 8 infographics).

## Doctrine adherence
- Presenter law: every animation is a pure clock function (settles once, then
  rests); interactives are rest states between taps.
- Fluid sizing throughout (`Stage::of` / `ChartStage::fit`) — correct in a
  preview pane and on the kiosk.
- Tone rides each slide; color resolves through the pool. Density/type only.
- Tofu law: glyphs are font-safe ASCII (no box tofu).

## Proof
- `cargo green ✔` (fmt + clippy -D + tests + release build).
- NEW `registry::every_demo_renders_at_start_and_at_rest` — the gallery gate
  with teeth: every one of the 28 demos PAINTS at animation-start (alpha 0,
  fresh clock) and at rest, headless, without panicking (catches arc math /
  NaN rects / galley errors). Plus `checklist` advance/clamp unit test.
- Visual: the Library templates wall reports "28 templates registered";
  `timeline`, `compare_vs` spot-checked live (dots + alternating labels; panel
  + strength meter + VS). Solo previews are inspector-cramped — full-width
  in-app / on-deck.

## Follow-ups
- Seed slides in the empty `split`/`animation`/`interactive` class folders
  (doc-layer starters) — the document renderer is template-agnostic, so seeds
  showcase Text/Media blocks, not these code layouts; lower priority.
- More infographics (bar race, sparkline row) and blocks (avatar, key-cap) as
  new templates recur the patterns.
