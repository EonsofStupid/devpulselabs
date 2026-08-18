# 2026-08-10 — Blender R0 spike proven · mock rev 4 (naming + HUD)

Lane: coach-seam. Two deliverables this session, both uncommitted and
awaiting operator red-pen before one docs PR. S2 remains the next code
slice, its plan still halted for approval
(`docs/plans/coach-seam-s2-protocol-rehoming.md`).

## 1 · Blender R0 proof of life — PROVEN

The operator demanded proof the seam is "not mock bullshit": drive a
packaged Blender install, open a model, make selections, capture the
click signal at machine pace, paint feedback. Pulled forward from S4 as
a spike (`docs/plans/coach-seam-s4-spike-blender-r0.md`, §5 has the
full evidence table).

What happened on this seat (Blender 4.0.2 apt/arm64, DISPLAY=:1):

- **Bridge:** stdlib-only Python loaded via `--python`. A socket thread
  ENQUEUES commands; `bpy.app.timers` drains them on the main thread
  (bpy is not thread-safe). Blender's own persistent handlers
  (`depsgraph_update_post`, `load_post`) emit timestamped facts to
  `spike-events.jsonl` — the operator-pace stream.
- **Drive:** ping 9 ms · `open model.blend` 16 ms RTT,
  `effect_observed:true` with before/after paths · select/deselect
  4–21 ms with named before/after objects.
- **Click signal:** a synthetic viewport click (xdotool stand-in,
  labeled as such) → Blender's own handler reported WHICH object was
  picked (`Light`) — **click → named-object fact in 109 ms, zero
  pixels, no vision.**
- **The gap, named:** `dpl-coach-overlay` REFUSED to paint — its
  presence gate is AT-SPI-only
  (`crates/coach-overlay/src/linux_self_test.rs::ensure_target_running`)
  and Blender is invisible to AT-SPI (custom OpenGL UI, census
  confirmed). So an R0 app can be driven and observed but not yet
  painted. The fix is small and named: an R0/X11 presence arm — the
  X11 geometry helper (`x11_window_for_class`) already exists in the
  same file. Not coded; the operator redirected to the mock pass first.

Verdict: the R0 rail works end to end on a packaged install, no Blender
source needed. Artifacts: `docs/research/coach/spikes/blender-r0/`.

## 2 · Mock rev 4 — the three systems, clearly named

Operator: "find our existing mocks and lets fix them and then have them
clearly named." The coach lane owns exactly one of the 15 mocks:
`docs/design/mocks/coach-seam-mock.html`. Rev 4 changes:

- **Naming, plain words, top of page:**
  1. **EguiPilot — the inside lane.** DevPULSE Labs + chat trigger AI
     to drive the platform itself; teach, or perform live. Aggressive
     because we own the inside (real semantic tree, typed verbs).
  2. **Fabric Foundry → the Catalog.** The progress-bar build that
     processes an installed app and generates its fabric into the
     catalog — one signed pack per app. The catalog IS the
     source-of-truth app-agnostic layer for AI control.
  3. **Coach + the HUD — the outside lane at lesson time.**
- **Rung defect fixed:** rev ≤3 labeled the ladder R1–R4; the ratified
  ADR and the MERGED S1 code speak R0 native / R1 a11y / R2 injection /
  R3 vision. The mock was contradicting shipped vocabulary. Shifted
  everywhere (legend, per-app ladders, trace table, profile TOMLs,
  downgrade strings), ascending order so nothing double-moved.
- **Stage 8 · Coach HUD (new):** persistent floating toolbar hovering
  OPPOSITE focus; **Ctrl+Shift+Space** freezes with a visible
  indicator (the hotkey genuinely works in the mock — verified in the
  browser); hover chips — steps 2/5, points, community, avg action ms,
  actions, tokens — each tooltip names its real ledger source
  (coach-domain beats · S1 seam spans · ai-usage domain #169). HUD
  laws in-mock: click-through, never overlaps the spotlight target,
  freeze is an operator right, all stats local-first.
- **Foundry stage** reframed as "building X into the catalog" and now
  cites the spike's REAL numbers for Blender instead of hypotheticals.
- **Red-pen queue → 8 items.** New: (6) HUD placement law — "opposite
  of focus" from a11y focus or pointer dwell, and the relocation
  animation budget; (7) confirm Ctrl+Shift+Space as the global hotkey;
  (8) **community score conflicts with the local-first ledger law —
  requires an explicit opt-in share ruling before anything ships.**
- Render-verified in browser (stages 1, 2, 8 + the hotkey path). Bug
  caught in-session: the mock palette has no `--panel` token, so the
  first HUD draft rendered transparent — fixed to `--raised`.

## Operator questions answered

- **"Am I wrong, is there a much better solution?"** No. The
  catalog/pack architecture is validated by the spike: a click came
  back as a NAMED OBJECT in 109 ms; vision-first alternatives guess at
  pixels, orders of magnitude slower and locale-fragile. One
  refinement: for R0 apps the fabric IS the native API surface — the
  pack records which rail, which is exactly what the catalog design
  already allows.
- **The Rust "connector" term:** a **trait** (interface) implemented by
  an **adapter** per app. Axum is merely the daemon's HTTP library.

## State

- UNCOMMITTED, riding the next docs PR: mock rev 4, spike plan +
  artifacts, this journal, SESSION/CURRENT/signals updates.
- Order from here: red-pen rev 4 → docs PR → **S2 plan approval** → S2
  → S3b (first clickable demo; hub swaps heuristics for registry
  resolve) → S4 Blender pack proper (seat now has Blender AND a proven
  bridge shape) → S5 Optimize cards. Overlay R0 presence arm rides S3b
  or a micro-slice — operator's call.
