# Spike — Blender R0 proof of life (S4 seat prep, pulled forward)

**Status:** IN FLIGHT 2026-08-10 — operator directive: "prove to me that
you are not wasting my time with mock bullshit." Real packaged Blender,
real driving, real signals, this seat, today.
**Related:** [`coach-seam-adr.md`](coach-seam-adr.md) §2.3 (R0 rail),
§4 (Blender = first Tier B pack) ·
[`coach-seam-s2-protocol-rehoming.md`](coach-seam-s2-protocol-rehoming.md)
(S2 continues after; this spike feeds its evidence) ·
[`coach-c2s-foreign-target-spike.md`](coach-c2s-foreign-target-spike.md)
(the spike pattern precedent).

## 1. What must be PROVEN (operator's words, made measurable)

1. **AI triggers file-open of a model** — a real `.blend` opened via
   Blender's embedded Python, `bpy.data.filepath` before/after as the
   observed effect. No pixels, no coordinates.
2. **AI triggers selection** — select a named object;
   `bpy.context.selected_objects` before/after as the effect contract.
3. **Latency measured** — command → observed effect, wall-clock ms per
   verb, written down. The claim "signals happen before you can see it"
   becomes a number.
4. **Operator pace recorded** — the operator clicks objects in the
   viewport at their own pace; Blender's own handlers
   (`depsgraph_update_post`) emit timestamped selection facts into a
   JSONL stream → folded into the ledger as seam spans. Pace = the
   inter-action gaps, computed from real data.
5. **Live paint from the click signal** — a viewport click fact arrives
   → `dpl-coach-overlay` spotlight spawns on Blender's real X11 window
   geometry. The paint is REACTIVE to the operator's action, not
   scripted.

**Honest non-goals (named so nothing is oversold):** the gamified steps
window + rupture animation is the ColdLight lesson-dock slice — it
consumes exactly the signals proven here and is NOT in this spike. No
pack induction (S4 proper), no protocol rehoming (S2, unchanged), no
Wayland.

## 2. Why R0 Python, stated once

Blender's UI is custom OpenGL; its AT-SPI tree is near-empty (known
from the census). The efficient rail is the one Blender itself ships:
the embedded Python API in every packaged install — locale-stable,
version-stable verbs (`bpy.ops.wm.open_mainfile`,
`bpy.ops.object.select_*`), typed observables, and app-native event
handlers that fire in-process, milliseconds ahead of any screen-level
observation. **No Blender source code needed** — the packaged install
carries the whole API. This is the ADR's R0 rung working as designed.

## 3. Design

```
blender (packaged, DISPLAY=:1)
  └─ --python bridge.py         ← loaded at launch, stdlib only
       ├─ TCP localhost:7844     ← JSON-lines command socket (open/select/list)
       ├─ handlers: depsgraph_update_post / load_post
       │     → facts JSONL (ts_ms, kind, objects) → spike-events.jsonl
       └─ every command replies {ok, verb, before, after, ms}

driver (curl / python3 one-liners from the shell)
  → sends open/select, records latency

fold (existing dpl-ledger example path)
  → spike-events.jsonl facts become seam spans (verb="observe",
    mechanism="bpy-handler") so pace lives in Parquet like everything else

paint (existing binaries, zero new UI code)
  → on selection fact: spawn dpl-coach-overlay spotlight with Blender's
    window rect (X11 geometry via coach-runtime path already proven on
    Calculator)
```

Artifacts live under `docs/research/coach/spikes/blender-r0/` (bridge
script + driver + evidence). Spike code is research fuel — promotion
into a crate happens at S4 proper with the pack.

## 4. Steps

1. `sudo apt install blender` (4.0.2 arm64 confirmed available).
2. Author `bridge.py` (stdlib-only socket server + handlers).
3. Generate a model file: default scene saved as `model.blend` by
   blender itself (self-contained, no operator content touched).
4. Drive: open `model.blend`, select `Cube`, deselect, select again —
   record before/after + ms for each.
5. Operator-pace capture: operator (or a scripted stand-in click via
   xdotool if the operator is away — labeled honestly as synthetic)
   selects objects; handler facts land in JSONL with timestamps.
6. Spotlight paint on the selection fact (spawn `dpl-coach-overlay`
   with Blender's window rect).
7. Evidence into §5; signals into ops ledger; STATE note.

## 5. Evidence (2026-08-10, this seat, DISPLAY=:1)

Executed same-day as authored. Blender 4.0.2 (apt, arm64) installed.

| Proof | Result |
|---|---|
| Bridge up inside packaged Blender | `bridge_started` + `bridge_listening` facts, pid 2647254 |
| `ping` | ok, **9 ms** round trip |
| `list` | Camera/Cube/Light enumerated with live selection state, 6 ms |
| `open model.blend` | `effect_observed: true`, before `""` → after full path, **13 ms internal / 16 ms RTT** |
| `select Cube` / `select Camera` / `deselect_all` | all `effect_observed: true` with named before/after, **4–21 ms RTT** |
| Operator-pace stream | every selection change (driven AND clicked) landed as a timestamped `selection_changed` fact in `spike-events.jsonl` |
| Click signal | synthetic viewport click (xdotool stand-in, labeled) → Blender's own handler reported **which object** was picked (`Light`) — **click→named-fact 109 ms**, zero pixels |
| Reactive paint | fact→overlay-spawn 520 ms — **REFUSED by the overlay's AT-SPI presence gate** |

**Named finding (the spike's most valuable output):** Blender is
invisible to AT-SPI (custom OpenGL UI — census confirmed live). The
`dpl-coach-overlay` presence gate is AT-SPI-only
(`linux_self_test.rs::ensure_target_running`), so an R0-driven app can
be DRIVEN and OBSERVED but not yet PAINTED. The overlay needs an **R0
presence arm** (bridge ping = presence) or an X11-window-presence arm —
a small, named follow-on; the X11 geometry path it needs already exists
in the same file (`x11_window_for_class`).

**Verdict:** R0 rail proven end to end on packaged Blender — drive,
observe effects, receive operator click signals by object name in
~100 ms. No Blender source needed. Artifacts:
`docs/research/coach/spikes/blender-r0/` (bridge.py, drive.py,
spike-events.jsonl, model.blend stays untracked scratch).
