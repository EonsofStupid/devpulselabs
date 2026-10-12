---
name: session-os
description: >-
  Default agent operations for DevPULSE Labs (Claude mirror of Cursor
  session-os — same docs/ops ledger). Boot from disk, identify opens,
  track milestones, journal near 75-85% context, resume after compact or clear.
  Use on every new chat, after compact/clear, when context is high, when starting
  a milestone, or when the operator mentions session OS, resume, journal, or ops ledger.
---

# Session OS — default operations

Any model. Progressive skill: keep this file loaded for the loop; open
references only when needed.

**Not** the product Parquet ledger (`dpl-ledger`). Ops live under `docs/ops/`.
Claude Code runs the same loop via skill `milestone-loop` +
`.claude/ledger/` hooks — compose, do not fork a second backlog.

## Boot (every session — do this first)

1. Read `docs/status.md`: now, known broken, next, plus its `active_lane`
   and `dirty` frontmatter.
2. Read `docs/requirements.md`, the operator's words. Nothing else is built.
3. Read `docs/README.md`, the map: the one doc that is the truth for each
   subject.
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
| Ground | `.claude/agents/grounder.md` | Fable — codebase + docs grounding |
| Close | `.claude/agents/gap-closer.md` | Opus — gaps, blast radius, plan integrity |
| Execute | `.claude/agents/executor.md` | inherit/Sonnet — implement approved cut |

Parent stays orchestrator. Each Task/subagent prompt must be
**self-contained** (subagents have no chat history). Still obey
project law: plan in `docs/plans/<pass>.md`, approve before code.

### Context budget (~75–85%)

When the context ring is high, a cut ends, or compact/clear is likely:

1. Set `dirty: true` in the `docs/status.md` frontmatter while writing.
2. Rewrite `docs/status.md`: now / known broken / next. Rewrite it, never
   append.
3. Write `docs/journal/YYYY-MM-DD-<topic>.md` (the handoff). Only the newest
   journal is kept; the previous one is deleted, and git keeps it.
4. Set `dirty: false` and emit `journal.wrote`.
5. Prefer stopping cleanly over pushing one more large tool storm.

`preCompact` may fire first — treat disk resume as truth after summary.

### After compact or clear

There is **no** chat-clear hook. New chat → Boot again. Trust
`docs/status.md` and the newest journal over memory. Optionally `@Past Chats`
if needed.

## Signals

Append one JSON line per event to `docs/ops/signals.jsonl`.
Vocabulary: [references/signals.md](references/signals.md) → also
`docs/ops/references/signals-schema.md`.

## Hooks (safety net)

| Hook | Role |
|---|---|
| `sessionStart` | Sets `DPL_OPS_*` env; injects `docs/status.md` + the newest journal |
| `preCompact` | Snapshots usage % + transcript tail → `docs/ops/resume/raw`; user_message |
| `stop` | If `docs/status.md` has `dirty: true`, one followup nudge to journal |

Disk wins if inject fails.

## Alignment

- Product laws: `AGENTS.md`, `CLAUDE.md`, `docs/requirements.md`
- The doc map (one doc per subject): `docs/README.md`
- Plan/verify craft: clyffy-plan / clyffy-verify (invoke, do not replace)
