# 2026-10-27 — portable-storage: ST1b, your library moves in

Milestone `portable-storage`, pass ST1b. Plan: `docs/plans/portable-storage.md`
§4.7 (ruled 2026-10-26) and §4d. Checkout **`D:\projects\devpulselabs`**,
branch `feat/portable-storage-st1b`.

## Before this pass, the same session

- **ST1 landed from a parallel session** (#297, `9ab0c8a`), built in an
  isolated worktree under `D:\localdev\devpulselabs\.worktrees\`. THE
  checkout's exe was rebuilt from it here (stamp `9ab0c8a · 2026-10-27
  03:27z`).
- With ST1 the exe no longer reached the operator's `content\`, `assets\`
  and `.userdata\`. The operator ruled "Move them next" and, on where (R291):
  *"stored inside the project properly what part of portable? you let the
  person decide"*; then "Approve" for ST1b.
- **R290** (AgentHub, after ST) and **R291** landed as docs (#298). Docs-only
  PRs skip both lanes (`paths-ignore`), so #298's lanes were dispatched by
  hand (`gh workflow run green.yml --ref <branch>`) and merged on their
  green.

## ST1b — what landed

- **`dpl-roots`:** `project_of_build` (a build's program folder is its
  project, two fixed levels); `migrate.rs` (`plan_move`, `move_in`: copy,
  byte-for-byte check, identity and `roots.toml` last, undo on failure);
  `setup.rs` shares `create_layout` / `publish_identity` / `write_config`.
- **The first screen:** "Keep my library in this project" for a build;
  **Bring in my existing library…** as its own page (name, a From / Into /
  what-moves card, Move it, Change where…, progress); `--bring-in`.
- **`cargo xtask move-library --dry-run`**; the move itself is the tap.
- **`.eguipilotai.toml`:** each profile defaults `CONTENT_ROOT` to
  `target/eguipilotai/default/content`.

## Exact next step

1. Land the PR (both lanes), rebuild the exe from main, and tell the
   operator to tap the move.
2. Then ST2. Its Disconnected screen lives in the startup window, which
   EguiPilotAI can't reach (the runtime attaches in the main app only; a
   second stdio transport would lose a frame to the first's reader thread).
   Plan one process-wide transport handed from the startup window to the
   main one.

## Landmines

- **Never run `--bring-in` or the first screen from `target\debug` or
  `target\release` without a scratch `--content-root`:** those exes attune
  to THE checkout, whose real rings are the source. Proof runs copy the exe
  into `target/st1b-proof/{project/target/debug,portable}/` with synthetic
  rings (the prep script is in the session scratchpad; the commands are in
  the plan's §8).
- `cargo test -p devpulselabs` rebuilds `target/debug/devpulselabs.exe`
  without the devtools feature; rebuild with `--features
  eguipilotai-devtools` before a pilot run.
- Docs-only PRs get no checks: dispatch the workflow on the branch.

## Verify commands

```sh
cd /d/projects/devpulselabs
git status --short && git log --oneline -4
gh pr list --state open
cargo test -p dpl-roots && cargo test -p devpulselabs storage_startup
target/debug/xtask.exe move-library --dry-run D:/projects/devpulselabs
```
