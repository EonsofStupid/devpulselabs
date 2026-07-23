# EguiPilotAI v1 delivery plan

Status: active execution plan derived from the 2026-07-22 Playwright/W3C/
AccessKit/egui review and DevPULSE source census. Slice 0 and the protocol-v2
core of Slice 1 are implemented and locally qualified; the next implementation
frontier is Slice 2. The canonical product architecture remains
`docs/plans/eguipilotai.md`.

This plan intentionally favors dependency order over visible UI progress. A
later slice may start only when the previous slice's exit gate is green. Types,
configuration fields, screenshots, and Test Lab placeholders do not count as
capabilities by themselves.

## Working laws

1. Preserve all unrelated DevPULSE work and existing public scenario IDs.
2. Every toolkit capability lands protocol -> runtime -> runner -> authoring ->
   optional MCP -> Test Lab -> conformance -> real DevPULSE.
3. One scenario model serves CLI, Test Lab, MCP exploration export, and CI.
4. No production attach endpoint or runner/runtime dependency.
5. No arbitrary command, environment, code, or filesystem authority crosses
   MCP or the private control protocol.
6. A claim is “implemented” only after a real child process exercises it.
7. Each slice ends with formatting, warning-free lint, focused/workspace tests,
   release build where affected, WASM gates where affected, and status-doc
   correction.
8. Development validation is local-first. The expensive pinned Lavapipe
   qualification is manual-only until Slice 9 explicitly ratifies an automated
   schedule or release gate; ordinary pushes and pull requests must not start it.

## Slice 0 — truth and compatibility baseline

Implementation: **complete for the declared Slice 0 scope.** Frozen wire and
shell fixtures, capability truth labels, evidence-backed status validation,
and the provisional native launch baseline are recorded in
`docs/research/eguipilotai/`.

Purpose: make the current partial surfaces unambiguous before breaking work.

Deliverables:

- label `SetValue`, `ScrollIntoView`, screenshot expectation, workers,
  retries, codegen recorder, replay, approval, and Test Lab live control as
  experimental/unsupported in generated help and schemas;
- add capability IDs to the handshake without changing v1 behavior;
- freeze current protocol v1 fixtures, serde examples, and real shell flow as
  migration evidence;
- record performance/memory baselines for the current shell slice;
- add a documentation status checker so `Ready/Core/Partial/Missing` references
  point to a test or issue/slice.

Exit gate:

- current workspace remains green;
- v1 behavior and wire fixtures are reproducible;
- no command presents an unimplemented action as supported.

Non-goal: add new automation behavior.

## Slice 1 — protocol v2 routing and lifecycle

Implementation: **core complete and exercised through a real DevPULSE child
process.** The evidence record is
`docs/research/eguipilotai/slice-1-protocol-v2-evidence.md`. The pinned
Lavapipe fifty-capture job is an explicit `workflow_dispatch` qualification
and still requires an assigned Linux runner before its separate renderer
qualification can be claimed.

Purpose: establish one response per command and independent runtime events
before trace, cancellation, multi-viewport, or concurrency depend on the wire.

Deliverables:

- typed `RequestId`, `EventId`, `RunId`, `StepId`, and `AttachmentId`;
- separate command, response, and runtime-event envelopes;
- pending-request router supporting interleaved/out-of-order responses;
- long-run progress events, terminal `RunResult`, status during run, and
  idempotent cancellation;
- negotiated capabilities, limits, and trace level;
- lossless/coalescible queue classes with explicit pressure diagnostics;
- protocol v1 removal or an isolated migration bridge in the runner only—no
  dual semantics inside the target runtime.

Tests:

- golden serde/schema fixtures;
- in-memory and stdio conformance with interleaving, out-of-order completion,
  cancellation, saturation, oversized/corrupt frames, disconnect, and version
  mismatch;
- real DevPULSE handshake/status/shell flow over v2.

Exit gate: the runner has no lockstep receive assumption and every accepted
command produces exactly one terminal response.

## Slice 2 — stateful input executor

Purpose: turn high-level actions into deterministic, inspectable input plans.

Deliverables:

- `InputSequence`, typed sources/ticks, session input state, virtual durations;
- deterministic hover/click/double-click/press/type/fill/clear/drag/scroll;
- action plan and tick trace events;
- release-all guard on every terminal/error path;
- per-action actionability matrix, trial mode, stale-geometry re-resolution;
- standard scroll-region wheel search;
- explicit unsupported result for unregistered virtualized scroll and value
  actions.

Tests:

- exhaustive state-machine units and property tests for stuck input;
- multi-DPI hit/drag conformance in a purpose-built egui fixture app;
- forced timeout/cancel/panic/disconnect cleanup;
- real shell hover/click/keyboard route scenarios.

Exit gate: no action is assembled from one-off runtime event vectors; input
state is empty after every test outcome.

## Slice 3 — semantic and AccessKit bridge

Purpose: make one owner registration produce truthful pilot semantics and
truthful real accessibility output.

Deliverables:

- stable pilot/egui/AccessKit identity correlation;
- complete standard helper set and `pilot_interact` for custom-painted
  controls;
- metadata projection/enrichment for role, name, description, values, state,
  relations, actions, bounds, and focus;
- per-viewport actual AccessKit `TreeUpdate` capture and reconstructed mirror;
- `PilotSemanticSnapshot`, `AccessKitSnapshot`, and parity report as distinct
  products;
- AccessKit action injection and diagnostics;
- semantic/accessibility snapshot matchers.

Tests:

- role/state/action mapping table conformance;
- atomic delta reconstruction and desynchronization failures;
- focus/click/increment/decrement/supported set-value paths;
- missing-name, duplicate identity, unstable correlation, and parity errors;
- real DevPULSE shell keyboard traversal and AccessKit snapshot.

Exit gate: no documentation or UI calls pilot-only data an AccessKit tree, and
all shell actionable nodes pass parity.

## Slice 4 — catalog, assertions, and timeout engine

Purpose: create a dependable serial runner before adding parallel complexity.

Deliverables:

- TOML/Rust discovery, catalog validation, deterministic ordering, list and
  selectors/tags/projects/last-failed;
- typed scenario/case/step identity and source hashes;
- stable parameter expansion, typed annotations, explicit
  skip/fixme/xfail/unexpected-pass outcomes, and isolated `--repeat-each`;
- local focus selection with CI `forbid_focus` validation before launch;
- named/nested step groups with source, deadline, evidence, and boxed causal
  failures; typed Rust flow fragments expand into ordinary steps;
- run/action/expect/capture/fixture/shutdown timeout hierarchy;
- complete matcher vocabulary, negation, soft failures, concise retry samples;
- compile-time registered typed custom matchers through the same retry/evidence
  pipeline, with no code-bearing TOML or MCP input;
- strict locator pattern/options/scoping and explanations;
- serial process-per-attempt execution and final classification;
- console and versioned JSON reporter.

Tests:

- catalog golden tests and invalid-input zero-launch assertions;
- parameter collision, annotation condition/outcome, nested step, and
  repetition-isolation matrices;
- fake-clock timeout precedence;
- assertion state transition tables and concise failure bundles;
- deterministic repeat of the existing shell flow through catalog selection.

Exit gate: `eguipilotai test --workers 1` is a complete, predictable runner
for semantic scenarios.

## Slice 5 — fixtures, project matrices, retries, shards, reports

Purpose: add scale only after serial lifecycle semantics are fixed.

Deliverables:

- typed fixture provider/lease/probe API, DAG composition, scenario and worker
  scope, unique jailed roots, reverse teardown;
- project expansion for renderer, viewport, DPI, theme, locale, timezone,
  fonts, input, motion, and fixtures;
- acyclic visible project-dependency scenarios with normal traces/retries/
  reports and immutable content-addressed outputs;
- worker scheduler and fresh process/fixture per retry attempt;
- pass/flaky/expected-failed/unexpected-passed/fail/infrastructure/skip/invalid
  classification;
- stable independently computable sharding and report merge validation;
- JUnit reporter and fail-fast cleanup behavior.

Tests:

- fixture failure matrix, cycle detection, teardown rescue, path jail;
- project-dependency ordering, failure/skip propagation, cycle rejection, and
  mutable-state isolation, including deterministic per-shard repetition;
- project required/optional capability and renderer policy matrix;
- retry isolation/flaky history;
- shard union/intersection and merged report integrity;
- real DevPULSE seeded Library projects at two viewport sizes.

Exit gate: configured workers/retries/projects are executed behavior, not
decorative fields.

## Slice 6 — structured traces, replay, and visual assertions

Purpose: make every failure explainable and every visual approval auditable.

Deliverables:

- typed trace event schema and committed content-addressed attachments;
- deterministic automatic screenshot retention (`off`, `on`, `failure`,
  `first-failure`) independent of baseline assertions;
- semantic/AccessKit checkpoints and verified deltas;
- evidence replay plus deterministic live replay to step;
- runner-integrated capture/compare with semantic regions/masks;
- renderer-keyed baseline lookup and immutable candidate/approval library;
- expected/actual/diff evidence and CLI review commands;
- trace retention/size policy.

Tests:

- interrupted/corrupt trace and attachment recovery;
- replay equality/divergence;
- missing/wrong-renderer/dimension/threshold visual errors;
- append-only approval under interruption;
- fifty byte-identical suppressed Lavapipe captures in the pinned container.

Exit gate: `Expectation::Screenshot` is removed in favor of the complete visual
expectation or works through that complete path; no separate capture semantics.

## Slice 7 — DevPULSE instrumentation waves

Purpose: dogfood the stable contracts across the real app without widening the
toolkit ad hoc.

### 7A shell and Library frame

- full-bleed, backdrop tiles, status launchers, inspector anchors;
- Library template wall, solo back/replay, standard scroll region;
- stable ID migration aliases for existing shell scenarios;
- keyboard traversal and full pilot/AccessKit parity.

### 7B static and flourish templates

- Title, Big Number, Callout, Split, Bar, Line, Probability, Stack, Gauge;
- Code copy, Stats flip, Donut slice interaction;
- stable data keys and finite motion registration;
- semantic and visual parameterized scenario matrix.

### 7C interactive templates

- Reveal, Choice, Quiz, Cost, Predict, Drag;
- state-machine semantics, enabled/selected/progress/status/error fields;
- pointer and keyboard flows;
- correct/wrong drag trajectories and keyboard pickup/place;
- natural/fast-forward/suppressed interaction captures.

### 7D Workbench and PixVix

- seed reload with fixture lifecycle;
- palette/region viewport roots and lifecycle;
- tabs/combo/actions/status, busy tokens, REC motion;
- jailed output probes and capture engine boundaries;
- launch/interact/cancel/capture/shutdown scenarios.

Each wave exits only when its default agent snapshot is action-complete and
within budget, real AccessKit parity passes, and a real-process scenario covers
every interaction family.

## Slice 8 — Test Lab and agent authoring

Purpose: project the finished runner/evidence model into human and autonomous
workflows.

Deliverables:

- live session status and bounded control through runner APIs;
- pruned/full pilot and real AccessKit tree inspection, search/highlight,
  actionability explanation;
- run/attempt timeline from structured trace;
- evidence and live replay;
- visual candidate review/approval;
- exploration recorder that captures high-level actions/assertions, chooses
  stable locators, and exports reviewable TOML/Rust diff;
- `codegen` validates/normalizes/exports but never silently overwrites durable
  tests;
- Codex/Cursor/Claude conformance sessions through the same six MCP tools.

Tests:

- Test Lab controls a separate DevPULSE target, never itself;
- recorder round-trip runs the exported scenario successfully;
- locator picker prefers declared IDs and flags volatile fallbacks;
- agent default snapshots stay within budget and continuation is lossless.

Exit gate: Test Lab contains no private runner logic and codegen is truthfully
a recorder/exporter.

## Slice 9 — CI, performance, security, and publication

Purpose: qualify the toolkit rather than merely demonstrating it.

Deliverables:

- digest-pinned Ubuntu/Xvfb/Mesa Lavapipe image and package/font manifest;
- scheduled/release hardware projects with separate baselines;
- WASM compilation gates for protocol/transport/instrumentation/runtime core;
- production dependency/endpoint audit;
- fuzz/property tests for wire, continuation, paths, traces, and scenario
  parsing;
- p95 frame-time and 10,000-frame memory benches;
- semver/API docs, examples, migration notes, crate metadata, license/security
  review, and publication dry run.

Exit gate: all canonical publication gates pass from a clean checkout and the
published examples use only supported, tested contracts.

## Dependency view

```text
truth baseline
  -> protocol routing
  -> input executor
  -> semantic/AccessKit bridge
  -> serial runner
  -> fixtures/scheduler
  -> traces/visuals
  -> DevPULSE coverage
  -> Test Lab/recorder
  -> publication qualification
```

Protocol routing precedes traces because trace causality depends on it. The
input executor precedes broad instrumentation because controls should target
one stable behavior. Accessibility precedes the catalog assertion surface so
snapshot names and matchers are honest. Serial lifecycle precedes workers and
retries so parallelism multiplies a correct unit rather than ambiguity.

## Change review checklist

Every pull request/slice answers:

- Which public contract changed?
- Which capability ID advertises it?
- What is the unsupported-path behavior?
- What runtime/runner/Test Lab evidence shares the same type?
- Which toolkit conformance test proves it?
- Which real DevPULSE scenario proves it?
- What production dependency/endpoint impact exists?
- What agent snapshot/token impact exists?
- What motion/renderer/replay impact exists?
- Which status row moved, and why is the new label earned?
