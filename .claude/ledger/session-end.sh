#!/bin/sh
# Milestone-loop SESSION-END edge (SessionEnd hook): the non-compaction
# clear path — record the transcript path for the next re-engagement and
# emit the signal. jq-optional (sed fallback).
input=$(cat)
if command -v jq >/dev/null 2>&1; then
  t=$(printf '%s' "$input" | jq -r '.transcript_path // empty' 2>/dev/null)
  reason=$(printf '%s' "$input" | jq -r '.reason // "unknown"' 2>/dev/null)
else
  t=$(printf '%s' "$input" | sed -n 's/.*"transcript_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
  reason=$(printf '%s' "$input" | sed -n 's/.*"reason"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
  [ -n "$reason" ] || reason=unknown
fi
mkdir -p .claude/ledger
[ -n "$t" ] && printf '%s' "$t" > .claude/ledger/last-session.txt
sh .claude/ledger/emit.sh session.ended "" "reason=$reason"
exit 0
