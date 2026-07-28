# Phase 2 — the Signal Ledger (`dpl-ledger`)

Status: planned this pass (2026-07-28), executing under
`docs/plans/signal-ledger-and-lesson-foundation.md` §3–§5 / Phase 2 and
the 2026-07-28 rulings in §9. PR #116 (determinism gate) is on main;
this is the next independently landable phase.

## 1. Context

Phase 1.5 put coach + intent facts on the fabric. Phase 1.5's first
consumer (the determinism gate) proved the INTENT domain live. The
ledger is the durable consumer: one row per stamped envelope, Parquet
from day one, files are the truth, DuckDB stays an external lens
(Phase 3 ADR — not this pass).

## 2. Ground truth (read in-session, 2026-07-28)

- Stamped fabric domains today: `capture`, `jobs`, `providers`, `build`,
  `coach`, `intent`. Shell and deck remain unstamped (doctrine); the
  ledger does not invent stamps for them.
- No `parquet` / `arrow` / `duckdb` dep in the workspace yet.
- `parquet` crate with `default-features = false, features = ["arrow",
  "snap"]` keeps Snappy (pure Rust) and drops `zstd` (C). Verified via
  crates.io feature matrix 59.x — `zstd` is a separate feature; `snap`
  is the pure-Rust snappy crate.
- `.userdata/` is the ADR-014 Ring-2 home (`userdata_root()`).
- Harness flags are census-gated (xtask ↔ CLAUDE.md). New flag =
  amend both + the census table in the same commit.
- Structure audit: new ENGINE crate → EGUI_FREE + architecture.md layer
  line + workspace member + workspace.dependency, same commit.

## 3. Target design

### 3.1 Crate

`dpl-ledger` — one concern: durably record the stamped signal stream.
ENGINE layer, egui-free. Depends on: `dpl-fabric`, `parquet`
(`default-features=false`, `arrow`+`snap`), `serde`, `serde_json`.

### 3.2 On-disk layout

```
.userdata/analytics/
  schema.json                 # { "v": 1, "columns": [...] }
  events/
    date=YYYY-MM-DD/
      <domain>-NNNNNN.parquet
```

### 3.3 Row contract (schema v1)

Fixed columns, additive forever:

| column | type | notes |
|---|---|---|
| seq | u64 | fabric stamp |
| ms | u64 | fabric monotonic ms |
| wall_ms | u64 | writer wall clock at enqueue (UTC epoch ms) |
| domain | utf8 | capture\|jobs\|providers\|build\|coach\|intent |
| type | utf8 | envelope `type` (same vocabulary as the seam) |
| body | utf8 | JSON of the type-specific fields (redacted as seam) |
| class | utf8 | reserved §9.5; empty string until classification lands |
| dropped | u64 | writer-local dropped-row counter snapshot at flush |

Hive partition by `date` + filename by `domain` — DuckDB pushdown works
without those as columns.

### 3.4 Writer

- Off by default. `--ledger` (or `DPL_LEDGER=1`) starts it.
- Bounded `std::sync::mpsc` (capacity 4096) from the app's drain loop
  into a dedicated writer thread. UI never touches the filesystem.
- Overflow: drop + increment `dropped`; never block the frame.
- Flush on: batch ≥ 64 rows, or 500 ms idle, or shutdown.
- Segment roll: ≥ 10_000 rows or process exit → next `NNNNNN`.
- Shutdown: `LedgerHandle::flush_and_join` from `App::on_exit` /
  screenshot exit so the proof shot's own session is on disk.

### 3.5 App wiring

One drain site in the app (or a tiny `LedgerTap` owned by App) that
subscribes to all six stamped domains at fabric birth (same ear-before-
publish law as the shell), maps each to a `LedgerRow`, and `try_send`s.
Screenshot boots leave the ledger off unless `--ledger` is also set —
evidence mode and the ledger are independent switches.

## 4. Changes (execution order)

1. Pass plan (this file).
2. `crates/ledger/` + workspace registration + parquet dep pin.
3. Schema + writer + unit tests (round-trip, overflow drop, segment roll).
4. App: `--ledger` flag, Boot field, subscribe+drain, shutdown flush.
5. Structure audit + architecture.md + CLAUDE.md harness census.
6. STATE.md + journal notes.
7. Proof: run with `--ledger --screenshot`, assert ≥1 parquet file, and
   a pure-Rust read (or `parquet-read` via the crate's own test helper)
   returns the evidence intent row.

## 5. Boundaries

- No DuckDB / Arrow C++ / `-sys` crates. Phase 3 is the ADR.
- No shell/deck recording (unstamped).
- No classification logic — `class` column reserved empty.
- Not a second artifact index; never writes `/content`.
- No pinning behavior for strict mode (replay's pass).

## 6. Verification

- Unit: schema.json round-trip; overflow increments dropped; a written
  segment reopens and yields the seeded rows.
- Real artifact: `--ledger --screenshot` leaves a non-empty parquet
  under `.userdata/analytics/events/date=…/` containing the
  `intent`/`determinismChanged` evidence row (because screenshot forces
  evidence).
- Gate: `cargo green`; structure audit counts +1 egui-free member.
- Cost: not measured this pass (soak later); the bounded lossy channel
  is the structural guarantee.

## 7. Commit plan

One PR: `feat(ledger): Parquet signal ledger (Phase 2)`.

## 8. Evidence

Ground truth §2 from the working tree and crates.io feature matrix this
session. Parent plan ratified Parquet-from-the-start and files-are-truth;
this pass implements that cut without the DuckDB dependency.

Harness proof (2026-07-28): `--ledger --screenshot` wrote
`.userdata/analytics/schema.json` (v=1) and
`events/date=2026-07-28/intent-000001.parquet` containing
`determinismChanged` / `{"mode":"strict","cause":"evidence"}` (read back
via `dpl_ledger::read_parquet_rows` — `PROOF OK`). Unit matrix green:
schema round-trip, parquet round-trip, overflow drop, live fabric →
segment. Structure audit: 37 egui-free · 65 members · 34 flags. `cargo
tree -p dpl-ledger` shows no `*-sys` crates.