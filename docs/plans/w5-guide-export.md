# W5 — the real .guide export (EXECUTION PLAN)

Supersedes-and-executes `docs/plans/w5-guide-export-proposal.md`. The
proposal's §2 inventory was RE-READ IN FULL this session (2026-07-24):
`crates/engine-steps/src/lib.rs` + `edit.rs`, `crates/hubs/wiredfront/
src/lib.rs`, `crates/composer/src/lib.rs` (all 2593 lines),
`crates/engine-media/src/lib.rs`, `crates/app/src/main.rs`,
`crates/viziwiz/src/session.rs`, `Cargo.toml`, index API surface.

## 1. Context

The operator's ask (2026-07-24, verbatim): "steps recorder where …
myself or AI can execute steps and then generate shots i can prune and
post process … session replays be true optimized incredibly light …
bundled with the steps recorder … a true guide, and then exported as a
stand alone self reliant [deliverable] … fully built and proven."

RATIFIED this session (AskUserQuestion, operator):
1. Deliverable = **BOTH** the self-contained `guide.html` AND a single
   `guide-video.mp4` render (stills held N seconds + replay stitched).
2. Replay video: **embed in the html by default** (data URI), with a
   sidecar option (the file always ships in the folder either way —
   it is the forward .guide ZIP payload).

What already exists (verified): the Steps recorder (two-frame do/done
WebP stills + click positions + guide.json), the prune/reorder/caption
ENGINE (`engine-steps::edit::update_guide` — "the W5 reviewer UI
drives this later"), session folders (`session.mp4` + `events.jsonl` +
`session.jpg`), the in-app tick-rail player, the Guide tab's 1-2-3-4
flow (comp render only). Missing: the reviewer UI, the bundle, the
standalone exports, the harness seam.

## 2. Current state (file-by-file, from this session's reads)

- `crates/engine-steps/src/lib.rs` — `StepRec { order, file,
  result_file: Option, action, click: Option<[f64;2]>, app, title,
  width, height, bytes }` serialized camelCase into `guide.json`
  (`kind:"guide", name, stepCount, totalBytes, steps`).
- `crates/engine-steps/src/edit.rs` — `GuideStepEdit` (StepRec shape +
  `caption: Option<String>`), `update_guide(dir, steps) -> Result<u32>`:
  validates bare filenames, renumbers 1..N, deletes orphan frames,
  refuses empty. Tested.
- `crates/hubs/wiredfront/src/lib.rs` — `parse_events` maps
  events.jsonl lines to tick labels (`route-changed` → "hub/sub",
  `full-bleed` → on/off, else the type string). Private, UI-coupled.
- `crates/composer/src/lib.rs` — `Composer` state struct; `Act` enum;
  `guide_tab` (lines 1976–2106) renders the 1-2-3-4 flow: WHAT SHIPS
  (comp facts) / NAME IT (`save_name` + `name_seeds()`) / RENDER
  (comp stitch) / FILE IT (`keep_chips`). `RenderState` +
  `start_render` is the off-thread ffmpeg pattern; `index_render` the
  write-through pattern; `chip`/`small_btn`/`row_line`/`section_header`
  the ColdLight helpers; `initial_tab` honors `DPL_COMPOSER_TAB=guide`.
- `crates/engine-media/src/lib.rs` — `stitch_args`/`render_stitch`
  (trim→fps→scale+pad→concat filtergraph, libx264 crf21 faststart
  -an), `run_ffmpeg` (private), `ffmpeg_program` OnceLock injection,
  `write_poster`. ADR-009: ffmpeg is a subprocess, never a crate dep.
- `crates/app/src/main.rs` — `--clone-seed` is the headless-seam
  pattern (runs before eframe, prints, exits). Boot struct carries all
  harness flags.
- `crates/index/src/lib.rs` — `Record` (rel_path/location/kind/name/
  tags/source/seconds/captured_at/poster_rel_path/size_kb/modified_ms),
  `upsert`, `keep` (guide folders move wholesale), kind detection =
  `guide.json` presence; `iso_from_ms`. Guide thumb = `step-001.webp`.
- `crates/viziwiz/src/session.rs` — session folder writer; events are
  observed fabric traffic only.
- `Cargo.toml` — workspace members + `dpl-*` path deps; NO `base64`
  workspace dep (transitive only in Cargo.lock) → hand-roll (proposal
  §5 sanctions this over adding a dep).

## 3. Target design

**NEW crate `crates/guide/` (`dpl-guide`)** — the .guide content model
+ both renderers. Deps: `serde`, `serde_json`, `dpl-engine-media`
(ffmpeg arg-building + subprocess only). NO UI deps.

- `Manifest { kind:"guide", name, step_count, steps: Vec<Step>,
  session: Option<SessionRef>, guide_video: Option<String> }`;
  `Step { order, file, result_file: Option, caption: Option, action,
  click: Option<[f64;2]> }` (camelCase serde — round-trips the
  recorder's guide.json, extra recorder fields tolerated on load via
  `#[serde(default)]`-shaped loader struct).
- `SessionRef { video: Option<String>, events: Vec<Tick{ms,label}> }`
  — `parse_events` reimplemented PURE here (same label mapping as the
  wiredfront player; wiredfront keeps its private copy — no shim).
- `load_guide(dir) -> Result<Manifest, String>` (from guide.json),
  `load_session(dir) -> SessionRef` (events.jsonl; absent video/file
  degrades honestly).
- `base64(data: &[u8]) -> String` — hand-rolled RFC 4648 encoder,
  tested against known vectors.
- `write_html(manifest, dir, opts) -> Result<PathBuf>` — ONE
  self-contained `guide.html`: viewport meta, dark touch-first UI,
  stills as `data:image/webp;base64` URIs, prev/next + arrow keys +
  touch swipe, step counter, captions, do/done toggle where
  `result_file` exists, CLICK MARKER dot at the recorded normalized
  position, session `<video>` (embedded data URI when
  `opts.embed_video`, else `src="session.mp4"` sidecar reference +
  honest missing-file note) with event ticks as tappable chapters
  seeking `currentTime`. ZERO external references (`http`/`https`
  never appear).
- `export_guide(source_guide_dir, session_dir: Option, out_dir, opts)
  -> Result<Export, String>`: copy + RENUMBER the manifest's kept
  stills into `out_dir` (sources never mutated; `step-001.webp` first
  → the index thumb law holds), light-re-encode the session video to
  `out_dir/session.mp4` (always shipped in-folder — the ZIP-forward
  payload), write `guide.json` (the export manifest — the folder
  indexes as kind "guide"), write `guide.html`, optionally
  `render_guide_video` → `guide-video.mp4`.
- `ExportOpts { embed_video: bool (true), make_guide_video: bool
  (true), secs_per_still: f32 (3.0) }`.

**`crates/engine-media`** grows two arg-builders + runners (ADR-009
pattern, arg-assertion tests like `stitch_tests`):
- `replay_args(src, out)` / `render_replay` — the "incredibly light"
  re-encode: `scale=-2:'min(720,ih)',fps=15`, libx264 crf 30 preset
  veryfast, +faststart, -an (screen capture compresses extremely well
  at these settings).
- `guide_video_args(stills, secs_per_still, session: Option, w, h,
  fps, out)` / `render_guide_video` — each still `-loop 1 -t {secs}
  -i`, session as a trailing plain input, every leg through the same
  scale+pad+fps chain, concat, libx264 crf 21 (its input is already
  the light replay).
- `run_ffmpeg` goes `pub(crate)`→ stays private; the new runners live
  beside `render_stitch` and reuse it.

**`crates/composer`** — the Guide tab becomes the real flow (the W5
reviewer UI): 1·PICK (guides list kind "guide" + sessions list kind
"session" from the index, tappable rows, comp render stays a line)
· 2·REVIEW & TUNE (per-step rows: 64px thumb via file:///, keep
toggle, ↑/↓ reorder, caption TextEdit; **Apply edits** calls
`engine_steps::edit::update_guide` — prune/reorder/caption land in
the SOURCE guide, orphan frames deleted by the engine) · 3·NAME IT
(existing seeds) · 4·EXPORT (embed-video toggle default ON,
guide-video toggle default ON, Export chip → off-thread
`ExportState` like `RenderState`) · 5·FILE IT (keep chips on the
export folder). `pub fn run_export(root, guide_rel, session_rel,
out_name, opts)` is the ONE path (tab + harness): mints
`inbox/<name>/` no-clobber (default `<guide>-share`), calls
`dpl_guide::export_guide`, upserts the index Record (kind "guide",
source "export").

**`crates/app/src/main.rs`** — `--export-guide <guide-rel>
[<session-rel>]` headless seam (the `--clone-seed` pattern): runs
`dpl_composer::run_export` with defaults, prints the html path,
exits by verdict.

Scale story: the Nth export = data through the same seam; the native
Guide Player later consumes the SAME manifest/folder zipped — nothing
throwaway.

## 4. Changes, in execution order

1. `Cargo.toml` — members: add `"crates/guide"` after
   `"crates/comp"`; workspace deps: add
   `dpl-guide = { path = "crates/guide" }` after `dpl-comp`.
2. CREATE `crates/guide/Cargo.toml` — `dpl-guide`, workspace
   version/edition/lints, deps: serde (derive), serde_json,
   dpl-engine-media.
3. CREATE `crates/guide/src/lib.rs` — everything in §3 (manifest,
   loaders, base64, html, export) + tests: base64 vectors; manifest
   round-trip from a recorder-shaped fixture guide.json; loader
   tolerates recorder extras; validator/missing-file refusals;
   write_html embeds N `data:image/webp;base64` URIs, contains no
   `http`, sidecar mode references `session.mp4`, embed mode contains
   `data:video/mp4;base64`; export_guide (no session, no video)
   copies + renumbers stills and writes html + manifest; chapter
   labels map events.jsonl lines (route-changed/full-bleed/other).
4. `crates/engine-media/src/lib.rs` — add `replay_args` +
   `render_replay`, `guide_video_args` + `render_guide_video` (after
   the stitch block) + arg tests.
5. `crates/composer/Cargo.toml` — add `dpl-guide.workspace = true`,
   `dpl-engine-steps.workspace = true` (verify current dep list at
   edit time).
6. `crates/composer/src/lib.rs` — Composer state adds `guides`,
   `sessions_list`, `sel_guide`, `sel_session`,
   `review: Option<GuideReview>`, `embed_video: bool`,
   `make_video: bool`, `export: Arc<Mutex<ExportState>>`; `Act` adds
   `PickGuide(String)`, `PickSession(String)`, `ReviewKeep(usize)`,
   `ReviewMove(usize, i32)`, `ReviewApply`, `ExportGuide`; rewrite
   `guide_tab` per §3; add `run_export` + `start_export` +
   `GuideReview`/`ExportState`; `ensure_project` loads the two lists;
   tests: review reorder/keep mapping to `GuideStepEdit`, export-name
   minting no-clobber, next-hint unchanged.
7. `crates/app/src/main.rs` — the `--export-guide` headless block
   directly after the `--clone-seed` block (same shape).
8. `CLAUDE.md` — harness flags line gains `--export-guide <rel>
   [<session-rel>]`.
9. `docs/STATE.md` — W5 landed entry (at PR time).

## 5. Boundaries

🚫 The NATIVE .guide player product (winit+wgpu+libmpv) — not this
pass; the manifest/folder is the forward contract. 🚫 No webview, no
external assets, no new third-party deps (base64 hand-rolled). 🚫
ffmpeg stays a subprocess (ADR-009). 🚫 Sources never mutated:
export writes NEW artifacts; `update_guide` edits only the guide the
operator explicitly reviews (its engine deletes pruned frames — that
IS its ratified contract, not a source mutation). 🚫 `/content` law:
nothing hand-authored into the real content root; drills run against
a scratch CONTENT_ROOT (cwd-resolved) plus one operator-eyes real-exe
run. 🚫 No `#[allow]`; no re-export shims. ⚠️ Ask-first: any new
crate dep beyond the workspace set; changing wiredfront's player.

## 6. Verification

1. `cargo test -p dpl-guide -p dpl-engine-media -p dpl-composer` —
   exit 0.
2. `cargo green` (fmt + clippy + all tests + release build) — exit 0.
3. Seed a synthetic guide (3 steps, real WebP bytes via the image
   crate or ffmpeg) + synthetic session (ffmpeg testsrc mp4 +
   hand-written events.jsonl) under a SCRATCH content root; run the
   REAL exe `--export-guide inbox/<g> inbox/<s>` with cwd at the
   scratch root → exit 0, prints the html path; assert: guide.html
   exists, contains ≥3 `data:image/webp` URIs + `data:video/mp4`,
   zero `http` refs; session.mp4 + guide-video.mp4 exist; ffprobe
   guide-video duration ≈ 3×3s + replay length; export folder
   indexes kind "guide".
4. Open the exported guide.html in Chrome (claude-in-chrome):
   phone-width screenshot proof — steps advance, do/done toggles,
   chapters seek.
5. Guide-tab proof shots at real sizes: `DPL_COMPOSER_TAB=guide
   --route wiredfront/editor --window 1100x700` and 1600x1000 →
   docs/proof/w5-guide-tab-{1100x700,1600x1000}.png.
6. Sequential exe runs only (index corruption law).

## 7. Commit plan

Branch `feat/w5-guide-export` off main. Commits: (1) dpl-guide crate
+ engine-media renderers + tests; (2) composer Guide tab reviewer +
export + harness seam + docs. `gh pr create` → squash-merge → rebuild
`target\release\devpulselabs.exe` → STATE.md update (§ session-end
law). Uncommitted operator edit `seeds/slides/static/headline-clean/
slide.json` stays untouched and uncommitted.

## 8. Evidence (2026-07-24/25 execution session)

1. `cargo test -p dpl-guide` — 6/6 (base64 vectors, recorder round-trip,
   event→chapter mapping, export renumber + self-contained html, embed vs
   sidecar, jail refusals). `cargo test -p dpl-engine-media` — 13/13
   (incl. the 4 new replay/guide-video arg tests). `cargo test -p
   dpl-composer --lib` — 18/18 (incl. review mapping lossless + blank
   captions clear; jail/mint/no-clobber; non-guide refused by name).
2. `cargo green ✔` — fmt + clippy + all tests + release build
   ("green ✔ exe: target/release\devpulselabs.exe").
3. REAL-EXE DRILL (scratch CONTENT_ROOT, synthetic 3-step guide with a
   done-frame + click + caption, 6s testsrc session with 4 events):
   `devpulselabs.exe --export-guide inbox/demo-guide_v1
   inbox/session-demo` → exit 0, printed
   `inbox\demo-guide_v1-share\guide.html`. Assertions on the output:
   guide.html has exactly 4 `data:image/webp` URIs (3 do + 1 done) +
   1 `data:video/mp4` URI + ZERO `http` occurrences; ffprobe
   guide-video.mp4 duration = 15.000000 (3×3s stills + 6s replay);
   session.mp4 re-encoded 88 KB → 44 KB at 1280x720 @ 15fps (the light
   law); guide.json manifest carries session.mp4 + 4 chapter events +
   guideVideo; whole html = 113 KB with everything embedded.
4. Proof shots at REAL window sizes: docs/proof/
   w5-guide-tab-1100x700.png (picker showing the index write-through
   row `demo-guide_v1-share` — live proof the export upsert landed) and
   w5-guide-tab-1600x1000.png (the reviewer open on the demo guide:
   thumbs, Keep chips, edge-clamped reorder arrows, loaded caption,
   two-tap Apply, "3 of 3 step(s) ship"; fit guard shrank to 1522×778
   with the footer visible).
5. **W5.3 AMENDMENT (operator, mid-execution): "a full test by you …
   creating stills and a truly highly optimized session replay that can
   be easily exported as gif or whatever the latest trending web
   friendly format is for like linkedin".** Landed as a third commit on
   the same branch: (a) engine-media `gif_args`/`render_gif` — the
   palettegen/paletteuse two-pass social GIF (12fps, ≤480px, bayer
   dither) + arg test; the LinkedIn-native upload remains the bundled
   light session.mp4; (b) `ExportOpts.make_gif` (default ON) →
   `replay.gif` beside the bundle, manifest `replayGif`, Guide-tab
   "Social GIF" toggle; (c) THE AI-DRIVEN FULL TEST seam:
   `--steps-drill <n>` — starts the Steps recorder through the REAL
   palette handler with a drill-only self-capture opt-in
   (`StepsRecorder::set_allow_self`; pid-0 disables the exclusion —
   engine-capture untouched), marks n REAL focused-window captures,
   stops, and files through the REAL Save-card path (strict meta law
   held; drill fills only what the operator would type).
   **FULL-TEST EVIDENCE (all REAL artifacts, AI-driven end to end,
   sequential exe runs, scratch content root):**
   `--steps-drill 3` → "steps: recording" → "3 steps pending" →
   "guide: 3 steps → <folder>" — 3 real focused-window captures saved +
   indexed through the real path. `--session-drill 8` → a real 490 KB
   screen recording + 2 genuinely observed route-changed events.
   `--export-guide <guide> <session>` → exit 0; the bundle:
   session.mp4 490 KB → **44 KB** (1280×720 @ 15fps, 6.6s — the light
   law, 11×); **replay.gif 179 KB, 480×270, 79 frames** (palettegen
   two-pass); guide-video.mp4 15.6s (3×3s stills + 6.6s replay);
   guide.html with 3 embedded webp URIs + 1 embedded mp4 URI + 0 http
   refs; manifest carries session.mp4 · 2 events · replay.gif ·
   guide-video.mp4. FINDING FIXED in the same commit: seeded guide
   names (from window titles) had no length cap — a path-length title
   minted a folder that broke Win32 cwd limits; `seed_name` now clamps
   to 48 chars (tested).
6. DEVIATIONS from §4: (a) added the proof-only `DPL_GUIDE_PICK` env
   override (the `DPL_COMPOSER_TAB` pattern, inert in normal use) so the
   reviewer screenshots reproduce; (b) the in-browser interactive html
   proof was not run this session (Chrome extension not connected) —
   the html's self-containment is content-asserted and unit-tested; the
   phone/double-click load is the operator's acceptance test, as §6
   already stated.
