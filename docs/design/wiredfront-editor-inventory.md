# WiredFRONT Editor — experience inventory (the fidelity reference)

Authored 2026-07-24 from the operator's two design references (the missing
gate KNOWN BROKEN #5 calls out). This is the FIDELITY BAR for every R1
editor pass — passes are measured against THIS, not against the invented
4-pane skeleton. Two surfaces, one design system (ColdLight, Voltage +
Native variants): the **WiredFRONT Editor** (capture→annotate→edit→narrate)
and the **Guide Workbench** (guide assembly/preview/export). R1 builds the
WiredFRONT Editor.

## Aesthetic — "COLDLIGHT VOLTAGE"

Near-black canvas (`surface::WELL`), neon **voltage purple** (~`#a855f7`) +
**cyan** accents, glow/bloom on active elements, thin bright edges, rounded
panels. This is the trait system's **cyber** stack (cyan glow + bright edge)
over the neubrutalist/claymorphic base — NOT flat. Selected/active =
lit-from-behind glow. Every panel is a rounded recessed plate.

## WiredFRONT Editor (reference image #2) — the R1 target

**Title bar.** Wordmark `WIRED FRONT` + `EDITOR`; center = project switcher
(`Project: DevPULSE Quickstart ▾`); right = theme selector
(`◆ COLDLIGHT VOLTAGE ▾`) + window controls.

**Workflow spine (top tabs, the backbone).** `CAPTURE · ANNOTATE · EDIT ·
NARRATE · PREVIEW · EXPORT`, each an icon+label; active tab glows. Right:
`⬈ EXPORT .GUIDE` (voltage button). These are the editor's MODES — the
whole surface changes per tab.

**Left column.**
- **SCENES** (＋): ordered scene cards, each a thumbnail + name + duration
  (Introduction 00:05 · Project Setup 00:12 · Import Assets 00:09 ·
  Generate Scene 00:18 · Export & Share 00:08). Selected scene glows.
- **ASSETS** (search · filter · grid): tabs `All · Images · Video · Audio`;
  rows = thumbnail + name + dims/duration + type badge (PNG/MP4/WAV).

**Center canvas (ANNOTATE/EDIT).** Title `Scene 2: Project Setup`. Toolbar:
select · hand · crop · rect · ellipse · circle · dashed-box · connector ·
freeform · arrow · text; zoom + fit. Canvas shows the scene frame with
ANNOTATIONS overlaid: numbered callout badges (①②③) with text bubbles,
holographic arrows (purple/yellow), box/spotlight highlights.

**Right column — tabs `INSPECTOR · LAYERS · ACTIONS`.** INSPECTOR:
- STEP SETTINGS: Step ID (`step_2_1`), Trigger (`Tap Anywhere ▾`), Delay ms.
- ANNOTATION: Type (`Holographic Arrow ▾`), Text, Style (`Voltage Purple ▾`),
  Animation (`Pulse ▾`), Duration ms (1500), Start Delay ms (200).
- POSITION & SIZE: 9-anchor grid, X/Y/W, Rotation.
- APPEARANCE: Glow Intensity (0.85), Thickness (3), Color swatch.

**Bottom.**
- **STEPS — SCENE 2**: ordered step cards (Highlight Button 00:02.5 · Show
  Settings Panel 00:03.0 · Highlight Save 00:02.0 · Wait for Action 00:04.0
  · ＋ Add Step). Each step = a beat with a duration.
- **NARRATION**: an audio waveform track (`narration_intro.wav`),
  00:07.5 / 00:12.0; transport (skip/play/skip/loop) + time.
- **PREVIEW** mini-panel: device (`1920×1080 ▾`) + play; Simulate input:
  `Touch · Mouse · Keyboard`.

**Footer.** `● Project Saved · Auto-save: On` + zoom.

## Guide Workbench (reference image #1) — the companion (Workbench product)

Theme `Coldlight Native`. Workspace tabs; `Capture ▾ · ＋ Add Step ·
▷ Preview · ⬆ Export`. Left ASSETS (Screenshots/Media/Overlays/Arrows/
Captions/Components/Scene Steps, with counts). Center = live guide preview
with a spotlight overlay + step callout ("Create a New Project", 2/7,
Next→). Right STEP INSPECTOR: Step Title, Trigger (`Click`), Advance Mode
(`On Click`), Overlay Style (`Spotlight`), Target (`.btn-new-project`),
Animation (`Pulse`), Timing (Delay 0.6s · Display 4.0s · Transition 0.4s),
Notes, Show on Mobile. Bottom transport: step dots 1–7, play, Touch/Click
mode, zoom, Guide Progress 28%.

## Gap: what exists vs the bar (2026-07-24)

BUILT (R1a/R1b.1): a single-mode 4-pane Composer (Palette/Timeline/
Inspector/Preview) + `comp.json` doctor + ffmpeg concat render. It is the
**EDIT** tab's seed only. MISSING vs the bar: the workflow-tab spine, the
SCENES model, the ANNOTATE canvas + annotation model (arrows/callouts/
spotlights with per-annotation inspector), STEPS-per-scene, the NARRATION
audio track, PREVIEW device/input simulation, EXPORT .guide, and the
Voltage (cyber) styling.

## Pass order toward the bar (drives the loop)

1. **R1s — editor shell**: the workflow-tab spine + SCENES/ASSETS rails +
   Voltage restyle; EDIT tab hosts the existing timeline; other tabs honest
   stubs. (THIS pass)
2. **R1-scene** — the scene data model (comp → scenes; each scene = frame +
   steps + annotations + narration ref).
3. **R1-annotate** — the annotation model + canvas tools + per-annotation
   inspector (arrows/callouts/spotlights, Voltage styles).
4. **R1-steps** — steps-per-scene with triggers/timing.
5. **R1-narrate** — narration audio track + waveform.
6. **R1-preview** — device + Touch/Mouse/Keyboard simulation.
7. **R1-export** — `.guide` export (ties to Program G).
Interleaved with the earlier R1 render/gen/matte/elements passes as they fit.
