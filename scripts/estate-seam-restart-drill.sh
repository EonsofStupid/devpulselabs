#!/usr/bin/env bash
# Two-boot curl drill: prove shell/route (+ optional lastComp) survives restart.
# Usage: from repo root, after `cargo build --release -p devpulselabs`
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXE="${EXE:-$ROOT/target/release/devpulselabs}"
PORT="${SEAM_PORT:-7341}"
BASE="http://127.0.0.1:${PORT}"

kill_app() {
  pkill -f "$EXE" 2>/dev/null || true
  sleep 0.5
}

cleanup() { kill_app; }
trap cleanup EXIT

kill_app
# Boot A — force a known route so estate writes shell/route.
"$EXE" --route wiredfront/editor --seam-port "$PORT" --mood 0 >/tmp/dpl-estate-boot-a.log 2>&1 &
APP_PID=$!
for i in $(seq 1 60); do
  if curl -sf "$BASE/state" >/tmp/dpl-estate-state-a.json 2>/dev/null; then
    ROUTE_A=$(python3 -c 'import json; print(json.load(open("/tmp/dpl-estate-state-a.json"))["route"])')
    if [ "$ROUTE_A" != "None" ]; then
      break
    fi
  fi
  sleep 0.25
done
test -s /tmp/dpl-estate-state-a.json || { echo "boot A: seam never answered"; exit 1; }
ROUTE_A=$(python3 -c 'import json; print(json.load(open("/tmp/dpl-estate-state-a.json"))["route"])')
ESTATE_A=$(python3 -c 'import json; print(json.load(open("/tmp/dpl-estate-state-a.json"))["estate"])')
echo "boot A: route=$ROUTE_A estate=$ESTATE_A"
test "$ROUTE_A" = "wiredfront/editor"
test "$ESTATE_A" = "True"

# Navigate to library so a *change* is persisted (not only the CLI boot route).
curl -sf -X POST "$BASE/command" \
  -H 'content-type: application/json' \
  -d '{"cmd":"navigate","route":"wiredfront/library"}' >/dev/null
sleep 0.75
curl -sf "$BASE/state" >/tmp/dpl-estate-state-a2.json
ROUTE_A2=$(python3 -c 'import json; print(json.load(open("/tmp/dpl-estate-state-a2.json"))["route"])')
echo "boot A after navigate: route=$ROUTE_A2"
test "$ROUTE_A2" = "wiredfront/library"

kill "$APP_PID" 2>/dev/null || true
wait "$APP_PID" 2>/dev/null || true
sleep 0.5

# Boot B — no --route; shell + seam should hydrate library from estate.
"$EXE" --seam-port "$PORT" --mood 0 >/tmp/dpl-estate-boot-b.log 2>&1 &
APP_PID=$!
for i in $(seq 1 40); do
  if curl -sf "$BASE/state" >/tmp/dpl-estate-state-b.json 2>/dev/null; then
    break
  fi
  sleep 0.25
done
test -s /tmp/dpl-estate-state-b.json || { echo "boot B: seam never answered"; exit 1; }
ROUTE_B=$(python3 -c 'import json; print(json.load(open("/tmp/dpl-estate-state-b.json"))["route"])')
echo "boot B: route=$ROUTE_B"
test "$ROUTE_B" = "wiredfront/library"
echo "estate-seam-restart-drill: PASS"
