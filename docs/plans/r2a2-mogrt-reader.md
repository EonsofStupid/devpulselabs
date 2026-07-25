# R2a.2 — the mogrt reader: real previews, names, and tags for 9k elements

Status: EXECUTING 2026-07-25, same session as R2a.1. The drive returned
("sorry the drive is back i was copying something"); the real stores are
PINNED in the real root now (`--pin-pack` on both
`D:\webwork\themeforest\{video,audio}`).

## 1. Context — the spike verdict (real specimens, 2 unrelated packs)

- **`.mogrt` = a plain ZIP** (DEFLATE, method 8) holding exactly
  `definition.json` + `project.aegraphic` + **`thumb.png`** — verified
  identical shape on `Title 01.mogrt` (videohive-1XFu5pmj, api 1.5) and
  `[Toko] Typography_Slide_01.mogrt` (videohive-TrhF8y7Z, api 1.7).
  `definition.json` carries `capsuleName`, `capsuleTags`, and
  `clientControls` (editable text/font/size declarations — the metadata
  layer of the ratified "change the color, font, context of the preset").
  9,021 of these exist in the video store: REAL preview cards without AE.
- **`.aep` = RIFX big-endian container, form `Egg!`** — opaque binary.
  OPERATOR RULING (this session): the parsing question STAYS OPEN while
  they research; meanwhile .aep files remain filed as elements.
- **`.cube` = trivial text** (`TITLE`/`LUT_3D_SIZE`/float triples) —
  R2b's ffmpeg lut3d path is confirmed cheap.
- **Mac junk**: the stores carry `._*.mogrt` AppleDouble twins — the
  existing dot-skip already excludes them everywhere.
- OPERATOR RULING (this session): dependency = **the full `zip` crate**
  ("I will need it eventually"), not a hand-rolled walker. `flate2`/
  `miniz_oxide` were already transitive, so the compile cost is small.

## 2. Current state (all read this session)

- `crates/composer/src/import.rs`: `import_file` routes elements
  (aep/lottie/mogrt/cube/ffx/prfpset) to `elements/imported/` +
  `record_for_capture(kind "element")` — NO poster, NO tags, NO real
  name (file stem only).
- `crates/composer/src/packs.rs::ingest_copy`: videos get poster +
  seconds; elements get a bare index row.
- `crates/composer/src/browse.rs` (post-R2a.1): `Entry{path,name,kind}`,
  `card()` paints Element entries as a lime ELEMENT badge + ext;
  `entry_grid` renders folder and search hits alike.
- `dpl_index::Record` has `tags` + `poster_rel_path`; `Index::set_tags`
  exists (unused).
- Workspace deps live in root `[workspace.dependencies]`; crates use
  `x.workspace = true`.

## 3. Target design

1. **Dep**: `zip` (workspace, `default-features = false, features =
   ["deflate"]`).
2. **NEW `crates/composer/src/mogrt.rs`**: `pub struct Mogrt { name:
   String, tags: Vec<String>, controls: usize, thumb_png: Vec<u8> }`;
   `pub fn read(path) -> Result<Mogrt, String>` (zip open →
   definition.json via serde_json → capsuleName/capsuleTags/
   clientControls.len(); thumb.png bytes; absent pieces degrade — name
   falls back to the file stem, missing thumb = empty). Tests write a
   real zip with `zip::ZipWriter` and read it back; a non-zip errors
   honestly.
3. **Import + ingest enrichment**: when the element is a `.mogrt`, the
   import seam and `ingest_copy` extract `thumb.png` beside the copy as
   `<stem>-poster.png`, set `poster_rel_path`, `name = capsuleName`,
   `tags = capsuleTags` on the index row. Other element kinds unchanged.
4. **Browse cards get REAL mogrt previews**: a per-window thumb cache
   (`HashMap<PathBuf, Option<egui bytes uri>>`) filled lazily at paint,
   BUDGETED (≤8 new zip reads per frame, repaint requested while any
   are pending) so a 300-mogrt folder fills smoothly instead of
   hitching. Cards with a loaded thumb paint it (badge fallback stays).
5. NOT this pass: rendering `project.aegraphic` (opaque), the editable
   controls UI (needs the effects pass), the elements overlay lane,
   `.aep` parsing (open research — the operator's).

## 4. Changes, in execution order

1. Root `Cargo.toml` `[workspace.dependencies]` + composer `Cargo.toml`:
   `zip`.
2. CREATE `crates/composer/src/mogrt.rs` + `mod mogrt;` + tests.
3. `import.rs::import_file`: mogrt branch (poster extract + name/tags).
4. `packs.rs::ingest_copy`: same enrichment via one shared helper.
5. `browse.rs`: thumb cache on `Browser` + budgeted fill + card paint.
6. Docs: STATE + §8.

## 5. Boundaries

🚫 Pinned trees READ-ONLY (zip reads only). 🚫 `project.aegraphic` is
never parsed or extracted. 🚫 No other new deps. 🚫 `/content` law. ⚠️
The `.aep` question stays OPEN by operator ruling — nothing here may
foreclose it.

## 6. Verification

1. `cargo test -p dpl-composer` — mogrt round-trip + enrichment tests +
   the 32 green.
2. `cargo green` ✔.
3. Real-exe drill: `--import-drill` a folder holding a REAL specimen
   copy → the element's index row carries the real capsuleName, tags,
   and a poster; the wiredFRONT Library shows it. Browse-window thumbs
   are interactive (operator's eyes) — unit + import proof carry the
   claim.

## 7. Commit plan

Branch `feat/r2a2-mogrt-reader` off main; code commit + docs commit;
PR → squash merge → exe rebuild → STATE.

## 8. Evidence

Executed 2026-07-25, `feat/r2a2-mogrt-reader` off main (post-R2a.1).

1. `cargo test -p dpl-composer` → **36 passed**: mogrt round trip
   (real zip written with `zip::ZipWriter`, name/tags/thumb read back;
   non-string tags skipped) · stem-fallback degradation · a non-zip
   errors honestly · the enrichment test (a `.mogrt` element record
   gains capsuleName, tags, and an on-disk `-poster.png`; a `.cube`
   passes through untouched) · the R2a.1/R1z/W suites intact.
2. `cargo green ✔`. One clippy-driven design trim: `clientControls` is
   documented but NOT read yet — an unread field is dead weight the
   lint law refuses; the reader grows it when the controls UI consumes
   it. Dep landed as workspace `zip = "8"` (deflate only).
3. REAL-specimen exe drill (scratch CONTENT_ROOT, sequential):
   `--import-drill` on a copy of the real `Title 01.mogrt` → index row
   `Title 01 | elements/imported/Title 01-poster.png | element`
   (sqlite3-verified), the poster file is the byte-identical embedded
   `thumb.png` (12,887 bytes), and the status reads "Title 01 on your
   elements shelf — preview and tags read from the template." Shots:
   `docs/proof/r2a2-mogrt-import-{1280x800,1600x1000}.png`.
4. The REAL stores are pinned in the REAL root now
   (`elements/sources.json` = both `D:\webwork\themeforest` trees).
   Browse-window mogrt thumbs are interactive-only (peer viewports) —
   the budgeted cache is exercised by the operator's first browse of
   the 9k-mogrt store.
