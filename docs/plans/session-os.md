# Pass — Session OS (agent default operations)

## 1. Intent

Give every Cursor agent a durable, project-stored operating loop:
boot from disk → identify opens → plan (capped) → multi-model for
non-trivial work → execute one cut → journal near context budget →
resume after compact/clear. Distinct from product `dpl-ledger`.

## 2. In / out / blast radius

**In:** `docs/ops/` ledger, `.cursor/skills/session-os/`, always-on
rule, `.cursor/agents/` (grounder / gap-closer / executor), hooks
(`sessionStart`, `preCompact`, `stop`), AGENTS.md + STATE pointers,
journal.

**Out:** product Parquet ledger changes; Automations; changing
clyffy skills; implementing coach-lane p6.

**Blast radius:** always-on rule tokens every chat (kept ≤30 lines);
hooks fire on IDE sessions (fail-open); no Rust/crate changes.

## 3. Operator rulings (2026-07-28)

- Ops ledger: committed `docs/ops/`
- Skill: auto-adopt (`disable-model-invocation` omitted)
- Multi-model: non-trivial only
- Full slice: Passes A–E

## 4. Atomic tasks

1. Seed `docs/ops/` (SESSION, signals schema, resume CURRENT)
2. Skill + references + thin always-on rule
3. Named agents with pinned models
4. Hooks + scripts (resume inject attempt, preCompact snapshot, stop dirty)
5. Wire AGENTS.md / STATE / journal; dry-run checklist for p6

## 5. Risks

- `sessionStart` `additional_context` may drop (known Cursor timing bug)
  → disk + skill boot is source of truth; `env` still set.
- `preCompact` cannot block → skill journals proactively; hook is backstop.
- No chat-clear hook → new chat relies on sessionStart + files.

## 6. Verification

1. Files present and linked from AGENTS.md / STATE.
2. Hook scripts parse sample JSON and exit 0 (`scripts/` dry-run).
3. Skill description contains trigger terms; SKILL.md &lt; 500 lines.
4. Operator: first live cycle on coach lane p6 when they say go.

## 7. Done when

Session OS is committed on a PR; next chat can boot from
`docs/ops/SESSION.md` + `resume/CURRENT.md` without prior conversation
memory.

## 8. Evidence

- Hook dry-run 2026-07-28: `sessionStart` → env + additional_context;
  `preCompact` @82% → `compact.observed` + `context.budget` in
  `signals.jsonl`; `stop` followup only when `dirty: true`.
- Journal: `docs/journal/2026-07-28-session-os.md`.
- Live Fable→Opus→Grok on p6: deferred to operator "go" after merge.
