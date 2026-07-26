# DG1 — the mixed-media guide: replay-segment steps, ordered

Operator ratification (2026-07-25): *"demonstrate the ability to show
where to click so that will be stills and then a mix of session replays
when appropriate — so when I check out the .guide it should have them
ordered."* Charter: `docs/plans/demo-guide-pipeline.md` (gap G-1/G-2/
G-4). Merge mode per operator: squash.

## §1 Context

A `.guide` today is ordered STILLS plus at most one whole-session
replay attached at the bottom. The customer deliverable needs replay
CLIPS as first-class steps interleaved with stills in ONE ordered
sequence, cut from the session recording, playing inside the same
self-contained player.

## §2 Current state (all read in full this session)

- `crates/guide/src/lib.rs` (753 lines) — `Step` (recorder shape +
  caption; no media kind), `SessionRef`/`Manifest`, `export_guide`
  (copy+renumber stills → light `session.mp4` re-encode → optional
  `replay.gif`/`guide-video.mp4` → `write_html` → manifest),
  `write_html` (stills as data URIs, one `<img>` stage + prev/next/
  toggle/swipe, replay `<video>` section at the bottom), `frame_ok`
  jail, hand-rolled `base64`, tests avoiding ffmpeg legs
  (`make_guide_video:false`, pre-encoded stand-ins).
- `crates/engine-media/src/lib.rs:372-408` — `replay_args`/
  `render_replay`: the "incredibly light" law (≤720p/15fps/crf30/
  faststart/-an). `:453-533` `guide_video_args`/`render_guide_video`
  (stills held N secs + optional session, concat) — callers: ONLY
  `guide/src/lib.rs:486` + own tests.
- `crates/engine-steps/src/edit.rs:25-53,73-135` — `GuideStepEdit` is a
  FIXED shape (would silently drop new fields) and `frame_ok` requires
  a non-empty `file` (would reject clip steps); `update_guide` rebuilds
  the manifest from `json!` and prunes orphaned frames by the
  `file`/`resultFile` reference set.

## §3 Target design

- **Schema (backward-compatible; recorder guides parse unchanged):**
  `Step` gains `kind: String` (serde default `""`; `"replay"` marks a
  clip step), `clip: Option<String>` (bundled clip filename, set by
  export), `clip_in: Option<f32>` / `clip_out: Option<f32>` (cut range
  in the source session, seconds; serde `clipIn`/`clipOut`). A SOURCE
  replay step: `kind:"replay"`, `clipIn/clipOut`, `file:""`. An
  EXPORTED replay step additionally: `clip:"step-00N.mp4"`.
- **Cutting (engine-media):** `clip_cut_args(src, in_s, dur_s, out)` —
  `-ss <in> -i src -t <dur>` + the replay-light chain verbatim;
  `render_clip_cut(src, in_s, out_s, out)` refuses `out_s <= in_s` and
  a missing source. Unit tests assert args verbatim (house pattern).
- **Export:** for each replay step: require the session video (honest
  error otherwise), cut `step-{order:03}.mp4` from the ORIGINAL
  session recording, set `clip`; stills path skipped. `guide-video`
  legs interleave in step order: `guide_video_args` EVOLVES to take
  `&[VideoLeg]` (`enum VideoLeg { Still { path, secs }, Clip { path } }`)
  — same scale/pad/concat graph; the whole-session tail leg stays via
  the caller appending it. Both existing call/test sites updated in
  place (only callers are the guide crate + engine-media tests — no
  shim, per the move law).
- **Player:** the stage gains a `<video id="stepv" controls playsinline>`
  beside `#shot`; `steps_js` entries gain `vid` (data URI when
  `embed_video`, sidecar name otherwise — `frame_ok`-jailed either
  way). `show()` branches: clip step → video visible, img/dot/toggle
  hidden; still → today's behavior; navigation PAUSES any playing
  clip. Keyboard/swipe/counter unchanged — one ordered sequence.
- **Reviewer round-trip:** `GuideStepEdit` gains the same four fields
  (optional, `skip_serializing_if` none/empty); validation branches:
  replay steps validate `clip` when present and tolerate empty `file`;
  the orphan-prune reference set includes `clip`.
- Whole-session attach (`SessionRef`) stays supported and orthogonal.

## §4 Changes, in execution order

1. `crates/engine-media/src/lib.rs`: add `clip_cut_args` +
   `render_clip_cut` + `VideoLeg`; evolve `guide_video_args`/
   `render_guide_video` to legs; update its two tests + add cut-args
   and interleave-order tests.
2. `crates/guide/src/lib.rs`: `Step` fields + `is_replay()`;
   `export_guide` replay branch (cut, clip name, still-skip, legs
   assembly); `write_html` mixed player per §3; tests: round-trip with
   a replay step, mixed html (stand-in clip bytes: data URI count +
   sidecar name + still behavior intact), honest refusals (replay step
   with no session; jailed clip name).
3. `crates/engine-steps/src/edit.rs`: `GuideStepEdit` fields +
   branched validation + prune set; round-trip test proving clip
   fields survive a caption edit.
4. `crates/guide/tests/mixed_export_drill.rs` (ignored): real ffmpeg
   generates a 4 s `testsrc` session → export a fixture guide of
   2 stills + 1 replay step (1.0→2.5 s) → assert `step-002.mp4`
   exists/manifest/html; then **dpl-engine-browser** (dev-dep) opens
   the exported `guide.html` from `file://`, asserts `STEPS.length`,
   steps to the clip, asserts the video element is visible with a
   non-empty src — the G1 headless-browser drill, dogfooding DEMO-B.
5. `crates/guide/Cargo.toml`: `[dev-dependencies]
   dpl-engine-browser.workspace = true`.
6. Docs: charter DG1 row → landed; STATE tip; this plan's §8.

## §5 Boundaries

- 🚫 Root `Cargo.toml`/`Cargo.lock`: untouched (lottie lines parked;
  no new deps — dev-dep uses the existing workspace entry).
- 🚫 Recorder (`engine-steps` capture side) and S1/S2: unchanged —
  replay steps are authored at review/export time, never faked by the
  recorder.
- 🚫 `.guide` self-containment law: zero external requests, both modes.
- ⚠️ Authoring UI (reviewer "insert clip" affordance) — the
  experience-gated pass after this; DG1 is the engine + player.
- Out of scope: DG2 composition, redaction tooling (G-6), RING-2.

## §6 Verification

1. `cargo test -p dpl-engine-media -p dpl-guide -p dpl-engine-steps`
   green.
2. `cargo test -p dpl-guide --test mixed_export_drill -- --ignored
   --nocapture` — real ffmpeg cut + real headless-browser walkthrough
   of the exported player.
3. `cargo green` exit 0; exe mtime moves.
4. Export the drill's output folder is CLOSED: html contains zero
   `http` refs (existing law, asserted in tests).

## §7 Commit plan

Branch `feat/dg1-mixed-guide` (off a502ffe). One commit: the four
crates + docs. Message: `DG1 — mixed-media guide: replay-segment
steps, ordered (stills + clips in one player)`. Push (gate hook),
PR, operator merges (squash, per instruction).

## §8 Evidence

- `crates/guide/src/lib.rs` — read IN FULL (1-753).
- `crates/engine-media/src/lib.rs` — read in full earlier this
  session; guide-video callers verified by grep (guide crate + own
  tests only).
- `crates/engine-steps/src/edit.rs:20-135` — fixed-shape drop risk +
  empty-file rejection verified by read.
- Fixture/test idioms: `guide` tests avoid ffmpeg (stand-ins);
  `composer/tests/import_drill.rs` ignored-drill pattern.
