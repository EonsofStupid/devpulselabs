# Phase 3 — the analytics program: instrument, then crunch

Status: planned 2026-07-29. Executes the Phase 3 slot deferred by
`signal-ledger-and-lesson-foundation.md` ("Phase order: 1 (coverage) →
2 (ledger) → 3 (ADR)") and named NEXT in `docs/STATE.md`
("… → strict pinning → **DuckDB ADR** → classification").

Supersedes the single-pass `phase3-analytics-adapter.md`, authored and
deleted the same session once the operator's product vision widened the
scope from "query the ledger" to "crunch editor + AI stats". Everything
in it is absorbed here.

Operator rulings (2026-07-29, verbatim):

> *"duckdb should be a sidecar, it should be a dedicated sub-process, and
> it should be behind stats.adapter or analytics.adapter for our agnostic
> layer. whatever the platform industry aligned would be and then allow
> parquet for escape hatch and file truth while having it work with
> duckdb and turso for whole backend system."*

> *"my vision is to build this like a true product would relying on
> duckdb for the deep crunching analytics of the stats of the editors,
> AI, and also have it work with turso."*

Ratified this session (four decisions):

1. **Instrument first** — signal domains before the query engine.
2. **AI stats** = cost/tokens per provider+model · latency/failure ·
   attribution (what the AI was doing, was it kept) · prompt/response
   content. *All four.*
3. **Editor stats** = friction · ingredient usage · throughput ·
   interaction trace for replay · **overall production stats**.
4. **Turso records, DuckDB crunches** — no rollup materialization, no
   `ATTACH`-and-join coupling.

## 1. Context

Phase 2 built storage with no query engine: the ledger writes
Hive-partitioned Parquet under `.userdata/analytics/`, and the only
reader is `dpl_ledger::read_parquet_rows` — a hand-rolled full-scan row
reader built as a *proof* helper. No aggregation, no pushdown, no ad-hoc
query.

But the decisive finding this session is that **the two things the
operator wants to crunch emit nothing analyzable today**:

- **AI**: `crates/chat/` contains ZERO fabric calls — no `publish_*`,
  nothing. `ProviderEvent` is published exclusively from `crates/app/`
  and all 14 variants are auth-handshake lifecycle
  (`ActivationRequested`, `DeviceCodeReceived`,
  `SystemBrowserDispatched`, `Ready`, `Failed` …). No model, no tokens,
  no cost, no latency. The single usage fact anywhere is
  `ChatEventContent::UsageUpdated { summary: String }` — **prose**
  (`"Codex token usage updated"`), chat-local, unstamped, never balanced
  into the ledger. You cannot aggregate a sentence.
- **Editors**: `EditorEvent` has zero hits repo-wide. No domain, no
  publish, no subscribe. `ShellEvent`/`DeckEvent` are deliberately
  UNSTAMPED per signal doctrine, so even navigation carries no `seq` to
  order against a capture.
- **Turso**: `crates/index` creates exactly one table, `artifacts`.

Building the engine first would deliver excellent queries over capture /
jobs / build / coach / intent and **not one row about editors or AI** —
a Ferrari with no fuel. Hence: instrumentation is Phase 3's substance;
the sidecar is its payoff.

What already exists and needs NO new instrumentation:
`BuildEvent::{CardAuthored, SlidePlaced, EpisodeChanged}` (W10 + the
authoring charter's "every ask leaves a real card") and the full
`CaptureEvent` surface. **Overall production stats are largely derivable
from these today** — a query, not a new domain.

## 2. Ground truth (read in-session, 2026-07-29)

- Fabric domains and stamping: `capture`, `jobs`, `providers`, `build`,
  `coach`, `intent` are `Stamped<T>`; `shell` and `deck` are NOT.
- `crates/fabric/src/lib.rs:248` `ProviderEvent` — 14 variants, auth
  lifecycle only; `impl` exposes `provider()` / `attempt()`.
- `crates/fabric/src/lib.rs:360` `BuildEvent` — `SlidePlaced`,
  `EpisodeChanged`, `CardAuthored { show, episode, slide }`.
- `crates/fabric/src/lib.rs:94` `CaptureEvent` — `Started{mode,target}`,
  `Saved{rel_path,kind,name,location,source}`, `Failed`, `InboxChanged`,
  `PaletteVisibility`, `Recording{on,count}`, `Skipped{reason}`.
- `crates/chat/core/src/lib.rs:454` `UsageUpdated { summary: String }`;
  `:405` `BoundedNativePayload::redacted*` with
  `MAX_NATIVE_PAYLOAD_BYTES` — the existing redaction primitive to reuse.
- `crates/index/src/*.rs:168` — `CREATE TABLE artifacts` is the ENTIRE
  Turso schema.
- `Cargo.toml:130` `turso = "0.7"` — **pure Rust**, needs no sidecar,
  stays in-process.
- `crates/xtask/src/main.rs:684` `SYS_ALLOWED = ["dpl-lottie-render"]`
  and `:768` `LINTS_OVERRIDE_ALLOWED = ["dpl-lottie-render"]` — the gate
  enforces exactly ONE FFI leaf. A second leaf FAILS `cargo green` until
  both consts and `architecture.md` §3 are amended in the same commit.
- **Adapter-trait convention in this repo: essentially none.**
  `HistorySource` (`crates/chat/store/src/lib.rs:222`) is the only
  `pub trait *Source|Adapter|Port|Store|Backend`. This program
  ESTABLISHES the convention rather than inheriting one.
- `deny.toml` has `multiple-versions = "deny"` — a live tripwire for any
  new dep tree.
- Standing fabric exception (architecture.md §3): `dpl-fabric → dpl-hub
  + dpl-chat-core + dpl-intent`, so new domains may reuse `ProviderId`
  and `Route` without a mirrored vocabulary.
- Sidecar precedents, both real and both load-bearing:
  - `dpl-lottie-render` — `publish = false`, a `[[bin]]`, links C++
    (`thorvg-sys`), carries a ratified per-crate `unsafe_code = "allow"`,
    and already has a **`serve` mode**: *"persistent stdin-time →
    stdout-RGBA scrub stream"*. Spawned by `crates/lottie/src/lib.rs:70,140`
    via `helper_path()`.
  - `crates/video/src/lib.rs:153` — owns an ffmpeg child for the
    thread's lifetime and **respawns on failure after a backoff**. This
    is the resilience pattern the DuckDB client copies.
- DuckDB `bundled` measured on this box (§8): **1 m 35 s** cold build,
  1.47 GB peak RSS, **42 MB** artifact, `v1.5.5`, exit 0 first try.

### 2.1 Two existing laws this program collides with

**(a) The redaction law.** `docs/plans/pre-release-capacity.md:167` —
*"Turso stores declared durable application records. DuckDB analytics
consume **redacted**, versioned event projections; neither store receives
provider credentials or opaque authentication payloads."*

The ratified "prompt/response content" choice contradicts the first
clause. §3.6 resolves it: content IS stored, but **segregated, opt-in,
default-off, and credential-scrubbed**. The *credentials* half of the law
is non-negotiable and stays intact. The text needs amending knowingly in
PR 6 — not silently violated.

**(b) The unstamped-shell doctrine.** Interaction trace (clicks, drags,
keys) is exactly the shell/deck traffic the signal doctrine declares
unstamped. §3.5 resolves it by giving the trace its OWN domain at its own
volume level rather than stamping `ShellEvent` — which respects the
doctrine's intent (keep the hot navigation path free of stamping cost)
while still ordering interaction against captures. This is
doctrine-adjacent and is **flagged for ratification in PR 7**, not
assumed.

### 2.2 A prior-agent error, corrected here

Earlier plan docs generalized ADR-009 into *"C libraries are subprocesses
here."* That sentence originates in
`signal-ledger-and-lesson-foundation.md` and was then quoted forward
across `STATE.md`, `signal-ledger-phase2.md` §5, and this session's first
draft, until it read as ratified law. **It is not.** Ground truth:

- `deny.toml` denies exactly FOUR crates, all ffmpeg bindings, by name:
  `ffmpeg-next`, `ffmpeg-sys-next`, `ffmpeg-sys`, `egui-video`.
- There is **no `adr-009-*.md` file** in `docs/design/` (which holds
  adr-011, adr-013, adr-014). ADR-009 exists only as a reference in
  CLAUDE.md and `architecture.md` §4, scoped to *"external capability =
  subprocess/service behind a seam"*.
- `architecture.md` §4 already **permits** `-sys` FFI links — in one leaf.

**ffmpeg itself is not banned and never was.** It is resolved as a
managed binary at `crates/app/src/main.rs:1176` and spawned across ~20
crates (`engine-capture`, `engine-media`, `video`, `composer/import`,
`lottie-render`, `backdrop`, …). What `deny.toml` blocks is *statically
linking libavcodec into the exe* — the Rust bindings — not the tool. The
subprocess choice there is correct on its own merits: full CLI surface,
swap the binary without recompiling, a segfault on a bad file kills a
child instead of the app, and the GPL/LGPL linking question is dodged
entirely.

So the sidecar in this program is chosen on **engineering merit** (§3.7),
not because a C++ dependency is forbidden. It is not.

## 3. Target design

### 3.1 Shape

Two new crates plus three new fabric domains:

- **`crates/analytics/` → `dpl-analytics`** (ENGINE, egui-free). One
  concern: *answer questions about the signal ledger without knowing what
  answers them.* Owns the port, query vocabulary, result types, adapter
  selection, and the sidecar's client half. No FFI, no `-sys`.
- **`crates/analytics-duckdb/` → `dpl-analytics-duckdb`** (BINARY,
  `publish = false`, `[[bin]]`). One concern: *serve queries over the
  Parquet ledger.* Links `duckdb` with `bundled`. The **second FFI leaf**.

### 3.2 The AI domain — `AiEvent` (new, stamped)

Correlation id `CallId(u64)` minted per call, mirroring
`ActivationAttemptId`.

```rust
pub enum AiEvent {
    CallStarted  { call: CallId, provider: ProviderId, model: String, origin: Origin },
    FirstToken   { call: CallId, ttft_ms: u32 },
    CallFinished { call: CallId, tokens_in: u32, tokens_out: u32,
                   cost_micros: u64, duration_ms: u32, outcome: AiOutcome },
    /// The attribution payoff: did the operator keep what the AI made?
    Disposition  { call: CallId, kept: bool, target: Option<String> },
}
pub enum AiOutcome { Ok, Refused, Cancelled, Error(AiFailureClass) }
```

**`Origin` is what makes attribution work** — route + editor id +
optional workflow/subject. Without it, *"is the AI earning its keep in
the authoring loop"* is unanswerable, and that is the question actually
asked.

**Cost is derived; tokens are truth.** `cost_micros` is computed at
publish from a versioned price table (§3.3). Raw `tokens_in/out` and
`model` are ALWAYS stored so cost can be **recomputed** when the table
was wrong or prices change. Never store only the derived number.

**The bridge.** `crates/chat/` touches no fabric today, and wiring fabric
into every provider crate would breach their ceilings. Instead: providers
keep emitting `ChatEvent`s, and **one adapter in `crates/app/`**
translates chat events → `AiEvent`, exactly like today's
`publish_provider` sites (`crates/app/src/chat.rs:531…1232`).
`UsageUpdated`'s prose stays for the UI; structured numbers come from the
provider-native usage payloads (`codex:"token_count"`,
`claude:"rate_limit_event"`, `grok: Usage{used,size}`).

### 3.3 The price table

Model pricing is authored data that goes stale — it needs the document
law, not a hardcoded match arm.

- `.userdata/analytics/pricing.v1.json` — `v` (schema version), per-model
  rates, an effective date. `load` refuses newer; `save` is pretty-LF via
  tmp → bak-step-aside → rename.
- Census law: one registry function enumerates known models; a census
  test fails when a model appears in events but not the table.
- Unknown model ⇒ `cost_micros = 0` **and** the row is flagged, never
  silently priced at zero.

### 3.4 The editor domain — `EditorEvent` (new, stamped)

```rust
pub enum EditorEvent {
    Opened     { editor: EditorId, subject: String },
    Closed     { editor: EditorId, subject: String, dwell_ms: u32, committed: bool },
    Undo       { editor: EditorId },
    Redo       { editor: EditorId },
    Reopened   { editor: EditorId, subject: String },
    Ingredient { editor: EditorId, tier: IngredientTier, id: String },
}
```

- **Friction** = `dwell_ms`, `committed: false` (abandonment), `Undo`
  rate, `Reopened` count. Closest to the experience law: *"the product is
  the EXPERIENCE"* — this is where authoring hurt becomes measurable.
- **Ingredient usage** = `Ingredient` against the DS3 ladder
  (element / block / template), answering *"what should the Library
  surface, and what is dead weight"*.
- **Throughput / production** = derived from existing `BuildEvent` +
  `CaptureEvent`. No new instrumentation.

### 3.5 Interaction trace — its own domain, level-gated

Clicks/drags/keys are 2–4 orders of magnitude higher volume than
everything else. Mixed into the shared stream they drown it and make
every other query scan them.

- Own domain `InteractionEvent { kind, node, ms }` → own Parquet segment
  (`events/date=…/interaction-NNNNNN.parquet`), so a query over `ai` or
  `editor` never touches it.
- **Level gate**: `off` (default) · `summary` (counts per node per
  window) · `full` (every event). Full is for eguiPilot-driven replay
  sessions, not daily operation.
- Closes the thread recovered from session `c8bc497b`: *"a pilot-injected
  click/drag/type publishes no fabric event… the actual clicks and drags
  are invisible to the replay."* Driven events become orderable against
  the video's event track.
- Flagged for ratification against §2.1b.

### 3.6 Prompt/response content — segregated, opt-in, scrubbed

Content is big, sensitive, and shaped nothing like a metric row. It does
NOT go in the event stream.

- Content-addressed sidecar store:
  `.userdata/analytics/content/<sha256>.json`.
- The `AiEvent` row carries only `content_sha` — a join key. Events stay
  small and fast; content is fetched on demand.
- **Opt-in** via `--analytics-content` / `DPL_ANALYTICS_CONTENT=1`.
  Default OFF. The boot log NAMES whether it is on (the honesty law,
  ADR-014 §5).
- **Credential scrubbing is mandatory and not opt-out** — the
  non-negotiable half of §2.1a. Reuse `BoundedNativePayload::redacted*`;
  scrub BEFORE hashing, so a credential never reaches the content address.
- **Retention is a report, not a reaper** (ADR-014 law 3): the store
  reports size, count, and oldest age. Nothing auto-deletes operator
  material, ever.

### 3.7 The port

```rust
/// The agnostic analytics surface. Adapters translate; callers never
/// speak an engine's dialect.
pub trait AnalyticsAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn health(&self) -> Health;
    fn query(&self, q: &Query) -> Result<ResultSet, AnalyticsError>;
}
```

`Query` is a **typed enum, not a SQL string** — that is what makes the
layer genuinely agnostic rather than DuckDB-with-extra-steps. A SQL
escape hatch exists but is adapter-optional and refused by adapters that
cannot honor it:

```rust
pub enum Query {
    CountByDomain { since: Option<Millis>, until: Option<Millis> },
    CountByType   { domain: Option<Domain>, since: Option<Millis> },
    Timeline      { domain: Option<Domain>, bucket: Bucket, since: Option<Millis> },
    Recent        { domain: Option<Domain>, limit: usize },
    AiCost        { group: AiGroup, since: Option<Millis> },
    EditorFriction{ editor: Option<EditorId>, since: Option<Millis> },
    DroppedTotal,
    /// Escape hatch. `ParquetScan` returns `Unsupported`.
    RawSql(String),
}
```

### 3.8 The adapters — both ship

| adapter | transport | honors `RawSql` | role |
|---|---|---|---|
| `DuckdbSidecar` | long-lived child, NDJSON over stdin/stdout | yes | primary — pushdown, aggregation, ad-hoc SQL |
| `ParquetScan` | in-process, existing `parquet`+`arrow` deps | **no** — `Unsupported` | escape hatch + floor; always available |

Selection at boot: probe for the sidecar beside the exe; if present and
it answers `health`, use it; otherwise fall back to `ParquetScan` and
**say so in the boot log**. Analytics degrade, never fail.

Two implementations is the point. **A port with one adapter is a guess.**
The test that proves the abstraction honest is that both return identical
`ResultSet`s over the same fixture.

### 3.9 Wire protocol

NDJSON, one JSON object per line, mirroring `dpl-lottie-render`'s `serve`:

```
→ {"id":1,"q":{"CountByDomain":{"since":null,"until":null}}}
← {"id":1,"ok":{"columns":["domain","n"],"rows":[["intent",42]]}}
← {"id":2,"err":{"kind":"Unsupported","msg":"RawSql disabled"}}
```

Laws: `id` echoes for correlation · one request per line · the sidecar
never writes non-JSON to stdout (logs go to stderr) · the client applies
a per-query timeout and respawns on EOF with backoff, per the
`dpl-video:153` precedent · spawned through `dpl_proc::quiet` (W8) or the
spawn audit fails.

### 3.10 Why sidecar, on merit

Recorded so a later pass does not relitigate it, and so the reasoning is
not mistaken for the (nonexistent) C++ ban:

- **FFI blast radius stays quarantined.** `unsafe` and a C++ link live in
  a binary that cannot take the app down — the same reason
  `dpl-lottie-render` exists. A DuckDB abort kills a child.
- **The exe stays lean.** Bundled DuckDB is 42 MB measured; the
  double-clicked exe does not carry it when analytics are off.
- **Build cost is isolated.** The C++ amalgamation rebuilds only when the
  sidecar changes, not on every app build.
- **A long-lived child keeps what a CLI shell-out loses**: prepared
  statements, connection state, streaming results. Spawn cost is paid
  once at boot, not per query. *This is the distinction that makes the
  sidecar right here where a per-query `Command::new` would be wrong* —
  and it is why the ffmpeg pattern (few invocations, big files, process
  isolation as a feature) does not transfer unchanged.

### 3.11 Turso's place — records, not crunching

Ratified split. No sync path, nothing to keep consistent:

| store | holds | property |
|---|---|---|
| Turso `.userdata/index.db` | entity records (artifacts; later shows/episodes) | transactional, small, precious |
| Parquet `.userdata/analytics/` | the append-only event stream | **file truth** |
| DuckDB sidecar | a lens over Parquet | disposable — deleting it loses nothing |

Rejected this session: **rollups written back into Turso** (adds a sync
path and a staleness question) and **DuckDB `ATTACH`-ing the SQLite file**
(most powerful, but couples the lens to the record store's schema). A
`TursoAdapter` behind the port stays possible; not built this program.

## 4. The roadmap (execution order)

Each row is one landable PR. Instrumentation first, per ratification.

| # | PR | Delivers | Gate / law risk |
|---|---|---|---|
| 1 | `feat(fabric): the AI usage domain` | `AiEvent`, `CallId`, `Origin`, `AiOutcome`; app-side chat→AI adapter; ledger records it | fabric ceiling + architecture.md §3 |
| 2 | `feat(analytics): the versioned price table` | `pricing.v1.json` (document law), cost derivation + recompute | census law (new authored data) |
| 3 | `feat(fabric): the editor domain` | `EditorEvent`; wired at the authoring/overlay editor first | — |
| 4 | `feat(analytics): the port + ParquetScan` | `dpl-analytics`, typed `Query`, `ResultSet`, adapter selection | +1 egui-free member |
| 5 | `feat(analytics): the DuckDB sidecar` | `dpl-analytics-duckdb`, NDJSON `serve`, `DuckdbSidecar` client | **2nd FFI leaf** — `SYS_ALLOWED`, `LINTS_OVERRIDE_ALLOWED`, architecture.md §1/§3, SAME commit |
| 6 | `feat(analytics): content capture (opt-in)` | content store, scrubbing, `--analytics-content` | **redaction law §2.1a** — amend knowingly |
| 7 | `feat(fabric): interaction trace (level-gated)` | own domain, own segment, off by default | **doctrine §2.1b** — needs ratification |
| 8 | `docs(adr): ADR-015 the analytics port` | port, two-store split, sidecar rationale, both amendments | — |

PRs 1–4 are pure instrumentation + pure Rust: no new C++, no doctrine
amendment. PR 5 is the only one a hostile Windows build can block, and by
then everything else is already shipping. PRs 6 and 7 each carry exactly
one law amendment.

**Production stats need no PR** — they are queries over `BuildEvent` +
`CaptureEvent`, available the moment PR 4 lands.

Harness flags added (census law — amend `crates/app/src/main.rs` +
CLAUDE.md's flag list + the xtask census in the SAME commit):
`--analytics <auto|duckdb|parquet>` (PR 4/5) · `--analytics-content`
(PR 6) · `--interaction <off|summary|full>` (PR 7).

## 5. Boundaries

- No ingest. DuckDB reads Parquet **in place**; it never becomes truth,
  and nothing reads from it that cannot be recomputed.
- The sidecar is never a hard dependency — missing binary = degraded
  mode, reported in the boot log, not an error.
- No writes to `/content`; no second artifact index.
- **No UI surface this program.** Query results have no route yet;
  surfacing rides the experience gate as its own pass.
- No Turso adapter; no rollup materialization (ratified).
- Credential / auth payloads never enter either store — non-negotiable.
- Interaction trace defaults OFF; content capture defaults OFF.
- No classification logic — the ledger's `class` column stays reserved.

## 6. Verification

- **Unit**: `Query` → SQL lowering pinned by test · NDJSON codec
  round-trip · **both adapters return identical `ResultSet`s** over one
  fixture (the test that proves the port is agnostic) · cost recomputed
  from stored tokens reproduces the stored `cost_micros` · unknown model
  flags rather than silently pricing zero · the scrubber drops a seeded
  fake credential BEFORE hashing · interaction level `off` writes zero
  rows · a malformed sidecar line is an error, not a hang.
- **Real artifact**: `--ledger --analytics duckdb --screenshot` then
  `CountByDomain` returns the Phase-2 `intent`/`determinismChanged` row;
  an authored card produces a joinable `ai` + `editor` + `build` trio.
- **The product question — the acceptance test for the whole program**:
  *"for episode X, what did the AI cost, which ingredients were used,
  where did authoring stall, and was the AI output kept?"* One query
  joining ai/editor/build. If that cannot be answered, this is not done.
- **Gate**: `cargo green`; structure audit +2 egui-free members and two
  FFI leaves; `cargo deny` clean (watch `multiple-versions`).
- **Both CI lanes.** The Windows MSVC 14.44 build of the sidecar is a
  PR-5 precondition and is UNMEASURED today — 1 m 35 s on ARM says
  nothing about the lane that already carries LNK1318 memory cures.
- **The real exe**: per the one-checkout law, a pass is not done until
  `target/release/devpulselabs.exe` is rebuilt and its mtime moves.

## 7. Commit plan

Eight PRs per §4, in order, each `gh pr create` → merge (no direct
pushes). PRs 5–7 each amend exactly one law, made explicitly in the same
commit as the code, per the architecture.md change protocol.

## 8. Evidence

Ground truth §2 read from the working tree this session:
`crates/fabric/src/lib.rs` (domains; `CaptureEvent` :94, `ProviderEvent`
:248, `BuildEvent` :360), `crates/chat/core/src/lib.rs:405,454`,
`crates/chat/store/src/lib.rs:222`, `crates/index/src/*.rs:168`,
`crates/lottie-render/Cargo.toml`, `crates/lottie/src/lib.rs:70,140`,
`crates/video/src/lib.rs:153`, `crates/app/src/main.rs:1176`,
`crates/app/src/chat.rs:531–1232`, `crates/xtask/src/main.rs:684,768`,
`Cargo.toml:130`, `deny.toml`,
`docs/plans/pre-release-capacity.md:167`, `docs/design/architecture.md`
§1–§6. Verified by search that `crates/chat/` publishes to the fabric
nowhere, that `EditorEvent` does not exist, and that `docs/design/` holds
no `adr-009-*.md`.

### 8.1 DuckDB `bundled` build — measured, not assumed

Throwaway crate, `duckdb = { version = "1", features = ["bundled"] }`,
against the repo's pinned `rust-toolchain.toml` (1.92.0, minimal), on
this aarch64 box, 2026-07-29:

| metric | measured |
|---|---|
| cold `cargo build --release` | **1 m 35 s** wall |
| peak RSS during build | **1.47 GB** |
| artifact (unstripped) | **42 MB** |
| target | `ELF 64-bit … ARM aarch64` — native, no cross-compile |
| runtime check | `SELECT version()` → **`v1.5.5`** |
| crate resolved | `duckdb v1.10505.0`, single direct dep |

Exit 0, first try, no build flags, no system deps, no patching. An
earlier caution in this session about "multi-minute C++ builds and ARM64
build risk" was **overstated** — recorded here so it is not inherited as
fear by a later pass. The 42 MB is the real cost, and §3.10 is where it
is paid down.

### 8.2 Session recovery (the thread this program came from)

The DuckDB thread lives in session **`c8bc497b`** (2026-07-25 → 07-26,
498 entries, cwd `/home/hades/devpulselabs`). The framing turn:

> *"My signal pattern properly implemented… I am supposed to have a clear
> signal emitted on actions clearly defined for **duckdb analytics and
> baseline optimizing**."*

That session also carried three threads NOT addressed by this program,
recorded so they are not lost again: the **pixvix guided-save gap**
(stills/video/session bypass the name+tag+episode card that only steps
gets), the **footer fluid-clamp**, and **eguiPilot-driven replay**
(partially closed by §3.5).

**"Baseline optimizing" was never defined** — not in that session, not in
any doc. It remains OPEN. A candidate meaning, neither ratified nor
built: a recorded p50/p95/max summary of per-domain event latencies and
job durations over a named window, stored as its own small Parquet, so
*"is it slower than it was?"* becomes a query rather than a feeling. It
is deliberately outside §4 until defined.

**Why the sessions appeared lost.** Claude Code keys history by working
directory; the repo has lived at four paths, splitting history four ways:

| cwd key | sessions | span |
|---|---|---|
| `/home/hades/Projects/devpulse_labs` | 5 | 07-26 → 07-29 |
| `/home/hades/devpulselabs` | 2 | 07-22, 07-25 → 26 |
| `/home/hades/Projects/devpulselabs` (current) | 2 | 07-29 |
| `/home/hades/Projects/DevPULSE` | 1 | 07-21 |

`--resume` in the current folder showed only the last two. All seven
devpulselabs-lineage transcripts were **copied** (originals intact) into
`~/.claude/projects/-home-hades-Projects-devpulselabs/`, so the full
history is now resumable from this checkout. **A folder rename splits
session history** — worth knowing before the next move.

### 8.3 Worktree inventory (ops, not this program)

The "5 PRs in this folder" are five sibling git **worktrees**, all clean
(no uncommitted work), not PRs:

| folder | branch | PR | unmerged commits |
|---|---|---|---|
| `devpulselabs-charter` | docs/authoring-charter | #119 merged | 0 |
| `devpulselabs-authoring-a` | feat/authoring-a-signals | #124 merged | 2 |
| `devpulselabs-authoring-b` | feat/authoring-b-gallery | #128 closed unmerged | 1 |
| `devpulselabs-authoring-c` | feat/authoring-c-editor | #130 closed unmerged | 2 |
| `devpulselabs-ds2` | feat/ds2-element-library | — | 2 |

Actual open PRs: **3** — #141 (overlay editor, salvaged from #130) plus
two dependabot bumps (#84 sysinfo, #80 fjall). Cleanup is the operator's
call and is not part of this program.
