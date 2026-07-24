# Port plan — ViziWiz: the capture suite + the AI-guide step recorder

Status: **DRAFT — awaiting operator ratification.** Produced by the
`/port-capture` protocol, 2026-07-24. Salvage source: `C:\localdev\devpulse labs`
(READ-ONLY), inventoried in full this session by the capture audit. Target:
`C:\localdev\devpulselabs`. Sibling/prior: `docs/plans/port-pixvix.md` (the
engine port) — this plan supersedes nothing there; it adds the ViziWiz rename,
the claymorphism migration, and the net-new AI-guide + export program.

Branch: `design/viziwiz-capture` (off `design/coldlight-traits`, so the trait
system is available for the claymorphism migration).

## 1. Context — the operator's requirement (2026-07-24)

Verbatim intent: *"methodically capture all of the work, the key addition will
be for step recorder where we can have AI leveraging eguiPilot and have it
generate guides for me to then be able to export as single slides to import, I
will also be able to export it as its own little contained double click and play
all styling and guide … but we will first ensure we get it truly built friendly
with claymorphism and properly ported to egui."*

This converges four roadmap lines into one program: the **ViziWiz rename** (the
capture overlay's intended identity, per port-pixvix.md §correction), Program
**DEMO** (AI drives EguiPilot to record a demonstration), **T4** (a guide becomes
one slide), and **G1** (a self-contained, double-click guide player). The
operator's ORDER is explicit: claymorphism/egui fidelity FIRST, then the new
features.

## 2. Current state — both repos, ground-truthed this session

### 2a. Original (the salvage surface — full audit on file)

The subsystem is real and largely shipped. Engine: `src-tauri/capture/src/lib.rs`
(1090) + `bin/capture_cli.rs`. Command layer: `src-tauri/src/commands/*.rs` (43
IPC commands registered; capture-relevant: capture.rs 12, steps.rs 6,
replay_capture.rs 4, replay.rs 6, guide_edit.rs 1, palette.rs 4, artifacts.rs 4,
inbox_read.rs 2). Front-end: `src/domains/wiredfront/**` (2113), `native/**`
(1224), `studio/**`, `annotation/**`, `session-replay/**`. 3 OS-global hotkeys
(Ctrl+Shift+S palette · Ctrl+Shift+X mark · Ctrl+Shift+9 steps).

The eight capability areas (each enumerated in the audit):
1. **Snip/stills** — 4 modes (Region/Window/Full/Freeform); 7 engine fns;
   frozen-frame WYSIWYG + 6× loupe; **arrow-key nudge (Shift×10)**; delay
   `0/3/5/10`; re-snip; **auto-clipboard default-ON**; fractional/DPI-proof.
2. **Video** — fixed-N + Start▸Stop; matrix mp4/vp9/av1 × none/system/mic +
   device enum; 720p default; caps; poster frames; DXGI recorder-reuse cache.
3. **Steps** — rdev global hook; do→done two-frame model + **aHash fingerprint
   collapse** (distance 0 = drop); window-following (skips own PID); click→
   normalized ring; WebP@82; seed-name from title; forced-metadata Save card;
   **non-activating REC pill** (`WS_EX_NOACTIVATE|WS_EX_TOOLWINDOW`).
4. **Guides** — guide = a FOLDER (`guide.json` + `step-NNN[-done].webp`); 11-field
   StepRec; GuideReviewer (do→done cards, click ring, reorder/delete/caption,
   frame→slide); `update_guide` (inbox-jail, renumber, prune); guide refused by
   inbox-keep (becomes slides, not content).
5. **ViziWiz/ink** — viz-editor (6 tones, width 2–24, undo/clear, flatten to
   inbox, source never mutated); annotation-layer (ephemeral stream ink,
   pressure width).
6. **Session replay** — **rrweb** DOM replay, 3 pipelines, one manifest; Replayer
   with VLC-like transport.
7. **Guide EXPORT** — **NOT BUILT**; specified only (steps-capture-and-filing-plan
   §S5): "the identical Guide rendered to self-contained, offline, ColdLight-styled,
   next→next→next HTML, one file, zero external requests."
8. **AI / EguiPilot UI-driving** — **ABSENT**. No in-app subsystem drives the UI
   to record a demonstration. (Only an agent-chat console + downstream AI labels.)

10 salvage-risk flags (easy to silently drop): arrow-nudge, frozen-frame loupe,
auto-clipboard, non-activating pill, DXGI reuse cache, fingerprint collapse,
guide-is-a-folder, rrweb one-manifest, unbuilt HTML export, video matrix.

### 2b. Target (the egui port — what actually landed)

- **`crates/pixvix`** (`lib.rs` 1120 read in full): the overlay (PV2), rebuilt on
  the ported engines. Has Img (Full/Window/Region/Freeform via
  `editor::{RegionSelect,SnipMode}`), delay (**0/3/10 only — the 5s value was
  dropped**), re-snip, auto-clipboard (`copy_to_clipboard`), the Video matrix
  (`FORMATS`/`AUDIO`/`CAPS` = mp4/vp9/av1 × none/system/mic × caps), Session (S1),
  Steps (C2: `StepsRecorder`, Save card `PendingGuide`, hotkeys, seed-name). The
  palette is an always-on-top immediate viewport that **collapses to a REC pill**
  while recording. Tone = `Magenta` (the capture family).
- **`crates/engine-steps`** (`lib.rs` 741 + `edit.rs` 241): `StepsRecorder`
  (start/mark/stop/save/discard/status), `CaptureMeta`, `StepRec`, `StopResult`,
  `install_hook`; `edit.rs::update_guide` (the W5 reviewer's engine) + `GuideStepEdit`.
  The guide.json producer is here.
- **`crates/egui-pilot-ai-runner`** (1095): `run(Flow) -> RunResult`,
  `capture(name) -> CaptureResult`, `request/send/receive`, `next_event`,
  `cancel/stop`, `load_flow/save_flow`. **It can drive the app with real input
  and capture frames** — the substrate for Program DEMO.
- **Prior plan** `port-pixvix.md`: the engine was PORT-VERBATIM (Tauri stripped,
  emits→fabric); ViziWiz is ratified as the app's intended name (rename = one
  clean pass, ids/paths untouched); rrweb RETIRE ratified (video-first S1).

### 2c. Gaps confirmed by direct reads

- **The palette is NOT on the trait system.** `crates/pixvix/src/palette.rs` uses
  hand-rolled `pilot_chip` (18+ sites) and `lib.rs` paints the viewport with
  `egui::Frame::new().fill(surface::RAISED).stroke(HAIRLINE)` — a clay-ish frame,
  not `cl::clay()`/`cl::treated(...)`. Claymorphism migration is real work.
- **T4 guide-as-slide** (`Block::Guide`) — **not built** (grep of `crates/doc`,
  `crates/slides`: no Guide block/kind).
- **G1 self-contained export** — **not built** (no export/standalone/player code).
- **The AI-guide step recorder** — **not built** (Area 8 absent in both repos).
- Deviations to reconcile: delay dropped 5s; verify arrow-nudge + loupe survived
  into `editor.rs`; verify the egui REC-pill viewport is non-activating (else the
  focus-follow capture breaks — salvage-risk #4).

## 3. Target design — four phases, the operator's order

### Phase V1 — ViziWiz rename + claymorphism migration (the "built friendly" gate)

- **Rename** `pixvix` → `viziwiz` as the app/overlay identity: crate name, overlay
  title, hub labels, palette, footer launcher, status strings, harness flags
  (`--pixvix*` → `--viziwiz*` with the old accepted one release? NO — pre-release,
  clean, no shim per repo law). Artifact ids/paths/`data/inbox` names UNTOUCHED
  (they are content identity). This is a mechanical, reviewable pass.
- **Trait migration**: the palette viewport frame → `cl::clay()` (passive plane);
  every `pilot_chip` → a tip-mandatory chip built on `cl::treated(TONE,
  "neubrutalist", …)` (actionable) with the layman tips it already carries; the
  Save card → claymorphic well; the REC pill → a neubrutalist pill. Gate:
  `coldlight-system` skill. Proof: palette + Save-card + REC-pill shots.
- **Reconcile deviations**: restore delay `0/3/5/10`; verify/port arrow-key nudge
  (+Shift×10) and the 6× loupe in `editor.rs`; verify the REC-pill viewport is
  non-activating (or port the Win32 `WS_EX_NOACTIVATE|WS_EX_TOOLWINDOW` seam) —
  salvage-risk #4.

### Phase V2 — the AI-guide step recorder (Program DEMO, net-new)

The composition: **an EguiPilot `Flow` drives the app with real input while a
capture adapter records each action into `guide.json`.** Because the pilot knows
the *semantic target* (a `PilotId` + its rect + label), the guide it produces is
BETTER than OS-click guides — exact element, exact caption, deterministic.

- **`GuidePilotSink`** (new, in `engine-steps` or a `viziwiz` module): a capture
  adapter the runner calls per driven action → writes `step-NNN.webp` (do: pre-
  action frame with the target rect for the click ring), settles, writes
  `-done.webp` (post-action frame), assembles the SAME `guide.json` schema
  (`StepRec` — reuse verbatim). Fingerprint-collapse carries (aHash distance 0 =
  drop). The pilot's `capture()` provides frames; the target rect comes from the
  resolved `PilotId`.
- **The AI seam**: a natural-language ask ("record the steps for enabling X")
  becomes a pilot `Flow`, authored by an agent through the chat platform's typed
  tools. Then `runner.run(flow)` with the `GuidePilotSink` attached → a guide
  files through the normal Save path (forced metadata, index).
- **Dependencies (honest, per roadmap DEMO)**: F3 typed chat tools (the agent's
  hands) + the Codex seat's pilot multi-window/gesture extensions. V2 lands the
  `GuidePilotSink` + a headless drill NOW (pilot drives a fixture flow → a real
  guide.json on disk); the chat-authored path lands when F3 is ready.

### Phase V3 — guide → single slide (T4 `Block::Guide`)

- A new `Block::Guide` kind in `crates/doc` referencing a captured guide through
  the folder contract (frames under the slide's reserved `assets/`); its internal
  reveal steps ARE the recorded steps, ridden by the D2 beat grammar
  (advance/retreat walks do→done with the click-highlight ring). One guide = one
  slide. Rendered through the one renderer (`crates/slides`), claymorphic frame,
  editable via the W5 reviewer (reorder/delete/caption over `edit::update_guide`).
  Import into a show = the X3 associate seam (copy-never-move). Depends: C2 (done),
  D2 (done), X1 (done).

### Phase V4 — guide → self-contained double-click player (G1)

- **One download**, built fresh from the original's §S5 spec: a folder (or single
  HTML) bundling the guide's WebP frames + `guide.json` + a dependency-free player
  — VLC-like transport (play/pause/step±/scrub, keyboard + touch), progress rail
  with step ticks, do→done toggle, click-highlight ring, captions. **Styled FROM
  ColdLight** — the mood/ink/font/**trait** vocabulary emitted to CSS from the
  same token sources (a build-time emitter, same pattern as the registries; the
  claymorphism/neubrutalism treatments render in the browser too). Browser-
  optimized: WebP, lazy-load + preload-next, zero network — works from `file://`.
  Double-click-and-play. **OQ-VZ2 (ask-first): "libmvc"** — the operator named a
  library/pattern for the player; unidentified. Default = a zero-dependency
  ColdLight-emitted vanilla player; confirm if "libmvc" is a specific target.

### Crate/target layout

`crates/viziwiz` (renamed from `pixvix`) owns the overlay + `GuidePilotSink`
wiring; `engine-steps` owns the guide schema + sink; `crates/doc` gains
`Block::Guide`; a new `crates/guide-export` (or `viziwiz::export`) owns the G1
emitter + the ColdLight→CSS build step.

## 4. Changes, in execution order (AFTER ratification — each its own pass)

1. **V1a rename** — `pixvix`→`viziwiz` crate + all labels/flags (mechanical, green).
2. **V1b trait migration** — palette/editor/Save-card/REC-pill onto
   `cl::treated`/`clay`; restore delay 5s; verify nudge/loupe/non-activating pill.
   Proof shots (`coldlight-system` + `true-verify`).
3. **V2 `GuidePilotSink`** — the pilot→guide adapter + a headless drill; the
   chat-authored path stubbed behind the F3 dependency.
4. **V3 `Block::Guide`** — doc block + renderer + reviewer wiring + import seam.
5. **V4 guide-export** — the ColdLight→CSS emitter + the vanilla player + a
   headless-browser drill asserting the bundle is CLOSED (no external refs).

## 5. Boundaries

- 🚫 **ffmpeg stays a subprocess** (ADR-009). 🚫 **Content never mutated**; copy-
  never-move; inbox never-delete; sources never touched. 🚫 **rrweb** — RETIRED
  (ratified, video-first). 🚫 No `#[allow]`, no re-export shims (pre-release,
  clean rename). 🚫 No blur/feathered borders (egui-build law).
- ⚠️ **Claymorphism-first gate**: V1 lands and is proof-shot BEFORE V2–V4.
- ⚠️ **V2 depends on F3 + the Codex pilot extensions** — the `GuidePilotSink` +
  drill land now; the chat-authored path waits on F3, stated not smuggled.
- ⚠️ Ask-firsts: **OQ-VZ1** delay 5s restoration (assumed yes — it was the
  original); **OQ-VZ2** "libmvc" for the G1 player (default = zero-dep vanilla);
  **OQ-VZ3** the ViziWiz rename scope (this plan assumes full rename incl. crate).

## 6. Verification

- **V1**: `cargo green`; palette/Save-card/REC-pill screenshots at maximized AND
  `--window 1100x700` (feel-bar vs the original's PixVix); a capture-smoke drill
  (a real snip lands in inbox; the REC pill doesn't steal focus).
- **V2**: a headless pilot drill runs a fixture Flow → asserts a real `guide.json`
  + WebP do/done frames on disk, click rings at the driven rects; fingerprint
  collapse drops a no-op step.
- **V3**: a fixture guide steps end-to-end through the deck scenario (D2 beats);
  the guide renders as one slide; `--screenshot` proof.
- **V4**: export a fixture guide → engine-unit asserts the bundle is CLOSED (no
  external refs) + a headless-browser drill steps it end-to-end from `file://`.

## 7. Commit plan

One PR per §4 pass: `refactor(viziwiz): rename`, `feat(viziwiz): claymorphism`,
`feat(viziwiz): ai-guide sink`, `feat(doc): guide slide`, `feat(guide-export):
self-contained player` — each green + proof-shot, operator exe rebuilt, STATE.md
updated. V1 must land before V2–V4.

## 8. Disposition table — the no-silent-drop gate

Every original-app capability, accounted for. **PORTED✓** = already in the egui
port (verify in the pass). **REAUTHOR** = rebuild in egui idiom. **NEW** = net-new
(absent in original). **RETIRE** = dropped with ratified reason.

| # | Original capability | Disposition | Phase |
|---|---|---|---|
| 1 | Snip modes Region/Window/Full/Freeform | PORTED✓ (`editor.rs`) | V1 verify |
| 2 | 7 still engine fns (monitor/window/region/freeform/from-png/freeze) | PORTED✓ (`engine-capture`) | — |
| 3 | Frozen-frame WYSIWYG + 6× loupe magnifier | PORTED✓ (verify in `editor.rs`) | V1 verify |
| 4 | Arrow-key nudge (+Shift×10) on committed region | **VERIFY / REAUTHOR** if absent | V1b |
| 5 | Capture delay 0/3/5/10 | REAUTHOR — restore 5s (port dropped it) | V1b |
| 6 | Re-snip (repeat last shape) | PORTED✓ (`LastSnip`) | — |
| 7 | Auto-clipboard default-ON | PORTED✓ (`copy_to_clipboard`) | — |
| 8 | Window picker + refresh | PORTED✓ | — |
| 9 | Video fixed-N + Start▸Stop | PORTED✓ (`VideoRec`) | — |
| 10 | Format matrix mp4/vp9/av1 | PORTED✓ (`FORMATS`) | — |
| 11 | Audio none/system/mic + device enum | PORTED✓ (`AUDIO`, `list_audio_devices`) | — |
| 12 | 720p default + caps + REC timer + poster | PORTED✓ (`CAPS`, `poster_for`) | — |
| 13 | DXGI recorder-reuse cache | PORTED✓ (verify in `engine-capture`) | V1 verify |
| 14 | Steps rdev global hook + Ctrl+Shift+S/9/X | PORTED✓ (`install_hook`, `Hotkey`) | — |
| 15 | do→done two-frame + aHash fingerprint collapse | PORTED✓ (`engine-steps`) | — |
| 16 | Window-following (skip own PID) + click ring | PORTED✓ (`StepRec`) | — |
| 17 | WebP@82 step frames, seed-name from title | PORTED✓ (`seed_name`) | — |
| 18 | Forced-metadata Save card | PORTED✓ (`PendingGuide`, C2) | — |
| 19 | Non-activating REC pill (WS_EX_NOACTIVATE) | **VERIFY / REAUTHOR** (focus-steal risk) | V1b |
| 20 | guide.json schema + 11-field StepRec (folder artifact) | PORTED✓ (`engine-steps`) | — |
| 21 | GuideReviewer UI (do→done cards, ring, reorder/delete/caption) | REAUTHOR — W5 reviewer (engine `edit.rs` exists) | V3 |
| 22 | `update_guide` (jail, renumber, prune) | PORTED✓ (`edit.rs`) | — |
| 23 | Frame→slide seam | REAUTHOR — X3 associate + T4 | V3 |
| 24 | Guide refused by inbox-keep (becomes slides) | PORTED✓ (content-tree law) | — |
| 25 | ViziWiz viz-editor ink (6 tones, width, undo, flatten) | PORTED✓ (C4 `engine-media::ink`) | — |
| 26 | Annotation-layer stream ink (ephemeral, pressure) | PORTED✓ (D3 presenter ink) | — |
| 27 | rrweb session replay (3 pipelines, Replayer) | **RETIRE** (ratified: video-first S1/S2) | — |
| 28 | Session capture + inbox/keep/dump/rekey | PORTED✓ (S1 `session.rs`, `dpl-index`) | — |
| 29 | **Guide standalone HTML export (§S5)** | **NEW** — build fresh (ColdLight-emitted) | V4 |
| 30 | **AI / EguiPilot UI-driving demonstration** | **NEW** — `GuidePilotSink` (absent in original) | V2 |
| 31 | Guide-as-one-slide (`Block::Guide`) | **NEW** — T4 (was S5-planned, unbuilt) | V3 |
| 32 | The ViziWiz name (app identity) | REAUTHOR — rename `pixvix`→`viziwiz` | V1a |
| 33 | Palette/editor styling | REAUTHOR — trait system (claymorphism) | V1b |

Row count = 33 dispositions covering every audited capability. RETIRE #27 is
operator-ratified (port-pixvix.md); all others are PORT/REAUTHOR/NEW.
