# C2f — Operator Import UI → Cursor lesson + paint

**Status:** PASS 2026-07-31 — hub + Import UI + overlay lesson spawn; proof `docs/proof/c2f-coach-hub.png`  
**Spine:** [`coach-operator-experience.md`](coach-operator-experience.md)  
**Lab / Import Process:** [`coach-lab-experiment-program.md`](coach-lab-experiment-program.md)  
**Mock:** [`../ops/references/coach-operator-mock-c2c.html`](../ops/references/coach-operator-mock-c2c.html)  
**Prior:** CX-cursor-01 PASS — locate `Search Ctrl+K` / `File` on Cursor  
**Out of scope:** Advanced Import harvest · Teacher · Pass-2 stdio · Notepad (next seat)

## 1. Goal

Ship the **Operator Coach hub** Import Process for **Cursor**: pick → honesty
prompt → essential checks → Standard Import (fabric mint) → Mount/paint seed
`Search Ctrl+K` → wait for real press → Triggered — without CLI, without a
screenshot hot path. Calculator remains a regression seed in the list.

## 2. Current state

| Piece | Status |
|---|---|
| Inventory CLI | PASS C2d |
| Fabric mint CLI | PASS C2e (Cursor 157 nodes) |
| Overlay locate Cursor | PASS CX-cursor-01 |
| Coach hub crate / rail | **Missing** |
| Import honesty UI | Mock only |
| In-app Mount on foreign target | Overlay under ADR — not auto from `app` |

## 3. Design

### 3.1 Product flow (must match mock)

```
Coach hub
  → list inventory (Cursor first, Calculator regression)
  → Import / analyze… → modal (ETA + checks named)
  → Essential checks panel (a11y / focus / density / refuse screenshots)
  → Standard Import progress (cancelable) → Fabric ready
  → Start guided lesson → overlay paint seed → Triggered
```

Thin a11y → show Advanced Import divert tip (no fake Standard); **do not**
implement harvest in C2f.

### 3.2 Crate / placement

| Piece | Placement |
|---|---|
| Hub UI | New `crates/hubs/coach` (`dpl-hub-coach`) — `HubId("coach")`, one line in `app/main.rs` |
| Inventory / mint | Depend on `dpl-coach-inventory` + `dpl-coach-fabric` (egui-free libs) |
| Overlay paint | Spawn/attach per **ADR** (`eguipilotai-devtools` / successor feature; fail-closed). Lesson start = explicit operator gesture — never silent auto-launch |
| Structure | Hub is not orphan; libs stay orphan-allowed until hub consumes (already may) |

### 3.3 Seed lesson (Cursor)

| Field | Value |
|---|---|
| Target | `cursor.desktop` / atspi `cursor` |
| Seed control | `Search Ctrl+K` (fallback `File`) |
| Paint | existing overlay spotlight / WINDOW compose |
| Success | `Triggered` on real press (or refuse + tip) |

### 3.4 Signals to stamp (ledger fuel)

`ImportRequested` · `CheckResult{a11y,…}` · `ImportWorkflow::Standard` ·
`MintProgress` / `MintReady` · `Mount` · `Triggered` / `Refused` — coach
domain only; never subscribe to self.

### 3.5 Proof

| Gate | Criteria |
|---|---|
| Unit | Hub registers; check gate classifies Cursor as Standard when mint would Ready |
| Reality | Screenshot Coach hub + live: select Cursor → import → paint seed (harness `--route coach/...` + overlay) |
| Adversary | No screenshot loop; thin-a11y path refuses Standard with tip; Calculator still in list |

## 4. Changes (order after approve)

1. `dpl-hub-coach` scaffold + rail entry + inventory list UI.  
2. Import modal + checks + Standard mint wiring (progress).  
3. “Start guided lesson” → overlay locate/spotlight Cursor seed (ADR-gated).  
4. Tips + journal + proof shots.  
5. **No** Advanced harvest implementation.

## 5. Boundaries

- Overlay ADR launch gates.  
- No DuckDB. No Pass-2. No Teacher agency.  
- Notepad deferred (Windows).  
- Copy never move; mock is look-target not production tokens.

## 6. Verification

`cargo green` · Coach route screenshot · live Cursor import→paint journal.

## 7. Commit plan

Branch `feat/c2f-coach-import-ui` after #184 merged + operator `approve`
this plan.

## 8. Evidence

CX-cursor-01 journal · Cursor a11y probe · Lab Import Process · C2c mock rev.
