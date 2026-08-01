# CX-verbs-p1 — press_key / select / scroll

**Status:** PASS 2026-08-01 — journal `../journal/2026-08-01-cx-verbs-p1.md`  
**ADR:** [`coach-operator-verb-catalog-adr.md`](coach-operator-verb-catalog-adr.md)  
**Prior:** CX-verbs-p0-02

## 1. Goal

Ship catalog P1 verbs on the control plane with effect contracts.

## 2. Done-when

1. `press_key` focuses app and injects a key (effect when display/state changes or key accepted).  
2. `select` activates a named selectable/toggle/menu control via AT-SPI.  
3. `scroll` scrolls a named scrollable (or window) with observable offset/bounds change when available.  
4. Live Calculator proof where the UI allows; honest refuse otherwise.  
5. Journal + `cargo green`.

## 3. Boundaries

`drag` stays P2. No screenshots. No `0.0.0.0`.
