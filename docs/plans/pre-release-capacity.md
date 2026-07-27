# DevPULSE pre-release capacity and scale plan

Status: BASELINE CONTRACT — measurements pending ratification

DevPULSE is expected to at least double its routes, Chat capabilities,
provider adapters, interaction surfaces, and native windows. The pre-release
goal is therefore not to make one current screenshot fast. It is to establish
boring, measurable operating limits that remain valid as features are added.

This plan separates three concerns that must never be traded against one
another:

1. **Developer iteration:** clean/warm compile, link, test, and local-gate cost.
2. **Shipped runtime:** startup, frame time, repaint behavior, memory, process
   lifecycle, persistence, and multi-window rendering.
3. **Validation cost:** deterministic unit, harness, real-process, visual, and
   full-workspace evidence without using CI as the first debugger.

No dependency, language, feature, test, or renderer may be removed merely to
make a number look better. Optimization follows measured ownership and keeps
the ratified product contract intact.

## 1. Capacity laws

- There is one build coordinator per checkout. Competing Cargo invocations are
  unsupported because they multiply memory pressure, linker work, cache
  contention, and diagnostic noise.
- All queues, histories, caches, attachments, images, traces, logs, and retry
  loops have explicit bounds and visible overflow behavior.
- Background work publishes a typed fabric fact and wakes the owning viewport.
  Stable hidden, minimized, or occluded windows do not poll.
- Perpetual repaint is allowed only for a registered visible animation, media
  surface, capture, or test clock. Every such owner declares its cadence and
  stop condition.
- Provider discovery, authentication, ACP startup, history loading, indexing,
  and artifact decoding never block an egui pass.
- Provider processes are lazy, generation-scoped, bounded, observable, and
  reaped. DevPULSE does not launch every installed provider at startup.
- Only the active visible transcript is fully projected into egui. Long
  histories are paged and virtualized, and streaming updates do not rebuild
  unaffected history.
- Main, Chat, and utility windows share authored resources where the platform
  permits, but have independent repaint and lifecycle policies.
- A foreign-language or native component is introduced only as an isolated
  process/leaf adapter after a measured Rust implementation fails a ratified
  budget. Language mixing is not a substitute for profiling.
- CI repeats locally qualified evidence. It is not the first place a known
  platform, linker, fixture, or protocol failure is discovered.

## 2. Measurement matrix

Every recorded result includes:

- Git revision and dirty-state marker;
- Rust toolchain, target, linker, profile, and enabled features;
- OS/build, CPU, logical cores, physical RAM, GPU/driver, renderer/backend;
- cold or warm cache;
- fixture/profile and native viewport count;
- sample duration and percentile method.

The initial reference machines are:

1. the operator's 16 GiB Windows laptop using MSVC ABI plus the
   Rust-toolchain-shipped `rust-lld.exe`;
2. the pinned Lavapipe Linux CI image;
3. a hardware-rendered Windows qualification profile.

Hardware classes keep separate records. A faster machine may qualify a release
but may not erase a regression on the reference laptop.

## 3. Provisional budgets

These are starting thresholds, not fabricated evidence. The baseline pass
records current behavior, then the thresholds are ratified or adjusted with a
written reason before they become release gates.

### 3.1 Developer iteration

| Measure | Provisional target | Gate behavior |
|---|---:|---|
| Concurrent Cargo coordinators | exactly 1 | hard failure |
| Warm focused `cargo check` for one changed leaf | <= 15 s | trend, then gate |
| Warm focused crate test suite | <= 30 s | trend, then gate |
| Warm affected-slice test suite | <= 90 s | trend, then gate |
| Peak local build memory | retain >= 2 GiB available RAM and avoid paging | hard investigation |
| Full local green gate | recorded per revision | required before push |

Clean builds, first builds after a toolchain/profile/linker change, and warm
incremental builds are recorded separately. The first `rust-lld` fingerprint
rebuild is not compared with a warm run.

The workspace keeps the current profile intent:

- application crates remain fast to iterate in dev;
- heavy dependencies compile once at the existing optimized dependency level;
- tests keep full behavior and omit test debuginfo only;
- release keeps ThinLTO;
- Windows uses the qualified `rust-lld` linker without changing the MSVC ABI,
  system libraries, crate features, or test graph.

#### Initial local observations

These observations are not yet release thresholds. They are the first
reference-laptop samples from the Grok persisted-auth pass and establish what
the next instrumented baseline must explain:

| Operation | Wall time | Useful detail |
|---|---:|---|
| Affected four-crate test slice | 69.2 s | compilation 51.6 s; 64 unit tests plus doc tests passed |
| Strict affected-app clippy slice | 29.5 s | warning-free across the application dependency surface |
| Warm devtools application build | 23.3 s | after the linker/process-boundary correction |
| Real-process persisted-Grok scenario | 14.4 s | six native peer-window steps; interaction time was 224 ms |

The first conclusion is deliberately narrow: current developer latency is
dominated by dependency compilation, linking, and real-process startup rather
than test execution or egui interaction. CAP-0 must capture cold versus warm
fingerprints and peak memory before changing profiles, crate boundaries, or
languages.

If iteration misses its budget, investigate in this order:

1. accidental concurrent builds and stale processes;
2. invalidated build fingerprints and unnecessary feature changes;
3. oversized test-binary dependency closures;
4. proc-macro/code-generation cost;
5. crate boundaries that cause broad recompilation;
6. linker peak memory and symbol payload;
7. only then, additional tooling or profile changes.

Crates remain domain-owned and hyper-modular, but a new crate must buy a real
dependency or ownership boundary. Fragmentation that adds compile/link cost
without isolation is not modularity.

### 3.2 Shipped application

| Measure | Provisional target | Gate behavior |
|---|---:|---|
| Warm launch to usable Main shell | <= 2 s | release trend |
| Warm open/focus of Chat | <= 500 ms | release trend |
| Stable visible idle CPU per window | <= 1% reference CPU | hard investigation |
| Minimized/hidden idle CPU | statistically near process baseline | hard investigation |
| Active egui CPU frame time | p95 <= 8 ms, p99 <= 12 ms | release gate |
| Frames over 16.67 ms during authored interaction | <= 1% | release gate |
| Local provider event to visible update | p95 <= 50 ms | release gate |
| Stable-frame memory growth | < 5 MiB over 10,000 frames after warm-up | release gate |
| Provider/process leak | zero after 100 start/stop cycles | release gate |
| Queue occupancy | < 75% sustained; zero silent loss | hard investigation |
| Default pruned agent snapshot | <= 12 KiB | existing hard gate |

Animation and video scenarios are reported separately from idle and ordinary
interaction. An ambient visual is not evidence that unrelated windows may
repaint continuously.

### 3.3 Data and retention

- Conversation/event history is append-oriented, bounded in memory, and paged
  from durable storage.
- A 50,000-event conversation remains scrollable without constructing 50,000
  egui rows in one pass.
- Provider payloads are normalized once. Views consume stable IDs and narrow
  projections rather than cloning whole provider/session maps every frame.
- Large artifacts are content-addressed. Metadata lives separately from image,
  video, trace, and diff bytes.
- Thumbnail decode/upload has a per-frame work budget and an LRU byte budget.
- Full-resolution media is released when no visible surface, active job, or
  retained artifact owns it.
- Turso stores declared durable application records. DuckDB analytics consume
  redacted, versioned event projections; neither store receives provider
  credentials or opaque authentication payloads.
- Provider credentials remain provider-owned in the OS user profile. Portable
  application binaries and `dptools-deps` do not imply portable secrets.

Each cache publishes current entries, estimated bytes, hits, misses, evictions,
and oldest age. “It is cached” is not a capacity policy.

## 4. Runtime architecture under doubled capability

### 4.1 Fabric and state fan-out

The typed fabric remains the state-transition authority. Scale requires:

- small facts carrying stable IDs, revisions, and provenance;
- payload bytes in bounded stores referenced by ID;
- latest-value/watch semantics for replaceable progress;
- ordered queues only for durable transitions;
- per-domain subscriptions so an inactive hub does not process unrelated
  provider/media traffic;
- sequence/generation checks that reject stale worker and window results;
- queue-depth, rejected-publish, lag, and consumer-time metrics.

No UI reads raw process output, database rows, or browser events directly.
Adapters translate them once into redacted typed facts.

### 4.2 Provider supervisor

Each activated provider profile has one supervisor generation:

`Unavailable -> SignedOut -> Authorizing -> Verifying -> Ready -> Recovering`

The supervisor owns:

- executable qualification and fixed launch specification;
- provider-owned authentication discovery;
- one ACP child and bounded command/event queues;
- interruptible startup and nonblocking shutdown;
- bounded retry/backoff with typed diagnostic facts;
- session paging and active-turn cancellation;
- crash/disconnect recovery without duplicate children;
- redacted stderr classification and timing metrics.

Installed but inactive providers are metadata, not live processes. Provider
probes are lazy or staggered. Startup never becomes `N providers × timeout`.

### 4.3 egui and native windows

Every viewport declares one of four repaint classes:

- `Dormant`: wake only on input, lifecycle, or fabric publish;
- `Interactive`: repaint for the active gesture and settle;
- `Timed`: request the next authored deadline;
- `Continuous`: visible registered animation/media only.

Main, Chat, overlays, and utility windows record:

- viewport identity and lifecycle state;
- last repaint request and reason;
- CPU frame time and pass count;
- visible widget/row count;
- texture delta bytes;
- minimized/occluded state where available.

Transcript rows, session groups, search results, artifacts, semantic trees, and
run histories are virtualized. Layout work is proportional to the visible
window plus a small overscan, not total retained history.

Texture atlases, fonts, icons, and immutable previews are shared where eframe/
wgpu ownership permits. Renderer identity stays explicit. Large dynamic
textures use declared upload and retention budgets; a new utility window does
not duplicate every asset by default.

## 5. Local validation ladder

Each implementation pass runs the smallest truthful layer first:

1. **T0 — static:** format, focused check, documentation/schema checks.
2. **T1 — logic:** affected unit and property tests with in-memory transports.
3. **T2 — surface:** egui harness tests at locked viewport sizes.
4. **T3 — native slice:** EguiPilotAI against the real executable and real
   native viewport(s).
5. **T4 — full local green:** workspace lint, tests, release build, and fresh
   executable assertion.
6. **T5 — remote qualification:** push/PR CI only after T4 evidence is green.

A failure descends to the smallest reproducing tier before another full gate.
Timeout increases require evidence that the operation is legitimately slower;
they may not mask a dropped wakeup, consumed command, deadlock, or unbounded
queue.

Performance runs use deterministic fixtures first and named real-provider
qualification second. Paid/network-backed tests are explicit and never hidden
inside the routine workspace gate.

## 6. Instrumentation deliverables

Before the next major capability expansion, add devtools-only metrics for:

- app/window launch and first-interactive timing;
- per-viewport repaint reason, frame CPU time, pass count, and idle cadence;
- fabric publish rate, queue depth, lag, and rejection;
- provider launch/auth/verification/session/turn timing and process generation;
- history page/cache sizes and transcript rows laid out per frame;
- image/texture cache byte estimates and evictions;
- resident/committed memory and child-process count;
- focused build/test durations captured by the local gate.

Metrics use bounded rings and stable typed IDs. They are inspectable in
devtools and exportable as a redacted artifact. Production defaults do not
retain verbose traces indefinitely.

## 7. Execution sequence

### CAP-0 — baseline

- keep the qualified Windows linker and single-build rule;
- capture clean/warm focused and full-gate build timings;
- capture idle/active Main and Chat frame/repaint/memory behavior;
- inventory unbounded collections, queues, caches, and unconditional repaints;
- ratify the provisional budgets from evidence.

### CAP-1 — observability

- add the bounded metrics model and devtools projection;
- attach repaint reasons and queue/process generations;
- create deterministic performance fixtures and a machine-readable report.

### CAP-2 — highest measured multipliers

- replace accidental polling with fabric wakeups/deadlines;
- virtualize the largest retained views;
- bound the largest caches and payload paths;
- stagger/lazily supervise provider processes;
- remove broad per-frame clones and reparsing found by profiling.

### CAP-3 — doubled-capability simulation

- run fixtures with twice the current providers, sessions, routes, windows,
  interaction components, and retained events;
- run 100 provider/window lifecycle cycles and 10,000 stable frames;
- prove local gates and renderer-keyed native scenarios;
- record remaining headroom and ratify the next capacity tier.

## 8. Release decision

DevPULSE is ready for capability expansion when:

- the baseline report is reproducible on the reference laptop;
- all ratified budgets have an owner and automated or repeatable gate;
- no known queue, cache, history, process, or repaint source is unbounded;
- the doubled-capability fixture remains within frame, memory, lifecycle, and
  validation budgets;
- a feature cannot bypass these limits by introducing a new adapter, window,
  background loop, or storage path.

Optimization is successful when the system becomes more predictable to extend,
not merely when one benchmark becomes smaller.
