---
updated: 2026-10-13
active_lane: docs-one-truth
dirty: false
---

# Status — where DevPULSE Labs is right now

This page is **rewritten, never appended**. History is `git log`, the PRs,
and the tag `docs-history-2026-10-12` (every doc before the one-truth
reorganization).

Read in this order: this page → [`requirements.md`](requirements.md) (the
operator's words) → [`README.md`](README.md) (the map of every doc).

**Updated:** 2026-10-13, main at `d3c0a73` (#262). The exe was rebuilt
2026-10-12 10:49 and verified on the real exe (footer `d3c0a73`).

## Now

**`docs/plans/docs-one-truth.md`** (approved 2026-10-12): the docs become one
organized set, and the library becomes one.

| Pass | What | State |
|---|---|---|
| P1 | requirements ledger · this page · the doc map · 247 finished/superseded/stale docs deleted (tagged first) · every reference fixed (0 dangling) | **merged** (#262, double-green) |
| P2 | one design doc per subject; ADRs into `adr/` | next |
| P3 | `design/library.md`, the one-library law, for operator approval | after P2 |
| L1… | code passes that make the library one | after P3 approval |

**D0, laptop dependencies** (`docs/plans/d0-laptop-deps.md`): paused behind
the one-truth program, because D0.1c is now library work.
- D0.0 clean rebuild: done.
- D0.1a honest media (#258): done.
- D0.1b depot-only helpers (#260): done.
- D0.1c roots: folded into the L passes.
- D0.2 startup dependency check, D0.3/D0.4 camera, D0.5 gate truth, D0.6
  portable release: still to come.

**CHAT + AI** (`docs/plans/chat-native-program.md`): paused behind D0.

## Known broken (verified 2026-10-12)

1. **A Gallery rescan hides material that is still on disk.**
   - `reindex` scans only the top level of `content/inbox`
     (`crates/index/src/lib.rs:449-474`, `:823-860`).
   - Every row is stamped `location:"inbox"` (`:873`).
   - Hidden: ViziWiz bucket captures, renders, elements, pack ingests,
     `.mov`/audio/Lottie imports. → L1.
2. **"Link in place" silently copies when Windows refuses a symlink.**
   - `import_reference` falls back to a full copy and still records
     `source:"reference"` (`crates/composer/src/import.rs:230-234`).
   - Developer Mode was turned on 2026-10-12, and the app can now create
     symlinks unprivileged (proven: `cmd mklink`, and `dpl-index`'s symlink
     test runs instead of skipping). The silent copy must still go.
     → L passes and D0.2.
3. **The library is about 15 parallel systems, not one.** WiredFRONT and
   Workbench see different material: inbox slides never reach Workbench, a
   Kept guide vanishes from Workbench, and Pictures is cut to the newest 61
   before filtering. → `docs-one-truth.md` §3, L passes.
4. **Four folder-finding walkers disagree** (content root, `locate_asset`,
   `dpl-doc` seeds, a dead `dpl-comp` copy). An exe copied into the repo
   (`out/portable-test`) adopted the operator's real library. → L passes.
5. **The shipped bundle shares `assets/` with the working ring.**
   `promote_asset` would delete a shipped backdrop (latent; no runtime
   caller). DQ7 ruled: separate them now. → L passes.
6. **The editor status bar is too short for its two lines.**
   `status_h = 58.0`, `crates/composer/src/lib.rs:3445`. Line 2 straddles
   the frame at 1600×900 and 1280×800.
7. **A Lottie renderer that exists but fails is re-opened every frame.** That
   is one spawn per tick. A missing renderer fails fast. → D0.2 follow-up.
8. **`content/artifacts.db.corrupt`** (2026-07-24) is unexplained. Never
   delete it; investigate it read-only.
9. **The release zip ships without the tool depot.** → D0.6.
10. **Six ignored drills do not run against the depot.** → D0.5.
11. **The Codex version gate** and unhandled server requests. → chat A2.
12. **A second launch opens a second, half-broken window** instead of raising
    the running one. The new instance fails the estate lock (`FjallError:
    Locked`) and the seam port bind (os error 10048), then opens anyway.
    The operator asked (2026-08-05) for the app to bring the running window
    to the front. This needs its own PR.
13. **No engine shows its startup** (R276). The library index, chat accounts
    and chat profiles (three in-process Turso files), the estate (`fjall`),
    the tool depot and providers start silently, and a failure is only a
    log line. Per-engine states (Starting / Ready / Error, plus progress for
    long work) → D0.2.
14. **Analytics can write but not query.** DataFusion (R241) is not in the
    dependency tree; only the Parquet writer exists.
15. **The UI still states the old split.**
    - The wiredFRONT hub blurb says *"ADR-010 is this hub's law"*, but
      ADR-010 is the stale, unratified draft that was deleted.
    - The Library card says *"Raw material, never app ingredients"*, the
      agent-coined "two-libraries law" that contradicts "it is all one".
    - → L passes.
16. **A ScrollArea ID clash** (found by the first dev run, 2026-10-19). In
    debug, egui flags "First use of ScrollArea ID EF0B" in the WiredFRONT
    Project panel and "Second use …" in Effects & Presets. The two scroll
    areas share one ID, so in release they silently share scroll state.
17. **The debug build renders at `dpr 1.0` on this 150% display, and release
    at 1.5.** For `--window 1600x900`, debug gives 1600×900 and release
    2400×1350. It is debug vs release (a direct debug launch does the same),
    and the cause is not yet diagnosed. UI judged on a dev run is at a
    different scale than the operator sees.

## Hazards outside this repo

- **`D:\localdev\platform_devpulse`'s `cargo xtask run-dev`** (often a
  parallel Claude session) runs `taskkill /F /T` on **every** `xtask.exe` at
  startup. A `cargo green` that dies silently with exit 1 was killed by it:
  check its `run-dev.log`, then re-run. Never touch that project.
- **The second clone, `D:\localdev\devpulselabs`**, is unused. Never work
  there, and never push its local branches (`wip/c1-paused`, `park/…`).

## Next, in order

The operator's direction (2026-10-13, R278): **develop on a dev run; finish
everything incomplete first, each fully tested.**

1. **`cargo xtask dev [-- <app flags>]`, the dev run: built** (`feat/xtask-dev`,
   proven on real runs; `docs/plans/dev-run.md` §8). It builds the debug app
   with both helpers, stages the depot into `target\debug` on the first run,
   and launches detached on the operator's library. The log is
   `target\debug\dev-run.log`.
2. **Finish the incomplete work** on the dev run, each with unit tests and
   real-run drills. The dev run's own findings come first (known broken #16
   ScrollArea ID clash, #17 debug DPI scale). Then:
   - **PR #222**: the job board, preset looks and workflow dry run, finished
     on the new docs (*"build the missing consumer, never delete"*);
   - **chat tabs**: close and clear a tab (only "New chat" exists today), and
     prove New chat works on the real run;
   - **every vendor CLI a registered, updatable depot subprocess**: Claude
     Code (none today; A4), and an in-place, SHA-verified tool update path
     (`stage-tool-deps` refuses an existing root);
   - the known-broken list above.
3. P2, then P3 for operator approval.
4. The L passes (one library), then D0.2 onward, then CHAT + AI. Codex (A2)
   and Claude Code (A4) each run on their own subscription first (R33).
5. Then **the knowledge engine (R275–R277)**: its own plan, from the
   2026-10-13 sourced research.
   - One store with a project id on every row; projects proposed from
     where sessions ran, confirmed in one tap.
   - Keyword search first; embeddings are one datapoint among keyword,
     project, provider and time (R277).
   - In-process Turso in its own derived-data file (the `turso` crate has
     no dense ANN index: exact scan).
   - `llama-server --embedding` as a depot subprocess.
   - Honest per-engine startup states.
   - An LLM lab in Workbench `/experiments` (R139, R140) for the layer and
     shard experiments of R277.
