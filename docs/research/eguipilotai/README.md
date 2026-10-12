# EguiPilotAI engineering research

Status: evidence and design input for the canonical architecture. These notes
do not claim implementation. A capability is complete only when its vertical
slice passes both toolkit conformance and a real DevPULSE process.

This research exists to keep EguiPilotAI boring at the point of use. DevPULSE
authors should declare identity and semantics beside a real widget, choose a
named fixture, and write an Arrange -> Act -> Assert -> Capture scenario. The
complexity required to make that path reliable belongs in the toolkit.

## Read order

1. [`playwright-capability-catalog.md`](playwright-capability-catalog.md)
   records the adopt/adapt/reject/later decision for the broad Playwright
   surface and fixes the allowed DevPULSE extension seams.
2. [`protocol-input.md`](protocol-input.md) defines the wire lifecycle and the
   stateful input model. Slice 1 replaced the v1 lockstep response/event
   conflation before retries, traces, or multi-viewport work can depend on it.
3. [`multi-viewport.md`](multi-viewport.md) audits the current node-tagging
   partial implementation and defines stable native window identity, lifetime,
   targeting, capture, close, and immediate/deferred qualification.
4. [`accessibility-semantics.md`](accessibility-semantics.md) defines one
   semantic registration projected into both pilot and real AccessKit output.
5. [`runner-contract.md`](runner-contract.md) defines discovery, selection,
   timeouts, assertions, fixtures, scheduling, retries, sharding, and reports.
6. [`traces-artifacts-visuals.md`](traces-artifacts-visuals.md) defines the
   evidence model, deterministic replay, and immutable visual approval.
7. [`docs-history-2026-10-12:docs/research/eguipilotai/devpulse-census.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/research/eguipilotai/devpulse-census.md) maps the live application onto
   those contracts and records the actual instrumentation gap.
8. [`../../plans/eguipilotai-v1-delivery.md`](../../plans/eguipilotai-v1-delivery.md)
   converts the findings into dependency-ordered implementation slices.
9. [`docs-history-2026-10-12:docs/research/eguipilotai/slice-0-shell-baseline.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/research/eguipilotai/slice-0-shell-baseline.md) freezes the first
   native DevPULSE launch evidence without misrepresenting it as a frame-time
   or long-run memory qualification result.
10. [`docs-history-2026-10-12:docs/research/eguipilotai/slice-1-protocol-v2-evidence.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/research/eguipilotai/slice-1-protocol-v2-evidence.md)
   records the frozen v1 evidence, protocol v2 conformance, real child-process
   proof, and the exact frontier left for Slice 2.

## Evidence hierarchy

The design uses primary sources and local source code:

- Playwright documentation for mature user-facing automation behavior:
  locators, actionability, retrying assertions, fixtures, projects, workers,
  annotations, named steps, parameterization, setup dependencies, retries,
  sharding, reports, traces, snapshots, code generation, and clocks.
- W3C WebDriver BiDi for correlated asynchronous commands and independent
  events.
- W3C WebDriver Actions for stateful input sources, synchronized ticks, and
  release semantics.
- AccessKit `TreeUpdate` and `ActionRequest` contracts.
- egui 0.35 source for plugin hooks, `RawInput`, `WidgetInfo`, AccessKit node
  construction, accessibility action input, screenshots, and animation state.
- the current DevPULSE and EguiPilotAI source tree, not screenshots or plan
  prose, for implementation status.

## Adopt, adapt, reject

| Category | Decision |
|---|---|
| Strict fresh locators, actionability, retrying assertions, isolated attempts, fixture lifecycles, project matrices, structured evidence | Adopt as behavioral contracts |
| Browser pages, storage state, downloads, network routes, device emulation | Adapt into viewports, typed fixtures/adapters, jailed artifacts, and native project profiles |
| CSS/XPath, DOM evaluation, arbitrary shell/environment/path input, generic network interception | Reject from the generic protocol |

## Status language

- **Implemented**: the reusable behavior exists and is exercised through a
  real target process.
- **Core-only**: reusable behavior has focused tests but lacks a qualified
  operator path.
- **Partial**: a type, field, placeholder, or narrow happy path exists.
- **Specified**: this research defines a contract but no implementation is
  implied.
- **Missing**: there is no dependable contract or vertical behavior.

## Primary references

- [Playwright locators](https://playwright.dev/docs/locators)
- [Playwright actionability](https://playwright.dev/docs/actionability)
- [Playwright assertions](https://playwright.dev/docs/test-assertions)
- [Playwright fixtures](https://playwright.dev/docs/test-fixtures)
- [Playwright projects](https://playwright.dev/docs/test-projects)
- [Playwright annotations and tags](https://playwright.dev/docs/test-annotations)
- [Playwright test steps](https://playwright.dev/docs/api/class-test#test-step)
- [Playwright parameterization](https://playwright.dev/docs/test-parameterize)
- [Playwright project dependencies](https://playwright.dev/docs/test-global-setup-teardown)
- [Playwright configuration](https://playwright.dev/docs/test-configuration)
- [Playwright timeouts](https://playwright.dev/docs/test-timeouts)
- [Playwright parallelism](https://playwright.dev/docs/test-parallel)
- [Playwright retries](https://playwright.dev/docs/test-retries)
- [Playwright sharding](https://playwright.dev/docs/test-sharding)
- [Playwright reporters](https://playwright.dev/docs/test-reporters)
- [Playwright trace viewer](https://playwright.dev/docs/trace-viewer-intro)
- [Playwright visual comparisons](https://playwright.dev/docs/test-snapshots)
- [Playwright ARIA snapshots](https://playwright.dev/docs/aria-snapshots)
- [Playwright pages and popups](https://playwright.dev/docs/pages)
- [W3C WebDriver BiDi](https://www.w3.org/TR/webdriver-bidi/)
- [W3C WebDriver](https://www.w3.org/TR/webdriver/)
- [AccessKit `TreeUpdate`](https://docs.rs/accesskit/latest/accesskit/struct.TreeUpdate.html)
- [AccessKit `ActionRequest`](https://docs.rs/accesskit/latest/accesskit/struct.ActionRequest.html)
- [egui `Context`](https://docs.rs/egui/latest/egui/struct.Context.html)
- [egui `RawInput`](https://docs.rs/egui/latest/egui/struct.RawInput.html)
- [egui `FullOutput`](https://docs.rs/egui/latest/egui/struct.FullOutput.html)
- [egui `ViewportInfo`](https://docs.rs/egui/latest/egui/struct.ViewportInfo.html)
