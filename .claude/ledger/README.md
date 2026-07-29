# The milestone-loop ledger

One JSONL stream — `.claude/ledger/ledger.jsonl` (gitignored, operator
telemetry) — capturing every phase transition of the agent's internal
workflow so the loop can be observed and TUNED over time. The loop
contract the signals trace is `.claude/skills/milestone-loop/SKILL.md`.

Line shape: `{"ts":"<utc iso>","event":"<domain.type>","milestone":"<slug>","detail":"k=v …"}`

## Signal vocabulary (kebab `domain.type`, the house signal style)

| event | emitter | when |
|---|---|---|
| `milestone.targeted` | agent | a major feature/sub-feature is chosen; detail names scope |
| `work.progress` | agent | a plan step lands (detail `step=N <name>`) |
| `journal.written` | agent | the handoff journal hits `docs/journal/` (detail `path=`) |
| `compact.pre` | PreCompact hook | context about to clear (detail `trigger=auto\|manual`) |
| `session.ended` | SessionEnd hook | the non-compaction clear path (detail `reason=`) |
| `session.reengaged` | SessionStart hook | a fresh context opened (detail `source=startup\|resume\|compact\|clear`) |
| `journal.absorbed` | agent | the fed journal was read back into working state |
| `transcript.reviewed` | SessionStart hook | the previous session's 500-entry tail was fed (free) |
| `plan.pass` | agent | one planning pass (detail `n=1..3` — HARD cap 3) |
| `research.spawned` | agent | a web-research subagent (detail `n=1..2` — HARD cap 2) |
| `plan.built` | agent | the pass plan is on disk (detail `path=`) |
| `drift.check` | agent | at every step transition (detail `step=N verdict=on-plan\|corrected`) |
| `verify.eguipilot` | agent | an EguiPilot scenario ran (detail `scenario= verdict=`) |
| `milestone.done` | agent | landed + verified + journaled |

## Capturing / tuning (operator)

```sh
tail -f .claude/ledger/ledger.jsonl                       # live
jq -r '[.ts,.event,.milestone,.detail]|@tsv' ledger.jsonl # table
jq 'select(.event=="drift.check" and (.detail|contains("corrected")))' ledger.jsonl  # drift rate
jq 'select(.event=="plan.pass")' ledger.jsonl             # planning cost per milestone
```

Tuning levers the signals expose: how often re-engagement happens per
milestone (journal cadence vs the 75–85% target), drift-correction rate
(plan quality), planning passes used vs the cap, verify failures per
milestone. Feed observations back into the SKILL.md — the contract is
meant to evolve.

## Files

- `emit.sh` — the one emit seam (agent + hooks both use it).
- `session-start.sh` — re-engagement feed: STATE · latest journal ·
  previous session's transcript tail (recorded path), all at zero
  tool-call cost.
- `pre-compact.sh` / `session-end.sh` — the clear edges: record the
  transcript path for the next feed, demand the journal.
- `ledger.jsonl`, `last-session.txt` — local, gitignored.
