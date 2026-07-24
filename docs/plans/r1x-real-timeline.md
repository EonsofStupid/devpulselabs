# R1x — the REAL editor core: ruler timeline · playhead · trim · sequence preview · thumbnail media bin

## 1. Context

Operator (2026-07-24, after seeing the r1u shots): "HOW IS ANY OF THIS
USEFUL FOR STITCHING VIDEOS, EDITING IMAGES — IN WHAT WAY DOES IT AT ALL
RESEMBLE AFTER EFFECTS." They are right. The Edit tab is a clip LIST:
text chips for media, a chip row for a "timeline", a preview that plays
one file. An After-Effects-shaped editor means: a media bin with
THUMBNAILS, a timeline with a TIME RULER, clips as BARS proportional to
duration on track lanes, a PLAYHEAD, click-to-seek scrubbing, drag-edge
TRIM, and a preview that plays the SEQUENCE. This pass builds that core.
WiredFRONT = raw video/images (the operator's After Effects). The
ColdLight ingredient wall does NOT belong here (two-libraries law) — its
mount moves to a follow-up fix in this same branch.

## 2. Current state (read in-session)

- `crates/composer/src/lib.rs` — `Composer{comp,dir,selected,preview,
  palette,render,tab,tool,status}`; `edit_tab` (tools left · preview ·
  inspector · chip "timeline" · bottom toolbar); `palette_pane` = text
  chips; `timeline_pane` = horizontal chip row + ^v/x buttons;
  `preview_pane` plays the selected layer's source only. `select()` opens
  `ClipPlayer` per layer. Layout idiom: manual rects + `new_child`.
- `crates/video/src/clip.rs` — `ClipPlayer{duration}`: `open(path)`,
  `texture(ctx)`, `toggle()`, `playing()`, `ended()`, `position()->f32`,
  `seek(f32)` (used by the session player at shell level).
- `crates/comp` — `Layer{id,source,start,in_pt,out_pt}`, `span()`,
  `end()`, `Composition::duration()`; times are seconds.
- `dpl_index::Record` — `rel_path, kind, name, seconds, poster_rel_path`.
  Video posters exist as sibling jpgs (poster convention).
- `crates/hubs/wiredfront` — `library` subroute currently mounts
  `dpl_composer::library_ui` (the MISPLACED ColdLight wall).
- `crates/hubs/workbench/src/library.rs` — the REAL ingredient library
  (Templates/Colors/Presets/Fonts/Moods walls) already exists.

## 3. Target design

`Composer` gains playback state: `time: f32` (the playhead), `playing:
bool`, `px_per_sec: f32` (zoom), and an edit-drag state
`drag: Option<Drag>` (`Trim{id, edge}` | `Move{id, grab_offset}`).

1. **Media bin** (left rail, replaces the chip palette): a 2-col grid of
   cards — poster/image THUMBNAIL (the gallery's `file:///` law), name,
   duration badge; tap adds to the end of the timeline (as today).
2. **Timeline** (the real one): a RULER row (marks + labels every fitted
   second step) over one LANE per track; each layer paints as a BAR at
   `x = start·pps`, `w = span·pps`, tone fill + name + duration, selected
   = brighter + handles. Interactions: tap bar = select; drag bar EDGE
   (8px zones) = trim `in_pt`/`out_pt` (clamped: ≥0.25s span, in_pt≥0,
   out_pt≤source duration when known); drag bar BODY = move `start`
   (clamped ≥0, 0.1s grid snap); tap/drag ruler or empty lane = SEEK the
   playhead. Zoom chips (- / +) clamp pps 20–240. The playhead is a
   voltage line over ruler+lanes at `time·pps`.
3. **Sequence preview**: the playhead drives playback. Each frame, the
   active layer = the one whose `[start, end)` contains `time` (topmost
   track wins; gaps show the well). The preview player follows the active
   layer's SOURCE at `in_pt + (time - start)`: switching layers reopens
   the player; scrubbing seeks it. When `playing`, `time` advances by
   real dt and stops at comp end. Transport (bottom toolbar): Play/Pause
   chip + `mm:ss / mm:ss` readout.
4. **Library un-crossing** (same branch, second commit):
   `wiredfront/library` stops mounting the ColdLight wall and becomes the
   MEDIA shelf stub-with-truth: the elements census (`content/elements/`,
   honest empty state naming the R1c ingest) + rendered comps list.
   `composer::library.rs`'s trait wall moves into
   `workbench/library` as a new TRAITS section (the one wall Workbench
   lacked); the rest of the file (presets/fonts/moods walls) is DELETED —
   Workbench already renders those from the same registries.

## 4. Changes, in execution order

1. `crates/composer/src/lib.rs` — add playback/zoom/drag state + `Act`
   variants (`Seek(f32)`, `TogglePlay`, `Zoom(f32)`, live drag applies
   directly via `&mut self` split — drags mutate comp in the timeline fn
   using deferred acts: `TrimTo{id, in_pt, out_pt}`, `MoveTo{id, start}`);
   rewrite `palette_pane` → `media_bin` (thumbnail cards);
   rewrite `timeline_pane` → ruler + lanes + bars + playhead + drag logic;
   rewrite `preview_pane` → sequence playback (active-layer resolution +
   follow/seek + advance when playing); toolbar gains the transport.
2. `crates/composer/src/lib.rs` tests — active-layer resolution (gaps,
   boundaries), trim clamps, move clamps/snapping, seek clamp to comp
   duration.
3. Library un-crossing: `crates/hubs/workbench/src/library.rs` gains the
   TRAITS section (live `compose()` swatches moved from composer);
   `crates/hubs/wiredfront/src/lib.rs` `LIBRARY` arm → new
   `media_library()` in the wiredfront hub (elements census + renders);
   `crates/composer/src/library.rs` DELETED; composer's `pub use` removed.

## 5. Boundaries

- 🚫 `/content` (the elements census only READS), generated files, tokens.
- 🚫 No new crates/deps; ClipPlayer is the only decoder (per-frame
  compositing of overlapping tracks is a later pass — sequential playback
  is this pass's stitch preview).
- ⚠️ Ask first: changing comp.json schema (this pass does NOT).
- Out of scope: effects panel, AE elements ingest (R1c), audio,
  multi-select, ripple edits, keyframes.

## 6. Verification

1. `cargo test -p dpl-composer` green (old 5 + new resolution/clamp tests).
2. `cargo green` via pre-push hook.
3. Proof shots (sequential): `wiredfront/editor` at 1280x800 with a
   2-clip comp staged in a scratch CONTENT_ROOT — the shot must show
   thumbnail cards, ruler marks, two proportional bars, the playhead;
   1100x700 fit check; `workbench/library` showing the TRAITS section;
   `wiredfront/library` showing the media shelf.
4. Feel bar: does the timeline read as an EDITOR (AE-shaped) at a glance?

## 7. Commit plan

Branch `feat/r1x-real-timeline` off `feat/r1w-comp-endpoints`. Two
commits: (1) the editor core; (2) the library un-crossing. Push + PR.

## 8. Evidence

(filled at execution)
