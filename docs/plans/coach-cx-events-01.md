# CX-events-01 — observe SSE + window verbs (move/resize/focus)

**Status:** PASS 2026-08-01 — journal `../journal/2026-08-01-cx-events-01.md`  
**ADR:** [`coach-operator-verb-catalog-adr.md`](coach-operator-verb-catalog-adr.md) (RATIFIED)  
**Prior:** CX-control-plane-01 · research `bleeding-edge-control-plane.md`

## 1. Goal

Ship the first **observe + window verb** slice of the catalog: agents can
`move` / `resize` / `focus` a presence-gated app and subscribe to normalized
geometry facts over localhost SSE — without screenshots.

## 2. Done-when

1. ADR status RATIFIED.  
2. `GET /verbs` lists catalog wire names + shipped vs planned.  
3. `POST /verb` with `move` / `resize` / `focus` against Calculator reports
   `effect_observed` with before/after outer rect (or focus truth).  
4. `GET /events?app=gnome-calculator&seconds=N` SSE emits `moved` / `resized`
   when geometry changes (agent or operator).  
5. Live proof + journal + `cargo green`.

## 3. Changes

1. `dpl-coach-runtime` — `OuterGeom`, `window_geom` / `window_move` /
   `window_resize` / `window_focus` (Linux X11 via `x11rb`).  
2. `dpl-coach-daemon` — `/verbs`, `/verb`, `/events` (geom poll → SSE).  
3. Control-plane ADR endpoint tip + this plan + journal.

## 4. Boundaries

No `app` dep · no `0.0.0.0` · no AT-SPI `window:move` as sole source ·
no MCP runtime · open/close/type_text deferred.
