# L3 — alpha overlay: FX elements composite on the timeline (export · preview · transform/tint) + the RTFX ingest enricher

Status: PLANNED + RATIFIED 2026-07-27 (operator: "lets plan it out and get it
built"; ratified full-pass scope, prerendered+SFX pin roots, DYNAMIC
resource-probed preview caps) — implementing.
The third rung of the AE-independence ladder. The operator's videohive packs
(ground truth: `videohive-GXVeOIgF-rtfx-generator-440-fx-pack`, 711 prerendered
PNG-codec RGBA straight-alpha `.mov` @1920×1080/24fps + PNG seqs + 703 SFX)
become USABLE: an FX element sits ON TOP of footage with correct alpha,
positioned/scaled/tinted, visible live in the preview AND in the exported mp4.
Also closes the L2 honesty gap (Lottie layers join the export) and fixes a
verified ingest-index bug that would make ingested packs invisible.

## 1. Context

The escape from the AE subscription is prerendered + native assets. The Pack
Shelf (pin/copy-on-use/ingest) already lands RTFX movs in the library as
`kind="video"` artifacts, and the timeline can CUT them — but export is
concat-only (`render_stitch`), so an overlay element is worthless: it needs to
composite OVER a base clip. Preview likewise follows a single active layer.
L3 delivers layered compositing end-to-end, plus the tag enrichment that makes
440-element packs browsable, and the pack's own preview jpg/mp4 as posters
(an alpha element's first frame is blank — generated posters would be wrong).
Agent-driven placement ("add a fire flash at 3.2s") is the NEXT rung; this
pass builds the substrate it needs (tags + overlay schema).

## 2. Current state (ground-truthed 2026-07-27 on main `68bbf4e`; Opus-swept,
##    Fable spot-checked — provenance in §8)

- `crates/comp/src/lib.rs` — `Layer {id, source, start, in_pt, out_pt}`;
  `tracks: Vec<Track>` exists in schema ("stack back-to-front, track 0 first")
  but the composer UI only touches `tracks.first_mut()`. `SCHEMA_V=1`, grows
  additively. Shape law test: comp.json contains no `'#'`.
- `crates/engine-media/src/lib.rs::stitch_args` (l.328) — concat filtergraph,
  every leg forced `yuv420p`, `-an`, libx264 + faststart. `StitchClip {path,
  in_pt, out_pt, still}`. `render_stitch` refuses an empty clip list.
- `crates/composer/src/lib.rs`:
  - `active_layer_at` (l.626) returns the FIRST layer covering `t` — single
    active layer everywhere. `tick_playback` (l.635) keeps ONE lane live:
    `preview: Option<(String, ClipPlayer)>` | `preview_still` |
    `preview_lottie` — single-slot Options keyed by layer id, cleared
    mutually. 0.35s re-seek tolerance (every seek respawns the ffmpeg pipe).
  - Both paint sites (viewer_pane l.2337, annotate l.3492) call
    `painter().image(tex, rect, uv, Color32::WHITE)` — the 4th arg is a
    per-draw TINT (unused today). Straight-alpha textures upload via
    `from_rgba_unmultiplied` and already blend correctly when stacked. Video
    paints cover_uv (crop); Lottie paints fit_aspect + FULL_UV (letterbox).
  - `add_clip` (l.555) is the ONE add-to-timeline choke point → always
    `Source::Artifact {rel}`. `stitch_clips` maps Artifact only; Element AND
    Lottie are skipped from export (L2's honest note).
  - `Source::Element` is constructed by tests ONLY — a fully-plumbed
    placeholder (skip in preview + stitch). No element-resolve work needed:
    ingested RTFX movs are `kind="video"` (via `media_kind`) and already
    surface on the FOOTAGE palette → `add_clip` → Artifact.
- `crates/video/src/clip.rs::ClipPlayer` — per-instance ffmpeg pipe thread,
  960×540 RGBA, wall-clock (-re) paced; N players = N pipes, per-player
  drift. `crates/lottie/src/lib.rs::Player::frame_at` is SYNCHRONOUS on the
  UI thread (~2MB read/frame at the 720px cap) — multiple live Lottie lanes
  serialize; `export_sequence(lottie, w, h, fps, out_dir)` (batch,
  `frame-%05d.png`) exists and is unused by the app.
- `crates/index` — `Record.tags: Vec<String>` exists; NO tag query/filter
  anywhere (in-memory kind filters only); tags render once (wiredfront
  gallery card meta). `set_tags` has zero callers. `enrich_element`
  (composer/src/import.rs l.277) is the pack-aware enrichment precedent
  (mogrt name/tags/poster).
- **BUG (verified)**: `start_ingest` (composer l.1369) → `ingest_copy(&content_root(),…)`
  → `Index::open(&root.join("artifacts.db"))` (packs.rs l.211) writes the
  LEGACY DB that ADR-014's migration retired; every read surface uses
  `open_default()` → `.userdata/index.db`. On a migrated install, ingested
  packs are indexed into a file nothing reads → invisible in the library.
- ffmpeg 8.1.2 (gyan full) ground truth, proven live on the real pack (§8):
  bare `overlay` handles straight alpha (no premultiply, no fringe);
  `setpts=PTS-STARTPTS+<at>/TB` places a trimmed leg at timeline time;
  `enable='between(t,a,b)'` + `eof_action=pass` bound the window (default
  eof_action=repeat FREEZES the last frame); `scale` on the overlay leg
  preserves alpha; `colorchannelmixer=rr:gg:bb` tints, `aa` defaults to 1
  (alpha preserved; setting `aa`<1 = opacity); PNG-seq inputs need
  `-framerate`/`-start_number` BEFORE `-i` (Sources seqs don't start at 1);
  NO fps filter needed on overlay legs (framesync repeats 24fps over a 30fps
  base, SSIM 0.9993 vs explicit fps=30); chained `[b][o1]overlay[t];[t][o2]
  overlay[out]` works; 2-overlay 6s 1080p encode = 1.6s.
- RTFX naming grammar (711-file census): `<PREFIX> - [V02 ][Simple ]<Category>
  <NUM> [descr…] [directions] [Loop] [MIX] [Stch] [noCT] [noRSZ]`; noCT=484
  (untintable), noRSZ=547, Loop=88, MIX=100, Stch=82; matte polarity
  Alpha/aInv/Inv; nested buckets Transitions/{Elemental,Matte},
  Backgrounds/{Backgrounds,Foregrounds}; known typos (`MiX`, `Explositon`,
  double spaces, lowercase drift); duplicate variant numbers within a family.
  PREVIEW RULE (verified 5/5): `<same basename>.jpg`/`.mp4` under
  `1. RTFX pack/(Footage)/previews` — a SUPERSET of the prerendered set.

## 3. Target design

1. **Schema (dpl-comp, additive, stays v1)** — `Layer` gains
   `#[serde(default)]` fields: `x: f32` (0.0), `y: f32` (0.0) — CENTER-relative
   offsets in comp px; `scale: f32` (default 1.0 via `fn one()`) — 1.0 =
   aspect-fit the comp frame (full-frame RTFX native behavior); `tint:
   Option<[u8; 4]>` — RGBA multiply, `None` = untinted (array form keeps the
   no-`'#'` shape law; tint[3] is layer opacity). `validate`: scale finite
   > 0, x/y finite. Old comps load unchanged; round-trip tests.
2. **Track semantics** — `tracks[0]` = base (cut/concat, unchanged);
   `tracks[1..]` = overlay tracks (composite with transform/tint, overlaps
   allowed). Composer materializes ONE overlay track `"fx"` on first use.
3. **Export (engine-media)** — new `OverlayClip { input: OverlayInput,
   in_pt, out_pt, at, x, y, scale, tint }` with `OverlayInput::File(PathBuf) |
   PngSeq { pattern, fps, start }`; new `comp_args(base, overlays, w, h, fps,
   out)`: base = the EXISTING concat legs verbatim → `[base]` (empty base
   track → lavfi `color=c=black:s=WxH:r=fps:d=dur` leg so an FX-only comp
   renders); each overlay leg = `trim,setpts=PTS-STARTPTS+at/TB[,scale=
   {W·s}:{H·s}:force_original_aspect_ratio=decrease][,colorchannelmixer=
   rr:gg:bb[:aa]]` then chained `overlay=x=(W-w)/2+{x}:y=(H-h)/2+{y}:
   enable='between(t,at,at+span)':eof_action=pass`. `render_comp` wraps it;
   `render_stitch` stays for base-only comps (and its callers). TDD on exact
   arg vectors (the §8-proven forms).
4. **Lottie in the export** — for each Lottie layer, `export_sequence` at comp
   fps into a std-temp dir (NEVER the comp folder — folder contract), then it
   joins `comp_args` as a `PngSeq` input (base-track Lottie = a concat leg
   from its seq; overlay Lottie = an overlay leg). Removes the L2 "Lottie
   layers skipped" honesty note. Temp seqs cleaned after encode.
5. **Layered preview (composer)** — `active_layer_at` joined by
   `overlay_layers_at(t)` (all tracks[1..] layers covering `t`, track order).
   Base lanes unchanged; new `overlay_lanes: Vec<(String, OverlayLane)>`,
   `OverlayLane = Clip(ClipPlayer) | Lottie(LottiePlayer) | Still(uri)`,
   reconciled each tick (open on enter, drop on exit, 0.35s drift re-seek).
   CAPS are DYNAMIC (operator-ratified 2026-07-27: "configurable … scale
   based on resources" — 64GB/3080 boxes must not inherit laptop limits):
   probed once at composer construction via sysinfo total RAM +
   `available_parallelism` (the `dpl_engine_ae::Caps::probe` pattern,
   replicated per the small-utility philosophy): overlay clip lanes =
   `clamp(cpus/4, 2, 8)` further clamped to 3 when RAM < 24 GB; Lottie lanes
   = 2 when RAM ≥ 32 GB else 1. TOPMOST wins past the cap; status names
   skipped layers honestly. Export is NEVER capped. Paint at BOTH
   sites after the base branch, back-to-front: rect = fit_aspect(frame,
   native aspect) × scale, offset (x,y)·(frame.width/comp.w), texture via
   `painter.image(tex, rect, FULL_UV, tint_color)` — overlays LETTERBOX
   (never cover_uv); tint None → WHITE; stills via `egui::Image::tint`.
6. **Editor UX (ColdLight-gated)** — an FX lane in the timeline above the
   main lane; "add as FX overlay" action on FOOTAGE palette/library rows;
   selected-overlay inspector: scale slider, x/y nudge, tint swatch +
   clear-tint (a `no-tint`-tagged element gets a tooltip advising against
   tinting). Tooltips on every new control; touch-first sizes. Invoke the
   `coldlight-system` skill before building this step.
7. **RTFX enricher (composer, pure fn + ingest wiring)** — `rtfx.rs`:
   `rtfx_tags(name, bucket) -> Option<Vec<String>>` parsing the §2 grammar
   case-insensitively, typo-tolerant (unknown tokens stay name-only): tags =
   category (`fire`), `dir:up`…, `loop`, `mix`, `stretch`, `no-tint`,
   `no-resize`, bucket (`fx`/`transition`/`matte`/`bg`/`fg`/`sfx`), `rtfx`.
   Wired into `ingest_copy` for ALL media kinds. POSTER: probe the pack's own
   preview (`(Footage)/previews/<basename>.jpg`, ≤3 ancestor levels) and COPY
   it beside the dest as the poster (pinned trees go offline; never
   reference); fallback `write_poster`. Unit-test matrix straight from the
   census incl. typos/double-spaces/casing.
8. **Ingest index fix** — `ingest_copy` writes through `Index::open_default()`
   (the ADR-014 DB), dropping the ad-hoc `root/artifacts.db` open; the drill
   test follows (scratch content root via the existing env seam).
9. **Library visibility of tags** — minimal in-memory tag filter chips
   (derived from the loaded records' tags, top-N by count) on the browse
   elements view + FOOTAGE palette; no FTS, no schema change (`set_tags`
   stays uncalled).
10. **Harness** — `--fx-drill <clip-rel>|<fx-rel>` (or two-arg like
    `--export-guide`): builds a comp (base clip + RTFX overlay, tinted +
    scaled + offset), saves it, renders, exits — the headless proof seam.

## 4. Changes, in execution order

1. `crates/comp/src/lib.rs`: x/y/scale/tint fields + defaults + validate
   arms + round-trip/validate/back-compat tests.
2. `crates/engine-media/src/lib.rs`: `OverlayInput`/`OverlayClip`/`comp_args`/
   `render_comp` (+ lavfi empty-base leg) + exact-arg-vector tests (timed
   overlay, transform, tint incl. aa-opacity, chained ×2, png-seq input,
   eof_action=pass, enable windows, center-position expressions).
3. `crates/composer/src/lib.rs`: `overlay_layers_at` + `overlay_lanes`
   reconcile in `tick_playback` + caps + both paint sites + `ensure_fx_track`
   + `add_overlay(rec)` + render path switches to `render_comp` when overlays
   or Lottie exist (Lottie temp-seq export first) + remove/new/open lane
   clearing. Tests: overlay selection, caps, comp_args mapping.
4. Editor UX (after invoking `coldlight-system`): FX timeline lane,
   add-as-overlay action, inspector (scale/x/y/tint) with tooltips.
5. `crates/composer/src/rtfx.rs` (parser + census-driven tests) +
   `packs.rs::ingest_copy` wiring (tags for all kinds, preview-jpg poster
   probe, `open_default` fix) + drill-test adjustment.
6. Browse/palette tag chips (in-memory).
7. `--fx-drill` harness flag (app boot + wiredfront delegate) + CLAUDE.md
   flag list + `docs/STATE.md` + §8 here.

## 5. Boundaries

🚫 No keyframed transforms (static per-layer x/y/scale/tint this pass; the
keyframe pass is its own rung). 🚫 No blend modes beyond normal alpha-over.
🚫 No audio layers/SFX timeline (mp3s ingest + tag only). 🚫 No agent/chat
placement API (next rung — this pass builds its substrate). 🚫 No AE bake
automation (aep-orchestrator lane is separate, operator-driven). 🚫 ONE
overlay track in the UI (schema supports more; additive later). 🚫
`Source::Element` stays a dormant placeholder — placement uses Artifact rels.
🚫 No `#[allow]`; no re-export shims; ffmpeg + dpl-lottie-render stay
subprocesses (ADR-009). 🚫 Pinned pack trees READ-ONLY; `/content` untouched
by the build; sources never mutated. ⚠️ Temp Lottie seqs never land in the
comp folder (folder contract) — std temp, cleaned after encode. ⚠️ Preview
caps are honest (status names what's skipped), never silent.

## 6. Verification

1. `cargo green` (fmt+clippy+tests+release build) — on server1 if the laptop
   OOMs; unit suites: comp (schema back-compat: a v1 comp WITHOUT new fields
   loads with defaults; with fields round-trips identically), engine-media
   (arg vectors verbatim vs the §8-proven forms), rtfx parser (census matrix
   incl. `MiX`, double spaces, lowercase, `Stch`, matte polarity, SFX),
   ingest (open_default write-through; poster-probe hit + fallback).
2. Real-exe drill (SEQUENTIAL, never parallel): pin a scratch copy → ingest a
   small RTFX subset → `--fx-drill` comp → `render.mp4` ffprobed (h264,
   comp WxH/fps) + frame extracts at t-inside vs t-outside the overlay window
   (element visible/absent, tint applied) — the M1/M3 proof, on OUR renderer.
3. Layered preview proof shots at REAL window sizes (`--window 1280x800` and
   one more), never only maximized: base clip + tinted overlay visible in the
   viewer; FX lane + inspector visible; tooltips present.
4. Lottie: a comp with one Lottie layer renders (no honesty note anymore);
   `dpl-lottie-render.exe` staged beside the exe at close-out.
5. Ingest-bug regression: ingest on a migrated-shape root indexes into
   `.userdata/index.db` and the element appears in the FOOTAGE palette.
6. Old comps: open an existing comp.json (no new fields) — loads, plays,
   renders exactly as before (concat path when no overlays).

## 7. Commit plan

Branch `feat/l3-alpha-overlay` off main. Commits: (1) comp schema fields +
tests; (2) engine-media comp_args/render_comp + tests; (3) composer layered
preview + render switch + Lottie export wiring; (4) editor UX (ColdLight);
(5) rtfx enricher + ingest fix + tag chips; (6) harness flag + docs. PR →
gate (local hook; server1 fallback) → merge → rebuild
`target\release\devpulselabs.exe` + `dpl-lottie-render` HERE (mtime moved) →
`docs/STATE.md`.

## 8. Evidence (provenance + to be completed during execution)

Planning provenance (deep-plan protocol): §2's sweeps were Opus-drafted by
five read-only/proof agents (index API + tag consumers; Element lifecycle +
add-to-timeline; viewer/player APIs; live ffmpeg overlay proofs; RTFX 711-file
naming census). Fable spot-checks that passed before trusting: `active_layer_at`
l.626-630 + `Color32::WHITE` tint arg l.2372 read directly; `ingest_copy`
kind/`artifacts.db` behavior read directly (bug confirmed first-hand);
`m4_chained.mp4` ffprobed (h264 1920×1080@30, 180f) + `m3_compare.png`
visually confirmed (red-tinted snow, alpha edges clean) in the session
scratchpad (`ffprobe-proofs/`). ffmpeg proof commands ran against the real
pack READ-ONLY on ffmpeg 8.1.2.

Execution evidence lands here per the law: gate output, drill transcripts,
proof-shot paths, exe mtime.
