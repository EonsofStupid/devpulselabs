# W2 — R1y media windows + the guided stitch→save→render→file workflow + .aep/.lottie enter the seam

## 1. Context

Operator (2026-07-24, after W1 landed): "do not stop working until you
actually have an editor where i can stitch together and save the new
file easily — guided ui workflow, seeded suggestions, while allowing me
to tag any shows… then the unique export workflow for .guide… the
actual .aep project files… so .aep, .lottie." This RATIFIES the open
question: `.aep` and `.lottie` are first-class imports (personal-use
tool; privacy workflow). This pass = the approved R1y plan
(`r1y-media-windows.md`, executed here) PLUS the guided workflow: a
NEXT-step hint, a seeded comp name on Save, the finished render indexed
and fileable into any show from the editor, and the Guide tab becoming
the tailored export flow (seeded values + tooltips; the double-click
player app remains its own ratified product pass — stated honestly).
`.aep`/`.lottie` files import as ELEMENTS (copied to
`content/elements/imported/`, indexed); PARSING them into comp layers
is a later spike, never faked. Experience gate rows 3, 4 move; the
save/tag/guide flow shortens walkthrough steps 1 and 8.

## 2. Current state (read in-session)

- W1 `crates/composer/src/lib.rs` (authored this session): `Composer`
  fields incl. `palette/comps/renders/elements` caches, `Act`,
  `ae_screen`, `project_panel`, `status_row`, `save()` minting
  `inbox/comp-N/`, `start_render()` writing `<comp-dir>/render.mp4`
  (NOT indexed), `tab_stub` for Capture/Narrate/Guide.
- `crates/app/src/chat.rs` + `egui-pilot-ai/src/lib.rs:1067` — the
  deferred peer-viewport law: `ctx.pilot_show_viewport_deferred(
  PilotViewportKey, ViewportId, ViewportBuilder, Fn(&mut Ui,
  ViewportClass) + Send + Sync + 'static)`; EmbeddedWindow = honest
  failure; close via `viewport().close_requested()`.
- `crates/engine-media/src/lib.rs` — private `poster_args`/`run_ffmpeg`
  (line 261/274); no public single-poster fn.
- `dpl_index` — `record_for_capture`, `upsert`, `keep(root, rel, show,
  ep) -> new_rel`, `iso_from_ms`; `dpl_doc::list_content_episodes`.
- Composer Cargo.toml — no `dpl-doc` dep yet (wiredfront/workbench use
  it via workspace).
- No `dropped_files` handling anywhere in the repo.

## 3. Target design

ONE import seam, three feeders, guided next steps:

- `dpl_engine_media::write_poster(src, out)` — public wrapper.
- `composer::import`: `media_kind` → video (mp4 mov mkv webm avi m4v) ·
  image (png jpg jpeg webp gif bmp) · **element (aep lottie)**.
  `import_file`: videos/images COPY to `content/inbox/` (no-clobber,
  poster + seconds for video); elements COPY to
  `content/elements/imported/`; all write through the index (source
  "import"); the operator's original file is NEVER touched.
- `composer::browse`: `Browser` peer windows (roots = profile media
  folders + drives; folder cards descend; image thumbs; video/element
  badge cards; click = pick). Composer spawns them from a Browse… chip,
  drains picks per frame, and drains the main viewport's
  `dropped_files` through the same seam; `hovered_files` paints a
  voltage "Drop to import" overlay over the editor.
- Guided workflow: `next_hint()` (add footage → save → render → file
  into a show) rendered in the status row; Save takes a SEEDED name
  (sanitized kebab; suggestions: `comp-N`, date-seeded) via a status-row
  field; the render thread now INDEXES `render.mp4` (poster + seconds,
  source "render") so it exists everywhere; when a render is Done the
  status row offers per-show **Keep chips** (`dpl_doc::
  list_content_episodes` + `Index::keep`) — tag any show, one tap.
- Guide tab = the tailored export flow (v1, honest): what ships → name
  (seeded + suggestion chips) → Render → file into a show; footer names
  the standalone player as its own ratified pass.

## 4. Changes, in execution order

1. `crates/engine-media/src/lib.rs`: add `pub fn write_poster` below
   `poster_args`.
2. `crates/composer/Cargo.toml`: add `dpl-doc.workspace = true`.
3. CREATE `crates/composer/src/import.rs` (media_kind incl. element
   tier · no_clobber · import_file as §3; tests: kind matrix incl.
   aep/lottie, no_clobber, sanitize).
4. CREATE `crates/composer/src/browse.rs` (Browser · roots · scan ·
   window_ui per the approved r1y plan, + Element cards).
5. `crates/composer/src/lib.rs`: mod decls; `ContextPilotExt` +
   `PilotViewportKey` imports; fields `browsers/next_browser/save_name/
   episodes`; `Act::{Browse, Keep(show, ep)}`; `show_browsers` +
   `drain_media_feeds` called from `ui()`; drop overlay in `ae_screen`;
   Browse… chip in the PROJECT header; seeded-name Save + `next_hint`
   + Keep chips in `status_row`; render thread indexes its output;
   `guide_tab` replaces the Guide stub.
6. Tests (import.rs + lib.rs): kind matrix, no_clobber, sanitize_name,
   next_hint transitions, keep-chip gating.

## 5. Boundaries

🚫 `/content`: imports COPY into inbox/elements (the sanctioned
inbox-born path); sources never moved/deleted. 🚫 no external crate
deps (dpl-doc is workspace-internal; no rfd, no HTTP — Drive stays
R2d-gated). 🚫 no `#[allow]`; ffmpeg subprocess only. 🚫 comp.json
schema untouched. `.aep`/`.lottie` PARSING is out of scope — files
enter the estate as element artifacts, honestly labeled; the parse
spike gets its own plan. Out of scope: cross-viewport OS drag,
favorites persistence, autosave/undo (R1z).

## 6. Verification

1. `cargo test -p dpl-composer` + `-p dpl-engine-media` exit 0.
2. `cargo green` (pre-push hook enforced).
3. Sequential real-exe proof: editor 1280x800 (Browse… chip, status-row
   name field + NEXT hint); Guide tab 1280x800 (the tailored flow);
   filesystem drill: import a staged mp4 + png + .lottie into a scratch
   CONTENT_ROOT → inbox gains the copies + poster, elements/imported
   gains the .lottie, sources untouched, index rows exist; render a
   2-clip comp → render.mp4 indexed; keep chip files it into a show.
4. Feel bar: the editor now TELLS you the next step; save/export are
   named, seeded, one tap each.

## 7. Commit plan

Branch `feat/r1y-media-windows` off main. Commits: (1) the import seam
+ write_poster + tests; (2) Browse windows + drop wiring; (3) guided
workflow (hint · seeded save · indexed render · keep chips · Guide
tab). Push, PR, merge; session-end law (exe + STATE).

## 8. Evidence

Executed 2026-07-24, same session as W1:

1. `cargo test -p dpl-composer` → **12 passed** (import kind matrix incl.
   aep/lottie · no-clobber · sanitize · next-hint walk · the W1 eight).
2. NEW permanent drills: `tests/import_drill.rs` (ignored FS drill) ran
   against a scratch CONTENT_ROOT + staged real media (2 ffmpeg-made
   mp4s, a png, a .lottie): copies landed (inbox / elements/imported),
   posters + seconds real, SOURCES untouched, index rows present —
   **1 passed**. Plus the real-exe seam `--import-drill <dir>`
   (main → hub → composer, the exact Explorer-drop path; CLAUDE.md
   harness list updated).
3. `cargo green` ✔ (twice; exe rebuilt).
4. Real-exe proof (`w2-*.png`, sequential): `--import-drill` against a
   FRESH scratch root → the 1280x800 editor shot shows BOTH clips as
   proportional bars on their own lanes (3.0s + 2.0s), outline rows
   with color chips + in-times, CTI, Info with the selected layer's
   real timing, transport 0:00/0:05, the status line reporting the
   .lottie filed into elements, and the NEXT hint ("name it and Save")
   on its own guided line. Guide tab shot: the tailored 1-2-3-4 flow
   with seeded name chips (`comp-1`, `guide-2026-07-24`) and honest
   gating. 1100x700 fit check clean.
5. Defect found by the FIRST shot round and fixed in-pass: the NEXT
   hint collided with the right-aligned facts → the status strip became
   two lines (actions / guidance+facts).
6. OBSERVED, pre-existing, not fixed here (noted for STATE): opening an
   index db whose WAL was left by ANOTHER process trips dpl-index's
   quarantine (`artifacts.db.corrupt`) — surfaced by the cross-process
   drill; in-app imports are same-process and unaffected.
7. Deviation from §7: ONE code commit instead of three — the guided
   workflow and browse wiring land in the same `lib.rs` and do not
   split into reviewable standalone commits.
