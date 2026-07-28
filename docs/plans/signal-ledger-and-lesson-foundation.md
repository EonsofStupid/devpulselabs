# Signal Ledger + lesson foundation — the analytics substrate, without the dependency

Status: RATIFIED (operator, 2026-07-27) with one amendment — **Parquet from the
start**, not JSONL-first (see §3.2). Completes the RING-2 analytics promise made
in `docs/design/adr-014-three-ring-root.md` and the coach findings in
`docs/plans/operator-coach-a2ui.md` §13. Phase order ratified: Phase 1 first.

## 1. Context

Three separate threads arrived at the same missing piece.

**The coach can now express what a step means, and has nothing to point at.**
C1.5e landed `Touchpoint.until`, so a lesson step completes on observed state
rather than on a click. A live snapshot of `workbench/library` then returned
**six nodes**: five rail tiles and the chat window. The Workbench hub — the one
the first lesson targets — has **zero** instrumentation sites.

**The app emits signals and remembers nothing.** `docs/design/signal-doctrine.md`
is LOCKED law: per-domain typed enums, stamped at publish with a global `seq`
and a monotonic `ms`, explicitly described as "flight recorder for replay/
analytics". That recorder is never written down anywhere durable and queryable.
Today's persistence is three ad-hoc, unversioned JSONL files with no shared
schema: session `events.jsonl`, EguiPilotAI `trace.jsonl`, and an in-memory
256-entry ring buffer in the shell.

**DuckDB is already ratified, and deliberately fenced off.** ADR-014 names
`.userdata\analytics.duckdb` as RING-2 and calls "embedded Turso (metadata) +
embedded DuckDB (analytics)" the right-sized standalone stack. Every other
document fences it: *"own C-dep ADR"*, *"never smuggled"*, *"No DuckDB dep in
this diff"*. Meanwhile ADR-009 makes the house rule on C libraries explicit and
machine-enforced — `deny.toml` **bans ffmpeg crates outright** because ffmpeg is
a subprocess, never a crate dependency.

The reconciliation is the point of this plan: **the analytics foundation is a
file format and a schema, not a database.** DuckDB reads Parquet and JSONL in
place, with predicate and projection pushdown and no ingest step. So the durable,
versioned, queryable substrate can ship now with **zero new dependencies**, and
the DuckDB C-dep question can be answered later, on evidence, without blocking
anything.

## 2. Current state (ground-truthed 2026-07-27, this session)

| Fact | Evidence |
|---|---|
| Workbench, Present, Shows hubs have **0** instrumentation sites | `rg -c 'pilot_track\|pilot_button\|pilot_label' crates` |
| 99 sites exist, concentrated elsewhere: chat/window 50, composer 21, wiredfront 11, viziwiz 7, deck 4, shell 3 | same |
| A live snapshot of `workbench/library` returns 6 nodes | `pilot_snapshot`, fixture `library`, 1280×832 |
| Rail values pack two facts into one string: `active`, `inactive`, `active+open` | `pilot_snapshot` after navigating |
| Fabric stamps every publish with global `seq` + monotonic `ms` | `crates/fabric/src/lib.rs`, doctrine law 5 |
| 126 coach signals are declared by the app | `pilot_status.coach_signals` |
| No `duckdb`, `arrow`, or `parquet` dependency exists | `Cargo.toml`, `Cargo.lock` |
| `deny.toml` bans ffmpeg crates (ADR-009) | `docs/workspace.md` |
| Artifact index law: "files + sidecars are the TRUTH; this DB is a queryable index, rebuilt any time" | `crates/index/src/lib.rs` 1–19 |

## 3. Target design

### 3.1 The Signal Ledger

One new crate, `dpl-ledger`, owning a single concern: **durably recording the
stamped signal stream in a shape an analytical engine can query directly.**

```
.userdata/analytics/
  schema.json                 # the ledger's own version + column contract
  events/
    date=2026-07-27/
      shell-000001.jsonl      # append-only, size-rolled segments
      capture-000001.jsonl
```

Design commitments, each answering a law already on the books:

- **Hive-style partitioning by date and domain.** Not decoration: it is what
  lets DuckDB's pushdown skip files it does not need, and it keeps segments
  small enough to roll and prune as whole files.
- **One row per stamped envelope**, columns fixed by contract: `seq`, `ms`,
  `wall_ms`, `domain`, `type`, then domain payload as typed columns. The
  envelope shape is *already* specified by signal-doctrine law 4 — this writes
  that same envelope down instead of inventing a second one.
- **Versioned and additive.** `schema.json` carries a version; new columns are
  additive only, exactly as the capture taxonomy rule requires. A reader that
  does not know a column ignores it.
- **Redacted by construction.** Per `pre-release-capacity.md`, the ledger takes
  the same redacted projections the seam already publishes. No credentials, no
  opaque auth payloads, no free-form provider output.
- **Files are the truth; the database is disposable.** The same law the artifact
  index lives under. Any `.duckdb` file is a *cache over these files*, deletable
  at any time, rebuilt by pointing DuckDB at the glob.
- **Retention is a report, not a reaper** (ADR-014 law 3). The ledger may
  *report* what is old. It never deletes operator material.

### 3.2 Why JSONL first, Parquet when it earns it

DuckDB reads both in place. JSONL costs **zero new dependencies** and is
directly greppable and diffable, which matters while the schema is still
settling. Parquet buys columnar compression and row-group skipping, and costs
the `parquet` crate — which is pure Rust only with `default-features = false`
plus `snap`, since the default feature set pulls `zstd-sys` (C). That is a
real ADR-009 conversation, and it should happen when volume justifies it, not
now. The partition layout is chosen so the format can change underneath without
the query surface moving.

### 3.3 Lightweight by construction

The ledger must never be the reason a frame is late.

- The writer owns a **bounded** channel and its own thread. The UI publishes
  and returns; it never touches the filesystem.
- Back-pressure is **lossy and counted**, matching the fabric's own broadcast
  law: on overflow, drop and increment a dropped-rows counter that is itself a
  recorded fact. A ledger that stalls the app to avoid losing a row has chosen
  wrong.
- Batched appends with a size-and-time flush, so a burst is one write.
- **Off by default, one switch on.** No ledger, no thread, no file handles.

### 3.4 Instrumentation coverage

The lesson blocker, and the smaller half of the work. `hubs/workbench` gets
`pilot_track` at the surfaces a lesson names — Library, Episodes, and the
controls a first lesson walks through. The rail's compound `active+open` value
splits: route activity stays the `value`, fold state moves to `NodeState`,
where openness already belongs, so a condition can ask about one without
matching a string that encodes the other.

## 4. Changes (execution order)

Each phase is independently landable and independently valuable.

**Phase 0 — unblock and repair.** Land PR #103. Fix `locate_asset`'s test so a
custom `CARGO_TARGET_DIR` cannot fail the gate spuriously: the test resolves
from `CARGO_MANIFEST_DIR` rather than walking up from the test binary, which
only finds `assets/` when the target directory happens to sit inside the repo.

**Phase 1 — the lesson can be authored.** Instrument `hubs/workbench`; split the
rail's compound value into `value` + `NodeState`. Proof: a snapshot of
`workbench/library` returns the named surfaces, and a three-step lesson runs
end to end on the real exe.

**Phase 2 — the ledger.** `dpl-ledger` crate, schema document, bounded writer
thread, one subscription to the stamped domains, partitioned segments. Proof:
the app runs a session, and a DuckDB CLI query over the glob answers a real
question the app cannot answer today.

**Phase 3 — the DuckDB C-dep ADR.** Written *after* Phase 2 produces real data,
so the ADR argues from measured volume and query latency rather than from
taste. Three candidates, all live: (a) no dependency ever — DuckDB stays an
external tool over the files, the strictest reading of ADR-009; (b) the DuckDB
CLI as a subprocess, the exact ffmpeg precedent; (c) the `duckdb` crate with
`bundled`, which compiles a large C++ tree on every clean build for both
targets and is the option ADR-009's ban on ffmpeg crates argues hardest against.

## 5. Boundaries

- No DuckDB, Arrow, or Parquet dependency in this plan's phases 0–2. Phase 3 is
  an ADR, not an implementation.
- The ledger is **not** a second artifact catalog. Ring 2 stays Turso's.
- The ledger never writes into `/content` and never mutates a source.
- No new language runtime. C++ and Python enter only as subprocesses under a
  written ADR, per ADR-009 — the pure-Rust contract in `AGENTS.md` is not
  relaxed by this plan.
- Signal doctrine is untouched: the ledger is a *consumer*, adding no domain,
  no channel, and no new envelope.

## 6. Verification

- Unit: schema round-trip; segment rolling; bounded-channel overflow increments
  the dropped counter rather than blocking.
- Real artifact: run the exe with the ledger on, then query the partition glob
  and get a non-empty, correctly-typed answer.
- Gate: `cargo green` clean, and clean with `CARGO_TARGET_DIR` set to a path
  outside the repo — the condition that currently breaks it.
- Cost: a frame-time comparison with the ledger off and on, to hold the
  "never the reason a frame is late" claim to evidence rather than intent.

## 7. Commit plan

One PR per phase. Phase 0 rides with #103 or follows immediately; phases 1–3
are separate, reviewable, and revertible.

## 8. Evidence

Gathered 2026-07-27 in-session; numbers in §2 are from the live exe and from
`rg` over the working tree, not from memory. Web research on DuckDB's in-place
Parquet/JSONL reading, pushdown behavior, and the `bundled` build cost is what
makes the "foundation without dependency" claim safe to state.
