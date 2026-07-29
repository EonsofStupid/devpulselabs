#!/bin/sh
# Milestone-loop RE-ENGAGEMENT feed (SessionStart hook). Everything this
# prints reaches the fresh session at zero tool-call cost: STATE, the
# latest journal, and the PREVIOUS session's transcript tail (recorded by
# pre-compact.sh / session-end.sh). The loop contract lives in
# .claude/skills/milestone-loop/SKILL.md; signals in ledger/README.md.
input=$(cat)
if command -v jq >/dev/null 2>&1; then
  src=$(printf '%s' "$input" | jq -r '.source // "unknown"' 2>/dev/null)
else
  src=$(printf '%s' "$input" | sed -n 's/.*"source"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
fi
[ -n "$src" ] || src=unknown
sh .claude/ledger/emit.sh session.reengaged "" "source=$src"

echo "=== STATE (docs/STATE.md) ==="
cat docs/STATE.md
echo "=== last 5 commits ==="
git log --oneline -5
echo "=== working tree ==="
git status --short --branch

j=$(ls -t docs/journal/*.md 2>/dev/null | head -1)
if [ -n "$j" ]; then
  echo "=== LATEST JOURNAL ($j) — absorb it, then: sh .claude/ledger/emit.sh journal.absorbed '<milestone>' ==="
  cat "$j"
fi

p=.claude/ledger/last-session.txt
if [ -f "$p" ]; then
  t=$(cat "$p")
  if [ -f "$t" ]; then
    echo "=== PREVIOUS SESSION — text of the last 500 transcript entries (fed free) ==="
    if command -v jq >/dev/null 2>&1; then
      tail -n 500 "$t" | jq -r '
        select(.type=="user" or .type=="assistant")
        | .message.content
        | if type=="array" then (.[] | select(.type=="text") | .text) else tostring end
      ' 2>/dev/null | tail -c 12000
    else
      # No jq in this shell: raw tail, harder to read but never silent.
      tail -n 500 "$t" | tail -c 12000
    fi
    sh .claude/ledger/emit.sh transcript.reviewed "" "entries=500 from=$t"
  fi
fi
exit 0
