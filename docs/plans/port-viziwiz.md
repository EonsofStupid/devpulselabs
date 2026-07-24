# Port plan — ViziWiz capture → Workbench authoring → Native Guide Player

Status: **DRAFT — awaiting operator ratification.** Rewritten 2026-07-24 to the
operator's authoritative three-product architecture
(`docs/design/viziwiz-architecture-spec.md`). Supersedes the earlier single-
program DRAFT (which pre-dated the spec pivot and modelled the exported guide as
a self-contained *HTML* player — now demoted to the media-only fallback; the
canonical player is **native Rust**).

Salvage source: `C:\localdev\devpulse labs` (READ-ONLY), inventoried in full by
the `/port-capture` audit. Target: `C:\localdev\devpulselabs`. Branch:
`design/viziwiz-capture`.

## 0. What changed from the previous draft (read this first)

The spec ratified a **three-product** split, all pure Rust/egui (no Svelte, no
web stack):

1. **WiredFRONT Editor** — media production. The **ViziWiz capture overlay is
   WiredFRONT's capture arm** (snip · video · session · steps). egui + ColdLight.
2. **Workbench Editor** — slide/guide authoring over finished assets: scene graph,
   holographic overlays, triggers, cue engine, presenter/audience modes, the
   export compiler. egui + ColdLight.
3. **Native Guide Player** — the self-contained playback runtime:
   **winit + wgpu + libmpv** (egui only for chrome). Opens `.guide` packages and
   the portable `.exe`. No Tauri, no WebView2, no browser, fully offline.

The single most important correction to the old draft: **the exported double-
click deliverable is a native `.exe` (stable player + embedded `.guide` +
footer), NOT an HTML bundle.** The old "ColdLight→CSS vanilla HTML player" is now
**Export Option C** only — a media-only `.mp4`/HTML fallback for platforms that
reject executables, with no branching/steps/nav. The canonical guide is the
`.guide` package played by the native player.

Consequently the phase map is re-keyed from "ViziWiz V1–V4" to the products:

| Old draft phase | New home |
|---|---|
| V1 rename + claymorphism | **W-CAP** (WiredFRONT capture overlay: done rename + claymorphism) |
| V2 AI-guide recorder | **WB-DEMO** (feeds Workbench authoring) |
| V3 guide→slide `Block::Guide` | **WB-SLIDE** (Workbench) |
| V4 self-contained HTML player | **RESHAPED → GF/GP/GX** (`.guide` format + native player + export compiler) |

## 1. Context — the operator's requirement

Verbatim intent (2026-07-24): *"methodically capture all of the work, the key
addition will be for step recorder where we can have AI leveraging eguiPilot and
have it generate guides for me to then be able to export as single slides to
import, I will also be able to export it as its own little contained double click
and play … but we will first ensure we get it truly built friendly with
claymorphism and properly ported to egui."*

Two hard constraints ride every phase below:
- **Order: claymorphism/egui fidelity FIRST**, then the net-new features (W-CAP
  before WB-DEMO / WB-SLIDE / GF-GP-GX). The operator stated this explicitly.
- **Architecture is the spec**, implemented in **pure Rust** — no Svelte, no web
  stack; the player is native (winit + wgpu + libmpv). "libmvc" = **libmpv**.

## 2. Current state — ground-truthed this session (real reads, not memory)

### 2a. Original (the salvage surface — full audit on file)

Engine `src-tauri/capture/src/lib.rs` (1090) + `bin/capture_cli.rs`; 43 IPC
commands (capture 12, steps 6, replay_capture 4, replay 6, guide_edit 1, palette
4, artifacts 4, inbox_read 2); front-end `src/domains/wiredfront/**` (2113),
`native/**` (1224), `studio/**`, `annotation/**`, `session-replay/**`; 3 OS-
global hotkeys. The eight capability areas and 33 enumerated capabilities are in
§8's disposition table. **Guide EXPORT was specified but NEVER BUILT** in the
original (steps-capture-and-filing-plan §S5); **AI/EguiPilot UI-driving was
ABSENT**. Those two are net-new here.

### 2b. Target (the egui port — what actually landed, verified this session)

- **`crates/viziwiz`** (renamed from `pixvix`, this session): `lib.rs` (~1120),
  `palette.rs` (726), `editor.rs` (427), `session.rs`. The overlay is an always-
  on-top immediate viewport that collapses to a REC pill while Steps records.
  Tone = `Magenta` (the capture family). Verified present: 4 snip modes; the
  frozen-frame editor with **arrow-key nudge (+Shift×10)** (`editor.rs:144`) and
  a **magnifier loupe** (`editor.rs:312` — currently **4×**, the original was
  **6×**); delay **`0/3/5/10`** (`lib.rs:950`, restored this session); re-snip;
  auto-clipboard; the video matrix (mp4/vp9/av1 × none/system/mic × caps);
  Session (S1); Steps (C2 `StepsRecorder` + forced-metadata Save card).
- **`crates/engine-steps`** (`lib.rs` 741 + `edit.rs` 241): `StepsRecorder`,
  `StepRec`, `install_hook`; `edit.rs::update_guide` (the W5 reviewer's engine).
  The `guide.json` producer.
- **`crates/egui-pilot-ai-runner`** (1095): `run(Flow)`, `capture(name)` — it can
  drive the app with real input and capture frames. The substrate for WB-DEMO.
- **`crates/coldlight`**: the composable **trait system** is live —
  `traits.json` (neubrutalist saturated fill tone@400 + tone@700 edge + `[3,3]`
  hard shadow + tone@300 back-glow + WELL ink, with authored hover/active;
  claymorphic RAISED plane, radius 16, directional double-shadow) → generated
  `traits_gen.rs` → `compose(tone, base, stack, state) -> ResolvedStyle` →
  `cl::treated`/`brutal`/`clay` paint API. Migrating a surface onto it is the
  W-CAP claymorphism job.

### 2c. Gaps confirmed by direct reads

- **The palette is NOT on the trait system.** `palette.rs::pilot_chip` hand-rolls
  a brutal chip with magic numbers (`surface::OVERLAY` fill, `black_alpha(230)`
  shadow, `tone.wash()` 2px edge, manual hover lift); `lib.rs:894` paints the
  viewport with a bespoke `egui::Frame::new().fill(RAISED).stroke(HAIRLINE)`
  instead of `cl::clay()`. This is the W-CAP claymorphism target.
- **Loupe is 4×, original was 6×** — a fidelity deviation (`editor.rs:314`).
- **REC-pill non-activating** — the palette viewport builder (`lib.rs:846`) sets
  `always_on_top`/`decorations(false)`/`resizable(false)` but has **no**
  `WS_EX_NOACTIVATE` equivalent (egui doesn't expose one). The original used
  `WS_EX_NOACTIVATE|WS_EX_TOOLWINDOW` so the pill never stole focus during
  capture. Needs a raw-window-handle Win32 seam (or empirical verification that
  the global rdev hook makes focus-steal harmless).
- **The `.guide` ZIP format** — not built.
- **The Native Guide Player** (winit/wgpu/libmpv) — not built.
- **The export compiler** (freeze/normalize/compile/package/footer/offline-verify)
  — not built.
- **T-slide `Block::Guide`** — not built.
- **The AI-guide step recorder** — not built (absent in original too).

## 3. Target design — phases in the operator's order

### Phase W-CAP — the ViziWiz capture overlay: claymorphism (the "built friendly" gate)

WiredFRONT's capture arm, made faithful to the ColdLight trait system. **Rename
is DONE** this session (crate `pixvix`→`viziwiz`, flags `--viziwiz*`, struct
`Viziwiz`, pilot keys `devpulse.viziwiz.*`, no shims; artifact ids/paths
untouched). Remaining W-CAP work:

- **Palette plate → `cl::clay()`.** Replace the bespoke viewport `egui::Frame`
  (`lib.rs:894`) with a `cl::clay()` passive plane wrapping the palette/REC-pill
  content (tighten its margin so the dense toolbar stays compact). The CentralPanel
  frame drops to the well fill behind the plate.
- **`pilot_chip` → the trait system.** Route every chip's paint through
  `cl::compose(TONE, "neubrutalist", &[], state)`, keeping the compact
  `allocate_exact_size` sizing (galley + 24px, 30px height) so the toolbar layout
  is preserved. State drives the authored overlays: hover → `Hover`, pressed →
  `Active` (the authored `[1,1]` press). Paint fill/edge/primary-shadow/ink from
  the `ResolvedStyle`; **omit the back-glow for these compact chips** (its spread
  bleeds into neighbours in dense rows — the glow is a hero-surface move). Zero
  magic numbers survive. This is a visible change from the current dark-brutal
  chips to **saturated Magenta neubrutalist** chips (dark WELL ink) — on-brand for
  the capture family; flagged for the operator's eyes at proof time.
- **Save card → claymorphic well; REC pill → neubrutalist pill** (same routing).
- **Fidelity fixes:** restore the loupe to **6×** (`editor.rs:314`); delay
  `0/3/5/10` already restored.
- **Non-activating pill:** verify empirically whether the pill steals focus during
  Steps capture; if it does, add a Win32 `WS_EX_NOACTIVATE|WS_EX_TOOLWINDOW` seam
  via `raw-window-handle` (Windows-only, `#[cfg(windows)]`). Salvage-risk #4.
- Gate: `coldlight-system` skill. Proof: palette + Save-card + REC-pill + editor
  shots at maximized AND `--window 1100x700` (feel-bar vs the original PixVix).

### Phase WB-DEMO — the AI-guide step recorder (feeds Workbench authoring, net-new)

An EguiPilot `Flow` drives the app with real input while a capture adapter records
each action into `guide.json`. Because the pilot knows the *semantic target*
(a `PilotId` + rect + label), the guide is BETTER than OS-click guides: exact
element, exact caption, deterministic.

- **`GuidePilotSink`** (new, in `engine-steps` or a `viziwiz` module): a capture
  adapter the runner calls per driven action → writes `step-NNN.webp` (do: pre-
  action frame with the target rect for the click ring), settles, writes
  `-done.webp` (post-action frame), assembles the SAME `guide.json` schema
  (`StepRec` — reuse verbatim). aHash fingerprint-collapse carries (distance 0 =
  drop). The pilot's `capture()` provides frames; the rect comes from the resolved
  `PilotId`.
- **The AI seam:** a natural-language ask ("record the steps for enabling X")
  becomes a pilot `Flow`, authored by an agent through the chat platform's typed
  tools; then `runner.run(flow)` with the `GuidePilotSink` attached → a guide
  files through the normal Save path (forced metadata, index).
- **Dependency (stated, not smuggled):** the chat-authored path needs the chat
  platform's typed tools + the pilot's multi-window/gesture extensions. WB-DEMO
  lands the `GuidePilotSink` + a **headless drill NOW** (pilot drives a fixture
  flow → a real `guide.json` + WebP do/done frames on disk); the chat-authored
  path lands when the typed tools are ready.

### Phase WB-SLIDE — guide → single slide (Workbench `Block::Guide`)

- A new `Block::Guide` kind in `crates/doc` referencing a captured guide through
  the folder contract (frames under the slide's reserved `assets/`); its internal
  reveal steps ARE the recorded steps, ridden by the deck beat grammar (advance/
  retreat walks do→done with the click-highlight ring). One guide = one slide,
  rendered through the one renderer (`crates/slides`), claymorphic frame, editable
  via the W5 reviewer (reorder/delete/caption over `edit::update_guide`). Import
  into a show = the associate seam (copy-never-move). Depends: C2/D2/X1 (done).
- This is the Workbench "insert a guide scene into a deck" capability from spec §20.

### Phase GF — the `.guide` package format (the third product's data spine)

A new crate `crates/guide-format` (name TBD) owning the canonical editable
package, per spec §7/§15:

- **`.guide` = a ZIP-compatible container** (inspectable, recoverable) with
  **stored/selective** compression — never recompress already-compressed media
  (MP4/WebM/JPEG stored, not deflated). Structure: `guide.json` (manifest),
  `runtime.bin` (compiled scene graph), `assets/`, `themes/`, `shaders/`,
  `authoring/`, `integrity.json`.
- **Rust types = the serde model** ported from spec §7: `Guide`, `GuideSlide`,
  `GuideLayer`, `GuideStep` (the trigger enum: tap/click-anywhere, tap/click-
  target, delay, media-ended, keyboard, presenter, manual), `GuideAction` (the
  full action vocabulary: show/hide/animate layer, animate arrow, pulse target,
  zoom/pan, play/pause/seek video, narration, fade audio, reveal caption, darken,
  highlight, before/after, advance/branch, wait, emit OBS, presenter cue),
  `GuideNavigation`, `PlaybackPolicy`. **Normalized coordinates mandatory** in the
  compiled runtime graph (`{x,y,width,height}` ∈ 0..1 relative to the bound
  media layer) — spec §10.
- Read + write + integrity (SHA-256 per asset + package hash). Unit tests: round-
  trip a fixture guide; reject a tampered checksum; assert media is stored not
  deflated.

### Phase GP — the Native Guide Player (winit + wgpu + libmpv)

A new binary crate `crates/guide-player`, per spec §13/§21:

- **winit** — window (fullscreen/windowed) + input (touch/mouse/keyboard/presenter).
- **wgpu** — renders composition + effects + **holographic arrows (runtime SDF/
  vector geometry, spec §9)** + overlays + highlights/masks + transitions.
- **libmpv** — the embedded audio/video decode/playback subsystem ONLY (not the
  format, not the renderer, not navigation).
- **The Rust cue engine** — owns timing/navigation/branching/playback policy;
  consumes `GuideStep` triggers → `GuideAction`s. Shared with Workbench preview so
  authoring and playback ride one engine (spec §20: one graph, two policies —
  guide mode vs presentation mode).
- **egui** — chrome only (controls, volume, TOC, settings, progress, debug).
- Opens a `.guide` package AND the portable `.exe` (§16/§17). Fully offline.
- **Holographic arrow component** (spec §9): one reusable wgpu SDF arrow — start/
  end, curve, thickness, glow width/intensity, pulse, direction, color, delay,
  target binding, entrance/exit — reused across resolutions/themes/layouts.

### Phase GX — the export compiler + portable `.exe` (Workbench owns it)

Per spec §16/§18/§19 — the strict preflight pipeline, in `crates/guide-export`
(or `workbench::export`):

1. **Validate** — every asset exists; every WiredFRONT revision resolves; no
   remote dependency; every trigger→valid target; every branch→valid slide; every
   codec supported by the bundled player; works with networking disabled.
2. **Freeze** — resolve links to exact revisions, copy into the package, record
   provenance + checksums.
3. **Normalize media** — approved runtime profiles (img PNG/JPEG/WebP; video
   MP4/WebM; audio AAC/Opus/FLAC/WAV; overlays compiled vector/SDF); don't
   transcode media already matching a profile. **ffmpeg stays a subprocess**
   (ADR-009).
4. **Compile** — editable graph → compact `runtime.bin`; strip undo/drafts/hidden/
   temp/thumbnails/diagnostics; retain authoring metadata when recovery enabled.
5. **Package** → `Guide Name.guide`.
6. **Portable exe when requested** — copy the stable player, append the package,
   write the footer `[magic/version/offset/length/SHA-256]` → `Guide Name.exe`.
7. **Offline verification** — run with networking disabled, from an empty temp
   dir, without WiredFRONT/Workbench/system-codecs/WebView2. Any external resource
   load = the export FAILS.

- **Export Option C (fallback only):** a media-only `.mp4` (or the old ColdLight-
  emitted vanilla HTML) for platforms that reject executables — NO branching/
  touch/steps/comments/nav/presenter. Explicitly a fallback, never the canonical
  guide. This is where the previous draft's "HTML player" lives now.

### Crate/target layout

```
crates/viziwiz        (renamed) WiredFRONT's capture overlay + GuidePilotSink wiring
crates/engine-steps   guide.json schema + GuidePilotSink
crates/doc            gains Block::Guide (WB-SLIDE)
crates/guide-format   NEW — the .guide serde model + ZIP container + integrity
crates/guide-player   NEW — native binary (winit + wgpu + libmpv + cue engine + egui chrome)
crates/guide-export   NEW — the export compiler + footer writer (+ Option C fallback)
```

## 4. Changes, in execution order (AFTER ratification — each its own PR)

1. **W-CAP** — palette/editor/Save-card/REC-pill onto `cl::treated`/`clay`; loupe
   6×; verify/seam the non-activating pill. Proof shots. *(claymorphism-first gate)*
2. **WB-DEMO** — `GuidePilotSink` + a headless drill; the chat-authored path
   stubbed behind the typed-tools dependency.
3. **WB-SLIDE** — `Block::Guide` + renderer + reviewer wiring + import seam.
4. **GF** — the `.guide` serde model + ZIP container + integrity + round-trip tests.
5. **GP** — the native player skeleton: winit window + wgpu clear + libmpv decode
   of one embedded video + the cue engine stepping a fixture graph + one SDF arrow.
   (Iterative — the player is the largest sub-program; land a proving skeleton
   first, then grow overlays/transitions/branching.)
6. **GX** — the export compiler + footer writer + an offline-verification drill
   (headless: export a fixture → assert the `.exe` opens itself from an empty temp
   dir with networking disabled).

GF precedes GP/GX because both consume the format. WB-SLIDE can proceed in
parallel with GF (it reuses the existing folder-guide contract, not the compiled
package). W-CAP gates everything (operator order).

## 5. Boundaries

- 🚫 **No Svelte / no web stack** — pure Rust/egui editors; native player. The
  spec's "Tauri 2 + Svelte 5" lines are overridden (`viziwiz-architecture-spec.md`
  header).
- 🚫 **No Tauri / no WebView2** in the player (spec §14) — native winit/wgpu/libmpv.
- 🚫 **ffmpeg stays a subprocess** (ADR-009), including in GX normalize.
- 🚫 **Content never mutated**; copy-never-move; inbox never-delete; sources never
  touched. Guides file as slides, not content.
- 🚫 **rrweb** — RETIRED (ratified, video-first S1).
- 🚫 No `#[allow]`, no re-export shims. 🚫 No blur/feathered borders (egui-build law).
- 🚫 **Never flatten a guide into a video** as the canonical form (spec §17) — the
  `.mp4` is Option C only.
- ⚠️ **Claymorphism-first gate:** W-CAP lands and is proof-shot BEFORE WB/GF/GP/GX.
- ⚠️ **WB-DEMO chat-authored path** depends on the chat typed-tools + pilot
  extensions — the sink + drill land now; the NL path waits, stated not smuggled.
- ⚠️ **Two chat surfaces** currently coexist post-merge (peer-window + rail hub) —
  a separate cleanup decision, out of scope for this plan.
- Out of scope here: WiredFRONT's full media-editing suite (generation, stitch,
  grade, transcode) beyond the capture overlay; the `PublishedAsset` manifest
  (its own WiredFRONT plan); OBS integration internals (GP emits the event).

## 6. Verification

- **W-CAP**: `cargo green`; palette/Save-card/REC-pill/editor screenshots at
  maximized AND `--window 1100x700`; a capture-smoke drill (a real snip lands in
  inbox; the loupe reads 6×; the REC pill doesn't steal focus).
- **WB-DEMO**: a headless pilot drill runs a fixture Flow → asserts a real
  `guide.json` + WebP do/done frames on disk, click rings at the driven rects;
  fingerprint collapse drops a no-op step.
- **WB-SLIDE**: a fixture guide steps end-to-end through the deck scenario (D2
  beats); the guide renders as one slide; `--screenshot` proof.
- **GF**: unit tests round-trip a fixture `.guide`; a tampered checksum is
  rejected; media assets are stored (not deflated).
- **GP**: the player opens a fixture `.guide`, decodes one embedded video via
  libmpv, steps the cue engine through its triggers, and renders one SDF arrow —
  proof capture of a running frame.
- **GX**: export a fixture guide → a headless offline-verification drill opens the
  produced `.exe` from an empty temp dir with networking disabled and asserts zero
  external resource loads; the footer round-trips (magic/version/offset/length/
  SHA-256 verify).

## 7. Commit plan

One PR per §4 phase: `feat(viziwiz): claymorphism`, `feat(viziwiz): ai-guide
sink`, `feat(doc): guide slide`, `feat(guide-format): the .guide package`,
`feat(guide-player): native runtime skeleton`, `feat(guide-export): compiler +
portable exe` — each green + proof-shot, operator exe rebuilt, STATE.md updated.
W-CAP must land before the rest.

## 8. Disposition table — the no-silent-drop gate

Every original-app capability, accounted for. **PORTED✓** = already in the egui
port (verify in the pass). **REAUTHOR** = rebuild in egui idiom. **NEW** = net-new
(absent in original). **RETIRE** = dropped with ratified reason. Phase column uses
the new product keys.

| # | Original capability | Disposition | Phase |
|---|---|---|---|
| 1 | Snip modes Region/Window/Full/Freeform | PORTED✓ (`editor.rs`) | W-CAP verify |
| 2 | 7 still engine fns (monitor/window/region/freeform/from-png/freeze) | PORTED✓ (`engine-capture`) | — |
| 3 | Frozen-frame WYSIWYG + loupe magnifier | PORTED✓ — restore **6×** (port shipped 4×) | W-CAP |
| 4 | Arrow-key nudge (+Shift×10) on committed region | PORTED✓ (`editor.rs:144`) | — |
| 5 | Capture delay 0/3/5/10 | PORTED✓ — 5s restored this session (`lib.rs:950`) | — |
| 6 | Re-snip (repeat last shape) | PORTED✓ (`LastSnip`) | — |
| 7 | Auto-clipboard default-ON | PORTED✓ (`copy_to_clipboard`) | — |
| 8 | Window picker + refresh | PORTED✓ | — |
| 9 | Video fixed-N + Start▸Stop | PORTED✓ (`VideoRec`) | — |
| 10 | Format matrix mp4/vp9/av1 | PORTED✓ (`FORMATS`) | — |
| 11 | Audio none/system/mic + device enum | PORTED✓ (`AUDIO`, `list_audio_devices`) | — |
| 12 | 720p default + caps + REC timer + poster | PORTED✓ (`CAPS`, `poster_for`) | — |
| 13 | DXGI recorder-reuse cache | PORTED✓ (verify in `engine-capture`) | W-CAP verify |
| 14 | Steps rdev global hook + Ctrl+Shift+S/9/X | PORTED✓ (`install_hook`, `Hotkey`) | — |
| 15 | do→done two-frame + aHash fingerprint collapse | PORTED✓ (`engine-steps`) | — |
| 16 | Window-following (skip own PID) + click ring | PORTED✓ (`StepRec`) | — |
| 17 | WebP@82 step frames, seed-name from title | PORTED✓ (`seed_name`) | — |
| 18 | Forced-metadata Save card | PORTED✓ (`PendingGuide`, C2) | — |
| 19 | Non-activating REC pill (WS_EX_NOACTIVATE) | **VERIFY / REAUTHOR** (Win32 seam if focus-steal confirmed) | W-CAP |
| 20 | guide.json schema + 11-field StepRec (folder artifact) | PORTED✓ (`engine-steps`) | — |
| 21 | GuideReviewer UI (do→done cards, ring, reorder/delete/caption) | REAUTHOR — W5 reviewer (engine `edit.rs` exists) | WB-SLIDE |
| 22 | `update_guide` (jail, renumber, prune) | PORTED✓ (`edit.rs`) | — |
| 23 | Frame→slide seam | REAUTHOR — associate + `Block::Guide` | WB-SLIDE |
| 24 | Guide refused by inbox-keep (becomes slides) | PORTED✓ (content-tree law) | — |
| 25 | ViziWiz viz-editor ink (6 tones, width, undo, flatten) | RETIRE as a capture subroute (drift, removed this session); the `engine-media::ink` flatten engine is parked for D3 presenter annotate-and-save | — |
| 26 | Annotation-layer stream ink (ephemeral, pressure) | REAUTHOR — D3 presenter ink (in-session over slides) | (D3, separate plan) |
| 27 | rrweb session replay (3 pipelines, Replayer) | **RETIRE** (ratified: video-first S1/S2) | — |
| 28 | Session capture + inbox/keep/dump/rekey | PORTED✓ (S1 `session.rs`, `dpl-index`) | — |
| 29 | **Guide standalone export (§S5, never built)** | **RESHAPED → NEW**: `.guide` + native player + portable `.exe` (the old HTML plan is now Option C fallback) | GF / GP / GX |
| 30 | **AI / EguiPilot UI-driving demonstration** | **NEW** — `GuidePilotSink` (absent in original) | WB-DEMO |
| 31 | Guide-as-one-slide (`Block::Guide`) | **NEW** — was S5-planned, unbuilt | WB-SLIDE |
| 32 | The ViziWiz name (app identity) | PORTED✓ — rename `pixvix`→`viziwiz` DONE this session | — |
| 33 | Palette/editor styling | REAUTHOR — trait system (claymorphism) | W-CAP |

Row count = 33 dispositions covering every audited capability. RETIRE #27 is
operator-ratified; RETIRE #25 (capture-subroute ink) is operator-ratified this
session ("just remove this … this should be my ink tool while I run a
presentation" → D3). All others are PORT/REAUTHOR/NEW.
