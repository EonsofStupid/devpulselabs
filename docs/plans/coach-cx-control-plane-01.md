# CX-control-plane-01 — Axum daemon + reality gates

**Status:** PASS 2026-08-01 — journal `../journal/2026-08-01-cx-control-plane-01.md`  
 
**ADR:** [`coach-control-plane-adr.md`](coach-control-plane-adr.md)  
**Prior failure:** CLI bridge PASS against ghost Calculator (no visible window)

## 1. Goal

Prove a **real** agent control plane: localhost Axum daemon; refuse when
Calculator is not mapped; PASS invoke only with observable display change.

## 2. Done-when

1. `dpl-coach-daemon` serves ADR endpoints on `127.0.0.1`.  
2. `GET /presence?app=gnome-calculator` is honest (mapped vs ghost).  
3. With **no mapped Calculator window**: `POST /import` and `/invoke` **refuse**.  
4. With Calculator **mapped and visible**: import → resolve → invoke changes display digit → JSON PASS.  
5. Journal + `cargo green`.

## 3. Changes

1. ADR + this plan + architecture tip.  
2. `dpl-coach-runtime` presence + effect helpers.  
3. `dpl-coach-daemon` Axum binary + session store.  
4. Live proof script/journal.

## 4. Boundaries

No `app` dep · no `0.0.0.0` · no silent pixel click fallback.
