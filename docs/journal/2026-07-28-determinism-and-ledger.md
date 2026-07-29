# Journal — 2026-07-28: determinism behind the gate, then the ledger

Narrative companion to STATE.md. Append-only: this entry closes the
coach → intent → determinism → ledger spine that Phase 1.5 opened.

## What this arc set out to do

After Phase 1.5 put coach lifecycle facts on the fabric and minted
`OperatorIntent` with no consumer, the operator asked for the next
methodical cut: give the proof a first consumer (determinism modes),
then land the durable record those facts were written for (`dpl-ledger`).
The product shape underneath never changed — analytics observes, the
platform recommends, a recommendation can mount a guided workflow —
but the foundation had to exist as files before DuckDB earned an ADR.

## What landed

### Determinism modes (PR #116, squash into main ahead of the ledger)

Plan: `docs/plans/determinism-intent-gate.md`. Proof shot:
`docs/proof/determinism-evidence-2026-07-28.png`.

- `DeterminismGate` in `dpl-intent`: `Behavioral | Strict`, escalate only
  by spending `OperatorIntent`, `force_evidence` for proof shots,
  evidence outranks the operator and refuses de-escalation.
- INTENT fabric domain (doctrine law 11): `DeterminismChanged` /
  `IntentRefused` through seam SSE and the shell activity feed.
- Two mint sites only — chat `/strict`·`/behavioral` at the Ai composer's
  Send (whole-buffer exact, draft cleared before it becomes a message),
  and the inspector DETERMINISM card (arm → confirm). Both refuse
  synthetic (agent-injected) activation and publish the refusal.
- `--screenshot` boots force evidence. Pinning (clock / RNG / adapters)
  deliberately deferred — the switch and its guards land first so
  authorization is settled law when replay arrives.

### Signal ledger Phase 2 (PR #117, squash `8845afd`)

Plan: `docs/plans/signal-ledger-phase2.md`. Parent plan:
`docs/plans/signal-ledger-and-lesson-foundation.md` Phase 2.

- New ENGINE crate `dpl-ledger`: hive-partitioned Parquet under
  `.userdata/analytics/` via pure-Rust `parquet` + `snap` (ADR-009
  clean — `cargo tree` shows no `*-sys`).
- Off by default; `--ledger` or `DPL_LEDGER=1` starts a bounded writer
  thread. Overflow drops and counts; the UI never touches the filesystem.
- Schema v1 columns: `seq · ms · wall_ms · domain · type · body · class ·
  dropped`. `class` reserved empty for §9.5 classification.
- Stamped domains only: capture, jobs, providers, build, coach, intent.
  Absorbed `CardAuthored` when main moved under the PR.
- Harness proof: `--ledger --screenshot` wrote
  `intent-000001.parquet` with `determinismChanged` /
  `{"mode":"strict","cause":"evidence"}` — read back with the crate's
  own reader (`PROOF OK`). DuckDB stays Phase 3.

## What surprised us

**Main is a moving train.** #117 sat CONFLICTING twice while authoring,
jobs-board, L3, DS2, assist mode, and runner runbooks landed. The house
lesson from #103 / #109 held: a CONFLICTING PR gets no Actions runs —
merge main, keep both STATE entries, re-push. Local `cargo green` (the
pre-push hook) remains the enforcement; Windows CI failure on the first
run was stale against an outdated merge ref.

**`CARGO_TARGET_DIR` still bites proof shots.** The first evidence
screenshot hit a stale `./target/release` binary while the real build
lived under the sandbox cache. Always run the binary cargo just built,
or unset the override for operator-facing proof paths.

**Fabric enum drift is real.** `BuildEvent::CardAuthored` arrived on
main between ledger drafts; the mapper had to grow an arm in the merge
commit or the crate would not compile. Census law for fabric variants
is not yet a structure-audit fitness function — worth remembering when
the ledger is the first consumer that must match every stamped arm.

## What is NOT done (honest)

- Strict-mode **pinning** (fixed clock, pinned RNG, locked adapters) —
  the mode exists; the behavior does not.
- Live agent refusal proof for `/strict` via synthetic Send (unit +
  provenance law cover it; the optional SSE witness was left on the
  table).
- DuckDB ADR (Phase 3) — needs measured volume from real ledger files.
- Classification Layer-1 vocabulary into the `class` column.
- The foreign-monitor / overlay / lesson product (C2s → C2a → C2b) —
  foundation for *recording* lessons is here; painting the whole
  monitor is a different stack.

## Resume

Cleared remaining work for this lane lives in
`docs/journal/2026-07-28-remaining-work.md`. Read that before the next
code pass. Parallel lanes (authoring Pass B, chat assist bake-off, L4,
workflow doctrine) have their own journals — do not steal their next
without an operator call.
