# R311 and R312 — organize code and data; the internal build

## Done so far

- **#332**: the two checkouts reconciled; the old clone is marked retired and
  the baseline guard refuses it.
- **#333 (M1)**: every slide component is one folder,
  `crates/slides/src/components/<contract id>/`; no `pub use` in the crate;
  the contracts gate compares by contract id.
- **#334 (M6)**: the AI folder is `<library>/.usr/.ai/`; old `.userdata/chat`
  is copied across once, checked, source untouched.
- **#335 (M7)**: a project's chats and Codex sessions are at
  `.usr/.ai/projects/<project id>/`; the project folder keeps only
  `.devpulse/chat/project.json`.
- All three hosted runs on `main` were green on Windows and Linux.
- **R312** recorded, and `docs/plans/internal-build.md` drafted on
  `feat/internal-build`. No code for it yet.

## Exact next step

BE0 of `docs/plans/backend-postgres.md`: unpack Postgres 18.7 in
`D:/projects/dptools-deps/artifacts/windows-x86_64/postgres/18.7/`, build
pgvector 0.8.7 against it, start it on a scratch data folder, run one
relational and one vector query, stop it, and show the operator the output.
It is blocked: the session's permission check denied unpacking and running
the binaries twice. The operator must allow it or run the unpack. Then BE1.

## Landmines

- Work only in `D:\projects\devpulselabs`.
- `gh run watch` burns the hourly API limit. Read a run once, or use
  `--interval 120`.
- A direct test or drill run must unset `ASSETS_ROOT`, `USERDATA_ROOT` and
  `CONTENT_ROOT` (`env -u`), not set them to empty.
- Bash heredocs break on apostrophes; write edit scripts to the scratchpad.
- Pass `--subject` to `gh pr merge --squash`, or the title can be wrong.
- If git reports unreadable objects after a pull, run
  `git commit-graph write --reachable --split=replace`.
- This computer has no chat project registry
  (`%LOCALAPPDATA%\DevPULSE\chat`), so M7 is proven by tests only.
- A ruling in `requirements.md` is carried out, never asked again.
- The Bash working folder can reset to the retired checkout after a `cd`
  outside the repo. Start every command with `cd /d/projects/devpulselabs`.
- "Postgres is essential" was not a yes to a download. Ask for outside
  downloads in so many words and wait for the word yes.

## Verify commands

```text
cargo xtask baseline check
cargo test -p dpl-chat-projects
cargo test -p dpl-roots
cargo test -p dpl-slides
cargo green
gh run list --branch main --limit 1
```
