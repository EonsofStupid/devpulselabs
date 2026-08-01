# CX-cursor-01 — Cursor Standard Import seed (test before hub UI)

**Status:** PLAN DRAFT 2026-07-31 — probe PASS; awaiting operator `approve`  
**Probe journal:** [`../journal/2026-07-31-cursor-a11y-probe.md`](../journal/2026-07-31-cursor-a11y-probe.md)  
**Lab:** [`coach-lab-experiment-program.md`](coach-lab-experiment-program.md)  
**Import:** Standard Import works on this box (157 nodes · ~400 ms)

## 1. Goal

Prove a **seed lesson target on Cursor** with existing CLIs + overlay
(locate/spotlight) before building Coach hub UI — persistent step toward
select-Cursor → fabric → teach, without boiling the ocean.

## 2. Current state

| Piece | Status |
|---|---|
| Calculator physics/mint | DONE (regression seed) |
| Cursor inventory + mint | PROBED READY |
| Overlay on Cursor control | **Not proven** |
| Coach hub select UI | Missing (C2f) |
| Advanced Import | Not needed *this seat* yet |

## 3. Design (spike)

1. Pick seed name from fabric: prefer **`Search Ctrl+K`**, fallback **`File`**.  
2. `dpl-coach-overlay locate --app cursor --name "<seed>"` (or equivalent
   flags — ground-truth CLI in-session).  
3. Optional spotlight paint once.  
4. Journal PASS/FAIL + extents.  
5. **No** hub UI, **no** harvest, **no** Teacher this spike.

If locate fails → tip Advanced Import / force-renderer-a11y; do not fake.

## 4. Changes after approve

1. Live overlay probe on Cursor seed.  
2. Journal tip + Lab registry PASS/KILL.  
3. Amend C2f plan: Cursor primary conquest; Calculator regression;
   Notepad next (Windows).

## 5. Boundaries

- Feature-gated / CLI only.  
- Overlay ADR launch gates.  
- No full Coach hub.

## 6. Verification

| Gate | Pass |
|---|---|
| Reality | locate (and optional paint) on Cursor seed name |
| Adversary | no screenshot loop; mint stays budgeted |
| Correctness | journal names exact seed + commands |

## 7. Commit plan

Branch `spike/cx-cursor-01` after `approve` — journal-first PR OK if
CLI-only evidence.

## 8. Evidence

See probe journal. Industry: Electron can go dark; **this seat did not**.
