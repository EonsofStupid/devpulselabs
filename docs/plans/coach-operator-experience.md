# Coach operator experience — product spine (frictionless vision)

**Status:** ALIGNED 2026-07-30 (doc Passes 1–5) + **C2c mock authored** —
awaiting operator look-approve before C2d inventory code  
**Lane:** coach-ledger  
**Mock:** [`../ops/references/coach-operator-mock-c2c.html`](../ops/references/coach-operator-mock-c2c.html)  
**Mock plan:** [`coach-c2c-operator-mock.md`](coach-c2c-operator-mock.md)  
**Provisional home:** shell **Coach hub** (not chat peer window)  
**Canvas:** `coach-vision-alignment.canvas.tsx` (Cursor; not repo truth)  
**Physics evidence:** C2s live Triggered · C2a ADR · C2b #158  
**Program map:** [`operator-coach-a2ui.md`](operator-coach-a2ui.md)  
**Alignment journal:** [`../journal/2026-07-30-coach-product-spine.md`](../journal/2026-07-30-coach-product-spine.md)

## 1. Goal

Name the **product** coach so engineering stops drifting into daemon-only
work: while DevPULSE runs, the operator picks an installed application,
we mint a **semantic fabric** from accessibility streams (not screenshots),
AI mounts typed touchpoints that paint and wait for real presses, and
refusals teach with tips. Advanced / EguiPilot remains a second surface.

## 2. Current state (ground truth)

| Layer | Status |
|---|---|
| In-app touchpoints (C1) | Landed — pilot `CoachCommand` / ColdLight spotlight |
| Dynamic in-app triggers (C1.5a) | Specified; partial |
| Foreign locate + compose + paint + Triggered | **PASS** (C2s) |
| Overlay process exception + self-test binary | **PASS** (C2a + C2b #158) |
| Stdio coach loop on overlay | Deferred (Advanced plumbing) |
| **Operator picker / mint / tip UX** | **MISSING — this is NEXT** |
| Lightweight installed-app inventory | Not started |
| Fabric mint progress + ETA | Not started |
| Doc NEXT pointers | **Aligned this pass** → C2c Operator UX |

## 3. Design

### 3.1 Two surfaces (must not collapse)

| Surface | Audience | Contract |
|---|---|---|
| **Operator Coach** | Layman | Guided: pick app → mint fabric → lesson paints → tips on refuse |
| **Advanced / EguiPilot** | Dev / agent | `dpl-coach-overlay`, probes, `CoachCommand`, stream dumps, adversary matrix |

Operator never needs Sync-grab, XTEST, or WINDOW extents unless Advanced
is open. Advanced never replaces the product entry point.

### 3.2 Glossary (layman → technical)

| Operator language | Engineering term |
|---|---|
| Engine while DevPULSE is open | Coach runtime (+ overlay peer for foreign paint) |
| Pick an installed program | Target inventory |
| Fabric / signal map for AI | Semantic fabric (AT-SPI / UIA graph) + `CoachEvent` / `FabricEvent` |
| Highlight + press advances lesson | Touchpoint Mount + observe → `Triggered` |
| Cheat without screenshots | Tier A stream access (roles, names, extents, actions) |
| Don’t freeze scanning apps | Phase 0 disk cache → Phase 1 shallow census → Phase 2 deep mint on select |
| Bigger app takes longer | Fabric mint cost / crawl budget + honest ETA |
| Seeds + tips when rejected | Affordance defaults + `Refused { reason, tip }` |

### 3.3 Tier ladder (unchanged law)

```
Tier A — semantic tree (AT-SPI / UIA / AccessKit)     ← primary; proven
Tier B — first-party app API (Blender, …)             ← later
Tier C — bounded vision                               ← last resort only
```

Screenshots are Tier C. The “cheat” is Tier A — industry spine (gptme
AT-SPI tree actions; a11y-first agents). Calculator proof is pure Tier A.

### 3.4 Inventory phases (anti-freeze)

1. **Phase 0 — cold catalog:** `.desktop` / Start Menu metadata, cached,
   idle refresh / file watch. No a11y walk.
2. **Phase 1 — live census:** Registry children / root apps — name + PID
   only. No deep tree.
3. **Phase 2 — fabric mint (on select):** budgeted BFS for **one** target;
   cancelable; progress events; refuse dark / oversize trees.

Never: continuous desktop capture, OCR loops, or deep-walking every binary
at startup.

### 3.5 Operator flow (mock-first)

1. Open Coach → list inventory (seeds if empty: Calculator).  
2. Select target → confirm mint with complexity copy + ETA.  
3. Progress: “Building signal map… N% · walking menus (Cancel)”.  
4. Ready → AI / lesson can Mount touchpoints; overlay paints.  
5. Refused → tip with next action (never silent).

### 3.6 Ordered product passes (after physics)

| ID | Pass | Done when |
|---|---|---|
| **C2c** | This spine + living-doc tip + UI mock approve | Operator signs mock |
| **C2d** | Lightweight inventory (Phase 0+1) spike/plan+code | List without freeze; unit + live |
| **C2e** | Fabric mint for selected Calculator | Progress + refuse tips; journal |
| **C2f** | Wire Operator select → existing overlay paint | One guided lesson path |
| *(parked)* | C2b Pass-2 stdio · strict pinning · DuckDB | Advanced / later spine |

## 4. Changes (this alignment pass — docs only)

1. Author this spine.  
2. Tip `remaining-work`, `STATE`, `SESSION`, `resume/CURRENT`.  
3. Tip `operator-coach-a2ui` delivery order (C2c+ after C2b).  
4. Tip C2b plan: pass-1 closed; product NEXT = C2c not Pass-2.  
5. Journal the audit + decisions.  
6. **No application code** until operator approves mock.

## 5. Boundaries

- Production `app` still must not auto-launch overlay (ADR launch gates).  
- Operator UX may live in-app; paint to foreign targets still via overlay
  under ADR.  
- No Tier C as primary eyes.  
- Copy, never move; never delete prior spike evidence.

## 6. Verification (alignment)

- Every living NEXT pointer agrees: **C2c mock / experience** next.  
- Pass-2 stdio and strict pinning marked **parked**, not NEXT.  
- Glossary in this doc matches operator brief 2026-07-30.  
- Second pass if any contradiction remains after cross-read.

## 7. Commit plan

Branch `docs/coach-operator-experience` → PR: spine + tips + journal.
UI mock is a follow-up pass after operator `approve` on look.

## 8. Evidence — Pass 1 audit → Pass 2–3 fixes (2026-07-30)

| Doc | Was | Now |
|---|---|---|
| `docs/ops/SESSION.md` | merge C2b / Pass-2 | NEXT = C2c mock → C2d |
| `docs/ops/resume/CURRENT.md` | land C2b PR | Operator experience spine |
| `docs/journal/2026-07-28-remaining-work.md` | C2b → pinning | C2c–C2f then pinning |
| `docs/STATE.md` tip | Pass-2 / pinning | C2c mock approve |
| `coach-c2b-overlay-daemon.md` | Pass-2 implied next | Product C2c; Pass-2 parked |
| `operator-coach-a2ui.md` §4 | Pass-2 after C2b | C2c–C2f slices |

Physics journals (C2s live, C2b) remain engineering truth — not deleted.
Journal: `docs/journal/2026-07-30-coach-product-spine.md`.

