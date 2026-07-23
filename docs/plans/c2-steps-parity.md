# C2 — Steps parity (OS-global hotkeys, Save card, guide-edit engine)

Parent: roadmap Program C (#108). After C1 (a8c63f0). Worktree:
`C:\localdev\dpl-main` on main.

## 1. Context

Roadmap C2: *"rdev OS-global hooks (Ctrl+Shift+S/9/X), global click hook +
window-following + click coords + do→done fingerprint dedup, seed-name from
window title, forced-metadata Save card, guide EDIT rewrite. Pilot: in-app
portions scenario-driven; OS-global portions engine-unit + drill (the pilot
never fakes OS hooks)."*

The middle of that list ALREADY landed with the engine-steps port: the
global click hook, window-following capture, click coords, and do→done
fingerprint dedup all live in `crates/engine-steps/src/lib.rs` (ground-
truthed this session). What C2 still owes:

1. **OS-global hotkeys.** The old app registered Ctrl+Shift+S (palette),
   Ctrl+Shift+9 (steps toggle), Ctrl+Shift+X (mark) via
   tauri-plugin-global-shortcut (`old lib.rs:12-38,105-116`). The new app
   only has APP-FOCUSED chords in `app/main.rs:483-497` ("OS-global is on
   the ledger"). The ratified design is rdev chords — the hook thread
   already sees every key globally.
2. **Forced-metadata Save card.** `stop_steps` today AUTO-FILES with blank
   meta ("the forced-metadata Save card is PV3" — this pass). The old flow:
   stop keeps the session PENDING, the Save card seeds its name from the
   first window title, forces kebab name (+auto `_v1`), ≥1 tag, and an
   episode/General destination, then `save_guide` files EXACTLY ONCE.
3. **Guide EDIT rewrite.** The old `commands/guide_edit.rs::update_guide`
   (prune/reorder/caption over guide.json, orphan-frame deletion, jail) —
   ported as an ENGINE function; the reviewer UI that drives it is W5.

Operator, mid-pass (2026-07-23): *"if we need to take a moment to refractor
some we can absolutely take a moment to ensure while in pre-release we
aggressively move and align our code"* — so this pass also SPLITS
`crates/pixvix/src/lib.rs` (1,435 lines, growing) into modules. Real moves,
no shims (the pre-release law).

## 2. Current state (ground-truthed this session)

- `crates/engine-steps/src/lib.rs` (622 lines, read in full): rdev listen
  installed once via `ensure_hook(recorder)` gated by `TRIGGERS: OnceLock`;
  the callback tracks MouseMove into `MOUSE_X/Y` atomics and, only while
  `running`, forwards `ButtonPress(Left)` / `KeyPress(Return)` triggers to
  a worker thread. `StepsRecorder{inner, running, sink}` with
  `start/mark/stop/save/discard/status`; `stop` → `StopResult{count,
  seed_title}`; `save` = rename-uniquify + guide.json, exactly once;
  `normalize_meta` (kebab + auto `_v1` + ≥1 tag + episode/General law),
  `normalize_meta_auto`. `StepsEvent::{State,Skip,Saved}` over an
  `EventSink`. rdev = "0.5" already in Cargo.toml. Hook installs only on
  first `start` — a boot-time install seam does not exist yet.
- `crates/pixvix/src/lib.rs` (1,435 lines, read in full): one file holds
  the Pixvix state machine, the palette paint fn (+`PaletteView`/
  `PaletteActions`/`rec_pill`/`pilot_chip`/`truncate`), and the frozen-
  frame editor (`RegionSelect`/`SnipMode`/`region_viewport`).
  `toggle_steps` (Ctrl+Shift+9's in-app handler) start/stops;
  `stop_steps` (lines 304-325) auto-files via `normalize_meta_auto` +
  `blank_meta` — no Save card. The steps sink only mirrors debug text into
  `steps_note`. `index_capture(path, kind)` writes sidecar + index with NO
  tags/show. Steps-tab hint says "(app focused)".
- `crates/app/src/main.rs:483-497`: the in-app chord block (Ctrl+Shift+9 /
  X via egui input) calling `pixvix.toggle_steps()/mark_step()` — becomes
  a DOUBLE-FIRE bug the moment rdev chords land; must be removed.
  Drill state machines (`--pixvix-snap` lines 518-528, `--session-drill`)
  are the pattern for the new hotkey drill.
- `crates/index/src/lib.rs`: `Record{tags, show_id, episode_folder, ...}`,
  `record_for_capture(path, root, kind, source)` (bare record),
  `sidecar::write`, `Index::upsert` — the Save card's metadata has a real
  home in the index row.
- Old repo (read this session): `src-tauri/src/lib.rs` (the three
  shortcuts + handlers), `commands/steps.rs` (save_guide files into INBOX
  and associates show/episode in the index — the artifact does not move),
  `commands/guide_edit.rs` (update_guide: validate frames BEFORE writing,
  renumber 1..N, rewrite guide.json, delete orphaned frames, refuse empty).

## 3. Target design

- **Hotkeys ride the existing rdev hook.** `Hotkey::{Palette, ToggleSteps,
  MarkStep}` + `HotkeySink`; the callback tracks Ctrl/Shift state
  (press/release of both variants) in atomics and maps
  Ctrl+Shift+S/9/X chords through a PURE `chord_hotkey(key, ctrl, shift)`
  (unit-testable without OS events). `install_hook(recorder, sink)` is the
  new boot-time public entry; `start()` keeps its lazy install as a
  fallback (no-op sink). Hotkeys fire regardless of `running` — that is
  the point. Chord keys do NOT also become step triggers (S/9/X never
  were; Return still is).
- **Pixvix wires the hook on its first frame.** A `hotkeys:
  Arc<Mutex<Vec<Hotkey>>>` queue; the sink pushes + `ctx.request_repaint()`
  (the clone moves into the closure) so a background app wakes. `ui()`
  drains: Palette → toggle `open`; ToggleSteps → `toggle_steps()`;
  MarkStep → `mark_step()` only while recording (old `mark_step_hotkey`
  semantics — no error spam when idle).
- **Stop opens the Save card instead of auto-filing.** `PendingGuide{count,
  name, tags, library, show, episode}`; `stop_steps` seeds `name` from
  `seed_title` via `seed_name()` (drop the trailing " - App" segment,
  kebab-ify) and opens the palette. The Steps tab renders the card while
  pending: name + tags fields, General ⇄ Episode destination (show/episode
  fields when Episode), Save + Discard. Save builds `CaptureMeta`, runs
  the engine's STRICT `normalize_meta` (a refusal keeps the pending
  session and lands in the status line), then `steps.save` +
  `index_guide()` — `record_for_capture` enriched with the card's tags and
  (when Episode) show_id/episode_folder, like the old `index_saved`. The
  guide files into the INBOX either way (old behavior; Gallery's Keep
  moves it later). Discard → `steps.discard()`.
- **Guide edit is an engine function.** `engine-steps/src/edit.rs`:
  `GuideStepEdit` (StepRec shape + `caption: Option<String>`),
  `update_guide(dir, steps) -> Result<u32, String>` — ported verbatim
  minus the Tauri/jail layer (the caller owns path scoping): frame names
  must be bare + existing, empty set refused, renumber 1..N, rewrite
  guide.json, delete orphans only after the write. The W5 reviewer UI
  consumes this later.
- **Module split (the refactor moment).** `crates/pixvix/src/`:
  - `lib.rs` — Pixvix state + engine-calling impl (snap/resnip/delay/
    video/session/steps + apply), `LastSnip`, `PendingSnip`, `VideoRec`,
    `PendingGuide`, fields `pub(crate)` so sibling modules paint them.
  - `palette.rs` — `Tab`, the pilot consts, `PaletteView`,
    `PaletteActions`, `palette()`, the Save card section, `rec_pill()`,
    `pilot_chip()`, `truncate()`.
  - `editor.rs` — `SnipMode`, `RegionSelect`, `impl Pixvix {
    region_viewport }` (an impl block in a sibling module — same crate,
    no shim).
  - `session.rs` — unchanged.
- Scale story: the next capture surface adds a variant + a palette arm;
  hotkeys add one line in `chord_hotkey`; guide tooling builds on
  `edit::update_guide` without touching the recorder.

## 4. Changes, in execution order

1. **edit `crates/engine-steps/src/lib.rs`** — add `Hotkey`, `HotkeySink`,
   `CTRL_DOWN`/`SHIFT_DOWN` atomics, `chord_hotkey`, `install_hook`
   (public, sink-carrying; the OnceLock guard stays), rewire `ensure_hook`
   → `install_hook(recorder, None)`; the rdev callback updates modifier
   state on press/release and emits chords through the sink BEFORE the
   `running` gate; add `simulate_toggle_chord()` (rdev::simulate, real OS
   input — the drill's sender). Tests: chord mapping (all three chords,
   modifier misses, plain keys).
2. **create `crates/engine-steps/src/edit.rs`** — the guide-edit port +
   `pub mod edit;` in lib.rs. Tests: reorder+prune renumbers and deletes
   orphans; empty set refused; separator/missing frame names refused;
   captions survive the round-trip.
3. **create `crates/pixvix/src/palette.rs` + `editor.rs`; edit `lib.rs`**
   — the module split (verbatim moves, `pub(crate)` visibility), then the
   C2 features on top: `hotkeys` queue + first-frame `install_hook` wiring
   + drain; `PendingGuide` + `seed_name()` + `stop_steps` → pending;
   the Save card in the Steps tab (pilot ids `pixvix.guide.name/tags/
   dest/save/discard`); `index_guide()`; hint text updated to "from any
   app". Tests: `seed_name` (title stripping + kebab), tag splitting.
4. **edit `crates/app/src/main.rs`** — DELETE the in-app chord block
   (lines 483-497); add `--steps-hotkey-drill`: at frame 30 spawn
   `simulate_toggle_chord()`, wait for `pixvix.steps_recording()`, fire
   the chord again, wait for not-recording, print the status, exit by
   verdict (timeout ~600 frames → exit 1). New Pixvix accessor
   `steps_recording()`.
5. `cargo green`; drills (§6); roadmap tick; commit; push backgrounded.

## 5. Boundaries

🚫 `engine-capture` untouched. 🚫 Video/Session tabs and `session.rs`
untouched (C3/S-ADR). 🚫 No new dependencies (rdev is already the
engine's; simulate is the same crate). 🚫 Never author into `/content`.
⚠️ The recorder's capture path (two-frame model, dedup) is settled —
C2 must not alter its semantics, only its surroundings. Out of scope:
the W5 reviewer UI over `edit::update_guide`; OS-hook pilot scenarios
(multi-window targets — the Codex lane, declared).

## 6. Verification

- `cargo green` exits 0 (fmt + clippy -D warnings + tests, including the
  new chord/edit/seed_name tests).
- `--steps-hotkey-drill` on the release exe exits 0: a REAL simulated
  Ctrl+Shift+9 (rdev::simulate → OS input → rdev::listen) starts the
  recorder, a second one stops it — the full OS-global loop, no fakes.
- `--pixvix-snap` still exits 0 (the C1 drill — regression guard).
- The Save card interactive flow awaits the operator's hands + the
  deferred multi-window pilot targets (the C1/D4 deviation, carried).

## 7. Commit plan

`feat(steps): C2 — steps parity (rdev OS-global hotkeys, Save card,
guide-edit engine, pixvix module split)`. Adds: engine-steps hotkey layer
+ edit.rs; pixvix palette.rs/editor.rs split + Save card; main.rs drill.
Push backgrounded.

## 8. Executed — evidence

Executed to plan, with two live findings folded in the same pass:

1. **A latent Start bug caught by the first drill run.** `blank_meta` shipped
   `library: false` with no show/episode, so `normalize_meta_auto` refused
   EVERY Steps start ("Pick an episode or the General library.") — the tab
   had never been drilled before. The auto path now defaults to the General
   library (the operator's real destination arrives via the Save card).
2. **The drill itself needed a settle between chords.** Firing the stop
   chord the moment `recording` flipped true interleaved the two simulated
   sequences — chord 1's modifier RELEASES landed inside chord 2 and broke
   its Ctrl state (drill timed out in phase 2, seen live). The drill now
   waits 60 frames between chords; both runs are recorded above as the
   deviation-and-fix trail.

Proven: `cargo green` exits 0 (workspace fmt + clippy -D warnings + all
tests, including the new chord-map, guide-edit, and seed-name tests);
`--steps-hotkey-drill` on the release exe exits 0 printing
"chord STARTED the recorder" then "chord STOPPED the recorder · status:
steps: no steps captured" — a REAL `rdev::simulate` Ctrl+Shift+9 through
the OS into `rdev::listen`, the chord map, the sink queue, and the
recorder, both directions (the empty-session refusal is the honest stop
path with zero steps). `--pixvix-snap` still exits 0 (C1 regression
guard). Deviation carried from C1/D4: the Save card's interactive flow
(palette viewport) sits outside the screenshot harness and its pilot
scenario waits on the Codex multi-window targets; the card's logic is
unit-tested (`seed_name`, strict-law refusal keeps the pending guide by
construction — `normalize_meta` runs before the session is taken).

The refactor moment (operator, 2026-07-23): `crates/pixvix/src/lib.rs`
split into `lib.rs` (state + engine calls, 890 lines), `palette.rs`
(paint + Save card), `editor.rs` (frozen-frame editor) — real moves,
no shims; child modules read the owner's private fields (Rust descendant
visibility), only child items consumed by `lib.rs` are `pub(crate)`.
