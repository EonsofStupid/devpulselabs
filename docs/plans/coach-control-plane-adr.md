# ADR — Coach control plane (localhost Axum daemon)

**Status:** RATIFIED 2026-08-01 — CX-control-plane-01 PASS (journal)  
 
**Related:** [`coach-overlay-adr.md`](coach-overlay-adr.md) · [`coach-bridge-fabric-runtime.md`](coach-bridge-fabric-runtime.md)  
**Industry:** kde-mcp / deskd / uia-x / Terminator MCP — daemon + agent HTTP/MCP; hands = a11y

## 1. Context

CLI probes printed `PASS` while the operator had no visible Calculator.
AT-SPI still listed `gnome-calculator` (zombie processes / unmapped
clients). That is not a product. The AI↔app bridge needs a **long-running
control plane** agents can call, with **reality gates** that refuse ghosts.

## 2. Decision

1. Ship **`dpl-coach-daemon`** — separate process, **never** a dep of
   `crates/app` production graph (same trust family as overlay).
2. Agent hop = **localhost-only Axum HTTP** (`127.0.0.1`, default port
   `7843`, overridable). Same stack as `dpl-seam` (ratified axum).
3. App hop stays **Tier A** (AT-SPI / later UIA) via `dpl-coach-runtime`
   fabric resolve/invoke — Calculator never speaks HTTP.
4. **Reality gate (hard):** Import / resolve / invoke **refuse** unless
   the target has a **mapped X11 client** matching WM_CLASS, outer size
   ≥ 50×50, and `_NET_WM_PID` alive. AT-SPI-only presence is insufficient.
5. **Effect gate (hard for digit seeds):** Invoke on Calculator-class
   seeds PASS only if display text **changes** (or typed refuse).
   `doAction(true)` alone is not PASS.
6. Overlay remains painter/observer under overlay ADR; daemon may spawn
   it with bridge `--window-rect` after gates pass.

## 3. Consequences

- New named process exception tip in `architecture.md` + structure audit.
- Pass-2 overlay stdio stays parked — HTTP control plane is the agent door.
- Production `devpulselabs` does not auto-start the daemon (operator /
  Advanced launch only), matching overlay launch gates.

## 4. Endpoints (v1)

| Method | Path | Role |
|---|---|---|
| GET | `/health` | liveness |
| GET | `/presence?app=` | mapped window + pid + atspi (truth) |
| GET | `/inventory` | cold+live list (presence-aware running flag) |
| POST | `/import` | mint fabric into daemon session (gated) |
| POST | `/resolve` | fabric resolve seed (gated) |
| POST | `/invoke` | Action + effect gate (gated) |
| POST | `/mount` | MountSpec (+ optional overlay spawn) |
| GET | `/verbs` | Verb catalog introspection |
| POST | `/verb` | Window verbs (`move`/`resize`/`focus`/`geom`) |
| GET | `/events?app=&seconds=` | SSE normalized geom facts (`moved`/`resized`) |

## 5. Non-goals

- Binding on `0.0.0.0` · auth beyond loopback · MCP (later adapter) ·
  production auto-launch · screenshot lesson loop.
