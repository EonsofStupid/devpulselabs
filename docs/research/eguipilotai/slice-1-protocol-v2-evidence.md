# Slice 1 protocol v2 qualification evidence

Recorded 2026-07-22 after the Playwright/W3C review and before Slice 2 input
executor work. This document freezes what Slice 0/1 actually proves. It does
not promote later scheduler, accessibility, trace replay, visual approval, or
Test Lab control work to implemented status.

## Truthfulness boundary

The capability catalog, CLI help, MCP schemas, runner configuration, passive
Test Lab, and Workbench copy now agree that these surfaces are unsupported:

- set-value and scroll-into-view actions;
- screenshot expectations and visual approval;
- parallel workers and retries;
- recorder codegen, deterministic replay, and live Test Lab control.

Unsupported flow capabilities fail during runtime preflight, before a
`run-started` event or any input is produced. Workers other than one and
non-zero retries fail runner configuration validation rather than being
silently ignored.

## Compatibility evidence

The removed v1 behavior is preserved only as fixtures under
`crates/egui-pilot-ai-protocol/tests/fixtures/v1/`. The compatibility test
reconstructs the original serde types, verifies every frozen JSON envelope,
and proves that the frozen shell TOML is identical to the durable project
scenario. No v1 command/event semantics remain in the target runtime.

Protocol v2 has golden command, response, and event fixtures. Its conformance
tests cover:

- typed request, run, step, event, and attachment identities;
- interleaved events and out-of-order response routing;
- one terminal response for every accepted command;
- status while a run is active and idempotent cancellation;
- queued-input cleanup on cancellation;
- lossless required events and coalesced telemetry pressure reporting;
- oversize, corrupt, partial, disconnected, unknown-variant, session, nonce,
  capability, and major-version failures.

## Real DevPULSE process proof

The Windows hardware profile launched the real devtools-enabled native
DevPULSE executable as a separate child and completed the frozen
`shell-navigation` scenario over framed stdio protocol v2.

After capture acquisition was changed from a one-shot request to a bounded,
retrying request with explicit repaint wakeups and a target-owned timeout, five
fresh processes passed consecutively. The named `test-lab` fixture pins both
boot-randomized shell moods to index 0. All five suppressed captures:

- settled after one comparison frame;
- used renderer key `windows-93c45ce20436289d`;
- produced content hash `752abbf3e8e46795`.

This is hardware smoke evidence, not the required fifty-capture Lavapipe
qualification. The CI profile performs that separate exact comparison.

## Gates passed locally

- `cargo green`: documentation evidence check, format check, warning-free
  workspace lint, workspace tests, doc tests, and production release build;
- `wasm32-unknown-unknown` checks for protocol, transport, instrumentation,
  and runtime core;
- production dependency inspection: runtime, transport, runner, and CLI/MCP
  crates are absent from the ordinary DevPULSE dependency graph;
- real devtools build plus the five-process v2 shell/capture proof above.

Docker is not installed on the recording Windows host, so the pinned Linux
container was not claimed as locally exercised. At recording time GitHub
Actions did not assign the job because the repository account reported a
billing/spending-limit condition; that is an external runner block, not a
passing or failing Linux test result.

## Next implementation frontier

Slice 2 replaces the remaining ad-hoc action event batches with a stateful
input executor and release-all guard. UI instrumentation and richer Test Lab
features remain behind that dependency boundary.
