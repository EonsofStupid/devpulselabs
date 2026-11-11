# Absorbing the guided-workflow work — 2026-11-10

R300: finish scattered work inside the boundaries that already exist.

## What was found
GitHub and `D:\projects\devpulselabs` held only `main`. The mess was in the old
clone: ten branches, three worktrees and uncommitted files. All its branches had
landed except `wip/c1-paused` and a parked process-supervisor branch. One worktree
held GF0, GF1a and GF1b uncommitted. A second session had recorded the same request
as a separate lane, so requirement numbers R294–R296 collided.

## What landed here
- GF1a and GF1b in the crates they already touched.
- GF0 as `dpl_workflows::interop`, with no new crate and no new normal dependency.
- Research, plans, proofs and journals. Requirements renumbered R297–R299.

## Left out on purpose
- `delivery.yml` and its scripts, plan and runbook. GitHub's built-in auto-merge
  replaces the bot (operator ruling).
- Two GF1c files that were never wired in. F3–F6 stay open in
  `docs/plans/coach-correctness.md`.

## Saved as local tags
`retired/guided-workflow-snapshot`, `retired/old-clone-uncommitted`,
`retired/wip-c1-paused-final`, `retired/codex-subprocess-lifecycle`,
`retired/stash-crate-consolidation-review`.

## Next
One PR each, no new crates: process supervisor into `dpl-proc`, Foundry into
`dpl-coach-pack`, chat modes, MCP and brainstorm seats into `crates/chat/*`.
