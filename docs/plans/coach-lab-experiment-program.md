# Coach Lab — experiment program + Import Process

**Status:** CAPTURED 2026-07-31 — awaiting operator look-approve on mock rev  
**Mock:** [`../ops/references/coach-operator-mock-c2c.html`](../ops/references/coach-operator-mock-c2c.html) (rev 2026-07-31)  
**Research:** [`../research/coach/README.md`](../research/coach/README.md)  
**Journal:** [`../journal/2026-07-31-coach-import-lab-capture.md`](../journal/2026-07-31-coach-import-lab-capture.md)  
**Spine:** [`coach-operator-experience.md`](coach-operator-experience.md)

## 1. Goal

Lock a **methodical Lab** for cutting-edge Coach work so product passes
(C2f…) and Advanced spikes (Harvest, Teacher) share one process: honesty
prompt → essential checks → Standard or Advanced Import → lightweight
lesson execute — with typed signals from day one for later DuckDB baselines.

## 2. Current state

| Piece | Status |
|---|---|
| Physics (locate/paint/Triggered) | PASS C2s/C2b |
| Inventory + fabric mint CLIs | PASS C2d/C2e |
| Operator hub UX | Mock only; C2f not started |
| Import honesty + dual workflow | **This capture** (mock + docs) |
| Harvest / Teacher experiments | Named below; not started |
| DuckDB | Phase 3 — stamp facts now |

## 3. Design

### 3.1 Import Process (product law)

```
Pick app
  → Honesty prompt (ETA + what we check) — never silent
  → Essential checks (a11y first)
       ├─ PASS  → Standard Import (minutes) → FabricGraph
       └─ THIN/MISSING → Advanced Import (longer, separate)
              → proprietary HarvestedGraph / structured parse
              → same FabricGraph shape for lessons
  → Lesson execute = Mount / highlight / observe only
       (no screenshot hot path)
```

**Essential checks (minimum):**

| Check | Role |
|---|---|
| Accessibility stream | Big gate — Standard vs Advanced |
| Window focus | Highlight / paint path |
| Interactive density | Named controls for lessons |
| Screenshot-as-eyes | Always refused as primary |

**Execution law:** Vision/screenshots may fuel **Advanced Import** only.
Lesson runtime never runs a capture/OCR/VLM loop.

### 3.2 Agnostic seams (plug-and-play)

| Name | Meaning |
|---|---|
| `PerceptionSource` | `A11yStream` \| `VisionParse` \| `AppApi` \| `AuthoredSeed` \| `HarvestedGraph` |
| `FabricGraph` | Nodes + actions + states — lesson mounts against IDs |
| `MountAdapter` | Circle \| HoloPanel \| … (chrome evolves without new fabric) |
| `ObserveAdapter` | Press \| ValueChanged \| FabricEvent \| SteadyState |
| `AgencyMode` | `Coach` (operator leads) \| `Teacher` (AI may open+execute) |
| `ImportWorkflow` | `Standard` \| `Advanced` |

### 3.3 Experiment lifecycle

```
Research note → Spike (feature-gated) → Journal PASS|KILL|AMEND
  → ADR only if law changes
  → Product pass ID only if Operator-facing
```

**Manifest fields (every CX-* before code):**  
`id`, hypothesis, kill criteria, `PerceptionSource`, `AgencyMode`,
signals stamped, proof, visual, product promotion (`none`|`C2f`|`C2g`|…).

### 3.4 Ordered queue (ratified this capture)

| Order | ID | Surface | Blocks C2f? |
|---|---|---|---|
| 0 | This program + mock rev | Docs / Lab | Unblocks |
| 1 | **CX-cursor-01** overlay seed on Cursor (test-first) | Spike | Unblocks C2f seed |
| 2 | **C2f** Operator import UI → Cursor lesson | Product | Spine NEXT |
| 3 | Notepad (Windows Standard Import) | Product | After Cursor |
| 4 | **CX-teacher-agency-01** in-app actor | Product-adjacent | No Pass-2 |
| 5 | **CX-harvest-01** Advanced Import | Advanced | When a11y thin |
| 6 | Pass-2 stdio / DuckDB | Parked / Phase 3 | — |

**Conquest:** Cursor first (Calculator = regression only).  
**Harvest:** only when essential checks fail — Cursor probe 2026-07-31 was
Standard Import PASS.  
**Teacher:** in-app `CoachCommand` + actor first; Pass-2 is transport.

### 3.5 Signal / ledger (baseline fuel)

Stamp at minimum: check results, workflow chosen, mint/harvest duration,
mount, armed, triggered, refuse, beat-clear. Coach never subscribes to
coach. No new free-form domains.

## 4. Changes (this capture — docs + mock only)

1. Rev mock with Import Process stages.  
2. Author this program + `docs/research/coach/*`.  
3. Journal research + decisions.  
4. Tip spine / remaining-work / ops.  
5. **No Rust** until operator look-approves mock rev and approves C2f plan.

## 5. Boundaries

- Advanced Import ≠ second Teacher stack.  
- Overlay ADR launch gates unchanged.  
- DuckDB stays sidecar Phase 3.  
- Copy, never move; do not delete C2c LOOK-APPROVED history (rev noted).

## 6. Verification

| Gate | Pass |
|---|---|
| Mock | Operator can narrate pick → confirm → checks → Standard/Advanced → lesson-ready |
| Docs | Queue + seams + execution law agree across spine/research/journal |
| Signals | Manifest names match §3.2; ledger stamp list named |

## 7. Commit plan

Branch `docs/coach-import-lab-capture` → PR docs + mock rev.

## 8. Evidence

Research notes: `docs/research/coach/`.  
Industry spine: a11y-first (gptme, Terminator); offline harvest
(ActionEngine/Murphy); vision as import fallthrough (OmniParser-class) —
see research README.
