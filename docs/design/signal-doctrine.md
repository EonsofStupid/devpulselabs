# THE SIGNAL DOCTRINE — per-domain typed signals, emit-with-mutation

Status: LOCKED (operator, 2026-07-26: "properly establish the signal
pattern and lock it in"). This document is LAW for every signal in the
app, present and future. A pass that adds a mutation without a signal,
or a signal outside this shape, is wrong by definition.

## Where this comes from (the two sources, both binding)

**The ORIGINAL** (`D:/localdev/devpulse labs`, READ-ONLY reference —
the operator's "signalkind"):

> "Typed capture signals — ONE channel (`dpl:capture`), one payload
> enum. Every capture vehicle emits these; the UI holds exactly one
> subscription." — src-tauri/src/commands/events.rs

> "Write the inbox row into the artifact index + emit `Saved`. ONE
> place so every PixVix path indexes + signals identically."
> — src-tauri/src/commands/capture.rs (`index_saved`)

> Payloads are schema-validated at the consumer; "a malformed payload
> is dropped with a console.warn (never throws)."
> — src/domains/native/services/events.native.ts

Original taxonomy (serde `tag = "type"`, camelCase — the wire shape):
`started{mode,target}` · `saved{file,kind,name,destination,source}` ·
`failed{error}` · `inboxChanged` · `paletteVisibility{visible}`.
Documented integration point: the AI labeler subscribes to `saved`.

**THE PORT'S OWN ARCHITECTURE** (crates/fabric/src/lib.rs, P0):

> "Per-domain enums on per-domain channels (a flat global enum would
> wake every consumer on every message). P0 ships the shell domain as
> the working exemplar. Engine domains (capture, jobs, providers) land
> with their ports and follow this file's shape exactly."

The capture domain was prescribed at P0 and never landed. This
doctrine completes it.

## The laws

1. **Per-domain enums on per-domain broadcast channels.** One Rust
   enum per domain, one `tokio::sync::broadcast` channel per domain,
   `publish_X`/`subscribe_X` on `Fabric`. Domains today: `shell`,
   `deck`, `capture` (this pass). Next domains follow the same shape:
   `jobs` (AE orchestrator/render progress — mpsc where delivery must
   be guaranteed, per the fabric's own note), `providers` (Chat activation
   and provider lifecycle facts).
   A flat global signal enum is FORBIDDEN.
   **Scale story: a new domain = one enum + one channel + two
   methods; a new signal = one enum variant; rustc enforces every
   producer and consumer at compile time. No registry, no stringly
   event names, no drift.**

2. **The capture taxonomy is the original's DNA; the field
   vocabulary is OUR index's.** Pre-release freedom (operator,
   2026-07-26): no legacy wire-compat debt — the original's dead
   webview consumer binds nothing. The KINDS carry over faithfully:
   `CaptureEvent { Started{mode,target} · Saved{rel_path,kind,name,
   location,source} · Failed{error} · InboxChanged ·
   PaletteVisibility{visible} · Recording{on,count} · Skipped{reason} }`
   — the last two are the ADDITIVE recorder-facts extension (2026-07-26):
   PORT ran the steps/session recorder over cross-window signals
   (`dpl:steps-state`/`-skip`/`-cmd`, `dpl:session-state`/`-cmd`); the
   faithful single-process translation keeps the toggle COMMAND a direct
   call (`Viziwiz::toggle_steps`) — stronger than a broadcast — and puts
   only the FACTS on the fabric, so the activity feed, status counter and
   `/signals` see a recording live and `skipped · <reason>` never reads as
   a frozen counter. Saved's fields align with
   `dpl_index::Record` (rel_path/kind/name/location/source), because
   OUR index is the truth signals report on. Extensions are ADDITIVE
   variants; never renames, never kind-reuse.

3. **Emit-with-mutation, one helper.** Every artifact mutation
   (capture save, import, render, export, keep, dump, ingest,
   flatten) goes through ONE index-and-signal seam per layer — the
   original's `index_saved` law. A mutation that does not signal is a
   bug; the signal fires AFTER the write succeeds (signals report
   truth, never intent). `Failed{error}` fires where the mutation
   fails.

4. **The fabric stays serde-free; ONE ENVELOPE at every boundary.**
   The enums are plain Rust in `dpl-fabric` (no serde dep — R1v law).
   Every external surface serializes the SAME envelope, hand-mapped
   and PINNED BY TEST:
   `{"domain":"capture","ms":<t>,"type":"saved", …camelCase payload}`
   — the seam's `/signals` SSE, the session recorder's `events.jsonl`
   (crash-safe, line-flushed), and any future consumer all speak this
   one stream shape. THE ENVELOPE IS THE CONVERGENCE SEAM: when this
   app merges into the one platform, its signal stream is already a
   uniform, versionable wire — new domains join the envelope without
   consumers changing shape. Remote consumers validate at their edge
   and drop malformed payloads with a warning, never a throw (the
   original's law).

5. **STAMP AT PUBLISH (ratified 2026-07-26).** The fabric wraps every
   signal as `Stamped<T> { seq, ms, ev }` at publish: `seq` from ONE
   global AtomicU64 across ALL domains (total causal order — replay
   and analytics can interleave domains truthfully), `ms` monotonic
   from the fabric's creation instant. The signal log is thereby a
   FLIGHT RECORDER: session replay reconstructs exact order; the
   platform bus inherits ordered streams. Boundaries add wall-clock
   when they need it; in-process ordering truth is seq.

6. **The WAKER law.** The fabric holds an optional waker callback
   (`Arc<dyn Fn() + Send + Sync>`, installed by the app as a
   `request_repaint` closure — the fabric stays UI-free). Every
   publish invokes it: background signals surface the same frame,
   no polling burn, no idle latency. Immediate-mode native.

7. **The TRAFFIC-CLASS law (the hybrid, ratified).** Four classes,
   four mechanisms — forcing one is forbidden:
   - FACTS (state re-readable; loss OK) → stamped lossy broadcast;
   - PROGRESS (high-frequency, latest-wins) → shared polled state
     (the RenderState idiom); signals fire ONLY at lifecycle
     transitions;
   - COMMANDS (must not be lost) → guaranteed mpsc with an ack
     shape. CONTRACT ONLY until R2c ships a real consumer — the
     doctrine reserves the seam; nothing speculative ships;
   - STREAMS (frames, SSE) → dedicated pipes, never the fabric.

8. **The BATCHING law.** High-volume operations (pack ingest, bulk
   import, reindex) NEVER emit per-item facts — they speak jobs-
   domain progress plus ONE `InboxChanged` at completion. A signal
   storm is a bug, not throughput.

9. **Signals are lossy broadcast; consumers drain per frame.**
   Broadcast drops oldest under pressure and tells the receiver —
   correct for signals, WRONG for jobs (jobs get mpsc). egui
   consumers hold a `Receiver`, `try_recv` in their frame, and treat
   any capture signal as cache invalidation (Saved/InboxChanged →
   reload lists lazily). Refresh chips remain as manual overrides but
   are never load-bearing. Publishers may sit on ANY thread (the
   `Sender` is Send+Sync); UI wake-up is the consumer's concern
   (poll-with-repaint where an off-thread signal must surface while
   idle — the render-pill pattern).

6. **The JOBS domain is established NOW (contract), migrated
   incrementally.** Long-running work (stitch render, pack ingest,
   guide export, AE orchestration, future gen backends) speaks
   `JobEvent { Queued{job,label} · Progress{job,stage} ·
   Done{job,out} · Failed{job,error} }` keyed by a `JobId`. Delivery
   for job COMMANDS must be guaranteed (mpsc, per the fabric's own
   note); job STATUS may broadcast. This pass adds the channel and
   emits ADDITIVELY from the three existing polled states
   (RenderState / IngestState / ExportState — three bespoke copies of
   the same polling pattern today); collapsing those states onto the
   jobs domain is its own later pass. QoL payoff now: chat/agents see
   render/ingest/export progress LIVE over `/signals`, and the status
   bar can show one honest "working…" truth instead of per-surface
   pills.

10. **The COACH domain (established 2026-07-28, Phase 1.5 of
   `docs/plans/signal-ledger-and-lesson-foundation.md`).** Touchpoint
   lifecycle facts from the operator coach — `CoachEvent { Armed ·
   Mounted · Triggered{trigger,actor} · Satisfied · Unmounted{reason} }`
   — stamped like every other fact domain. The PUBLISHER is the
   devtools-gated pilot bridge (`crates/app/src/coach_fabric.rs`, a tap
   on the plugin's event record path — a witness, never a consumer);
   the domain itself is unconditional so the activity feed, the seam
   envelope, and the future ledger compile once. `Refused` never
   publishes: a refusal is a command result on the pipe, not a
   lifecycle fact.    ANTI-LOOP LAW: the coachable-signal catalog
   (`coach_signals.rs`) never subscribes to this domain — a touchpoint
   waiting on a coach event would self-trigger.

11. **The INTENT domain (established 2026-07-28,
   `docs/plans/determinism-intent-gate.md`).** Privileged-mode transitions
   and their refusals — `IntentEvent { DeterminismChanged{mode,cause} ·
   IntentRefused{action,reason} }` — stamped like every other fact domain.
   The PUBLISHERS are the two greppable mint sites: the chat composer's
   slash path (`OperatorIntent::slash_command`) and the shell inspector's
   DETERMINISM card (`OperatorIntent::ui_acknowledged`), plus the harness
   evidence force (policy, no proof). ANTI-LOOP / ANTI-NARRATION: the
   coachable-signal catalog never forwards this domain — mode changes are
   not lesson material, and nothing that flows toward an agent should
   narrate the escalation surface.

## Enforcement

- Wire-shape tests in `crates/seam` assert the exact JSON per kind.
- The emit helper is the only index write-through path the UI layers
  call; new mutation sites route through it or fail review.
- This document is referenced from doctrine.md; the `coldlight-system`
  gate's per-pass checklist includes "mutations signal per
  signal-doctrine.md".

## Consumers (current map)

- Gallery / Library shelf / Sessions / FOOTAGE palette / pickers:
  invalidate on Saved + InboxChanged.
- Session recorder: logs every capture signal to events.jsonl.
- Seam `/signals` SSE: forwards all kinds — agents see saves live
  (the original's AI-labeler integration point, honored; the labeler
  itself is a later, ask-first pass).
- Chat provider activation: UI intents and provider commands remain direct or
  guaranteed; successful lifecycle transitions publish redacted stamped
  `ProviderEvent` facts. Full device codes, URLs, tokens, cookies, provider
  output, and message content are forbidden from the Fabric envelope.
- The status grid's signal counter: counts capture signals too — the
  operator SEES the fabric breathing.
- THE ACTIVITY FEED (ratified, ships with the signal pass): a
  ring-buffer subscriber (cap 256, all domains, stamped order)
  rendered live in the shell inspector's SIGNALS card — every save,
  import, render, job, navigation with its ms. The observability QoL
  surface AND the debug tool when something doesn't fire.
