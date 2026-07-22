# W4 — the tap-to-edit editor over seed documents

Parent: docs/plans/authoring-architecture.md + THE UI LAW (layman-friendly,
touch-first, stack invisible). Ledger #105. Worktree: main.

## 1. Context

The operator's spec: "click the actual background of the slide and add a
mood where two gradients blend... add the pulse animation... This edits the
colocated file", under the UI law — "not some high dev useless UI", the
token/registry stack stays INVISIBLE, pickers show the thing itself, tap it
and it applies on the real slide instantly, no save ceremony, big undo.

W4 delivers the editable surface over the seed documents W1/W2 built. It is
a VIEW over the document: tap → select → swap a ref → write back the
colocated slide.json. Presets (W3) feed one of its trays.

## 2. Current state (ground-truthed this session)

- `dpl_doc`: Slide/Background/Block schema (refs never values), load/save
  (atomic tmp→bak→swap), validate. save() is the write-back engine.
- `dpl_slides::document::render(ui, rect, &Slide, now, t0)` paints a
  document but returns NOTHING — the editor needs the block rects to
  hit-test. `DocSlide` wraps a Slide + Clock.
- `cl::FONTS` (13, drawn in themselves on the fonts wall), `cl::MOODS`
  (10, gradient cards), `cl::LOOK_PRESETS` (5), ambience set
  {pulse,drift,rings}, ink set {strong,base,muted,subtle,tone}, fluid
  size ramp — all enumerable, all with existing "render as itself" wall
  code to reuse as pickers.
- Workbench hub: SEEDS/STUDIO/LIBRARY/TOKENS/TEST_LAB subroutes; loads
  seeds via dpl_doc::list_seeds().
- deny.toml single-version rule; no egui_dnd yet (RS2 defers drag reorder).

## 3. Target design

**New crate `crates/editor` (dpl-editor)** — one concern, its own crate
(hyper-modular law; RS2 named `crates/editor/src/ui/*`). Depends on
dpl-doc, dpl-slides, dpl-coldlight, egui.

- **Renderer returns a hit map.** Add to `dpl_slides::document`:
  `render_traced(ui, rect, &Slide, now, t0) -> RenderMap` where
  `RenderMap { background: Rect, blocks: Vec<(String /*block id*/, Rect)> }`.
  Existing `render` delegates (drops the map) — no behavior change, all
  W2 tests still pass. The stage paints exactly as the deck will; the
  editor overlays selection + trays on top.
- **`Editor`** (the widget): holds a working `dpl_doc::Slide`, the source
  `dir`, a `Selection { None | Background | Block(String) }`, an undo stack
  `Vec<Slide>` (the doc is a small value — snapshot on every edit, cap 50),
  and a dirty flag. `Editor::open(dir) -> Result<Self>` loads; `ui(ui)`
  renders + drives; edits call `mutate()` which snapshots→applies→saves
  (atomic, immediate — no save button) and clears redo.
- **Tap → select.** After `render_traced`, allocate an interaction over
  the stage; on click, hit-test topmost block rect → `Block(id)`, else the
  background rect → `Background`, tuned to `tap_min` hit slop. A selected
  block/background draws a tone selection ring (the only editor chrome on
  the stage).
- **Contextual trays** (bottom sheet over the stage, touch geometry —
  `tap_min` targets, swipeable `HoloScroll`-style rows, NO dropdowns):
  - **Text block** → three trays: FONT (the fonts-wall cards, each face in
    itself; tap = swap `font` ref), SIZE (the fluid ramp names as sized
    sample chips), INK (the 5 inks as tone swatches), and an inline text
    field for the words. Plain labels: "Font", "Size", "Ink", "Text".
  - **Background** → MOOD A + MOOD B (the moods-wall gradient cards, tap to
    set each of the two blend slots), MOTION (blend-drift; future motions
    when authored), AMBIENCE (None/Pulse/Drift/Rings toggle chips). Plain
    labels: "Background", "Colours", "Motion".
  - **Nothing selected** → a LOOK row: the 5 W3 presets as cards; tapping
    one applies the whole bundle (font on every text block + background) —
    "one-tap looks".
- **Undo** — one big obvious button in the editor toolbar (not a stage
  affordance); pops the snapshot stack and re-saves. No redo in v1 (kept
  honest; ledger if wanted).
- **Write-back is the proof.** Every tray tap = `dpl_doc::save(dir, slide)`
  → the colocated slide.json changes by exactly the tapped ref. A footer
  crumb shows the path + "saved" so the operator sees the file is the
  truth (dev-honest, not ceremony).
- **Workbench EDITOR subroute** — a seed picker (reuse list_seeds) → opens
  the `Editor` on that folder. SEEDS wall stays (read-only gallery);
  EDITOR is the editable surface.

**Deferred to W4b (RS2):** drag-to-reorder blocks + drag-from-tray via
egui_dnd; add-block / delete-block; multi-select. W4 is
select-and-restyle-existing — the operator's literal spec — kept green
without a new dep.

## 4. Changes, in execution order

1. `crates/slides/src/document.rs` — add `RenderMap` + `render_traced`;
   `render` delegates; unit test: a 3-block doc traces 3 block rects + a
   background rect inside the stage.
2. create `crates/editor/{Cargo.toml, src/lib.rs}` — `Editor`, `Selection`,
   trays, undo, write-back. Owned pickers reuse the wall-render patterns
   (font-in-itself, mood gradient card). Tests: open→mutate font→reload
   asserts the on-disk ref changed to exactly the new value and nothing
   else; undo restores; apply-look sets every text block's font.
3. root Cargo.toml — member + `dpl-editor` workspace dep.
4. `crates/hubs/workbench` — EDITOR subroute + dep; picker → Editor.
5. `cargo green`; exe proofs (editor open on a seed; a tray visible);
   commit; push.

## 5. Boundaries

🚫 No egui_dnd / new deps this pass (W4b). 🚫 No add/delete blocks. 🚫 No
episode tier (Program D). 🚫 wiredFRONT / Codex branch untouched. The
editor writes ONLY to the selected slide.json via dpl_doc::save (the
never-clobber-content law: seeds are git-tracked product, edits are
diffable).

## 6. Verification

- green exits 0 incl. the write-back unit test (open→edit→reload proves the
  colocated file changed by exactly one ref — the census's marquee
  requirement, asserted on disk not pixels).
- Real exe: `--route workbench/editor` opens the seed picker; opening a
  seed shows the live document + selection; a proof shot with a tray up.
- Honest note: tap→tray→disk is unit-proven; the live tap sequence is
  operator-hands or a future pilot scenario (gesture extension, RS2/D2) —
  stated, not faked.

## 7. Commit plan

`feat(editor): W4 — the tap-to-edit editor over seed documents` on main.
Adds crates/editor, render_traced, workbench EDITOR subroute. Pushed.
