# Operator Coach & Agent-Driven UI — grounded plan (C-series)

Agent paints highlights the operator can press; the press flows back as a
typed signal; agents compose tailored ColdLight components declaratively; the
Linux desktop itself becomes coachable. Grounded 2026-07-26 against the real
codebase and the shipping 2026 protocol/tooling landscape (§8 cheat sheet).

## 1. Context

The operator wants AI-guided instructional workflows: "paint highlighted
areas on my monitor that launch a signal when I press them," tailored
agent-generated components, and eventually a Linux desktop fully drivable by
the agent for guided learning. This is the Slice 8 / Test Lab direction in
`docs/plans/eguipilotai-v1-delivery.md` (search/highlight, bounded live
control) — specified, not shipped. This plan turns it into concrete slices
(C1–C4) with a typed signal vocabulary aligned to the two protocols the
industry converged on in 2025–2026:

- **A2UI** (Google, Apache-2.0, v1.0 candidate 2026-06): agents send
  *declarative JSON* — `createSurface` / `updateComponents` /
  `updateDataModel` / `deleteSurface`, plus `action` (user → agent, with
  `wantResponse`) and `actionResponse` (agent → user). Clients render from a
  **pre-approved component catalog**; agent code never executes. This is our
  registry doctrine as an external standard.
- **AG-UI** (CopilotKit, MIT): ~16 typed lifecycle/stream events —
  `RUN_STARTED/FINISHED/ERROR`, `STEP_*`, `TEXT_MESSAGE_START/CONTENT/END`,
  `TOOL_CALL_*`, `STATE_SNAPSHOT` / `STATE_DELTA` (RFC 6902 JSON Patch),
  `CUSTOM`. The start/content/end discipline and snapshot/delta pattern are
  what we mirror.

We adopt these as **Rust enum vocabularies on our existing typed channels**,
not as a JS runtime. The no-webview/no-second-language contract holds.

## 2. Current state (ground-truthed 2026-07-26)

- **EguiPilot** is seven crates. Wire = 4-byte BE length + serde_json
  (`egui-pilot-ai-transport::write_frame`). Agent→app is
  `CommandEnvelope { request_id, session, command: ClientCommand }` with
  `ClientCommand::{Handshake, Snapshot, Run, Capture, Status, Cancel, Stop}`;
  app→agent is `ServerMessage::{Response, Event}` carrying `RuntimeEvent`.
  All defined in `crates/egui-pilot-ai-protocol/src/lib.rs`. Feature gate:
  `eguipilotai-devtools` on `devpulselabs` (crates/app/Cargo.toml).
- **Rects already exist.** Every `pilot_track`ed widget lands in
  `SemanticFrame` as a `SemanticNode` with `rect` + `clip_rect` from
  `response.interact_rect`, accessible via `PilotHandle::latest_frame()`.
  Spotlight geometry needs zero new instrumentation.
- **Fabric** (`dpl-fabric`) domains: Shell, Deck, Capture, Job, Provider,
  Build — each a tokio broadcast channel, hot domains wrapped `Stamped<T>`.
- **Overlay precedent:** `egui::Area` + `Order::Foreground` (chat provider
  hover card, `crates/chat/window/src/lib.rs`); shell paints
  `cl::frame_overlay` after hubs.
- **AccessKit is already on.** eframe 0.35 default features include
  `accesskit`; on Linux locked `accesskit_unix` is **0.21.1** (not the
  0.22.x the early draft assumed) and exposes the tree over AT-SPI via
  zbus. Re-check AccessKit #727 against 0.21.1 before assuming the bus
  panic.
- **This box:** GNOME on **X11** (`XDG_SESSION_TYPE=x11`). AT-SPI bus
  running. **`org.gnome.desktop.interface toolkit-accessibility` may stay
  false** — measured 2026-07-27 / reconfirmed C2s 2026-07-29: GTK3/GTK4
  apps (terminal, calculator, Nautilus) still publish AT-SPI trees with
  the flag off. No operator flip required for Tier A GTK. Electron still
  needs `--force-renderer-accessibility`. No ydotool — irrelevant on X11,
  XTEST covers input synthesis. Second target is the Windows laptop
  (UI Automation + layered windows).
- **Test Lab** today only text-filters the tree and prints rects; no
  spotlight paint, no live-control session.

## 3. Target design

### 3.1 Signal vocabulary — the touchpoint router

The core primitive is not "a highlight"; it is a **dynamically mounted
touchpoint** — router semantics in the axum sense: the agent mounts a
matcher + visual + trigger contract, the runtime fires typed events when it
activates, the agent unmounts (or `once` auto-unmounts). A spotlight
highlight is merely the first visual; an invisible tripwire ("tell me when
the Library rail opens") is the same primitive with `visual: None`.

Coach signals ride the **existing pilot pipe** — same framing, same gate.
Two new enum arms, mirroring AG-UI's lifecycle discipline:

```
ClientCommand::Coach(CoachCommand)         // agent → app
CoachCommand::Mount(Touchpoint {
    coach_id: CoachId,                     // newtype, no bare strings
    at: Target,                            // NodeLocator (existing machinery) | ScreenRect
    visual: CoachStyle,                    // Spotlight | Outline | Arrow | DimAround | None
    on: TriggerKind,                       // Press | Hover | NodeAppeared | ValueChanged
                                           //   | FabricEvent(domain filter)
    message: Option<String>,               // short operator-facing caption
    ttl_ms: Option<u64>,
    once: bool,                            // auto-unmount after first trigger
})
CoachCommand::Unmount { coach_id } | UnmountAll

RuntimeEvent::Coach(CoachEvent)            // app → agent, Stamped
CoachEvent::Mounted   { coach_id, rect, ms }
CoachEvent::Triggered { coach_id, kind, actor: Operator | Agent, ms }
CoachEvent::Unmounted { coach_id, reason: Ttl | Operator | Agent | Once }
CoachEvent::Refused   { coach_id, reason: TargetNotFound | Occluded | Unsupported }
```

Every `Mount` terminates in exactly one `Unmounted` or `Refused`, and a
`once` touchpoint emits exactly one `Triggered` first — the start/end law
from AG-UI, enforceable in property tests. `Refused` is mandatory: degrade,
never crash (Law 4). The C1 press-to-ack flow is
`Mount { visual: Spotlight, on: Press, once: true }` → `Triggered`.

`Triggered.actor` is what makes **coach mode and demo mode one system**:
coach mode = the operator presses the touchpoint; demo mode = the agent
fires the same touchpoint itself through EguiPilot's existing input
injection (`Run` flows) while the visual narrates. Same rails, different
actor — a mode flag, never a second feature.

### 3.2 Live semantic stream, not screenshots (C1.5)

Today agents poll `ClientCommand::Snapshot`. The agent's picture of the
screen should instead be a live typed model, with screenshots demoted to
proof artifacts — AG-UI's snapshot/delta pattern mapped onto `SemanticFrame`.

**Amended after the C1 pass (2026-07-27): this was one slice pretending to
be one problem, and it is two.** The draft justified the stream by saying it
"unlocks `NodeAppeared`/`ValueChanged` triggers." That is false. **Triggers
are evaluated in the app, by the router, not by the agent** — the plugin
already holds every `SemanticNode` (state, value, `changed_at`) for every
viewport on every pass. A trigger is a comparison against data already in
hand; it never touches the wire. So the split is:

- **C1.5a — dynamic triggers.** In-app only. `NodeAppeared`,
  `ValueChanged`, `FabricEvent`. No streaming, no new wire traffic beyond
  the coach events themselves.
- **C1.5b — the semantic stream.** The agent's live picture. Separable,
  and deferred until polling is an actual pain point.

And the streaming machinery is largely already shipped, which changes its
design from "build" to "compose":

- `SnapshotMode::Changes` + `since_revision` against per-node `changed_at`
  is a tested delta **pull** with continuation/pagination and the agent
  projection budget already applied.
- `queue_telemetry` is a coalescing latest-wins lane whose drops are
  reported as `QueuePressure { queue, dropped }` on the lossless lane —
  exactly the "bounded, coalesce on lag, resync on overflow" behaviour the
  draft listed as work to do. `RuntimeEvent::Telemetry { channel, revision }`
  is already shaped as "channel X advanced to revision N".

**Ruling (operator, 2026-07-27): C1.5b is a revision ticker, not a delta
push.** A `semantic` telemetry channel ticks the frame revision; the agent
pulls `Snapshot { mode: Changes, since_revision }` when it wants detail.
This reuses the shipped coalescing, pressure accounting, pagination, and
projection budget instead of duplicating them in a second push path, and it
cannot outrun a slow agent. Typed delta events are reconsidered only if a
measured round-trip cost justifies a second mechanism.

#### Fabric triggers cross a boundary — one way, app-chosen

`FabricEvent` fires a touchpoint when the app's own signal fabric does
something (e.g. `BuildEvent::Finished`). Fabric is app-side and the pilot is
toolkit-side, and §5 forbids fabric an agent-facing channel. **Ratified
(operator, 2026-07-27):** the app pushes, behind `eguipilotai-devtools`.
The app subscribes to whichever domains it wants coachable and forwards a
`SignalId` into `PilotHandle`; the toolkit never subscribes to fabric and
cannot enumerate it. The agent may only *wait for* signals the app chose to
publish. Per Law 5 the signal is a `SignalId` newtype, never a bare string.

#### Mounting on something that does not exist yet

`NodeAppeared` mounts on a target that by definition is not there — but
today mount refuses `TargetNotFound` and `Mounted { rect }` requires a rect.
**Ratified (operator, 2026-07-27): a distinct `Armed` event.** A touchpoint
whose trigger is an edge may mount unresolved and emits `Armed { coach_id }`;
`Mounted { coach_id, rect }` follows if and when the target resolves. This
keeps `Mounted` meaning "on screen at this rect" — an honest word an agent
and a proof screenshot can both rely on — and makes waiting explicit rather
than encoding it as an absent field. `Press` still refuses an unresolved
target: there is nothing to press.

Capability naming follows: `coach.stream-triggers` was misnamed, since
dynamic triggers ship without a stream. It becomes `coach.trigger.dynamic`
(supported at C1.5a); the ticker declares `coach.semantic-ticker` (C1.5b).

### 3.3 In-app rendering (C1)

Runtime resolves `at` against `latest_frame()`, then paints over the node
rect on a foreground layer. ColdLight vocabulary only: crisp edge ring,
gradient tint, black diffusion scatter dimming outside the rect — never
backdrop blur. Press inside → `Triggered`; agent `Unmount`, TTL, or a target
that stops rendering → `Unmounted`; unresolvable target → `Refused`.

**Amended in the C1 pass (2026-07-27): the overlay observes, it never
intercepts.** The draft said `egui::Area` + `Sense::click`. A sensing
foreground area would swallow the press, so the operator told to "click Save"
would highlight Save and never actually save — the coaching would be theatre.
The shipped renderer is a bare `layer_painter` on `Order::Foreground` with no
interaction area at all; the trigger is read out of the same frame's pointer
events. The real control still does its real work, and the agent still learns
the press happened.

### 3.4 Desktop overlay daemon (C2) — `dpl-coach-overlay`

A separate workspace binary painting on the whole monitor, over any app.
Default state: fully click-through. Touchpoints carve **input hotspots** —
the click-region *is* the trigger mechanism, no global hooks:

| Platform | Overlay | Click-through / hotspots |
|---|---|---|
| Windows laptop | layered topmost window | winit `set_cursor_hittest` (solid; `WS_EX_NOACTIVATE` variant) |
| X11 (this box, today) | override-redirect ARGB window | **XShape input rectangles via `x11rb` directly** — per-region hit-test; winit's hittest is all-or-nothing and was historically flaky on X11 (winit #3509/#4120, fix #4416 post-0.30) |
| Wayland wlroots (Sway/Hyprland/niri) | `zwlr_layer_shell_v1` Overlay layer (smithay-client-toolkit) | `wl_surface.set_input_region` with hotspot region |
| GNOME Wayland | **unsupported** — Mutter has no layer-shell | documented fallback: stay X11, or Shell extension (out of scope) |

Control: the same framed-JSON `CoachCommand`/`CoachEvent` protocol over the
daemon's inherited stdio — identical trust model to the EguiPilot pipe. This
is a **new process boundary** and requires its own ADR ratified against the
no-IPC law before C2 code (the chat provider adapters are the template).
Until ratified, C2 does not start.

### 3.5 Tailored components (C3) — the A2UI catalog

Define `catalog_id = "devpulse/coldlight/v1"`: the closed set of ColdLight
components an agent may compose (Text, Button, Card, Row/Column, Icon,
ProgressRing, …hand-picked). Adopt A2UI message semantics as enums:

```
CoachCommand::Surface(SurfaceCommand)
SurfaceCommand::Create { surface_id, catalog_id, anchor }   // anchor: node rect | screen pos
SurfaceCommand::UpdateComponents { surface_id, components } // adjacency list, ids only
SurfaceCommand::UpdateData { surface_id, path, value }      // upsert semantics
SurfaceCommand::Delete { surface_id }
CoachEvent::Action { surface_id, component_id, action, want_response }  // user → agent
```

Renderer walks the component list against the catalog registry; unknown
component → `Refused`, never crash. Works both in-app (C1 renderer) and on
the desktop daemon (C2 renderer).

### 3.6 Full desktop semantic control (C4)

Read side: `atspi` crate (pure Rust, zbus) — roles, names, bounds, actions
of every accessible app. Write side: XTEST on X11. Hybrid grounding (2026
SOTA, cf. UFO²): tree first, screenshot fallback for canvas-drawn surfaces.
DevPULSE itself is *already visible* to this layer via AccessKit. Decision
point inside C4: build the adapter vs. run `agent-sh/computer-use-linux`
(Rust MCP server, has `doctor`) as an external tool first and only
internalize what we actually use.

## 4. Changes (execution order)

1. **C1 — operator-coach vertical (in-app spotlight + press-to-trigger).**
   **LANDED 2026-07-27 — see §9.**
   Router-shaped protocol arms from day one (`Mount`/`Unmount`/`Triggered`
   + roundtrip tests) but only `visual: Spotlight`, `on: Press` implemented
   → runtime handling + resolver → egui Foreground renderer (ColdLight) →
   CLI verb `eguipilotai coach mount <node> --message <s> --once` →
   scenario TOML step (`coach.mount` + `expect triggered`) → Test Lab
   "coach" tab reusing the same commands. Unimplemented `TriggerKind`s
   answer `Refused { Unsupported }` — no silent stubs. Amends delivery-doc
   Slice 8 rather than forking it.
2. **C1.5a — dynamic triggers (in-app).** `NodeAppeared` / `ValueChanged`
   evaluated in `coach::run_pass` against the committed nodes it already
   receives; `FabricEvent` fed by the app-side push. `Armed` for unresolved
   mounts. Capability `coach.trigger.dynamic`. No wire streaming.
3. **C1.5b — semantic revision ticker.** A `semantic` channel on the
   existing telemetry lane ticking `SemanticFrame::revision`; agents pull
   `Snapshot { mode: Changes, since_revision }` for detail. Deferred until
   polling measurably hurts.
4. **C2a — coach-overlay ADR** (boundary, launch gates, forbidden set).
   **RATIFIED 2026-07-30** — [`coach-overlay-adr.md`](coach-overlay-adr.md).
   **C2b — daemon** pass-1 self-test **PASS** (#158) —
   [`docs-history-2026-10-12:docs/plans/coach-c2b-overlay-daemon.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/plans/coach-c2b-overlay-daemon.md);
   journal [`../journal/2026-07-30-c2b-coach-overlay.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/journal/2026-07-30-c2b-coach-overlay.md).
   X11 first; Windows second; layer-shell third. **Pass-2 stdio PARKED**
   (Advanced — not product NEXT).
4b. **C2c–C2f — Operator Coach experience** (product spine) —
   [`docs-history-2026-10-12:docs/plans/coach-operator-experience.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/plans/coach-operator-experience.md).
   Mock → lightweight inventory → fabric mint (Tier A) → wire to overlay.
   Dual surface: Operator vs Advanced/EguiPilot. **THIS IS NEXT.**
5. **C3 — ColdLight catalog + Surface commands**, in-app renderer first.
6. **C4 — desktop semantic control**, external-MCP-first evaluation, then
   the `atspi` adapter if internalization earns its keep.

Each slice is its own pass with its own plan §8 evidence; this document is
the program map.

## 5. Boundaries

- Coach signals live on the **pilot pipe**, gated by `eguipilotai-devtools`.
  Normal production builds contain no coach endpoint (AGENTS.md law holds).
- Fabric gains **no** agent-facing channel. The C1.5a `FabricEvent` trigger
  does not weaken this: the flow is one-way and app-chosen (app → pilot),
  the toolkit never subscribes to fabric and cannot enumerate it, and the
  agent may only wait for signals the app decided to publish. An in-process
  `CoachEvent` fabric domain (pilot → hubs) stays deferred until a hub
  actually needs to react to coaching.
- A2UI/AG-UI are adopted as **vocabulary and semantics**, not dependencies:
  no JSON-schema runtime, no JS, no webview. serde enums are the contract.
- The C2 daemon is devtools-only until its ADR is ratified; GNOME-Wayland
  overlay support is explicitly out of scope (documented fallback).
- `/content` untouched; prior app remains read-only reference.

## 6. Verification

- Protocol: serde roundtrip + exhaustive-match tests on the new arms;
  every `Mount` terminates in exactly one `Unmounted`/`Refused`, and a
  `once` touchpoint emits exactly one `Triggered` first (property test).
- C1 scenario: boot fixture → `coach.mount` on a known `PilotId` →
  synthetic click inside hotspot → `Triggered { actor }` within budget;
  second scenario proves `Refused` on a bogus locator. Proof PNG of the
  spotlight at a real window size vs committed baseline (ColdLight gate).
- C1.5a: a scenario mounts `NodeAppeared` on a node that is not rendered
  yet, asserts `Armed`, navigates so the node appears, then asserts
  `Mounted` then `Triggered` — proving the edge fires from in-app state with
  no snapshot poll in between. A second mount proves `ValueChanged` against
  a node whose value the run changes, and a third proves a `FabricEvent`
  touchpoint fires from an app-published signal. `Press` on an unresolved
  target still refuses.
- C1.5b: ticker advances only when the revision does; lag injection proves
  coalesce + `QueuePressure` accounting; a pull with `since_revision`
  returns exactly the nodes whose `changed_at` moved.
- C2: `dpl-coach-overlay --self-test` paints a hotspot, injects an XTEST
  click, asserts `Triggered` — CI-friendly on the Linux lane under Xvfb.
- `cargo green` both lanes; exe rebuilt, mtime moved.

## 7. Commit plan

- This plan lands alone first (branch `plan/operator-coach-a2ui`, PR, green).
- C1 lands as one pass (`coach-c1-touchpoint`), delivery-doc Slice 8
  amended in the same PR; C1.5a (`coach-c15a-dynamic-triggers`) stacks on
  it as its own pass, carrying this §3.2 amendment. C1.5b is deferred.
  C2a ADR + C2b self-test land together (ratified + proven #158);
  Pass-2 stdio is PARKED. Product NEXT = C2c–C2f
  ([`docs-history-2026-10-12:docs/plans/coach-operator-experience.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/plans/coach-operator-experience.md)).

## 8. Evidence — grounding cheat sheet (2026-07-26)

**Protocols**
- A2UI v1.0 candidate (2026-06-08): messages `createSurface` (fixed
  `surfaceId`+`catalogId`), `updateComponents` (flat adjacency list),
  `updateDataModel` (upsert, path `/` replaces), `deleteSurface`,
  `callFunction`/`functionResponse`, `action`+`wantResponse`→
  `actionResponse`. Catalog ids are agreed strings, not fetchable URIs.
  https://a2ui.org/specification/v1.0-a2ui/
- AG-UI: `RUN_*`, `STEP_*`, `TEXT_MESSAGE_START/CONTENT/END`, `TOOL_CALL_*`,
  `STATE_SNAPSHOT`/`STATE_DELTA` (RFC 6902), `MESSAGES_SNAPSHOT`, `RAW`,
  `CUSTOM`, `REASONING_*`. https://github.com/ag-ui-protocol/ag-ui

**Overlay mechanics**
- Wayland: `wl_surface.set_input_region` (empty = full click-through),
  layer-shell Overlay layer; GNOME/KDE lack wlr-layer-shell. Prior art in
  Rust: wayscriber, wlr-draw (daemon + control socket — good C2 template),
  wayland-shadow-layer (wgpu overlay skeleton).
- X11: override-redirect ARGB window + XShape *input* region = per-rect
  hotspots. winit `set_cursor_hittest`: solid Win32/Wayland; X11 fixed only
  post-0.30 (winit #4416) → use `x11rb` shape calls directly.
- Windows: `WS_EX_LAYERED|WS_EX_TRANSPARENT|WS_EX_TOPMOST`; winit hittest OK.

**Desktop control (Linux SOTA)**
- AT-SPI2 over D-Bus = the semantic layer; Rust `atspi` crate.
- Shipping MCP servers: agent-sh/computer-use-linux (Rust; doctor;
  GNOME/KDE/Hyprland; Wayland-first), varbhat/desktopmcp (144 tools:
  portals + AT-SPI + D-Bus). Hybrid tree+vision is the consensus (UFO²).
- egui apps are AT-SPI-visible **only** through AccessKit — our EguiPilot
  semantic layer remains the richer source for our own app.
- accesskit_unix 0.22.0 gate changed to `IsEnabled` → connects on normal
  desktops; broken-bus panic fixed upstream PR #741 (2026-07). Watch on
  next egui bump.

**Operator changes required (this box)**
1. **GTK Tier A: none.** Measured with `toolkit-accessibility` still false —
   calculator / terminal / Nautilus publish AT-SPI trees anyway (C2s). Flip
   the gsettings key only if Qt/Electron targets stay dark; Electron also
   needs `--force-renderer-accessibility`.
2. Stay on the X11 session for coach work — best overlay + input story
   today. If you later want Wayland: Hyprland/Sway/niri give the full
   layer-shell experience; GNOME Wayland does not.
3. Nothing to install for input (X11 = XTEST built in). `ydotool` only
   becomes relevant on a Wayland session.
4. Windows laptop: no changes — layered windows + UI Automation are native.
5. a11y bus address path is `/org/a11y/bus` (lowercase), not `/org/a11y/Bus`.

**Language verdict: stay pure Rust.** Every layer has first-class Rust:
winit/egui/AccessKit (UI + a11y), x11rb (XShape/XTEST), smithay-client-toolkit
(layer-shell), zbus/atspi (AT-SPI), windows-rs (Win32). A2UI/AG-UI are JSON
protocols — serde enums, no runtime to import. The single genuine non-Rust
corner is a GNOME-Shell extension (JavaScript) if GNOME-Wayland overlays were
ever required; the ruling here is: don't — X11 today, wlroots if Wayland
later. No expansion from Rust is warranted.

## 9. C1 evidence (2026-07-27)

Protocol 2.2 → **2.3**. New capability `coach.touchpoint` is *supported*;
`coach.visual.extended`, `coach.stream-triggers`, `coach.desktop-overlay`
are declared *unsupported* and each names the slice that ships it, so a
scenario asking for an unbuilt visual or trigger fails before the app boots
instead of half-running.

What shipped, crate by crate:

- `egui-pilot-ai-protocol` — `CoachId`, `Touchpoint`, `CoachStyle`,
  `TriggerKind`, `CoachActor`, `CoachUnmountReason`, `CoachRefusal`,
  `CoachCommand`, `CoachEvent`; `ClientCommand::Coach { coach }`,
  `CommandResult::Coach`, `RuntimeEvent::Coach`, `Step::Coach`,
  `Expectation::Coach`.
- `egui-pilot-ai` — `coach` module: `CoachPalette` (ColdLight ink),
  `CoachStore` (mount ledger, TTL, viewport cleanup), and the pass that
  paints and observes. Painted per target viewport, clipped to the
  viewport rect, unmounted automatically when its viewport closes.
- `egui-pilot-ai-runtime` — locator resolution against `latest_frame()`,
  the capability refusal gate, the `coach_ledger` that backs
  `Expectation::Coach`, event forwarding, and unmount-all on both `cancel`
  (`reason: agent`) and `stop` (`reason: session-ended`) so no touchpoint
  outlives its session.
- `egui-pilot-ai-runner` / `-cli` — `ProcessSession::coach` / `await_coach`,
  the `eguipilotai coach` verb, and a seventh MCP tool `pilot_coach`.
- `crates/app` — the ColdLight coach palette bound to the devtools runtime.

Three findings the implementation forced back into the design:

1. **Non-interception** (§3.3 amendment) — the overlay must not sense.
2. **Actor attribution needs a synthetic-input marker.** `Triggered.actor`
   is the whole coach-vs-demo distinction, and a press looks identical in
   `egui::RawInput` whichever side produced it. The plugin now records
   which viewports received injected presses in the frame it injects them,
   so demo-mode presses report `agent` and operator presses report
   `operator`. Without this the two modes are indistinguishable on the wire.
3. **`ClientCommand::Coach` had to become a struct variant.** Externally
   tagged `ClientCommand` plus internally tagged `CoachCommand` both emit
   `type`, and serde produced a duplicate-field error on the roundtrip test.
   Nesting under `coach` keeps both tags legible on the wire.

Proof: `tests/eguipilotai/coach-touchpoint.toml` runs the full lifecycle on
the real exe — mount → `expect coach mounted` → capture → agent click →
`expect coach triggered` → unmount → `expect coach unmounted`. Baseline
`docs/proof/coach-spotlight-rail-workbench.png` shows the Workbench rail
button lit inside the dim with its caption plate. The trace records
`actor: agent` for the injected press, proving attribution.

Not built, deliberately: the Test Lab coach tab (Test Lab holds no live
session — that is Slice 8's `test-lab.live-control`), `Screen`-anchored
touchpoints (no use before C2), and every trigger except `Press`.

## 10. C1.5a evidence (2026-07-27)

Protocol 2.3 → **2.4**. `coach.stream-triggers` is gone; `coach.trigger.dynamic`
is *supported* and `coach.semantic-ticker` is *unsupported*, pointing at C1.5b.

Every `TriggerKind` now fires: `Press`, `Hover`, `NodeAppeared`,
`ValueChanged`, and `FabricEvent { signal: SignalId }`. All five are **edges**,
never levels — a hovered node fires once on entry, not every frame it stays
hovered. `CoachEvent::Armed` and `CoachActor::App` are new. The CLI takes
`--on press|hover|appeared|value-changed|signal:<id>` and `pilot_coach` takes
the same string.

The app publishes its coachable catalog through `PilotHandle::note_coach_signal`
behind the devtools gate. The toolkit still cannot read the fabric, so an agent
can only wait for what the app chose to expose. The catalog itself is §11.

Four findings from building it:

1. **Stacked dimming made multi-touchpoint coaching unusable.** Each spotlight
   dimmed the whole screen, so two touchpoints applied two scatter layers and
   blacked out each other's target *and* caption. The renderer now paints one
   scatter with a hole per lit target (scanline decomposition of screen minus
   the union), then every ring, tint, and caption on top. Proof:
   `docs/proof/coach-armed-sub-episodes.png` shows two equally lit targets
   under a single dim.
2. **Actor attribution generalised.** The C1 marker only tracked injected
   *presses*. It now records any injected input, and attribution is derived
   from the frame: injected input is `Agent`, real input is `Operator`, and a
   frame with no input at all is `App`. That third case is why `CoachActor`
   grew an arm — an appearance or a published signal is nobody's press, and
   calling it an operator action would be a lie the agent reasons from.
3. **Signals are read by sequence, never drained.** Draining would race
   between viewport passes in one frame: whichever viewport painted first
   would consume the signal and the others would never see it. The store keeps
   a bounded ring and each touchpoint remembers the last sequence it accounted
   for.
4. **A waiting touchpoint needs a node id.** A name or role locator cannot
   match a node that has not rendered, so `NodeAppeared` with a name locator
   is refused with that reason rather than arming on a target it could never
   resolve.
5. **Two limits on coach proof baselines, found by checking rather than
   assuming.** Captures on the Library route are only reproducible once the
   slide art's ambient motion settles — a capture taken the instant the rail
   opens differs run to run inside the art, so the scenario waits before
   capturing. And every capture carries the build hash and a minute-stamped
   clock in the footer, so any coach baseline differs from the previous commit
   in that one strip. Coach baselines are therefore reviewable images, not
   byte-exact gates, until the footer is excluded from comparison.

`NodeAppeared` **always** arms, even when its target is already on screen:
"tell me when this is there" should answer immediately when it already is.
Waiting for a second appearance would be a trap.

Proof: `tests/eguipilotai/coach-dynamic-triggers.toml` on the real exe arms a
touchpoint on a rail sub-item that does not exist yet, opens the house, and
asserts `Armed` → `Mounted` → `Triggered { node-appeared, actor: app }`; a
second touchpoint waits on the app's own route signal and fires when the
navigation lands.

Deferred to C1.5b, unchanged: the semantic revision ticker. Signal discovery
was deferred here and is closed in §11.

## 11. C1.5c evidence — the whole nervous system, and a way to find it

C1.5a shipped the trigger but only three facts to aim it at, chosen ad hoc, and
no way to learn even those. The fabric has **six domains and thirty-one
distinct facts**; the coach now maps every one of them.

**Named once, not twice.** The agent seam already names these facts for its
`/signals` SSE stream (`dpl_seam::event_json` and friends, `domain` + `type`).
The coach reuses that vocabulary kebab-cased, so a fact has one name on both
surfaces: `shell.route-changed`, `capture.inbox-changed`, `providers.ready`.
The one id shipped in C1.5a that predated this rule, `shell.route/<hub>/<sub>`,
was renamed to `shell.route-changed/<hub>/<sub>`.

**Coarse and specific.** Every fact publishes a coarse `<domain>.<type>` id, so
an agent can wait for "any route change" or "any capture saved". A fact also
publishes `<domain>.<type>/<key>` when the key comes from a set the app can
enumerate at boot — routes from the hub registry, providers from the chat
registrations, and the booleans (`shell.full-bleed/on`,
`capture.recording/off`, `capture.palette-visibility/shown`). Keys that are
free-form runtime strings deliberately get **no** specific id: a capture mode,
a job id, a deck beat index, an operator's episode name. An agent could not be
told about those in advance, so it waits on the coarse fact and reads the
detail from the UI.

**Discovery.** `SessionStatus` grew `coach_signals`: every id this build can
publish. The app declares it at attach from the live registry, so adding a hub
or a provider widens what is coachable with no edit here (Law 4). It is
readable from MCP `pilot_status` and printed by `eguipilotai inspect` beside
the semantic inventory, because the two answer the same question — what can a
touchpoint be aimed at, and what can it wait for. A host that declares nothing
reports an empty list, which reads as "nothing is waitable", not "anything is".

**The catalog is enforced, not documented.** Three tests hold it closed:
every id the mapping can publish is declared, every declared id is actually
publishable, and every domain is genuinely subscribed and drained through a
real `Fabric`. Each mapping is an exhaustive match for the same reason the
seam's is — a new fabric variant must fail compilation rather than silently
become uncoachable.

Findings worth keeping:

1. **The provider redaction boundary survives id-building.** Provider ids are
   registered identities and safe to name; codes, urls, tokens, versions, and
   the code length never reach an id. A test asserts runtime payloads do not
   appear in ids at all.
2. **Operator content is kept out of the id space, with one honest
   exception.** Build ids carry the fact, never the show or episode name. But
   Shows subroutes *are* discovered from `/content`, so the declared catalog
   contains `shell.route-changed/shows/<show>`. That is the same registry the
   agent seam already publishes at `/state` and the same names the rail
   renders, so it exposes nothing new — but it is worth naming rather than
   claiming a purity the catalog does not have.
3. **The registry derivation is load-bearing, and visible.** The real exe
   declares 121 ids, including the three shows discovered at boot. A hand list
   would have silently omitted them.
4. **The exhaustiveness gate fired on its first real encounter.** Rebasing
   this work onto a `main` that had gained `ProviderEvent::
   VerificationRetryScheduled` (#91) failed to compile, exactly as intended —
   the new fact could not slip through as uncoachable. Closing it was three
   lines and a fixture, and the id followed the seam's name for free:
   `providers.verification-retry-scheduled`. That is the whole argument for
   the exhaustive match over a wildcard.

Proof: `eguipilotai inspect devpulse-cloud library` lists the 121 ids from the
running exe; `tests/eguipilotai/coach-dynamic-triggers.toml` still passes with
the renamed route signal.

## 12. C1.5d — a mount is a subscription, not a resolution

Found by running a real three-step lesson on the real exe with a real operator
(2026-07-27), which is the only reason it was found at all.

**The failure.** Step 1 spotlit the Workbench rail tile and completed
correctly on the operator's press (`actor: operator`). Step 2 then said "now
choose Episodes" and was **refused** `target-not-found` on
`root/shell.rail.sub/workbench/episodes`.

The node id was correct. Verified by injection: before the rail tile is
pressed there are **zero** `shell.rail.sub/*` nodes — the accordion is folded —
and after the press all five appear, `episodes` among them. So step 2 mounted
a `Press` touchpoint on a node that step 1 had *just brought into existence*,
and §4's arm predicate only arms for `NodeAppeared` (and for an unresolved
`FabricEvent`). Everything else refuses.

**This is not an edge case; it is the shape of every tutorial.** "Click here,
now click the thing that just appeared" is what a lesson *is*. §10 finding #4
concluded "`Press` still refuses an unresolved target: there is nothing to
press." That is true of a bare mount and false of a lesson, where the whole
point is *this is about to exist, and then you press it*.

**The industry has a name for our bug.** Playwright draws exactly this
distinction: an `ElementHandle` points at one node and goes stale; a `Locator`
is a *lazy description* re-resolved immediately before every action, with
auto-waiting actionability checks (attached, visible, **stable**, receives
pointer events, enabled). Our `Mount` behaved like an ElementHandle — resolve
once, refuse if absent — while the shape we want is the Locator.

Two things fall out of reading it that way:

1. **`locator.click({ trial: true })`** performs the actionability checks and
   skips the action. That is precisely coach semantics: *wait until this is
   genuinely ready, then point at it and let the human act*. The coach is a
   trial-mode Playwright action with a paint step.
2. **Playwright's "stable" check** (same bounding box across two consecutive
   frames) is independently the fix for §11's finding 5, where Library
   baselines varied because captures raced the slide art's ambient motion. One
   check answers a targeting question and a proof-determinism question.

**Ruling (2026-07-27): a mount is a subscription, not a resolution.** A
touchpoint carrying a **node-id** locator arms whenever its target is not
resolvable *yet*, for **every** trigger kind, and binds when the target
resolves — `Armed → Mounted{rect} → Triggered`. §10 finding #4 is amended
accordingly. Name and role locators still refuse when unresolved, for the
reason §10 gave and which still holds: a name cannot be watched for.

The store already worked this way. `coach::run_pass` re-resolves every mounted
touchpoint by id on every pass and emits `Mounted{rect}` on the absent→present
edge for any trigger; only the *firing* of `NodeAppeared` was trigger-specific.
C1.5a built the whole lazy machine and then gated the entrance to it. The
change is the arm predicate in `mount_coach`, not new machinery — which is the
argument for having built `Armed` as a first-class event in the first place.

### Completion should be a postcondition, not an input event

The deeper lesson, and the one that shapes the lesson document type before it
is authored. The 2026 GUI-agent literature converges hard here:

- **WorldGUI** (interactive desktop benchmark) distils three principles:
  post-planning critique, **pre-action validation**, and **post-action
  evaluation** — verify the step's prerequisites *and* its outcome, not the
  click.
- **VLAA-GUI** makes a Completeness Verifier mandatory against UI-observable
  success criteria, and reports the failure mode that matters to us: *when the
  agent fails, it almost always believes it succeeded* — false completion is
  dominant, not action failure.
- **VAGEN** shows passive observation is the ceiling, and proactively probing
  latent state (file attributes, background config) lifts OSWorld-Verified
  verification accuracy from 84.7% to 92.9%.

Applied here: **a press is a hint, and a signal is the truth.** A step that
ends when a button was clicked has verified nothing; a step that ends when the
route actually changed, the file was actually saved, or the value actually
moved has verified the thing the lesson was teaching. The demo's step 3 already
did this correctly — `signal:shell.route-changed/workbench/library` — and it is
the one step that would have survived a UI relayout. That pattern is the norm
for lesson steps, not the exception, and the 121-id catalog from §11 is exactly
the postcondition vocabulary it needs.

**Ships in this pass:** the arm predicate, so "click what just appeared" is
expressible at all.

**Named, deferred, each its own small pass:** the remaining actionability
checks (stable, receives-pointer-events/occlusion, enabled) as a mount gate and
a capture gate; a lesson step carrying a distinct *advance-on postcondition*
separate from its *point-at target*, with `Satisfied` distinguished from
`Triggered`; and the lesson document type itself (`validate → Vec<Issue>`, per
the slide/episode contract). None of these are blocked by the others.

### Two more findings from proving the above

**Expectations already auto-retry — the flake was the window, not the
assertion.** Recorded because the first reading of this evidence was wrong, and
the wrong reading is the tempting one. Proving the arm predicate took two
attempts: `expect mounted` failed after a 900 ms wait and passed after 2000 ms,
which looks exactly like a sample-once assertion racing a slow bind, and would
send someone off to build auto-retrying expectations. They exist.
`Step::Expect` re-evaluates every millisecond for a hardcoded five seconds,
and `complete_step` clears `wait_started`, so the budget is per step — the
shape of
Playwright's auto-retrying assertions.

Which makes the failing run *more* interesting, not less: a step can only fail
after the full five seconds have elapsed, so the touchpoint genuinely never
bound in five seconds of polling, and the cause was the degenerate 600×400
accordion below, where whether a row is committed at all is animation
dependent. The lesson is about diagnosis rather than design — a retry budget
makes a **flaky** failure wear the costume of a **timing** failure, and only
the trace tells them apart. The one real limitation is that five seconds is
hardcoded rather than per-expectation.

**The rail accordion becomes unreachable below the app's minimum window.** At
the 600×400 the harness launched, `shell.rail.sub/*` rows are still tracked and
still carry ids, but every one reports `hidden + clipped` with an **inverted
rect** (`min_y` 366…502 against `max_y` 335) because the unfold is clipped to
the rail's bottom edge, which sits exactly at the last house tile. The rows are
invisible *and* unclickable, and a locator aimed at one is correctly refused
`not-visible` — the tree advertises five controls that cannot be reached.
`with_min_inner_size` declares 640×480, so 600×400 is below contract and this
is not a shipped-size regression; the open question is what the smallest
supported size is and whether the accordion should scroll or fold rather than
clip to nothing. Worth its own small pass, and a reminder that the semantic
tree can be honest about identity while lying about reachability.

## 13. C1.5e — completion is a level, not an edge (2026-07-27)

### The gap, measured

`TriggerKind`'s own doc comment states the law: "every trigger is an **edge**,
never a level." That is right for input, and fatal for lessons. Measured on the
real exe, one session, two flows differing only in whether the goal was already
true:

| Flow | Result |
|---|---|
| Touchpoint waits on `shell.route-changed/workbench/library` while already on `workbench/library` | **never fires** — failed after a 3s wait plus the full 5s retry budget |
| Same mechanism waiting on `shell.route-changed/home/main`, then navigating there | fires normally |

Signals work; they cannot answer *already true*. So a step whose goal is
already met hangs forever. "Make sure you are on the Library route" hangs for
everyone already there, and a lesson resumed part-way hangs on every step it
has already passed — which is exactly the "from any starting point" premise
this program is aiming at.

### The ruling

**Ratified (operator, 2026-07-27): a narrow level condition, `Touchpoint.until`,
evaluated in the coach.** `CoachCondition` is `NodeValue` / `NodeVisible` /
`NodeGone`, and a new `CoachEvent::Satisfied` is distinct from `Triggered`:
the step is done because the application *reached the state*, not because a
button was pressed. It is read every pass, so a condition already true at mount
satisfies on the mounting pass. `PROTOCOL_VERSION` 2.4 → 2.5.

Two constraints shaped it. It lives in `egui-pilot-ai`, not the runtime,
because the runtime is devtools-gated and a shipped lesson cannot depend on the
pilot pipe. And the vocabulary stays narrow because the semantic tree already
carries the state a lesson asks about, so a node-state check is enough without
importing the whole `Expectation` evaluator (which needs frame data the coach
pass does not have).

### What proving it taught

**A trigger must not end a step that has a postcondition.** The first live
attempt at the transition case failed, and the cause is the doctrine itself:
the touchpoint's `on: press` fired on the click that merely *opened the rail
accordion*, and `once` unmounted it before navigation ever happened. `push_fired`
now skips its auto-unmount whenever `until` is set — the press is still reported,
because an agent wants to know the operator tried, but only the state ends the
step. This is "a press is a hint, the state is the truth" made mechanical, and
it is what makes the ordinary lesson shape — click the thing that reveals the
thing — expressible at all.

**Node values are compound strings, which makes state hard to ask about.** A
rail tile reports `active`, `inactive`, or `active+open`: "which route is
current" and "is the accordion unfolded" are packed into one free-form value.
An author writing `value: "active"` gets silence after navigating, because the
truth is `active+open`. `NodeValue` stays an exact match — a matcher that
guesses at `+` delimiters would be inventing protocol semantics out of one
app's habit — so the defect to fix is upstream: openness belongs in `NodeState`,
not concatenated into the value. Until then, lesson authors must snapshot first
and copy the literal value.

**The tree is nearly empty outside the rail.** A full snapshot on
`workbench/library` at 1280×832 returns **six nodes**: five hub tiles and the
chat window. The accordion sub-rows do not exist until their house tile is
clicked, and the Library content carries no pilot ids at all. The coach can now
express what a step means, and there is almost nothing in the app to point it
at. Instrumentation coverage, not coach capability, is the binding constraint
on the first real lesson.

### Evidence

Both flows pass on the real exe after the fix (`--window 1280x832`, fixture
`library`): a touchpoint whose condition already holds satisfies at mount, and
one whose condition is reached by navigating survives the intermediate click
and satisfies on arrival. Unit tests cover the already-true case, the
press-does-not-end-the-step rule, and the viewport asymmetry in `condition_holds`
(absence only counts from the viewport the touchpoint is bound to, or a second
viewport would report every unrelated node as gone).

### An operational trap worth naming

`target/debug/devpulselabs` built **without** `--features eguipilotai-devtools`
silently disarms every coach proof, and the only symptom is
`RunnerError::Protocol("target disconnected")` from the handshake. A plain
`cargo build -p devpulselabs` from any other session is enough to cause it. The
error should name the likely cause rather than leaving the next session to
rediscover it.

## 14. C1.5f — the coach had nothing to point at (2026-07-27)

C1.5e gave a step the ability to complete on observed state. Asked to walk a
real lesson, the coach then had almost nothing to observe.

### Measured, before

A live `inspect devpulse-cloud library` returned **6 nodes**: five rail tiles
and the chat window button. The Workbench hub — the target of the first
lesson — contributed **zero**. Across the workspace, 99 instrumentation sites
existed, concentrated in chat/window (50), composer (21), wiredfront (11),
viziwiz (7), deck (4) and shell (3). `hubs/workbench`, `hubs/present` and
`hubs/shows` had none. The crate did not even depend on `egui-pilot-ai`.

### Two facts in one string

The rail reported `active+open`, `active`, `open`, `inactive` — the cartesian
product of two independent booleans flattened into a value. No condition could
ask a question of it. `NodeValue` is an exact match by design, so an author
who asked for `active` got silence whenever the accordion happened to be open,
which is a bug that only appears sometimes: the worst kind.

The fields to fix it already existed. `NodeState` has carried `expanded`,
`selected` and `checked` all along, and `pilot_track` already copies them from
`NodeMetadata` into the node. What was missing was the three builder methods to
*set* them — so every caller in the codebase reached for `.value()` instead and
encoded state as prose. The convention was an accident of a missing setter.

### Ruling

`NodeMetadata::expanded/selected/checked` land as builders. The rail's tile
reports `value = active|inactive` **and** `expanded`, as two facts. Sub-rows
keep their `selected`/`available` value — a single fact in a string is merely
unfortunate, not broken, and a committed fixture depends on it — and gain the
typed `selected` alongside it, additively.

Splitting the value would have made fold state unaskable, so the condition
vocabulary grows to match: `CoachCondition::NodeFlag { node, flag, on }` over
`NodeFlagKind` (expanded · selected · checked · focused · disabled). Wire
change: `PROTOCOL_VERSION` 2.5 → 2.6.

**An unreported flag is neither true nor false.** `NodeFlagKind::read` returns
`Option<bool>`, and a node that does not report the flag satisfies neither
`on: true` nor `on: false`. A condition that treated silence as `false` would
advance a lesson on a fact nobody stated. Unit test:
`an_unreported_flag_is_neither_true_nor_false`.

### Evidence

`inspect devpulse-cloud library` on the real exe: **6 nodes → 14**. Eight
`workbench.library.section/*` tabs where there were none, each reporting
`value` and typed `selected`. `root/shell.rail/workbench` now reads
`value=active`, `expanded=false` — the compound string is gone from the tree.

### A wrong inference, corrected by measurement

First reading of the snapshot was that the template nodes "sit below the fold",
making `scroll-into-view` the next binding constraint. Measured, that is false.

The `library` fixture runs `--route workbench/library --focus title`, which puts
the gallery in **solo** mode, so the wall of titles is never drawn. A second
fixture (`library-wall`, no `--focus`) lands on the Library's default section,
which is **seeds** — so the templates wall is not drawn there either. The
Library is a section switcher: exactly one wall renders at a time, by design.
Nothing was hidden; it was never asked to exist.

Both fixtures report **zero clipped and zero hidden nodes**. There is at present
no measured case of a node the coach cannot reach because of scroll position.
`Action::ScrollIntoView` remains genuinely unimplemented — the runtime answers
`scroll-adapter-required` — but nothing observed so far needs it, and building
it now would be building on the inference above rather than on evidence.

### Still open

Instrumentation is proven per fixture, not in aggregate: `library` shows the
solo controls (`template/back`, `template/replay`), `library-wall` shows
`seeds/reload`, and both show all eight section tabs. The Episodes surfaces and
the templates wall are authored but unwitnessed, because no harness flag can
select a Library section without also soloing a template. That flag — not
scrolling — is what a long Workbench lesson needs next.
