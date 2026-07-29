#!/bin/sh
# The milestone-loop ledger's ONE emit seam (vocabulary: README.md here).
# Usage: emit.sh <event> <milestone> [detail...]
# Appends one JSONL line to .claude/ledger/ledger.jsonl — the operator
# tails this file to observe and tune the loop. Called by hooks AND by
# the agent at phase transitions; always from the project root.
# jq-optional: environments differ (Bash tool vs hook shell), so the
# fallback strips double quotes and builds the line by hand.
event="$1"
milestone="${2:-}"
if [ $# -ge 2 ]; then shift 2; else shift $#; fi
detail="$*"
ts=$(date -u +%Y-%m-%dT%H:%M:%SZ)
mkdir -p .claude/ledger
if command -v jq >/dev/null 2>&1; then
  jq -cn --arg ts "$ts" --arg e "$event" --arg m "$milestone" --arg d "$detail" \
    '{ts:$ts,event:$e,milestone:$m,detail:$d}' >> .claude/ledger/ledger.jsonl
else
  e=$(printf '%s' "$event" | tr -d '"\\')
  m=$(printf '%s' "$milestone" | tr -d '"\\')
  d=$(printf '%s' "$detail" | tr -d '"\\')
  printf '{"ts":"%s","event":"%s","milestone":"%s","detail":"%s"}\n' \
    "$ts" "$e" "$m" "$d" >> .claude/ledger/ledger.jsonl
fi
