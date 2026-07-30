# ADR proposal: Coach overlay process exception (`dpl-coach-overlay`)

**Status:** RATIFIED — operator authorized build 2026-07-30 ("methodically
build… fully built out"); C2b implementation unlocked under the gates below  
**Date:** 2026-07-30  
**Owner:** DevPULSE operator  
**Program map:** [`operator-coach-a2ui.md`](operator-coach-a2ui.md) §3.4 / §4 item 4  
**Spike evidence:** [`coach-c2s-foreign-target-spike.md`](coach-c2s-foreign-target-spike.md) §9–§10  
**Live proof:** [`../journal/2026-07-30-c2s-triggered-live.md`](../journal/2026-07-30-c2s-triggered-live.md)  
**Sibling exception (template):** [`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md)  
**Overlay spike:** [`lesson-overlay-spike.md`](lesson-overlay-spike.md) (operator ruling: own process)
**C2b plan:** [`coach-c2b-overlay-daemon.md`](coach-c2b-overlay-daemon.md)

## Ratification record

The DevPULSE operator authorized execution of this exception on 2026-07-30
and directed a methodical build/verify/journal loop to land `dpl-coach-overlay`.
Ratification unlocks C2b under the launch gates in this document. It does not
waive Linux-first platform order, production wiring, or Tier B/C scope.

## 1. Context

DevPULSE has a deliberate pure-Rust, no-production-IPC law. In-app coaching
(C1 / C1.5*) already works inside `devpulselabs` over the feature-gated
EguiPilot pipe. Teaching *foreign* apps (calculator today; Blender /
SolidWorks later) and painting a whole-monitor lesson chrome require a
window whose root viewport is transparent — and egui-wgpu decides the
transparent backbuffer **once** from the root viewport. Putting that
capability on the main app would force an app-wide render change for one
panel.

The operator therefore ruled (2026-07-27): the overlay is its **own
process** (`dpl-coach-overlay`), with its own root viewport. That is a
**new process boundary** and needs an ADR against the no-IPC law before
daemon code — same discipline as the Chat provider adapters.

C2s (`foreign-target-probe`) measured the physics on this box and closed
live Triggered on 2026-07-30. This ADR freezes the boundary those
measurements imply. It does **not** authorize C2b implementation until
ratified.

## 2. Decision (proposed)

> Permit one named, feature-gated, **devtools-only** process —
> `dpl-coach-overlay` — that paints ColdLight coach chrome over foreign
> (and eventually whole-monitor) targets and speaks the existing framed
> `CoachCommand` / `CoachEvent` protocol on an inherited stdio pipe.
> Production `devpulselabs` builds contain no overlay endpoint. Everything
> else keeps the no-IPC law.

Ratification of this ADR unlocks **C2b** (daemon implementation) only.
It does not unlock Tier B adapters, vision fallback, Windows/Wayland
ship claims, or production (non-devtools) wiring.

## 3. Why an exception is necessary

- **Transparent root required.** Measured: Vulkan/X11 + egui-wgpu share
  one transparent-backbuffer decision from the root viewport
  (`lesson-overlay-spike.md`). A peer viewport inside the main exe is the
  wrong shape.
- **Foreign geometry is OS-local.** GTK4 AT-SPI `SCREEN` extents lie
  (`(0,0)`); compose is WINDOW + X11 origin + `_GTK_FRAME_EXTENTS`. That
  work belongs beside the painter that consumes it, not inside every hub.
- **Same trust model as EguiPilot.** Inherited stdio + framed JSON is
  already the ratified test/devtools pipe. Reusing `CoachCommand` /
  `CoachEvent` avoids a second agent surface.
- **Chat providers already set the exception pattern.** Named boundary,
  launch gates, forbidden set, fail-closed — copy the discipline, not the
  credential world.

## 4. Allowed boundary

Only `dpl-coach-overlay` (and its Linux/Windows backends behind
`cfg` / feature gates) may:

1. **Launch** as a child of a **devtools-gated** owner (CLI /
   `eguipilotai-devtools` path / explicit operator harness) — never from a
   normal production binary, never from model output, MCP args, documents,
   or route context.
2. **Paint** ColdLight vocabulary only: crisp ring, gradient tint, black
   diffusion scatter with holes, caption plate — never backdrop blur.
3. **Position paint in AT-SPI / UIA WINDOW (client) space** when the target
   is a foreign window: overlay sized to the target client rect; hotspot
   rects are WINDOW extents. Screen composition is for observe/actuation
   math and proofs, not a second paint coordinate system.
4. **Observe without interception:** default fully click-through
   (`mouse_passthrough` / empty input region). Triggered via Sync-grab +
   `ReplayPointer` (X11), equivalent non-eating observe on other backends,
   and/or semantic state change that **matches the coached control**.
5. **Identity-gate before paint:** `GetAccessibleAtPoint` (or UIA hit-test)
   at the hotspot center must name-match; else `Refused`, never paint at
   `(0,0)` or a stale rect.
6. **Speak** only framed `CoachCommand` / `CoachEvent` on the inherited
   pipe (same enums the in-app coach already uses). Capability
   `coach.desktop-overlay` flips to *supported* only in builds that
   include the daemon behind the gate.
7. **Semantic invoke** (`Action.doAction` / UIA Invoke) as an explicit
   command arm, separate from observe — never implied by mount.
8. **Self-test** (`--self-test`): paint a hotspot, prove non-interception,
   prove Triggered on a known control, exit.

## 5. Forbidden

- Shipping the overlay endpoint or AT-SPI/UIA deps in a normal production
  `devpulselabs` build.
- Generic command runners, shell concatenation, or “drive anything”
  surfaces.
- Credential handling of any kind.
- Reading foreign app **content** beyond roles, names, actions, and
  geometry needed to point and verify (same line as C2s §5 / chat ADR).
- Trusting GTK4 `GetExtents(SCREEN)` (or equivalent lies) as screen truth.
- Painting caption chrome that overlaps sibling controls as if it were the
  hotspot (caption prefers above / clear of the keypad).
- Treating any display/text change as Triggered without matching the
  coached control.
- XTEST / synthetic pointer into foreign windows except under an explicit
  proof or future C4 actuation command — observe path must remain
  ReplayPointer / non-eating.
- GNOME Wayland whole-screen overlay as a supported target (no layer-shell;
  documented fallback: stay X11 or wlroots).
- Fabric gaining an agent-facing channel; `/content` mutation; prior-app
  writes.
- Vision / SigLIP as the primary locate path while Tier A is available.

## 6. Launch gates (fail-closed)

C2b may not claim done, and no harness may auto-start the daemon, until:

| Gate | Bar |
|---|---|
| Feature | `eguipilotai-devtools` (or successor overlay feature) required to build/link the daemon into any operator path |
| Owner | Exactly one lifecycle owner; stop / crash / timeout / parent death reaps the child |
| Protocol | Serde roundtrip + exhaustive match on overlay-relevant `CoachCommand` / `CoachEvent` arms |
| Identity | Mount refuses unless stream hit-test name-matches |
| Observe | Self-test: Sync-grab (or backend equiv.) Triggered on known control; click-through elsewhere |
| Adversary | Wrong control → timeout/refuse; dead target window → unmount/refuse; no `(0,0)` paint |
| Platform | X11 Linux proven first (this box); Windows layered + UIA second; wlroots third; GNOME Wayland unsupported |
| Structure | New crate(s) follow `docs/design/architecture.md`; Linux-only deps `cfg`-gated so `green-windows` stays honest |
| Docs | `doctor` / capability bit `coach.desktop-overlay` honest; AGENTS.md gains a one-line pointer only after ratification |

## 7. Runtime profile (C2b shape — not built by this ADR)

```
dpl-coach-overlay
  ├── stdio framed CoachCommand / CoachEvent
  ├── backend_x11   (atspi + x11rb; WINDOW paint; Sync-grab observe)
  ├── backend_win   (UIA + layered window; later)
  └── backend_wlr   (layer-shell; later)
```

- Process is orphaned from `crates/app` production graph (spike precedent:
  `dpl-desktop-spike` stays a probe; daemon is the productized sibling).
- In-app coach (C1*) and desktop overlay share protocol types; they do not
  share a process or a wgpu device.
- `Target::ScreenRect` / foreign `SemanticNode` emission remain **C3** —
  this ADR only bounds the painter/observer process.

## 8. Verification target (for ratification closeout + C2b)

When the operator ratifies and C2b lands, close with three modes:

1. **Correctness** — unit tests for compose + identity refuse; protocol
   roundtrips.
2. **Reality** — `dpl-coach-overlay --self-test` against live
   `gnome-calculator` on X11: stream_hit match, ring on coached control,
   caption clear of siblings, operator or XTEST press → `Triggered`,
   click-through elsewhere.
3. **Adversary** — wrong digit / dead window / unsupported visual →
   refuse or timeout; Windows green does not compile Linux-only deps.

Spike commands that already witness the physics (pre-daemon):

```bash
cargo test -p dpl-desktop-spike --lib
foreign-target-probe locate --name 7
foreign-target-probe spotlight --name 7 --proof-xtest --debug-pointer
foreign-target-probe spotlight --name 7 --seconds 60 --message "Press 7" --debug-pointer
```

## 9. Ratification

**RATIFIED 2026-07-30** — operator directed full C2b build-out.

On ratify: tip `AGENTS.md` with a one-line pointer (devtools-only overlay
process; chat providers remain the production-process exception);
Status = RATIFIED; C2b plan = [`coach-c2b-overlay-daemon.md`](coach-c2b-overlay-daemon.md).

## 10. Boundaries (summary)

| In | Out |
|---|---|
| ADR text + ratification record | C2b daemon code |
| Laws derived from C2s live proof | Tier B/C, SolidWorks, SigLIP gate |
| Devtools-only process exception | Production IPC, fabric agent channel |
| X11-first platform order | Claiming Wayland GNOME overlay support |
