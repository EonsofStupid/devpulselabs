# R1z — never lose work: undo/redo · autosave · reopen-last · the keyboard map

Status: EXECUTED 2026-07-25 (same session, after W5's PR #34 merged and
the tree returned to `main`). The operator ratified the full §3.5
keyboard map including the arrows/Home/End additions. §2 was re-grounded
against the MERGED tree before coding (the merge had grown `make_gif`
and the P1 data-root resolution — `content_root()` API unchanged, so the
design held; one API drift found, noted in §8).

## 1. Context

The experience gate (`docs/plans/wiredfront-experience.md`) names R1z as
"never lose work" — it must move friction rows **1** ("The editor never
opens empty-handed… reopening the editor restores your last comp") and
**8** ("Undo/redo | None | R1z"), with the ratified keyboard map: "Space
play/pause, Delete remove-selected" (§ line 112) and "Mistakes are free:
undo/redo across every timeline edit" (§ line 46). Today the editor has
NO undo (a mis-drag silently rewrites a trim), NO autosave (edits after
Save live only in RAM), opens EMPTY every boot, and every action is
mouse-only. This pass is pure editor-safety quality-of-life — no new
surfaces, no schema change.

## 2. Current state (ground-truthed 2026-07-25, W5 tree — RE-READ at execution)

- `crates/composer/src/lib.rs` (3227 lines): `Composer` holds
  `comp: Composition`, `dir: Option<PathBuf>`, `selected`, `preview`,
  caches (`palette/comps/renders/elements/packs/episodes/guides/
  session_rows`), `render`/`ingest`/`export` `Arc<Mutex<..>>` job states,
  `time/playing/px_per_sec/drag/expanded/info_open/fx_search/browsers/
  next_browser/save_name` + W5 reviewer fields. Facts that shape R1z:
  - **`Act::New` ⇒ `*self = Composer::default()`** (the `match act` in
    `ui()`) — this also severs the in-flight ingest/render/export Arcs
    (their threads write to the OLD state), drops open Browse windows,
    and dumps every cache. A pre-existing wart R1z must fix, because
    undo history must also survive New-as-reset-of-the-comp-only.
  - Comp mutations live in exactly: `add_clip` (Add/AddRender/video
    import route), `remove`, `apply_drag` (called per-frame during a
    drag; the drag STARTS at `resp.drag_started()` in `lanes()`, where
    `hit_test` returns `TrimIn/TrimOut/Move/Scrub`).
  - `save()` mints/keeps `dir` and calls `dpl_comp::save`; `open_comp`
    loads + resets `selected/preview/expanded/time/playing`.
  - `Default::default()` is called by the HUB (`WiredfrontHub` is
    `#[derive(Default)]`) AND directly by composer unit tests — so any
    boot-restore living inside `default()` would break test hermeticity.
    `DPL_GUIDE_PICK` proof-env is read there (touches `review` only).
  - TextEdits exist in the same surface (`save_name`, `fx_search`,
    reviewer captions) — raw key handling must respect
    `ctx.wants_keyboard_input()`.
  - `status_row` line 2 shows `next_hint` left + facts right (layers ·
    duration · WxH@fps · dir-or-"unsaved").
- `crates/comp/src/lib.rs`: `Composition` is pure serde data,
  `Clone + PartialEq` — snapshot undo is a cheap `comp.clone()`.
  `dpl_comp::save` is already crash-safe (tmp → bak → swap, LF pretty
  JSON) — autosave rides it unchanged.
- `crates/deck/src/lib.rs` 195–219: the house keyboard idiom — collect
  `i.key_pressed(..)` for an explicit key list, feed each through ONE
  pure `press` path (unit-testable).
- `crates/app/src/main.rs`: no eframe persistence (default
  `NativeOptions`, no storage). `Boot` struct + `App::new` wiring is the
  pattern for the new `--save-comp` seam (mirror `import_drill`).
- App-written state precedent under the content root: `artifacts.db`
  (index) and `elements/sources.json` (pack registry). No prefs file
  exists yet anywhere.
- `crates/hubs/wiredfront/src/lib.rs`: `composer` field constructed via
  derive-Default; `import_dir`/`pin_pack` delegate pattern at 821/827.

## 3. Target design

1. **Undo/redo (comp-scoped snapshots).** `undo: Vec<Composition>`,
   `redo: Vec<Composition>`, cap 100. `checkpoint()` pushes
   `comp.clone()` (skipped when identical to the stack top — dedup via
   `PartialEq`), clears `redo`, truncates the oldest past the cap.
   Called BEFORE each mutation: inside `add_clip` and `remove`, and at
   `drag_started` when `hit_test` returns Trim/Move (ONE undo step per
   completed drag, never per frame). `undo()`/`redo()` swap the comp
   across the stacks, then fix up derived state: clamp `time` to the new
   duration, drop a `selected`/`expanded` id that no longer exists, mark
   dirty. History is per-comp: cleared by `open_comp` and New.
2. **Autosave (debounced, honest).** `dirty: bool` +
   `edited_at: Option<Instant>` set by every mutation (including
   undo/redo). In `ui()`: when `dirty && dir.is_some()` and ≥2s since
   `edited_at`, write via `dpl_comp::save`, clear dirty;
   `request_repaint_after(2s)` while dirty so the debounce fires without
   input. Flush immediately on `open_comp`/New (leaving the comp) and in
   `Drop for Composer` (app exit). Unsaved-dir comps do NOT autosave —
   the NEXT hint already says "Save"; the facts line grows an honest
   `· unsaved edits` marker while dirty.
3. **Reopen-last.** New module `crates/composer/src/state.rs`:
   `EditorState { last_comp: Option<String> }` (serde, rel path) ↔
   `<content_root>/composer-state.json`, best-effort load/save
   (app-written only, the `sources.json` precedent; lives under /content
   so it is never committed). Written by `save()` and `open_comp()`.
   Restored ONCE, lazily, at the top of `ui()` (`restored: bool` field):
   only when the comp is pristine (`dir.is_none() && layers().is_empty()`)
   — so tests (which never call `ui()`), `--import-drill`, and scratch
   roots stay hermetic, and a restore never stomps real work.
4. **`Act::New` becomes `new_comp()`** — resets ONLY the editing fields
   (comp, dir, selected, preview, proj_sel, time, playing, drag,
   expanded, save_name, undo/redo, dirty) and keeps caches, Browse
   windows, and the ingest/render/export Arcs alive (fixing the severed-
   Arc wart). `*self = Composer::default()` disappears from `ui()`.
5. **The keyboard map** — pure `fn key_act(key: egui::Key, mods:
   egui::Modifiers, has_selection: bool) -> Option<Act>` (deck's
   one-path idiom): Space→TogglePlay · Delete/Backspace→Remove(selected)
   · Home/End→ToStart/ToEnd · ←/→→Step(∓1.0) · Ctrl+Z→Undo ·
   Ctrl+Shift+Z or Ctrl+Y→Redo. New `Act::Undo`/`Act::Redo` variants.
   Collected in `ui()` ONLY when `tab == Tab::Editor` and
   `!ctx.wants_keyboard_input()` (typing in a name/search/caption field
   never deletes a layer). Tooltip law: the timeline header hint line
   grows "Space plays · Delete removes · Ctrl+Z undoes".
6. **Harness seam `--save-comp <name>`** (the reopen-last real-exe
   proof): after `--import-drill`, saves the comp under `<name>` through
   the SAME `save()` path (sets `save_name`, calls `save()` — which now
   also writes `composer-state.json`). `Boot` + `App::new` + hub
   delegate, mirroring `import_drill`.

## 4. Changes, in execution order

1. `crates/composer/src/state.rs` — CREATE: `EditorState`,
   `load(root) -> EditorState` (absent/corrupt → default),
   `save(root, &EditorState)` (best-effort, pretty JSON). Tests:
   round-trip in a scratch root; corrupt file loads default.
2. `crates/composer/src/lib.rs` — EDIT:
   - `mod state;` + struct fields `undo`, `redo`, `dirty`, `edited_at`,
     `restored` (+ `Default` init).
   - `checkpoint()`, `undo()`, `redo()`, `mark_edit()` (dirty +
     edited_at), `new_comp()`, `maybe_restore()` (the once-lazy reopen),
     `maybe_autosave()`, `flush_save()` (+ `Drop` impl calling it).
   - `add_clip`/`remove`: `self.checkpoint()` first + `mark_edit()`
     after. `lanes()` `drag_started`: checkpoint when hit ≠ Scrub;
     `mark_edit()` on `drag_stopped` with a non-Scrub drag.
   - `save()`/`open_comp()`: write `state::save(root, ..)`; `open_comp`
     also clears undo/redo/dirty. `Act::New` arm → `self.new_comp()`.
   - `Act::Undo`/`Act::Redo` variants + arms; `key_act` free fn + the
     collect loop at the top of `ui()`; facts-line `· unsaved edits`;
     timeline hint text extended.
   - Tests: checkpoint dedup + cap; undo/redo round-trip restores spans;
     undo prunes dangling selection/expanded + clamps time; redo cleared
     on fresh edit; `new_comp` keeps caches (`packs`/`palette` stay
     `Some`) and clears history; `key_act` full matrix incl. "no
     selection → Delete is None" and Ctrl+Z vs plain Z.
3. `crates/app/src/main.rs` — EDIT: `save_comp: Option<String>` in
   `Boot`, parse `--save-comp <name>`, wire after `import_drill` in
   `App::new`; hub delegate `pub fn save_comp(&mut self, name: &str)` in
   `crates/hubs/wiredfront/src/lib.rs` → composer
   `pub fn save_comp(&mut self, name: &str)`.
4. `CLAUDE.md` harness flags list gains `--save-comp <name>`;
   `docs/STATE.md` tip; `wiredfront-experience.md` §5 rows 1 + 8 marked
   moved by R1z.

## 5. Boundaries

🚫 Do NOT touch the Guide tab / W5 reviewer / dpl-guide surfaces (fresh-
landed by the parallel session; reviewer Apply already has its own
two-tap + one-undo-step semantics via `engine-steps::edit`). 🚫 No new
deps (std `Instant`, workspace serde only). 🚫 `comp.json` schema
unchanged — undo is editor state, never persisted. 🚫 /content law:
`composer-state.json` is app-written operator data — never committed,
never authored by hand. 🚫 The pinned D: trees stay read-only. ⚠️
Ask-first: any keys beyond the map in §3.5 (the arrows/Home/End additions
are already beyond the two ratified keys — flag them to the operator at
review, remove on objection). Out of scope: undo for reviewer/caption
edits, layer vertical reorder, zoom/J-K-L keys, eframe-storage window
state.

## 6. Verification

1. `cargo test -p dpl-composer` green (existing 16+ plus the §4 tests).
2. `cargo green` ✔ via the pre-push hook.
3. Real-exe SEQUENTIAL drill, scratch root (`$env:CONTENT_ROOT`):
   - Run 1: `devpulselabs.exe --import-drill <clips> --save-comp
     r1z-proof --route wiredfront/editor --window 1280x800 --mood 0
     --screenshot r1z-run1.png` → assert `inbox/r1z-proof/comp.json` and
     `composer-state.json` naming it exist.
   - Run 2 (same root, after Run 1 exits): `devpulselabs.exe --route
     wiredfront/editor --window 1280x800 --mood 0 --screenshot
     r1z-run2.png` → the shot shows the RESTORED layers + the
     "Reopened…" status: row 1 of the friction audit, proven on the exe.
4. Keyboard + undo on the real exe: operator drill (add two clips →
   drag a trim → Ctrl+Z restores it → Space plays → Delete removes →
   Ctrl+Z brings it back); `key_act` matrix is unit-proven. Autosave:
   edit a SAVED comp, wait 2s, kill the exe → reopen shows the edit
   (comp.json mtime moved, no Save tapped).
5. Proof shots at REAL window sizes (1280x800 + one more), sequential
   runs only.

## 7. Commit plan

Branch `feat/r1z-editor-safety` off `main` AFTER W5's PR merges (verify
`git log --oneline -1` shows the W5 merge before branching). Commits:
(1) `feat(composer): R1z — undo/redo + the editor keyboard map +
new_comp` (state.rs test scaffold included); (2) `feat(composer): R1z —
autosave + reopen-last + --save-comp seam + docs`. PR → merge → rebuild
`target\release\devpulselabs.exe` → `docs/STATE.md` — the session-end
law, no exceptions.

## 8. Evidence

Executed 2026-07-25 on `feat/r1z-editor-safety` off main (post-W5/P1).

1. `cargo test -p dpl-composer` → **27 passed** (the W3/W4/W5 suite plus
   the R1z nine: undo/redo round trip through an add + a trim drag ·
   checkpoint dedup + the 100 cap · undo prunes dangling selection/
   twirls and clamps the playhead · fresh edits clear redo · `new_comp`
   resets the comp but KEEPS the session caches · Drop flushes pending
   edits to a real `comp.json` · the full `key_act` matrix (Space, Del/
   Backspace gated on selection, Home/End, arrows, Ctrl+Z/Ctrl+Shift+Z/
   Ctrl+Y, bare-Z unbound, Ctrl+Space unbound) · `state.rs` round trip ·
   corrupt state reads as default).
2. `cargo green ✔` — release exe rebuilt (3m37s).
3. Real-exe reopen drill (scratch `CONTENT_ROOT`, SEQUENTIAL):
   - Run 1: `--import-drill <2 clips> --save-comp r1z-proof` →
     `inbox/r1z-proof/comp.json` exists; `composer-state.json` =
     `{"last_comp": "inbox/r1z-proof"}`.
   - Runs 2 + 3 (cold boots, no flags): the editor opens WITH the comp —
     status "Reopened inbox/r1z-proof — your last comp.", both layers on
     the timeline, facts `2 layer(s) · 9.0s · … · inbox/r1z-proof`, and
     the new hint line "Space plays · Del removes · Ctrl+Z undoes".
     Shots: `docs/proof/r1z-reopen-{1280x800,1600x1000}.png`. Friction
     row 1 proven on the exe; row 8's bindings are unit-proven (the
     keyboard/undo feel drill is the operator's, listed in §6.4).
4. One deviation from §4: egui 0.35 names the focus guard
   `egui_wants_keyboard_input()` (not `wants_keyboard_input`) — the
   compiler named it, the guard is otherwise as planned.
5. Autosave on the real exe rides the same `dpl_comp::save` the drill
   exercised; the debounce decision + Drop flush are unit-proven (test
   `drop_flushes_pending_edits_to_disk` writes through the real writer).
