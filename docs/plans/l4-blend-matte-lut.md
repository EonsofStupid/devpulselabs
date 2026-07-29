# L4 — blend · matte · LUT: the estate unlock (screen/add overlays, luma-matte wipes, .cube grades)

Status: PLANNED 2026-07-28 (operator: "sounds good can we continue?").
The fourth AE-independence rung. The 2026-07-28 estate census (30 packs,
~60 GB, Opus-swept + Fable spot-checked) showed most non-template media is
NOT straight-alpha: light-leak/dust overlays are `yuv420p` meant for
SCREEN/ADD blending (cinepunch 8K leaks, trendy film dust), the RTFX
source movs and matte-transition packs are LUMA-MATTE wipes (grayscale
h264 defining reveals), and the Bundle ships 860 `.cube` LUTs. Three
ffmpeg mechanisms make all of it native — each proven LIVE against real
estate files before this plan (`l4_proof_strip.png`, §8).

## 1. Context

L3 landed alpha-over compositing (PR #110). The census verdicts split the
estate: straight-alpha packs work today; template packs await the
operator's Bodymovin/bake session (their lane, parked); and a large
middle bucket — blend-mode overlays, matte wipes, LUT grades — needs only
GRAPH VOCABULARY, no Adobe. This pass adds that vocabulary end-to-end:
schema, one-pass export graph, honest preview, inspector controls.

## 2. Current state (ground-truthed 2026-07-28 on main `51997ee`)

- L3 shipped: `Layer {x, y, scale, tint}` (additive v1);
  `engine-media::comp_args/render_comp` (concat `[base]` + chained
  overlay legs, black gap legs, PngSeq inputs, held stills);
  composer `render_parts` → `render_comp_resolving` (temp Lottie seqs);
  overlay preview lanes with machine-probed caps; the FX lane + OVERLAY
  inspector; the rtfx enricher + tag chips; `--fx-drill`.
- **#120 (post-L3) folded RenderState/IngestState/ExportState into the
  fabric `JobBoard`** — `start_render`/`start_ingest` now publish jobs
  and the status bar reads the board. L4's render changes graft onto the
  BOARD-backed paths (re-read them in-session before editing; the L3-era
  polled-state shapes in memory are stale).
- `media_kind`: `.cube → "element"`; the enricher tags by name grammar.
- ffmpeg 8.1.2 LIVE proofs (2026-07-28, real estate files, scratchpad
  `p1_screen/p2_matte/p3_lut.mp4` + `l4_proof_strip.png`):
  - SCREEN: `[leg]trim,setpts+at/TB,scale=W:H,format=gbrp[lk];
    [prev]format=gbrp[b];[b][lk]blend=all_mode=screen:
    enable='between(t,a,b)'[o];[o]format=yuv420p` — works; base passes
    clean outside the window; 8K HEVC leak input fine.
  - MATTE: `[matte]trim,setpts,format=gray[m];[fg]…[f];[f][m]alphamerge
    [am];[prev][am]overlay=enable=…:eof_action=pass` — the brush stroke
    carved exactly; matte + fg sized alike (1080p estate norm).
  - LUT: `lut3d=<file>` regrades correctly. Windows path hazard: the
    filter arg parser eats `C:` — needs escaping (`\` → `/`, `:` →
    `\\:`) or relative paths; the proof used cwd-relative. A helper +
    unit tests own the escaping.

## 3. Target design

1. **Schema (dpl-comp, additive, stays v1).** `Layer` gains:
   `#[serde(default)] blend: Blend` where `Blend { Normal(default),
   Screen, Add }` (kebab serde; Normal skipped on disk); `matte:
   Option<String>` — rel of a luma-matte video whose LUMINANCE becomes
   this layer's alpha; `lut: Option<String>` — rel of a `.cube` applied
   to this layer. Validation: matte/lut non-empty when Some; unknown
   blend refuses at parse (serde). Blend/matte honored on OVERLAY
   tracks; `lut` honored on EVERY layer (grading a base cut is the
   860-LUT use case). Round-trip + back-compat tests.
2. **Export (engine-media).** `OverlayClip` gains `blend`, `matte:
   Option<PathBuf>`, `lut: Option<PathBuf>`; base `StitchClip` legs gain
   optional `lut` (a wrapping `CompBase::Clip` change or a parallel lut
   field — decide at the code seam, additive either way). Graph:
   - matte: extra input per matted overlay; `[m]trim/setpts (the LAYER'S
     in/out/at),format=gray[mN];[fx-leg][mN]alphamerge` before the
     overlay chain (proven form).
   - blend Screen/Add: the leg scales to its fit box then pads to FULL
     frame on BLACK at the offset position (`pad=W:H:x:y:black` — black
     is the neutral element for screen and add, so the pad is invisible)
     → `format=gbrp` both sides → `blend=all_mode=<mode>:enable=…` →
     back to `yuv420p` for the next link. Tint rides before the blend
     (colorchannelmixer unchanged). NOTE: a blend link replaces the
     overlay link for that layer; chain order preserved.
   - lut: `,lut3d='<escaped>'` appended to the leg (overlay) or wrapped
     around a base leg. `fn lut3d_escape(path) -> String` owns Windows
     escaping; unit-tested against `C:\x y\a.cube` shapes.
3. **Preview (composer) — exact where cheap, HONEST where not.**
   - LUT: exact — the layer's ClipPlayer pipe gains `lut3d` in its `-vf`
     chain (decode already runs through scale/crop; one more filter).
   - MATTE: exact — a matted layer's lane opens a TWO-INPUT pipe
     (`-i fg -i matte -filter_complex alphamerge` → rgba rawvideo), one
     process, same frame contract as today's ClipPlayer (a
     `clip_pipe_args` variant; the reader thread is unchanged).
   - BLEND: approximated — egui's painter has no screen/add compositing;
     the lane paints normal-alpha at reduced strength with a small
     `screen · export-exact` badge on the bar/inspector. The RENDER is
     exact; the viewer never lies about it (honesty law).
4. **Inspector (ColdLight-gated).** The OVERLAY inspector gains: BLEND
   segmented control (Normal · Screen · Add — tooltips in plain words);
   MATTE picker (choose a video artifact from the palette; `x` clears);
   LUT picker (choose a `lut`-tagged element; `x` clears) — LUT row also
   appears for a selected BASE layer. The enricher already tags cubes
   (`lut` tag joins the grammar map for `LUTs/` bucket paths).
5. **Ingest/enricher touch.** `bucket_tags` gains `("luts", "lut")`;
   `.cube` files keep kind `element` and surface in the picker by tag.
   AUTO-PAIR MATTES (operator-ratified): any media stem containing a
   `… Matte NN` shape gets tag `matte` at ingest (independent of the
   RTFX prefix grammar — the source movs are bare `Brush Matte 01.mov`);
   the inspector's MATTE picker lists `matte`-tagged artifacts FIRST and,
   when the selected layer shares its leading name token with exactly one
   matte (e.g. `Brush …` ↔ `Brush Matte 01`), the status line SUGGESTS
   it by name. Suggest, never auto-apply — the operator outranks magic.
6. **Harness.** `--l4-drill` — against a conventional scratch root
   (`inbox/base.mp4`, `elements/leak.mp4`, `elements/matte.mov`,
   `elements/grade.cube`): builds one comp exercising screen-blend +
   matte overlay + base LUT, renders synchronously, verdict on stdout
   (the `--fx-drill` pattern).
7. **Real-pack verification (the L3 follow-up, drive back).** Scratch-
   copy a small REAL RTFX subset (≥1 prerendered mov + its preview jpg +
   1 SFX) → pin → ingest through the app path → assert tags + shipped-
   preview poster + palette visibility. Part of §6, not a new surface.

## 4. Changes, in execution order

1. `crates/comp`: `Blend` enum + `blend`/`matte`/`lut` fields +
   validate + tests (round-trip, back-compat, kebab shape).
2. `crates/engine-media`: `lut3d_escape` + OverlayClip/base-lut fields +
   graph arms (matte inputs, blend pad+gbrp links, lut appends) +
   exact-arg tests per mechanism + a combined graph test.
3. `crates/composer`: render_parts mapping (matte/lut resolution to
   absolute paths, blend passthrough) grafted on the #120 board-backed
   render path; preview — lut in clip pipes, matte two-input pipe
   variant in `dpl-video` (`clip.rs` args builder + tests), blend
   approx + badge.
4. Inspector UI (invoke `coldlight-system` first): blend control, matte
   picker, LUT row (overlay + base), tooltips; enricher `lut` tag.
5. `--l4-drill` (app flag + hub delegate + composer method) + CLAUDE.md.
6. Real-RTFX ingest drill + proof shots + docs (STATE §8 evidence).

## 5. Boundaries

🚫 Multiply/overlay/soft-light blend modes (screen+add cover the
estate's leak/dust/glow use; more modes are additive later — multiply
needs a WHITE pad neutral, noted for then). 🚫 Matte auto-pairing SUGGESTS only
(picker ordering + a named status hint) — it never applies itself. 🚫 No comp-level output LUT (layer-level only).
🚫 Template variants / Bodymovin lane untouched (waits on the operator's
AE exports). 🚫 Agent placement still parked. 🚫 No new blend math in
egui — the preview approximates blends and SAYS so; export is truth.
🚫 ADR-009 unchanged: ffmpeg subprocess, args verbatim, no new crates.

## 6. Verification

1. Unit: comp (blend kebab round-trip; matte/lut back-compat), engine-
   media (per-mechanism arg vectors matching the §2 proven forms;
   `lut3d_escape` Windows shapes), dpl-video (matte pipe args), composer
   (render_parts mapping incl. matte path resolution).
2. `cargo green` (server1) + both CI lanes.
3. Real-exe `--l4-drill` on a scratch root with REAL estate files
   (cinepunch leak, RTFX brush matte, Bundle cube): render.mp4 ffprobed
   + frame extracts (leak lifts inside window, brush-carved overlay,
   graded base) — the l4_proof_strip, reproduced by OUR renderer.
4. Real-RTFX subset ingest drill: tags + shipped-preview poster + tag
   chips visible; proof shots 1280×800 AND 1100×700 (inspector with
   blend/matte/LUT controls; blend badge visible).
5. Pre-L4 comps load/play/render byte-identically (no blend/matte/lut).

## 7. Commit plan

Branch `feat/l4-blend-matte-lut` (this worktree). Commits: (1) comp
schema; (2) engine-media graph + escaping; (3) video matte pipe +
composer preview/render mapping; (4) inspector UI + enricher tag; (5)
drill + docs. Gate on server1 → PR → CI → squash → exe rebuild in THE
checkout + STATE.

## 8. Evidence

Planning: mechanisms proven live 2026-07-28 BEFORE the plan —
`p1_screen.mp4` (cinepunch 8K HEVC leak screen-blended over the base,
window-bounded, clean exit), `p2_matte.mp4` (RTFX `Brush Matte 01.mov`
luminance carving a red fill via alphamerge, window-bounded),
`p3_lut.mp4` (`CC - Basic 1.cube` regrade), composited in
`l4_proof_strip.png` (session scratchpad); estate census by Opus sweep
with Fable ffprobe spot-checks (flash-fx png/rgba confirmed 2118×1952).
Execution evidence lands here.
