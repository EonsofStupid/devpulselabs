---
dirty: true
active_lane: ai-layer
updated: 2026-08-03
---

# Ops session ledger

Living index for agent default operations (Cursor Session OS + Claude
milestone-loop). The RESUME truth is `docs/ops/resume/CURRENT.md`.
`docs/STATE.md` is the append-only product ledger — 2098 lines and
2026-08-01, so read it for history, never for "what is done now".

## Active lane

**ai-layer** — the persona (Clyffy), the session primitive, and the one
typed funnel every frontier provider is forced through.
Program `docs/plans/providers-presence-and-the-brain.md` §7–§9.

**NEXT:** commit the provider runtime seam → land the docs alignment →
land #206 → **Codex conversation transport** (the real gap).

**CLOSED** (nothing elided — the ellipsis in the previous revision was
hiding exactly the "previous completed" that kept getting lost):
D1+H1 (#180) · `/experiments` (#185) · STATE ledger (#189) · typed run
manifest (#190) · configured artifact root (#191) · **the whole Providers
program: P1 · V1 · V2 · V3 · A1 · A2** (#192) · Enter/Shift+Enter ·
AI usage domain (#169) · coach C2e/C2f + fabric bridge (#179/#187/#188)

## Rulings in force (operator, 2026-08-03)

| # | Ruling | Supersedes |
|---|---|---|
| 1 | **Chat owns its store** — one chat store; platform reads the published fact via `fabric.provider_roster()` | ADR-017 §2 (`Tree::Agents`) |
| 2 | **Analytics = Arrow/DataFusion over Parquet** | ADR-014's DuckDB |
| 3 | **Clyffy** = global/per-session face; ABSENT from group chat; backing provider VISIBLE (font + symbol) | — |
| 4 | **Round-robin IS group chat** — one shared transcript, operator in the ring | — |
| 5 | **Research council** = isolated fan-out; disagreement FIRST; disclose each result as it lands | — |
| 6 | **Passes law** — name the boundary you align to; STOP for the operator on a new one | — |
| 7 | **Method** — locate with tools, then read the file IN FULL before editing | — |

## Opens

| Kind | Ref | Notes |
|---|---|---|
| AI layer | Codex transport | `providers/codex` = `activation.rs` only; nothing qualifies `StreamTurns`. Grok's `runtime.rs` is the template; the seam is the target shape |
| AI layer | runtime seam | BUILT, UNCOMMITTED — `chat/runtime/src/seam.rs`, `grok/src/seam.rs`, 30 tests green |
| AI layer | chat store migration | `provider-profiles.db` + roster converge on ONE store; moves existing operator data, so its own pass |
| AI layer | session primitive | none exists — `grep session crates/seam` = 2 hits, neither a conversation. Group chat and the council both need it |
| Structure | 45 flat crates | §4's naming law already mandates grouped folders past ~3; `chat/*`+`hubs/*` comply, the rest predate it. **Operator is doing this pass** |
| Docs | unpushed | `docs/align-store-and-analytics` (`ec805f9`) — ADR-014, ADR-017 amendment, map §7–§9 |
| PR | #206 open | harness `--route` must not persist as the landing route; my drills corrupted the operator's estate |
| Debt | `chat-renderers` | dead AND duplicated by `chat/window::event_row`; #169 merged more into it |
| Debt | `chat-extensions` | zero references |
| Debt | Claude decoder | `UNKNOWN` for `last-prompt`/`ai-title`/`mode`/`permission-mode` |
| Providers | Cursor / Gemini | probe needles UNVERIFIED until their CLIs exist on a machine |
| Coach | C2f / Notepad | #187; CX-harvest + Pass-2 parked |
| Harness | no mid-animation capture | every capture is `MotionPolicy::Suppressed` |
| Harness | 4 scenarios red headless | `chat-local-histories`, `chat-content-spine`, `chat-provider-tab`, `shell-navigation` — need a real display |

## Milestone

seam committed → docs aligned → #206 landed → **Codex transport** →
session primitive → Clyffy → group chat → research council
(coach C2f resumes on `approve`; operator owns the folder restructure)

## Signals

- Cursor/ops: `docs/ops/signals.jsonl`
