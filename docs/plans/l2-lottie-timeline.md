# L2 — native Lottie on the timeline: import · Source::Lottie · live preview

Status: PLANNED + implementing 2026-07-27 (operator: "plan and implement").
Builds on the landed L1 (`crates/lottie-render` helper + `crates/lottie`
façade, PR #44). This is the app wiring: a `.lottie`/`.json` Lottie becomes
a first-class artifact, joins the comp as a new source kind, and
PREVIEWS/PLAYS natively on the timeline via a subprocess-backed player —
the experience-gate payoff (you SEE your Lottie in the editor, no Adobe).

## 1. Context

L1 proved ThorVG renders real Lottie via the `dpl-lottie-render` subprocess
helper (frame/export/serve) behind the pure-Rust `dpl-lottie` façade
(`Player::open` + `frame_at(t)`; `export_sequence`). Nothing in the app
uses it yet. L2 wires it into the WiredFRONT editor so a Lottie layer sits
on the timeline and the composition viewer plays it as the playhead moves —
the `ClipPlayer` experience, for vector motion-graphics.

Scope law (`docs/plans/native-lottie.md`): Lottie carries shapes/keyframes/
masks/mattes/gradients/text — the vector class; effects/3D/expressions stay
the AE orchestrator's job.

## 2. Current state (ground-truthed 2026-07-27, current main #94)

- `crates/comp/src/lib.rs` — `Source` is `Artifact { rel }` | `Element { id }`
  (serde `tag="kind"`, kebab). Additive: a third variant is a clean grow.
- `crates/composer/src/import.rs::media_kind` — ext→kind:
  video/image/audio, and `aep|lottie|mogrt|cube|ffx|prfpset → "element"`.
  `.json` is UNCLASSIFIED (correctly — comp.json/guide.json/slide.json must
  not be "media"). `.lottie` currently misfiles as a generic element.
- `crates/composer/src/lib.rs`:
  - Preview model is PARALLEL Options: `preview: Option<(String, ClipPlayer)>`
    (video), `preview_still: Option<(String, String)>` (id, `file:///` uri
    for held frames / images). `tick_playback` (l.602) picks the active
    layer, then: `Source::Artifact` → `is_still`? still : ClipPlayer;
    `Source::Element` → `return` (no preview).
  - `add_clip` (l.549) appends a `Source::Artifact` layer at comp end,
    duration from index seconds / probe / fallback.
  - `handle_import` (l.1502) routes by `rec.kind`: video/image → `add_clip`;
    element → status-only; audio → status; else library status.
  - `stitch_clips` (l.925) maps `Source::Artifact` → `StitchClip`, SKIPS
    `Source::Element`. (The Render/export path.)
  - Viewer paints at TWO sites (viewer_pane ~l.2139, annotate canvas
    ~l.3273): `preview_still` → `egui::Image::new(uri)`; else `preview`
    → `player.texture(ctx)` → `ui.painter().image(tex, frame, cover_uv,…)`.
  - Deps: `serde_json` + `zip` already present (mogrt reader). NO `dpl-lottie`.
- `crates/video/src/clip.rs::ClipPlayer::texture(ctx) -> Option<TextureId>`
  — decodes at `DECODE_W×DECODE_H` (960×540), builds
  `ColorImage::from_rgba_unmultiplied`, `ctx.load_texture`/`handle.set`,
  resyncs on a seq counter. The texture-cache pattern to mirror.
- `crates/lottie/src/lib.rs` (façade, landed): `Player::open(path,w,h)`
  (spawns helper `serve`, reads `READY w h fps total`), `frame_at(t) ->
  Frame{width,height,rgba}`; `export_sequence`. `helper_path()`:
  beside-exe → `DPL_LOTTIE_RENDER` → PATH.
- `crates/lottie-render/src/main.rs` (helper): renders with
  `TVG_COLORSPACE_ABGR8888` — **alpha-PREMULTIPLIED** (per the ThorVG
  header). Fine for opaque comps (the gatin proof), WRONG for transparent
  Lottie composited over the stage. Must switch to `ABGR8888S` (straight)
  for correct egui/PNG alpha.
- Packaging: the helper builds to `target/{debug,release}/` only when built
  (`cargo build -p dpl-lottie-render`); the app's `-p devpulselabs` build
  does NOT emit it. It currently exists beside the exe from the gate's
  workspace build.

## 3. Target design

1. **`Source::Lottie { rel }`** in `dpl-comp` (additive serde variant;
   `reference()` returns `rel`; validate like Artifact — non-empty ref).
2. **Media kind**: `media_kind` maps `lottie|json → "lottie"`. Because
   `.json` is ambiguous, `import_file` CONTENT-SNIFFS a `.json`: accept only
   a Bodymovin doc (top-level object with `"fr"` and `"layers"`); reject
   others honestly (protects comp.json/guide.json/slide.json). `.lottie`
   (dotLottie zip) is always accepted. Lottie artifacts copy into
   `inbox/` (playable, colocated with comps) and index as `kind="lottie"`.
3. **`add_lottie(rec)`** — like `add_clip` but `Source::Lottie`; duration
   from the Lottie's own `op/fr` (read cheaply: parse the JSON — or the
   zipped JSON for `.lottie` — for `ip/op/fr`; fallback if unreadable).
4. **`LottiePlayer`** (new, in the composer — keeps `dpl-lottie` egui-free):
   wraps `dpl_lottie::Player` + an egui `TextureHandle` + `last_t`. Opened
   at a preview size derived from the Lottie's NATIVE aspect (read `w/h`
   from the JSON) capped to ~720px (pipe-cheap). `texture(ctx, t) ->
   Option<TextureId>`: when `t` changed (or first), `frame_at(t)` → the
   RGBA (straight alpha) → `ColorImage::from_rgba_unmultiplied` →
   set/load texture; return id. Stores its render `(w,h)` for aspect fit.
5. **Preview wiring**: add `preview_lottie: Option<(String, LottiePlayer)>`
   (parallel Option, mirrors `preview_still`). `tick_playback` gains a
   `Source::Lottie` arm: compute `want = in_pt + (time - start)`; open the
   player on layer switch; `texture(ctx, want)`; clear `preview`/
   `preview_still`. Both viewer sites gain a `preview_lottie` branch:
   paint its texture with an aspect-correct fit (using the player's render
   size, not a hardcoded 16:9). Playing simply advances `time`; the Lottie
   re-renders per frame (cheap — vector raster at ≤720px).
6. **Helper straight-alpha fix**: `TVG_COLORSPACE_ABGR8888S`. Verify the
   gatin proof still renders; transparent Lottie now composites correctly.
7. **Export honesty (L2 boundary)**: `stitch_clips` still skips
   `Source::Lottie` (export lands in L3). The Render action, when the comp
   contains Lottie layers, appends an honest note ("N Lottie layer(s)
   preview live; timeline export lands next"). No silent wrong render.
8. **Packaging**: build the helper beside the exe. Update the release build
   (xtask/`cargo green` exe step + CLAUDE.md exe command) to
   `-p devpulselabs -p dpl-lottie-render` so `helper_path()` (beside-exe)
   always resolves. A `--lottie-import <file>` harness seam (mirrors
   `--import-drill`) drives the import+add path headlessly for the drill.

## 4. Changes, in execution order

1. `crates/comp/src/lib.rs`: `Source::Lottie { rel }` + validate arm +
   test (round-trip, validate clean).
2. `crates/lottie/src/lib.rs`: `pub fn lottie_meta(path) -> Option<Meta{w,h,
   in_point,out_point,fps}>` (parse `.json` / unzip `.lottie` for the
   top-level fields; no ThorVG) + tests. (Façade gains `serde_json`+`zip`.)
3. `crates/lottie-render/src/main.rs`: `ABGR8888` → `ABGR8888S`.
4. `crates/composer/Cargo.toml`: `dpl-lottie.workspace = true`.
5. `crates/composer/src/import.rs`: `media_kind` `lottie|json→"lottie"`;
   `import_file` bodymovin-sniff for `.json`; route lottie to `inbox/`.
   Tests: matrix, sniff accept/reject.
6. `crates/composer/src/lib.rs`: `LottiePlayer` type; `preview_lottie`
   field; `add_lottie`; `tick_playback` + `remove` + `new_comp` Lottie
   arms; both viewer paint sites; `handle_import` lottie route; the Render
   honesty note; `--lottie-import` seam + `import_lottie(path)`. Tests:
   add_lottie stitches + validates; media feeds route.
7. `crates/app/src/main.rs`: `--lottie-import` boot flag → hub delegate.
8. `crates/hubs/wiredfront/src/lib.rs`: `import_lottie` delegate.
9. Build wiring (xtask + CLAUDE.md); `docs/STATE.md` + §8 here.

## 5. Boundaries

🚫 Export/render of Lottie layers (frame-seq → ffmpeg) = L3, NOT here
(stitch skips it, honestly noted). 🚫 Slot recolor/retext = L3. 🚫 No layer
transform/size model yet — a Lottie layer fills the frame at its native
aspect (letterboxed), like a full-frame clip. 🚫 `.aep`/mogrt routing
unchanged (still elements). 🚫 `dpl-lottie` stays egui-free (LottiePlayer's
texture layer lives in the composer). 🚫 The helper stays the ONE FFI leaf.
⚠️ `.json` ambiguity handled by the sniff — never treat internal
comp/guide/slide JSON as media.

## 6. Verification

1. `cargo test` (comp, lottie, composer) + `cargo green` ✔.
2. Unit: Source::Lottie round-trip/validate; `lottie_meta` on a real
   bodymovin fixture; media_kind + sniff (accept bodymovin .json, reject a
   plain `{}`); `add_lottie` stitches at comp end + validates.
3. Real-exe drill (`--lottie-import <gatin.json>` on a scratch root): the
   Lottie imports, lands as a `Source::Lottie` layer, and the composition
   viewer shows it — scrub the playhead, the frame changes (native render,
   no ffmpeg/AE). Proof shots at real window sizes.
4. Alpha: a transparent Lottie composites cleanly over the stage (straight
   alpha), not fringed.

## 7. Commit plan

Branch `feat/l2-lottie-timeline` off main. Commits: (1) comp Source +
façade lottie_meta + helper straight-alpha; (2) composer wiring + import +
LottiePlayer + app seam; (3) build wiring + docs. PR → squash → exe rebuild
(with the helper) → STATE.

## 8. Evidence

Implemented 2026-07-27 on `feat/l2-lottie-timeline`:
- `Source::Lottie` + comp test (round-trip/validate); façade `lottie_meta`
  + `is_bodymovin_json` + tests; helper straight-alpha (ABGR8888S);
  composer `LottiePlayer` + `preview_lottie` lane + `add_lottie` +
  tick_playback/viewer/import/stitch/new_comp/remove/open_comp arms +
  Render honesty note + `--lottie-import` seam; app flag + hub delegate;
  CLAUDE.md exe command emits the helper.
- **`cargo check --workspace --exclude xtask` ✔ (8m26s)** — the whole
  workspace compiles with the new source kind + all wiring; nothing
  cross-crate broke. `dpl-comp` (9) + `dpl-lottie` (4) tests PASS.
- BLOCKED on this 16 GB laptop: the composer TEST binary + `cargo green`
  OOM-kill during the build (the egui/vello/fonts/turso stack exceeds
  memory). The real-exe `--lottie-import` drill, the full gate, and proof
  shots are DEFERRED to the W server (128 GB/88-core, confirmed up 2026-07-27
  — ping + SSH:22 open). Verify + merge from there.
