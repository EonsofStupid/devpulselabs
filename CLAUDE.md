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

- Gate: `cargo green` · exe: `cargo build --release -p devpulselabs -p dpl-lottie-render`
  (the Lottie render helper ships BESIDE the exe — the façade spawns it, ADR-009)
- The gate is WIRED as a pre-push hook (`.githooks/pre-push`; fresh
  clones run `git config core.hooksPath .githooks` once). Never bypass
  with --no-verify. NOTE: GitHub branch protection / required checks are
  Pro-gated on this private repo — the local hook IS the enforcement;
  a self-hosted Actions runner + protection becomes worthwhile only if
  the repo goes Pro or public.
- Harness flags: `--screenshot <png> --shot-frame <n> --route <hub/sub>
  --window <w>x<h> --mood 0 --pixvix --pixvix-snap --present <id>
  --presenter-drill <n> --session-drill <s> --steps-hotkey-drill
  --video-drill <s> [--video-format <f>] --ink-drill <rel> --open-rail
  --open-episode --open-session --open-replay --clone-seed
  --import-drill <dir> --pin-pack <dir> --save-comp <name>
  --export-guide <guide-rel> [<session-rel>] --steps-drill <n>`
- EguiPilotAI: `cargo build --release -p egui-pilot-ai-cli` + devtools exe
  `cargo build -p devpulselabs --features eguipilotai-devtools`; then
  `target\release\eguipilotai.exe doctor | inspect devpulse-windows pixvix`.
  Kill any orphaned `devpulselabs` process after CLI runs.

## CI runners (self-hosted, free)

- `dpl-win-laptop` (labels: dpl-win) runs at `C:\actions-runner` on this
  laptop — `.github/workflows/green.yml` runs the gate on every PR.
  NOT yet a service: after a reboot, restart it (`C:\actions-runner\
  run.cmd`) or, ONCE from an elevated shell, register the logon task:
  `schtasks /Create /TN dpl-actions-runner /TR C:\actions-runner\run.cmd /SC ONLOGON`.
- Ubuntu/Wayland mini-PC lane: register a runner there with label
  `dpl-linux` (repo Settings → Actions → Runners → New self-hosted
  runner, or `gh api -X POST repos/EonsofStupid/devpulselabs/actions/
  runners/registration-token`), then uncomment the `green-linux` job in
  green.yml. The app targets BOTH: Windows laptop + Wayland Ubuntu
  mini-PC — cross-support is a product requirement, not a nicety.
- INTERIM Linux lane via WSL (Ubuntu-26.04 on this laptop): Rust 1.92
  is already installed (rustup, no sudo). Cross-checking from Windows
  dies at khronos-egl's build script — the check must run INSIDE Linux.
  ONE operator command finishes the setup (needs sudo):
  `sudo apt-get update && sudo apt-get install -y build-essential pkg-config libssl-dev libegl1-mesa-dev libgl1-mesa-dev libwayland-dev libxkbcommon-dev libx11-dev libxcursor-dev libxrandr-dev libxi-dev`
  After that, any session can run the Linux gate:
  `wsl -e bash -lc "source ~/.cargo/env; cd /mnt/c/localdev/devpulselabs && cargo check --workspace"`
  (and a WSL-side runner with label dpl-linux is an option until the
  mini-PC takes the lane).

## Session end (every session, no exceptions)

1. Land the work on `main` via PR. 2. Rebuild `target\release\
devpulselabs.exe` HERE and verify mtime. 3. Update `docs/STATE.md`.
