# Journal — milestone-loop-ledger (2026-07-28)

The loop's own first artifact: this journal exercises the format the
milestone-loop skill demands.

## Done so far

- `.claude/ledger/` built: `emit.sh` (one emit seam, jq-optional),
  `session-start.sh` (re-engagement feed: STATE · latest journal ·
  previous session's 500-entry transcript tail, zero tool-call cost),
  `pre-compact.sh` + `session-end.sh` (clear edges: record transcript
  path, emit, demand the journal), `README.md` (signal vocabulary + the
  operator's tuning recipes).
- `.claude/skills/milestone-loop/SKILL.md`: the loop contract — target →
  work → journal at 75–85% → clear → hook-fed re-engage → plan (≤3
  passes, ≤2 web researches, HARD caps) → anti-drift checks bound to the
  plan's numbered steps → EguiPilot verify → journal, repeat.
- `.claude/settings.json` hooks: SessionStart/PreCompact/SessionEnd →
  the three scripts (the old STATE echo lives inside session-start.sh).
- `.gitignore`: ledger.jsonl + last-session.txt (local telemetry).
- END-TO-END TESTED live: emit → JSONL clean; pre-compact armed
  `last-session.txt` with this session's real transcript; session-start
  fed STATE + the tail and emitted `session.reengaged` +
  `transcript.reviewed`. Environment truth: the Bash tool's PATH lacks
  jq, the git-bash hook environment has it — every script degrades
  honestly without it.

## Exact next step

Land the pass: branch `feat/milestone-loop-ledger`, commit, push via the
server1 lane (local gate slices get killed — memory law), PR, merge,
session-end trio. First command: `git checkout -b feat/milestone-loop-ledger`.

## Landmines

- Parallel sessions are landing PRs fast (#119–#127 today); STATE.md tip
  collisions are ROUTINE — resolve keep-both, newest first, Edit tool
  only (UTF-8 law).
- The estate LXC runner just gained `libgbm-dev`; if green-linux fails on
  another `-l<x>`, install `lib<x>-dev` via `pct exec 121` from server1.
- Hook changes take effect on the NEXT session start — this session
  still runs the old SessionStart hook.

## Verify commands

- `bash -c "printf '{}' | sh .claude/ledger/session-start.sh | head -5"`
- `cat .claude/ledger/ledger.jsonl` (expect the milestone.targeted line)
- `git log --oneline -3` (expect the ledger commit on main after merge)
