# Journal — 2026-08-01: CX-control-plane-01 Axum daemon + reality gates

## Verdict

**PASS** — localhost Axum control plane refuses ghost Calculator; with a
mapped window, import → resolve → invoke reports **observable display
change** (`effect_observed: true`).

## Why this pass exists

Prior CLI bridge printed PASS while the operator had no visible Calculator.
AT-SPI / zombie processes lied. Reality = mapped X11 client + live pid.

## Evidence

### Closed (killed all `gnome-calculator`)

```
GET /presence?app=gnome-calculator
→ real=false, mapped_window=false, atspi_listed=false

POST /import {"name":"Calculator"}
→ HTTP 409 refused: "Calculator is not running" / not operator-real
```

### Open (operator-visible window on DISPLAY=:1)

```
GET /presence → real=true, mapped_window=true, 482×613, pid alive

POST /import → ok, session, 81 nodes, seed 7
POST /resolve → name=7, window_rect=16,293,64,44
POST /invoke {"require_effect":true}
→ action=Click, display_before="", display_after="7", effect_observed=true
```

## Shipped

| Piece | Role |
|---|---|
| ADR `coach-control-plane-adr.md` | Localhost Axum; app hop = AT-SPI |
| `dpl-coach-runtime` presence | Mapped window + pid gate |
| `invoke_gated(..., require_effect)` | Digit seeds need display change |
| `dpl-coach-daemon` | `/health` `/presence` `/inventory` `/import` `/resolve` `/invoke` `/mount` |

## Operator commands

```bash
cargo build -p dpl-coach-daemon
./target/debug/dpl-coach-daemon --port 7843
curl -s http://127.0.0.1:7843/presence?app=gnome-calculator
# open Calculator, then:
curl -s -X POST http://127.0.0.1:7843/import -H 'content-type: application/json' \
  -d '{"name":"Calculator","seed":"7"}'
```

## Next

Wire Teacher/agent clients to this plane · Cursor effect observables ·
MCP adapter optional later.
