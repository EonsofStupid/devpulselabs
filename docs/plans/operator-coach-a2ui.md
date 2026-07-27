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
  `accesskit`; on Linux `accesskit_unix` (0.22.x) exposes the tree over
  AT-SPI via zbus. Upstream caveat: 0.22.0 panics on a broken AT-SPI bus
  (AccessKit #727; fix PR #741, 2026-07).
- **This box:** GNOME on **X11** (`XDG_SESSION_TYPE=x11`). AT-SPI bus
  running; `org.gnome.desktop.interface toolkit-accessibility` = **false**
  (toolkit apps other than AccessKit ones won't publish trees until
  flipped). No ydotool — irrelevant on X11, XTEST covers input synthesis.
  Second target is the Windows laptop (UI Automation + layered windows).
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

Today agents poll `ClientCommand::Snapshot`. The coach loop must instead
**subscribe to the streams**: AG-UI's snapshot/delta pattern mapped onto
`SemanticFrame` — one full frame on subscribe, then per-frame deltas (node
appeared / moved / state-changed / removed) as `RuntimeEvent`s. Screenshots
are demoted to proof artifacts only; the agent's picture of the screen is a
live typed model. Triggers like `NodeAppeared` / `ValueChanged` are filters
on this delta stream, and `FabricEvent` triggers subscribe the router to the
app's own signal fabric (e.g. fire a touchpoint when `BuildEvent::Finished`
lands). Deltas are computed against the already-per-frame `SemanticFrame`
in the plugin — no new instrumentation, just diffing + backpressure (bounded
channel, coalesce on lag, `STATE_SNAPSHOT`-style resync on overflow).

### 3.3 In-app rendering (C1)

Runtime resolves `at` against `latest_frame()`, then paints an
`egui::Area` at `Order::Foreground` with `Sense::click` over the node rect.
ColdLight vocabulary only: crisp edge ring, gradient tint, black diffusion
scatter dimming outside the rect — never backdrop blur. Click inside →
`Triggered`; Esc or agent `Unmount` → `Unmounted`; missing/clipped target →
`Refused`.

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
   Router-shaped protocol arms from day one (`Mount`/`Unmount`/`Triggered`
   + roundtrip tests) but only `visual: Spotlight`, `on: Press` implemented
   → runtime handling + resolver → egui Foreground renderer (ColdLight) →
   CLI verb `eguipilotai coach mount <node> --message <s> --once` →
   scenario TOML step (`coach.mount` + `expect triggered`) → Test Lab
   "coach" tab reusing the same commands. Unimplemented `TriggerKind`s
   answer `Refused { Unsupported }` — no silent stubs. Amends delivery-doc
   Slice 8 rather than forking it.
2. **C1.5 — live semantic stream.** Subscribe command, full-frame-then-delta
   events over the pipe (diff of `SemanticFrame`, bounded + coalescing,
   resync on lag) → unlock `NodeAppeared`/`ValueChanged`/`FabricEvent`
   triggers and demo-mode pacing (`Triggered.actor: Agent`).
3. **C2a — coach-overlay ADR** (boundary, launch gates, forbidden set;
   ratify before code). **C2b — daemon**: X11 backend first (this box),
   Windows backend second, layer-shell third.
4. **C3 — ColdLight catalog + Surface commands**, in-app renderer first.
5. **C4 — desktop semantic control**, external-MCP-first evaluation, then
   the `atspi` adapter if internalization earns its keep.

Each slice is its own pass with its own plan §8 evidence; this document is
the program map.

## 5. Boundaries

- Coach signals live on the **pilot pipe**, gated by `eguipilotai-devtools`.
  Normal production builds contain no coach endpoint (AGENTS.md law holds).
- Fabric gains **no** agent-facing channel. An in-process `CoachEvent`
  fabric domain is deferred until a hub actually needs to react to coaching.
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
- C1.5: subscribe → mutate a fixture route → asserted delta sequence
  matches; lag injection proves coalesce + snapshot resync (no unbounded
  buffering); a `NodeAppeared` touchpoint fires from the delta stream alone.
- C2: `dpl-coach-overlay --self-test` paints a hotspot, injects an XTEST
  click, asserts `Triggered` — CI-friendly on the Linux lane under Xvfb.
- `cargo green` both lanes; exe rebuilt, mtime moved.

## 7. Commit plan

- This plan lands alone first (branch `plan/operator-coach-a2ui`, PR, green).
- C1 lands as one pass (`coach-c1-touchpoint`), delivery-doc Slice 8
  amended in the same PR; C1.5 (`coach-c15-semantic-stream`) follows as its
  own pass. C2a ADR is its own PR before any C2b code.

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
1. One-time: `gsettings set org.gnome.desktop.interface
   toolkit-accessibility true` — lets GTK/Qt/Electron apps publish AT-SPI
   trees (needed for C4; DevPULSE itself already publishes via AccessKit).
2. Stay on the X11 session for coach work — best overlay + input story
   today. If you later want Wayland: Hyprland/Sway/niri give the full
   layer-shell experience; GNOME Wayland does not.
3. Nothing to install for input (X11 = XTEST built in). `ydotool` only
   becomes relevant on a Wayland session.
4. Windows laptop: no changes — layered windows + UI Automation are native.

**Language verdict: stay pure Rust.** Every layer has first-class Rust:
winit/egui/AccessKit (UI + a11y), x11rb (XShape/XTEST), smithay-client-toolkit
(layer-shell), zbus/atspi (AT-SPI), windows-rs (Win32). A2UI/AG-UI are JSON
protocols — serde enums, no runtime to import. The single genuine non-Rust
corner is a GNOME-Shell extension (JavaScript) if GNOME-Wayland overlays were
ever required; the ruling here is: don't — X11 today, wlroots if Wayland
later. No expansion from Rust is warranted.
