# EguiPilotAI — canonical agent-first egui testing architecture

Status: ratified for implementation on 2026-07-22; native multi-viewport
contract amended on 2026-07-23 after the exact egui 0.35 and DevPULSE audit.

EguiPilotAI is a reusable Rust toolkit for deterministic agent exploration and
durable testing of native egui applications. DevPULSE Labs is the first
qualification target. Autonomous MCP exploration and reviewed Rust/TOML tests
are equal product contracts backed by one typed scenario model.

The implementation is reviewed against Playwright's behavioral contracts in
`docs/eguipilotai-playwright-review.md`. DevPULSE's one-path authoring rules
live in `docs/eguipilotai-devpulse-authoring.md`. Those files report status and
show usage; this file remains the only architectural authority.

Detailed engineering contracts and the source census are indexed in
`docs/research/eguipilotai/README.md`. The dependency-ordered execution plan is
`docs/plans/eguipilotai-v1-delivery.md`. Those documents elaborate this
architecture and must not weaken its security, production, or evidence laws.

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
9. Every accepted protocol command has exactly one correlated success/error
   response. Asynchronous runtime events are independently identified and
   never masquerade as command responses.
10. Pilot semantics and the real AccessKit output are distinct verified
    projections of one owner registration. Reports never call pilot-only data
    an accessibility tree.

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
- Pass/output hooks commit semantic frames, actual AccessKit deltas and their
  reconstructed per-viewport trees, screenshots, repaint state, diagnostics,
  and viewport mappings.
- `UiPilotExt` wraps standard widgets; `pilot_track` instruments custom
  `allocate_rect` interactions.
- Standard wrappers and `pilot_track` register automation and accessibility
  data from one owner call: the stable `PilotPath` keys the pilot semantic
  node, and the same role and name feed egui's real AccessKit output through
  `WidgetInfo`. Recording the real `egui::Id` and AccessKit tree/node IDs
  against that path, with parity checks, is specified in
  `docs/research/eguipilotai/accessibility-semantics.md` and not yet built.
- The control runtime is compiled only by the devtools feature.

## Boring extension contract

EguiPilotAI deliberately has one extension lane. A new capability is not
complete when only a protocol enum or a Test Lab button exists. It lands as a
vertical slice in this order:

1. shared typed protocol data and serialization tests;
2. runtime behavior using real egui input, or an explicitly named white-box
   adapter when real input cannot represent the operation;
3. runner orchestration, artifact capture, and failure classification;
4. durable TOML/Rust authoring support;
5. compact MCP exposure only when an autonomous agent genuinely needs it;
6. Test Lab inspection/replay support;
7. one toolkit conformance test and one real DevPULSE scenario.

Application customization does not fork the toolkit. DevPULSE supplies only:

- IDs and semantic metadata in the crate that owns the control;
- named profiles, projects, and fixtures in `.eguipilotai.toml`;
- deterministic fixture/service adapters registered by typed ID;
- durable flows under `tests/eguipilotai/`;
- renderer-keyed approvals under `tests/ui/baselines/`.

Stable controls use one declared `PilotId`. Repeated controls use one declared
parent plus a domain-stable `PilotKey`. Standard widgets use `UiPilotExt`;
custom-painted interactions use `pilot_track`. Async work holds one busy token
for its exact lifetime. Visual motion consumes `PilotClock` or registers once
at its point of authorship. There is no second instrumentation path.

App-specific operations such as seeding a content root, inspecting a saved
document, or simulating an OS boundary use a named, typed adapter. They are
reported as adapter operations in traces and are never described as pointer or
keyboard input. MCP may select a predeclared adapter through a fixture, but may
not supply executable code, paths, environment, or arbitrary adapter names.

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

Commands, responses, and runtime events are distinct envelopes. Typed request
IDs correlate exactly one success/error response to every accepted command;
typed event IDs order independent lifecycle, step, diagnostic, viewport,
motion, and attachment events. Long-running runs may emit progress events and
finish out of request order while status/cancel commands remain serviceable.
Required events are lossless; declared high-rate telemetry may coalesce with a
reported drop count. Full trees and image bytes are externalized as bounded,
jailed attachments rather than queued inline.

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

High-level actions compile into an internal stateful input sequence with typed
keyboard, pointer, and wheel sources and synchronized per-frame ticks. The
runtime tracks pressed keys/buttons/modifiers and performs release-all cleanup
after success, failure, timeout, cancellation, stop, or recoverable panic. A
cleanup failure prevents process reuse. Input plans and ticks are traceable;
raw event injection is not an MCP capability.

Settlement requires no pending input action, no busy token, and stable relevant
semantic/layout hashes for two frames. Global repaint silence is never required
because DevPULSE intentionally contains perpetual ambient loops. Assertions
retry through normal frame timing until their bounded deadline.

## Native multi-viewport contract

An additional egui viewport is a first-class window lifetime inside the same
target process and session. It is not inferred from the presence of semantic
nodes and is not modeled as an unrelated process or browser page.

- Applications declare a stable typed `PilotViewportKey` beside the code that
  owns each window. Root is registered automatically.
- The runtime allocates a new `ViewportId` generation every time a declared
  key becomes live. Durable scenarios select the key; actions pin and recheck
  the current lifetime so reopened windows cannot satisfy stale work.
- The registry records parent, immediate/deferred/embedded class, title,
  focus/visibility/minimize/maximize/occlusion state, logical geometry,
  native pixels-per-point, and lifecycle revision from authoritative
  `RawInput.viewports`.
- Created, close-requested, focused, and closed transitions are lossless
  runtime events. Geometry/status updates may coalesce.
- Closed windows are removed from live semantics, AccessKit mirrors, input,
  captures, motion/busy ownership, and lookup maps. Bounded tombstones exist
  only for waits and evidence and are never actionable.
- Locators, non-node keyboard/wheel actions, focus/close operations, and
  captures use strict `ViewportSelector` resolution. An omitted selector is
  valid only when exactly one viewport is live.
- Input source state is viewport-bound. Cross-window drag remains unsupported
  because per-viewport `RawInput` cannot honestly represent OS-global drag.
- Captures resolve one viewport strictly, match screenshot events to that
  viewport, settle against its semantic/geometry/pixel hashes, and record its
  key, lifetime, class, dimensions, and DPI. Root capture is qualified.
  Immediate child capture is explicitly unsupported in eframe 0.35 native wgpu
  because its renderer drops screenshot requests. Real Chat qualification
  also proves background deferred viewports do not deliver queued screenshot
  requests. Both child classes fail at capability preflight. Session-wide
  stitched capture is a separate future capability.
- Native-required projects reject `EmbeddedWindow` fallback. Platform-optional
  geometry such as Wayland outer position is never fabricated.
- Shutdown releases input, closes children leaf-first, flushes lifecycle
  evidence, then closes root.

The shared instrumentation wrapper supports immediate and deferred egui
viewports. PixVix qualifies the immediate path. The app-owned Chat peer window
qualifies deferred discovery, independent interaction while Main is minimized,
focus, minimize/restore, owner-mediated close, reopen, and teardown. Native
child pixels remain a separate blocked capability and are never inferred from
a root screenshot. The detailed evidence and open gates are in
`docs/research/eguipilotai/multi-viewport.md`.

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

Runner parity also requires predictable commands/options for test discovery,
project selection, filtering/tags, timeout hierarchy, workers, sharding,
retries, fail-fast, last-failed, structured reporters, trace opening, and
baseline candidate review. These are runner/CLI concerns and do not expand the
six-tool MCP surface.

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

1. Freeze honest current status and protocol-v1 migration evidence.
2. Implement protocol-v2 correlated responses, independent events,
   cancellation, capabilities, and bounded pressure.
3. Implement the stateful input-source/tick executor and release-all cleanup.
4. Implement native viewport identity, lifecycle, targeting, scoped capture,
   and deterministic child-first shutdown.
5. Implement one semantic registration projected into pilot and actual
   AccessKit output, with tree reconstruction and parity tests.
6. Implement the serial catalog/assertion/timeout runner.
7. Add fixture lifecycles, project scheduling, isolated retries, sharding, and
   structured reports.
8. Add structured traces, deterministic replay, renderer-keyed visuals, and
   immutable approval.
9. Instrument DevPULSE in shell/Library, templates, then Workbench/PixVix
   waves, each with real-process pointer, keyboard, accessibility, and motion
   scenarios.
10. Project the shared runner/evidence model into Test Lab and reviewed agent
   recorder/codegen workflows.
11. Complete Lavapipe/hardware CI, security/performance/memory/WASM gates, and
    publication qualification.

Exact slice deliverables, tests, non-goals, and exit gates live in
`docs/plans/eguipilotai-v1-delivery.md`.

The sequence is capability-gated. Data types or UI placeholders do not advance
a phase. In particular, project `workers`/`retries`, structured reporters,
fixture lifecycle, semantic snapshot matching, baseline approval, recorder
codegen, replay, and Test Lab live control remain incomplete until their full
vertical slices pass real-process scenarios.

## Publication gates

Required DevPULSE behavior includes shell hover flyouts, route navigation,
responsive layouts, keyboard traversal; choice, quiz, reveal, prediction,
cost-meter, statistics, and copy interactions; real Drag Match movement/drop;
finite and ambient motion under all policies; PixVix launch/multi-viewport
interaction/capture/shutdown; accessibility snapshots; and keyboard-only use.

Required diagnostics cover duplicate IDs, ambiguity, clipping, disabled
targets, uncontrolled motion, stalled busy tokens, protocol mismatch,
disconnect, timeout, panic, renderer failure, stale viewport lifetime,
ambiguous viewport, unsupported embedded fallback, child capture mismatch,
and child shutdown timeout.

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
