# C2c — Operator Coach UI mock

**Status:** LOOK-APPROVED 2026-07-30 — operator “looks good”  
**Spine:** [`coach-operator-experience.md`](coach-operator-experience.md)  
**Mock:** [`../ops/references/coach-operator-mock-c2c.html`](../ops/references/coach-operator-mock-c2c.html)

## 1. Goal

Freeze the **frictionless Operator Coach** first composition so inventory
(C2d) and fabric mint (C2e) implement against an approved look — not
against CLI probes.

## 2. Current state

- Physics closed (C2s/C2b #158).
- Product spine ALIGNED (doc Passes 1–5).
- No in-app Coach hub yet.

## 3. Design

### Provisional home (operator may redirect)

**Coach as a shell hub route** (rail entry like Home / Workbench / Shows).

Why not a chat-style peer window: Coach teaches while foreign apps stay
visible; a second peer window competes for focus. Why not only Workbench
Library: Coach is a product verb (“teach”), not a catalog wall.

Advanced remains a secondary control (topbar) → CLI / EguiPilot / overlay
self-test — never the primary entry.

### First composition (mock)

| Region | Content |
|---|---|
| Rail | Coach hub selected |
| Hero | “Teach with Coach” + Tier A / no-screenshot dek |
| Left | Installed & available list + seed tip + Build / Refresh |
| Right | Mint progress + steps + cancel + example refuse tip |
| Footer | Advanced pointer (explicitly secondary) |

Complexity honesty: pills Ready·small / Ready·medium / Later·deep.

## 4. Changes

1. HTML mock under `docs/ops/references/` (copy, never move prior mocks).  
2. Tip spine + remaining-work C2c status.  
3. No egui code this pass.

## 5. Boundaries

- Mock is not production; ColdLight tokens later when egui lands.  
- Overlay launch gates unchanged (ADR).  
- No inventory code until look-approve.

## 6. Verification

- Operator opens mock cold and can narrate the flow without CLI.  
- Seed tip + refuse tip visible.  
- Advanced clearly secondary.

## 7. Commit plan

Same PR as product-spine doc alignment (`docs/c2c-coach-operator-experience`).

## 8. Evidence

Mock path: `docs/ops/references/coach-operator-mock-c2c.html`.  
Operator look-approve still required before C2d.
