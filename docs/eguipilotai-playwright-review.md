# EguiPilotAI Playwright capability review

Reviewed 2026-07-22 against the official Playwright documentation. Playwright
is the behavioral reference for reliable automation; its browser/DOM surface
is not an API to copy into egui.

The broad adopt/adapt/reject/later decision is in
`docs/research/eguipilotai/playwright-capability-catalog.md`. The deeper
engineering contracts and local source census are indexed at
`docs/research/eguipilotai/README.md`. The dependency-ordered execution plan is
`docs/plans/eguipilotai-v1-delivery.md`.

Status words are strict:

- **Ready** means a real DevPULSE process has exercised the capability.
- **Core** means reusable code and unit/conformance tests exist, but the whole
  operator flow is not yet qualified.
- **Partial** means types, fields, or UI placeholders exist without the full
  vertical behavior.
- **Missing** means no dependable product contract exists yet.
- **Translate** means the browser feature needs a native equivalent rather
  than direct parity.

## Capability matrix

| Playwright contract | Native EguiPilotAI contract | Status now | What remains | Evidence |
|---|---|---:|---|---|
| Fresh, strict locators | Resolve `PilotId` or role/name/text/within against the latest semantic frame | Core | Exact/contains/regex options, label/description/state filters, chained/scoped queries, locator diagnostics and picker | test:`runtime::strict_locator_rejects_ambiguity` |
| Auto-waiting and actionability | Unique, visible through clip chain, stable for two frames, enabled/editable, hit-testable | Core | Per-action check matrix, actionable timeout diagnostics, working auto-scroll adapters, trial mode | slice:`3` |
| Real input actions | egui pointer/key/text microframes | Ready for shell hover/click path | Check/uncheck/select, focus/blur, key-down/up, granular mouse, configurable drag paths and keyboard drag alternatives | test:`v1_compat::frozen_shell_flow_matches_the_real_migration_evidence` |
| Retrying assertions | Semantic expectation polling | Core | Timeout per expectation, contains/exact/regex, checked/selected/editable/in-viewport/accessibility matchers, negation, soft assertions, custom messages | slice:`3` |
| Accessibility/ARIA snapshots | Pilot semantic frame, real AccessKit tree, parity report, and pruned agent snapshot | Partial | Current code records rich pilot metadata separately but only basic `WidgetInfo` into AccessKit and retains only an update-present boolean; implement real TreeUpdate capture/reconstruction, complete projection, ActionRequest tests, snapshot matchers, and keyboard traversal | slice:`2` |
| Test isolation | One separately launched target process and run directory | Ready | Explicit clean fixture lifecycle and guaranteed temporary data roots | test:`v1_compat::frozen_shell_flow_matches_the_real_migration_evidence` |
| Fixtures | Trusted named argument/environment bundles | Partial | Typed setup/use/teardown, composition/dependencies, worker versus scenario scope, attachments, deterministic service seams | slice:`4` |
| Projects and emulation | Named profile/project with renderer policy | Partial | Actually execute project matrices; window size, DPI, theme, locale, timezone, font pack, input mode and fixture parameters | test:`runner::required_adapter_policies_never_cross_families` |
| Parallel workers, sharding, retries | Unsupported settings fail configuration validation | Partial | Scheduler, deterministic shard algorithm, retry isolation, flaky classification and mergeable reports | test:`runner::unsupported_scheduler_settings_fail_configuration_validation` |
| Test discovery/filtering | Direct scenario-path execution | Missing | Catalog/list, project/tag/ID filters, fail-fast, last-failed and stable ordering | slice:`4` |
| Annotations, tags and expected failures | Stable catalog metadata and explicit result policy | Missing | Conditional skip, expected-failure/unexpected-pass, fixme with required reason/issue, and report-visible annotations; no hidden timeout mutation | slice:`4` |
| Named/nested steps and hooks | Typed step groups plus fixture lifecycles | Partial | Nested groups with source/deadline/evidence, reusable Rust flow fragments, and boxed failures; fixtures replace arbitrary setup/teardown hooks | slice:`4` |
| Parameterized and repeated tests | Stable author-supplied `CaseId` plus repeat ordinal | Missing | Rust/TOML case expansion, collision validation, `--repeat-each`, per-repeat isolation, and reports that preserve every repetition | slice:`4` |
| Project/setup dependencies | Visible dependency scenarios and fixture DAGs | Missing | Acyclic project dependencies with normal traces/retries/reports; immutable content-addressed outputs only, never opaque global setup | slice:`4` |
| Timeout hierarchy | Profile-wide request timeout | Partial | Run/action/expect/fixture timeouts with clear precedence and timeout evidence | slice:`3` |
| Reporters | Run manifest, trace JSONL and stderr | Partial | Reporter trait plus console, JSON and JUnit outputs; final pass/fail/flaky/skip summary | slice:`4` |
| Trace viewer/UI Mode | Raw framed trace and passive Test Lab preview | Partial | Structured step timeline, before/after semantic revisions, input batches, attachments, locator/actionability panes, replay-to-step | slice:`5` |
| Screenshots and visual comparison | Stable capture and renderer key; approval workflow unsupported | Core | Wire comparison/approval into runner and Test Lab; masks/region capture; update policy; expected/actual/diff presentation | slice:`5` |
| Clock control | `PilotClock` and natural/fast-forward/suppressed motion | Core | Scenario-level policy, explicit clock actions, all-policy DevPULSE scenarios, uncontrolled wall-clock inventory | test:`egui_pilot_ai::suppression_rejects_uncontrolled_ambient_motion` |
| Code generator and locator picker | Experimental TOML normalizer; recorder unsupported | Partial | Record successful exploration, choose stable locators, generate assertions, review diff, export valid Rust/TOML | slice:`6` |
| Multiple pages/tabs | Typed native viewport lifetimes; qualified PixVix and deferred Chat focus/input/minimize/restore/close/reopen flows | Partial | Richer selector filters, stress gates, and a reviewed child-pixel path remain; eframe 0.35 native wgpu immediate and deferred child capture are explicitly unsupported | slice:`2B` |
| Attachments, uploads and downloads | Run artifact directory | Translate | Typed attachment API and fixture-owned jailed file sandbox; no unrestricted paths | slice:`4` |
| Authentication/storage state | Seeded application fixture | Translate | Typed deterministic app-state fixture and reusable setup dependency | slice:`4` |
| Network/HAR/API mocking | Application service seam | Translate | Named typed fake/record/replay adapters where DevPULSE has external services; no generic packet interception | slice:`4` |
| Dialogs and popups | Modal/viewport semantics | Translate | Modal roles, blocking-state diagnostics and typed accept/dismiss actions only where the app exposes them | slice:`3` |
| Device/browser emulation | Native project matrix | Translate | Window/DPI/theme/locale/input profiles rather than browser user-agent or DOM emulation | slice:`4` |
| Browser eval/CSS/DOM | No native equivalent | Intentionally absent | Use semantic metadata, typed probes, unit tests or fixture adapters; never add arbitrary code execution | slice:`n/a` |

## Slice 0/1 truthfulness ledger

These currently look more complete than they are:

1. **Closed in Slice 1:** protocol v2 uses typed command, response, and runtime
   event envelopes. The runner routes by `RequestId`, accepts out-of-order
   responses, validates monotonic `EventId` values, and never treats an event
   as a terminal response.
2. `NodeMetadata` is richer than the `WidgetInfo::labeled` data currently sent
   to egui/AccessKit, and the plugin does not retain the real `TreeUpdate`.
   Pilot semantics must not be reported as verified AccessKit semantics.
3. **Explicitly unsupported:** `Action::SetValue` and
   `Action::ScrollIntoView` remain schema-compatible but fail capability
   preflight before a run starts.
4. **Explicitly unsupported:** `Expectation::Screenshot` remains
   schema-compatible; visual comparison belongs to Slice 5.
5. **Explicitly unsupported:** non-default `Project.workers` and
   `Project.retries` fail configuration validation until the scheduler exists.
6. **Explicitly experimental:** `codegen` parses and rewrites TOML; CLI help
   and output state that it is a normalizer, not a recorder.
7. **Explicitly passive:** Test Lab labels live control, replay, and visual
   approval unsupported.

## Ordered backlog

### P0 — dependable runner

1. Protocol v2 with correlated responses, independent runtime events,
   cancellation, capability negotiation, and bounded event pressure.
2. Stateful input-source/tick executor with release-all cleanup.
3. Complete pilot-to-AccessKit projection, actual TreeUpdate reconstruction,
   parity diagnostics, and accessibility action tests.
4. Test catalog, selection, tags and deterministic ordering.
5. Explicit annotations, stable parameterized cases, nested step groups, and
   repeat-each behavior.
6. Run/action/expect/fixture timeout hierarchy.
7. Complete semantic/accessibility assertion vocabulary and failure messages.
8. Project scheduler with visible dependency scenarios, process-per-test
   isolation, retries, shards and
   flaky classification.
9. Reporter trait with console, JSON and JUnit implementations.
10. Structured trace schema with step spans, input ticks, semantic/AccessKit deltas,
   busy/motion events, logs and attachments.
11. Fixture lifecycle and fixture-owned temporary data roots.
12. Runner-integrated semantic, accessibility and visual snapshot comparison.

### P1 — DevPULSE qualification

1. Instrument every interactive template with owner-declared IDs and state.
2. Keyboard traversal and keyboard-only Drag Match.
3. Configurable drag trajectories and auto-scroll.
4. Slice 2B native viewport registry plus multi-viewport PixVix flows.
5. Named document/data-root probes for save/import/index assertions.
6. Real Test Lab timeline, actionability explanation, highlight and visual
   candidate review.
7. Performance and 10,000-frame memory gates.

### P2 — agent ergonomics and future targets

1. Exploration recorder, locator picker and reviewed codegen.
2. Property/fuzz case generation layered on stable explicit case identities.
3. Touch/swipe/pinch vocabulary where DevPULSE actually supports it.
4. WASM browser transport/runtime adapter after native publication.

## Source references

- [Locators](https://playwright.dev/docs/locators)
- [Auto-waiting and actionability](https://playwright.dev/docs/actionability)
- [Assertions](https://playwright.dev/docs/test-assertions)
- [Fixtures](https://playwright.dev/docs/test-fixtures)
- [Projects](https://playwright.dev/docs/test-projects)
- [Annotations and tags](https://playwright.dev/docs/test-annotations)
- [Test steps](https://playwright.dev/docs/api/class-test#test-step)
- [Parameterized tests](https://playwright.dev/docs/test-parameterize)
- [Project dependencies and global setup](https://playwright.dev/docs/test-global-setup-teardown)
- [Configuration](https://playwright.dev/docs/test-configuration)
- [Retries](https://playwright.dev/docs/test-retries)
- [Sharding](https://playwright.dev/docs/test-sharding)
- [Reporters](https://playwright.dev/docs/test-reporters)
- [Trace viewer](https://playwright.dev/docs/trace-viewer-intro)
- [Visual comparisons](https://playwright.dev/docs/test-snapshots)
- [ARIA snapshots](https://playwright.dev/docs/aria-snapshots)
- [Code generation](https://playwright.dev/docs/codegen-intro)
- [Clock](https://playwright.dev/docs/clock)
- [Network interception](https://playwright.dev/docs/network)
