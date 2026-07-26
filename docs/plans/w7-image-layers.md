# W7 — IMAGE LAYERS: stills are first-class timeline citizens

Operator (2026-07-25, after W6): "it is kind of essential for all of
this to matter please plan accordingly." Every image path that today
answers "images join the timeline with the image-layers pass" now just
DOES it.

## 1. Context

Videos are full citizens of the comp (timeline, preview, render).
Images are second-class: FOOTAGE taps, Library double-clicks, Explorer
drops and Browse picks all deflect them to the library with an honest
status. For a content tool whose material is AI stills + captures,
that guts the workflow. The guide-video renderer (W5) already proved
the ffmpeg still pattern (`-loop 1 -t N` + scale/pad/fps/yuv420p leg).

## 2. Current state (ground-truthed on main @ 39ae7a4, this session)

- `crates/comp/src/lib.rs` (read IN FULL): `Layer { source, start,
  in_pt, out_pt }` — NOTHING image-specific needed; an image layer is
  `in_pt=0, out_pt=<seconds shown>`; `validate` passes it; schema v1
  untouched.
- `crates/engine-media/src/lib.rs`: `StitchClip { path, in_pt, out_pt }`
  + `stitch_args` (per-input `trim=start:end,setpts,fps,scale,pad,
  setsar` legs, concat); `IMAGE_EXTS = [png,jpg,jpeg,webp]` (private);
  `guide_video_args` is the proven still leg.
- `StitchClip` consumers (grep): `composer::stitch_clips` (lib.rs:823)
  and `seam` (lib.rs:244) — a struct change touches exactly these two.
- `crates/composer/src/lib.rs`: `add_clip` probes duration (fallback
  6s); image deflections live in `Act::Add` handling ("images join the
  timeline with the image-layers pass"), `add_from_library` (W6),
  `handle_import`, and the FOOTAGE row tip. Preview =
  `Option<(String, ClipPlayer)>` driven by `tick_playback`; the viewer
  pane and the Annotate canvas both read it.

## 3. Target design

1. **engine-media**: `StitchClip` gains `pub still: bool`. In
   `stitch_args`, a still input becomes `-loop 1 -t {span:.3} -i p`
   and its filter leg drops trim/setpts (`fps, scale, pad, setsar,
   format=yuv420p`); ALL legs (video too) gain `format=yuv420p` so
   mixed still+video concat never fights pixel formats. NEW
   `pub fn is_still(path) -> bool` over IMAGE_EXTS.
2. **seam**: construct `StitchClip` with `still: is_still(&path)` —
   agents get image layers over HTTP for free.
3. **composer**: `DEFAULT_IMAGE_SECS: f32 = 5.0`. `add_clip` takes
   images: `out_pt = 5.0`, no probe. Every deflection site now
   appends: `Act::Add`, `add_from_library`, `handle_import`. Preview
   grows a still lane: `preview_still: Option<(String, String)>`
   (layer id, file:/// uri) set by `tick_playback` when the active
   layer is a still (video ClipPlayer lane untouched); the viewer pane
   and Annotate canvas paint it first. `stitch_clips` maps
   `still: is_still(...)`. Trim/move/undo/autosave ride unchanged —
   an image layer is just a layer.
4. Copy sweep: every "image-layers pass" string dies (grep-verified
   zero after the pass); FOOTAGE tip → "Add this to the timeline".
5. Friction table row 15: "Images are timeline layers" ✅ W7; §4
   media-bin blurb updated.

Scale story: any future source kind that renders as a held frame
(element posters, generated stills) rides `still: true`.

## 4. Changes, in execution order

1. `crates/engine-media/src/lib.rs` — StitchClip.still + stitch_args
   still legs + format=yuv420p everywhere + `is_still` + tests
   (still leg has loop/-t and no trim; mixed concat n counts; video
   legs keep trim).
2. `crates/seam/src/lib.rs` — the one construction site gains
   `still: dpl_engine_media::is_still(&path)`.
3. `crates/composer/src/lib.rs` — DEFAULT_IMAGE_SECS; add_clip image
   branch; deflection sites append; preview_still lane (tick_playback,
   viewer, annotate, remove/new/open clears); stitch_clips maps still;
   copy sweep; tests (image add → 5s valid layer + undo; mixed
   stitch_clips maps flags; add_from_library image → timeline now).
4. `docs/plans/wiredfront-experience.md` — row 15 + §4 blurb.
5. `docs/STATE.md` — W7 entry (at PR time).

## 5. Boundaries

🚫 comp schema (v1 untouched — no image-specific fields). 🚫 Ken Burns
/ transforms / effects on stills — later passes. 🚫 ffmpeg subprocess
law. 🚫 Elements (`Source::Element`) still render nothing — the AE
orchestrator lane owns them. ⚠️ Ask-first: changing DEFAULT_IMAGE_SECS
from 5.0.

## 6. Verification

1. `cargo test -p dpl-engine-media -p dpl-composer --lib -p dpl-seam`
   green; `cargo green` exit 0; `grep -r "image-layers pass" crates`
   returns NOTHING.
2. REAL-EXE drill (scratch root, `--content-root`): `--import-drill`
   a folder holding one png + one mp4 → BOTH land on the timeline;
   `--save-comp` names it; seam-less render via the Editor path is
   covered by: POST the comp through the drill? No — simplest real
   proof: after import, screenshot the editor (image layer bar visible,
   preview showing the STILL at playhead 0 when image is first) then
   render via `--render-comp`? (no such flag — render through the
   existing R1w seam: POST /command render with the seam ON) →
   ffprobe duration == 5.0 + <video secs> ± 0.1.
3. Proof shots docs/proof/w7-image-timeline-{1100x700,1600x1000}.png.

## 7. Commit plan

Branch `feat/w7-image-layers` off main. One commit; PR → squash-merge
→ exe rebuild → STATE. Coordinate: the parallel AE session touches
engine-ae only (phases 2–5); composer/engine-media are clear today.

## 8. Evidence (2026-07-25)

1. Tests: engine-media 17/17 (still leg loops `-loop 1 -t 5.000`, no
   trim; every leg `format=yuv420p`; `is_still` case-insensitive);
   composer 38/38 (image via `add_from_library` -> a real 5.0s layer
   appended after the video, valid comp, still flags mapped); seam
   compiles with `still: is_still(&path)`. `cargo green ✔`.
   Copy sweep: `grep -r "image-layers pass" crates` -> nothing.
2. REAL-EXE drill (scratch root, `--content-root`): `--import-drill`
   of a-still.png + b-clip.mp4 -> comp.json reads
   `[(inbox/a-still.png, start 0.0, 5.0s), (inbox/b-clip.mp4,
   start 5.0, 3.0s)]` — BOTH on the timeline through the real import
   path. Editor shot: the still's bar (5.0s) + the clip at 0:05, comp
   0:00/0:08, and the PREVIEW WELL SHOWS THE STILL —
   docs/proof/w7-image-timeline-{1100x700,1600x1000}.png (the 1600
   shot also proves reopen-last restored the comp).
3. Render e2e over the AGENT SEAM (the exe live, POST /command
   render): ffprobe render.mp4 = **8.000000s exactly** (5s held still
   + 3s video) at 1920x1080 — the mixed concat pixel-format law holds.
