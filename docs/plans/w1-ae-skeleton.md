# W1 — the AE skeleton: the WiredFRONT editor rebuilt to the approved mock

## 1. Context

The operator approved `docs/design/mocks/wiredfront-editor-mock.html`
(v2.1, 2026-07-24) as THE spec: "After Effects, panel for panel," inside
the `wiredfront/editor` subroute. 3D/camera/tracker/motion-blur are
ratified LATER phases; Capture/Annotate/Narrate/Guide are tailored
sibling tabs, never the editor's spine; AI is ephemeral in-app + chat
(later pass); focus is WiredFRONT only — it produces the final
image/video that Workbench then inserts and enhances. W1 builds the
2D skeleton of that screen and rewires the salvaged organs (comp doc,
trim/drag math, sequence playback, stitch render) under it. Experience
gate rows 2/5/10 (`wiredfront-experience.md` §5) move with this pass.

## 2. Current state (read in-session)

- `crates/composer/src/lib.rs` (uncommitted R1x working tree, read in
  full): guide-DNA spine `Tab { Capture, Annotate, Edit, Narrate,
  Preview, Export }` + "Export .guide" chip; `Tool` (annotate tools) on a
  left rail; SCENES rail + media-bin cards; single-lane ruler timeline
  with trim/move/scrub (`Drag`, `hit_test`, `apply_drag`, `snap_tenth`);
  sequence playback (`tick_playback`, `active_layer_at`); stitch render
  thread (`RenderState`, `start_render`); save via `dpl_comp::save`.
- `crates/comp/src/lib.rs` API: `load(dir)`, `save`, `validate`,
  `Composition{fps,w,h,tracks}`, `Layer{id,source,start,in_pt,out_pt}`,
  `Source::{Artifact{rel},Element{id}}`, `reference()`.
- `dpl_index`: `recent(500, None)` records (kind video/image, poster
  convention); `dpl_video::clip::{ClipPlayer, clip_duration}`;
  `dpl_engine_capture::content_root()`; ColdLight vocabulary as used
  throughout the file.

## 3. Target design

The mock, literally, 2D slice: workflow tabs **Editor · Capture ·
Annotate · Narrate · Guide** (Editor default; Guide = the standalone
export flow, stubbed honestly). The Editor screen: **tool strip** (top:
Selection functional; Hand/Pen/Camera/Text + null/pre-comp/track +
m.blur/3D present with honest "lands with <pass>" tooltips) · **Project
panel** (left: preview well; COMPS = `inbox/*/comp.json`, click opens
via `dpl_comp::load`; FOOTAGE = index video+image rows, click-to-add
videos; ELEMENTS census; RENDERS = `inbox/*/render.mp4`, click appends)
· **Composition viewer** (center: stage + full transport ⏮◀▶▶|⏭ +
timecode) · **right stack** (Info: closable, default on, selected-layer
facts; Effects & Presets: search + honest empty states naming the
registry/elements passes) · **timeline** (AE layer OUTLINE left — one
row per layer, #, color chip, twirl-down timing properties, name; lanes
right — one lane per row, bars, CTI, trim/move/scrub salvaged, zoom) ·
**status row** (New/Save/Render + saved crumb + comp facts). Old
`Tool` stays as the ANNOTATE tab's tool set; `preview_tab` (guide
"simulate input") dies.

## 4. Changes, in execution order

1. `crates/composer/src/lib.rs` — REWRITE (one file, salvage listed in
   §2 kept verbatim or adapted): new `Tab` enum + blurbs per redlines;
   new `EditorTool` strip enum; fields gain `etool, info_open,
   fx_search, proj_sel, expanded: BTreeSet<String>, comps/renders/
   elements caches`; `Act` gains `OpenComp(PathBuf), ProjSel(usize),
   AddRender(usize), ETool, ToggleInfo, ToggleExpand(String), Step(f32),
   ToStart, ToEnd`; `ui()` routes Editor→`ae_screen` (tool_strip /
   project_panel / viewer_pane / info_pane / fx_pane / timeline_panel /
   status_row), Annotate keeps `annotate_tab`, others honest stubs;
   timeline becomes per-layer lanes (row model: layer row 26px + 20px
   property rows when expanded), hit-test row-aware, drag math reused.
2. Tests: keep the existing five green (public behavior unchanged);
   add row-model tests (expanded row count; per-lane hit mapping;
   `initial_tab` guide value).
3. Library un-crossing (r1x plan §4.3, unchanged): `wiredfront/library`
   → media shelf; traits wall → Workbench; `composer/src/library.rs`
   deleted. Second commit.

## 5. Boundaries

🚫 comp.json schema (transform/keyframes = the schema pass, ask-first) ·
🚫 `/content` authoring · 🚫 new deps · 🚫 `#[allow]` · 🚫 3D/camera/
tracker/motion-blur behavior (UI affordances only, honestly tooltipped) ·
out of scope: browse windows + drag-drop (R1y next), effects registry,
elements ingest, ephemeral ✦ + chat tools, autosave/undo (R1z).

## 6. Verification

1. `cargo test -p dpl-composer` exit 0. 2. `cargo clippy -p dpl-composer
--all-targets` clean. 3. `cargo green` on push. 4. Sequential proof
shots: `--route wiredfront/editor --window 1280x800` (tabs, tool strip,
project sections, viewer+transport, outline+lanes+CTI) and `1100x700`
fit check. 5. Feel bar: reads as AE at a glance, matches the mock.

## 7. Commit plan

Stay on `feat/r1x-real-timeline` (it holds the salvaged core this
subsumes). Commit 1: `feat(wiredfront): W1 — the editor rebuilt to the
approved AE mock (project · viewer · layer-outline timeline · tailored
tabs)`. Commit 2: the library un-crossing. Push, PR to main.

## 8. Evidence

Executed 2026-07-24 (same session as the mock ratification):

1. `cargo test -p dpl-composer` → **8 passed, 0 failed** (five salvaged +
   rows-model / trim-clamp-snap / topmost-active-layer new).
2. `cargo clippy -p dpl-composer --all-targets` → clean (one doc lint
   found and fixed during the pass; no `#[allow]`).
3. `cargo green` → **green ✔**, `target/release/devpulselabs.exe`
   rebuilt (two orphaned exe processes killed first — the known lock).
4. Sequential proof shots at real sizes (scratchpad `w1-*.png`):
   - `wiredfront/editor` 1280x800: five tailored tabs; tool strip
     (Select active + Hand/Pen/Camera/Text + null/pre-comp/track/
     m.blur/3D affordances); PROJECT (preview well + COMPS/FOOTAGE/
     ELEMENTS/RENDERS honest empty states); viewer + full transport +
     timecode; INFO closable ("x"); EFFECTS & PRESETS with honest
     pass-naming notes; TIMELINE header + zoom; status row.
   - 1100x700 fit check: no starvation, all panes render.
   - `wiredfront/library` 1280x800: the MEDIA shelf on its recessed
     plate (elements census + renders, honest empty states).
   - `workbench/library`: chip row gains the TRAITS wall.
   - Defects caught by the shots and fixed in-pass: missing font glyphs
     (▸ ▾ ✕ ▣ ◼ ◈ → "+/-", "x", monospace kind tags), transport/label
     overlap (duplicate resolution label removed), viewer empty-state
     overflow (shortened), media shelf missing its WELL plate (added).
5. Feel bar: the Editor reads as project/viewer/timeline (AE-shaped)
   at a glance; the guide-DNA spine and "Export .guide" chip are gone.
