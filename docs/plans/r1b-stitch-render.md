# R1b — the stitch render (concat) + Render button

Pass R1b of the WiredFRONT editor (ledger #114 / R1). Parent: R1a
(`r1a-comp-document.md`, landed — `dpl-comp` + `dpl-composer`). ADR:
`r1-editor-adr-proposal.md` §3 (preview vs render, two tiers) + ADR-009
(ffmpeg subprocess). Ground-truthed in-session 2026-07-24.

R1b makes the clip-list **produce a video**. The core (R1b.1, this build):
the ffmpeg CONCAT stitch render — every layer trimmed + scaled + joined
into one H.264 mp4 in the inbox, driven from a **Render** button in the
Composer, off the UI thread. This is the operator's payoff:
*"stitch a series of 8–15s clips."*

**Split (to keep one reviewable diff):** R1b.1 = concat stitch render +
Render button (THIS pass). R1b.2 (next) = xfade transitions + composited
timeline preview playback + `egui_tiles` docking (RS4). R1 §3's registry
tier (arbitrary effects) stays R1d. Recorded so the split is explicit, not
silent.

---

## 1. Context

R1a made the comp a validated document and gave it an editor, but the
Composer can only *arrange* clips — it cannot yet render one. R1b closes
that: the same `Composition` the doctor validates becomes real pixels via
ffmpeg (a subprocess, ADR-009 — never a crate dep). The render is the
FINAL tier (R1 §3): ffmpeg concat/xfade, not the GPU preview. R1b.1 does
the straight concat (cuts); xfade rides R1b.2. Audio is dropped (`-an`),
matching the app's existing `video_args` convention — an audio pass is
later.

## 2. Current state (reads this session)

**`crates/engine-media/src/lib.rs` — THE ffmpeg-encode home.** Single-input
transcode args + a subprocess runner, args verbatim (ADR-009):

```rust
pub fn video_args(src: &Path, out: &Path) -> Vec<String> {
    vec!["-y","-v","error","-i",src…, "-vf",
         format!("scale=-2:'min({VIDEO_MAX_HEIGHT},ih)'"),
         "-c:v","libx264","-preset","slow","-crf",VIDEO_CRF.to_string(),
         "-movflags","+faststart","-an", out…]
}
fn run_ffmpeg(args: Vec<String>) -> Result<(), String> {
    let status = Command::new(ffmpeg_program()).args(&args).status()…;
    if status.success() { Ok(()) } else { Err(format!("ffmpeg exited {status}")) }
}
```
`ffmpeg_program()` returns the injected managed binary else bare `ffmpeg`
from PATH (dev). `VIDEO_MAX_HEIGHT`, `VIDEO_CRF` are the crate's authored
constants. Tests assert the args are "the old scripts verbatim"
(`ffmpeg_args_are_the_old_scripts_verbatim`, line 479) — R1b's stitch args
get the same golden-string test.

**`crates/comp/src/lib.rs` (R1a).** `Composition { fps, w, h, tracks }`,
`Layer { source: Source, start, in_pt, out_pt }`, `Layer::span()`,
`Composition::duration()`. `Source::Artifact { rel }` /
`Source::Element { id }`, `Source::reference()`.

**`crates/composer/src/lib.rs` (R1a).** `Composer` owns the comp + the
`content_root()` / `resolve(rel)` helpers + the deferred-`Act` pattern +
the toolbar (New/Save chips). Depends on dpl-comp, dpl-video, dpl-index,
dpl-engine-capture, dpl-coldlight, egui-pilot-ai. NOT yet on
dpl-engine-media.

## 3. Target design

**engine-media gains a decoupled stitch API — primitive inputs, NO
`dpl-comp` dep (no layering inversion).**

```rust
/// One trimmed source clip in a stitch.
pub struct StitchClip { pub path: PathBuf, pub in_pt: f32, pub out_pt: f32 }

/// The concat filtergraph args (pure, golden-tested): each input trimmed
/// to [in,out], reset PTS, scaled+padded to WxH@fps, then concat=n=N.
pub fn stitch_args(clips: &[StitchClip], w: u32, h: u32, fps: u32, out: &Path) -> Vec<String>;

/// Run the stitch (subprocess). Errors honestly (no clips, ffmpeg absent).
pub fn render_stitch(clips: &[StitchClip], w: u32, h: u32, fps: u32, out: &Path) -> Result<(), String>;
```

Filtergraph (N inputs): for each i,
`[i:v]trim=start={in}:end={out},setpts=PTS-STARTPTS,fps={fps},
scale={w}:{h}:force_original_aspect_ratio=decrease,
pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1[v{i}]`; then
`[v0][v1]…[v{N-1}]concat=n={N}:v=1:a=0[out]`; `-map [out] -c:v libx264
-preset slow -crf {VIDEO_CRF} -movflags +faststart -an`. `-i {path}` per
input before `-filter_complex`. Pad (not crop) so no source content is
lost across mixed aspect ratios — the comp's WxH is the frame. `scale=-2`
is replaced by explicit WxH because concat requires uniform frames.

**Composer gains Render (off-thread).** A `Render` chip in the toolbar
(disabled/greyed when the comp is empty). On click: map the main track's
layers whose source is an `Artifact` → `StitchClip { resolve(rel), in, out }`
and spawn a thread running `render_stitch` to
`<comp-dir-or-fresh-inbox>/render.mp4`; a shared `RenderJob {
Arc<Mutex<RenderState>> }` (Idle | Running | Done(path) | Failed(msg))
the toolbar polls each frame (request_repaint while Running), showing an
honest status pill. Element-source layers are skipped in R1b (no file
yet) with a named status. Not a comp doctor concern — render is a runtime
action.

**Scale story:** xfade (R1b.2) swaps `concat` for a chained `xfade` graph —
same input prep, different join; the `stitch_args` signature grows an
enum `Join { Cut, Xfade(f32) }`. The effect registry (R1d) adds per-layer
filter refs before the join. One render path, additive.

## 4. Changes, in execution order

1. **`crates/engine-media/src/lib.rs`** — add `StitchClip`, `stitch_args`
   (pure), `render_stitch`; authored constants reused. Add `#[cfg(test)]`
   golden-string tests (single + multi clip; trims + concat=n present;
   `-an` + faststart; empty → the args builder is only called with ≥1).
2. **Verify stage 1:** `cargo test -p dpl-engine-media`; `cargo clippy -p
   dpl-engine-media -- -D warnings`.
3. **`crates/composer/Cargo.toml`** — add `dpl-engine-media`.
4. **`crates/composer/src/lib.rs`** — `RenderJob`/`RenderState`, the
   Render chip (tipped, pilot id, disabled-when-empty), the map + spawn,
   the poll + status pill. Unit-test the comp→StitchClip mapping (order,
   trims, element-skip).
5. **Verify stage 2 + gate:** `cargo green`.
6. **Exe proof:** synthesize two testsrc clips into the inbox
   (ffmpeg lavfi), open the Composer, add both, Render, and confirm
   `render.mp4` exists + probes to the summed duration; screenshot the
   Composer showing the "Rendered" status pill + a populated timeline at
   `--window 1100x700`.

## 5. Boundaries

- 🚫 Never touch `crates/editor` (Workbench) or the comp DOCTOR (render is
  runtime, not validation). 🚫 ffmpeg stays a subprocess (ADR-009) — no
  crate dep. 🚫 `/content` is operator data — render writes only under
  `inbox/`.
- ⚠️ Flagged: R1b.1/R1b.2 split (xfade + composited preview + egui_tiles
  deferred to R1b.2); `-an` (no audio) for now; pad-not-crop framing;
  render output name `render.mp4`. Redline at review.
- Out of scope: xfade, composited preview, egui_tiles, audio, the effect
  registry, generation, matting, keep-into-episode.

## 6. Verification

1. `cargo test -p dpl-engine-media` → stitch-args golden tests green.
2. `cargo test -p dpl-composer` → the mapping test green (order + trims +
   element-skip).
3. `cargo green` → fmt + clippy + tests + release.
4. Exe end-to-end: two synthesized `testsrc` clips → add both → Render →
   `inbox/comp-N/render.mp4` EXISTS and `ffmpeg -i` reports a duration ≈
   the sum of the two spans (proves the concat ran). Screenshot at
   1100×700 shows the timeline populated + the "Rendered" pill; footer
   visible (fit guard).
5. Honest-failure check: Render with zero playable clips → a named status,
   no panic, no empty file.

## 7. Commit plan

Branch `feat/r1b-stitch-render` off `feat/r1a-comp-document` (stacked —
R1b builds on R1a). One PR based on the R1a branch. `git add` the
engine-media + composer changes, this plan, the proof PNG. Message:
`feat(wiredfront): R1b.1 — ffmpeg concat stitch render + Render button`.

## 8. Evidence (executed 2026-07-24)

- [x] `cargo test -p dpl-engine-media` → stitch tests green
  (`stitch_args_trims_scales_pads_and_concats`, `a_single_clip_still_concats_cleanly`,
  `render_refuses_an_empty_stitch`).
- [x] `cargo test -p dpl-composer` → **5 passed**, incl.
  `stitch_clips_map_in_order_skipping_elements` (order + trims + element-skip).
- [x] `cargo green` → **green ✔** (exe rebuilt; first run caught an fmt-only
  miss, fixed with `cargo fmt`, re-gated clean).
- [x] **End-to-end render proof** — synthesized two MISMATCHED clips
  (640×480@30 + 800×600@24) and ran the EXACT filtergraph `stitch_args`
  generates → `render.mp4` = **1280×720@30 H.264, Duration 00:00:05.00**
  (= 2s + 3s). Proves trim + fps-normalize + scale/pad + concat across
  mixed inputs. Artifact: `docs/proof/r1b-stitch-render-demo.mp4`.
- [x] `docs/proof/r1b-composer-1100x700.png` — no regression; the Render
  chip is correctly hidden on an empty timeline; footer visible.
- **Deviations:** none in scope. As-planned split (R1b.2 = xfade +
  composited preview + egui_tiles). No exe screenshot of the *populated*
  render flow: the harness has no flag to add clips + click Render in a
  single-frame shot, so the render is proved by the end-to-end ffmpeg run +
  golden unit tests instead (recorded, not skipped silently). A
  populated-timeline screenshot arrives once R1c indexes real clips or a
  composer drill lands.
