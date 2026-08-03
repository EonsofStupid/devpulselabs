# Fabric Bridge Runtime — agnostic AI↔app control

**Status:** PASS 2026-07-31 — CX-bridge-01 Cursor resolve→invoke; Calculator regression; hub session wire  
**Journal:** [`../journal/2026-07-31-cx-bridge-01.md`](../journal/2026-07-31-cx-bridge-01.md)  
 
**Spine:** [`coach-operator-experience.md`](coach-operator-experience.md)  
**Lab:** [`coach-lab-experiment-program.md`](coach-lab-experiment-program.md)  
**Import:** [`../research/coach/import-process.md`](../research/coach/import-process.md)  
**Prior:** C2f Operator chrome (#187) · CX-cursor-01 locate-only · C2e mint  
**Out of scope:** Harvest / VisionParse lesson eyes · DuckDB queries · Pass-2 stdio · claiming C2f delivered the bridge

## 1. Goal

Ship the **FabricGraph runtime bridge** so Operator Coach and Teacher address
foreign apps by **fabric IDs** (Tier A Action invoke), independent of viewport
pixels. Overlay paint is optional Mount feedback. Cursor is the conquest that
proves the agnostic framework; Calculator is regression only.

## 2. Current state

| Piece | Status |
|---|---|
| Inventory + fabric mint | PASS C2d/C2e |
| Operator hub Import UI | C2f chrome — mints then **discards** fabric |
| Overlay locate/paint/invoke | Name search only — **does not read Fabric** |
| FabricSession / resolve / invoke | **Missing — this plan** |
| CX-cursor-01 | Locate + extents print; spotlight deferred |
| Teacher AgencyMode | Named in lab; no shared rails yet |

## 3. Design

### 3.1 Definitions

| Term | Meaning |
|---|---|
| Fabric address | `path` + role + name fingerprint inside a minted graph |
| Viewport-independent control | AT-SPI `doAction` / UIA Invoke on re-resolved accessible — not click-xy |
| Bridge | Hold Fabric → resolve → Invoke / Mount / Observe → Triggered\|Refused |
| Overlay | MountAdapter painter under ADR — not the address space |

### 3.2 Types (`dpl-coach-runtime`)

```
FabricSession { fabric: Fabric, minted_at, agency: AgencyMode }
NodeRef { path, role, name, actions }   // from FabricNode
LiveHandle { /* platform proxies */ }
AgencyMode { Coach | Teacher }          // same resolve/invoke/mount

resolve(session, NodeRef|seed_path) -> Result<LiveHandle, Refused>
invoke(handle, action_name) -> Result<InvokeEffect, Refused>
extents(handle) -> Result<IRect, Refused>   // paint only
```

**Resolve v1:** walk index chain in `path` from app root; leaf must match
name (+ role); on mismatch, bounded name+role search under parent; refuse
if ambiguous.

### 3.3 CX-bridge-01 done-when

1. Mint Cursor fabric; seed `Search Ctrl+K` (or fallback) present with actions.  
2. `resolve(seed)` by fabric identity — not product CLI name hunt.  
3. `invoke(seed, click)` via Action (or typed refuse — no silent pixel fallback).  
4. Paint optional from live extents after resolve.  
5. Hub lesson holds Fabric and calls bridge API.

Calculator `7` must stay green on the same API.

### 3.4 Overlay / hub

- Overlay accepts resolved rect + expected name from bridge (fail-closed).  
- Hub keeps `Fabric` after mint; lesson → resolve → invoke and/or Mount.  
- Persistence of FabricGraph = later; v1 session-held.

## 4. Changes (order)

0. This plan + tip lab/census/STATE/spine (docs).  
1. CX-bridge-01 probe CLI + journal.  
2. `dpl-coach-runtime` + structure.  
3. Wire hub + overlay Mount to bridge.  
4. Teacher `AgencyMode` on same rails (no second stack).

## 5. Boundaries

- No screenshot lesson hot path.  
- No `app` crate dep on overlay.  
- No waiving identity by painting wrong control.  
- Advanced harvest remains CX-harvest-01.

## 6. Verification

| Gate | Criteria |
|---|---|
| Unit | Path resolve + fingerprint refuse on MemTree |
| Reality | Cursor mint→resolve→invoke journal; Calculator regression |
| Adversary | Wrong seed / dead app refuse; no name-only product success |

## 7. Commit plan

Branch `feat/coach-fabric-bridge` (from C2f chrome). PR: docs + runtime +
probe + hub wire + AgencyMode seam.

## 8. Evidence

Lab §3.2 seams · import-process lesson execute · C2e FabricNode shape ·
overlay ADR painter-only · CX-cursor-01 locate ≠ bridge.
