# C2c — Operator Coach UI mock

**Status:** LOOK-APPROVED 2026-07-30 (“looks good”); **rev 2026-07-31**
Import Process — awaiting operator look-approve  
**Spine:** [`coach-operator-experience.md`](coach-operator-experience.md)  
**Lab:** [`coach-lab-experiment-program.md`](coach-lab-experiment-program.md)  
**Mock:** [`../ops/references/coach-operator-mock-c2c.html`](../ops/references/coach-operator-mock-c2c.html)

## 1. Goal

Freeze the **frictionless Operator Coach** composition — including the
**Import Process** (honesty prompt → essential checks → Standard vs
Advanced Import → lesson-ready) — so C2f implements against an approved
look, not CLI probes.

## 2. Current state

- Physics closed; inventory + fabric mint CLIs PASS.  
- Base mock LOOK-APPROVED 2026-07-30.  
- Rev adds dual import workflow + stage walkthrough.

## 3. Design

### Provisional home (unchanged)

**Coach as a shell hub route.** Advanced secondary (topbar / Lab).

### Composition (rev 2026-07-31)

| Region | Content |
|---|---|
| Rail | Coach hub selected |
| Hero | Teach with Coach + import / no-screenshot-hot-path dek |
| Stage bar | Pick → Confirm → Checks → Standard \| Advanced → Lesson-ready |
| Left | App list (incl. thin-a11y example) + Import / analyze |
| Right | Stage panel (prompt, checks, progress, lesson-ready) |
| Modal | Honesty confirm — ETA + what we check |
| Law strip | Execution law: fabric at lesson time; vision = import fuel only |
| Footer | Advanced pointer |

## 4. Changes (this rev)

1. Update HTML mock in place (copy/rev; do not delete history).  
2. Tip Lab program + research + journal.  
3. No egui code.

## 5. Boundaries

- Mock is not production.  
- Overlay ADR unchanged.  
- Advanced Import is a separate workflow, not a fake Standard mint.

## 6. Verification

- Operator can narrate all six stages without CLI.  
- Thin-a11y app diverts to Advanced Import.  
- Lesson-ready states no screenshot loop.

## 7. Commit plan

`docs/coach-import-lab-capture` PR with Lab program + journal.

## 8. Evidence

Mock path above. Journal:
`docs/journal/2026-07-31-coach-import-lab-capture.md`.
