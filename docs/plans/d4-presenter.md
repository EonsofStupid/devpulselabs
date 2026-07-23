# D4 — Presenter + sync (the teleprompter viewport)

Parent: roadmap Program D (#106). After D5 (ba30c61). Worktree:
`C:\localdev\dpl-main` on main.

## 1. Context

Roadmap: *"Second viewport teleprompter (beat list, active script,
up-next, timer) synced over the fabric (typed event, not
BroadcastChannel)."* LANES: EguiPilotAI multi-window extensions are
the CODEX seat's work — this pass authors the FEATURE and declares the
pilot deferral; their viewport-lifecycle slice (83b7841) is the
runway. The presenter consumes D5's scripts.

## 2. Current state (ground-truthed this session)

- `crates/fabric/src/lib.rs` (read in full): shell domain only —
  `ShellEvent` (Copy), broadcast·64, `publish_shell`/`subscribe_shell`.
  The stated design: per-domain enums on per-domain channels.
- `crates/deck/src/lib.rs` (read in full): `DeckHost{title, look,
  slides, steps, markers, pos, …}`; `go(pos)` is the ONE position
  mutation (ink clear + clock restart); `press()` the one key path;
  `DeckAction{None,Exit}`; the bottom toolbar's left cluster is
  Home · Navigator · Ink (`tool_button` + `track_button`); no fabric
  handle anywhere in the crate.
- `crates/hubs/present/src/lib.rs` (read in full): `PresentHub{fabric,
  deck, error}`; `open_dir` builds DeckHost + FullBleedRequest(true);
  `open_show` accepts bare id or `show/episode` (content wins);
  `ui()` renders the deck or the picker.
- `dpl_doc::script`: `load_script(slide_dir)`, `Script.beats`
  (`["setup", ...reveals]` — beats index IS the step index when parity
  holds). `Slide::step_count()`.
- The screenshot harness captures the MAIN viewport only.

## 3. Target design

- **Fabric, DECK domain**: `DeckEvent::Beat { index, step, slides,
  steps }` (all usize — Copy), `publish_deck`/`subscribe_deck`/
  `DeckReceiver`, broadcast·64. S1's EventLog observes the SHELL
  domain and is untouched (a session's deck track is a future pass).
- **DeckHost**: `fabric: Option<Fabric>` + `pub fn wire_fabric`;
  `go()` publishes Beat on every real move; `pub fn
  publish_position()` (the presenter's opening snapshot);
  `DeckAction` gains `TogglePresenter`; the toolbar's left cluster
  gains a "Cue" button returning it. (No hotkey this pass.)
- **PresentHub**: remembers `dir`; `presenter: Option<Presenter>`.
  `Presenter { rx, index, step, started: Instant, episode_title,
  rows: Vec<PrompterSlide{name, steps, script:
  Option<Script>}> }` built from the open episode dir; `drain()`
  applies Beat events (sync is fabric-only, one-way deck→presenter).
  `presenter_ui`: header (episode title · elapsed mm:ss · slide/beat
  counter) · the ACTIVE CUE big with its note (script beat at the
  step index; honest "no script for this slide" fallback) · the BEAT
  LIST (id chips, active lit) · UP NEXT (next beat's cue, else the
  next slide's first cue/name) · the script body, small, scrollable.
  Rendered in a real second viewport via
  `ctx.show_viewport_immediate` ("DevPULSE Presenter"); close-request
  or deck Exit drops it. TogglePresenter builds it AND asks the deck
  to `publish_position()`.
- **Harness** (the main-viewport-only screenshot constraint):
  `--presenter-drill <n>` — opens the show's presenter in PANEL mode
  (the SAME `presenter_ui` rendered in the main surface in place of
  the deck) after advancing the deck `n` beats through `press()` —
  every code path real except the OS window. `open_show` learns
  `template/<id>` (the workbench badge vocabulary) so the scripted
  SEED episode can drive the proof.
- **Pilot deferral declared**: the presenter-viewport scenario waits
  on the Codex seat's multi-window pilot targets; the panel mode +
  unit suite carry verification until then.

## 4. Changes, in execution order

1. **edit `crates/fabric/src/lib.rs`** — the deck domain (+ tests:
   typed round-trip, no-subscriber publish).
2. **edit `crates/deck/src/lib.rs`** — fabric field + wire_fabric +
   publish in `go` + `publish_position`; `DeckAction::TogglePresenter`
   + the Cue tool button; Cargo dep dpl-fabric; test:
   `advancing_publishes_the_beat_on_the_deck_channel`.
3. **edit `crates/hubs/present/src/lib.rs`** — `dir` retention;
   `Presenter`/`PrompterSlide` + `drain` + `presenter_ui` + the
   viewport mount; TogglePresenter/Exit handling; `open_show` gains
   `template/<id>`; `pub fn presenter_drill(&mut self, n)` (panel
   mode + n×ArrowRight + publish_position); tests: drain applies
   Beat; cue resolution (scripted slide, unscripted fallback,
   up-next).
4. **edit `crates/app/src/main.rs`** — `--presenter-drill <n>` flag →
   Boot → after `--present` wiring.
5. `cargo green`; proof (§6); commit; push backgrounded.

## 5. Boundaries

🚫 EguiPilotAI toolkit untouched (the Codex lane) — the pilot scenario
is DECLARED DEFERRED, not faked. 🚫 One-way sync only (presenter never
drives the deck this pass). 🚫 S1 EventLog untouched (deck-track
capture is its own pass). ⚠️ The loop player/backdrop and D2 nav math
byte-identical.

## 6. Verification

- `cargo green` exits 0 (fabric + deck + present suites).
- Real-exe proof: `--present template/how-ai-works --presenter-drill 2
  --mood 0 --screenshot docs/proof/d4-presenter.png --shot-frame 60` —
  the presenter surface at slide 2 (headline) beat "subtitle": the
  cue "Ten thousand tiny calculators…", its note, the beat list with
  subtitle lit, the timer, and UP NEXT naming the next slide.

## 7. Commit plan

`feat(deck): D4 — presenter + sync (deck fabric domain, teleprompter
viewport)` — fabric + deck + present + main + proof. Push
backgrounded. Program D closes to "D-complete" on the roadmap (D1–D5).

## 8. Executed — evidence

Execution matched the plan (one mechanical detour: clippy's
items-after-test-module moved the present tests to the file end). Green:
`the_deck_domain_round_trips_typed_beats` (and deck beats never wake
shell ears), `advancing_publishes_the_beat_on_the_deck_channel` (real
key path publishes; held positions publish nothing; the snapshot seam),
`the_presenter_follows_the_deck_over_the_fabric` (the whole loop
headlessly — latest beat wins, scripted cue + note resolve),
`up_next_walks_beats_then_slides_then_ends_honestly`. Proof:
docs/proof/d4-presenter.png — the presenter surface after a 2-beat
drill on the scripted seed: slide 2/4 · step 1, beat list with
`subtitle` lit, the cue "Ten thousand tiny calculators…" with its note,
UP NEXT "stat", the timer, and the inert prompter prose.

**PILOT DEFERRAL DECLARED**: the second-viewport pilot scenario waits
on the Codex seat's multi-window EguiPilotAI targets (their
viewport-lifecycle slice, 83b7841, is the runway); until then the
panel mode + this unit suite carry verification. Program D (#106) is
COMPLETE: D1–D5 all landed.
