# 2026-10-19 — dev-run-findings: the dev run's two findings, closed

Milestone `dev-run-findings`: the two defects the first dev run surfaced
(R278: develop on a running debug app). Checkout: **`D:\projects\devpulselabs`**.

## Done so far

- **Merged since the last journal:** docs-one-truth P1 (#262), notes (#264,
  #266: R279–R281, the chat program's §4.3.7), dependencies (#261, #265),
  and **the dev run, `cargo xtask dev`** (#267). The release exe was rebuilt
  from `7eb0a5b` at 03:08 and verified (footer `7eb0a5b`).
- **Widget-ID clash: fixed** (branch `fix/dev-run-findings`).
  - Cause: the editor built its sibling panels with
    `ui.new_child(UiBuilder::new().max_rect(..))` and no salt, so the Project
    and Effects scroll areas derived one ID and shared scroll state.
  - Fix: both panel builders in `crates/composer/src/lib.rs` take the
    panel's name and salt the child with it.
  - Test: `no_editor_tab_paints_a_widget_id_clash` runs every editor tab
    headless at 1600×900 and fails on any egui "First/Second/Double use of …
    ID" warning. Red without the fix (`ScrollArea ID D92C` twice), green with
    it; composer's 66 tests pass and clippy is clean.
  - Dev run at `--route wiredfront/editor --window 1600x900`: no clash
    overlays.
- **"Debug renders at dpr 1.0, release at 1.5": not a defect.**
  - Both exes are per-monitor DPI aware (measured from outside with
    `GetDpiAwarenessContextForProcess`: awareness 2), and neither embeds a
    manifest.
  - The only display now is a 2560×1440 monitor at 96 DPI (scale 1.0).
    Every 2400×1350 shot is from 10-10 and 10-12, on a 150% display; every
    10-19 shot, release included, is 1600×900.
  - A fresh release shot and the debug shot of the same route are both
    1600×900 at `dpr 1.00`. Recorded as a hazard in `status.md`.
- **One-truth cleanup:** `docs/plans/dev-run.md` (landed with #267) is
  deleted, and xtask's comments cite R278 instead. The findings plan is not
  committed: its evidence lives in the PR and `status.md`.

## Exact next step

1. Push `fix/dev-run-findings` through the gate, `gh pr create`, then
   `gh workflow run green.yml --ref fix/dev-run-findings` only if the PR's
   own run does not start (it touches `.rs`, so it should).
2. Merge on double-green, rebuild the release exe from the merge commit,
   check its mtime and footer.
3. Then the incomplete list in `docs/status.md` "Next" (item 2), in order:
   PR #222, chat tabs, Claude Code as a depot subprocess plus the in-place
   tool update, the eight dependency PRs #268–#275, the known-broken list.

## Landmines

- `D:\localdev\platform_devpulse`'s run-dev reaper kills every `xtask.exe`:
  a silent exit-1 gate means re-run it.
- Markdown-only PRs need `gh workflow run green.yml --ref <branch>`
  (`paths-ignore`).
- A transient "Could not resolve host: github.com" on push: retry.
- Screenshot pixel size follows the attached display; compare by the
  footer's `dpr`.
- `.vscode/` and `out/` are untracked and not ours: never commit them.

## Verify commands

```sh
cd /d/projects/devpulselabs
git fetch -q && git log --oneline -3 origin/main
gh pr list --state open
cargo test -p dpl-composer --lib no_editor_tab_paints_a_widget_id_clash
```
