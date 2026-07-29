---
dirty: false
active_lane: coach-ledger
updated: 2026-07-28
---

# Ops session ledger

Living index for agent default operations. Product truth stays in
`docs/STATE.md`. This file tracks **opens, tasks, milestones, NEXT**.
Any AI adopting Session OS reads this after STATE on boot.

## Active lane

**coach / ledger / lesson** — ordered backlog:
`docs/journal/2026-07-28-remaining-work.md`

**NEXT:** Library section harness (**p6**) — first live Session OS
multi-model cycle when operator says go.

## Opens

| Kind | Ref | Notes |
|---|---|---|
| PR / branch | `feat/session-os` | Land Session OS, then start p6 |
| Parallel — do not steal | authoring Pass B | authoring handoff journal |
| Parallel — do not steal | chat assist bake-off | STATE assist-mode entry |
| Parallel — do not steal | L4 / workflows | their feature branches |

## Milestone

Session OS authored → merge → p6 with grounder→gap-closer→approve→executor.

## Context budget

Journal and refresh `docs/ops/resume/CURRENT.md` near ~75–85% context,
at end of a cut, or when compact/clear is imminent. `dirty: true` while
unsaved; `stop` hook may nudge once.

## Signals

`docs/ops/signals.jsonl` · schema `docs/ops/references/signals-schema.md`
