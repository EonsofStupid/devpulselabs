# ViziWiz architecture — three products (operator's authoritative spec, 2026-07-24)

**Authority.** This is the operator's ratified architecture for the capture →
authoring → playback pipeline. It rules structure; passes execute against it.

**STACK OVERRIDE (operator directive: "THERE IS NO SVELTE").** The pasted spec's
"Recommended stack: Tauri 2 + Svelte 5" lines are **overridden**. The
implementation is **pure Rust** — no Svelte, no web/WebView stack:

- **WiredFRONT Editor** and **Workbench Editor** — **egui/eframe** (this
  workspace), styled by **ColdLight** (the Rust design system + trait compositor).
- **Native Guide Player** — native Rust: **winit + wgpu + libmpv** (+ serde,
  tokio). egui only for player chrome.

Everything else in this document — product boundaries, `.guide` format, export
pipeline, scene model, overlays, coordinates, modes — is authoritative as written.

---

## 1. Product boundaries (three products, no duplicated responsibilities)

1. **WiredFRONT Editor** — the media-production environment. Produces/edits
   finished media assets.
2. **Workbench Editor** — the presentation & guide-authoring environment. Turns
   finished media into slides, guides, cue sequences, touch-driven presentations.
3. **Native Guide Player** — the lightweight self-contained playback runtime.
   Opens/plays exported guides with **no** WiredFRONT, Workbench, browser,
   installer, internet, or external codecs.

They share asset & scene **schemas** where useful but must not duplicate work.

## 2. WiredFRONT owns (media production)

Capture · generation · image/video/audio editing · stitching · post-processing ·
narration recording · cleanup · upscaling · transcoding · caption generation ·
proxy generation · final render · asset provenance. Supports capturing
window/monitor/region, screenshot sequences, short screen segments; import
img/video/audio; generate via Higgsfield/other providers; trim/split/combine/
reorder clips; add audio/narration/music/SFX + VFX/post; publish finalized assets
into the shared library.

**WiredFRONT does NOT own** (→ Workbench): slide layout, guide step sequencing,
touch navigation, presenter controls, audience playback policy, timed advancement,
branching, slide-level arrows/instructions/cue behavior.

## 3. WiredFRONT output contract (PublishedAsset)

Every finished asset is published through a stable manifest, not handed over as an
untracked file. Workbench consumes the manifest and never re-inspects finalized
media unless validation detects stale metadata.

```ts
interface PublishedAsset {
  id: string; projectId: string;
  type: "image" | "video" | "audio" | "image-sequence"; title: string;
  sourcePath: string; proxyPath?: string; thumbnailPath?: string;
  width?: number; height?: number; durationMs?: number;
  frameRate?: { numerator: number; denominator: number };
  codec?: string; colorSpace?: string; audioChannels?: number; sampleRate?: number;
  generation?: { provider?: string; model?: string; prompt?: string; seed?: string; sourceAssetIds?: string[] };
  checksum: string; createdAt: string; updatedAt: string;
}
```
(Ported to Rust: a `PublishedAsset` struct + a manifest; the checksum/provenance
are load-bearing for the export freeze step.)

## 4. Workbench owns (authoring)

Slide/guide authoring · scene composition · layer placement · touch/click behavior
· timed advancement · presenter controls · audience output · step sequencing ·
instruction overlays · holographic arrows · highlights/spotlights · comments &
narration cues · branching · before/after states · guide export · presentation
export · OBS/display triggers. Operator can: create a slide from image/video,
compose multiple assets, add titles/captions/instructions/notes/callouts, add
arrows & focus regions, define click/tap/keyboard/timer/media-ended triggers,
define what changes on advance, preview desktop/touch/presenter/audience, insert
guide scenes into a deck, export `.guide` and a portable player.

## 5. Workbench is NOT a second media editor

Presentation-safe transforms only: position, scale, crop, opacity, mask,
playback start/end, volume, loop, simple fades, slide transitions, layer
entrance/exit. It does NOT duplicate full video/audio editing, color grading,
noise reduction, generation, long-form stitching, codec management, heavy post,
restoration. Substantive edits route through **"Open in WiredFRONT"**; when
WiredFRONT republishes a revised asset, Workbench updates the linked slide while
preserving placement, overlays, triggers, and cue logic.

## 6. Shared scene model (Workbench owns the authored graph)

`Asset → Slide/Scene → Layers → Steps → Triggers → Actions → Playback Policy`.
A finalized WiredFRONT composition becomes an **Asset**; Workbench places it in a
Slide/Scene that gains authored behavior WITHOUT altering the original media.

## 7. Guide data model — the `.guide` package

Canonical editable extension: **`.guide`** (e.g. `My DevPULSE Setup.guide`).
**NOT** `PulseGuide`, **NOT** `.pulseguide`. Contains guide source, finalized
media, runtime scene graph, theme data, integrity metadata.

```ts
interface Guide {
  schemaVersion: number; id: string; title: string; projectId: string; themeId: string;
  assets: Record<string, GuideAsset>; slides: GuideSlide[];
  navigation: GuideNavigation; startup: GuideStartup; metadata: GuideMetadata;
}
interface GuideSlide {
  id: string; title?: string; durationMs?: number;
  canvas: { width: number; height: number; aspectRatio: string };
  layers: GuideLayer[]; steps: GuideStep[]; entrance?: Transition; exit?: Transition;
}
interface GuideStep {
  id: string; name?: string;
  trigger:
    | { type: "tap-anywhere" } | { type: "click-anywhere" }
    | { type: "tap-target"; targetId: string } | { type: "click-target"; targetId: string }
    | { type: "delay"; milliseconds: number } | { type: "media-ended"; assetId: string }
    | { type: "keyboard"; key: string } | { type: "presenter" } | { type: "manual" };
  actions: GuideAction[];
}
```
**Actions:** show/hide/animate layer · animate arrow · pulse target · zoom region
· pan image · play/pause/seek video · play narration · fade audio · reveal caption
· darken background · highlight target · before/after state · advance slide/step ·
branch to slide · wait · emit OBS event · send presenter cue.
(These become Rust enums/serde types — the runtime cue engine's vocabulary.)

## 8. Workbench component library (declarative, reusable)

- **Slide:** full-bleed image/video · split · picture-in-picture · before/after ·
  browser frame · desktop-app frame · device frame · scrollable tall screenshot ·
  image gallery · video gallery · title · section divider · closing.
- **Instruction:** numbered step · caption card · tooltip · warning · tip · code
  block · keyboard shortcut · click/tap indicator · expected result · presenter
  note · comment · narration cue.
- **Focus:** spotlight · dim background · bounding box · pulsing border ·
  holographic arrow · animated path · cursor simulation · touch ripple ·
  magnified region · target lock · success state.
- **Playback:** timed/manual transition · tap/click-to-advance · media-ended
  transition · looping/paused/auto-play media · narration sync · background music
  · volume ducking.

## 9. Holographic arrows & overlays — runtime geometry

Arrows are **runtime vector / SDF geometry** (wgpu), NOT pre-rendered images. Each
supports: start/end point, curve, length, thickness, glow width/intensity, pulse
rate, direction, color, animation delay, target binding, entrance/exit animation.
One arrow component reused across resolutions, themes, layouts.

## 10. Normalized coordinates (mandatory in the runtime graph)

All overlays use normalized coords relative to the slide/bound media layer
(`{ "x": 0.6531, "y": 0.3814, "width": 0.142, "height": 0.086 }`), never raw
pixels. Prevents breakage across windowed/full-screen, 1080p/1440p/4K, touch,
other aspect ratios, and OBS. Workbench may keep design-space pixels for editing;
the **compiled runtime graph is normalized**.

## 11. Capture-to-guide workflow

Capture/generate in WiredFRONT → edit/stitch/narrate/post → publish asset → asset
appears in Workbench library → create slide from asset → add instructions/arrows/
focus/comments → define trigger (tap/click/timer/keyboard/media-ended) → preview
(guide / touch / presenter / audience) → export `.guide` or portable player.

## 12. WiredFRONT ⇄ Workbench linking (no duplication)

Immutable asset IDs + versioned revisions (`asset://devpulse-quickstart/plugins-screen@4`).
Workbench stores asset id, revision policy, placement, crop, transforms, playback
range, overlays, cue logic. Revision policies: **Pinned** · **Track latest
compatible** · **Prompt before update**.

## 13. Native Guide Player (winit + wgpu + libmpv)

**No Tauri, no WebView2.** Native Rust runtime: create the window, full-screen/
windowed, open `.guide`, decode embedded video/audio, render images/slides/arrows/
highlights/masks/overlays, handle touch/mouse/keyboard/presenter/timing, advance
slides/steps, preserve progress, fully offline. **Division of labor:** `wgpu`
renders composition + effects + arrows + overlays + transitions; `winit` windows +
input; the **Rust cue engine** owns timing/navigation/branching/policy; **libmpv**
is ONLY the embedded audio/video decode/playback subsystem (not the format, not
the renderer, not navigation). egui only for chrome (controls, volume, TOC,
settings, progress, debug).

## 14. Why not Tauri for the player

Tauri on Windows relies on WebView2 → the portable player would depend on an
installed WebView2 or bundle a large runtime. Native Rust avoids the browser
dependency and gives tight control over startup, decode, touch, render, memory,
GPU selection, offline, and package loading. (Tauri was only ever acceptable for
installed authoring apps — and per the stack override, we don't use it there
either; the editors are egui.)

## 15. `.guide` package format

A `.guide` is a self-contained package but not independently executable without
the player. Contains: guide manifest, compiled runtime scene graph, finished
image/video/audio assets, overlay defs, theme/shader presets, authoring metadata,
asset provenance, integrity hashes, package version, migration metadata.
Structure: `guide.json` · `runtime.bin` · `assets/` · `themes/` · `shaders/` ·
`authoring/` · `integrity.json`. Implement as a **ZIP-compatible container** first
(inspectable, recoverable, debuggable). Use **stored/selective** compression —
never recompress already-compressed media (MP4/WebM/JPEG).

## 16. Self-contained portable export (`.exe`)

`.guide` (editable) and the portable `.exe` are DIFFERENT deliverables. The `.exe`
= native player + embedded `.guide` + footer `[magic/version/offset/length/SHA-256]`.
On launch it opens itself, locates the footer, finds + checksum-verifies the
embedded package, loads the runtime graph, starts playback — one double-click file,
no install. It's an `.exe` because Windows requires executable code to launch
independently. User-facing name may be "Portable Guide" / "Guide Capsule" /
"Self-Contained Guide"; the editable source stays `.guide`.

## 17. Reopening & editing

Workbench opens both `.guide` packages AND exported `.exe` players: locate the
embedded package, verify checksum, extract/mount, load authoring metadata, restore
the slide/step graph, allow edit + re-export. The exported guide is simultaneously
a player, a deliverable, a backup, and a recoverable authoring source. **Never
flatten a guide into a video.**

## 18. Export pipeline (strict preflight)

1. **Validate** — every asset exists; every WiredFRONT revision resolves; no
   remote dependency/CDN/font/script; every trigger→valid target; every
   branch→valid slide; every codec supported by the bundled player; works with
   networking disabled.
2. **Freeze** — resolve links to exact revisions, copy into the package, record
   provenance + checksums (later source changes can't alter the export).
3. **Normalize media** — approved runtime profiles (img PNG/JPEG/WebP; video
   controlled MP4/WebM; audio AAC/Opus/FLAC/WAV; overlays compiled vector/SDF;
   narration JSON/binary). Don't transcode media already matching a profile.
4. **Compile** — editable graph → compact runtime; strip undo history, unused
   assets, drafts, hidden variants, temp gen files, editor thumbnails, unresolved
   comments, diagnostics; retain authoring metadata when recovery is enabled.
5. **Package** → `Guide Name.guide`.
6. **Portable exe when requested** — copy the stable player, append the package,
   write the footer → `Guide Name.exe`.
7. **Offline verification** — run with networking disabled, from an empty temp
   dir, without WiredFRONT/Workbench/system-codecs/WebView2. Any external resource
   load = test FAILS.

## 19. Export options

- **A — editable `.guide`**: your library, future editing, version control,
  backups, moving machines, opening in Workbench. Needs Workbench or the player.
- **B — portable `.exe`**: sending to others, USB, offline demos, training,
  machines without Workbench, guaranteed double-click. Bundles player + package +
  all media/shaders/metadata.
- **C — media-only `.mp4`** (fallback): platforms that reject executables, social
  uploads, non-interactive playback. NO branching/touch/steps/comments/nav/
  presenter. A fallback, not the canonical guide.

## 20. Presentation & guide modes (one graph, two policies)

- **Guide mode:** tap/click target, wait-delay, advance-on-media-end, instructional
  arrows, user-controlled nav, optional progress, branching.
- **Presentation mode:** advance from presenter console/touch/timed-cue, OBS
  actions, presenter notes, audience sync, manual override, delayed transitions.
- A guide slide is insertable into a deck; a deck slide is exportable as part of a
  guide. The shared scene graph prevents duplicate authoring.

## 21. Final architecture (Rust everywhere)

```
WiredFRONT Editor (egui + ColdLight + Rust)
  capture · img/video/audio gen · img/video/audio edit · stitch · post · narrate
  → publish finished assets
        │
        ▼
  Shared Asset Library
        │
        ▼
Workbench Editor (egui + ColdLight + Rust)
  slide/guide authoring · layers · holographic overlays · comments · triggers
  · timed transitions · presenter/audience · OBS cues · export compiler
        │
   ┌────┴────┐
   ▼         ▼
 .guide   Portable Guide.exe
              │
              ▼
   Native Rust Guide Player (winit + wgpu + libmpv)
```

## 22. Final decisions

WiredFRONT = capture/generation/media-edit/stitch/post. Workbench = slide/guide/
overlay/cue/touch/presentation authoring, consuming finished WiredFRONT assets
(never duplicating media editing). Canonical editable extension = **`.guide`**
(never `.pulseguide`). The self-contained double-click export is a portable
**`.exe`**. The player is **native Rust** — no Tauri/WebView2/browser/internet/
external codecs. **libmpv** = embedded media playback only. **wgpu** renders
slides/effects/arrows/overlays/transitions. **winit** = windows + input. The Rust
**cue engine** owns timing/navigation/branching/policy. WiredFRONT & Workbench
share ids/metadata/schemas but keep strict ownership boundaries. **The whole
system is Rust/egui — there is no Svelte and no web stack.**
