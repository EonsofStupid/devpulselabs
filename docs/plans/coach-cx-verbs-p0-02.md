# CX-verbs-p0-02 — open / close / type_text / wait_until / assert_state

**Status:** PASS 2026-08-01 — journal `../journal/2026-08-01-cx-verbs-p0-02.md`  
**ADR:** [`coach-operator-verb-catalog-adr.md`](coach-operator-verb-catalog-adr.md)  
**Prior:** CX-events-01 (move/resize/focus/observe)

## 1. Goal

Ship the remaining **P0** catalog verbs so Teacher can open/close an app,
wait/assert presence, and type into an editable (with Calculator keystroke
fallback for display digits).

## 2. Done-when

1. `open` → presence becomes `real` (spawn via inventory `exec_hint`).  
2. `close` → presence becomes not real (WM delete, then TERM).  
3. `wait_until` `{predicate: real|unreal}` within timeout.  
4. `assert_state` `{expect: real|unreal}` pass/fail JSON.  
5. `type_text` changes Calculator display (keystroke after focus) with
   `effect_observed`.  
6. Live proof + journal + `cargo green`.

## 3. Boundaries

No `0.0.0.0` · no screenshot · no MCP · P1 select/scroll deferred.
