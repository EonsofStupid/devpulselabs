---
name: session-os
description: >-
  Default agent operations for DevPULSE Labs. Boot from disk, identify opens,
  track milestones, journal near 75-85% context, resume after compact or clear.
  Use on every new chat, after compact/clear, when context is high, when starting
  a milestone, or when the operator mentions session OS, resume, journal, or ops ledger.
---

# Session OS — default operations

Any model. Progressive skill: keep this file loaded for the loop; open
references only when needed.

**Not** the product Parquet ledger (`dpl-ledger`). Ops live under `docs/ops/`.

## Boot (every session — do this first)

1. Read `docs/STATE.md` (top entries only unless needed).
2. Read `docs/ops/SESSION.md` (active lane, NEXT, opens, `dirty`).
3. Read `docs/ops/resume/CURRENT.md`.
4. Append `session.boot` + `resume.loaded` to `docs/ops/signals.jsonl`.
5. Identify opens (git status/branch, open PRs for the lane) → `open.identified`.
6. Confirm NEXT with the operator if ambiguous; do not steal parallel lanes.

## Loop

```
boot → pick NEXT → plan (≤3 passes) → [non-trivial: multi-model] →
execute one cut → signal → journal if budget/end → repeat
```

### Planning caps

- **≤3** plan passes (`plan.pass` with `n`).
- **≤2** web-research subagents (`research.web` with `n`).
- First grounding prefers Fable (`grounder`); Opus closes gaps
  (`gap-closer`); then halt for operator approve before execute.

### Multi-model (non-trivial only)

Skip for typo / one-file zero-blast-radius fixes. Otherwise:

| Stage | Agent | Model role |
|---|---|---|
| Ground | `.cursor/agents/grounder.md` | Fable — codebase + docs grounding |
| Close | `.cursor/agents/gap-closer.md` | Opus 5 — gaps, blast radius, plan integrity |
| Execute | `.cursor/agents/executor.md` | Grok High Fast — implement approved cut |

Parent stays orchestrator. Each Task/subagent prompt must be
**self-contained** (subagents have no chat history). Still obey
project law: plan in `docs/plans/<pass>.md`, approve before code.

### Context budget (~75–85%)

When the context ring is high, a cut ends, or compact/clear is likely:

1. Set `dirty: true` in SESSION frontmatter while writing.
2. Update `docs/ops/resume/CURRENT.md` (goal / done / next / blockers / files).
3. Append a short `docs/journal/YYYY-MM-DD-<topic>.md` if the arc needs narrative.
4. Update SESSION opens/NEXT; `dirty: false`; emit `journal.wrote`.
5. Prefer stopping cleanly over pushing one more large tool storm.

`preCompact` may fire first — treat disk resume as truth after summary.

### After compact or clear

There is **no** chat-clear hook. New chat → Boot again. Trust
`resume/CURRENT.md` + STATE over memory. Optionally `@Past Chats` if needed.

## Signals

Append one JSON line per event to `docs/ops/signals.jsonl`.
Vocabulary: [references/signals.md](references/signals.md) → also
`docs/ops/references/signals-schema.md`.

## Hooks (safety net)

| Hook | Role |
|---|---|
| `sessionStart` | Sets `DPL_OPS_*` env; attempts resume pointer inject |
| `preCompact` | Snapshots usage % + transcript tail → resume raw; user_message |
| `stop` | If SESSION `dirty: true`, one followup nudge to journal |

Disk wins if inject fails.

## Alignment

- Product laws: `AGENTS.md`, `CLAUDE.md`, `docs/STATE.md`
- Plan/verify craft: clyffy-plan / clyffy-verify (invoke, do not replace)
- Lane backlogs: journals under `docs/journal/`; SESSION only indexes

## Dry-run checklist (p6 first live cycle)

When operator says go on coach lane:

1. Boot Session OS
2. NEXT = p6 Library section harness
3. Spawn grounder → gap-closer (≤3 plan / ≤2 web)
4. Halt for approve with `docs/plans/...`
5. Spawn executor for one cut
6. Journal + clear dirty
