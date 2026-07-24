# DevPULSE Labs — agent operating contract

Read `docs/STATE.md` FIRST — it is the living ledger (current state, known
broken, next). Then `docs/plans/roadmap.md` for the program map.

## The one checkout

This is THE checkout (`C:\localdev\devpulselabs`), on `main`. There is no
other worktree. The operator double-clicks `target\release\devpulselabs.exe`
— a pass is NOT done until that exe is rebuilt and its mtime moved.

## Non-negotiable laws

- **Plan before code** (docs/plans/<pass>.md, 7 sections + §8 evidence).
  Ground-truth by reading the real files in-session; never code from memory.
- **PRs, not direct pushes**: branch → `gh pr create` → merge. Pre-release
  means move fast, not push blind.
- **The product is the EXPERIENCE, not the feature list.** Layman-friendly,
  touch-first, tooltips on controls, finesse. A pass that works but looks/
  feels worse than the old app (`C:\localdev\devpulse labs`, READ-ONLY
  reference) is a regression. The experience inventory (docs/plans/
  experience-inventory.md, once authored) gates UI passes.
- **Proof on the real exe**: `cargo green` (fmt+clippy+tests+release build),
  then drills/screenshots. Screenshots at REAL window sizes
  (`--window <w>x<h>`), never only maximized — the footer bug hid there.
- `/content` is gitignored operator data: never commit, never author into,
  never delete. Sources are never mutated (flatten/captures file NEW).
- No `#[allow]` attributes; no re-export shims on moves; ffmpeg is a
  subprocess, never a crate dep (ADR-009).
- Proof-shot exe runs are SEQUENTIAL (parallel runs corrupted the index).

## Commands

- Gate: `cargo green` · exe: `cargo build --release -p devpulselabs`
- Harness flags: `--screenshot <png> --shot-frame <n> --route <hub/sub>
  --window <w>x<h> --mood 0 --pixvix --pixvix-snap --present <id>
  --presenter-drill <n> --session-drill <s> --steps-hotkey-drill
  --video-drill <s> [--video-format <f>] --ink-drill <rel> --open-rail
  --open-episode --open-session --open-replay --clone-seed`
- EguiPilotAI: `cargo build --release -p egui-pilot-ai-cli` + devtools exe
  `cargo build -p devpulselabs --features eguipilotai-devtools`; then
  `target\release\eguipilotai.exe doctor | inspect devpulse-windows pixvix`.
  Kill any orphaned `devpulselabs` process after CLI runs.

## Session end (every session, no exceptions)

1. Land the work on `main` via PR. 2. Rebuild `target\release\
devpulselabs.exe` HERE and verify mtime. 3. Update `docs/STATE.md`.
