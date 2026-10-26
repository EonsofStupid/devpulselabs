# ST1 — portable storage, implementation and verification

## Authorization and checkout

The operator confirmed the portable-storage plan was approved, then answered
"yes" to reading its untracked copy from `D:/projects/devpulselabs` and
implementing in an isolated checkout here. Do not re-ask either approval.

Worktree: `D:/localdev/devpulselabs/.worktrees/portable-storage-st1`.
Branch: `feat/portable-storage-st1`, base `530dd17` (RL #296).
Parent checkout remains on `wip/c1-paused`; it contains only session notes
from this task and pre-existing `.claude/settings.local.json`.
The source checkout was read for the plan and instructions, never modified.

## Implementation

Read the copied approved `docs/plans/portable-storage.md`, especially new
section 4c. ST1 changes are implemented and locally verified:

- `crates/roots`: serde/serde_json/toml-only library identity, config,
  layout, stateful resolver, bounded drive candidates, explicit setup,
  no-clobber staging and scratch overrides. Typed library/account ids.
- All root consumers moved out of capture to `dpl-roots`; no capture
  re-export. Video delegates to the same resolver. Standalone CLI errors
  are handled. Composer state uses .userdata with read-only old-file
  fallback; common temp/cache replaces production scratch sites.
- `app/src/storage_startup.rs`: first-launch choice and failure gate before
  any stores/upgrades/drills. Retry/Quit for unavailable config. It runs
  the native event loop, then the normal app in the same process; eframe's
  local source confirms event-loop reuse. Screenshot code is shared in
  `app/src/screenshot.rs`.
- `xtask stage-roots <program-dir> <existing-library>`; workspace tests get
  private absolute scratch roots rather than ambient operator folders.
- Workspace/lock/census/architecture/ADR/status/docs updated together.

ST2 polling, live IO lifetimes and retained edits; ST5 Locations; ST3
relocation; ST4 migration/accounts/AI store split remain future passes.

## Verification completed

20 Windows root tests passed, including two review fixes (roots.toml path
collision and files occupying ring paths). Core Clippy with warnings denied,
format check and locked offline metadata passed. The final gate includes
the typed account path and the Windows drive-prefix review fix.

The broader test run **21910 completed successfully**:

```powershell
$env:CARGO_PROFILE_DEV_DEBUG='0'
$env:CONTENT_ROOT='D:/localdev/devpulselabs/.worktrees/portable-storage-st1/target/st1-tests/content'
cargo test --offline -j 2 -p dpl-roots -p dpl-composer -p dpl-video -p dpl-index -p dpl-engine-capture -p dpl-engine-browser -p dpl-viziwiz -p devpulselabs -p xtask
```

It compiled the fresh target in 25m14s and all selected package tests passed.
The final root source also passed its separate 20-test run. Native executable
build **25345 passed** in 3m22s. Six captures passed via the approved
`target/st1-native-proof.ps1`; all PNGs in `docs/proof/portable-storage-st1`
were inspected. Fixtures/logs: `target/st1-native-20261026-015432-1524`.
The two startup states are clean at both sizes. The configured control shows
the existing WiredFRONT hub; its compact Sessions card extends behind the
footer (no layout change in this cut; the report states the limit).

Initial **cargo green passed in session 67178**, log
`target/st1-green-20261026-0022.log` (the filename preceded the native-run
approval wait). Env: dev/test debug info 0, build jobs 8, offline true.
Fmt, lock, docs status, spawn, structure, contracts and workspace Clippy
passed. Workspace tests: **1187 passed, 0 failed, 6 existing live-tool tests
ignored** (163 result lines including doc tests). Release compilation and
renderer smoke passed. No operator processes or binaries were touched.

Late review fixed Windows prefix validation in roots/src/disk.rs: do not
stat bare `D:` before adding its RootDir component (that would consult the
drive's CWD). Its separate final root test run 53420 passed all 20 tests.
The final incremental **cargo green also passed in session 8725**, after
that edit: `target/st1-green-final.log`. Smoke: 16,616 shape pixels,
10,634 text pixels and 60 exported frames. No build sessions remain active.

The scratch native proof script now targets RELEASE and writes fresh PNGs
under `docs/proof/portable-storage-st1/release`, preserving initial debug
captures. After six launch captures it uses Windows UIA, scoped only to its
own new process, to invoke the imported-library button, verify roots.toml
and marker agreement, verify all rings and assert that the main window
opens in the same process. It closes only its own process. All six release
captures and that native interaction **passed in session 78316**. Every
PNG was inspected. Release fixture: `target/st1-native-20261026-021335-0102`.
The initial release wrapper returned before the GUI process had finished;
it was corrected to use Start-Process with explicit waiting. No app fix
was needed. The command prefix is approved.

Captured release SHA-256:
`90BBF6B361DD07AC84FF4F7C0ED7DEAF33D6CC061F7547D5F8CB2FC44B7FABFF`.
The report, plan evidence and status reflect these final local outcomes.

## Next

1. Commit the reviewed ST1 files; open the PR and satisfy the required
   Windows/Linux double-green delivery gate. No competing storage PR was
   present in the last open-PR check (only dependency PRs #268-275).
2. Do not modify the operator's source checkout. Keep its release and
   existing library untouched. The isolated release is the verified build.
3. Record the final PR/CI/delivery result in the parent resume notes.

Screenshots, native interaction and release gate passed; no commit, push,
PR or merge at this journal checkpoint. Source and
operator libraries remain untouched. An untracked `Microsoft/` directory
was generated by PowerShell's module cache in this worktree; it is not part
of the change and must not be staged. Do not stage via `git add .`.
