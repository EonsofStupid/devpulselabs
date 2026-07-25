# W6 — frictionless media-in: the real Library shelf + managed ffmpeg

Operator redlines (2026-07-25, verbatim): "library does not allow me to
frictionless double click on img or video to import into the media
editor. When dragged directly to editor shows needs FFMPEG on path
error."

## 1. Context

Two friction failures on the WiredFRONT editor path. (a) The
`wiredfront/library` MEDIA shelf is still a census TEXT page
(`media_library(ui)` — labels only, no thumbnails, no interaction), so
there is no double-click-to-editor. (b) ffmpeg is spawned as bare
`ffmpeg` everywhere: all three engine crates expose `set_ffmpeg_path`
injection seams (the original app's managed-binary law) but NOTHING in
the app calls them — a double-clicked exe inherits Explorer's stale
PATH and every preview/probe/render fails with "needs ffmpeg on PATH".

## 2. Current state (ground-truthed on main @ 6825cff, this session)

- `crates/hubs/wiredfront/src/lib.rs` — `media_library(ui)` (free fn,
  census only) mounted at `route.sub == LIBRARY`; `WiredfrontHub` holds
  `fabric: Option<Fabric>` (NavigateRequest publisher) and
  `composer: dpl_composer::Composer`; `thumb_uri`/`filter_chip`/grid
  card patterns live in the same file (Gallery).
- `crates/composer/src/lib.rs` (3719 lines; R1z added checkpoint/
  undo/autosave/state) — `add_clip(&mut self, rec: &Record)` is the ONE
  timeline-append path (checkpoint + mark_edit + select + status);
  pub seams: `pin_pack`, `save_comp`, `import_dir`, `ui`, `run_export`.
  Viewer fallback text: "Loading the frame… (needs ffmpeg on PATH)".
- ffmpeg seams (all injected-once OnceLock, PATH fallback):
  `dpl_engine_capture::set_ffmpeg_path` (lib.rs:68),
  `dpl_engine_media::set_ffmpeg_path` (lib.rs:33),
  `dpl_video::set_ffmpeg_path` (lib.rs:35). Sole caller anywhere:
  `engine-capture/src/bin/capture_cli.rs` (exe-adjacent). The APP
  (`crates/app/src/main.rs`) never injects.
- The original's law (`devpulse labs/src-tauri/src/commands/tools.rs`
  `init_ffmpeg`, read earlier this session): ① `bin/` beside the
  portable anchor ② bundled resources ③ dev binaries ④ bare PATH,
  warned loudly.

## 3. Target design

**A. Managed ffmpeg at boot** (`init_ffmpeg()` in app main.rs, called
before `App::new` and before the headless seams): resolve ONE absolute
ffmpeg and inject it into ALL THREE engine seams. Candidates in order,
first that `is_file()`:
1. `bin/ffmpeg[.exe]` beside the exe (the portable-bundle story);
2. absolute resolution from the PROCESS PATH (walk `PATH` entries,
   join the binary name — an absolute path survives even if children
   inherit a weird env);
3. `%LOCALAPPDATA%/Microsoft/WinGet/Links/ffmpeg.exe` (Windows only —
   rescues the double-click-with-stale-Explorer-PATH case, which is
   the operator's exact failure);
None found → `tracing::warn!` naming every candidate (the honesty law)
and the engines keep their dev PATH fallback.

**B. The Library becomes the real shelf** (`library_surface` method on
`WiredfrontHub` + `LibraryView` state): thumbnailed adaptive card grid
of index rows kind `video`/`image` (renders are kind video and appear
naturally), Gallery's grid/thumb idiom, cl::tip on every card
("Double-click to send it to the Editor"), **double-click → the editor
imports it and the app navigates to `wiredfront/editor`** (fabric
NavigateRequest — the frictionless law). ELEMENTS census + honest
empty states stay below the wall. Refresh chip re-loads rows.

**C. `Composer::add_from_library(&mut self, rec: &Record)`** (new pub
seam): video → the existing `add_clip` path (checkpoint/undo/autosave
all ride free); image → honest status naming the image-layers pass
(image layers stay OUT of scope — own ratified pass) + palette cache
invalidated so the row shows in FOOTAGE.

Scale story: the Nth outside surface that wants to feed the editor
calls the same one seam.

## 4. Changes, in execution order

1. `crates/app/src/main.rs` — add `fn init_ffmpeg()` + `fn
   find_on_path(bin: &str) -> Option<PathBuf>`; call `init_ffmpeg()` in
   `main()` directly after the user-data-root log block. Uses
   `dpl_engine_capture::set_ffmpeg_path`,
   `dpl_engine_media::set_ffmpeg_path`, `dpl_video::set_ffmpeg_path`
   (app Cargo.toml gains `dpl-engine-media.workspace = true`,
   `dpl-video.workspace = true` if absent — verify at edit).
2. `crates/composer/src/lib.rs` — add `pub fn add_from_library` beside
   `import_dir`; unit test: video rec → 1 layer + undo snapshot; image
   rec → 0 layers + status names the pass.
3. `crates/hubs/wiredfront/src/lib.rs` — `LibraryView` struct (rows,
   status) + `library_surface(&mut self, ui)` replacing the
   `media_library` free fn (census sections fold in); double-click →
   `self.composer.add_from_library(&rec)` + NavigateRequest to
   EDITOR via `self.fabric`; keep `count_files` (elements census).
4. `docs/plans/wiredfront-experience.md` — friction table: row 10 notes
   the Library is now the real media shelf (W6); new row 13 "Double-
   click Library → editor" ✅ W6; new row 14 "ffmpeg found without
   PATH" ✅ W6.
5. `CLAUDE.md` — no new flags (no harness change needed).
6. `docs/STATE.md` — W6 entry (at PR time).

## 5. Boundaries

🚫 Image LAYERS (timeline images) — its own ratified pass; the image
double-click stays honest, never fakes a layer. 🚫 The Gallery's card
behavior (Dump/Keep/session-play) unchanged. 🚫 ffmpeg stays a
subprocess (ADR-009) — this pass only resolves WHICH binary. 🚫 No new
crate deps. ⚠️ Ask-first: bundling an ffmpeg.exe into bin/ (license +
size call — this pass only LOOKS there).

## 6. Verification

1. `cargo test -p dpl-composer --lib` — new seam tests pass; whole
   gate: `cargo green` exit 0.
2. REAL-EXE ffmpeg drill: from the scratch W5 root, run
   `PATH="C:\Windows\System32" target\release\devpulselabs.exe
   --export-guide inbox/demo-guide_v1 inbox/session-demo` — with
   ffmpeg STRIPPED from PATH the export still succeeds end-to-end
   (replay re-encode + gif + guide-video all spawn the resolved
   absolute binary); boot log names the source (`winget` expected on
   this laptop). Before the fix this exact command fails.
3. Library proof shots at real sizes: `--route wiredfront/library
   --window 1100x700` and `1600x1000` against the scratch root —
   thumbnailed cards visible; docs/proof/w6-library-{1100x700,
   1600x1000}.png.
4. Sequential exe runs (index law).

## 7. Commit plan

Branch `feat/w6-frictionless-media-in` off main. One commit (both fixes
are one experience story), PR → squash-merge → exe rebuild → STATE.

## 8. Evidence (2026-07-25)

1. `cargo green ✔` (fmt + clippy + all tests; the first run's release
   leg failed only on the operator's RUNNING exe file-lock — rebuilt
   clean after close). `add_from_library` seam test: video → 1 layer +
   undo snapshot + dirty; image → no fake layer, status names the
   image-layers pass.
2. MANAGED-FFMPEG DRILL (the operator's exact failure): with
   `PATH=C:\Windows\System32` ONLY, the exe resolved
   `...WinGet/Packages/Gyan.FFmpeg.../bin/ffmpeg.exe`
   (source "winget-package" — FINDING: winget never made a Links shim
   for ffmpeg on this machine; the Packages scan was added mid-pass and
   is the candidate that fires) and ran the FULL guide export end to
   end: exit 0, re-encode + replay.gif + guide-video + html all
   written. Before the fix the same command warns "no ffmpeg found"
   and fails.
3. The P1 root law correctly out-ranked the drill's cwd (exe walk-up
   found the repo root) — the drill now passes `--content-root`, which
   ALSO proves the P1 CLI-arg seam live on the real exe.
4. Library proof shots at real sizes:
   docs/proof/w6-library-{1100x700,1600x1000}.png — 4 real thumbnailed
   cards (video POSTERS generated by the managed ffmpeg during
   `--import-drill`, proving the injection chain), refresh chip,
   double-click blurb, honest ELEMENTS line.
