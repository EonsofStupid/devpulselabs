# Build-out plan — the visible app: backdrop, icons, media optimization, overlay

Directive (operator, 2026-07-22): *"you need to stop and build my actual app so
i can see my optimization of avif, webp, the rest the background animations
properly implemented my statusbar icons, the overlay"* — plan → execute per
pass until the exe is testable end to end.

Ground truth: three full-repo sweeps of `C:\localdev\devpulse labs`
(read-only salvage) this session — the media-optimization pipeline, the shell
icon inventory, and the backdrop render chain — plus direct reads of every
new-repo file this plan touches. Siblings: port-pixvix.md (PV phases),
port-slides.md.

## 1. Context

Four things the operator must SEE on the real exe:

1. **The AVIF/WebP optimization.** Old repo `scripts/optimize-media.mjs`
   (sharp + ffmpeg): every content image → AVIF q60 + WebP q80 + JPEG q82
   (mozjpeg), resized ≤2560 w with EXIF-orientation bake, idempotent
   freshness skip; alpha cutouts → AVIF/WebP/PNG (never JPEG); anim videos →
   animated WebP (ffmpeg libwebp, 24 fps, q70, loop) + poster. Runtime Steps
   WebP q82 is ALREADY ported (engine-capture). The build-time pipeline is not.
2. **The background animations, properly.** The old mainpage backdrop is NOT
   procedural: `BackdropStage` = static teal aurora gradient (135° in oklch:
   0.32/0.0798/190 → 0.72/0.145/190 @25% → 0.84/0.116/190 @55% →
   0.95/0.0435/190) + a looping mp4 (`object-cover`, muted) + a scrim
   (surface oklch(0.12 0.01 220) at 70% alpha over video, 35% aurora-only).
   **"My two loops" = `public/backdrops/dpl.mp4` (1,356,318 B) and
   `clyffy-chases-the-dragon.mp4` (789,677 B)** + posters — verified on disk.
   This RESOLVES OQ-PV1: the files exist; they get copied (never moved) and
   played. The authored Drift/Pulse ambiences stay as extra switcher options.
3. **The statusbar icons.** Old footer: brand `●` + ColdLight label; **Snip
   launcher** (Camera glyph → opens the PixVix palette); Dock grip; center
   viewport readout `{w} × {h} · dpr {d}`; kbd legend `[ ] \`; the oversized
   **Bot agent launcher** with tab-count badge. Rail: Home/Hammer/LayoutGrid/
   Clapperboard/Radio/Flame/GraduationCap (lucide). New shell footer currently
   has a reserved slot and text-only glyphs — no real icons.
4. **The overlay.** PV2 from port-pixvix.md: the PixVix palette as an
   always-on-top viewport — tabs Img·Video·Session·Steps (icons Camera/Video/
   SquarePlay/Footprints), region select on freeze-frame + loupe, REC pill
   (CircleDot pulse + count, Mark=Footprints, Stop=Square), global hotkey
   Ctrl+Shift+9, footer launcher.

## 2. Current state (all read this session)

- `Cargo.toml` (workspace): `image` PNG-only; no webp/jpeg decode → the app
  cannot display the very formats the pipeline emits.
- `crates/coldlight/src/lib.rs`: Tone/pool/surface/ink/space + motion runtime;
  no icon font, no gradient-mesh helper.
- `crates/hub/src/lib.rs`: `HubSpec.icon: &'static str` — doc comment already
  promises "until the sprite atlas lands (P1)"; a font glyph IS a str, so the
  contract holds unchanged.
- `crates/shell/src/lib.rs`: footer = route readout + "fabric · live" + a
  comment reserving the launcher slot; rail items paint `spec.icon` as text.
- `crates/hubs/home/src/lib.rs`: subroutes DRIFT/PULSE → `media_backdrop`
  ambiences + hero text. No aurora, no video, no scrim.
- `crates/slides/src/blocks/media.rs`: `Media::Video` = honest placeholder
  plate ("engine pass (ADR-009)").
- `crates/app/src/main.rs`: harness (`--screenshot/--route/--focus/
  --shot-frame`), `install_image_loaders`, hub registry.
- `crates/engine-capture`: xcap stills/recorder + ffmpeg subprocess via
  `set_ffmpeg_path` OnceLock; PNG stills, WebP steps; inbox never-clobber.
- `crates/engine-steps`: recorder + rdev hook + EventSink.

## 3. Target design

Four passes, each: plan section → build → tests → exe proof → commit.

- **B1 `feat(backdrop)`** — the mainpage truth: assets copied, a video-loop
  player (ffmpeg SUBPROCESS → egui texture, ADR-009), aurora + loop + scrim
  in Home, `Media::Video` made real.
- **B2 `feat(icons)`** — Phosphor icon font into ColdLight (`cl::icon`),
  rail + footer rebuilt to the old inventory (Snip launcher, viewport
  readout, kbd legend, agent bot), full-bleed toggle.
- **B3 `feat(media)`** — `crates/engine-media`: the optimize pipeline in pure
  Rust (+ ffmpeg subprocess for anim/video), `media-cli` bin, and webp/jpeg
  DECODE features so the app renders its own outputs.
- **B4 `feat(pixvix)`** — PV2: the overlay viewport (transparency spike →
  fallback borderless), palette tabs wired to the real engines, region
  select + loupe, REC pill, Ctrl+Shift+9, footer launcher.

Scale story: a new backdrop loop = drop an mp4 + one catalog line; a new icon
= one const; a new optimized format = one encoder arm; overlay tabs consume
engines — no engine logic in UI.

## 4. Changes, in execution order

### B1 — backdrop (`feat(backdrop)`)

1. COPY (never move) old `public/backdrops/{dpl.mp4,dpl-poster.jpg,
   clyffy-chases-the-dragon.mp4,clyffy-chases-the-dragon-poster.jpg}` →
   `assets/backdrops/`.
2. CREATE `crates/video` (`dpl-video`): `Player` — spawn
   `ffmpeg -v error -stream_loop -1 -i <path> -f rawvideo -pix_fmt rgba
   -vf scale=<w>:<h> -` (decode size ≈ 960×540 — backdrop sits under a 70%
   scrim; full-res is waste), reader thread fills a triple-buffered frame
   slot, `Player::texture(ctx) -> Option<TextureHandle>` uploads the newest
   frame per tick at the stream's fps. ffmpeg resolution: explicit path via
   `set_ffmpeg_path` else `ffmpeg` on PATH; missing ffmpeg → `Player::poster`
   mode (image fallback), never a crash. Public: `loop_backdrop(ui, rect,
   path, poster)` cover-fits (center-crop UV) + repaint request.
   Unit tests: cover-fit UV math; args builder; poster fallback selection.
3. EDIT `crates/coldlight/src/lib.rs`: `pub fn aurora(painter, rect)` — the
   135° 4-stop oklch gradient as a vertex-tinted mesh grid (stops converted
   through the SAME oklch→srgb core build.rs already generates for ramps;
   consts for the four stops), plus `pub const SCRIM_VIDEO: Color32` (surface
   base at α179) and `SCRIM_BARE: Color32` (α89).
4. EDIT `crates/slides/src/blocks/media.rs`: `Media::Video(path)` arm →
   `dpl_video::loop_backdrop`; placeholder text only when ffmpeg absent AND
   no poster.
5. EDIT `crates/hubs/home/src/lib.rs`: subroutes become the backdrop
   switcher — `dpl` ("DPL loop", default), `dragon` ("Dragon loop"),
   `drift`, `pulse`. Paint order: aurora → media (video loop via
   `Media::Video`, or ambient) → scrim (VIDEO α over loops, BARE over
   ambiences) → hero. Asset paths resolved relative to the exe's repo root
   (`assets/backdrops/…`), missing file → poster → aurora-only (honest).
6. Workspace: add `crates/video` member + dep entries; app + slides consume.

### B2 — icons (`feat(icons)`)

1. Add `egui-phosphor` (verify egui 0.35 support via `cargo info`; if its
   egui pin mismatches, vendor its TTF + generated consts into
   `crates/coldlight/assets/` and register ourselves — same font, no
   second egui).
2. EDIT `crates/coldlight`: `pub mod icon` — font registration inside
   `apply_theme` + named consts mapped from the OLD inventory: HOME(House),
   WORKBENCH(Hammer), LIBRARY(SquaresFour), WIREDFRONT(FilmSlate),
   CHAT(Robot), CAMERA, VIDEO, SESSION(Play), STEPS(Footprints),
   REC(RecordFill), MARK(Footprints), STOP(Square), REGION(Scan),
   WINDOW(AppWindow), FULL(Monitor), FREE(Lasso), DOCK(CaretUp),
   FULLBLEED(ArrowsOut), CLOSE(X), BACK(CaretLeft).
3. EDIT hub crates: each `HubSpec.icon` → its `cl::icon::*` const.
4. EDIT `crates/shell/src/lib.rs` footer per the old spec: left = brand dot
   + "ColdLight" + **Snip launcher** (CAMERA + "Snip", tone halo; publishes
   `ShellEvent::PixvixSummon` on the fabric — B4 consumes; until then the
   inspector's signal count proves it fires); center = live viewport
   readout; right = kbd legend + Bot launcher (badge slot, navigates to the
   chat hub until the agent dock exists). Topbar right = full-bleed toggle
   (icon) → `full_bleed: bool` hides rails/bars (Esc exits); harness-visible.
5. EDIT `crates/fabric`: add `PixvixSummon` to `ShellEvent`.

### B3 — media optimization (`feat(media)`)

1. CREATE `crates/engine-media` (`dpl-engine-media`): 
   - consts VERBATIM: `MAX_W=2560, AVIF_Q=60, WEBP_Q=80, JPEG_Q=82,
     ANIM_FPS=24, ANIM_Q=70, VIDEO_MAX_H=1080, VIDEO_CRF=21`.
   - `optimize_image(src, out_dir)`: decode (image), EXIF-orientation bake
     (kamadak-exif), resize ≤2560 Lanczos3, emit `.avif` (ravif), `.webp`
     (webp crate), `.jpg` (image jpeg) — metadata stripped by re-encode.
   - `optimize_cutout`: alpha inputs → avif/webp/png, never jpeg. (The ONNX
     background-REMOVAL model itself is DEFERRED — ledger; this arm handles
     already-cut sources.)
   - `optimize_anim` / `optimize_video`: ffmpeg subprocess, args ported
     verbatim (libwebp q70 fps24 loop0; libx264 slow crf21 faststart,
     ≤1080p, poster).
   - freshness skip (`out newer than src`), never-clobber temps, recursive
     `optimize_tree(root)` over `assets/source` → `assets/generated`.
   - `[[bin]] media-cli <root>` so the operator runs it directly.
   - Tests: real tiny PNG through the pipeline in a temp dir → all three
     outputs exist, avif/webp smaller than source, second run skips (fresh).
2. EDIT workspace `image` dep: features `png, jpeg, webp` (decode+display);
   AVIF decode intentionally NOT in-app — the pipeline always emits a webp
   sibling; recorded here as the display strategy.
3. Library hub: media slot demos updated so an optimized webp renders live
   (proof the decode features work end to end).

### B4 — the overlay (`feat(pixvix)`)

1. SPIKE (first, throwaway harness flag): immediate viewport with
   `transparent(true).always_on_top()` on wgpu/Windows — screenshot proves
   per-pixel transparency; failure → borderless fullscreen fallback (plan
   boundary from port-pixvix §5).
2. CREATE `crates/pixvix` (`dpl-pixvix`): the palette viewport (compact,
   always-on-top): tabs **Img · Video · Session · Steps** (cl::icon), Img =
   mode row Region/Window/Full/Free → `dpl_engine_capture` stills → inbox;
   Video = recorder start/stop + elapsed REC readout; Steps =
   `dpl_engine_steps::StepsRecorder` start/mark/stop + REC-pill mode
   (CircleDot pulse, `REC · n`, Mark/Stop) that the palette collapses into
   while recording; Session = honest "lands with PV1c" plate. Region select:
   fullscreen frozen viewport over `freeze_primary_to` + drag rect + the
   magnifier loupe (zoomed texture sample around the cursor).
3. Global hotkey: extend engine-steps' rdev hook TRIGGERS with
   Ctrl+Shift+9 (steps toggle) publishing through the EventSink → fabric.
4. Wire the B2 footer launcher: `PixvixSummon` → palette viewport opens.
5. Exe proof: palette screenshot; a region snip lands in the inbox
   (never-clobber name) and the file is a real PNG.

## 5. Boundaries

- 🚫 ffmpeg stays a subprocess everywhere (ADR-009) — `egui-video`/
  `ffmpeg-next` are banned deps. 🚫 Old repo read-only; assets COPY only.
  🚫 Never delete Drift/Pulse (authored work). 🚫 No shims: shell footer is
  REAUTHORED, not wrapped.
- ⚠️ Ask-first: retiring the aurora (it stays — it's the fallback layer);
  any HubSpec contract change beyond icon VALUES.
- Deferred (ledger): ONNX background removal (cutout MODEL); AVIF decode
  in-app; Session video tab wiring (PV1c); agent dock console (the Bot
  launcher navigates to chat until then); NavDock trapezoid (old Z-pass
  surface — revisit after PV3).

## 6. Verification

Per pass: `cargo test --workspace` green; `cargo build --release` with
strays killed (`taskkill //F //IM devpulselabs.exe`); exe screenshots into
`docs/proof/` and eyeballed before commit:

- B1: `--route home/dpl --shot-frame 30` shows a REAL video frame (not the
  placeholder plate) over aurora under scrim; `home/dragon`, `home/drift`,
  `home/pulse` each distinct. ffmpeg absent (PATH scrubbed run) → poster.
- B2: default boot shows rail icons + footer (Snip camera, viewport
  readout, kbd legend, Bot); clicking Snip increments the inspector's
  signal count (fabric proof); full-bleed hides chrome.
- B3: `media-cli` on a temp root with a real source image → avif+webp+jpg
  on disk with sane sizes; rerun prints skip; library media demo renders
  the webp on the exe.
- B4: transparency spike shot; palette shot; REC pill shot; a snip file in
  the inbox opened and verified > 10 KB PNG.

End-to-end: cold `devpulselabs.exe` → mainpage plays the DPL loop under the
iconized shell; footer Snip opens the palette; a region snip lands in the
inbox; `media-cli` optimizes it to avif/webp/jpg; the library shows it.

## 7. Commit plan

One commit per pass, in order: `feat(backdrop)`, `feat(icons)`,
`feat(media)`, `feat(pixvix)` — plus this plan + the port-pixvix amendment
committed first as `docs(plan): build-out plan + OQ-PV1 resolved`. No push.
