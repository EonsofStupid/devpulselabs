# Runner contract

Status: specified, not implemented. This document defines the minimum
Playwright-class orchestration behavior required before EguiPilotAI is called
a dependable test runner.

## Design goal

One scenario must behave the same when invoked from the CLI, Test Lab, CI, or
an agent export. The runner owns discovery, selection, lifecycle, deadlines,
attempt isolation, evidence, classification, and reporting. The target process
owns egui behavior. MCP is a bounded client, not a second runner.

## Test model

The durable identity hierarchy is:

```text
SuiteId / ScenarioId / CaseId / StepId
```

- `SuiteId` is derived from the normalized path below `tests/eguipilotai/` or
  declared explicitly in Rust.
- `ScenarioId` is mandatory and unique within the workspace.
- `CaseId` is a stable, author-supplied parameterized case key. Array indexes
  are not identities.
- `StepId` is explicit when referenced by replay/codegen; otherwise it is a
  deterministic hash of scenario identity and source position.

A catalog entry contains source path/line, tags, required capabilities,
fixture, project include/exclude rules, timeout overrides, motion policy,
record policy, and immutable source hash. Discovery parses and validates
without launching the application.

### Parameters and repetition

Parameterized Rust and TOML declarations expand into ordinary catalog entries
before selection. Each case supplies a stable `CaseId`; the runner never uses
an array index, display name, or serialized parameter blob as identity.
Parameters must be schema-valid, deterministic data and are recorded in the
catalog hash and report.

`--repeat-each N` creates isolated repetition attempts after selection. A
`RepeatOrdinal` is evidence metadata, not part of `CaseId`, so repetition does
not create new baseline names or durable scenario identities. Every repetition
gets its own process, fixture lease, run directory, trace, and result. Retries
remain nested inside a repetition and are never confused with repetitions.

### Annotations and outcome policy

Annotations are typed catalog metadata, not executable callbacks:

- tags are stable strings used for selection and reporting;
- `focus` narrows local selection but is rejected before launch when
  `forbid_focus` is enabled; CI profiles enable it by default;
- `skip` prevents execution only from declared project, platform, negotiated
  capability, or fixture facts and always records a reason;
- `xfail` executes normally and declares an issue/reference plus allowed test
  failure classifiers. A matching failure becomes `expected-failed`; a pass
  becomes `unexpected-passed`; a different test failure remains `failed`.
  Infrastructure failures are never accepted by `xfail`;
- `fixme` does not execute and requires a reason plus an issue/reference;
- freeform notes are report-only metadata.

EguiPilotAI does not copy Playwright's implicit `slow` timeout multiplier.
Slow work uses an explicit timeout override or a named timeout profile; a
`slow` tag may describe it but cannot silently change execution.

Annotation conditions cannot inspect arbitrary environment variables, run
shell commands, or execute code from TOML. Trusted Rust declarations may map
typed application facts into the same serializable condition vocabulary.

### Step groups and reusable flows

Leaf steps are runner-owned actions, expectations, captures, fixture probes,
or explicit clock operations. Authors may wrap leaves in named, nested step
groups with source location, optional deadline, tags, and evidence policy.
Groups appear in console/JSON/JUnit/trace output and may box their child
failure at the group boundary while retaining the full cause chain.

TOML has no arbitrary `beforeEach`, `afterEach`, `beforeAll`, or `afterAll`
callbacks. Fixture setup/teardown owns resources and application state;
project dependency scenarios own visible one-time qualification work. Rust
authors may define typed reusable flow fragments, but expansion yields normal
steps with stable source/identity and no hidden execution.

## Discovery and selection

Default roots:

- `tests/eguipilotai/**/*.toml`;
- Rust inventory registered by `egui-pilot-ai-runner` test macros.

The catalog is sorted by `(suite_id, scenario_id, case_id)`. Duplicate IDs,
unknown fixtures/projects/tags, invalid annotation conditions, invalid key
grammar, empty cases, unsupported capabilities, and invalid timeout values
fail discovery before any process is launched.

CLI selection composes as an intersection:

- explicit scenario IDs or path patterns;
- include/exclude tags;
- project filters;
- shard membership;
- `--last-failed` from a compatible previous manifest.

An empty selection is a successful no-op only with `--allow-empty`; otherwise
it is an authoring/configuration error. `--list` prints the resolved catalog in
execution order without launching targets.

## Timeout hierarchy

Timeouts use typed durations and one monotonic runner clock. Virtual
application time never consumes a wall-clock deadline by itself.

| Deadline | Built-in default | Covers |
|---|---:|---|
| scenario | 30 s | target launch after fixture setup, all steps, and graceful target stop |
| action | 5 s | resolution, auto-scroll, actionability, input ticks, settlement |
| expectation | 5 s | retry samples until matcher success |
| capture | 10 s | motion preflight, stable renders, image persistence |
| fixture setup | 30 s | one provider setup |
| fixture teardown | 30 s | one provider teardown, even after scenario deadline |
| target shutdown | 3 s | graceful stop before forced termination |

Precedence for a specific operation is:

```text
leaf override -> enclosing groups (inner to outer) -> scenario override
  -> project override -> profile/config -> built-in
```

The scenario deadline is still a cap on action/expect/capture work. Fixture
setup occurs before the scenario clock. Teardown receives a separate rescue
budget so cleanup is not skipped because the scenario timed out. Worker-scope
fixture setup/teardown uses its own declared budget.

Every timeout names the expired deadline, effective value and source, elapsed
time, current run/step, last progress event, busy tokens, motion state, last
semantic revision, target health, and available partial artifacts.

## Assertions

All user-visible state assertions are retrying unless documented as immediate.
Each sample resolves the locator against the newest semantic revision and
records only state changes by default.

### Match vocabulary

- text patterns: exact, contains, prefix, suffix, and regex with explicit
  normalization/case options;
- state: visible, hidden, enabled, disabled, editable, read-only, focused,
  checked/mixed, selected, expanded/collapsed, in viewport;
- identity/content: attached, role, accessible name, description, text,
  value, count;
- structure: child/descendant count, ordered focus path, scoped semantic
  snapshot;
- accessibility: normalized AccessKit snapshot and semantic/AccessKit parity;
- diagnostics: none, exact code, or bounded expected set;
- artifacts: visual snapshot and typed fixture probe result.

Every expectation supports negation, timeout, and custom message. Soft
expectations append failures and allow the scenario to continue, but the
attempt is failed at its end. A hard expectation terminates the remaining
scenario steps after cleanup.

Immediate assertions are reserved for immutable runner data such as catalog
metadata, negotiated capability, or completed attachment hashes. They are not
a back door around UI retrying.

Custom matchers are trusted, compile-time registrations addressed by stable
`MatcherId`. TOML and MCP may select a registered matcher and pass its
schema-validated data; they cannot upload code or closures. A matcher returns
typed samples, retryability, and structured expected/actual evidence through
the same timeout and failure pipeline as built-in expectations.

### Failure evidence

An expectation failure includes expected matcher data, final actual value,
sample count/timestamps, semantic revisions in which the value changed,
locator explanation, relevant ancestry and siblings, current focus, busy and
motion state, and attachments selected by policy. It does not dump the whole
application tree.

## Fixtures

Configuration names fixtures; trusted Rust code may implement their lifecycle.
They are different concepts:

- `FixtureSpec`: serializable declaration and safe parameters in
  `.eguipilotai.toml`.
- `FixtureProvider`: compiled, registered implementation selected by typed ID.
- `FixtureLease`: one prepared resource lifetime with its jailed roots,
  launch overlay, probes, attachments, and teardown.

```rust,ignore
trait FixtureProvider {
    fn id(&self) -> FixtureId;
    fn scope(&self) -> FixtureScope;
    fn dependencies(&self) -> &[FixtureId];
    fn prepare(&self, context: &FixtureContext) -> Result<Box<dyn FixtureLease>>;
}

trait FixtureLease {
    fn launch_overlay(&self) -> &NamedLaunchOverlay;
    fn probes(&self) -> &[Box<dyn FixtureProbe>];
    fn teardown(self: Box<Self>) -> Result<Vec<Attachment>>;
}
```

This is a target API shape. The concrete design may avoid trait objects, but
the lifecycle and authority boundary are mandatory.

Rules:

1. Dependencies form an acyclic graph. Setup is topological; teardown is
   reverse order.
2. The default scope is `ScenarioAttempt`. It creates a unique data root for
   every retry.
3. `Worker` scope is allowed only for immutable or explicitly resettable
   services. Mutable DevPULSE state is never shared between scenarios.
4. A lease may contribute only named, schema-validated launch arguments and
   environment fields approved in project configuration.
5. MCP selects a fixture ID and public parameters only. It cannot choose a
   provider, executable, raw environment, shell command, or filesystem path.
6. Probes are typed read-only or narrowly declared mutations. Trace records
   label them `fixture-probe`/`fixture-operation`, never user input.
7. Teardown always runs. A teardown failure is attached separately and can
   upgrade a passed attempt to infrastructure failure.

DevPULSE initial providers should cover a temporary content root, seeded
Library/episode data, empty inbox, PixVix output jail, and deterministic fake
external services as those services arrive.

## Process and attempt isolation

The default unit of isolation is one native target process per scenario
attempt. The runner never reuses a process after failure, timeout, input
cleanup failure, panic, renderer mismatch, or protocol error.

Attempt lifecycle:

1. prepare fixture graph and create jailed run/data roots;
2. validate launch and renderer policy;
3. start target with private authenticated transport;
4. handshake and capability preflight;
5. wait for declared ready anchor;
6. execute steps;
7. stop target gracefully;
8. finalize trace and attachments;
9. teardown fixtures;
10. classify and report.

`beforeAll`-style shared application state is intentionally absent in v1.
Expensive immutable services belong to worker fixtures; UI state remains
attempt-local.

## Projects and scheduling

A project is a named matrix row: profile, fixture defaults, renderer policy,
viewport/window size, DPI, theme, locale, timezone, font pack, input mode,
motion policy, visual policy, worker count, retry count, tag filters, and
optional project dependencies.

Project dependencies are ordinary reportable projects containing explicit
setup/qualification scenarios. The dependency graph is validated as acyclic
before launch. Dependencies run first with their normal fixtures, traces,
retries, and reporters. Failure or skip prevents dependent work and records
the causal result. A dependency may expose only immutable, content-addressed
fixture artifacts; it cannot pass a live UI process or mutable application
state to dependents. Resource composition remains a fixture-DAG concern.

Within one runner invocation a dependency project executes once before its
selected dependents. Each independently launched CI shard executes its own
dependency project, because v1 has no distributed coordinator. Dependencies
therefore must be deterministic and safe to repeat; externally singleton work
belongs in the surrounding build pipeline, not a hidden runner hook.

The scheduler expands `(catalog entry x selected project)` and performs
capability preflight before queueing. Required unsupported combinations fail;
optional incompatible combinations are skipped with a reason.

Workers are runner processes/threads that own fixture scope and launch child
targets. A worker handles one attempt at a time. On target failure the next
attempt starts a fresh child; on worker corruption the scheduler replaces the
worker while preserving its stable parallel slot identity.

Default DevPULSE lanes:

- pinned cloud Lavapipe: routine required semantic and visual gate;
- local hardware: developer feedback, renderer-keyed separately;
- scheduled/release hardware qualification: required on the release policy.

## Retries and classification

Each retry is a complete new attempt with a new target process, run directory,
fixture lease, and input state. The source scenario and project stay fixed.

Final outcomes:

- `passed`: first attempt passed;
- `flaky`: an earlier attempt failed and a later attempt passed;
- `expected-failed`: an `xfail` case failed as declared;
- `unexpected-passed`: an `xfail` case passed and requires attention;
- `failed`: all allowed attempts failed with a test failure;
- `infrastructure-failed`: the environment/runner prevented a valid result;
- `skipped`: declared condition/capability excluded execution;
- `invalid`: discovery/config/authoring failed.

Non-retryable by default: invalid scenario/config, protocol major mismatch,
missing required capability, duplicate IDs, renderer policy violation, missing
baseline, and deterministic fixture schema errors. Retryable by default:
assertion/action/capture timeout, target crash/disconnect, and fixture/service
transients explicitly marked retryable. CLI can reduce retries but cannot make
an invalid case valid.

Flaky remains visible and nonzero under `--fail-on-flaky`. Retry history is
never collapsed into the final pass.

## Sharding

The v1 algorithm is deliberately simple and independently computable:

1. build the fully expanded, sorted project/scenario/case catalog;
2. compute `stable_hash(project_id, scenario_id, case_id) % shard_total`;
3. run entries equal to zero-based `shard_index`;
4. write the full catalog hash and shard parameters into every report.

This is stable under worker count changes and does not require a coordinator.
Duration-aware balancing may be an opt-in future scheduler, never the default
identity rule. Merged reports reject mismatched catalog/config/source hashes.

## Reporter contract

Reporters consume typed runner events and cannot mutate execution. Required
implementations:

- concise terminal/list reporter;
- versioned JSON report with all attempts and artifact descriptors;
- JUnit XML with project/case identity, retry/flaky properties, durations, and
  failure attachments by safe relative path.

Reporter events include catalog completion, project start/finish, attempt
start/finish, step start/finish, attachment committed, stdout/stderr summary,
and run finish. Reporter failures are recorded as reporter errors and do not
rewrite a test result, though a required CI reporter failure makes the command
fail.

## CLI surface

Minimum predictable commands/options:

```text
eguipilotai test [selectors]
  --list
  --project <id>...
  --tag <tag> / --exclude-tag <tag>
  --workers <n>
  --retries <n>
  --shard <index>/<total>
  --repeat-each <n>
  --last-failed
  --fail-fast[=<n>]
  --forbid-focus
  --reporter console,json,junit
  --trace off|failure|retain-on-failure|on
  --allow-empty
```

Exit status distinguishes success, test failure/flaky policy, invalid input,
and infrastructure failure. The JSON report remains the machine authority.

## Conformance gates

- discovery ordering is stable across machines and worker counts;
- duplicate IDs, forbidden focus, invalid annotations, dependency cycles, and
  invalid configuration launch zero processes;
- parameter expansion is stable, and every repetition has isolated evidence;
- skip/fixme/xfail/unexpected-pass classification is preserved in every
  reporter;
- nested step groups preserve source, deadline, and causal failure evidence;
- timeout precedence and teardown rescue budgets are proved with a fake clock;
- assertion samples re-resolve and report concise state changes;
- fixture DAG setup/teardown order and failure combinations are exhaustive;
- every retry receives a unique process, data root, and run directory;
- shard union equals the catalog and shard intersection is empty;
- merged JSON/JUnit preserves every attempt and flaky classification;
- fail-fast stops queueing new work but allows active attempts to clean up;
- project dependencies run once per selected dependency project and propagate
  causal failure/skip without leaking mutable UI state;
- renderer/project capability mismatches follow required/optional policy;
- the same scenario outcome and artifact manifest result from CLI, Test Lab,
  and MCP-triggered runner paths.
