# 2026-10-12 — docs-one-truth: P1 in PR

Milestone `docs-one-truth` (plan `docs/plans/docs-one-truth.md`, approved by
the operator 2026-10-12). Checkout: **`D:\projects\devpulselabs`**.

## Done so far

- **D0.1b merged** (#260, `aa81b7d`). The exe was rebuilt 03:49 and verified
  on the real exe.
- **P1 on branch `docs/one-truth`:**
  - the tag `docs-history-2026-10-12` (pushed);
  - `docs/requirements.md`: 275 verbatim operator requirements;
  - `docs/status.md` and the doc map `docs/README.md`;
  - 247 docs deleted;
  - every reference fixed (0 dangling);
  - hooks, skills and agent files read `status.md`.
  - Evidence: plan §8.
- **Developer Mode is ON** (operator, 2026-10-12). Unprivileged symlinks are
  proven.
- **Rulings recorded** in the plan's §4:
  - delete, tagging first;
  - one `status.md`;
  - real symlinks;
  - **Locations** (existing folders as linked locations; offline offers
    Locate or Remove);
  - DQ7 (separate the shipped bundle from `assets\`).

## Exact next step

1. **P1:** push through the gate, then `gh pr create`. Both lanes run,
   because the PR touches `crates/` comments and the coach-daemon
   `/health` string. Wait for both with `gh pr checks <n> --watch`, merge,
   rebuild the exe here and check its mtime.
2. **P2:** merge each subject into its one design doc (docs/README.md marks
   the **P2** rows).
3. **P3:** `design/library.md`, the one-library law (embedded / imported /
   linked, Locations, one index, one resolver). The operator approves it
   before any code.

## Landmines

- The Bash tool resets cwd to `d:\localdev\devpulselabs` after every call:
  always `cd /d/projects/devpulselabs &&` first.
- **`D:\localdev\platform_devpulse`'s `cargo xtask run-dev`** (a parallel
  session) runs `taskkill /F /T` on every `xtask.exe` at startup. A gate
  that dies silently with exit 1 → check its `run-dev.log`, then re-run.
- Run `cargo fmt --all -- --check` before every push.
- Docs-only PRs skip CI (paths-ignore): run `gh workflow run green.yml
  --ref <branch>`.
- There is ONE Windows estate runner. Lanes queue; Windows takes ~40 min.
- `cargo clean` deletes the depot in `target/release/dptools-deps`.
  Re-stage it with `cargo xtask stage-tool-deps target/release`.
- Untracked `out/` and `.vscode/` are not ours. Never commit them.
- Never push `D:\localdev\devpulselabs`'s local branches.
- Windows PowerShell 5.1 `New-Item -ItemType SymbolicLink` needs admin even
  with Developer Mode. Probe with `cmd /c mklink` or the Rust tests instead.
- The quote tooling (verifier, harvest JSON, renderer) is in the session
  scratchpad, not the repo. To add a requirement, append the next R-id to
  `docs/requirements.md` by hand.

## Verify commands

```
git -C D:/projects/devpulselabs status --short
git -C D:/projects/devpulselabs log --oneline -3
gh pr list
git ls-remote --tags origin docs-history-2026-10-12
find D:/projects/devpulselabs/docs -name '*.md' | wc -l    # 82 after P1
```
