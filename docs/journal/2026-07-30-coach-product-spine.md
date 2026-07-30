# Journal — 2026-07-30: Coach product spine alignment (C2c docs)

## Intent

Operator ruled we were drifting: physics POC (calculator paint + Triggered
+ overlay) is real, but the **frictionless Operator Coach** (pick installed
app → mint fabric from streams → guide with seeds/tips) was not the living
NEXT. Align docs diligently; mock UI before more daemon work. Multiple
alignment passes allowed for accuracy.

## Pass 1 — audit (contradictions found)

| Living pointer | Was saying | Truth |
|---|---|---|
| SESSION / CURRENT | merge C2b; then Pass-2 stdio | #158 already merged |
| remaining-work §6–7 | after C2b → strict pinning | skipped Operator UX |
| STATE C2b tip | NEXT = Pass-2 / pinning | product gap unnamed |
| C2b plan | Pass-2 “deferred” read as next | Advanced, not product |
| operator-coach §4 | Pass-2 after C2b | no C2c–C2f |

Physics journals left intact (engineering truth).

## Pass 2–3 — amendments landed this session

- **Spine:** `docs/plans/coach-operator-experience.md` (glossary, dual
  surface, inventory phases, C2c–C2f order, Tier A no-screenshot law).
- **remaining-work** rewritten: vision + ordered C2c–C2f; Pass-2 parked.
- **STATE / SESSION / CURRENT** tipped to C2c mock approve.
- **operator-coach-a2ui** §4 + §7: C2c–C2f NEXT; Pass-2 parked.
- **coach-c2b-overlay-daemon** tip: product NEXT ≠ Pass-2.

## Pass 4 — cross-read + ADR tip (this session)

- Living NEXT pointers agree on C2c (SESSION, CURRENT, STATE, remaining-work,
  operator-coach §4, C2b plan).
- ADR §7 clarified: overlay = painter/observer; Operator UX = separate
  in-app product (`coach-operator-experience.md`). Devtools-only ≠ no
  Operator product.
- Spine §2/§8 updated from “STALE” to “aligned.”

## Pass 5 — Session OS + leftover tip check (2026-07-30)

Re-verified dirty flags, NEXT agreement, and historical journals:

| Check | Result |
|---|---|
| `SESSION.md` / `CURRENT.md` dirty | both `false` |
| Living NEXT | C2c mock approve (SESSION, CURRENT, STATE, remaining-work) |
| Stale “merge C2b” as active NEXT | none in living ops |
| C2b journal `## Next` still named Pass-2 | **fixed** → product C2c+; Pass-2 parked |
| Spine status still DRAFT | **tipped** → ALIGNED (await mock approve) |
| ADR §7 Operator vs painter | present |

Historical `signals.jsonl` `pr.merged` next=`optional-pass2-stdio` left as
history; superseded by later `journal.wrote` product-next signals.

## Still needs operator

1. **Look-approve** mock:
   `docs/ops/references/coach-operator-mock-c2c.html`
   (or mark red-pen changes). Provisional home = Coach hub.
2. Confirm or redirect Coach home (hub vs Workbench vs peer).  
3. After approve: C2d inventory plan → code.

## C2c mock authored (same session as “go”)

- Plan: `docs/plans/coach-c2c-operator-mock.md`
- HTML mock under `docs/ops/references/` (ColdLight-adjacent vocabulary
  shared with authoring mock).

## Decisions recorded

1. Product NEXT = C2c → C2d → C2e → C2f.  
2. Pass-2 stdio + strict pinning + DuckDB = parked relative to product.  
3. Screenshot loops forbidden as primary fabric path (Tier C last).  
4. Inventory must be phased (cold → shallow → deep-on-select).

## Confidence

**High** on NEXT retarget and dual-surface split.  
**Med** on exact in-app hub home (Workbench vs new Coach route) — mock
will force the choice.  
**Open:** Windows inventory shape; Electron force-renderer-accessibility.

## Verify commands (docs only)

```bash
rg -n 'NEXT|Pass-2|Pass 2|C2c' docs/ops/SESSION.md docs/ops/resume/CURRENT.md \
  docs/plans/coach-operator-experience.md docs/journal/2026-07-28-remaining-work.md
```
