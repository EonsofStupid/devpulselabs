# Pass D0 — this laptop's deps, proven (camera · ffmpeg · files) before anything else

**Status:** APPROVED 2026-10-10. The operator said "go" to the plan, with the
§4.5 recommendations DQ1–DQ6 adopted:
- DQ1: `roots.toml`, no folder-climbing.
- DQ2: camera presence at boot, and a real open in camera features and
  `--deps-check`.
- DQ3: a named CI profile.
- DQ4: depot Chromium only.
- DQ5: Codex/Grok not required to open the app.
- DQ6: a live preview through the recorder.

Authored 2026-10-10 in THE checkout (`D:\projects\devpulselabs`). Chat (C1+)
is paused behind this pass.

## 1. Context

Operator, 2026-10-10:

> *"I DO NOT WANT THIS FUCKING RUNNING IF MY FUCKING DEPS ARE NOT LOADING MY
> CAMERA MY FFMPEG MY FUCKING FILES GET LOADED PROPERLY CONFIGURED ON THIS
> LAPTOP"*

> *"If we need to install or patch we are in pre-release do not reward hack"*

On camera, the operator chose all four:
- face-cam picture-in-picture over screen recordings;
- the webcam recorded as its own clip;
- live camera preview;
- camera and microphone health in the startup check.

Neither this app nor the old app ever had webcam support.

The app today does the opposite of the first ruling. It **opens its workspace
whether or not its tools work**, and it reports recordings as "saved" when
nothing was captured. A green gate and a painted window have stood in for
"working". That is the reward-hack pattern this pass removes.

## 2. Goal / non-goals

**Goal:**
- On this laptop, the app **refuses to open its workspace unless every
  required dependency is proven working**: ffmpeg, pdftoppm, the Lottie
  helper, screen capture, the camera, the microphone, the operator's
  library, and a writable `.userdata`.
- When it refuses, it says in plain words what failed and how to fix it.
- Every media and file path is fixed where it is broken, and proven on the
  real exe by drills that **fail** when the thing fails.
- Camera support is built, all four parts.

**Non-goals:**
- No chat work.
- No new UI beyond the dependency screen and the camera controls.
- No in-process capture crate (camera goes through the depot ffmpeg as a
  subprocess, ADR-009).
- The operator's library (`content/`) is never moved, renamed or written by
  this pass except through the app's normal capture/import paths. Drills
  that import use a scratch copy.

## 3. Ground truth (verified in-session 2026-10-10 unless marked)

### 3.1 The build itself is stale in THE checkout

- `target/debug/deps/dpl_svg-*.exe` has `C:\localdev\devpulselabs\crates\svg/…`
  baked in. The checkout moved from `C:\localdev` to `D:\projects`, and cargo
  reused artifacts compiled there.
- So the gate fails
  (`icon::tests::the_whole_vendored_pool_tessellates`: "the vendored lucide
  pool is missing") and the push was correctly blocked.
- `target/` is 58 GB. It holds build output, the staged depot
  (`release/dptools-deps`, re-stageable), `target/tmp/` (Jul 27) and
  `target/tmp-h.png` (Jul 30), which have not been inspected yet.

### 3.2 Startup never refuses

- `init_ffmpeg` (`crates/app/src/main.rs:1571-1592`): on failure it logs
  `"verified embedded ffmpeg is unavailable; previews, probes, and renders will
  fail"` and **continues**.
- `init_pdftoppm` (`main.rs:1598-1617`): warns and **falls back to PATH**.
- Nothing checks the Lottie helper at boot (no `helper_path` use in
  `crates/app/src`).
- There is no dependency check or health surface anywhere in the app.

### 3.3 Microphone recording cannot work

- `list_audio_devices` (`crates/engine-capture/src/lib.rs:1174-1215`) only
  collects names after a `"DirectShow audio devices"` header line
  (`:1196`).
- The depot ffmpeg 8.1.2 prints no such header. On this laptop it prints
  `[in#0 @ …] "Microphone Array (Realtek(R) Audio)" (audio)` (live probe,
  2026-10-10).
- So the list is **always empty**, and Mic/System audio can never be picked.

### 3.4 Recordings lie about success

In `record_monitor_ctl` (`engine-capture/src/lib.rs:1262-1402`):
- ffmpeg stderr goes to null (`:1345`);
- the exit status is discarded (`let _ = ff.wait()`, `:1400`);
- it returns `Ok(frames)` even when `frames == 0` (`:1401`).

ViziWiz then prints `"saved {} ({frames} frames)"` for any N
(`crates/viziwiz/src/lib.rs:1114-1121`). The video and session drills exit 0
on a `"saved"` prefix, so **a failed recording passes its drill**.

### 3.5 Audio drifts off the video

- Frames are paced every 33 ms (`:1350`), which is 30.3 fps, while ffmpeg is
  told `-r 30` (`:1285-1286`).
- The video timeline stretches about 1% against wall-clock audio: roughly
  0.6 s of drift per minute. That figure is derived and is measured in §6.

### 3.6 The library lookup can leave the app folder

- `dpl_engine_capture::content_root()` (`lib.rs:93-180`) tries, in order:
  `--content-root`, `CONTENT_ROOT`, exe-dir `content/`, **4 parent
  levels**, then **CWD**.
- On Oct 2 a "portable" copy in `out/portable-test` resolved
  `D:\projects\devpulselabs\content`, **outside its own folder**
  (`out/portable-test/portable-run.log`).
- `dpl_video::locate_asset` (`crates/video/src/lib.rs:280-317`) is a second
  lookup with its own walk: the exe dir plus 3 parents, then CWD, trying
  `<r>/<rel>` and then `<r>/content/<rel>`. It can resolve a different tree.

### 3.7 Depot gaps

- Chromium is staged but **never injected**: `set_browser_path`
  (`crates/engine-browser/src/lib.rs:39`) has no caller. The browser search
  goes to system installs and PATH instead.
- The Lottie helper resolves `DPL_LOTTIE_RENDER`, then beside the exe, then
  a **bare name on PATH** (`crates/lottie/src/lib.rs:32-45`). Its doc
  comment states the order wrongly.

### 3.8 Camera

- There is no webcam code at all. The only dshow use is audio.
- This laptop's devices (depot ffmpeg, live): `"Integrated Camera"`
  (video), `"OBS Virtual Camera"` (video), `"Microphone Array (Realtek(R)
  Audio)"` (audio).
- The depot ffmpeg build includes dshow, overlay, scale, split, libx264,
  aac and libopus.

### 3.9 Where things plug in

These seams come from a read-only sweep; file:line was re-read for the
items above.
- **Recording args:** built at `engine-capture/src/lib.rs:1279-1316`;
  `VideoOpts` at `:1030-1041`; Windows audio args at `:1115-1143`.
- **ViziWiz palette:** fixed-size viewport (396×290; `viziwiz/src/lib.rs:1183-1196`);
  Video-tab chips and device ComboBox at `viziwiz/src/palette.rs:353-420`;
  `start_video` at `viziwiz/src/lib.rs:880-962`; session video hard-wired
  silent 1080p at `viziwiz/src/session.rs:195-230`.
- **Decode pattern:** a live preview would reuse `dpl_video::Player`'s
  rawvideo-pipe reader (`video/src/lib.rs:89-217`).
- **Natural camera home:** the wiredFRONT `VIZIWIZ` subroute is a
  placeholder today (`hubs/wiredfront/src/lib.rs:1474-1484`).
- **Tools status cell:** the status-bar registry at
  `shell/src/status.rs:89-130`.

## 4. Design

### 4.1 Sub-passes (each is its own PR, double-green, real-exe proof)

| Sub-pass | Scope |
|---|---|
| **D0.0 Clean rebuild of THE checkout** | Inspect `target/tmp/` and `target/tmp-h.png` and keep anything that is not scratch. Then `cargo clean`, a full rebuild from `D:\projects\devpulselabs`, re-stage the depot (`stage-tool-deps`, SHA-verified), and a full `cargo green`. Land the #257 correction. Rebuild the exe and check its mtime. Long jobs run in the background and are waited out. |
| **D0.1 Honest media paths** | §4.2. The fixes for 3.3–3.7, each with a unit test against real captured ffmpeg output. |
| **D0.2 The startup dependency check** | §4.3. The workspace does not open on a failed required dependency. |
| **D0.3 Camera, engine side** | §4.4 a–c: device list, webcam clip, face-cam PiP in the recorder, and `--camera-drill` / `--pip-drill`. |
| **D0.4 Camera, UI side** | §4.4 d: live preview, ViziWiz camera controls and the wiredFRONT camera panel. Runs through the `coldlight-system` gate (tooltips, tokens, 1100×700 + maximized proofs). |
| **D0.5 Gate truth (was G1)** | The `#[ignore]`d drills that only lacked a tool run against the depot inside `cargo green`, and the depot is installed on both estate runners. |
| **D0.6 A release is portable for real** | The operator's 08-05 ruling (`release-contract.md` §5.3): *"no remote … unzip copy a staged dptools deps is stupid we are not doing that"*. The nightly/release zip **bundles the depot** (ffmpeg, pdftoppm, chromium, plus codex/grok on the private repo, operator's call) and `roots.toml`, so **unzip on any Windows machine and it works**. Proof: unzip into a fresh folder outside every repo (e.g. `D:\portable-proof\`). The startup check is all-green, the library resolves inside that folder only, and a capture lands there. The Oct 2 `out/portable-test` leak (it reached the repo's `content/`) must be impossible. |

### 4.2 D0.1 — honest media paths

**Amended 2026-10-11 (re-grounded before code): D0.1 lands as two PRs.**

**D0.1a covers items 1–3 below.**
- **Devices.** `engine-capture` gains `pub enum DeviceKind { Video, Audio }`,
  `pub struct CaptureDevice { name, alt_name: Option<String>, kind }`, and
  `pub fn list_capture_devices() -> Result<Vec<CaptureDevice>, String>`.
  - It returns an honest `Err` when the probe fails, not an empty list.
  - The pure parser `parse_dshow_devices(&str)` reads the ffmpeg-8
    `"Name" (video|audio)` + `Alternative name "…"` format, and the older
    header format the previous parser targeted.
  - `list_audio_devices` is **removed** and its two ViziWiz callers
    (`viziwiz/src/lib.rs:1354,1364`) move to the new API (no shim).
  - The device **id** passed to dshow is the `alt_name` (unique, e.g.
    `audio=@device_cm_…`); the UI shows the `name`.
  - The fixture is this laptop's verbatim listing, CRLF preserved:
    `crates/engine-capture/tests/fixtures/dshow-list-ffmpeg-8.1.2.txt`.
- **Honest recordings, in both `record_monitor_ctl` and `record_monitor`.**
  - ffmpeg runs with `-hide_banner -nostats -loglevel error`. stderr is
    piped and drained into the last 64 lines; the exit status is checked.
  - `frames == 0` is an `Err`.
  - Before `Ok`, the output is verified: it exists, it is non-empty, and
    `ffmpeg -i` shows a Video stream, plus an Audio stream when audio was
    requested.
  - So ViziWiz's "saved …" (`lib.rs:1114-1121`) and the session's
    "session saved" (`:1148`) can only follow a real file.
- **Pacing.** Tick-indexed deadlines `start + n/30 s` replace the
  accumulating 33 ms sleep, in both recorders.
- **Drill.** A new harness flag, `--video-audio <none|mic|system>` (CLAUDE.md
  flag list + structure census, same commit). `--video-drill` then exits 0
  only on a verified save. The 60 s mic run measures |video − audio|
  duration.

**D0.1b covers items 4–5:** depot-only Chromium/pdftoppm/Lottie, and the
single library rule (`roots.toml`, `xtask stage-roots`, no parent walk or
CWD, `locate_asset` on injected roots).

1. **Device listing.**
   - `list_capture_devices() -> Vec<CaptureDevice { name, alt_name, kind:
     Video|Audio }>` reads the trailing `(video)`/`(audio)` tag and the
     `Alternative name` lines, and still accepts the old header format.
   - Identity is the `alt_name` (unique), and the display name is shown.
   - Unit-tested against this laptop's verbatim output, committed as a
     fixture.
   - Linux: v4l2 and pulse. The CI Linux lane compiles and unit-tests it; a
     real Linux device test is **not possible on this laptop** and is stated
     as such.
2. **Recordings tell the truth.**
   - ffmpeg stderr is captured into a bounded ring (last ~64 lines), and the
     exit status is checked.
   - `frames == 0` is an error.
   - "saved" is only reported once the file exists, is non-empty, and
     `ffmpeg -i` shows a video stream (plus an audio stream when audio was
     requested).
   - On failure the user sees the real ffmpeg reason in plain words, not
     "saved".
3. **Drift fixed.** Frames are paced on the schedule `start + n/30`, not on
   accumulating sleeps.
4. **Depot only, no PATH:**
   - Chromium is injected from the depot at boot.
   - pdftoppm's PATH fallback is removed.
   - The Lottie helper resolves beside the exe, or through an explicit
     `DPL_LOTTIE_RENDER`, never a bare PATH name.
5. **One library rule (DQ1):**
   - The content root is the flag, then `CONTENT_ROOT`, then a **`roots.toml`
     beside the exe** naming content/assets/userdata, then the beside-exe
     rings. There is **no parent walk and no CWD**.
   - `cargo xtask stage-roots` writes `target/release/roots.toml` for THE
     checkout, pointing at `D:\projects\devpulselabs\{content,assets,.userdata}`.
   - `locate_asset` stops walking and resolves against the injected roots.
   - The dependency screen shows which rule picked each root.

### 4.3 D0.2 — the startup dependency check

1. **The checks run first.** At launch, on worker threads in parallel, the
   window shows "Checking your tools…". If all **required** checks pass, the
   workspace opens.
2. **On any failure, the workspace stays shut.** The window stays on the
   dependency screen. Each row shows ✓ or ✗, with what failed, why, and what
   to do. Example: *"Camera can't be opened — close apps using it (Teams,
   OBS), or turn on Settings → Privacy → Camera → Let desktop apps access
   your camera"*. The screen has **Re-check** and **Copy report**. There is no
   "continue anyway".

| Check | Required | Proven by |
|---|---|---|
| Depot manifest | yes | All staged payloads re-verify (size + SHA-256) |
| ffmpeg | yes | Resolved from the depot, `-version` exits 0, **and** a 1-second test encode (lavfi → libx264 + aac) into `.userdata/temp` succeeds and is deleted |
| pdftoppm | yes | Resolved from the depot; `-v` runs |
| Lottie helper | yes | Beside the exe; renders 1 frame of a tiny Lottie embedded in the app |
| Screen capture | yes | At least one monitor is enumerable |
| Camera | yes | At least one video device is listed (DQ2 decides presence-only vs a real 1-frame open at boot) |
| Microphone | yes | At least one audio device is listed |
| Library | yes | Roots are resolved by the rule in §4.2 item 5; content exists and is readable. The file count is shown. |
| `.userdata` | yes | Writable: a probe file is written and deleted |
| Chromium | DQ4 | Depot payload verified and injected |
| Codex / Grok | no (DQ5) | Shown, but chat owns their states |

3. **After boot:** a **Tools** status-bar cell shows the result; clicking it
   reopens the screen.
4. **Headless:** `--deps-check` prints the table and exits non-zero on any
   required failure (for drills and CI).
5. **CI runners have no camera or microphone (DQ3).**

### 4.4 D0.3 / D0.4 — camera

**a. Webcam clip.**
- `record_camera_ctl(out, device, audio, stop)` runs ffmpeg with `-f dshow
  -rtbufsize 256M -thread_queue_size 1024 -i video=<alt>[:audio=<mic>]`.
- It stops gracefully by writing `q` to stdin, never kill or EOF, so the
  mp4 is finalised (`-movflags +faststart`).
- Poster, then `land()` as `CaptureKind::Video` into the library inbox.

**b. Face-cam PiP on screen recordings.**
- `VideoOpts.camera: Option<CameraOpts { device, corner, size_pct, margin }>`.
- The argument builder adds the camera input and replaces `-vf scale` with
  `-filter_complex "[0:v]scale…[s];[1:v]scale=…[c];[s][c]overlay=…[v]" -map
  [v]`. The audio map index is recomputed.
- Wired into both `start_video` and the session video.

**c. Drills that fail honestly.**
- `--camera-drill <s>`: the clip exists, has a video stream, its duration is
  within ±10%, and the poster exists.
- `--pip-drill <s>`: the same, **plus** the PiP corner region of a
  mid-recording frame is measured (mean luma / variance via ffmpeg `crop` +
  `signalstats`) to prove camera pixels are there, not black.

**d. Live preview + UI (D0.4).**
- A `dpl_video::camera` player: dshow → rawvideo pipe at a small decode size,
  no `-re`, with cooldown respawn.
- **One process owns the camera** (DQ6): while recording, the recording
  ffmpeg also emits a low-res preview stream (split), so the preview keeps
  running.
- ViziWiz Video tab:
  - a **Cam** chip with a device picker and a corner picker;
  - a **Cam** tab for webcam-only clips;
  - the preview opens in its own small always-on-top viewport (the palette
    is fixed-size).
- The wiredFRONT **VIZIWIZ** subroute (a placeholder today) becomes the full
  camera panel: device list, live preview, Test camera, mic level.
- Every control has a tooltip.
- Proof shots at 1100×700 and maximized.

### 4.5 Decisions for the operator

| DQ | Question | Recommendation |
|---|---|---|
| DQ1 | Library rule: drop the parent-walk/CWD fallbacks and use an explicit `roots.toml` beside the exe (written by `xtask stage-roots`)? | **Yes.** Never silent again. The screen shows the chosen root. |
| DQ2 | Camera at boot: presence only (no camera light every launch), or really open it for 1 frame every launch (the light flashes)? | Presence at every launch. A real 1-frame open when camera features open, and in `--deps-check`. |
| DQ3 | CI runners have no camera or mic. | A named **CI profile** (`--deps-profile ci`) that marks camera and mic as not checked. It is printed on every CI screenshot and log, and is never used on your machine. The alternative is installing virtual devices on the runners. |
| DQ4 | Browser: depot Chromium only, or also allow system Chrome/Edge? | Depot only, required (R36: no silent fallbacks). |
| DQ5 | Codex/Grok required to open the app? | No. They are chat providers; chat shows their state. |
| DQ6 | Preview during recording: keep it live through the recorder (split), or pause the preview while recording? | Keep it live (split): one process owns the camera. |

## 5. Wiring

- **engine-capture:** device listing, `record_camera_ctl`, camera in
  `VideoOpts`, honest results, pacing, single roots rule (+ `roots.toml`).
- **video:** `camera` preview player; `locate_asset` resolves through
  injected roots.
- **engine-browser:** depot injection at boot.
- **lottie:** no PATH.
- **composer:** pdftoppm, no PATH.
- **app:**
  - a `deps` module running the checks before the workspace, plus
    `--deps-check`;
  - each check lives in its owning crate as a `health()` function, and the
    app only orchestrates;
  - new drill flags (`--camera-drill`, `--pip-drill`, `--video-audio <mode>`).
- **shell:** Tools status cell.
- **viziwiz / hubs/wiredfront:** camera UI.
- **xtask:** `stage-roots`; the drills in the gate (D0.5).
- No new crate is planned. If one becomes necessary, `architecture.md` and
  the xtask audit table are amended in the same commit.

## 6. Proof (on this laptop, real exe, every sub-pass)

**R36 rules for every item below.** No check passes by being skipped. No
mock device. No fixture presented as live. Every drill exits non-zero on
failure.

- **D0.0:** `cargo green` passes in THE checkout. The exe mtime moves.
  `--deps-check` (once it exists) is green.
- **D0.1:**
  - Mic device list unit test against the real fixture.
  - `--video-drill 5` in mp4, vp9 and av1: frames > 0, the file has a video
    stream.
  - `--video-drill 60 --video-audio mic`: the audio stream is present, and
    |video duration − audio duration| < 1 frame (measures 3.5's fix).
  - A deliberately broken run (scratch copy of the depot with a corrupted
    ffmpeg) fails loudly instead of "saved".
- **D0.2:** screenshots of the dependency screen all-green, and **really**
  failing. The failure is real: the exe is pointed, via `roots.toml` in a
  scratch folder, at an empty library and an incomplete depot copy. Nothing
  is mocked.
- **D0.3:** `--camera-drill 5` and `--pip-drill 5` green on the Integrated
  Camera, and the PiP luma check passes.
- **D0.4:** proof shots of the preview and controls at 1100×700 and
  maximized.
- **Orphan check:** before and after each drill, no stray `ffmpeg.exe` or
  `devpulselabs.exe`.
- **Every PR:** both CI lanes green before merge (docs-only PRs via
  `workflow_dispatch`). `STATE.md` and `CURRENT.md` are updated.

## 7. Risks

- **Camera exclusivity.** Teams or OBS holding the camera means an open
  fails. The check names the cause.
- **Windows camera privacy setting.** It blocks dshow silently. The check
  detects the dshow error text and points at the setting.
- **OBS Virtual Camera** only produces frames while OBS's virtual camera
  runs. It is listed, never auto-picked over the Integrated Camera.
- **Performance.** Full-resolution RGBA through stdin plus camera decode and
  overlay in one ffmpeg. CPU is measured during the 60 s drill, and frame
  drops are reported (dshow `rtbufsize`).
- **xcap holds one DXGI duplication per process.** New capture goes through
  the existing `RECORDERS` cache (`engine-capture/src/lib.rs:1242-1256`).
- **Linux camera cannot be exercised on this laptop.** It is compiled and
  unit-tested on the Linux lane only, and stated as unproven on hardware.
- **Scale.** Six sub-passes. Each lands green and usable; nothing is
  half-merged.
- **Parallel lanes.** `gh pr list` before each sub-pass.

## 8. Evidence

- **Clone and library state.** THE checkout was fast-forwarded to `3ae1dd3`.
  `sources.local.toml` was repointed to `D:/projects/dptools-deps`, and all
  five sources were verified. `content/` holds 62 files / 150 MB.
- **Build-cache cause.** `C:\localdev\devpulselabs\crates\svg/../coldlight/assets/lucide/icons`
  is baked into `target/debug/deps/dpl_svg-648d2c64567c228f.exe` (Aug 10) and
  `dpl_svg-00085f28ccb5d4c5.exe` (Aug 7).
- **Gate failure.** The push of the close-out correction (`69ef111`) was
  blocked by the pre-push gate on that test. That is correct behaviour.
- **Live device listing** (depot ffmpeg 8.1.2, `-list_devices true -f dshow
  -i dummy`) shows the three devices in §3.8 and no section headers.
- **Oct 2 portability logs.** The summary's `screenshot=False` is wrong
  (each log shows `screenshot written`). The portable run resolved the
  repo's `content/`.
### D0.0 — clean rebuild of THE checkout (2026-10-10/11)

- **Cache inspected first.** `target/tmp/` was empty; `target/tmp-h.png` was
  a Jul 31 harness shot (build `16e9d9c`), backed up to the session
  scratchpad. Then `cargo clean` removed **124,679 files, 57.5 GiB**.
- **The cold gate passed.** The `cargo green` that ran when pushing
  `a10d80e` went 21:48 → 22:06 with every phase green: fmt · lock · spawn
  audit · structure · coverage · clippy · tests **997 passed, 0 failed, 6
  ignored** (the six ignored are D0.5's subject) · release · smoke. Output:
  `green ✔ exe: D:\projects\devpulselabs\target\release\devpulselabs.exe`
  (Oct 11 22:06). `dpl-lottie-render.exe` was rebuilt (21:58).
  - The earlier failure (`icon::tests::the_whole_vendored_pool_tessellates`,
    with `C:\localdev` paths baked into old test binaries) is gone. It was
    the stale cache, not code.
- **Depot re-staged.** `cargo xtask stage-tool-deps target/release` staged
  codex, grok, chromium, ffmpeg and pdftoppm, and ended with *"DevPULSE tool
  depot verified"*.
- **Real exe, 1600×900** (`docs/proof/d0-0-boot-1600x900.png`):
  - Library: `three-ring root ready content=D:\projects\devpulselabs\content`
    (found by the parent walk that D0.1 replaces).
  - ffmpeg and pdftoppm: `verified embedded … resolved` from the depot.
  - Codex and Grok: `loaded non-secret … activation evidence state=Ready`.
    That is **stored evidence, not a live check**; A2 owns the live proof.
  - The editor's Project panel lists the operator's footage.
  - The footer is visible, stamped `a10d80e`. No orphan `devpulselabs.exe`.
### D0.1a — honest media paths (2026-10-11)

**Fixtures (real bytes).** Three files are captured from the depot ffmpeg
8.1.2 on this laptop:
- `crates/engine-capture/tests/fixtures/dshow-list-ffmpeg-8.1.2.txt`, the
  verbatim device listing, kept CRLF by a new `.gitattributes` rule for that
  folder only;
- `ffmpeg-i-video-audio-8.1.2.txt` and `ffmpeg-i-video-only-8.1.2.txt`,
  `ffmpeg -i` of real lavfi-generated clips.

**Unit tests:** 9 new in `honest_media_tests`, 14/14 in
`dpl-engine-capture` lib. They cover:
- the ffmpeg-8 inline listing parses all three devices, CRLF and LF;
- the older header format still parses;
- the device id is the unique alternative name;
- `parse_streams` tells video+audio from video-only;
- frame deadlines are exact (tick 30 = 1 s, tick 1800 = 60 s);
- drift catches the old pacing (1818 frames / 60 s > 0.5 s);
- a failure carries ffmpeg's last stderr line.

**Static checks.** clippy `-D warnings` is clean on `dpl-engine-capture`,
`dpl-viziwiz` and `devpulselabs`. The structure audit reports **48 flags in
census** (`--video-audio` added to `main.rs` and CLAUDE.md together).

**Real-exe drills.** Each ran into a scratch `--content-root`, never the
operator's library, one at a time:

| Drill | Result |
|---|---|
| `--video-drill 5` mp4 · vp9 · av1 | each **saved 149 frames**, file verified; `149 frames = 4.967s of video over 4.967s of wall clock · drift 0.000s` |
| `--video-drill 60 --video-audio mic` | **saved 1798 frames**, `59.933s of video over 59.934s of wall clock · drift 0.001s`; `ffmpeg -i`: `Video: h264 … 30 fps` + `Audio: aac (LC) 44100 Hz` (the Realtek microphone) |
| failure (output folder write-denied via `icacls`) | exit 1: `video: the recording failed (ffmpeg exit code: 0xfffffff3): Error opening output files: Permission denied`; no file; this previously printed "saved" |
| orphan check, after every drill | no `ffmpeg.exe` left |

**Corrected in-session, recorded (R36).** The first mic run reported
`video 59.960s · audio 59.960s · |Δ| 0.000s` by measuring file stream
durations. That was **not evidence**: the recorder's `-shortest` trims both
streams to equal length whatever the drift. It was replaced by the engine's
`Recorded { frames, seconds }`: frames ÷ 30 against the loop's measured
wall clock, which the old 33 ms pacing fails.

**Also fixed in the touched code:**
- A failed ffmpeg spawn now stops the already-started screen recorder.
- The palette tooltip "Theme for audio devices again" now reads "Scan for
  audio devices again".

**Privacy.** The drill recordings, including the mic ones, were deleted from
the scratchpad after measurement.

- Later sub-passes are filled in as they land.
