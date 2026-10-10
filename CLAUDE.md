# DevPULSE Labs — agent operating contract

Read `docs/STATE.md` FIRST — it is the living ledger (current state, known
broken, next). Then `docs/ops/SESSION.md` + `docs/ops/resume/CURRENT.md`
(shared durable ops index). Then `docs/plans/roadmap.md` for the program map.
Default ops: Claude skill `milestone-loop` + Cursor skill `session-os`
(same loop; different surfaces).

## The one checkout

This is THE checkout (`D:\localdev\devpulselabs`), on `main`. A second
clone exists at `D:\projects\devpulselabs` (found 2026-10-10; which one the
operator runs is pending — `docs/plans/external-reach-inventory.md` row 55);
never work in it without the operator saying so. The operator double-clicks
`target\release\devpulselabs.exe` — a pass is NOT done until that exe is
rebuilt and its mtime moved. The tool depot is staged beside it from
`dptools-deps/sources.local.toml` (gitignored) via
`cargo xtask stage-tool-deps target/release`; the depot source is
`D:\projects\dptools-deps` (row 56).

## Non-negotiable laws

- **Plan before code** (docs/plans/<pass>.md, 7 sections + §8 evidence).
  Ground-truth by reading the real files in-session; never code from memory.
- **PRs, not direct pushes**: branch → `gh pr create` → merge. Pre-release
  means move fast, not push blind.
- **MERGE ONLY ON DOUBLE-GREEN** (2026-07-29 incident law): never merge a
  PR until BOTH Actions lanes (`green-windows` + `green-linux`) are
  COMPLETED SUCCESS — `gh pr checks <n> --watch` first, then merge. The
  local pre-push gate only proves the pusher's own OS; #139 merged with
  Windows red (unconditional x11rb → `xcb.lib` link death), #142 merged
  mid-run and turned the Linux lane red (clippy in a linux-only file).
  A running check is a red check. Corollary: before authoring any fix,
  `gh pr list` — parallel sessions fix the same defect (it happened
  twice in one hour); an open PR for the defect means adopt it, not
  duplicate it.
- **The product is the EXPERIENCE, not the feature list.** Layman-friendly,
  touch-first, tooltips on controls, finesse. A pass that works but looks/
  feels worse than the old app (`D:\localdev\review-archive\devpulse labs`, READ-ONLY
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
  with --no-verify. The hook proves the pusher's own OS only — the two
  hosted lanes are what prove the other one (see CI below).
- Harness flags (ground-truthed vs `crates/app/src/main.rs` 2026-07-27):
  `--screenshot <png> --shot-frame <n> --route <hub/sub> --focus <id>
  --lib-section <slug> --author-editor --author-container <kind>
  --author-draft <n> --card-tab <slug>
  --window <w>x<h> --mood 0 --viziwiz --viziwiz-snap --present <id>
  --presenter-drill <n> --session-drill <s> --steps-hotkey-drill
  --video-drill <s> [--video-format <f>] --open-rail <hub>
  --open-episode <e> --open-session <s> --open-replay <rel>
  --clone-seed <seed> <show> <ep> --import-drill <dir> --pin-pack <dir>
  --save-comp <name> --export-guide <guide-rel> [<session-rel>]
  --steps-drill <n> --make-slide <asset-rel> --lottie-import <file>
  --inspector --preset <id> --preview-drill
  --fx-drill <clip-rel> <fx-rel> --l4-drill --mblur-drill
  --run-workflow <id> --brainstorm-drill <scaffold>
  --chat-open --chat-authored --chat-proof <mode>
  --seam-port <n> --no-seam --ledger --ai-backfill --content-root <path>` (also honored as the
  `CONTENT_ROOT` env; drill scratch roots use either; `DPL_LEDGER=1` also
  enables the signal ledger)
- EguiPilotAI: `cargo build --release -p egui-pilot-ai-cli` + devtools exe
  `cargo build -p devpulselabs --features eguipilotai-devtools`; then
  `target\release\eguipilotai.exe doctor | inspect devpulse-windows pixvix`.
  Kill any orphaned `devpulselabs` process after CLI runs.

## CI (estate runners again since 2026-08-07 — repo is PRIVATE)

- The repo returned to **private** on 2026-08-07 (operator ruling: personal
  tool; no open-source/licensing exposure wanted). Hosted minutes are
  METERED on private repos, so both lanes moved back to the **estate
  runners**: Proxmox VM 119 (`dpl-win`) + LXC 121 (`dpl-linux`) in
  `.github/workflows/green.yml`.
- The standing security rule is unchanged (`docs/runbook/ci-runners.md`):
  self-hosted runners on `pull_request` are safe ONLY while the repo is
  private and every PR author is the operator. **If the repo ever goes
  public again, deregister the runners FIRST, same day, before anything
  else.**
- The toolchain is pinned by `rust-toolchain.toml` (1.97.1 since #177), so
  a guest's preinstalled Rust cannot silently become the lane's compiler.
- `concurrency: cancel-in-progress` supersedes a stale run when a PR is
  force-pushed — the estate has one Windows slot, and stale runs used to
  sit in the queue holding it.
- Branch protection with required checks needs a paid plan on private
  repos — the double-green law stays discipline-enforced
  (`gh pr checks <n> --watch`, then merge).

## Session end (every session, no exceptions)

1. Land the work on `main` via PR. 2. Rebuild `target\release\
devpulselabs.exe` HERE and verify mtime. 3. Update `docs/STATE.md`.
