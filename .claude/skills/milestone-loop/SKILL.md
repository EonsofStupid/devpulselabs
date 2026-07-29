---
name: milestone-loop
description: The internal workflow ledger loop — target a milestone, work, journal at 75-85% context, clear, hook-fed re-engagement, bounded planning (≤3 passes, ≤2 researches), anti-drift checks, EguiPilot verify, journal, repeat. Every phase emits a ledger signal.
---

# milestone-loop — the self-instrumenting workflow

Every phase transition emits a signal via
`sh .claude/ledger/emit.sh <event> <milestone> [detail]` (Bash tool, from
the repo root). The vocabulary and the operator's tuning recipes live in
`.claude/ledger/README.md`. This skill COMPOSES with the house laws — it
wraps `plan-code-change` / `deep-plan` / `eguipilotai`, never replaces
them.

Shared durable index with Cursor Session OS: `docs/ops/SESSION.md` +
`docs/ops/resume/CURRENT.md` (read on every re-engage; do not maintain a
second NEXT list).

## The loop

### 1. TARGET
Pick ONE milestone: a major task series, feature, or sub-feature. Name it
as a kebab slug. Emit `milestone.targeted <slug> "scope=<one line>"`.
One milestone at a time — a second target means the first is done or
explicitly parked (journal it).

### 2. WORK
Execute against the CURRENT plan (see §4). After each completed plan
step: emit `work.progress <slug> "step=N <name>"`.

### 3. JOURNAL AT 75–85%
When context pressure shows (the context-limit warnings, or the
PreCompact hook fires — its reminder is the hard backstop), STOP building
and write `docs/journal/<yyyy-mm-dd>-<slug>.md`:

- **Done so far** — landed PRs/commits, verified facts (with evidence).
- **Exact next step** — the plan step number + the first command to run.
- **Landmines** — everything mid-flight that would bite a fresh session
  (operator WIP, moved checkouts, half-applied edits, env quirks).
- **Verify commands** — how the fresh session proves state before moving.

Emit `journal.written <slug> "path=<journal>"`. Then let the clear
happen — never fight compaction with more work.

### 4. RE-ENGAGE (fresh context)
The SessionStart hook feeds — at ZERO tool-call cost — STATE, the latest
journal, and the text tail of the previous session's transcript (~500
entries; `transcript.reviewed` is emitted for you). Protocol:

1. Absorb the journal FIRST; emit `journal.absorbed <slug>`.
2. Run the journal's Verify commands before trusting any claim in it —
   parallel sessions move this repo (a memory-documented law).
3. PLAN the next pass: `plan-code-change` (or `deep-plan` when it earns
   its cost). Budget — HARD caps, emit as you spend:
   - up to **3 planning passes** (`plan.pass <slug> "n=1|2|3"`): a pass =
     ground → draft → gate; pass 2/3 only when the gate FAILS a named
     criterion (record which). Senior-clean on pass 1 is the bar, not the
     hope.
   - up to **2 web-research subagents** (`research.spawned <slug> "n=1|2"`)
     when the design touches evolving external truth (APIs, crates,
     industry practice). Zero is the default.
4. Emit `plan.built <slug> "path=docs/plans/<pass>.md"`.

### 5. ANTI-DRIFT (the plan polices the work)
The plan's §4 steps are the ONLY work queue. At every step transition —
and whenever you notice activity not matching a numbered step — run the
check: *"Which plan step is this?"*

- On a step → emit `drift.check <slug> "step=N verdict=on-plan"`.
- Not on a step → STOP. Either amend the plan explicitly (the
  plan-code-change amendment law: reground, edit, report) or drop the
  tangent. Emit `drift.check <slug> "step=N verdict=corrected"`.

A `corrected` is not a failure — it is the signal working. An hour off
plan without one IS a failure.

### 6. VERIFY
Every milestone slice proves itself on the real artifact through the
`eguipilotai` skill (scenarios/fixtures; the devtools exe), plus the pass
plan's §6. Emit `verify.eguipilot <slug> "scenario=<id> verdict=pass|fail"`
per scenario run. A fail loops back to §5's queue — never to ad-hoc
fixing.

### 7. CLOSE or CYCLE
- Milestone fully landed (PR merged, session-end trio done): emit
  `milestone.done <slug>`, write the closing journal entry, target next.
- Context filling before done: back to §3. Rinse, repeat.

## Rules

- The ledger is APPEND-ONLY telemetry; never edit ledger.jsonl.
- Signals are cheap — when in doubt, emit; the operator tunes on them.
- The caps (3 passes / 2 researches) are HARD; needing more means the
  milestone is scoped too big — split it and journal the split.
- Journals are repo-committed (`docs/journal/`) — they are handoffs, not
  scratch.
