# ADR-014 — The three-ring data root

| Field | Value |
|-------|-------|
| **Status** | ACCEPTED (operator ratification 2026-07-25; senior-review conditions 1–6 folded in) · **AMENDED 2026-08-03: analytics is Arrow/DataFusion over Parquet, not DuckDB** (operator ruling; see Amendment below) |
| **Owner** | `dpl-engine-capture` (the root owner) · pass plan `docs/plans/three-ring-root.md` |

## Decision

One portable application folder, three rings with hard boundaries:

| Ring | Folder | Contract |
|------|--------|----------|
| 0 — STATE | `.userdata\` | App state: `index.db` (Turso), the RING-2 analytics **Parquet estate** (queried in-process by DataFusion), `cache\`, `temp\`, `profiles\`. `cache\`+`temp\` are 100 % regenerable (deleting them can never lose work — the deletability contract is written into `.userdata\README.txt`); the DBs and profiles are precious. Never creative material. |
| 1 — WORKING | `assets\` | Raw + in-process, **auto-filed, zero dialogs**: `assets\captures\<kind>\<job>\…`. WiredFRONT (the PREP house) operates here. Shallow nesting by context — taxonomies calcify. |
| 2 — CONFIRMED | `content\` | The library universe, entered ONLY by an explicit operator gesture (KEEP / Save / Export). Indexed, portable, shareable. Workbench (the BUILD house) builds from here. Layout unchanged. |

Laws:

1. **Generation is automatic into Ring 1; promotion to Ring 2 is always
   explicit** — and transactional: copy + index write-through commit
   first, the assets original is removed only after success, and only
   when it lives under `assets\` (external sources are undeletable by
   construction). Failure direction = duplicate, never loss.
2. **Root-relative paths only** in the index, for every ring.
3. **Retention is a report, not a reaper** (RING-2 analytics): nothing
   auto-deletes operator material, ever.
4. **Additive migrations**: the index moved `content\artifacts.db` →
   `.userdata\index.db` by copy → verify → labeled rename
   (`.migrated-to-userdata`) — never deletion.
5. Rings resolve as siblings derived from `content_root()` (which keeps
   its arg → env → ambient → self-create chain); drill seams
   `ASSETS_ROOT` / `USERDATA_ROOT` mirror `CONTENT_ROOT`. Boot creates
   the rings and NAMES them in the log (the honesty law).

Precedent: Unreal `Saved`/`Intermediate`/`Content`, Resolve
cache/media-pool/gallery, Lightroom ingest/catalog/library. Embedded
Turso (metadata) + in-process Arrow/DataFusion over Parquet (analytics)
is the right-sized standalone stack — ClickHouse/Postgres rejected as
servers in a portable app.

## Amendment 2026-08-03 — analytics is DataFusion, not DuckDB

**Operator ruling:** *"we will be removing ducksdb and be using arrow data
fusion as well instead."*

The original text named embedded DuckDB as RING-2. Ground truth says the
codebase had already chosen the other half of that stack:

- `dpl-ledger` **already writes Parquet through Arrow** — `ArrowWriter`,
  `RecordBatch`, `arrow-array`/`arrow-schema` 59 (`crates/ledger/Cargo.toml`).
- Nothing in `crates/` ever linked DuckDB. It was a decision on paper with
  a Parquet-emitting ledger underneath it.

So this is alignment, not a pivot: **DataFusion queries the Parquet the
ledger already emits**, in-process and pure Rust, which keeps ADR-009's
spirit (no C++ engine embedded in a portable app) and removes a native
dependency the build never took on.

What does NOT change: the ring boundaries, root-relative paths, the
transactional promotion law, and law 3 — retention stays a report. The
query engine changed; the contract did not.

**Downstream:** `docs/plans/coach-lab-experiment-program.md` names "DuckDB
baselines" for Lab analytics. That plan is owned by another workstream —
flagged here rather than edited, so its owner rules on it.

## Rejected

- Everything-in-`content\` (working material silently pollutes the
  library; today's status quo failure).
- Tool-chosen locations (scratchpads, repo-root folders) — the drift
  this ADR exists to kill.
- Auto-promotion on capture — the library stays trustworthy only if a
  human confirms entry.
