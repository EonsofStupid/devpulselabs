# R2a.1 — deep-pack browse: search inside packs + offline-pack honesty

Status: EXECUTED 2026-07-25 (operator: "go" on R2a). SCOPE NOTE: R2a's
headline `.aep`/`.mogrt` parse spike is **physically blocked this
session** — ground-truthed: `D:\webwork\themeforest\{video,audio}` is
OFFLINE (D: absent; G: is Google Drive), the real content root has NO
`elements/` and NO `sources.json` (the operator never pinned the real
stores), so zero specimens exist anywhere reachable. The spike + element
tagging = **R2a.2**, gated on the store returning. This slice ships the
D:-independent half — and the offline drive itself exposed the bug this
slice fixes.

## 1. Context

Experience gate row 6 ("AE elements usable") is R2a's target; getting
there needs the operator living INSIDE 70k-file packs. Two frictions
this slice kills:

1. **The shelf lies when the drive is away.** `packs::list_packs` does
   `let Ok(rd) = read_dir(&source) else { continue; }` — an unplugged
   drive makes every pinned pack VANISH silently. For an operator whose
   stores live on a removable disk (today's reality), the shelf must say
   "offline", not pretend nothing was ever pinned.
2. **No way to find anything in a pack.** Browse windows list ONE folder;
   a "lower thirds" search across a 70k-file store means manual descent.
   Deep-pack browse = type-to-filter the open folder + a recursive
   in-pack search, off the UI thread, capped honestly.

## 2. Current state (ground-truthed this session, post-R1z main)

- `crates/composer/src/packs.rs` (read in full): `Pack{name,path,files,
  ingested}`; `list_packs` skips unreadable sources (the bug);
  `load_sources`/`add_source` (sources.json); `media_files`/`pack_size`/
  `ingest_copy` (W4). Tests: 4, hermetic scratch roots.
- `crates/composer/src/browse.rs` (read in full): `Browser{slot,cwd,
  entries,picked,pinned,ingest_req,arm,open}`; `scan` (one folder, dirs
  then media, dot-skip); `window_ui` (bar: up · roots · Pin pack ·
  two-tap Ingest · cwd crumb; card grid; click media → picked); `card`
  (image thumbs, badge plates). No search anywhere.
- `crates/composer/src/lib.rs` (re-read this session for R1z): the
  shelf rows in `project_panel` (`row_line` with tooltip "Open this pack
  in its own window…", click → `Act::BrowseAt(pack.path)`), the
  `IngestState`/thread pattern, Browse windows as `Arc<Mutex<Browser>>`
  deferred viewports.
- `dpl_index::Record.tags` + `Index::set_tags` EXIST (no UI uses them
  yet) — tagging has a data seam waiting for R2a.2's element shelf.
- The editor keyboard map (R1z) is guarded by `egui_wants_keyboard_input`
  — a search TextEdit in a Browse window takes focus and the guard
  already covers it.

## 3. Target design

1. **Offline honesty** (`packs.rs` + shelf rows):
   - `Pack` gains `online: bool`. A pinned source whose dir cannot be
     read lists as ONE pack (named from the source folder), `files: 0`,
     `online: false` — pins NEVER silently vanish.
   - Shelf row: trail reads `offline`, tooltip says the drive isn't
     connected and the pack returns when it is; the row is not clickable
     (no Browse window onto a void).
2. **Type-to-filter + deep search** (`browse.rs`):
   - `Browser` gains `query: String` and `search:
     Option<Arc<Mutex<Search>>>` where `Search{root: PathBuf, query:
     String, hits: Vec<Entry>, running: bool}`.
   - The bar gains a search field. While TYPING: the grid filters the
     current folder's entries by case-insensitive substring (instant,
     zero IO). On ENTER: a named std thread walks `cwd` recursively
     (dot-skip, media + dirs excluded → media files only), pushing
     case-insensitive name matches into `hits`, capped at 500 (the cap
     is SAID in the UI — no silent truncation), `running=false` at end.
     Results replace the grid (same cards, so click-to-import Just
     Works); a line names the state honestly ("searching…", "N found
     under <folder> (first 500)" / "nothing matches"); a `x clear` chip
     (and navigating) returns to the folder view.
   - Pure helper `search_files(root, query, cap) -> Vec<Entry>` does the
     walk — unit-tested; the thread is a thin wrapper.
3. NOT this slice (R2a.2, gated on the store): `.aep`/`.mogrt`
   dissection (mogrt = zip with definition.json + thumbnail — likely
   needs a zip/deflate dep = ask-first), element tag chips over
   `Index::set_tags`, the elements shelf with real previews, overlay
   lane. NOT this slice: searching across ALL packs at once (one window
   = one root stays the model).

## 4. Changes, in execution order

1. `packs.rs`: `Pack.online` + offline branch in `list_packs` + test
   (pin a scratch source, delete it, list → one offline pack, name
   kept). Existing constructors set `online: true`.
2. `browse.rs`: `search_files` + `Search` + bar field + filtered grid +
   results view + clear/disarm on nav + tests (nested match,
   case-insensitive, cap honored, dot-skip, non-media excluded).
3. `lib.rs`: shelf row offline trail/tooltip/click-gate.
4. `docs/STATE.md` + §8 here.

## 5. Boundaries

🚫 Pinned trees stay READ-ONLY (search walks, never writes). 🚫 No new
deps (std threads; the zip question belongs to R2a.2's ask-first). 🚫
`/content` law. 🚫 Don't touch the Guide tab, import seam, ingest, or
R1z surfaces beyond the shelf row. ⚠️ Ask-first: any all-packs global
search UI; the zip/deflate dep when R2a.2 arrives.

## 6. Verification

1. `cargo test -p dpl-composer` — new packs/browse tests + the 27 green.
2. `cargo green` ✔ via the hook.
3. Real-exe SEQUENTIAL drill (scratch CONTENT_ROOT): run 1 `--pin-pack
   <scratch pack>` → shelf shows the pack with its count; DELETE the
   scratch pack dir; run 2 → the row remains, trail `offline` (shot at a
   real window size). Search is proven by unit tests + the operator's
   hands (Browse windows are interactive peer viewports the headless
   shot can't reach).

## 7. Commit plan

Branch `feat/r2a1-deep-pack-browse` off main. One commit (code+tests) +
one docs commit. PR → merge → exe rebuild → STATE.

## 8. Evidence

Executed 2026-07-25, `feat/r2a1-deep-pack-browse` off main (post-R1z).

1. `cargo test -p dpl-composer` → **32 passed**: the offline-pack test
   (pin a scratch source, delete it → ONE pack row, `online: false`,
   name kept), the search suite (nested + case-insensitive + name-sorted
   hits; txt and dot-dirs never; empty query = every media file; cap
   honored at 3-of-7; navigation clears query/search/arm), the census
   fix test (`sources.json` is not an element), plus the R1z/W-series
   suite intact.
2. `cargo green ✔`; release exe rebuilt (census fix re-tested +
   rebuilt after — 32nd test — and re-shot).
3. Real-exe SEQUENTIAL drill (scratch CONTENT_ROOT): run 1 `--pin-pack
   <scratch store>` → `elements/sources.json` written; the store dir was
   then DELETED (the unplugged-drive scenario); runs 2/3 → the shelf
   row READS `pak r2a-store · offline` with the plug-it-back-in tooltip
   — the pin never vanished. Shots:
   `docs/proof/r2a1-offline-pack-{1280x800,1600x1000}.png`.
4. BONUS FIX surfaced by the drill's first shot: the elements census
   counted `sources.json` itself ("1 imported element file(s)" on a
   pinned-but-empty library) — every pinned operator saw a phantom
   element since W3. Skipped now, unit-tested, re-shot clean.
5. Deep search on the exe is interactive-only (Browse windows are peer
   viewports the headless shot can't reach) — proven by the unit suite;
   the operator's first real search is theirs to type.
