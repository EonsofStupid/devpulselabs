# Pass — Coach Seam S1: the tracing spine

**Status:** LANDED 2026-08-08 · first slice of the ratified seam ADR
(`coach-seam-adr.md` §8). S1 lands FIRST because every later slice must
arrive already instrumented — the verb battery that proved 25 verbs and
captured zero reliability stats can never recur.

## 1. Context

The seam ADR (RATIFIED 2026-08-07) rules: every verb execution emits a
span; the span rides a new `seam` fabric domain; `dpl-ledger` (already
built) writes it to Hive-partitioned Parquet; reliability-per-verb becomes
a query. Today verbs execute in ONE place: `dpl-coach-daemon`'s `/verb`
funnel (plus `/import`, `/resolve`, `/invoke`), calling typed
`dpl-coach-runtime` functions that return effect contracts.

## 2. Goal / non-goals

**Goal:** a daemon run against a real app leaves `seam-*.parquet` rows —
one per verb execution or refusal — carrying verb, app, transport rung,
mechanism, ok, effect_observed, duration, and the open attrs map; and the
question "reliability per verb" is answered from those rows.

**Non-goals (later slices, by ruling):** protocol rehoming (S2) · profile
registry (S3) · Foundry/Blender (S4) · Optimize cards (S5) · the
DataFusion dependency (ADR §10 amendment: waits for a DataFusion release
on arrow 59; the in-tree pure-Rust reader is the S1 lens) · spans for the
`/events` SSE geometry ticks (observation, not execution) · in-app
publishers (no in-app verb executor exists yet).

## 3. Ground truth (read in full, 2026-08-07)

- `crates/fabric/src/lib.rs` — per-domain typed enums, `Stamped<T>`
  (`seq`/`ms`, one global counter), `publish_*`/`subscribe_*`, serde-free
  (doctrine law 4), mirrors protocol types where needed (`CoachActor`
  precedent).
- `crates/ledger/src/lib.rs` — `LedgerTap` subscribes every domain at
  fabric birth, `drain()` polls, `row_*` projections build
  `LedgerRow { seq, ms, wall_ms, domain, type, body, class }`, writer
  thread flushes `<domain>-NNNNNN.parquet` under `events/date=…/`.
  `read_parquet_rows` is the in-tree proof reader.
- `crates/coach-runtime/src/lib.rs` — per-verb functions returning typed
  effects (`WindowVerbEffect`, `LifecycleEffect`, `TextTypeEffect`,
  `KeyPressEffect`, `SelectEffect`, `ScrollEffect` (carries `via`),
  `WaitAssertEffect`, `InvokeEffect`) or `Refused`.
- `crates/coach-daemon/src/main.rs` — the `/verb` match is the single
  execution funnel; daemon is its own PROCESS (its spans cannot ride the
  app's in-process fabric — it owns a fabric + tap of its own; the
  Parquet files are the meeting point).
- App analytics root: `dpl_engine_capture::userdata_root().join("analytics")`.
  The daemon does NOT take an `engine-capture` edge for one path fn —
  it resolves its root by flag/env (§5) and logs it loudly.
- xtask structure audit: no dep ceiling on `coach-daemon`/`dpl-fabric`;
  `dpl-ledger` stays in the egui-free census (unchanged). New daemon
  edges are documented in `architecture.md` in the same commit.
- DataFusion 54.1 (stable) pins arrow 58; ledger is arrow/parquet 59.
  Named in the ADR §10 amendment — no second arrow for S1.

## 4. Design — the `seam` domain (dpl-fabric)

```rust
pub enum TransportRung { R0Native, R1Semantic, R2Injection, R3Vision } // + label()
pub enum SeamAgency { Coach, Teacher }                                 // + as_str()

/// One verb execution — the span (schema v1, ADR §7.5: open attrs beside
/// the typed core; lesson/beat present from day one, None until lessons).
pub struct SeamSpan {
    pub verb: String,           // ratified wire name ("type_text", …)
    pub app: String,            // atspi/app identity as addressed
    pub rung: TransportRung,
    pub downgraded_from: Option<TransportRung>,
    pub mechanism: String,      // honest means: "atspi-action" | "xdotool" | "x11-wm" | "spawn" | "wheel" | …
    pub agency: SeamAgency,
    pub ok: bool,               // the call returned (refusals are the other variant)
    pub effect_observed: bool,
    pub duration_ms: u64,
    pub lesson: Option<String>,
    pub beat: Option<u32>,
    pub attrs: Vec<(String, String)>,
}

pub enum SeamEvent {
    Executed(SeamSpan),
    Refused { verb: String, app: String, reason: String, duration_ms: u64 },
}
```

`publish_seam` / `subscribe_seam` / `SeamReceiver`, stamped like every
domain. Channel sized 128 (verb cadence ≈ provider lane, not capture).

**Rung classification v1** (per implementation; the per-verb ladder
resolution proper is S3's profile registry):

| Verbs | Rung | Mechanism |
|---|---|---|
| invoke/click, select, scroll via AT-SPI | R1Semantic | atspi-action |
| type_text, press_key, scroll via wheel | R2Injection | xdotool / wheel |
| move, resize, focus, geom | R2Injection | x11-wm |
| open, close | R2Injection | spawn / wm-delete |
| import, resolve, presence, wait_until, assert_state | R1Semantic | atspi-read |

`scroll` picks its rung from the effect's `via` field at runtime — the
first live downgrade-visible verb.

## 5. Ledger + daemon wiring

- `dpl-ledger`: `LedgerTap` subscribes `seam`; `row_seam` projects
  `type_name = verb wire name` (so reliability is GROUP BY `type`) and a
  body of `{v:1, app, rung, downgraded_from, mechanism, agency, ok,
  effect_observed, duration_ms, lesson, beat, attrs, refused?, reason?}`.
  Refusals share the verb's `type` with `"refused":true` — one group
  carries its failures beside its successes.
- `dpl-coach-daemon`: owns a `dpl_fabric::Fabric` + `LedgerTap`.
  Analytics root: `--analytics-root <path>` → env `DPL_ANALYTICS_ROOT` →
  env `USERDATA_ROOT` + `/analytics` → `./.userdata/analytics`; resolved
  path logged at boot (honesty law — silent non-recording is how weeks of
  stats were lost). A tokio task drains the tap every 250 ms; flush+join
  on graceful shutdown. Every `/verb` arm, `/import`, `/resolve`,
  `/invoke` is timed and publishes `Executed` or `Refused`.
- `architecture.md`: coach-daemon entry amended — may depend on
  `dpl-fabric` + `dpl-ledger` (span emission), still never a dep of app.

## 6. Proof

1. Unit: fabric seam round-trip + stamp ordering; ledger seam row lands in
   its own `seam-*.parquet` segment with `type` = verb and v1 body;
   refusal row carries `"refused":true`; reliability aggregation
   (per-verb n / effect-ok / refused counts) computed over
   `read_parquet_rows` output.
2. Live (this seat): build daemon, run with an explicit analytics root,
   drive gnome-calculator through `open → focus → move → type_text →
   assert_state → close` via `/verb` + one refused verb; stop the daemon;
   read the `seam-*.parquet` segments and print the reliability table.
3. `cargo green` (the full gate) before push.

## 7. Risks

- **Two processes, two sequence spaces** — daemon `seq` is daemon-local;
  cross-process ordering joins on `wall_ms` (already true for every
  multi-process future; recorded honestly, not papered over).
- **Rung table is v1 static** — per-verb ladder resolution belongs to the
  profile registry (S3); the `mechanism` string keeps the row honest
  meanwhile.
- **X11/AT-SPI live proof needs this seat** — unit tests carry the gate;
  the live drill is run here, not in CI.

## 8. Evidence (2026-08-08)

- **Unit:** `dpl-fabric` 23 green (incl.
  `seam_spans_stamp_in_total_order_and_refusals_are_facts`); `dpl-ledger`
  8 green (incl. `seam_spans_land_in_their_own_segment_grouped_by_verb`
  — segment isolation, `type`=verb, v1 body, refusal fact, attrs map,
  and the `seam_reliability` fold asserted over the rows on disk).
- **Live (this seat, DISPLAY=:1):** debug daemon on port 7855 with
  `--analytics-root /tmp/seam-s1-proof`; drove gnome-calculator through
  `open → focus → move → type_text("7") → assert_state(display 7) →
  scroll → drag(refused) → close`; SIGINT flushed 8 `seam-*.parquet`
  segments under `events/date=2026-08-08/`.
- **Lens over real rows** (`cargo run -p dpl-ledger --example
  seam_reliability -- /tmp/seam-s1-proof`): 8 seam rows; every executed
  verb 100 % effect rate except scroll (0 % — Calculator's scroll pane
  exposes no offset; delivery honest); `drag` counted as 1 refusal.
- **The downgrade is data:** the scroll row landed
  `"rung":"r2","downgraded_from":"r1","mechanism":"xdotool_wheel"` — the
  first live ladder fall-through, visible in Parquet, no code read
  required.
- `dump_rows` example added beside `seam_reliability` as the eyeball
  drill.
