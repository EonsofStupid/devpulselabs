# Playwright capability catalog and native disposition

Status: researched target disposition, not implementation. Reviewed 2026-07-22
against the official Playwright documentation.

The objective is Playwright-class reliability for native egui applications,
not a Rust spelling of browser APIs. Every Playwright capability receives one
of four explicit decisions:

- **Adopt**: preserve the user-visible behavioral contract.
- **Adapt**: provide a native egui equivalent through a typed boundary.
- **Reject**: the capability would add unsafe, nondeterministic, or
  browser-specific authority without a valid native contract.
- **Later**: useful, but not required to call the v1 runner dependable.

An item listed here is not implemented merely because a similarly named type
or command exists. Current status remains in
[`../../eguipilotai-playwright-review.md`](../../eguipilotai-playwright-review.md).

## Authoring and runner behavior

| Playwright surface | EguiPilotAI disposition | v1 contract |
|---|---|---|
| Test discovery and configuration | Adopt | Deterministic TOML/Rust catalog, schema validation before launch, stable IDs/source hashes, named configuration |
| File/title/tag/project filters | Adopt | Intersection-based ID/path/tag/project selection, stable order, list mode, last-failed |
| Focused tests and `forbidOnly` | Adapt | Optional local `focus` annotation; CI rejects any focused entry before launch unless an explicit non-CI policy permits it |
| Annotations (`skip`, `fail`, `fixme`, `slow`) | Adapt | Typed skip/fixme/xfail conditions and report metadata; explicit timeout profiles replace hidden `slow` multiplication |
| Parameterized tests | Adopt | Cases expand before selection and require stable author-supplied `CaseId` values |
| Named and nested steps | Adopt | Nested groups with source, deadlines, evidence policy, and causal failure chains |
| Hooks | Adapt | Fixture lifecycle, typed Rust flow fragments, and visible dependency scenarios; no arbitrary TOML callbacks |
| Per-file/per-group use options | Adapt | Hierarchical declarative defaults resolved into the catalog; final effective project/scenario values are reportable |
| Test isolation | Adopt more strictly | Fresh native process and fixture lease per attempt by default |
| Parallel workers | Adopt | One active attempt per worker, clean child replacement after corruption, deterministic scheduling inputs |
| Serial suites | Reject as shared-state mechanism | `workers=1` is allowed for resource limits; cases remain isolated and cannot depend on execution order |
| Retries and flaky classification | Adopt | Full isolated attempt per retry, all history retained, optional fail-on-flaky |
| Repeat-each | Adopt | Isolated repetition with stable case identity and separate retry history |
| Max failures/fail fast | Adopt | Stop queueing at the threshold; active work receives normal cleanup and evidence finalization |
| Sharding and report merge | Adopt | Stable hash partition over expanded project/scenario/case identity and compatibility-checked merge |
| Projects | Adopt and extend | Renderer, viewport, DPI, fonts, theme, locale, timezone, input, motion, fixtures, retries, and workers |
| Project dependencies/global setup | Adapt | Visible dependency scenarios with traces/results plus fixture DAGs; no opaque global callback or shared UI state |
| Web server lifecycle | Adapt | Target launcher plus named service fixtures with readiness probes, jailed logs, and teardown |
| Timeouts | Adopt | Scenario/action/expect/capture/fixture/shutdown hierarchy with one monotonic runner clock and explicit precedence |
| Web-first assertions | Adopt | Retry against freshly resolved semantic/AccessKit state; immediate checks only for immutable runner facts |
| Custom matchers/polling | Adapt | Registered typed matchers and fixture probes; no arbitrary code sent over MCP |
| Soft assertions | Adopt | Accumulate structured failures, continue safely, fail attempt after cleanup |
| Reporters | Adopt | Typed event stream with console, versioned JSON, JUnit, and merge support; reporters cannot mutate execution |
| Metadata and output paths | Adopt more strictly | Content-addressed attachments and runner-issued jailed paths; no unrestricted caller paths |

## Native UI control

| Playwright surface | EguiPilotAI disposition | v1 contract |
|---|---|---|
| Browser/context/page lifecycle | Adapt | Target process/session/viewport lifecycle with explicit discovery and close events |
| Navigation and URL waits | Adapt | Typed route actions and semantic route/readiness anchors; no sleep-based navigation |
| Locators | Adopt | Fresh strict resolution by `PilotId`, role/name, text, label, state, relation, ancestry, region, and viewport |
| CSS/XPath selectors | Reject | Layout and paint implementation details are not a stable public testing API |
| Frames and shadow DOM | Adapt | Viewport and meaningful semantic-region scoping; layout containers are hoisted from agent snapshots |
| Actionability | Adopt | Unique, current, visible through clip chain, stable, enabled/editable, hit-testable, correct viewport |
| Trial actions | Adopt | Run resolution/actionability/scroll planning without injecting input |
| Auto-scroll | Adapt | Standard scroll-region search plus registered adapters for virtualized/custom surfaces |
| Mouse | Adopt | Stateful pointer sources and synchronized hover/move/down/up/click/double-click/wheel ticks |
| Keyboard | Adopt | Stateful key-down/up/press/type plus release-all cleanup and real focus traversal |
| Text fill/clear | Adopt | Select/replace semantics through real egui input frames, with editable-state checks |
| Check/select/set value | Adapt | Semantic state actions compiled to real input or a registered accessible value adapter; never direct state mutation |
| Drag and drop | Adopt | Configurable pointer trajectory, intermediate frames, hit verification, and keyboard alternative where supported |
| Touchscreen, swipe, pinch | Later | Add typed touch sources only after a DevPULSE surface and conformance app exercise them |
| File upload | Adapt | Fixture-owned jailed input attachment selected by typed ID, then exercised through the real application chooser seam |
| Download | Adapt | Fixture-owned output jail plus typed completion/probe semantics and content-addressed artifact commit |
| Clipboard | Adapt | Test-owned clipboard provider with copy/paste actions and typed read probe; never host-global clipboard in CI |
| Dialogs | Adapt | Modal semantic roles, blocking diagnostics, and declared accept/dismiss actions |
| Popups/new pages | Adapt | Viewport-created events, scoped locators, lifecycle waits, and PixVix multi-viewport flows |
| Browser-side evaluation | Reject | Use semantic metadata, accessibility output, typed probes, or ordinary Rust unit tests |
| Forced actions | Narrowly adapt | Explicit diagnostic escape hatch for supported pointer actions, off by default and clearly reported; never bypass identity/currentness |

## Assertions, snapshots, and evidence

| Playwright surface | EguiPilotAI disposition | v1 contract |
|---|---|---|
| Text/state/count assertions | Adopt | Retryable typed matcher vocabulary with negation, normalization options, and concise changed samples |
| Accessibility/ARIA snapshots | Adopt and extend | Reconstructed real AccessKit snapshot, pilot semantic snapshot, parity report, and keyboard/action-request tests |
| Generic snapshots | Adapt | Versioned semantic, accessibility, typed probe, and structured-data snapshots with explicit normalization |
| Screenshot assertions | Adopt and extend | Motion preflight, viewport/region capture, masks, renderer-keyed baselines, exact/tight policy, immutable approval |
| Full-page screenshots | Adapt | Viewport capture in v1; registered scroll-surface stitching only when geometry and motion can be proven deterministic |
| Automatic failure screenshots | Adopt | Configurable off/on/failure/first-failure retention through the same capture pipeline |
| Trace recording | Adopt and extend | Typed steps, input ticks, semantic/AccessKit deltas, logs, diagnostics, motion, renderer, attachments, and causal IDs |
| Trace viewer/UI mode | Adapt | Native Test Lab timeline, evidence replay, deterministic live replay, actionability, and visual panes |
| Inspector/locator picker | Adapt | Highlight a semantic node, explain candidate ranking/actionability, and select a stable locator |
| Code generator | Adapt | Record high-level exploration, generate reviewed TOML/Rust diffs, then execute the exported scenario |
| Video | Later, non-authoritative | Optional content-addressed attempt recording with Playwright-like retention modes; structured trace and captured frames remain the v1 diagnostic authority |
| Attachments | Adopt | Typed MIME/role/hash/producer descriptors committed atomically within the run jail |
| Console/page errors | Adapt | Structured application tracing, stderr, panic, protocol, renderer, and diagnostic events |
| Coverage | Reject from UI protocol | Rust coverage belongs to the build toolchain and can be linked as an external report artifact |

## Environment, services, and emulation

| Playwright surface | EguiPilotAI disposition | v1 contract |
|---|---|---|
| Browser selection/channels | Adapt | Named renderer/backend/adapter policies with verified identity and separately keyed baselines |
| Viewport/screen/device scale | Adapt | Explicit window/viewport size and DPI project fields; host-dependent defaults are invalid for visual projects |
| Color scheme/reduced motion/contrast | Adapt | Theme and motion policy projects; add contrast mode only if DevPULSE implements it semantically |
| Locale/timezone | Adopt | Pinned project values propagated through named launch configuration and recorded in evidence |
| Fonts | Extend | Digest-pinned font pack is part of renderer identity and visual baseline key |
| Mobile/device emulation | Adapt narrowly | Native window/DPI/input matrices; no user-agent fiction or claim of physical-device equivalence |
| Geolocation, permissions, offline, proxy, TLS | Reject generically | Add a named typed application/service fixture only when DevPULSE consumes that capability |
| Authentication/storage state | Adapt | Seeded deterministic app-state fixture with schema/version/hash and isolated data root |
| API testing | Keep separate | Ordinary Rust HTTP/service tests own API correctness; runner fixture probes may prepare or verify state around a UI flow |
| Network interception | Reject generically | Instrument application service traits with named fake/record/replay adapters at dependency-injection seams |
| HAR and WebSocket mocking | Adapt only at service seam | Typed application-domain recordings with schema/version/redaction; never generic packet control in MCP |
| Clock | Adopt and extend | Runner monotonic deadlines plus application virtual time, finite/ambient motion policy, and capture canonicalization |
| Browser API mocks | Adapt only when real | Named injected platform-service adapters, visible in project/trace, with no runtime code injection |

## Component and lower-level testing

Playwright component testing is experimental and mounts components into a
browser facade. EguiPilotAI will not make isolated widget mounting the primary
truth, because many egui defects depend on the real application frame,
viewport, renderer, focus graph, and surrounding layout.

The native translation is a small, publishable conformance target that mounts
purpose-built widgets and custom interactions inside a real egui/wgpu process.
It proves toolkit behavior such as clipping, scrolling, drag trajectories,
multi-DPI hit testing, AccessKit updates, motion, and input cleanup. DevPULSE
acceptance still runs the real DevPULSE executable. Project-specific widgets
may use a similar harness for fast diagnosis, but passing it never replaces a
real-application scenario.

## Deliberately small extension surface

DevPULSE should customize the toolkit only through these stable seams:

| Seam | Owns | Must not own |
|---|---|---|
| `UiPilotExt` / `pilot_track` plus motion registration beside authorship | Identity, semantics, actions, bounds, focus, motion association | Test assertions or runner lifecycle |
| `ScrollAdapter` | Deterministic reveal plan for a registered custom/virtual surface | Arbitrary state mutation |
| `ValueAdapter` | Accessible value/action translation for a custom control | Direct test-only model access |
| `MotionAdapter` | Terminal/canonical phase and status for custom motion | Wall-clock sleeps or screenshot comparison |
| `FixtureProvider` / `FixtureLease` | Trusted setup, jailed roots, launch overlay, probes, teardown | Raw MCP paths, commands, or environment authority |
| `ServiceAdapter` | Named fake/record/replay application dependency | Generic network interception |
| `Reporter` | Read-only typed runner events | Scheduling or result mutation |
| `VisualComparator` | Registered algorithm and policy version | Baseline selection, renderer identity, or approval mutation |

New DevPULSE features first compose these seams. A new generic extension point
requires two independent use cases, a bounded authority model, conformance
tests, and a real DevPULSE scenario. A one-off application need stays in a
project fixture or typed probe rather than widening the protocol.

## Claim gate for every catalog row

A row moves to implemented only when all applicable layers agree:

1. public types and unsupported-path behavior;
2. protocol/runtime behavior and capability negotiation;
3. runner execution and structured evidence;
4. conformance target coverage;
5. a real DevPULSE child-process scenario;
6. CLI and, where exposed, MCP/Test Lab projection;
7. production dependency/endpoint audit.

## Primary references

- [Playwright configuration](https://playwright.dev/docs/test-configuration)
- [Playwright command line](https://playwright.dev/docs/test-cli)
- [Playwright annotations](https://playwright.dev/docs/test-annotations)
- [Playwright parameterization](https://playwright.dev/docs/test-parameterize)
- [Playwright projects](https://playwright.dev/docs/test-projects)
- [Playwright project dependencies](https://playwright.dev/docs/test-global-setup-teardown)
- [Playwright fixtures](https://playwright.dev/docs/test-fixtures)
- [Playwright actionability](https://playwright.dev/docs/actionability)
- [Playwright assertions](https://playwright.dev/docs/test-assertions)
- [Playwright test options](https://playwright.dev/docs/api/class-testoptions)
- [Playwright API testing](https://playwright.dev/docs/api-testing)
- [Playwright API mocking](https://playwright.dev/docs/mock)
- [Playwright videos](https://playwright.dev/docs/videos)
- [Playwright component testing](https://playwright.dev/docs/test-components)
