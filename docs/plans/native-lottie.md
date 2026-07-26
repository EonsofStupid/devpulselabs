# Native Lottie — AE-independent rendering, editable presets, no Adobe at render time

Status: PLANNED 2026-07-25 via `/deep-plan` (Fable architects; two Opus
agents researched the renderer landscape + the Bodymovin format, findings
spot-checked). Operator-directed: "deep-plan the native AE-independence
roadmap (Lottie import + native render first — the real path to not
needing AE)." This is the COMPLEMENT to the AEP orchestrator, not a
replacement: native Lottie is the Adobe-free future for Lottie-safe
content; the orchestrator remains the bridge for the heavy `.aep`
library. AE is OUTGROWN gradually, never auto-deleted.

## 1. Context — the honest framing

The operator wants devpulse-labs to eventually not need After Effects.
The legitimate path (established this session: extracting Adobe's engine
is a non-starter, technically and by licensing) is NATIVE
reimplementation via the open **Lottie/Bodymovin** interchange format:
render AE-style motion graphics in-app with zero Adobe at render time,
and edit presets (color/font/text) as plain data. Two hard truths from
the research bound the ambition:

- **Lottie carries a large-but-not-total slice of AE.** Survives:
  shapes, keyframes + bezier easing, masks, alpha/luma track mattes,
  linear/radial gradients, trim paths, transforms, precomps, text.
  Does NOT survive: effects, 3D layers, expressions, blend modes, and
  video/footage/audio layers. So native rendering retires AE for the
  vector/motion-graphics/title class (the bulk of videohive lower-thirds
  + typography); the orchestrator stays for the residue.
- **There is NO headless AE→Lottie converter.** Bodymovin is a GUI CEP
  panel with no CLI; `aerender` can't run it (Adobe's headless-script
  gap is an open, unfilled feature request). So the operator's 9,021
  existing `.aep`/mogrt do NOT auto-convert — migration is a manual,
  brittle, one-comp-at-a-time GUI campaign with per-project QA. Native
  Lottie is therefore for NEW/authored/manually-exported content first;
  bulk migration of the existing library is a later, bounded,
  operator-directed effort — not a scriptable button.

The win that IS clean: **preset editing is pure JSON** (verified against
the canonical schema) — a fill's color is `c.k` (RGBA), a text layer's
content/font/size/color is its text document `t.d.k[].s` (`t`/`f`/`s`/
`fc`). Recolor + retext with no AE, delivering the "change the color,
font, and text of a preset" goal the mogrt work pointed at.

## 2. Ground truth (this session — provenance noted)

**Renderer landscape (Opus agent A; Fable spot-checked existence +
features via `cargo add --dry-run`):**
- **`rasterlottie` 0.2.1** (2026-07-10, MIT/Apache) — **pure-Rust CPU**
  (tiny-skia). Features confirmed present: **`text`, `images`, `gif`**
  (default), `dotlottie` opt-in. `render_frame(&anim, time, cfg)` →
  RGBA `Pixmap` (arbitrary-`t` SEEK). Coverage: shapes, gradients, masks,
  **track mattes (alpha+luma+inverted)**, glyph text, images (embedded +
  external), precomps + time-remap, merge paths. ✗ expressions,
  text-animators, layer effects. **Trivial build, identical Win+Linux, no
  C/C++ toolchain** → and being a dep, the workspace `unsafe_code` deny
  is untouched. RISK: young (0.2.x, single maintainer, "validated corpus"
  not full parity) — correctness on arbitrary real exports UNPROVEN; CPU
  scrub cost on big comps.
- **`velato` 0.11 + vello** (Linebender, active) — pure-Rust **GPU**
  (wgpu). Clean wgpu-texture path, but **NO TEXT, NO IMAGES**, no
  time-remap → disqualifying for a titles tool; also needs a
  compute-capable GPU (a Wayland-mini-PC liability) and non-trivial
  device-sharing with egui-wgpu.
- Fallback **`rlottie`/`rlottie-sys`** (Samsung C++ FFI, MIT, moved to
  Codeberg — maintained) — mature, proven, CPU, strong coverage
  (masks/mattes/gradients/images), ARGB buffer like rasterlottie. COST:
  compiles rlottie C++ via cmake (Windows/MSVC build less proven),
  limited text shaping. `thorvg`/`dotlottie-rs` have the best coverage
  (proper text) but the heaviest build (vendored ThorVG + bindgen).

**Bodymovin format (Opus agent B; Fable verified structures against the
canonical Lottie schema):** stable, documented. Top level `v/fr/ip/op/
w/h/assets/layers`; layer `ty` (shape=4, text=5, image=2, precomp=0…);
animatable props `{a,k}`; keyframes `{t,s,i,o}` with a **v4 `e`
end-value vs v5+ next-`s`** convention a renderer must handle; text doc
`t.d.k[].s`; fill color `fl.c.k`. Recolor/retext = pure JSON edits,
CAVEATS: 0–255 (v4) vs 0–1 (v5+) color normalization; glyph-SUBSETTED
exports can't add unseen characters (need the referenced full font).
Non-AE authoring paths exist (python-lottie, Glaxnimate) for net-new
presets.

**Repo spine (Fable read directly):** NO vector rasterizer exists —
`crates/vector` is only an epaint triangle-mesh helper (ring sectors,
polylines); only wgpu (via eframe) is present. So native Lottie is NEW
capability. `dpl-comp::Source` is `Artifact | Element` (a third `Lottie`
kind slots in). `ClipPlayer` (video→texture, seek/play) is the exact
shape a `LottiePlayer` mirrors. The render/export path is ffmpeg
(engine-media). `media_kind` maps `.lottie`→element; `.json` is
unclassified (a design point).

## 3. Target design

**Renderer decision: `rasterlottie` (pure-Rust CPU), `rlottie` the
ratified fallback.** It is the only pure-Rust option combining
arbitrary-`t` seek with text+images+mattes — the features a titles tool
cannot ship without and velato lacks — at zero build cost on both target
OSes. Its maturity risk is bought down by a CHEAP first-phase spike
against real Lottie (it's zero-toolchain); if correctness disappoints,
`rlottie` is the drop-in CPU fallback (same RGBA-buffer shape). A single
rasterizer feeds BOTH the timeline preview (buffer→egui texture) and
export (buffer sequence→ffmpeg).

- **`dpl-lottie` crate** (pure, no egui): `load(path) -> Doc` (`.json`
  + `.lottie` zip), `render_frame(&Doc, t_secs) -> Rgba8` over
  rasterlottie; `duration`/`fps`/`size`; the JSON edit layer
  (`recolor(fill_path, rgba)`, `set_text(layer, str)`) handling the
  0–1/0–255 + glyph-subset caveats honestly (report when a retext can't
  render a character). Golden-frame + edit tests.
- **`Source::Lottie { rel }`** in `dpl-comp` (schema grows additively).
- **`LottiePlayer`** in the composer (the `ClipPlayer` analog): frame
  cache + a background render thread (the `RenderState` pattern) so
  scrubbing a big comp never stalls the UI.
- **Import + library**: `.json`/`.lottie` become a first-class media
  kind; imported to content; the library/browse cards show a real frame-0
  preview (reusing R2a.2's budgeted-thumb machinery).
- **Editable presets UI**: recolor/retext controls that patch the doc —
  the preset-customization goal, native.
- **Export**: lottie frame sequence → ffmpeg (mp4/webp/avif/gif),
  reusing the engine-media export selector from the AEP plan.

## 4. Changes, in execution order (phased PRs)

- **L1 — renderer spike + `dpl-lottie` foundation:** add `rasterlottie`
  (ratified); `load` + `render_frame(t)` + duration/size; validate
  against REAL Lottie (public AE-authored samples + a couple the operator
  exports) — the GATE on rasterlottie vs the rlottie fallback. Golden
  tests.
- **L2 — Lottie on the timeline:** `Source::Lottie` + `LottiePlayer` +
  frame cache/bg thread; scrub/play a lottie layer natively in the
  editor.
- **L3 — import + library previews:** `.json`/`.lottie` media kind +
  import seam + real frame-0 cards.
- **L4 — editable presets:** recolor/retext JSON patching + UI (the
  caveats surfaced honestly).
- **L5 — export + AE-free authoring:** lottie→ffmpeg export; evaluate a
  python-lottie/Glaxnimate authoring path for net-new presets; a MANUAL
  "convert this comp" (AE-orchestrated, one at a time) only if the
  operator wants it — NOT a batch pipe.

## 5. Boundaries

🚫 No Adobe binary extraction (settled). 🚫 The existing 9k are NOT
auto-converted — no headless Bodymovin exists; bulk migration is a
later, manual, operator-directed campaign, never silently promised. 🚫
Sources never mutated; edited presets write NEW artifacts (`/content`
law). 🚫 Own crates stay `unsafe_code`-free (the renderer dep may FFI;
that's allowed). ⚠️ Ask-first: the renderer DEPENDENCY (rasterlottie vs
rlottie — §Decisions); a second dep only if the fallback is taken. Out
of scope: effects/3D/expressions (Lottie can't carry them — AE
orchestrator's job); a full Lottie authoring editor; blend modes.

## 6. Verification

1. **L1 spike (gates the renderer):** render N real Lottie files (public
   AE-authored + operator exports) frame-at-`t`; a golden-frame test
   pins a known pixel/hash; visually confirm text, gradients, masks,
   mattes, images. If rasterlottie mis-renders the corpus → switch to
   rlottie (re-ground, same crate shape).
2. `cargo test -p dpl-lottie` + `cargo green` each phase.
3. Real-exe drill: drop/import a `.lottie` → it plays on the timeline
   natively (no ffmpeg, no AE), scrubs smoothly (cache), exports to mp4/
   webp; recolor/retext visibly changes the render. Proof shots.
4. Editable-preset proof: patch a fill color + text string in code →
   the rendered frame reflects it; the glyph-subset + color-convention
   caveats are unit-tested.

## 7. Commit plan

Branch per phase off main (`feat/lottie-1-spike`, …); code+tests + docs
commits; PR → squash → exe rebuild → STATE. L1 first, fully; the
renderer dep is ratified before L1.

## L1 GATE OUTCOME (2026-07-25) — renderer decision REVERSED, then re-decided

The L1 spike did its job and overturned the §3 renderer choice:

- **rasterlottie FAILED the gate.** It renders static shapes (hermetic
  tests pass — a red rect rasterizes, center red / corner transparent),
  but on the FIRST real file (a mainstream public 800×800/25fps sample)
  it HARD-REFUSES with "animated value uses unsupported keyframe
  features" on the layers' position/scale/opacity — the most basic
  animated props. Traced to source: its keyframe parser rejects any
  keyframe it can't fully model and there is NO permissive override
  (forcing every `allow_*` flag on still refuses). Unusable as a general
  editor engine. This is exactly the maturity risk agent A flagged.
- **Operator re-ratified → ThorVG.** Ground truth on ThorVG (`thorvg`
  0.5.0 / `thorvg-sys` 0.3.0+thorvg-1.0.7): **builds clean on Windows/
  MSVC in ~94s** (no meson/ninja/cmake drama — the C++ build fear is
  gone); **PARSES the real gatin sample correctly** (total_frame=80,
  duration=3.2s, 800×800 — the exact file rasterlottie refused);
  production-proven renderer (Samsung stickers, dotLottie, LVGL); has a
  built-in SLOT system (`gen_slot`/`apply_slot`) that directly delivers
  the editable-presets goal (recolor/retext named properties).
- **Adoption architecture RATIFIED (operator): a SUBPROCESS render
  helper** (not linked in-process). ThorVG is linked C++ FFI whose
  `SwCanvas::set_target` is `unsafe`, colliding with the workspace
  `unsafe_code = "deny"` law and the subprocess-only ADR-009 pattern
  (ffmpeg/aerender). So a small leaf binary links ThorVG/dotLottie and
  renders frames; the app SPAWNS it (like ffmpeg). Every app crate stays
  unsafe-free and pure-Rust; trivially cross-platform (serves the Wayland
  target). The thorvg-rs 0.5 safe wrapper's animation→canvas mount is
  also awkward (young binding: `canvas.add` consumes, `Animation::
  picture()` borrows, `from_raw` is crate-private) — the helper sidesteps
  it via `dotlottie-rs` (a complete player) or `thorvg-sys` raw.

**Renderer decision NOW: ThorVG via a subprocess render-helper.** §3/§4
are superseded by this on the engine + integration shape; the crate
`dpl-lottie` stays as the app-facing façade that spawns the helper.

## 8. Evidence

**L1 LANDED (2026-07-25, `feat/lottie-native`):** the ThorVG render
path, folded into main as two crates:
- **`crates/lottie-render`** — the ADR-009 subprocess helper (the ONE FFI
  leaf; opts out of the workspace `unsafe_code = deny` with a documented
  crate-level lint). Links `thorvg-sys` (ThorVG C++ vendored), raw FFI
  contained in one `engine` module behind a safe `Lottie` facade. Three
  modes: `frame` (one-shot PNG), `export` (PNG sequence → ffmpeg), `serve`
  (persistent: stdin time → stdout raw RGBA, the scrub stream).
- **`crates/lottie`** — the app-facing façade: NO deps, unsafe-free,
  spawns the helper (`helper_path()` beside the exe / `DPL_LOTTIE_RENDER`
  / PATH). `Player::open` + `frame_at(t)` (the timeline scrub path);
  `export_sequence` (ffmpeg). Unit tests: READY-header parse, platform
  binary name.
- **PROOF:** ThorVG renders REAL Lottie natively — frame-0 (cat, tail
  down) and t=1.6s (tail raised) rendered through the workspace binary;
  the gate `cargo green ✔`. rasterlottie's hard-refusal of the same file
  is what overturned the original choice. Screens shared with the
  operator.
- **NOT yet wired into the app** (deliberately, to avoid colliding with
  in-flight composer work): `Source::Lottie` + the timeline `LottiePlayer`
  + import seam + the ThorVG slot recolor/retext are the next milestone.
  Packaging note: the helper builds via `cargo build --release -p
  dpl-lottie-render` and must ship beside the app exe (a build-wiring
  follow-up so `cargo green`'s app build emits it too).

## Provenance (deep-plan)

Opus agent A researched the Rust Lottie-renderer landscape; Opus agent B
the Bodymovin format + AE-export reality. Fable spot-checked crate
existence/features (`cargo add --dry-run` → rasterlottie 0.2.1 with
text/images/gif; velato 0.11 vello-only), verified the schema structures
against the canonical Lottie spec, and read the repo rendering spine
directly (no existing rasterizer). The renderer decision, phasing, and
scope are Fable's.
