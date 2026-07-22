# Port plan — the backdrop: two loops + authored random moods

Produced by /port-capture (backdrop), 2026-07-22. Salvage source read IN FULL
this session: `backdrop-stage.tsx` (55), `backdrop-picker.tsx` (72),
`backdrop-catalog.service.ts` (30), `backdrop.store.ts` (25),
`video-loop.tsx` (47).

## 1. Context — the operator's ratified spec (2026-07-22, verbatim)

*"THAT BACKDROP SYSTEM IS BULLSHIT AND NEVER WAS CORRECT ITS LITERALLY JUST
THOSE TWO OPTIONS THE DRAGON OR DPL THEN EACH CLICK SHOULD RANDOMLY PULL A
MOOD THAT IS NOT JUST ONE COLOR ... EVERY COLOR EVERY EFFECT NEEDS TO BE
AUTHORED SO IT EXISTS IN THE RUNTIME SO IT CAN PROPERLY BE COLOCATED WITH
THE FEATURE DOMAIN."*

And (same day): *"YOU MADE 2 RANDOM ANIMATIONS THAT SHOULD OF BEEN FOR
SLIDES ... making drift as a background and pulse when those are for slides
as backgrounds or container effects."*

So the OLD system (aurora base + off-tile + localStorage persistence) is
explicitly NOT carried — it "never was correct." The correct system:

1. Backdrop = **DPL loop or Dragon loop**, nothing else. App-wide, behind
   the content well on every route.
2. **Every click pulls a random MOOD**: an authored multi-hue treatment
   (gradient stops from the POOL + scrim alpha + accent) composed with the
   loop. Never a single color.
3. Moods are **authored tokens** → `tokens/moods.json` → build.rs →
   `moods_gen.rs` — the fourth generated source (hues, fluid, motion,
   moods). The backdrop feature consumes the runtime registry; nothing
   improvised in paint code.
4. **Drift/Pulse are slide/container ambiences ONLY** — off the mainpage,
   gallery-gated through the slides library.

## 2. Current state

**Old repo (read this session):** BackdropStage = aurora → VideoLoop
(cover, muted mp4) → scrim 70/35; catalog = exactly `dpl` + `dragon`
(+ poster each); picker = poster tiles + aurora tile; store = persisted id,
default "dpl". Carried truths: the two mp4s + posters, cover-fit, muted
loop, poster fallback, default DPL, scrim-for-legibility. NOT carried:
aurora tile/off state, localStorage persistence (session state is fine),
picker-in-topbar-as-tiles (replaced by the two-tile + mood-roll behavior).

**New repo:** `dpl-video` plays both mp4s (ffmpeg subprocess, cover UV,
poster fallback) — B1, keep. `crates/hubs/home` wrongly owns the backdrop
as subroutes dpl/dragon/drift/pulse — replace. `cl::aurora` — becomes the
poster-era fallback wash only if BOTH loops are missing (honest floor, not
an option). build.rs generates ramps/fluid/motion — moods joins it.

## 3. Target design

- **`crates/coldlight/tokens/moods.json`** — the authored mood registry.
  A mood = `{ name, stops: [[hue_id, step], ...≥3], scrim: 0..255,
  accent: hue_id }`. Colors resolve through the GENERATED pool at runtime
  (single source of color truth). Ships 10 authored moods, each ≥3 hues.
- **build.rs `generate_moods()`** → `moods_gen.rs`: `pub struct Mood`,
  `pub static MOODS: [Mood; N]`, `MOOD_COUNT`, lookup by name. Editing the
  JSON + `cargo build` IS the design build.
- **`crates/backdrop` (`dpl-backdrop`)** — the feature domain, colocated:
  `Backdrop` state = `{ loop: Dpl|Dragon, mood: usize }`. `click_loop(x)`:
  selects the loop AND re-rolls a random mood (never the same one twice in
  a row). Paint: mood gradient (multi-stop mesh, 135°) → the loop video →
  mood-tinted scrim at the mood's alpha. No ffmpeg → poster → mood wash.
- **Shell integration:** `Shell` owns `Backdrop`; the central well paints
  it under EVERY hub surface. The topbar carries the two poster tiles
  (DPL · Dragon); clicking the active one re-rolls the mood.
- **Gallery gate:** Library gains a "Moods" wall — every authored mood
  renders as a gradient swatch card with its name and stop count.
- **Slides:** Drift/Pulse stay ONLY as `Media::Anim` ambiences; a template
  demo carries Drift so the slide-ambience usage stays gallery-visible.
- Scale story: a new mood = one JSON entry; a new loop = drop mp4+poster in
  `assets/backdrops/` + one catalog line in the backdrop crate.

## 4. Changes, in execution order

1. CREATE `crates/coldlight/tokens/moods.json` (10 moods, stops from pool
   hue ids). 2. EDIT `crates/coldlight/build.rs`: `generate_moods()`
   (rerun-if-changed, data-only gen like motion). 3. EDIT coldlight lib:
   include `moods_gen.rs`; `Mood::stop_colors()` resolving via `ramp()`;
   tests (count, ≥3 stops, all hues resolve, accents resolve).
4. CREATE `crates/backdrop` — state, catalog (2 loops), `paint(ui, rect)`,
   `tiles(ui)` for the topbar, xorshift re-roll (time-seeded, no new dep),
   tests (click re-rolls and never repeats consecutively; loop switch
   keeps mood law; catalog is exactly 2).
5. EDIT shell: own `Backdrop`, paint in central under the hub, tiles in
   the topbar right (before full-bleed toggle).
6. EDIT home hub: subroutes → the old placeholder shape (mainpage hero
   only; Shows land with the content port); remove dpl-video dep, drop
   drift/pulse routes. 7. EDIT one slides template demo (callout_card) to
   `Media::Anim(Ambient::Drift)` — the slide-ambience gallery gate.
8. EDIT library hub: the Moods wall (pool-wall pattern).
9. Workspace wiring; delete dead home code paths (no shims).

## 5. Boundaries

- 🚫 Old repo read-only; loops stay the committed copies. 🚫 No color
  literals in the backdrop paint — pool + generated moods only. 🚫 No new
  RNG dep. ⚠️ Ask-first: adding/removing loops; changing mood JSON schema.
- Out of scope: mood persistence across launches; container-level mood
  application to fx-cards (arrives with the fx port).

## 6. Verification

`cargo test --workspace` green. Exe: `--route home/main --shot-frame 400`
shows a REAL loop frame under a mood treatment (multi-hue visible);
repeated boots with `--mood <n>`-less clicks can't be scripted, so the
re-roll law is unit-tested + two shots at different forced moods
(`--mood` harness arg) visibly differ; `library/moods` wall renders all
MOOD_COUNT swatches; drift demo visible in `library/templates` (callout).
Baselines updated in docs/proof/ with the reason in the commit.

## 7. Commit plan

One commit: `feat(backdrop): the real system — two loops, authored random
moods, app-wide` — includes this plan; baselines updated (backdrop is an
intended visual change everywhere). No push.
