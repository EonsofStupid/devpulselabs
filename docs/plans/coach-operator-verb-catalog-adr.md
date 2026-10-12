# ADR — Coach operator verb catalog (the 101)

**Status:** RATIFIED 2026-08-01 — operator `go` · CX-events-01 implements P0 observe/window slice  
**Amended 2026-08-11 (S2 rehoming, operator `gfo`):** the catalog's ONE
home is now `egui_pilot_ai_protocol::seam` — typed `SeamVerb` +
`seam_verb_catalog()` (21 entries: the 20 below **plus `geom`**, which
shipped 2026-08-01 as an uncataloged alias of `window_geom` — the S2
census made it a first-class row). §2.1's "own the catalog in
`dpl-coach-runtime` / daemon JSON" is superseded: the runtime keeps the
ENGINE, the protocol owns the VOCABULARY, and the daemon is a
translator (verbs parse at the HTTP edge; answers are typed
`EffectContract`s; `/events` emits typed `SeamEvent`s). Extension verbs
ride the `x.<namespace>.<verb>` arm — additive, never a breaking bump.
Plan: [`docs-history-2026-10-12:docs/plans/coach-seam-s2-protocol-rehoming.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/plans/coach-seam-s2-protocol-rehoming.md).

**Related:** [`docs-history-2026-10-12:docs/plans/coach-control-plane-adr.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/plans/coach-control-plane-adr.md) ·
[`docs-history-2026-10-12:docs/plans/coach-bridge-fabric-runtime.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/plans/coach-bridge-fabric-runtime.md) ·
[`../research/coach/bleeding-edge-control-plane.md`](https://github.com/EonsofStupid/devpulselabs/blob/docs-history-2026-10-12/docs/research/coach/bleeding-edge-control-plane.md)

## 1. Context

Control plane v1 can **mint → resolve → invoke** with presence + effect gates.
That is not enough for dynamic teaching. The operator needs an **app-agnostic
verb vocabulary** — open, close, move, resize, focus, click, type, select,
scroll, wait, assert — backed by **fabric identity** and a **live event stream**,
not screenshots.

Industry peers (Geisterhand HTTP, gnome-ui-mcp snapshot/uid, Odilia `atspi`
event streams, xa11y locators) converge on the same loop: find → act → verify.
DevPULSE keeps the product law: **our daemon + fabric**, peers as reference.

## 2. Decision

1. **Own the catalog** in `dpl-coach-runtime` / daemon JSON — typed Rust enums,
   stable wire names (`snake_case`). Do **not** depend on third-party MCP at
   runtime for production Coach.
2. Every verb targets either:
   - a **fabric address** (seed / path / role+name fingerprint), or
   - an **app/window handle** (presence-gated: mapped + pid alive).
3. Every mutating verb returns an **effect contract** result:
   `effect_observed: bool` + typed before/after observables (text, rect,
   focus, selection, running).
4. Ship a **normalized event stream** on the control plane
   (`GET /events` SSE) that emits the same verb-shaped facts when the
   *operator* or the *agent* changes the world (X11/UIA geometry first;
   AT-SPI/UIA Object/Window/Focus via Odilia `atspi` as follow-on).
5. Screenshots remain **import / harvest / proof**, never the lesson hot path.
6. MCP (if ever) is a **thin adapter** over this catalog — not a second brain.

## 3. Verb catalog v0 (the 101 core)

Wire name = JSON `verb`. Platform backends map to AT-SPI Action / UIA pattern
or window manager primitives. `P0` = next implementation pass.

| Verb | Target | Observable (min) | Priority |
|---|---|---|---|
| `inventory` | desktop | app list + `operator_real` | shipped |
| `presence` | app | mapped, pid, outer rect | shipped |
| `import` | app | fabric session + seed | shipped |
| `resolve` | fabric | role, name, window_rect, actions | shipped |
| `invoke` / `click` | fabric | effect (e.g. display text) | shipped |
| `mount` | fabric | MountSpec / overlay | shipped |
| `open` | app id / exe | presence becomes real | shipped (CX-verbs-p0-02) |
| `close` | app/window | presence becomes false | shipped (CX-verbs-p0-02) |
| `focus` | app/window | focus event / active | shipped (CX-events-01) |
| `move` | window | outer origin delta | shipped (CX-events-01) |
| `resize` | window | outer size delta | shipped (CX-events-01) |
| `type_text` | app (named Action / editable) | text before→after | shipped (CX-verbs-p0-02) |
| `press_key` | app/window | optional text/focus effect | shipped (CX-verbs-p1) |
| `select` | app | selection / checked / selected | shipped (CX-verbs-p1) |
| `scroll` | app | bounds / value / child offset | shipped (CX-verbs-p1) |
| `drag` | fabric→fabric \| point | bounds / drop effect | P2 |
| `wait_until` | predicate | timeout \| matched event | shipped (CX-verbs-p0-02) |
| `assert_state` | fabric/app | pass/fail + observed | shipped (CX-verbs-p0-02) |
| `observe` | session | subscribe events (SSE) | shipped (CX-events-01) |

Non-goals for v0: OCR verbs, coordinate-only click as primary, remote-desktop
pixel sessions, binding `0.0.0.0`.

## 4. Event → verb normalization

Listeners produce **facts**; the catalog names them. Seat evidence
(2026-08-01 capture):

| Raw signal | Normalizes to |
|---|---|
| X11 `ConfigureNotify` / geom poll | `moved` / `resized` |
| AT-SPI `object:bounds-changed` | `layout_changed` (controls reflow) |
| AT-SPI `object:text-changed:*` | `text_changed` |
| AT-SPI `object:state-changed` | `state_changed` (focus, checked, …) |
| AT-SPI `window:activate` / deactivate | `focused` / `blurred` |
| AT-SPI `window:move` / `resize` | **unreliable on this seat** — prefer X11/UIA |

Linux implementation preference: Odilia `atspi` `event_stream` inside
`dpl-coach-daemon` (already a workspace dep). Window move/resize: X11
(`x11rb`) now; Wayland later via compositor/libei peers.

## 5. Control plane additions

| Method | Path | Role |
|---|---|---|
| GET | `/events?app=&seconds=` | SSE of normalized facts (geom poll v1) |
| POST | `/verb` | `{ "verb", "app", "x"?, "y"?, "w"?, "h"? }` → effect |
| GET | `/verbs` | catalog introspection for agents/Teacher |

Existing endpoints remain; `/verb` wraps window verbs first, then grows.

## 6. Consequences

- Teacher / agents speak **verbs + fabric**, never pixels.
- Cursor conquest and Calculator regression share one catalog.
- Research peers (Geisterhand, gnome-ui-mcp, xa11y, wdotool) inform shapes
  only — see research note.
- **CX-events-01** implements observe + move/resize/focus on this catalog.

## 7. Ratification

**RATIFIED** 2026-08-01 by operator (`go` after draft review). Catalog names
and effect contracts are the Teacher/agent contract going forward.
