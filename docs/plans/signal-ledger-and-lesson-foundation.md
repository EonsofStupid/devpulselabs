# Signal Ledger + lesson foundation — the analytics substrate, without the dependency

Status: RATIFIED (operator, 2026-07-27) with amendments — **Parquet from the
start**, not JSONL-first (see §3.2), and the **2026-07-28 rulings** on the
closed loop, determinism, operator intent, and classification (see §9).
Completes the RING-2 analytics promise made in
`docs/design/adr-014-three-ring-root.md` and the coach findings in
`docs/plans/operator-coach-a2ui.md` §13. Phase order ratified: Phase 1 first,
then Phase 1.5 (§9.6), then the ledger.

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
      shell-000001.parquet    # append-only, size-rolled segments
      capture-000001.parquet
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

### 3.2 Parquet from the start (AMENDED — operator, 2026-07-27)

The original draft argued JSONL-first, Parquet when volume earned it. The
operator ruled the other way: the ledger writes **Parquet from day one**, via
the pure-Rust configuration of the `parquet` crate — `default-features =
false` plus `snap`, which keeps `zstd-sys` (C) out and stays inside ADR-009
without an exception. What the JSONL argument was protecting (greppable,
diffable while the schema settles) is answered instead by the schema document
and by DuckDB/`parquet` CLI tooling over the files. Segment file extensions in
§3.1 read `.parquet` accordingly. DuckDB itself still enters only at Phase 3,
as an ADR.

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

**Phase 1.5 — the coach speaks on the record, and intent becomes law-in-code**
(added under the §9 rulings). Two small pieces, one PR. First, a `coach`
domain on the fabric: touchpoint lifecycle facts (armed, mounted, triggered,
satisfied, unmounted) published as stamped signals, so the ledger has coach
rows to record the day it lands, the activity feed shows lessons breathing,
and A/B measurement has its outcome stream. The publisher is the devtools-
gated pilot bridge; the domain itself is unconditional, like every other.
The coach **never subscribes to its own domain** — a touchpoint waiting on a
coach event would self-trigger. Second, `dpl-intent`: the operator-intent
proof type (§9.3) as a dependency-free crate, so every privileged transition
that follows has one gate to import instead of one convention to remember.

**Phase 2 — the ledger.** `dpl-ledger` crate, schema document, bounded writer
thread, one subscription to the stamped domains, partitioned segments. Coach
rows carry lesson outcomes; the schema reserves the §9.5 classification
columns (`class`, never the value). Proof: the app runs a session, and a
DuckDB CLI query over the glob answers a real question the app cannot answer
today.

**Phase 3 — the DuckDB C-dep ADR.** Written *after* Phase 2 produces real data,
so the ADR argues from measured volume and query latency rather than from
taste. Three candidates, all live: (a) no dependency ever — DuckDB stays an
external tool over the files, the strictest reading of ADR-009; (b) the DuckDB
CLI as a subprocess, the exact ffmpeg precedent; (c) the `duckdb` crate with
`bundled`, which compiles a large C++ tree on every clean build for both
targets and is the option ADR-009's ban on ffmpeg crates argues hardest against.

## 5. Boundaries

- No DuckDB or Arrow dependency in this plan's phases. The pure-Rust `parquet`
  crate (`default-features = false`, `snap`) enters at Phase 2 under the §3.2
  amendment. Phase 3 is an ADR, not an implementation.
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

## 9. Ratified rulings (operator, 2026-07-28)

A design session on determinism, escalation, and sensitive data closed with
these rulings. They are law for this plan's remaining phases and for the
replay/A-B work that follows.

### 9.1 The closed loop is the product shape

Analytics exists to close a loop, not to fill a dashboard: the platform
observes (signals → ledger), understands (queries over the ledger), and then
**recommends an action that can trigger the guided workflow itself** — "you
really should do X" arrives with a button that mounts the lesson. DuckDB's
role, in the operator's words, is to work with Turso and the platform to
**generate tools and visual widgets** that drive that loop. It is a lens over
the ledger files, never a store of record.

### 9.2 Determinism is a named, dynamic mode

Two modes, chosen per run:

- **Behavioral** (default): the same trace produces the same *step outcomes*.
  Pixels, animation timing, and wall-clock may vary. This is the measurement
  mode — A/B comparison of lesson arms runs here.
- **Strict** (escalated): byte-identical output — fixed clock, pinned RNG,
  locked renderer identity. This is the evidence mode — baselines, proof
  shots, anything that will be *compared byte-for-byte later*.

Escalation is **dynamic**: strict engages when the run is sensitive or when
the operator demands it. Evidence forces strict; measurement stays
behavioral.

### 9.3 The operator intent gate (the provenance law)

Privileged transitions — strict mode on demand, and every future state change
expensive enough to require unambiguous operator intent — are authorized by
**the provenance of the keystroke, not the pattern in a string**:

- A slash command (e.g. `/strict`) is recognized at the operator's
  input-submit event, from the operator's own text buffer, *before* that text
  ever becomes a message. **Model output is never re-parsed for commands**; a
  transcript containing `/strict` is just text.
- The UI path is a second-acknowledgement control the coach cannot
  synthesize: the pilot attributes injected input as `Agent`, and only
  `Operator`-attributed activation may mint the proof.

Law-in-code: `dpl-intent`'s `OperatorIntent` is a single-use proof value with
no `Clone`, no `Default`, and no serde — it cannot arrive over a wire, be
minted from config, or be replayed from a ledger. Its two constructors name
the two channels, so every construction site is a greppable audit surface.

### 9.4 Redaction outranks reproducibility

If a run touches classified material, redaction wins over replay fidelity,
always. A strict-mode run over sensitive content is redacted first and
byte-identical second; if those conflict, the bytes lose.

### 9.5 Classification is three separated layers

- **Layer 1 — base vocabulary.** Universal taxonomies (the NARA CUI Registry
  category codes, Presidio's PII entity set) embedded as static, versioned
  tables. The ledger stores **the class, never the value** — `CUI//PRVCY` is
  a column entry; the phone number is not.
- **Layer 2 — per-application tuning packs.** For heavily-used hosts (Cursor,
  Blender, SolidWorks, After Effects, chat clients…), a pack maps that app's
  surfaces onto the base vocabulary and carries perception and pedagogy
  tuning. Packs are versioned against app versions. An AI (frontier model)
  authors and maintains packs **offline**; the runtime is table lookup only.
- **Layer 3 — identity without recovery.** When "did the same secret appear
  twice" matters, store an HMAC of the value with the key living outside the
  ledger. Equality and joins, nothing recoverable. One function, not a
  system.

Compile-time redaction stays the first line: sensitive fields ride `Secret`-
style wrappers with no `Serialize`, so the ledger cannot receive them even by
accident.

### 9.6 Sequence and boundaries for the rulings

Build order: intent gate + coach fabric domain (Phase 1.5) → determinism
modes wired to the gate → ledger (Phase 2, schema carrying Layer-1 classes) →
DuckDB ADR (Phase 3). Games integration (multiplayer tips during training) is
**parked**: training-only was the ruling, and the anti-cheat hazard is noted;
nothing in this plan builds toward it.
