# Port plan — the slide catalog (blocks · templates · infographics)

Produced by `/port-capture slides` protocol, 2026-07-21. Salvage source:
`C:\localdev\devpulse labs` (read-only). Target: this repo. Every source file
cited below was read IN FULL in the authoring session.

---

## 1. Context

The operator's requirement (2026-07-21): *"do not dare rush this but methodically
upgrade their visuals and get them authored properly clean for rust/egui"*, on top
of the standing directives *"the library of slides need to be expanded built so
they are properly blocks, and templates"* and *"gets my visuals and slides fully
built so i can later easily edit them via workbench"*. The 75" streaming/teaching
prop is the end surface: touch-driven, presenter-paced slides that must survive
the Rust rewrite with their visuals **upgraded**, not merely translated.

The prior app's slide catalog is a mature, doctrine-laden system: a three-tier
component library (elements → templates → infographics) behind a name registry,
driven by manifest data, with hard-won interaction law (presenter pacing, rest
states, gesture guards, keyboard paths). None of it has landed in the new app yet
— the Workbench/Library hubs are placeholders. This plan captures ALL of it, with
an explicit disposition per item, and fully specs Phase S1.

## 2. Current state

### 2.1 Salvage source (all paths under `src/domains/` unless noted)

**Registry core** — `registry/types/registry.types.ts`: a registration =
`{name, componentClass: static|split|animation|interactive, propsSchema (arktype),
render(props, ctx), description, formHints?}`; `SlideLook {tone?, density?}`
reaches components via a `data-fx-tone` wrapper. `registry/register.ts`: 19
side-effect registrations + a `custom/<episode-id>/<name>` slot (currently empty,
`custom/register.ts` is `export {}`).

**Frame** — `slide-catalog/frame/slide-frame/slide-frame.tsx`: the ONE outer
shell; align center|top|stretch, `bleed` kills the safe-area padding (px-f8/py-f8).

**Elements (3)** — `eyebrow` (THE mono/uppercase micro-label, 0.2em/0.24em
tracking, tone-able; recurred 9× before extraction); `callout-badge`
(neutral surface chip | live brand chip with dot); `media-figure` (picture +
caption + credit, composes media/PictureStill).

**Templates (15)** — the triad `<name>.tsx` + `.types.ts` (+ `.variants.ts`):
- `title-card` (static): eyebrow + display title (leading 0.95) + subtitle; align left|center.
- `big-number` (static, "the receipt"): eyebrow + wall-sized value in tone color + label + footnote; density comfy|compact; `tone:"ink"` renders ink-strong.
- `bullets-list` (static + `visibleCount` steps seam): numbered 2-digit bullets, label+detail, per-item tone; hidden items keep their box (stable rest state).
- `reveal-list` (interactive): tap-to-reveal; self-paced (whole panel is the tap target) OR controlled via `visibleCount`; counter eyebrow `n / total`; rest = all revealed.
- `choice-poll` (interactive): predict-then-reveal A/B; truth gets tone surface + ring, loser dims to 40/60%; LAND line fades in; "the room's call" marker.
- `quiz-panel` (interactive): single (radio) / multi (checkbox) commit-then-explain; correct = glass+ring, wrong picked = 50%; "Lock it in" button; "Called it"/"Not quite".
- `stats-grid` (static): auto-fit card grid; flip = crossfade front/back; per-stat tone.
- `callout-card` (static): toned card, left tone bar + sheen, kicker eyebrow, title, body, CTA.
- `code-block` (static): raw code panel, line numbers in a gutter, tone-washed line highlights, copy button (1.4s "Copied" rest).
- `comparator-panel`/`split` (split): A/B columns, LEFT=then/hype RIGHT=now/truth; accent ring on a side; registration wraps body lines.
- `cost-meter` (interactive): SEND taps fill a meter bar + exchange log; superlinearity lives in data; call-it line, then the flip lands ("+tiny cost") with wash+ring highlight.
- `predict-rank` (interactive): call-the-next-word rounds; hidden ranking (`?` rows at 40%) → reveal animates ranked bars (chart-rise, 90ms stagger); rounds counter; LAND line.
- `drag-match` (interactive): chips → buckets; correct locks (reward-pulse), wrong shakes+returns; full keyboard pick-up/place path (Enter/arrows/Escape) with aria-live announcer; hit-test at release.
- `media-hotspots` (interactive): still + pulsing tone-tinted markers at `{x,y}%` + tooltips.
- `still-zoomable` (static): pinch-zoom/pan/double-tap-reset still; reset button appears at scale>1.

**Infographics (6 + engine)** — `chart-frame.tsx` (960×540 logical viewBox stage,
margins {24,32,56,32}, title eyebrow + caption); `tone-series.util.ts` (paint =
tone slots; `TONE_CYCLE = [brand, cyan, lime, magenta, violet, coral]`);
`diagram-map.ts` (name → component (+ dataSchema); degrade-to-defaults on bad
data, never an error slide). Diagrams: `donut-share` (arcs innerR=0.62·R,
padAngle .02, cornerRadius 6; center readout; labels at 1.45·centroid; staggered
fade 140ms), `bar-compare` (band scale, rise-in 120ms stagger, display value over
bar in paint color, label under in mono uppercase), `line-trend` (1-2 series,
monotone curve, area wash fade + line draw-in, end-of-line series label),
`probability-bars` (prompt with blank, ranked rounded bars on wash tracks, top
bar gets halo; magenta joke tail), `stack-layers` (two labeled columns of
fx-card layers, bottom layer glass-strong), `cost-meter-face` (static semicircle
arc r=80, track+fill dasharray, needle + readout + eyebrow label).

**Doctrine carried by the code** (readme + remarks): presenter law (every state
a rest state, nothing auto-advances); gesture guard (`data-no-swipe` on every
interactive region so taps never page the deck); density knob changes spacing/
type only, never color; tone rides slots with per-item overrides; motion always
has a reduced-motion escape; promotion rule (≥2 recurrences before a pattern
becomes an element); interactive-budget counts class=interactive only.

### 2.2 Target repo (as of commit 7b73e32)

Workspace crates: `hub` (Hub/HubSpec/Route + P0 placeholder), `coldlight`
(generated 53×13 pool, `Tone` {Cyan,Lime,Violet,Amber,Magenta}, paint vocabulary
`scatter_frame/card_frame/gradient_tint/sheen/diffusion/seam/frame_overlay`,
`Fluid::of(rect)` with 36 generated methods incl. the full type ramp
`fs_eyebrow..fs_display_xl`), `fabric`, `shell`, `hubs/*` (placeholders except
library's pool wall), `app` (registry + `--screenshot/--route` harness).
`docs/proof/` holds 2 baselines. No slide, block, or vector code exists.

## 3. Target design

Two new crates, faithful to hyper-modular colocation:

- **`crates/vector` (dpl-vector)** — pure geometry + animation timing, no
  colors, no registry knowledge. epaint has no arc primitive, so this crate owns:
  `ring_sector` (triangle-strip mesh between inner/outer point rings),
  `polyline_stroke` (round-capped thick line as quad strip + cap fans),
  `area_fill` (quad strip down to a baseline), `sample_monotone` (curve
  interpolation for line-trend), and `anim` easing (`ease_out`, `sweep(now,
  delay, dur)` clamped 0..1). Unit-tested geometry.

- **`crates/slides` (dpl-slides)** — the catalog. Module tree mirrors the old
  tiers, colocated: `frame` (the stage: safe-area padding via `Fluid::of`,
  align modes), `blocks/` (eyebrow, badge, …), `templates/` (one module per
  template, owning its props struct + demo data + retained interaction state),
  `infographics/` (chart stage 960×540 logical → fitted into any rect;
  tone cycle drawing from the pool), `registry` (TemplateId newtype,
  `TemplateClass {Static, Split, Animation, Interactive}`, `TemplateSpec`,
  `demos() -> Vec<Box<dyn SlideTemplate>>` — ONE line per template).
  `trait SlideTemplate { spec() -> &'static TemplateSpec; ui(&mut self, ui) }`.
  Tone gains `Coral` (hue `coral` is in the pool). Manifest-data-driven
  authoring (serde props) arrives with the deck runtime pass — demo data is
  typed Rust defaults carrying the same EOS content the old defaults carried.

- **Gallery gate**: Library hub gains subroute `templates` rendering every
  registered template LIVE (interactive, real state) with its spec label/class —
  the slide analog of the pool wall. Adding a template = its module + one
  registry line; it appears in the gallery automatically.

**The visual upgrade** (why egui beats the CSS originals): crisp mesh-rendered
arcs/bars with per-vertex gradient tint instead of flat CSS fills; black
diffusion scatter under cards instead of box-shadows; sweep/rise animations
driven by `ui.input(|i| i.time)` at compositor precision; everything sized by
`Fluid::of(stage)` so the same slide is correct in a Workbench preview pane and
on the 75".

**Phases** (each its own commit + verify):
- **S1 (fully specced here):** vector crate · slides crate core (frame, registry,
  blocks eyebrow/badge/stat) · templates `title-card`, `big-number` · all 6
  infographics · Library `templates` gallery · tests + baselines. The donut
  sweep+tap is the vector engine's birth certificate.
- **S2:** the interactive tranche — reveal-list, bullets-list, choice-poll,
  quiz-panel, stats-grid, callout-card, code-block, split/comparator,
  cost-meter, predict-rank, drag-match (incl. the keyboard path law).
- **S3:** media-backed — media-figure block, still-zoomable (multi_touch
  pinch), media-hotspots; `video`/`video-loop` land with the engine port (P2)
  since frames come from the ffmpeg subprocess (ADR-009).
- **S4:** serde props + manifest loading (content-as-data returns; episodes on
  disk stay untouched until then), Workbench editing per the studio vision.

## 4. Disposition table (46 items — nothing dropped silently)

| # | Item | Disposition | Phase / target |
|---|------|-------------|----------------|
| 1 | slide-frame | REAUTHOR | S1 `slides::frame` |
| 2 | eyebrow element | REAUTHOR | S1 `blocks::eyebrow` |
| 3 | callout-badge | REAUTHOR | S1 `blocks::badge` |
| 4 | media-figure | REAUTHOR | S3 (image loading) |
| 5 | title-card | REAUTHOR | S1 `templates::title_card` |
| 6 | big-number | REAUTHOR | S1 `templates::big_number` |
| 7 | bullets-list | REAUTHOR | S2 (visibleCount seam carried) |
| 8 | reveal-list | REAUTHOR | S2 (self-paced + controlled modes) |
| 9 | choice-poll | REAUTHOR | S2 |
| 10 | quiz-panel | REAUTHOR | S2 (single/multi commit-then-explain) |
| 11 | stats-grid | REAUTHOR | S2 (flip crossfade) |
| 12 | callout-card | REAUTHOR | S2 (tone bar + sheen → paint vocab) |
| 13 | code-block | REAUTHOR | S2 (gutter, highlights, copy) |
| 14 | comparator-panel + split reg. | REAUTHOR | S2 (then→now law carried) |
| 15 | cost-meter | REAUTHOR | S2 (meter+log+flip) |
| 16 | predict-rank | REAUTHOR | S2 (shares bar anim w/ probability-bars) |
| 17 | drag-match | REAUTHOR | S2 (pointer + keyboard path BOTH) |
| 18 | media-hotspots | REAUTHOR | S3 |
| 19 | still-zoomable | REAUTHOR | S3 (multi_touch) |
| 20 | video registration | REAUTHOR | engine pass P2 (ADR-009 frames) |
| 21 | video-loop registration | REAUTHOR | engine pass P2 |
| 22 | replay registration (rrweb) | RETIRE | ratified: sessions video-first |
| 23 | chart-frame | REAUTHOR | S1 `infographics::stage` |
| 24 | tone-series util | REAUTHOR | S1 `infographics::ToneCycle` (+Coral) |
| 25 | diagram-map | REAUTHOR | S1 (subsumed by template registry) |
| 26 | donut-share | REAUTHOR | S1 (sweep + tap re-roll — the proof) |
| 27 | bar-compare | REAUTHOR | S1 (rise stagger) |
| 28 | line-trend | REAUTHOR | S1 (draw-in + area wash) |
| 29 | probability-bars | REAUTHOR | S1 |
| 30 | stack-layers | REAUTHOR | S1 |
| 31 | cost-meter-face | REAUTHOR | S1 (arc + needle) |
| 32 | registry.service/.types | REAUTHOR | S1 `slides::registry` |
| 33 | propsSchema (arktype) | REAUTHOR | S4 serde (typed demo data in S1) |
| 34 | formHints | REAUTHOR | S4 (Workbench editor forms) |
| 35 | custom/<episode> slot | REAUTHOR | S4 design note (registry supports) |
| 36 | SlideLook tone/density | PORT-VERBATIM (semantics) | S1 `Density`, tone params |
| 37 | TONE_CYCLE order | PORT-VERBATIM | S1 (brand→cyan→lime→magenta→violet→coral) |
| 38 | presenter law (rest states) | PORT-VERBATIM (doctrine) | S1 crate docs + upheld per template |
| 39 | gesture guard | REAUTHOR | S2 (egui Sense containment; deck nav later) |
| 40 | density knob (spacing only) | PORT-VERBATIM (semantics) | S1 |
| 41 | per-item tone override | PORT-VERBATIM (semantics) | S1 data model |
| 42 | reduced-motion escape | REAUTHOR | S2 (animations snap when disabled) |
| 43 | promotion rule (≥2) | PORT-VERBATIM (doctrine) | S1 crate docs |
| 44 | keyboard path (drag-match) | PORT-VERBATIM (behavior) | S2 |
| 45 | degrade-to-defaults (bad data) | PORT-VERBATIM (doctrine) | S4 (serde fallback) |
| 46 | gallery fixtures (EOS demo data) | PORT-VERBATIM (content) | S1+ demo() defaults |

## 5. Changes, in execution order — PHASE S1

1. **create** `crates/vector/{Cargo.toml, src/lib.rs}` — dpl-vector; dep egui
   (for Mesh/Pos2/Color32 only). `ring_sector`, `polyline_stroke`, `area_fill`,
   `sample_monotone`, `mod anim {ease_out, sweep}`. Tests: sector triangle
   count/winding, sweep clamping, monotone endpoints.
2. **edit** workspace `Cargo.toml` — add members `crates/vector`, `crates/slides`.
3. **edit** `crates/coldlight` — add `Tone::Coral` (hue `"coral"`); update the
   pool-membership test.
4. **create** `crates/slides/{Cargo.toml, src/lib.rs}` — deps coldlight, vector,
   hub, egui. `registry.rs` (TemplateId, TemplateClass, TemplateSpec, demos()),
   `frame.rs`, `blocks/{eyebrow,badge,stat}.rs`,
   `templates/{title_card,big_number}.rs`,
   `infographics/{stage.rs, tone_cycle.rs, donut_share.rs, bar_compare.rs,
   line_trend.rs, probability_bars.rs, stack_layers.rs, cost_meter_face.rs}`.
   Tests: registry complete (8 demos), spec ids unique, tone cycle order.
5. **edit** `crates/hubs/library` — add subroute `templates`; render the live
   gallery (ScrollArea of stages, each `demos()` entry with label/class chip).
6. **verify + baselines** per §6; **commit** per §7.

(S2/S3/S4 get their own fully-specced continuation of this document before
execution — same file, appended, re-approved.)

## 6. Boundaries

- 🚫 Old repo read-only; content/episodes untouched; no rrweb code ports.
- 🚫 No new color literals — pool only; no sizing literals where a fluid token
  exists; never blur.
- 🚫 No video crates (ADR-009); no serde/manifest layer in S1 (S4).
- ⚠️ Ask before changing the `Hub` trait or coldlight's public API surface
  beyond adding `Coral`.
- Out of scope for S1: everything listed under S2/S3/S4.

## 7. Verification (S1)

- `cargo test --workspace` exits 0 (existing 17 + new vector/slides/registry
  tests).
- Kill strays → `cargo build` rc 0 → exe mtime advanced.
- `--screenshot proof-library-templates.png --route library/templates` exit 0;
  PNG shows: gallery header, every S1 template rendered (title-card,
  big-number, 6 infographics), donut mid-sweep or at rest with all slices
  colored from the cycle, no □ glyphs, no blank panes.
- Baseline: commit shot to `docs/proof/library-templates.png` (new baseline);
  re-shoot `library/components` + `wiredfront/pixvix` and diff against
  existing baselines (no regression).

## 8. Commit plan

One S1 commit:
`feat(slides): the catalog returns — vector engine, blocks, first templates, 6 infographics, live gallery`
adds `crates/vector`, `crates/slides`, library gallery route, baselines. No push.
