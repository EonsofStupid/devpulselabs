# Traces, artifacts, replay, and visual approval

Status: specified, not implemented. The existing JSONL command transcript and
capture primitives are useful foundations, but they are not yet a structured
trace, deterministic replay system, or visual approval workflow.

## Evidence principles

1. The run directory is immutable evidence for one attempt.
2. Structured metadata is the authority; UI presentation is a projection.
3. Large bytes are content-addressed attachments, never inline protocol blobs.
4. A trace records what the runner and target observed, not reconstructed
   stories.
5. Replay means relaunching a declared fixture and re-executing recorded plans;
   it does not pretend an arbitrary native process can be rewound.
6. Visual approval is additive. Existing approved bytes and records are never
   overwritten or deleted by the tool.

## Attempt directory

```text
target/eguipilotai/runs/<run-id>/
  manifest.json
  result.json
  trace.jsonl
  stderr.log
  attachments/
    <sha256>.<ext>
  semantic/
    <revision>.json.zst
  accessibility/
    <revision>.json.zst
  visual/
    actual-<step>.png
    expected-<step>.png
    diff-<step>.png
```

Every path stored in JSON is relative to the run directory. Artifact writes
use create-new semantics, size limits, and a final hash check. Symlink/reparse
escapes and `..` components are rejected.

`manifest.json` is written before target launch and finalized only by adding a
separate completion record; fields are not silently rewritten. It contains:

- schema/protocol/tool versions;
- source workspace identity without secrets;
- catalog, scenario source, project, profile, fixture, and config hashes;
- run/attempt IDs and retry ordinal;
- selected timeout and trace policies;
- executable/build identity and launch argument names with secret values
  redacted;
- OS, renderer, DPI, font, locale, timezone, viewport, motion, and visual
  identities;
- shard and worker slot identity.

`result.json` contains final classification, timing, step results, diagnostics,
attachments, cleanup/teardown outcomes, and links to prior retry attempts.

## Structured trace envelope

One JSON object per line:

```text
TraceEvent {
  schema_version,
  event_id,
  monotonic_ns,
  virtual_time_seconds?,
  session_id,
  run_id?, attempt_id?, step_id?, viewport_id?,
  kind,
  payload
}
```

`event_id` follows the protocol event order. Runner-only records use a
separate namespaced sequence and state their causal runtime event/request ID.
Wall-clock timestamps are metadata and never used for deterministic ordering.

Required event families:

- lifecycle: fixture, process, handshake, viewport, run, step, teardown;
- protocol: command sent, response received, event received, pressure/drop;
- locator: query, candidates, strict result, pinned revision;
- actionability: each check, auto-scroll attempt, final decision;
- input: compiled sequence, tick, resolved coordinates, state before/after,
  cleanup;
- assertion: matcher, distinct samples, terminal result;
- semantics: pilot revision/delta descriptor, AccessKit update descriptor,
  parity result;
- waits: settlement hashes, busy/motion transitions, timeout progress;
- capture: preflight, renderer identity, stable hashes, attachment;
- visual: baseline lookup, comparison metrics, candidate/diff descriptors;
- adapter/probe: typed ID, safe input/output hashes, result;
- diagnostic/log/crash/panic/disconnect.

Default traces do not include every unchanged frame. Repeated assertion
samples and settlement frames coalesce until their observed state/hash
changes. Full debug tracing is a named project policy with an explicit size
limit.

## Semantic revision storage

The target keeps full pilot and reconstructed AccessKit state. The trace stores
snapshots at declared checkpoints plus atomic deltas between them:

- before and after every action;
- every terminal assertion result;
- every diagnostic transition;
- capture preflight and committed capture;
- viewport creation/destruction;
- run failure.

A delta declares its base and result revision/hash. The artifact reader rejects
a missing base or mismatched result. Periodic full checkpoints bound replay
cost. Agent snapshots are separate pruned projections and are not used to
reconstruct the full tree.

## Attachment API

An attachment descriptor includes typed kind, MIME type, byte length, SHA-256,
safe relative path, producer, run/step scope, and optional human label.
Required kinds include screenshot, expected image, diff image, semantic tree,
AccessKit tree, stdout/stderr, fixture probe, crash report, and exported
scenario.

Attachment creation is a two-phase commit: write into a run-local temporary
file, validate size/hash/type, atomically rename to the content-addressed name,
then emit `attachment.committed`. An incomplete file is never referenced.

An attempt-level screenshot policy is separate from visual assertions:
`Off`, `On`, `Failure`, or `FirstFailure`. Automatic screenshots pass through
the same deterministic capture preflight and attachment API, but do not look
up or update a baseline. `Failure` retains every failed retry's image;
`FirstFailure` retains only the first failed attempt. The effective policy is
written to the manifest before launch.

## Replay contract

There are two distinct replay products:

### Evidence replay

Test Lab reads the immutable trace, semantic revisions, logs, and images. It
can move through steps and frames without launching the app. This is a viewer,
not behavioral re-execution.

### Deterministic live replay

The runner verifies source/config/fixture/project compatibility, prepares a
fresh fixture lease, launches a fresh target, and replays the recorded
high-level steps or compiled input ticks to a selected checkpoint.

Default replay uses high-level actions and re-resolves locators. Exact-input
replay is a diagnostic mode that uses recorded tick coordinates only when
viewport, DPI, renderer class, source hash, and semantic checkpoint all match.
It never becomes the durable test source.

At every checkpoint replay compares semantic/accessibility hashes. Divergence
stops with the first differing revision and evidence. A replay cannot skip
fixture setup, process launch, or earlier state-producing steps unless the
fixture explicitly provides a content-addressed checkpoint restore adapter.

## Visual assertion model

A durable visual expectation declares:

- stable baseline name;
- viewport and optional semantic region locator;
- motion policy;
- renderer/project key requirements;
- exact or threshold comparison policy;
- optional masks named by semantic locator;
- maximum allowed differing pixels/ratio and per-channel delta for threshold
  mode;
- capture timeout.

Capture flow:

1. resolve region/masks against the latest revision;
2. run deterministic motion preflight;
3. render until two consecutive semantic, geometry, and pixel hashes match;
4. persist the actual image and metadata;
5. resolve exactly one approved baseline for the full renderer key;
6. compare with dimensions/color space checked first;
7. persist expected and diff evidence on failure;
8. return one retrying visual assertion result.

Pinned Lavapipe defaults to byte-exact PNG pixel content after deterministic
encoding. Hardware projects use separately approved tight thresholds. A
cross-renderer image is informational and can never satisfy a required
baseline.

Masks are semantic: the runner records the locator, resolved node IDs,
revision, and final rectangles. Arbitrary pixel rectangles in durable files
are rejected unless the scenario is explicitly viewport-geometry testing.

## Baseline storage and approval

```text
tests/ui/baselines/
  objects/<sha256>.png
  approvals.toml
```

An approval entry maps `(project, renderer_key, baseline_name, capture_spec`
`hash)` to a content hash plus approval metadata. Approval adds the object if
absent and appends a new record. Superseding a baseline marks the earlier
record as superseded; it does not alter or remove it.

Approval preconditions:

- run and capture completed without uncontrolled motion;
- renderer policy and key are valid;
- actual attachment hash and capture specification are known;
- candidate is inside the run jail;
- reviewer explicitly selects the project/renderer row;
- Test Lab displays actual, previous expected, diff, semantic region, and
  renderer identity together.

CI never approves. `--update-baselines` may create candidates but does not
change the approval manifest without a separate explicit approval command/UI
action.

## Failure bundle policy

On failure, retain:

- manifest/result and complete required trace;
- final relevant pilot and AccessKit snapshots plus parity report;
- locator/actionability/expectation evidence;
- last screenshot for UI failures and expected/diff for visuals;
- stderr and crash/panic data;
- fixture probe/teardown results;
- renderer, busy, motion, viewport, and input cleanup state.

On pass, the default keeps manifest/result, compact trace, and referenced
approved capture hashes. Project policy may retain all or delete unreferenced
pass attachments after reporting. Approved objects are never subject to run
retention.

## Test Lab views derived from this model

- run/project/attempt tree with final classification;
- step timeline with correlated protocol, locator, actionability, input,
  assertion, and settlement records;
- before/after pilot and AccessKit trees with parity differences;
- viewport selector and node highlight using recorded geometry;
- expected/actual/diff visual review and additive approval;
- live replay launch to a selected step with divergence reporting.

The Test Lab invokes the runner APIs. It does not reimplement file parsing,
baseline mutation, or target control.

## Conformance gates

- trace readers reject gaps, invalid causal references, oversized payloads,
  and delta hash mismatches;
- an interrupted attachment is never referenced;
- every failure class produces the required minimal bundle;
- evidence replay renders without a target process;
- live replay reaches the same semantic checkpoints for a deterministic
  fixture and reports first divergence after an intentional mutation;
- exact Lavapipe captures remain byte-identical across fifty runs;
- hardware thresholds are isolated by renderer key;
- missing, ambiguous, wrong-renderer, and dimension-mismatched baselines have
  distinct errors;
- approval only appends objects/records and survives interrupted writes;
- JSON and JUnit reports link only to jailed, existing attachments;
- Test Lab and CLI use the same artifact and approval libraries.
