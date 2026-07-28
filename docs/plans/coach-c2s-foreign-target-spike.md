# Coach C2s — foreign-target spike (can we point at an app we did not write?)

A probe, not a product. One question answered on the real desktop: can
DevPULSE resolve a control inside an application it did not write, paint the
shipped ColdLight spotlight on it at true screen coordinates, observe the
operator press it, and separately invoke it by identity with no coordinates at
all? Ground-truthed 2026-07-27 against this box, not against memory.

Operator rulings that scope this pass (2026-07-27):

1. First target is **this ARM64 Linux box** (X11), not the Windows laptop.
2. Deliverable is **proof of concept only** — no product surface, no daemon.
3. Foreign-host plug-ins are **ratified narrowly**: a plug-in may only
   marshal our typed protocol onto the host's main thread and carry no logic
   of its own. (Not exercised by this spike; recorded because it unblocks
   Tier B later.)
4. Lesson one is a **Tier A app** — prove the accessibility path with zero
   per-app adapter work.
5. Spike runs **before** the C2a ADR, as an explicitly non-product probe, and
   its findings feed the ADR. See §5.

## 1. Context

The operator's ask is larger than C1 delivered: the whole monitor paintable,
DevPULSE reading OS and application state, and foreign applications (Blender,
SolidWorks named explicitly) opened and taught step by step — without "the
sloppy screenshot concept that will never work."

That instinct is correct and is now the industry consensus rather than a
preference. Microsoft's UFO² (TMLR, May 2026) builds exactly this for Windows
and treats vision as a *fallback*: it reads UI Automation for structure, runs
OmniParser-v2 only to find controls UIA cannot see, and discards any visual
detection overlapping a UIA control by more than 10% IoU. Screenshots fill
unnamed rectangles. They never drive.

The program decomposes into four stacks that are routinely confused for one:

- **Paint** — a click-through, always-on-top, per-monitor overlay with carved
  input hotspots. Mechanically well-trodden; the renderer already exists.
- **Perceive** — what is on screen, semantically. Tiered (§3.1).
- **Actuate** — prefer *semantic invoke* by identity over synthesized input;
  and for a teaching product the default is **no actuation at all** — the
  agent points, the operator clicks. That is already C1 doctrine
  (`operator-coach-a2ui.md` §3.3) and it is the differentiator over
  computer-use agents.
- **Know** — durable per-app capability packs and induced workflows. The
  expensive stack, and the one the operator correctly identified as needing
  hours of offline preparation. Out of scope here; named in §3.4.

This spike touches only Paint, Perceive, and the top rung of Actuate, against
one app, to retire the largest unknown before anything is designed on top of
it.

## 2. Current state — ground truth (probed 2026-07-27, this box)

Every fact below was measured in-session, not recalled.

**The machine.** `aarch64` (NVIDIA GB10, driver 580.173.02). GNOME Shell 46.0
on **X11** (`XDG_SESSION_TYPE=x11`), real Xorg on vt2, display `:1` at
1920×1080 (351×191 mm). `gnome-shell` is the compositing WM, so ARGB
transparency is available.

**X11 extensions present:** `SHAPE`, `XTEST`, `XFIXES`, `COMPOSITE`, `DAMAGE`,
`RECORD`. Everything the overlay and input synthesis need.

**`xrandr --listmonitors` reports `Monitors: 0`** despite the working
1920×1080 display — an NVIDIA RandR quirk. Overlay geometry must come from the
root window, never from RandR. This would have been a silent multi-monitor bug.

**Accessibility is already live, and the plan's §8 operator step is wrong.**
`operator-coach-a2ui.md` §8 states that toolkit apps other than AccessKit ones
will not publish trees until `org.gnome.desktop.interface toolkit-accessibility`
is flipped. That key is still **`false`** on this box, and **20 applications
publish AT-SPI trees anyway** — GTK3 (`gnome-terminal-server`) and GTK4
(`gnome-calculator`, `org.gnome.Nautilus`) alike. The a11y bus runs at
`unix:path=/run/user/1000/at-spi/bus_1`. **Amend §8: no operator setup is
required for GTK.** (Qt and Electron are unverified; see the Electron finding.)

**Identity and actions are clean.** `gnome-calculator` exposed **24 named,
actionable controls**, each advertising a `click` action through the AT-SPI
`Action` interface. Semantic invoke requires no geometry whatsoever.

**Geometry is the trap.** GTK4 reports `CoordType.SCREEN` extents as
`(0,0)` — correct sizes, no position — for widgets *and* for the toplevel
frame. GTK3 reports SCREEN correctly. A naive reader would paint every GTK4
spotlight in the monitor's top-left corner and look like it worked.

Measured, `gnome-calculator` button `7`:

| Source | Value |
|---|---|
| AT-SPI `SCREEN` extents | `(0, 0, 64×44)` — broken |
| AT-SPI `WINDOW` extents | `(16, 293, 64×44)` — correct |
| AT-SPI toplevel frame | `SCREEN=(0,0,360×491)` — broken |
| X11 window `0x3a00004` | origin `(5, −23)`, `482×613` |
| `_GTK_FRAME_EXTENTS` | `61, 61, 55, 67` (l, r, t, b) |

The client-side-decoration shadow explains the discrepancy exactly:
`482 − 61 − 61 = 360` and `613 − 55 − 67 = 491`, reconciling to AT-SPI's frame
size to the pixel. Composition therefore resolves button `7` to screen rect
`(82, 325, 64×44)`, click point `(114, 347)` — plausible against the GNOME top
bar, and self-consistent in both axes.

**Electron publishes nothing.** `cursor` is on the a11y bus but exposed zero
named components. Chromium enables accessibility lazily and needs
`--force-renderer-accessibility`. Relevant because VS Code-class apps would
otherwise be assumed free.

**Dependencies are already in the tree.** `Cargo.lock` holds `atspi 0.29.0`,
`atspi-proxies 0.13.0`, `x11rb 0.13.2`, `zbus 5.18.0` — all transitive (via
`accesskit_unix` and `winit`), none declared in the workspace `Cargo.toml`.
The spike **promotes existing locked crates to direct deps** and pulls no new
third-party tree. Also noted: locked `accesskit_unix` is **0.21.1**, not the
0.22.x the program map §2 assumed — so the AccessKit #727 broken-bus panic
note needs re-checking against the version we actually build.

**Named apps are out of reach here, and one is out of reach entirely.**
SolidWorks is Windows x64 only and **cannot run on this machine at all**;
it is strictly a Windows-laptop deliverable. Blender ships **no official
Linux arm64 build** (x64 and macOS-arm64 only) — distro packages exist but
link the system Python, which breaks addon dependency assumptions. Neither is
a Tier A app in any case: both draw their own UI and publish no usable
accessibility tree, which is precisely why they are Tier B (§3.1).

**What C1/C1.5a/C1.5c already shipped that this reuses verbatim:**
`CoachPalette`, `CoachStore` with its TTL and mount ledger, the single-scatter
dimmer with one hole per lit target (scanline decomposition), the
`Mount → Armed/Mounted → Triggered → Unmounted/Refused` lifecycle, and
`CoachActor` attribution. `crates/egui-pilot-ai/src/coach.rs`. The protocol's
`coach.desktop-overlay` capability is declared and **unsupported** — this spike
does not flip it.

## 3. Target design

### 3.1 The tier ladder (the load-bearing decision)

Every foreign app classifies into a tier; an adapter always uses the highest
tier available. This is the architecture, not a nicety, because the operator's
two named apps land in Tier B and would be invisible to a Tier-A-only design.

| Tier | Perceive | Actuate | Covers |
|---|---|---|---|
| A | AT-SPI2 (Linux) / UI Automation (Windows) | `Action.doAction` / `InvokePattern` | GTK, Qt, Electron, WinUI, WPF, Win32 |
| B | the app's own automation API | native call | Blender (`bpy`), SolidWorks (COM), Office, AE |
| C | vision detection on unnamed regions | synthesized input at a rect | canvas-drawn custom widgets |

This spike proves **Tier A only**, on one app.

### 3.2 The composition rule

AT-SPI `SCREEN` coordinates are not trustworthy and must never be used
directly. Derive the toplevel's visible origin once per window, then treat
every descendant's `WINDOW` rect as relative to it:

```
origin       = x11_window_abs_origin + _GTK_FRAME_EXTENTS[left, top]
screen_rect  = origin + atspi_WINDOW_rect
```

One code path serves GTK3 and GTK4, because GTK3's correct `SCREEN` value
becomes a cross-check rather than a second branch. The toplevel is correlated
to its X11 window by matching AT-SPI's process id against `_NET_WM_PID` over
`_NET_CLIENT_LIST`. Where `_GTK_FRAME_EXTENTS` is absent (non-GTK, server-side
decorations) the margin is zero, which is the correct answer for those apps.

A window whose origin cannot be resolved must **refuse**, never paint at
`(0,0)` — the failure this probe caught is exactly the failure that would
otherwise ship.

### 3.3 The spike

A single workspace binary, `crates/desktop-spike` (`dpl-desktop-spike`), doing
five things and nothing else:

1. **Resolve** — `atspi` finds a target in `gnome-calculator` by role + name.
2. **Locate** — `x11rb` supplies the X11 origin and `_GTK_FRAME_EXTENTS`;
   compose per §3.2.
3. **Paint** — override-redirect ARGB window at root-window geometry, `XShape`
   input region empty (fully click-through), painting the shipped ColdLight
   spotlight vocabulary at the composed rect: crisp edge ring, gradient tint,
   one black diffusion scatter with a hole for the target, caption plate.
   Never backdrop blur.
4. **Observe** — carve a single `XShape` input rectangle at the target and
   report `Triggered` when the operator presses it, mirroring C1's
   observe-never-intercept law: the real button still does its real work.
5. **Invoke** — separately drive the same control through
   `Action.doAction("click")` with no coordinates, establishing the
   semantic-first rung of the actuation ladder and proving the two halves are
   independent.

Deliberately **not** in this spike: the `dpl-coach-overlay` daemon, the pilot
protocol's `Target::ScreenRect` arm, Windows, Wayland, multi-monitor, Tier B
or Tier C, workflow induction, and any product surface. Each is named in §3.4
so nothing is dropped silently.

### 3.4 Where this leads (named, not built)

C2a ADR (boundary, informed by this spike) → C2b `dpl-coach-overlay` daemon →
C3 Tier A perception emitting the *same* `SemanticNode` shape the pilot
already speaks, so `Locator`, the touchpoint router, all five triggers and the
MCP tools work on foreign apps with no new agent surface → C4 the actuation
ladder (semantic invoke, then XTEST, then `libei`/`reis` + the RemoteDesktop
portal for Wayland; `libei.so.1` and `libeis.so.1` are already installed here)
→ C5 the `AppAdapter` trait plus the first Tier B pack → C6 capability packs
and workflow induction → C7 SolidWorks on the Windows lane → C8 bounded
vision fallback → C9 the lesson product.

On C6, the industry name for what the operator described as "cycle through
workflows for hours to build a library" is **Agent Workflow Memory**: induce
abstract reusable workflows from successful traces rather than storing raw
trajectories. Worth building because it measurably works — +51.1% relative
success on WebArena with *fewer* steps per task — and it fits the house
doctrine: a versioned, hash-fingerprinted pack per app+version (the
`dptools-deps` precedent), verified by reading state back through Tier A or B,
never by diffing a screenshot.

## 4. Changes (execution order)

1. Promote `atspi` and `x11rb` to workspace deps at their already-locked
   versions (**new-workspace-dep ratification required**, per the `tungstenite`
   and `zip` precedents).
2. `crates/desktop-spike`: AT-SPI resolution behind a small typed result, so
   a broken bus degrades rather than panics (Law 4).
3. The X11 origin + `_GTK_FRAME_EXTENTS` composition, with the reconciliation
   arithmetic as a unit test using the real measured numbers from §2.
4. The override-redirect ARGB overlay + `XShape` click-through and one hotspot.
5. ColdLight spotlight paint reusing `CoachPalette` ink; the scatter-with-hole
   approach ported from `coach.rs` rather than reinvented.
6. Press observation → `Triggered`, and the coordinate-free
   `Action.doAction` path as a separate subcommand.

## 5. Boundaries

- **This is a probe, and it is not the daemon.** It ships behind the
  `eguipilotai-devtools` gate, is not wired into `devpulselabs`, is not
  reachable from any production build, and declares no new protocol
  capability. `coach.desktop-overlay` stays *unsupported*.
- **The C2a ADR still gates C2b.** The operator ruled (2026-07-27) that the
  spike runs first because the probe already overturned two §8 assumptions
  (the accessibility flag, the `accesskit_unix` version) and would likely
  overturn more. The ADR is authored *from* this spike's findings, before any
  daemon code. This document does not grant C2b permission.
- **Read and point. Do not drive.** The spike synthesizes no input into
  foreign apps except the one explicit `Action.doAction` proof, which is a
  semantic invoke, not a cursor. No XTEST injection into foreign windows.
- **Forbidden set:** no credential handling, no generic command runner, no
  reading of foreign app *content* beyond the roles, names, actions and
  geometry needed to point at a control. Same line the chat provider ADR drew.
- Fabric gains no agent-facing channel. `/content` untouched. The prior app
  stays read-only reference.

## 6. Verification

- **Unit:** the §2 reconciliation as a test with the measured constants —
  `482 − 61 − 61 = 360`, `613 − 55 − 67 = 491`, and button `7` composing to
  `(82, 325, 64×44)`. A window with no resolvable origin returns a refusal
  rather than `(0,0)`.
- **Real artifact:** the spike run against a live `gnome-calculator`, proving
  in one pass that the spotlight lands on button `7`, that pressing it both
  reaches the calculator (the display changes) and reports `Triggered`, and
  that the overlay is otherwise fully click-through.
- **Proof shot:** a real screen capture showing our ring and caption on a
  foreign app's button inside the dim. Per the C1.5c finding, coach baselines
  are reviewable images rather than byte-exact gates.
- **Semantic invoke:** `Action.doAction("click")` changes the calculator's
  displayed value with the overlay absent entirely.
- **Negative:** a bogus role/name refuses by name; a killed target app
  unmounts rather than painting a stale rect.
- `cargo green` on the Linux lane.

## 7. Commit plan

This plan lands alone first (branch `plan/coach-c2s-foreign-target-spike`, PR,
docs-only so CI is skipped). The spike is a second PR on its own branch
(`coach-c2s-foreign-spike`) carrying the §2 amendments to
`operator-coach-a2ui.md` §8 in the same PR, since those are corrections to a
ratified document and must not drift. The C2a ADR is a third PR, after the
spike reports.

## 8. Evidence — grounding cheat sheet (2026-07-27)

**Probed in-session on this box** (see §2 for values): session type, GNOME
version, X11 extension list, a11y bus address, the 20 publishing applications
and their names, `gnome-calculator`'s 24 named actionable controls, the
SCREEN-vs-WINDOW extent divergence across GTK3/GTK4, `_GTK_FRAME_EXTENTS`,
the X11 window match by `_NET_WM_PID`, the RandR zero-monitor quirk, the
absence of any creative app, and the locked dependency versions.

**Tooling verified present:** `atspi` + `x11rb` + `zbus` in `Cargo.lock`;
`libei.so.1`, `libeis.so.1`, `libatspi.so.0` installed; `xwininfo`, `xprop`,
`busctl`, and the `Atspi` GObject-introspection typelib available for probing.

**Perception / hybrid grounding**
- UFO²: The Desktop AgentOS, TMLR May 2026 — hybrid UIA + OmniParser-v2
  fusion, IoU > 10% dedup, vision wrapped as pseudo-UIA nodes, per-app
  AppAgents with native APIs, knowledge substrate over docs + history.
  https://arxiv.org/abs/2504.14603
- Agent Workflow Memory — workflow induction from traces; +24.6% (Mind2Web)
  and +51.1% (WebArena) relative success, fewer steps.
  https://arxiv.org/abs/2409.07429

**Linux desktop stack**
- `atspi` 0.30.0 published (0.29.0 locked here); pure Rust over zbus.
- `libei`/EIS is the standard Wayland input-emulation transport, in the XDG
  RemoteDesktop and InputCapture portals since 1.17; Rust binding is `reis`
  (used by `ashpd`). Xwayland already translates XTEST to libei via the portal.
  http://who-t.blogspot.com/2026/07/libei-integrations-in-xdg-remotedesktop.html
- Newton (Wayland-native a11y, push-based, built on AccessKit) remains a
  **prototype** as of July 2026 — protocols unfinalized, not upstream. AT-SPI2
  is the only shippable Linux semantic layer today.
- GNOME Wayland has no wlr-layer-shell, so whole-screen overlay dies on a
  Wayland GNOME session. Ruling stands: stay X11; wlroots is the upgrade path.

**Windows stack (for C7, not this spike)**
- `uiautomation` 0.25.0 (Rust wrapper over UIA); `xa11y` 0.11.0 is a newer
  cross-platform a11y client worth evaluating rather than assumed.
- App Actions on Windows: apps register actions via MSIX
  `windows.appExtension` + `com.microsoft.windows.ai.actions`, invoked by COM
  `IActionProvider` or URI launch. Requires package identity — so it covers
  packaged modern apps, **not** SolidWorks.
  https://learn.microsoft.com/en-us/windows/ai/app-actions/

**Tier B patterns (for C5/C7)**
- Blender: addon opens a loopback socket and marshals work onto the main
  thread via `bpy.app.timers.register`, because `bpy` is not thread-safe. Buys
  enumeration of `bpy.ops`, the keymap, and area/region geometry — thousands
  of verified operators with no vision and no exploration guesswork.
- SolidWorks: must be an **in-process** COM add-in (`ISwAddin`). Out-of-process
  COM marshalling is unreliable, and event sinks only fire in-process; the
  documented pattern for an external process is the add-in servicing a queue
  on `ISldWorks::OnIdle`. Both satisfy operator ruling 3 (marshal only).

**Not needed, explicitly:** kernel or eBPF work. Every OS signal this program
wants is userspace — window/focus from AT-SPI or UIA events, file changes from
`fanotify` / `ReadDirectoryChangesW`, process lifecycle from `/proc` or ETW,
and app-internal facts from the app's own event API (`DSldWorksEvents`,
`bpy.app.handlers`), which is strictly richer than anything the kernel sees.
eBPF would deliver syscall noise where lesson-level meaning is required.
