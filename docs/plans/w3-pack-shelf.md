# W3 — the pack shelf: themeforest packs pinned IN PLACE (copy-on-use, never copy-all)

## 1. Context

Operator (2026-07-24): "we will start with D:/webwork/themeforest/audio/
D:/webwork/themeforest/video/". CENSUS (run in-session, read-only):
audio = 250 files / 2.56 GB (wav+mp3 packs, `audiojungle-*`); video =
**70,018 files / 79.61 GB** (`videohive-*` packs: 9,021 .mogrt, 8,693
.mp4, 5,392 .aep, 1,841 .mov, 19,748 .png, 12,749 .gif, 1,728 .mp3,
1,682 .wav, 860 .cube LUTs, 70 .prfpset, 30 .ffx, mac `._*` junk).
**C: free = 14.4 GB** → the ratified copy-all (R1 decision 3) is
ARITHMETICALLY IMPOSSIBLE and is hereby superseded by ground truth:
packs are pinned and browsed IN PLACE on D:; using a file copies THAT
file through the existing W2 import seam (copy-on-use). This also
answers the element-format inventory that gated R1c.

## 2. Current state (read in-session)

- `composer/src/import.rs` (W2): `media_kind` = video/image/element
  (aep·lottie); `import_file` routes element→`elements/imported/`,
  else→`inbox/`; poster+seconds for video; `Index::upsert`.
- `composer/src/browse.rs` (W2): `Browser{slot,cwd,entries,picked,open}`,
  `roots()`, `scan()` (dirs + media files, dot-hidden skipped),
  `window_ui` grid, badge cards, `bar_chip`.
- `composer/src/lib.rs` (W2): `show_browsers`/`drain_media_feeds`/
  `handle_import`; PROJECT panel ELEMENTS section = `count_files`
  census only; `Act::Browse` opens at Videos; `import_dir` harness.
- `app/src/main.rs`: Boot flags incl. `import_drill`; pattern for new
  flags established.
- Index `rel_path` is CONTENT-ROOT-RELATIVE — out-of-root files cannot
  be artifacts rows; the shelf therefore lists packs LIVE from the
  filesystem, no 70k-row index.
- serde_json is a workspace dep (hubs use it); composer doesn't yet.

## 3. Target design

- **Sources registry**: `content_root()/elements/sources.json` — a
  JSON array of absolute pack-root paths. Written ONLY by app actions
  (pin/unpin); read at shelf load. Composer module `packs.rs`:
  `load_sources()`, `add_source(path)`, `Pack{name,path,files}` via a
  one-level listing (`list_packs`: each source's top-level dirs = packs,
  file count computed lazily per pack, capped scan).
- **Kind matrix grows** (import.rs): audio = wav mp3 ogg flac m4a aac →
  kind "audio", routed to `inbox/`; elements grow mogrt cube ffx
  prfpset (+ existing aep lottie) → `elements/imported/`. gif/png stay
  images. `._*`/.DS_Store excluded by the existing dot-skip.
- **Browse windows**: Audio entries get badge cards (AUDIO, Coral);
  top bar gains **Pin pack** (tooltip: "Add this folder to your
  Elements shelf — files stay right here; using one copies just it")
  → `pinned: Vec<PathBuf>` drained by the composer like `picked`.
- **PROJECT › ELEMENTS section** becomes the shelf: one row per pinned
  SOURCE with its pack count; a row per pack (name + media count),
  click = OPEN A BROWSE WINDOW rooted at that pack (the shelf IS the
  browse machinery). Honest empty state names Pin pack.
- **Harness**: `--pin-pack <dir>` (main→hub→composer, the same seam the
  chip takes) for deterministic proofs.
- Audio on the TIMELINE is out of scope (honest status on click);
  parsing mogrt/aep stays the spike.

## 4. Changes, in execution order

1. `composer/Cargo.toml`: `serde_json.workspace = true`.
2. CREATE `composer/src/packs.rs`: sources load/add (pretty JSON,
   create-dirs, dedupe), `list_packs(sources) -> Vec<Pack>`; tests:
   round-trip add/load dedupes, packs listed from a scratch source.
3. `composer/src/import.rs`: extend `media_kind` (audio + new element
   exts); route "audio" → inbox; tests extend the matrix.
4. `composer/src/browse.rs`: `pinned: Vec<PathBuf>` on Browser; AUDIO
   badge arm; **Pin pack** chip in the bar.
5. `composer/src/lib.rs`: `packs: Option<Vec<packs::Pack>>` cache
   (+ ensure/reset on pin); drain `pinned` in `drain_media_feeds` →
   `packs::add_source` + status; ELEMENTS section renders the shelf
   (source rows + pack rows, click → `Act::BrowseAt(PathBuf)`);
   `Act::BrowseAt` opens a Browser at that path (Browse… reuses it);
   `pub fn pin_pack(&mut self, dir)` for the harness; audio import
   status ("in your library — timeline audio lands with the Narrate
   pass").
6. `hubs/wiredfront/src/lib.rs`: `pub fn pin_pack` delegate; media
   shelf lists pinned sources + counts.
7. `app/src/main.rs`: `--pin-pack <dir>` flag → Boot → wiredfront.
8. CLAUDE.md harness list gains `--pin-pack <dir>`.

## 5. Boundaries

- 🚫 The D: pack folders are READ-ONLY — scanned and browsed, never
  written, never reorganized, never "cleaned".
- 🚫 NO bulk copy: only click-picked files pass the import seam.
- 🚫 `/content` law: `sources.json` is app-written state (pin action),
  like captures; sessions never hand-author it.
- 🚫 No new external deps beyond `serde_json.workspace`.
- Out of scope: 70k-row indexing/X1 sidecars, audio playback,
  mogrt/aep/cube parsing (the spike), unpin UI (registry is editable
  JSON; UI rides a later pass).

## 6. Verification

1. `cargo test -p dpl-composer` green (W2 12 + packs round-trip +
   kind-matrix additions).
2. `cargo green` ✔ via hook.
3. Real-exe proof, sequential, scratch CONTENT_ROOT + the REAL D:
   sources (read-only): `--pin-pack D:\webwork\themeforest\video
   --pin-pack D:\webwork\themeforest\audio --route wiredfront/editor
   --window 1280x800` → ELEMENTS shelf lists both sources with real
   pack rows (videohive-*/audiojungle-* names + counts).
4. FS assertions: `sources.json` contains exactly the two paths; D:
   trees' newest write-time UNCHANGED after the run.
5. Feel bar: your packs are one click from a window, two from the
   timeline.

## 7. Commit plan

Branch `feat/w3-pack-shelf` off main; one commit (same-file
entanglement as W2); push, PR, merge; session-end law.

## 8. Evidence

Executed 2026-07-24/25, same session as W1/W2:

1. `cargo test -p dpl-composer` → **15 passed** (packs registry
   round-trip/dedupe · pack listing with media counts · flat-source
   pack · kind matrix grown to audio + mogrt/cube/ffx/prfpset · the
   W2 twelve).
2. `cargo green` ✔ (three runs; mid-pass C: ran OUT OF DISK — the
   51.4 GB `target\debug` tree was deleted, 61 GB free after; the
   debug tree rebuilt clean).
3. Real-exe proof against the REAL folders (read-only): scratch
   CONTENT_ROOT + `--pin-pack D:\webwork\themeforest\video --pin-pack
   D:\webwork\themeforest\audio` → `elements/sources.json` holds
   exactly the two paths; the 1280x800 shot shows ELEMENTS — your
   packs with real rows (`rtfx 1000+`, `videohive-1XFu5pmj-mogrt-ti…`,
   `videohive-DyQIG0nf-flash-fx-p…`, `videohive-GXVeOIgF-rtfx-gen…`),
   the status line naming the pin, and the NEXT hint intact.
4. READ-ONLY assertion: the D: tree's newest write-time is IDENTICAL
   before/after the pin run (2024-03-04 15:37:04).
5. Defects caught by shots and fixed in-pass: shelf buried below the
   panel fold (empty preview well now collapses; ELEMENTS moved above
   FOOTAGE) and long pack names bleeding across the panel edge
   (row painting now clips, names ellipsis under the count).
6. CENSUS recorded (§1): the copy-all ratification is superseded by
   arithmetic — 80 GB of packs vs 14 GB free on C:; pin-in-place +
   copy-on-use is the standing law for pack sources.
