---
dirty: false
active_lane: coach-ledger
updated: 2026-07-30
---

# Ops session ledger

Living index for agent default operations (Cursor Session OS + Claude
milestone-loop). Product truth stays in `docs/STATE.md`.

## Active lane

**coach / ledger / lesson** — ordered backlog:
`docs/journal/2026-07-28-remaining-work.md`

**NEXT:** merge C2b PR (`feat/c2b-coach-overlay`); optional Pass 2 stdio
coach loop; then strict pinning

**CLOSED:** p6 · `/strict` · **p2b** · **p4** · **C2s** (Tier A + live
Triggered) · **C2a** ADR ratified · **C2b** self-test pass 1
(`dpl-coach-overlay --self-test` → Triggered)

## Opens

| Kind | Ref | Notes |
|---|---|---|
| Authoring | **`docs/plans/authoring-lane-poam.md`** is the one page for this lane — goal, milestones, done, remaining, and what each remaining item waits on | M1–M4 + M6 closed (#135 #136 #141 #146 #147); M5 tabs 3 of 7 (#149 #151). Do not steal unless redirected. |
| Parallel | chat assist bake-off | STATE assist-mode entry |
| Parallel | authoring charter #119 merged | Pass B/C closed unmerged |
| Coach | C2b Pass 2 stdio `CoachCommand` loop | deferred; ADR launch gates apply |
| Land | `feat/c2b-coach-overlay` PR | merge on double-green |

## Milestone

Land C2b → (optional Pass 2) → strict pinning → DuckDB ADR → class column

## Signals

- Cursor/ops: `docs/ops/signals.jsonl`
- Claude telemetry: `.claude/ledger/ledger.jsonl` (gitignored)
