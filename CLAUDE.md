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
- **Structure is law**: `docs/design/architecture.md` (layer stack, dep
  ceilings, named exceptions, new-crate checklist) is enforced by the
  `structure` phase of `cargo green` — amend the doc AND the audit table
  in `crates/xtask/src/main.rs` in the SAME commit.
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
- Harness flags (ground-truthed vs `crates/app/src/main.rs` 2026-07-27):
  `--screenshot <png> --shot-frame <n> --route <hub/sub> --focus <id>
  --window <w>x<h> --mood 0 --viziwiz --viziwiz-snap --present <id>
  --presenter-drill <n> --session-drill <s> --steps-hotkey-drill
  --video-drill <s> [--video-format <f>] --open-rail <hub>
  --open-episode <e> --open-session <s> --open-replay <rel>
  --clone-seed <seed> <show> <ep> --import-drill <dir> --pin-pack <dir>
  --save-comp <name> --export-guide <guide-rel> [<session-rel>]
  --steps-drill <n> --make-slide <asset-rel> --lottie-import <file>
  --fx-drill <clip-rel> <fx-rel>
  --run-workflow <id> --chat-open --chat-authored --chat-proof <mode>
  --seam-port <n> --no-seam --content-root <path>` (also honored as the
  `CONTENT_ROOT` env; drill scratch roots use either)
- EguiPilotAI: `cargo build --release -p egui-pilot-ai-cli` + devtools exe
  `cargo build -p devpulselabs --features eguipilotai-devtools`; then
  `target\release\eguipilotai.exe doctor | inspect devpulse-windows pixvix`.
  Kill any orphaned `devpulselabs` process after CLI runs.

## CI runners (self-hosted, free — on the estate since 2026-07-28)

- BOTH lanes live on the dedicated server (the server1 / ssh.clyffy.ai
  Proxmox host, 88 cores / 128 GB). The LAPTOP IS NOT A RUNNER — its
  registration was removed 2026-07-28; never restart `C:\actions-runner`
  here. (Leftover: `schtasks /Delete /TN dpl-actions-runner /F` from an
  elevated shell, once.)
- Windows lane: `devpulse-runner-win-01` (labels: dpl-win) — Proxmox VM
  119, runner service `actions.runner.EonsofStupid-devpulselabs.
  devpulse-runner-win-01` running as SYSTEM at `C:\actions-runner`.
  Toolchain (all machine-usable by SYSTEM): git 2.55, rustup with the
  pinned 1.92.0 (default profile — clippy+rustfmt) in the SYSTEM
  profile, VS Build Tools MSVC 14.44 + Win11 SDK 26100. Reach the VM
  from server1 via `qm guest exec 119 -- ...` (guest agent; quote
  Windows paths — bare backslashes die in the ssh/sh layers, base64 a
  .cmd through certutil for anything with embedded quotes).
- Linux lane: `devpulse-runner-lnx-01` (labels: dpl-linux, rust) — LXC
  121 on the same host; `dpl-linux-clyffy-lab` (ARM64, dpl-linux) rides
  as extra capacity.
- The 16GB-era LNK1318 env cures in green.yml (`debug=0`, `jobs=8`)
  stay until the Windows VM proves it doesn't need them (it boots with
  16 GB too).
- Rebuild-from-blank, snapshot policy, heartbeat, and the never-public
  security rule: `docs/runbook/ci-runners.md` (idempotent bootstrap
  scripts beside it — LLVM/libclang included; also `-VerifyOnly` /
  `--verify-only` audits for a runner that looks online but won't pick
  up jobs).

## Session end (every session, no exceptions)

1. Land the work on `main` via PR. 2. Rebuild `target\release\
devpulselabs.exe` HERE and verify mtime. 3. Update `docs/STATE.md`.
