# D2 — DeckHost: the show machine's runtime surface

Parent: docs/plans/roadmap.md Program D. Ledger #106. After D1 (episode
tier) + W3b (look cascade) — the deck is born over the final document
shape. Worktree: `C:\localdev\dpl-main` on main.

## 1. Context

An episode can be authored, validated, and viewed as a wall — but not
PRESENTED. D2 ports the old deck runtime's beat grammar and full-bleed
surface into egui: the operator taps a show and steps through it on the
75" — tap/keyboard/edge-zone navigation, progress rail with chapter
ticks, toolbar, navigator overlay, chrome-hide with the home-dot +
edge-reveal escape hatches (touch law: tap-min, no hover-only).

## 2. Current state (ground-truthed this session, all files read in full)

**Old repo (read-only, port source):**
- `deck-nav.service.ts` — THE beat math, pure: `stepCount = reveals+1`
  (step 0 = setup, arriving never pre-reveals), `advance` (next step,
  else next slide step 0, holds at end), `retreat` (prev step, else
  previous slide's LAST step, holds at {0,0}), `canAdvance`/`canRetreat`
  by comparing positions, clamp helpers.
- `deck-host.tsx` — composition: position is source of truth; keyboard +
  swipe + edge taps move one BEAT; ArrowUp/Down jump whole slides;
  chrome-hidden = per-slide override OR toggle; when hidden: a bottom 7%
  invisible strip restores chrome, a Home dot (bottom-left, 40% opacity)
  exits — "ALWAYS a way back on a touchscreen".
- `edge-nav.tsx` — full-height 12vw tap zones left/right, chevron pills,
  disabled at 30% opacity, chevrons fade with chrome (zones stay).
- `progress-rail.tsx` — top strip (h-1), fill `= (index+1)/total`,
  chapter markers as 1px ticks at `marker.index/total`, skip index 0.
- `useKeyboardNav.ts` — ←/PgUp retreat beat; →/PgDn/Space advance beat;
  ↑/↓ whole slide; G navigator; A annotate (D3); H chrome. Bails in
  inputs; Space bails on focused controls.
- `runtime-toolbar.tsx` — bottom bar: Home / navigator / presenter (D4)
  · deck title + slide title center · `NN / NN · step k/K` counter ·
  annotate (D3) · chrome toggle. `pointer-events-none` when hidden.
- `slide-navigator.tsx` — full overlay, card grid (`NN · kind`, title),
  tap → `{index, step 0}` + close.

**New repo:**
- `dpl_doc` (post-W3b): Slide sparse + Theme cascade + `episode_slides`
  flat walk + folder contract. NO reveals field yet — every slide is
  currently a single step.
- `dpl_slides::document` — `render/render_traced(ui, rect, slide, look,
  now, t0)`; resolve-once; entrance `rise` staggers all blocks from t0.
- `dpl_shell::Shell` — `full_bleed` mode exists: content owns the
  viewport, Esc + floating chip exit; toggled today only from the
  topbar button. Shell OWNS the fabric, publishes only.
- `dpl_fabric` — `ShellEvent { RouteChanged, PixvixSummon }` broadcast;
  "a new consumer follows this file's shape".
- `dpl_hub` — Hub trait `{ spec(), ui(ui, route) }`; a new hub is one
  line in `crates/app/src/main.rs` (`App::new` builds hubs BEFORE the
  fabric today — the Present hub needs the fabric, so the fabric moves
  above the hub registry).
- Harness flags in main.rs: `--route hub/sub`, `--screenshot`,
  `--shot-frame`; HubId/SubId leak-once pattern for CLI routes.

## 3. Target design

**A new hub: PRESENT** (`crates/hubs/present`, registered one line) —
the show machine's front door. Subroute SHOWS: the episode picker
(cards, like Workbench's, but this one PLAYS). Tapping a show:
1. builds a `DeckHost` from the episode folder (episode + look +
   ordered slides via `episode_slides`; a slide that fails to load
   becomes an honest labeled plate — degrade, never crash),
2. publishes `ShellEvent::FullBleedRequest(true)` on the fabric — the
   shell (now a subscriber, draining its receiver each frame) drops all
   chrome; the deck owns the viewport.

Leaving (toolbar Home or the home-dot) publishes
`FullBleedRequest(false)` and returns to the picker. Esc still exits
full-bleed via the shell's existing path; the deck then simply runs
inside the well — graceful, not an error.

**Reveals enter the schema** (the beat grammar needs them):
`slide.json` gains optional `"reveals": ["subtitle", ...]` — an ORDERED
list of block ids revealed one per step. `step_count = reveals.len()+1`;
step 0 shows only non-reveal blocks (setup); step k shows reveals[0..k].
Blocks not listed are always visible. Validator: a reveal ref must name
an existing block id, no duplicates. Absent = single-step slide (all
callers never branch — the old service's exact property).

**The nav math ports verbatim** into `dpl_deck::nav` — pure functions
over `steps: &[usize]` (per-slide step counts), mirroring the TS
1-for-1: `clamp_index`, `clamp_step`, `advance`, `retreat`,
`can_advance`, `can_retreat`, `Position { index, step }`. Exhaustive
unit tests are the port proof (hold-at-end, retreat-to-LAST-step,
roll-over, single-step slides, empty deck).

**Rendering steps:** `dpl_slides::document::render_step(ui, rect,
slide, look, step, now, t0) -> RenderMap` — one body shared with
`render_traced` (which passes `step = usize::MAX`, everything
revealed). Layout measures ALL blocks (no reflow between steps);
unrevealed blocks aren't painted. The LATEST-revealed block rises with
the position clock (each beat restarts the DeckHost clock, so arriving
at a slide plays the slide entrance and each reveal plays one rise).

**DeckHost chrome** (all ColdLight, all tap-min, PilotIds registered):
- Progress rail: top, 4px, tone fill `(index+1)/total`, chapter ticks.
- Edge zones: 12% width, full height, always tappable; chevron pills
  painted only while chrome shows; 30% alpha when can't move.
- Toolbar (bottom, scrim + hairline): Home · navigator toggle ·
  center = episode title (eyebrow) + chapter·slide name ·
  `NN / NN · step k/K` · chrome eye. Hidden = not painted, not
  interactive (except via the reveal strip).
- Navigator: full overlay, card grid (`NN · chapter`, slide name),
  tap → `{i, 0}`; G toggles; close chip.
- Chrome-hide: H or the eye; hidden → rail/toolbar/chevrons gone; an
  invisible bottom-7% strip restores; the home-dot (bottom-left, 40%
  alpha circle, full alpha on hover/press) exits the show.
- Keyboard: exactly the old map (minus A/annotate, D3; minus
  presenter, D4).

**Fabric grows its second event + first consumer** —
`ShellEvent::FullBleedRequest(bool)`; `Shell` subscribes at
construction and drains `try_recv` at the top of `ui()`. The fabric
file's own shape, followed.

**Harness:** `--present <episode-id>` boots Present, opens the show,
requests full-bleed — so the REAL exe proves the deck surface.

Scale story: a new show is still just a folder; the deck adds zero
per-slide or per-show code. Reveals are authored data validated by the
same doctor; the beat math never branches on slide kind.

## 4. Changes, in execution order

1. **`crates/doc/src/lib.rs`** (edit): `Slide` gains
   `#[serde(default, skip_serializing_if = "Vec::is_empty")] pub
   reveals: Vec<String>`; `validate` adds: each reveal names an existing
   block id (`blocks.{id}` issue "reveal names unknown block"), no
   duplicate reveal ids; `Slide::step_count(&self) -> usize` =
   `reveals.len() + 1`. `resolve` copies it (clone covers it). Tests:
   reveal validation (unknown + dup named), sparse save omits empty
   reveals, step_count.
2. **`seeds/episodes/how-ai-works/headline/slide.json`** (edit): add
   `"reveals": ["subtitle"]` — the seed show proves a real beat.
3. **`crates/slides/src/document.rs`** (edit): `render_step` public;
   `render_traced` delegates with `usize::MAX`; body: skip painting a
   block whose reveal position `>= step`; the block at reveal position
   `step-1` animates ENTER_RISE from t0 (others static once revealed);
   measurement unchanged (all blocks). Test: at step 0 the revealed
   block's rect is still reported in RenderMap (hit-testing stability)
   but a step-0 vs full render differ only by that block's paint —
   assert via `render_step` not panicking at every step 0..=count and
   RenderMap block lists being identical across steps.
4. **`crates/fabric/src/lib.rs`** (edit): add
   `FullBleedRequest(bool)` variant (Copy stays). Test: subscriber sees
   it.
5. **`crates/shell/src/lib.rs`** (edit): `Shell` gains
   `events: broadcast::Receiver<ShellEvent>` (subscribed in `new`
   BEFORE moving fabric in); top of `ui()`:
   `while let Ok(ev) = self.events.try_recv() { if let
   ShellEvent::FullBleedRequest(on) = ev { self.full_bleed = on; } }`.
   Test: publishing the event flips full_bleed on the next frame
   (kittest one step).
6. **`crates/deck/`** (create, `dpl-deck`):
   - `src/nav.rs` — the verbatim math + exhaustive tests (documented
     against the old service's doc comments).
   - `src/lib.rs` — `DeckHost::open(dir) -> Result<Self, String>`
     (episode + look + slides + markers `(index, chapter title)` +
     per-slide honest load errors as plates), `pub fn ui(&mut self,
     ui) -> DeckAction` where `DeckAction { None, Exit }`; internal:
     position, chrome_hidden, navigator_open, clock restarted on every
     position change; keyboard map; edge zones; rail; toolbar;
     navigator; home-dot + reveal strip; PilotIds
     `devpulse.deck.{next,prev,home,navigator,chrome,rail}`.
     Tests: open over a scratch episode; keyboard advance mutates
     position per the math (kittest); exit action from home.
7. **`crates/hubs/present/`** (create, `dpl-hub-present`): HubId
   "present", label "Present", tone (Violet if free — check cl::Tone
   variants; else Cyan), subroute SHOWS ("The shows — tap one to
   present it full-bleed."); `PresentHub::new(fabric: Fabric)`; picker
   (list_episodes cards: title, id, slide count) → `DeckHost::open` +
   `FullBleedRequest(true)`; `DeckAction::Exit` → drop host +
   `FullBleedRequest(false)`; `pub fn open_show(&mut self, id)` for the
   harness.
8. **Root `Cargo.toml` + `crates/app/Cargo.toml` + `main.rs`** (edit):
   workspace members + deps; in `App::new` move `Fabric::new()` ABOVE
   the hub registry, pass a clone to `PresentHub::new`; register the
   hub (one line); add `--present <id>` flag → `shell.navigate(present/
   shows)` + `hub.open_show(id)` + `shell.set_full_bleed(true)` (direct,
   no fabric race in the harness path). NOTE: App::new holds hubs in the
   Vec — expose open_show via the hub before boxing.
9. **Docs**: roadmap D2 entry gets its "landed" cross-ref; this plan's
   execution notes.
10. `cargo green` → exe proofs → commit → backgrounded push.

## 5. Boundaries

- 🚫 No ink/annotate (D3), no presenter window/sync (D4), no scripts
  (D5), no editor reveal-authoring UI (W4b tray later — reveals are
  hand-authored data this pass).
- 🚫 The old repo stays read-only; Codex checkout untouched; work in
  `C:\localdev\dpl-main` (verify `git branch --show-current`).
- 🚫 No third fabric domain; one new variant + one consumer, in-shape.
- ⚠️ The pilot scenario `deck-golden-path.toml` requires the waiter +
  gesture toolkit extensions (roadmap Pilot Law list). This pass lands
  PilotIds + exhaustive nav-math units + kittest keyboard-drive; the
  TOML scenario lands WITH the toolkit extension (named deferral, same
  honesty as W4's tap-flow).
- ⚠️ `slide.chrome === "hidden"` per-slide override and `allowAnnotate`
  from the old schema are NOT ported yet — they belong to the authored
  show-kind doctrine pass (TODO(doctrine) already in dpl-doc).

## 6. Verification

1. `cargo green` exits 0 (nav math + reveal validation + shell
   full-bleed event + deck tests all in).
2. Nav-math unit suite proves the verbatim port: hold-at-end,
   retreat-lands-on-last-step, roll-over, step0-setup, single-step
   slides, clamp on out-of-range input.
3. The seed show's headline slide validates clean WITH its reveal and
   the episode doctrine stays clean.
4. Real exe: `--present how-ai-works --screenshot
   docs/proof/d2-deck.png` — the deck full-bleed on slide 1 with rail,
   toolbar, chevrons. `--present how-ai-works --shot-frame 40` variant
   captures after settle. Picker proof: `--route present/shows
   --screenshot docs/proof/d2-present-picker.png`.
5. Kittest: a synthetic ArrowRight sequence walks the whole seed show
   (index/step trace equals the expected beat list, INCLUDING the
   headline reveal beat) and holds at the end.

## 7. Commit plan

`feat(deck): D2 — DeckHost: beat grammar, full-bleed deck surface, Present hub`
on main; push backgrounded. Adds: crates/deck, crates/hubs/present,
reveals in dpl-doc, render_step, FullBleedRequest + shell consumer,
`--present` harness flag, seed reveal, proofs.
