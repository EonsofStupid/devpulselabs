# Ops signals schema

One JSON object per line in `docs/ops/signals.jsonl`.

```json
{
  "ts": "2026-07-28T20:00:00Z",
  "signal": "session.boot",
  "session_id": "optional-cursor-session-id",
  "lane": "coach-ledger",
  "detail": {}
}
```

## Vocabulary

| `signal` | When |
|---|---|
| `session.boot` | Agent read STATE + SESSION + resume |
| `open.identified` | Found PR/branch/blocker |
| `milestone.set` | Working toward named milestone |
| `plan.pass` | Plan pass n/3 (`detail.n`) |
| `research.web` | Web research n/2 (`detail.n`) |
| `model.ground` | Fable grounder ran |
| `model.close` | Opus gap-closer ran |
| `model.exec` | Grok executor ran |
| `context.budget` | Approaching 75–85% or preCompact observed |
| `journal.wrote` | Journal / resume refreshed |
| `compact.observed` | `preCompact` hook fired |
| `resume.loaded` | Resume pack consumed |
| `dirty.set` / `dirty.clear` | SESSION frontmatter toggled |

Improve Session OS by counting missed journals, plan overruns, and
which model stage fails.
