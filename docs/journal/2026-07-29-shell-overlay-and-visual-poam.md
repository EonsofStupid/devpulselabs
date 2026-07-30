# 2026-07-29 — shell-overlay-and-visual-poam

## Done so far

- **Inspector is an overlay** (#148, merged double-green): PANEL zone no
  longer docks — closed by default (`Zones.panel == Rect::ZERO`, well spans
  to the root's right edge), opens as a floating plate over the well via the
  topbar toggle or `--inspector`. Law 8 added to `docs/shell.md`; kittest
  `the_inspector_overlays_the_content_instead_of_docking` pins it. Release
  exe rebuilt (mtime moved).
- **STATE entry landed** (#150) after resolving the concurrent-session
  conflict (kept both the Background-tab and inspector entries).
- **Visual-experience POAM ratified** (#153, docs-only):
  `docs/plans/poam-visual-experience.md` — V1 backdrop policy, V2 mood card
  backgrounds, V3 SVG icon language, V4 real rail buttons, V5 Home showcase.

## Exact next step

Operator directive (2026-07-29, verbatim): "lets take the time to pull
lucide and then properly own it as part of coldlight, we will have the SVG
engine that is based off the airbnb engine airvx and establish baselines
for our diverse and default polished stylings." → This starts V3 with
Lucide as the baseline set. Next: ground-truth + ADR (SVG→egui path, new
crate through the architecture checklist) + `docs/plans/<pass>.md`, operator
approval, then code.

## Landmines

- Docs-only PRs get ZERO Actions runs BY DESIGN (green.yml paths-ignore) —
  local `cargo green` is the whole gate; don't wait on phantom checks.
- Concurrent sessions land PRs into main (STATE.md conflicts routine) —
  `gh pr list` before authoring anything.
- PS 5.1 mangles embedded quotes in native args — use Bash heredocs for
  commit messages / PR bodies.
- Inspector cards spill under the plate's right margin (parity-kept from the
  docked era, noted in the plan §8) — future polish, not a regression.
- Proof-shot exe runs are SEQUENTIAL; screenshots at REAL window sizes.

## Verify commands

- `cargo test -p dpl-shell` (22 pass, incl. the two zone-contract tests)
- `target\release\devpulselabs.exe --inspector --window 1100x700 --screenshot <png>`
- `git log --oneline -5` → contains #148, #150, #153 merges
