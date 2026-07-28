#!/bin/sh
# Milestone-loop CLEAR edge (PreCompact hook): record this session's
# transcript path so session-start.sh can feed its tail to the next
# context, emit the signal, and demand the journal if it is not written.
# jq-optional: sed fallback for the two stdin fields.
input=$(cat)
if command -v jq >/dev/null 2>&1; then
  t=$(printf '%s' "$input" | jq -r '.transcript_path // empty' 2>/dev/null)
  trg=$(printf '%s' "$input" | jq -r '.trigger // "unknown"' 2>/dev/null)
else
  t=$(printf '%s' "$input" | sed -n 's/.*"transcript_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
  trg=$(printf '%s' "$input" | sed -n 's/.*"trigger"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p')
  [ -n "$trg" ] || trg=unknown
fi
mkdir -p .claude/ledger
[ -n "$t" ] && printf '%s' "$t" > .claude/ledger/last-session.txt
sh .claude/ledger/emit.sh compact.pre "" "trigger=$trg"
echo "MILESTONE-LOOP: the context is about to clear ($trg). If the current milestone's journal is not on disk yet, write docs/journal/<yyyy-mm-dd>-<milestone>.md NOW (Done so far / Exact next step / Landmines / Verify commands), then: sh .claude/ledger/emit.sh journal.written '<milestone>' 'path=<journal>'"
exit 0
