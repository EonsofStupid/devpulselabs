# Port plan — PixVix capture suite + mainpage ambience

Produced by the /port-capture protocol, 2026-07-21. Salvage source:
`C:\localdev\devpulse labs` (read-only). Sibling: docs/plans/port-slides.md.
Inventory grounded by direct sweep this session; each execution pass re-reads
its files IN FULL before building (the port-slides discipline).

## 1. Context

Operator (2026-07-21): *"pixvix the overlay, my background animations for the
mainpage, my two loops, my pixvix overlay/app which was suppose to be viziwiz
so we can fix that or just keep it pixvix, my session replay, steps recorder,
img, video."* This is the capture side of the one-stop shop: the engine that
feeds the Library the slides now consume.

**Naming decision (veto-able):** the capture overlay/app KEEPS **PixVix** —
established identity (footer launcher, artifact index, guides on disk).
**ViziWiz** was the ink-mode viz-editor, superseded by the ADR-010 studio;
the name is retired with it and free for future use.

## 2. Inventory (the census — old repo)

**Rust engine, `src-tauri/src/commands/` (~3,487 LOC — ports near-verbatim,
Tauri wrappers deleted, window plumbing → egui viewports, emits → fabric):**
- `capture.rs` (710): screen-capture engine — monitor grab, region crop,
  freeze-frame, snip save → inbox; the overlay flow's backend.
- `steps.rs` (505): Steps recorder — auto + hotkey capture, click points,
  WebP frames, `guide.json` artifact, `save_guide` filing.
- `replay_capture.rs` (607): session VIDEO capture (ffmpeg subprocess,
  ADR-009) — start/stop, region/window, normalize.
- `replay.rs` (760): inbox filing + preview jail + listing.
- `palette.rs` (147): the PixVix palette window (summon/dismiss).
- `artifacts.rs` (440) + db index: the rebuildable artifact index (CL1).
- `guide_edit.rs` (131): guide rename/reorder through the inbox law.
- `events.rs` (20) + `tools.rs` (142): emit seam + ffmpeg/tool discovery.
- Global shortcut in lib.rs: summon/dismiss the overlay from anywhere.

**Frontend to REAUTHOR in egui:** `overlay.pixvix.tsx` (25 — thin route) +
the snip overlay stack (4 tabs Photo/Video/Session/Steps, pro region select,
ColdLight magnifier loupe on the Rust freeze-frame, REC pill), `pixvix-home`
(recents + thumbnails), `pixvix-palette`, `capture-form`/`meta-fields`
(forced-metadata filing), footer launcher icon.

**Mainpage ambience:** no loop video assets exist in old content (verified —
find/grep empty). Reading taken: "my background animations / my two loops" =
TWO looping background ambiences for the mainpage. Disposition: author them
as pre-made vector ambiences (`Media::Anim`) on the motion runtime — pool-
colored, gallery-gated, zero assets. **OQ-PV1 (ask-first):** if the operator
means two specific video files, they get copied (never moved) into content
and rendered via the engine instead.

## 3. Dispositions

| Item | Disposition | Phase |
|---|---|---|
| capture.rs engine | PORT-VERBATIM (minus Tauri) | PV1 |
| steps.rs recorder | PORT-VERBATIM (minus Tauri) | PV1 |
| replay_capture.rs (session video) | PORT-VERBATIM (ffmpeg subprocess law) | PV1 |
| replay.rs inbox filing/jail | PORT-VERBATIM | PV1 |
| artifacts.rs + db index | PORT-VERBATIM | PV1 |
| guide_edit.rs | PORT-VERBATIM | PV1 |
| events emit seam | REAUTHOR → fabric typed events (never synthesized) | PV1 |
| tools.rs (ffmpeg discovery) | PORT-VERBATIM (fetch:ffmpeg carried) | PV1 |
| rrweb DOM replay remnants | RETIRE (ratified: video-first) | — |
| snip overlay + tabs + loupe | REAUTHOR — egui transparent viewport | PV2 |
| palette window | REAUTHOR — egui viewport | PV2 |
| global shortcut + footer launcher | REAUTHOR — shell footer + hotkey | PV2 |
| pixvix-home recents | REAUTHOR — wiredFRONT hub surface | PV3 |
| capture-form forced metadata | REAUTHOR | PV3 |
| session replay viewing | REAUTHOR — video player on engine frames | PV3 |
| img/video artifacts → Library | REAUTHOR — one library, one index | PV3 |
| ViziWiz name | RETIRE (studio superseded the ink editor) | — |
| mainpage background loops ×2 | REAUTHOR — `Ambient` vocabulary + Home backdrop | PV0 |

## 4. Phases

- **PV0 — mainpage ambience (small, immediate):** two new named `Ambient`
  loops (e.g. `Drift` — slow diagonal tone washes; `Pulse` — the breathing
  rings constellation) + the Home hub renders them as its backdrop with a
  switcher; gallery-gated via the media block.
- **PV1 — engine crates:** `crates/engine/{capture,steps,session,inbox,index}`
  — the 3.5k LOC Rust port, fabric-evented, content laws intact, exercised
  headlessly (unit tests + a capture smoke on the real exe).
- **PV2 — the PixVix overlay in egui:** transparent always-on-top viewport,
  4 tabs, region select + magnifier on freeze-frame, REC pill, palette,
  footer launcher + global hotkey.
- **PV3 — wiredFRONT surfaces:** pixvix home recents, forced-metadata filing,
  video-first session replay player, img/video into the one Library.

## 5. Boundaries

- 🚫 ffmpeg stays a subprocess (ADR-009). 🚫 Content never mutated; copy
  never move; inbox never-delete law carried. 🚫 No rrweb.
- ⚠️ OQ-PV1 above. ⚠️ Multi-viewport transparency on wgpu must be spiked
  early in PV2 (known egui platform-variance risk) — if it fails, the
  overlay falls back to a borderless fullscreen viewport.

## 6. Verification

Per pass: cargo tests + real-exe proof (PV0: Home backdrop screenshots both
loops; PV1: engine smoke writes a real snip/guide into a temp root and the
index lists it; PV2: overlay screenshot + a captured snip landing in inbox;
PV3: library shows the artifact with thumbnail). Baselines per surface.

## 7. Commit plan

One commit per phase: `feat(ambient)`, `feat(engine)`, `feat(pixvix)`,
`feat(wiredfront)` — each with plan-appended execution spec first.

## Correction (operator, 2026-07-21)

*"Viziwiz was my pixvix and the ink editor is just part of the slide
presenter."* Two fixes to §1/§3:

1. **ViziWiz was the intended name for the capture overlay/app itself** —
   not the ink editor. The name is NOT retired; it is the app's original
   identity. Default remains PixVix (per "or just keep it pixvix" — the
   artifacts/launcher already carry it), with a clean rename to ViziWiz
   available as a single-pass change whenever the operator calls "fix it":
   overlay title, hub labels, palette — ids/paths untouched.
2. **The ink layer is presenter equipment, not an app.** The stream-time
   annotation/ink layer (old `annotation-layer.tsx`, kept in the old repo)
   is CARRIED as part of the slide PRESENTER: re-dispositioned
   REAUTHOR → the deck/presenter pass in port-slides (draw-over-slide,
   pool-colored ink, presenter-paced). Nothing about it was superseded.
