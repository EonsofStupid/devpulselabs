# Research — Coach Import Process (Standard vs Advanced)

**Captured:** 2026-07-31  
**Program:** [`../../plans/coach-lab-experiment-program.md`](../../plans/coach-lab-experiment-program.md)  
**Mock stages:** Pick → Confirm → Checks → Standard | Advanced → Lesson-ready

## Premise

Operators must never get a silent deep scan. Selecting an app to **import**
always shows an honesty prompt (“this will take a few minutes…”), runs
**essential checks** (accessibility first), then chooses a workflow.
Lesson execution must stay high-performance and screenshot-free even when
import used heavier methods.

## Standard Import

| | |
|---|---|
| Gate | a11y PASS + usable interactable density |
| Work | Budgeted Tier A walk (`dpl-coach-fabric` / C2e shape) |
| ETA | About a minute → a few minutes (complexity pills) |
| Output | `FabricGraph` via `PerceptionSource::A11yStream` |
| Cancel | Always |

## Advanced Import (proprietary fabric path)

| | |
|---|---|
| Gate | a11y THIN / MISSING / dark tree (Electron, canvas, legacy) |
| Work | Bounded offline harvest + optional structured vision **as import fuel** |
| ETA | Longer; honest copy; kill criteria (time, freeze, unsafe action) |
| Output | Same `FabricGraph` shape via `HarvestedGraph` / `VisionParse` |
| Surface | Advanced / Lab — not the default Operator lie |

This is the “custom advanced proprietary system”: compile once into fabric
IDs so runtime can **open, manipulate, edit, highlight** accurately without
cumbersome screenshots.

## Essential checks (ordered)

1. **Accessibility stream** — bus + app root + ChildCount sanity.  
2. **Window focus** — resolvable target for paint.  
3. **Interactive density** — enough named actions for a lesson.  
4. **Refuse screenshot-as-eyes** — Tier C never primary.

Failing (1) or low (3) → divert to Advanced Import (or refuse with tip).

## Lesson execute (both workflows)

- Mount touchpoints against fabric IDs  
- Highlight from extents / compose (overlay under ADR)  
- Observe Press / ValueChanged / FabricEvent  
- Beat clear: chrome animation / minimize notify — **not** a new capture  
- Teacher: same rails, `AgencyMode::Teacher` may open+execute  

## What we are not claiming yet

- Harvest safety matrix on destructive menus (CX-harvest-01)  
- OmniParser-class VisionParse wired in-repo  
- DuckDB queries (stamp only until Phase 3)

## Falsifiers

- If Standard Import runs when a11y is dark → process bug.  
- If lesson path takes screenshots every step → law violation.  
- If Advanced Import produces a different Mount API → seam bug.
