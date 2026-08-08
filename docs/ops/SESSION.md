---
dirty: false
active_lane: coach-seam
updated: 2026-08-08
---

# Ops session ledger

Living index for agent default operations (Cursor Session OS + Claude
milestone-loop). The RESUME truth is `docs/ops/resume/CURRENT.md`.
`docs/STATE.md` is the append-only product ledger — 2098 lines and
2026-08-01, so read it for history, never for "what is done now".

## Active lane

**coach-seam** (2026-08-07) — operator stopped the coach lane for an
audit: the CX daemon was a second agent surface; tracing/profiles/mock
were skipped. Audit + research + the true mock landed this session.
Journal: `docs/journal/2026-08-07-coach-seam-audit.md`.

Seam ADR **RATIFIED 2026-08-08** (operator `so lets plan and implement`;
§6 defaults stand until individually overruled). §10 amendment: the
DataFusion dep WAITS for a release on arrow 59 (stable 54.1 pins arrow
58); the in-tree pure-Rust reader is the interim lens — files stay
Arrow/Parquet, ruling substance unchanged.

**S1 tracing spine LANDED 2026-08-08** (plan
`docs/plans/coach-seam-s1-tracing-spine.md`): `seam` domain on
`dpl-fabric` (SeamSpan v1: verb/app/rung/downgraded_from/mechanism/
agency/ok/effect_observed/duration + lesson/beat + open attrs);
`dpl-ledger` writes it (`type`=verb → reliability is GROUP BY) +
`seam_reliability` fold + `seam_reliability`/`dump_rows` examples;
daemon owns a local signal fabric + tap (`--analytics-root`, loud boot
log), spans on every `/verb`+`/import`+`/resolve`+`/invoke` execution
AND refusal. Live-proven on gnome-calculator: 8 spans in Parquet, scroll
landed `rung:r2, downgraded_from:r1, mechanism:xdotool_wheel` — the
ladder downgrade is queryable data.

**S3a identity spine + pack registry LANDED 2026-08-08** (plan
`docs/plans/coach-seam-s3a-identity-spine.md`): new egui-free crate
`dpl-coach-pack` — minted-once `AppId` (`x.unresolved:<slug>` for
strangers), `FingerprintBundle` evidence, weighted resolver (4/3/3/2/2/1,
accept ≥ 3.0, ties honest `Unresolved`), schema-1 tolerant TOML packs
(unknown keys warn + survive rewrites), content-hashed `[signing]`,
tmp→bak→rename writes, promote-only in types (`PromotedRung` cannot say
r2/r3; registry refuses `UnresolvedWrite`). 25 unit tests; live drill:
running gnome-calculator resolved (score 8, live AT-SPI evidence) among
65 real rows, 64 honest strangers; Blender pack proven by shape (install
= S4 seat prep). architecture.md §3 + xtask census amended in-commit.

**NEXT:** **S3b** Foundry v0 + hub UI (Pick gains "Build fabric",
staged checkpointed job, console + profile card; every probe emits a
seam span; hub calls `FingerprintBundle::from_inventory` + registry
resolve — no more hardcoded seed strings) → **S4 Blender first pack**
(install Blender on this seat; thin a11y RECORDED, R0 Python rail is a
later slice) → S2 protocol rehoming → S5 Optimize cards (+ DataFusion
lens when its arrow-59 release ships). Full design:
`docs/journal/2026-08-08-coach-seam-s1-spine-s3-design.md`. Each slice:
plan doc → tests → live drill → `cargo green` → PR on double-green →
STATE/journal.

**Prior lane — ai-layer** (2026-08-03 state; #212 landed since, verify):
persona (Clyffy), session primitive, typed provider funnel.
Program `docs/plans/providers-presence-and-the-brain.md` §7–§9.
Its NEXT was: land #206 → Codex conversation transport.

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
| Coach | seam mock | rev 3 committed (#223); ADR ratified; standing red-pen invite on the 5 §6 defaults stays open |
| Coach | trace sink | RESOLVED 2026-08-07: Arrow/DataFusion over Parquet (operator go); mock rev 2 + research note updated |
| Coach | C2f / Notepad | #187; CX-harvest + Pass-2 parked |
| Harness | no mid-animation capture | every capture is `MotionPolicy::Suppressed` |
| Harness | 4 scenarios red headless | `chat-local-histories`, `chat-content-spine`, `chat-provider-tab`, `shell-navigation` — need a real display |

## Milestone

seam committed → docs aligned → #206 landed → **Codex transport** →
session primitive → Clyffy → group chat → research council
(coach C2f resumes on `approve`; operator owns the folder restructure)

## Signals

- Cursor/ops: `docs/ops/signals.jsonl`
