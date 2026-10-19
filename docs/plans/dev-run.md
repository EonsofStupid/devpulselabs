# Dev run — `cargo xtask dev`

**Status:** APPROVED by the operator 2026-10-19 ("GO") and implemented on
`feat/xtask-dev`. Grounded 2026-10-13 at `d3c0a73`.

## 1. Context

The operator, 2026-10-13 (R278): *"I was wrong to only expect the binary we
must adopt proper dev practices running it while we dev and focus on all
incomplete first that get full testing"*. CLAUDE.md now says to develop on a
dev run. No dev run exists yet, and the incomplete work (status.md, Next 2)
gets developed and tested on it. So this comes first.

## 2. Goal / non-goals

**Goal:** one command, `cargo xtask dev [-- <app flags>]`. It builds the
debug app and its helpers, makes sure the tool depot is beside the debug
exe, and opens the app on the operator's library. The operator, or a
session, can then iterate on a running app.

**Non-goals:**
- No hot reload.
- No new app behaviour.
- The release exe and the gate are unchanged (still how work lands).

## 3. Ground truth (2026-10-13)

- **A debug build can't run properly today.** Since D0.1b every helper
  must sit beside the exe, and the depot resolves only from
  `<exe dir>/dptools-deps` (`crates/tools-deps/src/lib.rs:147-160`).
  `target\debug` has no `devpulselabs.exe`, no `dpl-lottie-render.exe` and
  no `dptools-deps` (checked 2026-10-12), so a debug run boots with ffmpeg
  and pdftoppm "unavailable".
- **xtask has no dev task.** Its tasks are green, sweep, docs-status,
  structure, coverage, smoke, lock, stage-tool-deps and fix
  (`crates/xtask/src/main.rs:1803-1832`).
- **Staging exists.** `stage_tool_dependencies` (`main.rs:459`) copies and
  SHA-verifies from `dptools-deps/sources.local.toml`. It refuses a root
  that already has `dptools-deps`. The staged depot is 1.2 GB.
- **The helpers are workspace members:** `dpl-lottie-render`
  (`crates/lottie-render`) and `dpl-coach-overlay` (`crates/coach-overlay`).
- **Hazard:** `D:\localdev\platform_devpulse`'s `cargo xtask run-dev` runs
  `taskkill /F /T` on every `xtask.exe` at startup (status.md hazards).
  `/T` kills the whole tree, so a dev app held as a child of `xtask.exe`
  would be killed with it.

## 4. Design

`cargo xtask dev [-- <app flags>]`:

1. **Build (debug):** `cargo build -p devpulselabs -p dpl-lottie-render
   -p dpl-coach-overlay`.
2. **Depot:**
   - If `target/debug/dptools-deps` is missing, stage it through the
     existing `stage_tool_dependencies("target/debug")` (the same
     SHA-verified copy as release).
   - If it exists, leave it: the app SHA-verifies on resolve at boot and
     says so in its log.
3. **Launch, detached:**
   - Start `target/debug/devpulselabs.exe` as its own process group
     (`DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` on Windows, `setsid`
     on Linux), so no `xtask.exe` reaper can take it down.
   - Its stdout/stderr go to `target/debug/dev-run.log`.
   - xtask prints the PID, the log path and the library it opened, then
     exits.
4. **Library:**
   - By default, `--content-root <repo>/content`: the operator's library,
     explicit rather than walked to.
   - When the caller passes its own `--content-root` (any drill), that is
     used instead, and nothing is added.
5. **Flags pass through:** everything after `--` goes to the app, so every
   harness flag in CLAUDE.md works on a dev run.

## 5. Wiring

| File | Change |
|---|---|
| `crates/xtask/src/main.rs` | `"dev" => dev(&arguments)`; `fn dev` (build, depot, detached launch). The spawn is quiet (spawn audit). |
| `CLAUDE.md` Commands | `cargo xtask dev [-- <flags>]`: the dev run. |
| `docs/status.md` | Next step 1 → done. Known broken #16 closed. |

## 6. Proof

1. **The depot gets staged.** With `target/debug/dptools-deps` absent,
   `cargo xtask dev -- --content-root <scratch> --window 1600x900` builds,
   stages and launches. The dev log shows `verified embedded ffmpeg
   resolved … target\debug\dptools-deps…`, and a screenshot from the dev
   run shows the app (scratch root, never the operator's library, for the
   proof).
2. **A Lottie render works on the dev run**, through the debug
   `dpl-lottie-render` (the D0.1b drill, in scratch).
3. **A second run** skips staging and launches in seconds.
4. **The launch is detached.** The app outlives `xtask.exe`: xtask exits,
   and the app's PID is still alive with no parent link (checked with
   `Get-CimInstance Win32_Process`).
5. `cargo green`, then both lanes, then merge. The release exe is rebuilt
   at landing.

## 7. Risks

- **Disk:** the first dev run copies 1.2 GB into `target/debug`. `cargo
  clean` deletes it, and the next dev run re-stages it.
- **The default library is the operator's real `content/`.** A dev run
  writes app state there, just as the release exe does (for example
  `composer-state.json`, known broken in the L passes). Proof runs always
  pass a scratch `--content-root`.
- **The debug exe is slower than release.** That's fine for development.

## 8. Evidence

Implemented 2026-10-19 on `feat/xtask-dev` (based on `42f9577`):
- **The task.** `crates/xtask/src/main.rs` gains `"dev" => dev(&arguments)`
  and these functions:
  - `dev` (build → depot → detached launch);
  - `dev_app_args`, which is pure and puts the operator's library first
    unless a `--content-root`/`CONTENT_ROOT` is named;
  - `dev_library_note`;
  - `workspace_root` (from `CARGO_MANIFEST_DIR`);
  - `detach` (`DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` on Windows,
    `process_group(0)` on Unix).
  
  CLAUDE.md Commands names it. The structure audit still reports 48 flags in
  the census.
- **Unit tests: 3 new, all pass** (`dev_tests`): a bare run opens the
  library explicitly; flags after `--` pass through; a drill's scratch root
  always wins, including from `CONTENT_ROOT`. `clippy -p xtask --all-targets
  -D warnings` is clean.

**Real runs, one at a time, each on a scratch root:**

| # | Proof | Result |
|---|---|---|
| 1 | First dev run, `target\debug` without depot or exe | Debug build 4m04s. The depot was staged (codex, grok, chromium, ffmpeg, pdftoppm), and "DevPULSE tool depot verified … root: …\target\debug\dptools-deps". Launched as pid 60508. `dev-run.log`: `verified embedded ffmpeg resolved … target\debug\dptools-deps\media\ffmpeg\8.1.2` and pdftoppm likewise. |
| 2 | A Lottie render on the dev run (`--lottie-import` at 1600×900) | The cyan shape Lottie is drawn by the **debug** `dpl-lottie-render`; status "Added d01b-box — native Lottie, 2.0s."; footer `42f9577`. |
| 3 | Second run | "depot: … is staged"; **launched in 1 s**. |
| 4 | Detached | The app (pid 63392) is alive after xtask exits, and its parent (pid 64884) **no longer exists**, so it is on its own. A foreign `xtask.exe` (another project's runner) was running at the same time and did not touch it. It was closed gracefully afterwards. |

**Found by the dev run (the reason it exists):**
- **A ScrollArea ID clash.** In debug, egui flags "First use of ScrollArea
  ID EF0B" (the WiredFRONT Project panel) and "Second use of ScrollArea ID
  EF0B" (Effects & Presets). The two scroll areas share one ID, so in
  release they silently share scroll state. → status.md known broken.
- **The debug build renders at `dpr 1.0` on this 150% display, and release
  at 1.5.** The debug exe launched directly gives 1600×900; release gives
  2400×1350 for `--window 1600x900`. So it is debug vs release, not the
  detached launch. The cause is not yet diagnosed. → status.md known
  broken.
