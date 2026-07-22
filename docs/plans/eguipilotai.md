# EguiPilotAI — canonical agent-first egui testing architecture

Status: ratified for implementation on 2026-07-22.

EguiPilotAI is a reusable Rust toolkit for deterministic agent exploration and
durable testing of native egui applications. DevPULSE Labs is the first
qualification target. Autonomous MCP exploration and reviewed Rust/TOML tests
are equal product contracts backed by one typed scenario model.

`eguidev` is research inspiration only. Browser concepts are translated to
native equivalents. WASM execution is outside v1, but protocol, transport,
instrumentation, and runtime-core boundaries must remain browser-adapter-ready.

## Ratified exception and invariants

DevPULSE's no-IPC law remains the production rule. EguiPilotAI may use a
private, inherited, nonce-authenticated pipe only in an explicitly enabled
test/devtools build controlling a separate child process. Normal production
builds expose no endpoint and depend on no runtime, process launcher, or MCP
server.

1. Static controls use declared `PilotId` constants; dynamic controls use
   typed parent scopes and stable keys.
2. Runtime `NodeId` values carry viewport and hierarchy. Duplicate live IDs
   fail immediately and report both source locations.
3. Real egui `RawInput` frames perform pointer and keyboard actions. Named
   white-box adapters are never mislabeled as user input.
4. Full semantics remain local to Test Lab, traces, and diagnostics. Agents
   receive bounded projections.
5. Visual motion consumes `PilotClock` or registers a motion adapter. A
   capture fails on uncontrolled motion.
6. Renderer selection is explicit and recorded. Software and hardware
   renderer families never share required baselines.
7. Approved baseline bytes are immutable and content-addressed.
8. MCP selects named profiles and fixtures only. It accepts no executable,
   environment, shell, or unrestricted artifact path.

## Crate boundaries

- `egui-pilot-ai-protocol`: versioned envelopes, scenarios, traces, selectors,
  actions, expectations, projects, fixtures, renderer identities, and errors.
- `egui-pilot-ai-transport`: bounded nonblocking transport plus in-memory and
  native stdio adapters and conformance tests.
- `egui-pilot-ai`: plugin, typed identity, semantic/AccessKit metadata,
  virtual clock, motion registry, busy tokens, and widget helpers.
- `egui-pilot-ai-runtime`: input injection, viewport discovery, settlement,
  deterministic capture, lifecycle coordination, and transport-independent
  sessions.
- `egui-pilot-ai-runner`: named configuration, isolated processes, locators,
  assertions, projects, fixtures, traces, artifacts, and visual comparison.
- `egui-pilot-ai-cli`: the `eguipilotai` CLI and stdio MCP server.
- `egui-pilot-ai-lab`: native views for complete/pruned trees, actionability,
  sessions, traces, replay evidence, and visual approval.

Each crate owns one concern and uses public APIs across the boundary. The
application owns `RuntimeGuard`, preventing the plugin from retaining an
`egui::Context` and creating a lifecycle cycle.

## Runtime integration

Register one `EguiPilotPlugin` with the application's `egui::Context`.

- Input hooks inject queued events, modifiers, viewport identity, and virtual
  time.
- Pass/output hooks commit semantic frames, output/AccessKit diagnostics,
  screenshots, repaint state, and viewport mappings.
- `UiPilotExt` wraps standard widgets; `pilot_track` instruments custom
  `allocate_rect` interactions.
- Automation and accessibility data originate from the same registration.
- The control runtime is compiled only by the devtools feature.

## Transport and protocol

`Transport` is deliberately nonblocking and does not require `Send`, native
threads, or an async runtime. It provides `try_receive`, `try_send`, `close`,
bounded capacity, explicit backpressure, and size-checked versioned envelopes.

Native `StdioTransport` performs blocking pipe I/O on bounded worker threads.
`InMemoryTransport` drives deterministic conformance tests. A future
`MessageChannelTransport` or `WebSocketTransport` must be possible without
changing the protocol or runner scenario model. Native process launch, CLI,
and stdio remain target-gated; core crates compile for
`wasm32-unknown-unknown`.

The handshake validates protocol version, target build, session ID, and a
nonce that exists only in the private launch environment. Stdout contains only
framed protocol bytes; target diagnostics are captured from stderr.

## Agent snapshot contract

The runtime stores a complete `SemanticFrame`. `pilot_snapshot` returns a
separate `AgentSnapshot`:

- `agent` is the default pruned projection.
- `changes` includes nodes changed since a supplied revision.
- `full` is an explicit diagnostic projection with a stricter default.
- Requests can scope by viewport, subtree, role, state, or locator.
- Overflow is losslessly discoverable through deterministic continuation.

The default projection includes actionable controls, the focus path, headings,
labels, alerts, status/error text, changed nodes, and text needed to understand
visible actions. It hoists useful children through layout-only containers and
retains meaningful region breadcrumbs. It omits hidden/fully clipped,
decorative, styling-only, empty layout, null/default, and repeated inherited
data unless explicitly requested.

Default budget: 200 nodes and 12 KiB. Hard maximum: 1,000 nodes and 64 KiB.
Every snapshot returns revision, content hash, truncation reason, and optional
continuation token. `pilot_run` returns compact step outcomes; failures attach
only relevant ancestry, nearby controls, diagnostics, and the changed subtree.

Exploratory flows use the same actions, expectations, and fixtures as durable
TOML/Rust scenarios, so useful sessions can be exported without translation
to a second automation model.

## Locators, actionability, and settlement

Before every action the runtime resolves the locator again and requires a
single current target in the requested viewport. Unless force is explicitly
supported, the target must be visible through its clip chain, stable for two
frames, enabled/editable as relevant, and capable of receiving the intended
hit. Ambiguity is always an error. Scroll adapters may bring a target into
view; a missing adapter is reported honestly.

Pointer operations use real hover, press, move, and release frames. Drag uses
multiple pointer-move frames between source and target. Keyboard actions use
real key/text events.

Settlement requires no pending input action, no busy token, and stable relevant
semantic/layout hashes for two frames. Global repaint silence is never required
because DevPULSE intentionally contains perpetual ambient loops. Assertions
retry through normal frame timing until their bounded deadline.

## Deterministic motion and capture

Public motion vocabulary:

- `PilotClock`
- `MotionId`
- `MotionKind::Finite | Ambient`
- `MotionPolicy::Natural | FastForward | Suppressed`
- `MotionStatus`

Capture preflight pins virtual time, sets egui animation duration to zero,
clears egui's animation manager, advances registered finite motion to its
terminal time, resolves scroll easing, and moves ambient motion to its declared
canonical phase. Capture renders until two consecutive semantic, geometry, and
pixel hashes match. Registered motion without a canonical state produces
`UncontrolledMotion`; arbitrary application wall-clock code is never claimed
to be generically finishable.

## Renderer policy and visual identity

Every named profile declares one policy:

- `SoftwareRequired`
- `HardwareRequired`
- `HardwarePreferred`
- `HardwareThenSoftware`

The wgpu adapter selector enforces required policies rather than silently
crossing adapter classes. The runtime records OS, backend, adapter name, device
type, driver, renderer/wgpu version, surface format, DPI, font bundle, and
dithering configuration. The runner verifies the reported device class after
handshake.

The routine cloud lane runs a digest-pinned Ubuntu image under Xvfb, explicitly
selects Vulkan, Mesa Lavapipe/LLVMpipe, CPU device type, locale, timezone, and
font packages, and records exact installed package/font manifests. Dedicated
hardware qualification runs separately.

Baseline keys include OS, backend, adapter class/name, renderer version,
surface format, DPI, font bundle, and visual configuration. Pinned Lavapipe
defaults to exact pixels. Approved hardware projects use a separate tight
threshold. Cross-renderer comparison is informational only. An allowed
fallback needs its own approved renderer key.

## CLI and MCP

CLI surface:

- `eguipilotai doctor`
- `eguipilotai mcp`
- `eguipilotai test`
- `eguipilotai inspect`
- `eguipilotai fixtures`
- `eguipilotai codegen`
- `eguipilotai open`
- `eguipilotai agent install codex|cursor|claude`

MCP exposes exactly:

- `pilot_start`
- `pilot_status`
- `pilot_snapshot`
- `pilot_run`
- `pilot_capture`
- `pilot_stop`

MCP schemas reject additional properties and never accept launch or shell
authority. The trusted project config is `.eguipilotai.toml`.

## Process, artifact, and baseline rules

Test Lab launches a separate target process and never controls itself
in-process. A run is isolated under `target/eguipilotai/runs/<run-id>/` and may
contain:

- launch manifest and full renderer identity;
- framed trace and semantic revisions;
- screenshots and visual diffs;
- stderr/logs and timing;
- timeout, disconnect, panic, crash, or renderer failure evidence.

Project baselines live under `tests/ui/baselines/`. Approval creates a new
content-addressed image and additive approval record. Existing images and
approval records are never overwritten or deleted.

## Native Test Lab

Workbench Test Lab provides:

- pruned and full semantic views;
- node search, hierarchy, accessibility, highlighting, and actionability
  explanations;
- target session and renderer status;
- run history and failure bundles;
- trace inspection for input, waits, tree changes, diagnostics, motion, and
  timing;
- expected/actual/diff review and immutable candidate approval;
- fixture-based replay to a selected step.

## Delivery sequence

1. Ratify architecture and the test/devtools IPC exception.
2. Implement protocol, transport, typed semantics, pruning, and in-memory
   conformance.
3. Instrument the DevPULSE shell and prove an exploratory-plus-durable slice.
4. Implement native stdio runtime, actionability, real input, clocks, motion,
   fixtures, capture settlement, and multi-viewport discovery.
5. Implement process runner, durable scenarios, retries/sharding/reporters,
   traces, and exploration export.
6. Add Lavapipe CI, renderer-keyed visuals, replay, and Test Lab.
7. Complete DevPULSE interaction/accessibility coverage, agent conformance,
   performance gates, WASM gates, and publication qualification.

## Publication gates

Required DevPULSE behavior includes shell hover flyouts, route navigation,
responsive layouts, keyboard traversal; choice, quiz, reveal, prediction,
cost-meter, statistics, and copy interactions; real Drag Match movement/drop;
finite and ambient motion under all policies; PixVix launch/multi-viewport
interaction/capture/shutdown; accessibility snapshots; and keyboard-only use.

Required diagnostics cover duplicate IDs, ambiguity, clipping, disabled
targets, uncontrolled motion, stalled busy tokens, protocol mismatch,
disconnect, timeout, panic, and renderer failure.

Quantitative gates:

- The Library default agent snapshot stays within 12 KiB and retains all
  visible actions, route context, labels, headings, alerts, and errors;
  continuation discovers every omitted node.
- Fifty suppressed captures are byte-identical under pinned Lavapipe.
- Required software/hardware profiles never silently cross adapter classes.
- Transport conformance passes for in-memory and stdio adapters.
- Runtime-core crates compile for `wasm32-unknown-unknown`.
- Production dependency inspection finds no runtime endpoint or MCP server.
- Representative instrumentation remains within 15% p95 CPU frame-time
  overhead.
- Growth after warm-up stays below 5 MiB across 10,000 stable frames.
- Formatting, warning-free linting, workspace tests, release build, real
  executable suites, and both autonomous/durable agent workflows pass before
  publication.

## Memory contract

This file is the only canonical architectural plan. Project agent files point
here instead of copying it. User-level Codex, Claude, and Cursor guidance holds
only stable cross-project operation rules and records this file's content hash
so drift is detectable. Existing Clyffy-specific global instructions remain
untouched.
