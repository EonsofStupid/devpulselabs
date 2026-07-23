# EguiPilotAI native multi-viewport contract

Status: specified from a source audit on 2026-07-23. The immediate-window
discovery/input/focus/close vertical is implemented and locally qualified
against the real PixVix process. Generic multi-viewport support is not complete:
deferred viewports are unimplemented, immediate child screenshots are blocked
by an explicit eframe 0.35 wgpu limitation, and the remaining conformance gates
below are still open.

“Extended window”, “second window”, and “multi-window” mean an additional
native egui viewport owned by the same target process and EguiPilotAI session.
They do not mean controlling unrelated OS windows, faking global input, or
copying Playwright's browser `Page` API.

DevPULSE has two qualification targets:

- PixVix's existing immediate `pixvix` and `pixvix-region` viewports;
- the planned deferred presenter/teleprompter viewport.

The toolkit must support both egui viewport classes before calling its native
multi-window contract reusable.

## Primary behavior references

- egui 0.35 `Context::show_viewport_immediate` requires the child to be
  declared on every parent pass and couples child repaint to the parent.
- egui 0.35 `Context::show_viewport_deferred` owns an independently repainting
  callback and requires shared `Send + Sync` state.
- `RawInput.viewports` carries information about all viewports while
  `RawInput.viewport_id` identifies the active pass.
- `ViewportInfo` carries parent, title, close request, logical geometry, native
  pixels-per-point, focus, minimization, maximization, and occlusion. Outer
  position is legitimately unavailable on Wayland and some other targets.
- `Context::send_viewport_cmd_to` targets screenshot, close, focus, resize,
  position, and visibility commands to one egui viewport.
- eframe removes native child viewports that are not present in the pass
  output. A close request does not mutate application ownership; the
  application must stop declaring the child.
- eframe 0.35's native wgpu `render_immediate_viewport` path passes an empty
  screenshot-request list to the painter and does not poll completed
  screenshots into that child pass. EguiPilotAI therefore reports immediate
  child capture as unsupported instead of timing out or returning root pixels.
- Playwright's useful transferable contract is not its browser object model.
  It is event-backed page discovery, obtaining the new page before interacting
  with it, and keeping actions scoped to one page.

Exact local sources:

- `egui-0.35.0/src/context.rs`
- `egui-0.35.0/src/data/input/raw_input.rs`
- `egui-0.35.0/src/data/input/viewport_info.rs`
- `egui-0.35.0/src/viewport.rs`
- `eframe-0.35.0/src/native/*_integration.rs`

External references:

- [Playwright pages and popups](https://playwright.dev/docs/pages)
- [egui `Context`](https://docs.rs/egui/0.35.0/egui/struct.Context.html)
- [egui `ViewportInfo`](https://docs.rs/egui/0.35.0/egui/struct.ViewportInfo.html)

## Implementation checkpoint

| Layer | Evidence now | Status | Remaining behavior |
|---|---|---:|---|
| Protocol | Typed durable keys, generated lifetime IDs, descriptors, strict key/ID selectors, lifecycle expectations/events, viewport fields, granular capability catalog | Implemented core | Title/class/parent/state selector filters, exact-count expectation, repeated-key policy, additional schema goldens |
| Instrumentation | Automatic root, typed immediate declaration wrapper, authoritative inventory diff, generation counter, stale semantic/input/capture pruning | Implemented immediate | Deferred wrapper, source-location duplicate diagnostics, AccessKit tree-per-lifetime work in Slice 3 |
| Runtime input | Viewport-bound pointer/keyboard state, source-switch rejection, cleanup in original viewport, target repaint, focus/close waits | Implemented core | Cross-window drag remains intentionally unsupported; deferred independent-repaint proof remains |
| Runtime capture | Strict target resolution, screenshot-event matching, viewport-scoped hashes and artifact metadata | Root only | Immediate child capture is explicitly unsupported by the eframe 0.35 wgpu path; deferred capture is unqualified |
| Runtime stop | Child close requests, bounded closure wait, lifecycle forwarding, root/transport close after child teardown | Implemented core | Deep parent ordering and forced-timeout real-process fixture |
| Runner/CLI/MCP | Durable TOML viewport actions/expectations, compact live inventory, viewport-scoped capture schema, failed CLI scenarios return non-zero | Implemented core | Catalog discovery, stale-instance negative flow, richer selector authoring, report/Test Lab projection |
| DevPULSE | Stable PixVix palette/region keys and controls; real discover/focus/click/open/close scenario | Qualified on Windows | Reopen through the root launcher, same-window region drag, deferred presenter proof, child visual evidence |
| Conformance | Registry reopen-generation unit test, input ownership tests, stop-waits-for-child test, real PixVix process flow | Partial | Deferred fixture, 100-cycle/memory gate, Lavapipe qualification, renderer-supported child capture |

The current boundary is deliberately fail-closed:

1. immediate child capture returns `viewport-capture-unsupported` immediately;
2. deferred capture returns `viewport-capture-unqualified`;
3. a required native operation rejects embedded fallback;
4. ambiguous omitted viewport selectors fail when more than one lifetime is
   live;
5. failed CLI scenarios return a non-zero process exit code, so local gates and
   CI observe the same failure.

## Identity and lifetime model

Two identities are required:

```text
PilotViewportKey       durable author identity, e.g. devpulse.pixvix.palette
ViewportId             one runtime lifetime, e.g. devpulse.pixvix.palette#2
```

- `PilotViewportKey` is a typed constant declared beside the application code
  that owns the window. It is the preferred durable scenario/MCP selector.
- `ViewportId` is allocated by the runtime when that declared key becomes live.
  Reopening the same egui viewport creates a new monotonically increasing
  generation.
- `egui::ViewportId` remains process-local integration data and never becomes
  the durable authoring contract.
- Root has the declared key `root` and one session lifetime.
- Nodes carry both the declared key and current lifetime ID. Node paths remain
  stable across reopen, while actions pin and recheck the lifetime ID to reject
  stale targets.
- A key may have at most one live instance unless its declaration explicitly
  permits keyed repeated children. Duplicate live declarations fail with both
  source locations.

The application uses one wrapper/registration path for immediate and deferred
children. The wrapper records the typed key, egui ID, parent, class, title,
close request, and source location, then calls the real egui viewport API. Root
is registered automatically.

## Viewport registry

The runtime maintains an authoritative inventory independent of semantic
nodes:

```text
ViewportDescriptor
  id, key, parent_id, class, title
  lifecycle_revision
  close_requested
  visible, focused, minimized, maximized, occluded
  inner_rect, outer_rect?, monitor_size?, native_pixels_per_point?
```

Lifecycle is:

```text
absent -> opening -> open -> close-requested -> closed
                    |  ^
                    +--+ metadata/focus/geometry revisions
```

Visibility, focus, minimization, maximization, and occlusion are orthogonal
metadata, not lifecycle states.

Rules:

1. A declaration is not a creation event. `ViewportCreated` is emitted only
   after the child appears in authoritative `RawInput.viewports`.
2. `RawInput.viewports` from a root pass is the integration inventory. A
   previously live child absent from the next authoritative root inventory is
   closed.
3. Closed children are removed immediately from live mappings, semantics,
   input queues, capture queues, motion/busy ownership, and AccessKit mirrors.
4. A bounded tombstone journal retains descriptors and close reasons for
   waits, traces, and diagnostics; tombstones are never actionable.
5. Reappearance of the same key after closure allocates the next generation.
6. Geometry changes are coalescible telemetry. Created, close-requested,
   closed, and focus-transition events are required and lossless.
7. `ViewportClass::EmbeddedWindow` is reported honestly. A project requiring
   native second-window behavior fails capability preflight rather than
   accepting an embedded fallback.

## Protocol and capability surface

Protocol 2.1 adds:

- `PilotViewportKey`, lifetime `ViewportId`, `ViewportClass`,
  `ViewportDescriptor`, `ViewportSelector`, and `ViewportExpectation`;
- `ViewportCreated`, `ViewportChanged`, `ViewportCloseRequested`,
  `ViewportFocused`, and `ViewportClosed` runtime events;
- compact live descriptors in status/snapshot results;
- viewport fields on non-node actions such as press and scroll;
- a viewport selector on capture;
- typed focus and close actions;
- lifecycle expectations for open, visible, focused, close-requested, closed,
  and exact count.

Capability IDs remain granular:

- `viewport.inventory`
- `viewport.lifecycle`
- `viewport.targeted-input`
- `viewport.targeted-capture`
- `viewport.focus`
- `viewport.close`
- `viewport.immediate`
- `viewport.deferred`
- `viewport.native`

They begin `Unsupported` and move to `Supported` only with named conformance
evidence. Cross-viewport drag remains a separate unsupported capability. MCP
does not gain another tool: status/snapshot expose inventory, run owns
lifecycle actions/expectations, and capture accepts a strict selector.

`ViewportSelector` resolves by lifetime ID or durable key, with optional title,
class, parent, and state filters. Resolution is strict: zero and many matches
are errors. Arbitrary egui debug IDs from MCP are rejected.

## Input, actionability, and focus

- A locator-targeted action inherits the node's current lifetime and rechecks
  it immediately before the first input tick.
- Press, wheel, close, focus, and capture require an explicit viewport when
  more than one is live. They may omit it only when the registry has one live
  viewport.
- Pointer/keyboard source state records its viewport. A source cannot switch
  windows while a key/button/touch is pressed.
- Cleanup releases input in the original viewport and renders its cleanup
  pass. A closed original viewport makes cleanup an infrastructure error and
  prevents process reuse.
- Child injection requests repaint for the child. Immediate children also
  wake the parent; deferred children repaint independently.
- Keyboard actionability requires the target viewport to be focused. A
  scenario may explicitly focus it and wait for focus. Platforms where native
  focus cannot be proven fail `viewport.focus` rather than faking it.
- Cross-window drag is not representable as honest per-viewport `RawInput`.
  It stays unsupported until an explicitly separate OS-input backend exists.

## Capture, settlement, and artifacts

- Capture addresses exactly one lifetime and queues the screenshot command into
  that viewport's own output pass.
- Screenshot events are matched to the requested egui viewport; unrelated
  images remain independently queued or are discarded with diagnostics.
- Semantic and geometry hashes are scoped to the selected viewport. Activity
  in another window cannot make a stable target capture fail.
- Motion/busy ownership gains an optional viewport. Session-owned work still
  blocks captures only when the selected policy declares it relevant.
- The artifact descriptor records viewport key, lifetime, class, logical
  dimensions, native pixels-per-point, renderer identity, and content hash.
- A session composite is a later explicit feature. It is never synthesized by
  silently stitching native windows.
- Renderer baselines include viewport key, logical size, DPI, and class.

Root capture is qualified. Deferred capture remains unqualified. Native wgpu
capture of an immediate child is unavailable in eframe 0.35 because
`render_immediate_viewport` hard-codes an empty screenshot request list; that
class fails immediately with evidence instead of entering settlement. Resolving
this requires an upstream eframe change, a reviewed dependency patch, or a
separately specified native-client-area capture backend. OS desktop cropping is
not accepted as a silent substitute for deterministic renderer output.

## Close and shutdown ownership

The application remains the owner of whether a child is declared. The pilot
wrapper exposes close requests to that owner; the owner stops calling the real
egui viewport API. A close action succeeds only after the registry observes
the closed lifetime.

Session stop:

1. cancel the active run;
2. release all input in its owning viewports;
3. cancel capture jobs and fixture work;
4. request child close from leaves to root;
5. wait a bounded interval for child lifecycle closure;
6. flush required events/artifacts;
7. close root and then the transport.

Process death closes all remaining instances with a process-exit reason in the
partial trace.

## Boring authoring pattern

Application code declares one key beside each window and uses the common
wrapper. Durable tests use keys, never egui hashes or window titles:

```toml
[[steps]]
type = "action"
action = "click"
target.id = "root/devpulse.shell.status.snip"

[[steps]]
type = "expect-viewport"
key = "devpulse.pixvix.palette"
state = "open"

[[steps]]
type = "action"
action = "press"
viewport.key = "devpulse.pixvix.palette"
key = "Escape"

[[steps]]
type = "capture"
name = "pixvix-palette"
viewport.key = "devpulse.pixvix.palette"
```

Exact TOML field spelling lands with protocol serde goldens; the pattern is
the contract.

## Slice 2B execution order

Slice 2B begins only after the Slice 2 input executor tracks source state and
can release it deterministically.

1. **Truth first:** add granular viewport capabilities as unsupported, freeze
   current partial wire examples, and add protocol 2.x goldens for the new
   types.
2. **Registry core:** implement typed declaration, lifetime generations,
   authoritative RawInput inventory diffing, stale-state pruning, tombstones,
   and immediate/deferred in-memory conformance.
3. **Runtime vertical:** implement targeted repaint, input, focus, close,
   scoped settlement/capture, leaf-to-root shutdown, and required events.
4. **Runner/authoring:** add inventory, strict selectors, lifecycle waits,
   TOML/Rust syntax, diagnostics, and trace routing.
5. **Existing MCP tools:** extend status/snapshot/run/capture schemas without
   adding authority or a seventh tool.
6. **DevPULSE immediate proof:** instrument PixVix palette and region windows,
   their controls and close-request handling, then qualify discovery, focus,
   targeted input, opening, closure, and teardown. Record immediate child
   screenshot capture as an upstream-blocked capability.
7. **Reusable deferred proof:** qualify a deferred fixture window before the
   toolkit claims generic support; the real presenter window later repeats
   that proof in DevPULSE.
8. **Test Lab projection:** after its runner-backed slice, show inventory,
   parent tree, per-window semantics/captures, and lifecycle traces.

## Acceptance gates

Toolkit conformance:

- immediate and deferred windows create, update, focus, close, and reopen with
  monotonic lifetime generations;
- embedded fallback cannot satisfy a native-required project;
- required lifecycle events are ordered and lossless under queue pressure;
- stale nodes, mappings, queued input, captures, AccessKit trees, and motion/
  busy ownership disappear on close;
- press/scroll/capture without a selector fail as ambiguous when several
  windows are live;
- targeted input reaches only the selected viewport and cleanup returns every
  source to empty state;
- root and renderer-supported child captures are independently stable and carry
  the correct metadata; unsupported viewport classes fail at preflight;
- close and stop shut children leaf-first and never hang;
- one hundred open/close cycles leave zero live tombstone-owned resources and
  remain within the global memory-growth budget.

Real DevPULSE scenario:

1. click the root Snip launcher;
2. wait for exactly root plus the PixVix palette;
3. interact with palette tabs using palette-scoped locators;
4. open region selection and wait for the third viewport;
5. perform the real same-window region drag;
6. request palette and region capture and receive the explicit immediate-class
   unsupported result until the eframe limitation is resolved;
7. cancel region with region-scoped Escape and prove its lifetime closed;
8. close palette, prove only root remains, and prove its old lifetime and
   nodes are stale;
9. reopen palette and prove the durable key resolves a new generation;
10. stop and prove clean input, viewport, artifact, and process teardown.

Qualification runs on Windows hardware locally and the manually dispatched
pinned Lavapipe/Xvfb project. Outer-position assertions remain optional where
the platform does not report them.

## Dependency impact on the remaining roadmap

| Later work | Multi-viewport dependency |
|---|---|
| Slice 3 AccessKit | Separate live tree/mirror/root and stale-tree removal per lifetime |
| Slice 4 assertions | Strict viewport selectors, lifecycle retrying expectations, timeout evidence |
| Slice 5 projects | Per-window size/DPI expectations and native-required capability policy |
| Slice 6 traces/visuals | Lifecycle timeline, per-window checkpoints, scoped capture/baselines |
| Slice 7D PixVix | First real immediate-window qualification |
| Program D4 presenter | Real deferred-window and dual-surface synchronization qualification |
| Slice 8 Test Lab | Inventory, selection, highlight, timeline, per-window replay/evidence |
| Publication | Immediate and deferred support, deterministic teardown, Windows and Lavapipe evidence |

## Local evidence — 2026-07-23

The implementation checkpoint above was qualified locally before push:

- `cargo green` passed the documentation status check, workspace/all-target
  clippy with warnings denied, every workspace test, and the release build
  freshness check.
- Protocol, transport, instrumentation, and runtime compiled for
  `wasm32-unknown-unknown`.
- The feature-gated DevPULSE target and EguiPilotAI CLI were built together at
  protocol 2.1. A deliberately stale 2.0 CLI was first rejected with
  `protocol-version`, proving minor-version drift fails before actions.
- `shell-navigation.toml` passed all four real-process steps, including the
  qualified root screenshot path.
- `pixvix-multi-viewport.toml` passed all nine real-process steps: palette
  discovery, native focus and confirmation, palette-scoped Region click,
  region discovery, region close and closure observation, then palette close
  and closure observation.
- Focused conformance covers lifetime generation after reopen, viewport-bound
  input cleanup and switch rejection, strict snapshot key selection, child
  shutdown waiting, immediate-capture preflight, protocol mismatch, bounded
  snapshot hashing, and failed CLI outcomes returning an error.

This evidence does not close the deferred, immediate-child-pixel, reopen-
through-launcher, stress/memory, Lavapipe, or Test Lab gates.
