# Journal — 2026-07-28: Session OS

## Why

Operator asked for a project-stored, trackable default-ops loop any AI
can adopt: emit clear signals, work milestones to ~75–85% context,
journal, and on compact/clear resume from disk (last ~500 transcript
lines as raw safety net). Grounding researched against Cursor 2026
skills / hooks / subagents docs; rulings: committed `docs/ops/`,
auto-adopt skill, multi-model only for non-trivial, Passes A–E.

## What landed (`feat/session-os`)

| Piece | Path |
|---|---|
| Plan | `docs/plans/session-os.md` |
| Ops ledger | `docs/ops/SESSION.md`, `signals.jsonl`, resume pack |
| Skill | `.cursor/skills/session-os/` (auto) |
| Always-on boot rule | `.cursor/rules/session-os.mdc` |
| Agents | `grounder` (Fable) → `gap-closer` (Opus) → `executor` (Grok) |
| Hooks | `sessionStart`, `preCompact`, `stop` (loop_limit 1) |
| Pointers | `AGENTS.md`, `docs/STATE.md` |

## Evidence (this pass)

Hook dry-run (local): `sessionStart` returns env + context; `preCompact`
at 82% appends `compact.observed` + `context.budget`; `stop` nudges only
when `dirty: true`. No Rust / `cargo green` required (docs + Cursor
config only).

## Honest limits (still)

- No programmable 75% callback — skill journals; `preCompact` is backstop.
- No chat-clear hook — new chat boots from disk.
- `sessionStart` `additional_context` may drop (Cursor timing bug) — disk wins.
- First **live** Fable→Opus→Grok cycle deferred until operator says go on **p6**.

## Resume

After merge: new chat → Session OS boot → NEXT = Library section harness
(p6) per `docs/journal/2026-07-28-remaining-work.md`.
