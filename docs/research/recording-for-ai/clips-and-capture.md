# Recording across apps and generating data video: research digest

Researched 2026-11-03 to 2026-11-04. Nothing was run. No repo file was changed.

**How to read the marks**

- **[V]** I read it on a page in the source list. The number in brackets points to that page.
- **[R]** I read it in the repo, at the file:line given. Repo paths are relative to `D:\projects\devpulselabs`.
- **[I]** My inference. Treat it as an argument, not a fact.
- **[U]** I could not verify it.

**A limit on [V].** Pages came through a fetch tool that summarises them, so short quotes passed through that tool. Before relying on a licence line, read the original page. I am not a lawyer.

---

## The short answer

1. **Build a native clip compositor, not a Remotion pipeline.** It works over screenshots from the real recording ("plates"), and it writes one label row for every video frame.
2. **Neither Remotion nor egui can redraw another company's app.** Re-enacting Chrome or Excel means showing captured plates and animating a cursor, click marks, key captions and zoom on top. [I] That is a small 2D job, and it removes most of Remotion's advantage.
3. **Only re-enactment gives exact per-frame truth.** The live recorder writes "the latest frame" on a clock (`crates/engine-capture/src/lib.rs:361-363`, `:1392-1398`), so a live label can be a frame or so off. [R, I]
4. **Remotion's licence is not the blocker.** A solo person is free. The blockers are weight, a second copy of the UI in React, and no per-frame label output.
5. **One FFI exception is needed**, for `WS_EX_NOACTIVATE`. Everything else in the recorder can use crates with safe public APIs.
6. **Speech-to-text: whisper.cpp as a depot subprocess.** First check whether the depot's ffmpeg already has the `whisper` filter.
7. **A local vision model can describe key frames. It cannot find what was clicked.** Hooks and UI Automation already give that exactly.

---

## 1. Remotion today

### Version

- `remotion` on npm is **4.0.540**, with licence field `SEE LICENSE IN LICENSE.md` and unpacked size 2,086,743 bytes. [V 8]
- A search snippet dated v4.0.537 to 2026-10-19 on a SourceForge mirror, so 4.0.540 is recent. [U: I did not open that page or confirm the 4.0.540 date]
- **5.0 is not out.** The docs say: "Remotion 5.0 is not yet released." [V 7]

### How rendering works

The flow has three steps: `bundle()`, then `selectComposition()`, then `renderMedia()`. A bundle is made once and can be reused. [V 26]

| Part | What it does | Source |
|---|---|---|
| `renderMedia` | Needs `serveUrl`, `composition` and `codec`. Runs in Node or Bun. | [V 13] |
| `serveUrl` | Can be a local path to a bundle. | [V 13] |
| `inputProps` | Must be a JSON object. Pass the same values to `selectComposition`. | [V 13] |
| Concurrency | Defaults to half the CPU threads. | [V 13] |
| CLI | `--props` takes JSON, but "Inline JSON string isn't supported on Windows shells", so use a file. `--frames` plus `--sequence` output images. | [V 14] |
| `calculateMetadata` | Sets duration, size, fps, props and codec from data. Called "a single time, independently from the concurrency of the render", in a separate tab, before frames render. The result must be JSON-serialisable. | [V 15] |

**Schema validation from JSON is weaker than it sounds.**

- `<Composition schema>` takes a top-level `z.object()` that default props must satisfy. [V 17, 16]
- The docs do not say what happens at render time if input props fail. [V 16, 17]
- The one core file I checked, `resolve-video-config.ts`, does no Zod parsing. It validates only width, height, fps, duration and codec. [V 18]
- So I could not confirm that render-time input props are validated. [U]
- Validate in Rust before spawning, as the document law already requires (`docs/design/architecture.md:149-150`). [R, I]

### Browser

- Default is **Chrome Headless Shell**. Remotion auto-installs it into `node_modules/.remotion/chrome-headless-shell/[platform]/`. [V 19]
- Pinned version is **158.0.8127.2**, from `TESTED_VERSION` in source. [V 19, 21]
- Remotion writes a `VERSION` file and replaces the browser on mismatch. [V 19]
- Downloads for win64, mac-arm64 and Linux with glibc 2.35 or newer come from `remotion.media`. Other cases use Google's Chrome for Testing storage. [V 21]
- `npx remotion browser ensure` or `ensureBrowser()` pre-fetches it. `browserExecutable` points at your own copy. [V 19, 22]

### Node

The docs say: "To use Remotion, you need at least Node 16 or Bun 1.0.3." [V 24]

### Determinism

I found **no formal guarantee**. The `/docs/flakiness` URL I tried returned 404.

- Frames are a pure function of `useCurrentFrame()` only if you write them that way.
- Parallel rendering is why `Math.random()` breaks: "Remotion is spinning up multiple instances of the webpage to render frames in parallel". Use seeded `random()`. [V 27]
- With a full Chrome binary, rendering "may be less deterministic than Chrome Headless Shell". [V 19]
- So determinism depends on your code plus the pinned browser. Byte-identical output across machines is unconfirmed. [I, U]

### Render speed

- Remotion publishes **no fps figures**. Its performance page has none. [V 28]
- It lists what matters: your code, `--concurrency`, GPU effects on machines without a GPU, PNG versus JPEG, and resolution. [V 28]
- Hardware acceleration covers **encoding only**, is off by default, and uses VideoToolbox on macOS or NVENC on Windows and Linux. Linux ARM64 lacks NVENC. [V 29]
- Search snippets disagree by about 10x: an unmeasured "10-30 minutes on a laptop for 60 s at 1080p30", against one blog's 1080 frames in about 15 s. [U: I did not open these, and neither states its hardware]
- Measure with `npx remotion benchmark` on your own composition. [V 28]

### Offline use

The docs I read do not mention it. [V 22, 26] My reading is that a staged node, shell, compositor and bundle should need no network unless the composition fetches something. [I]

- Server-side telemetry is sent only when a licence key is set. [V 5]
- 5.0 asks eligible free users to pass `"free-license"`. [V 7, 5] Whether that makes a network call offline is unknown. [U]

### Platform support

| Platform | Status | Source |
|---|---|---|
| Windows x64 | Yes | [V 19] |
| Windows ARM64 | **No.** Source error: "Chrome Headless Shell is not available for Windows for arm64 architecture." No compositor package for it either. | [V 20, 9] |
| Linux x64 and arm64 | Yes, with glibc 2.35 or newer, plus system libraries (libnss3, libgbm, libcups2 and others). | [V 19, 24, 25] |
| Alpine, NixOS | Unsupported. | [V 24, 25] |
| macOS x64 and arm64 | Yes, but "macOS 15 (Sequoia) or later is required." | [V 19, 24] |

On Linux the install is **not fully self-contained**, because of those system libraries. [V 25, I]

### Can a Rust host drive it as a subprocess, with no web UI in the app?

**Yes.** [I, from V 13, 14, 26] Rust writes a job file and spawns `node render.mjs job.json`, following the existing quiet-spawn pattern (`crates/engine-capture/src/lib.rs:72-83`). [R] Studio is not needed at render time.

It would not stay Rust-only, though. Compositions are React and TypeScript, so the repo gains `package.json`, a lockfile, a bundling step and JS in CI. [I]

### Size of a minimal install

| Piece | Size | Status |
|---|---|---|
| Node 24.22.2 | 38 MB zip on Windows, 31 MB tar.xz on Linux ARM64, 53 MB tar.gz on macOS ARM64 | [V 31] |
| `@remotion/compositor-win32-x64-msvc` | 29.8 MB unpacked | [V 11] |
| `@remotion/compositor-linux-arm64-gnu` | 15.0 MB unpacked | [V 12] |
| `@remotion/renderer` | 3.0 MB, plus ws, execa, source-map, licensing, streaming | [V 9] |
| `remotion` | 2.1 MB, plus React and ReactDOM as peers | [V 8] |
| Chrome Headless Shell | About 90 MB download, from a third-party crate's estimate. Exact size unverified: Google's listing returned 403. Unpacked size unknown. | [V 32, U] |
| `@remotion/bundler` | Author-time only if you pre-bundle. It pulls webpack, rspack and esbuild. | [V 10, I] |

**Total estimate: about 125-175 MB to download, and very roughly 300-500 MB on disk. I did not measure this.** [I]

### Can it be bundled into a depot and pinned?

**Yes.**

- Versions pin exactly with `--save-exact`. [V 30]
- `binariesDirectory` relocates the "platform-specific binaries and libraries", which the docs say suits places like "bundled serverless functions or Electron". [V 23]
- `browserExecutable` relocates the shell. [V 13, 19]
- The depot's checksum resolver (`docs/design/architecture.md:53`) fits this. [R, I]
- Keep Remotion's own files out of the public repo. See section 2.

---

## 2. Remotion's licence

### The lines that decide it

From `LICENSE.md` [V 1], https://raw.githubusercontent.com/remotion-dev/remotion/main/LICENSE.md:

> "You are eligible to use Remotion for free if you are:"
> "an individual"
> "a for-profit organization with up to 3 employees"
> "a non-profit or not-for-profit organization"
> "evaluating whether Remotion is a good fit, and are not yet using it in a commercial way"

> "You are required to obtain a Company License" ... "if you are not within the group of entities eligible for a Free License."

> "It is not allowed to copy or modify Remotion code for the purpose of selling, renting, licensing, relicensing, or sublicensing your own derivate of Remotion."

> "In Remotion 5.0, the license will slightly change."

From the licence FAQ [V 2], https://www.remotion.dev/docs/license/faq:

> "Any commercial use case is allowed as long as you are not selling Remotion as a product itself."
> "If you are eligible for the Free License, you can also run an automation."
> "You don't have to contact us to use Remotion under the Free License."
> "Remotion is source-available software, but it is not open source software."
> "their headcount aggregates with yours for the 4-person threshold."

The last line is about freelancers or an agency who operate the software on the same project.

From the terms [V 3], https://www.remotion.dev/docs/terms:

> "Any User qualifying as a Free License User is allowed to use the Remotion Software for free."

From the telemetry page [V 5], https://www.remotion.dev/docs/license/telemetry:

> "Server-side rendering only sends telemetry when a license key is set, and only for successful renders."

### What needs a Company License

Anyone outside the free group, in practice a team of 4 or more. [V 1, 2, 4] Prices are on https://www.remotion.pro/license: [V 4]

- **Automators:** "$0.01 per render, $100/mo minimum."
- **Creators:** "$25/mo per seat."
- **Enterprise:** "Starting at $500 per month."

### What it means for a solo operator with a public repository

- **Free.** The test is who you are, not whether the repo is public. I found no term that ties the licence to repo visibility. [V 1, 2, 3; I]
- **It is not open source, so it cannot be relicensed under the repo's `LICENSE`.** Do not vendor Remotion source or `node_modules` into the repo. Pin it in a manifest and stage it into the gitignored depot. [V 2, 3; I]
- **Readers of the public repo get no licence from you.** A company of 4 or more that uses your compositions needs its own. The terms say this for exported code: "If the end-user qualifies as a Company License User, they are required to obtain a Company License independently." [V 3] Extending it to a public repo is my reading. [I]
- **The terms restrict shipping apps.** Desktop apps may be distributed only if end users cannot directly edit or upload Remotion code, and exported code must say it was built with Remotion. A personal tool is not affected today. [V 3; I]
- **Watch two triggers:** bringing in a contractor to operate it, and 5.0.
  - PR #3750 (open, draft) adds "Contractors also count towards team size." [V 6]
  - 5.0 also ties the licence to updated terms. [V 7]

---

## 3. Alternatives for scripted, deterministic clips

| Option | Determinism | Dependency weight | Licence | Exact per-frame labels |
|---|---|---|---|---|
| **Native Rust: egui and wgpu offscreen, piped to ffmpeg** | Host sets the clock (`RawInput.time`). [V 48] Layout and timing should be repeatable. [I] **Pixels can differ** by GPU, OS, backend and driver. [V 49] | **Almost none new.** `egui-wgpu` is already direct, `wgpu` 30.0.0 is locked, ffmpeg is already staged. [R: `Cargo.toml:98-100`, `Cargo.lock:8768`] | Same as now. | **Best.** The same loop computes state, draws the frame and writes the label, so they cannot drift. Widget rects are readable: `Node::rect()` "returns the rect in logical ui coordinates". [V 51; I] |
| **Lottie / ThorVG** | Frame-seek rendering, so repeatable. [I] WebGPU output described as "identical visual output", but no general guarantee. [V 46] | Already in the repo as the one FFI helper (`Cargo.toml:133-135`). [R] ThorVG core as small as about 170 KB. [V 46] | MIT. v1.1.3, 2026-10-15. [V 46, 47] | Only what you authored. No layout engine. Good for cursor and ripple overlays, not whole screens. [I] |
| **HyperFrames (HTML)** | Strongest published claim: "Same input, identical output." Integer frame maths, `seekFrame`. Local renders can differ by a pixel across machines; Docker is the fix. [V 35] Its engine now falls back to screenshot capture on newer Chromium (PR #296). [V 36] | Heavy: Node 22 or newer, headless Chrome via Puppeteer, FFmpeg. CLI 23.9 MB unpacked. [V 34, 37] | **Apache-2.0**, "no per-render fees or commercial-use thresholds". v0.9.8. [V 34, 37] | Possible through a custom adapter reading DOM rects after each seek. Not built in. [I] |
| **Remotion** | See section 1. | Heavy, see section 1. | Proprietary, source-available. [V 1, 2] | Not built in. Per-frame output is only counts through `onProgress`, plus `slowestFrames`. [V 13] Workaround: compute labels in Rust from the same JSON if layout is absolute, or log rects through `onBrowserLog`. [I] |
| **Revideo** | Scripted scenes in a headless browser. Guarantee not checked. [U] | Heavy: "runs anywhere Node and a headless browser run". [V 39] | MIT. `@revideo/core` 0.11.0. [V 40, 41] | Custom work. [I] |
| **Motion Canvas** | Generator-based. Guarantee not checked. [U] | Node plus a browser editor. Output depends on "the capabilities of your browser". [V 45] | MIT. `@motion-canvas/core` 3.17.2. [V 43, 44] | Custom work. [I] |

**Project health notes**

- Revideo's repo now resolves to `midrender/revideo`, last pushed 2026-07-15, and sends anonymous render counts through PostHog unless `DISABLE_TELEMETRY=true`. [V 39, 40]
- Motion Canvas was last pushed 2026-07-02. [V 43] Its rendering docs describe a Render button in the editor and mention no headless CLI. [V 45] That is a poor fit for a subprocess.
- HyperFrames was created 2026-03-10 and is very active (pushed 2026-11-03), so it is young and moving quickly. [V 38]

### Which can re-enact a GUI timeline with exact per-frame truth?

All of them can draw it, if the timeline JSON is the single source and positions are absolute. Only the native route gets labels with no extra machinery, and only native reuses the app's real widgets. [I]

The React options would need a second copy of every ColdLight widget, which fights the "One truth" law (`CLAUDE.md:37-42`) and "Surfaces paint ColdLight only" (`docs/design/architecture.md:161`). [R, I]

### Native renderer notes

- **Offscreen rendering is standard:** render to a texture, copy to a buffer, map and read it. [V 54] Those bytes go into the existing pipe, `-f rawvideo -pix_fmt rgba -i -` (`crates/engine-capture/src/lib.rs:327-352`). [R]
- **Ready-made headless harness:** `egui_kittest` has `Harness::render() -> Result<RgbaImage, String>`, `step()`, `with_step_dt()` and `input_mut()`. [V 50, 52] It is only a dev dependency here today (`Cargo.toml:104-105`), and docs.rs shows 0.37.0 while the repo locks 0.36.1 (`Cargo.lock:2917-2918`). [R] Using `egui-wgpu` directly avoids shipping a test crate. [I]
- **Identical pixels across machines:** do not assume them. Labels do not depend on this, because geometry comes from the layout code. [I] A CPU option exists but is young: `egui_software_backend` 0.0.3. [V 53] wgpu's fallback adapter is another route, but its docs page returned 404 for me. [U]
- **For training data:** the current encoder uses `yuv420p` (`crates/engine-capture/src/lib.rs:345-346`). [R] That subsampling softens thin coloured text, so consider lossless frames for datasets and H.264 for demos. [I]
- **Timeline document:** it fits the existing law (`v`, refuse newer, crash-safe writes) and the schema pattern at `docs/design/architecture.md:51` and `:149-150`. [R, I]

### Where Remotion would still win

CSS, SVG and typography polish, a large effects library (a search snippet mentions `@remotion/effects`, not opened), and a tested parallel render pipeline. If demos ever need that polish, add it later behind an ADR-009 seam. HyperFrames has the simpler licence if a web lane is ever wanted. [I]

---

## 4. Recording across apps on Windows, and the FFI question

### House law today

- `unsafe_code = "deny"` across the workspace (`Cargo.toml:82-83`). [R]
- The one opt-out is `crates/lottie-render/Cargo.toml:18-22`, recorded at `docs/design/architecture.md:78-81`. [R]
- `-sys` links are allowed only there (`docs/design/architecture.md:158-160`, `:184`). [R]
- **Wrapped FFI is already accepted in practice:**
  - `webp` pulls `libwebp-sys` (`Cargo.lock:8743-8750`). [R]
  - `xcap` pulls `windows`, `pipewire`, `xcb` and `objc2-*` (`Cargo.lock:9350-9378`). [R]
  - `rdev` pulls `winapi`, `x11` and `cocoa` (`Cargo.lock:6479-6492`). [R]
  - So "no FFI in our code" holds for any crate with a safe public API. I did not read the audit's code, so the claim that it checks only direct deps is inferred. [I]

### Capability table

| Need | Crate and version | Internal unsafe | Safe API for us? | In this repo? |
|---|---|---|---|---|
| Focused window, title, app, pid, screen origin | `xcap` 0.9.8, Apache-2.0 [V 64] | Yes [I, from its deps] | Yes | **Yes** (`crates/engine-capture/Cargo.toml:15`; used at `crates/engine-capture/src/lib.rs:846-876`) [R] |
| Focus-change events | `uiautomation` `add_focus_changed_event_handler` [V 57] | Yes, over `windows` [V 56] | Plain `pub fn` [V 57] | No |
| Foreground-change events | `wineventhook` 0.11.0, MIT [V 66, 67] | Wraps `SetWinEventHook` [V 66] | Example has no `unsafe` [V 67] | No. Needs tokio and async-thread. [V 67] Workspace has tokio with `sync` only (`Cargo.toml:145`). [R] Exact foreground variant unconfirmed. [U] |
| Active window, polling, three OSes | `active-win-pos-rs` 0.11.0, MIT OR Apache-2.0 [V 65] | Not checked [U] | Not checked [U] | No. xcap already covers this. |
| Global click and key capture | `rdev` 0.5.3, MIT [V 59, 60] | Yes, heavily, including `static mut` [V 61] | `listen` is safe to call | **Yes** (`crates/engine-steps/Cargo.toml:12`; used at `crates/engine-steps/src/lib.rs:545`) [R] |
| What was clicked (UI Automation) | `uiautomation` 0.25.1, Apache-2.0, depends on `windows ^0.62.2` [V 55, 56] | Yes [I] | **Yes**: `element_from_point`, `get_name`, `get_control_type`, `get_bounding_rectangle`, `get_automation_id`, `get_process_id`, `is_password` are all plain `pub fn` [V 57, 58] | No, but the same `windows` 0.62.2 is already locked (`Cargo.lock:9000-9001`), so little new code. [R, I] |
| **Non-activating overlay** | **No safe route found** | n/a | **No** | No |
| Raw Win32, if we must | `windows` 0.62.2 [V 70]; `winsafe` 0.0.29 [V 68] | Yes | **No**: winsafe's call is `pub unsafe fn SetWindowLongPtr` [V 69] | `windows` only transitively. No direct `windows` or `winsafe` line in any `Cargo.toml` I grepped. [R] |

### Overlay detail

`WS_EX_NOACTIVATE` is the right flag. Microsoft says a window with this style "does not become the foreground window when the user clicks it". [V 73]

It is not reachable from our current stack:

- winit 0.30.13's Windows window options do not include it. [V 71] The repo locks that same winit (`Cargo.lock:9201-9202`). [R]
- egui has `with_active`, `with_always_on_top`, `with_taskbar`, `with_mouse_passthrough` and `with_transparent`, but nothing that stops activation on click. `with_active` is only "initially focused". [V 72] (Docs are 0.37.0; the repo is on 0.36.1.)
- So this needs a few lines of raw `GetWindowLongPtrW`, `SetWindowLongPtrW` and `SetWindowPos`. [I] That is a new named exception, and the architecture doc and the audit table must change in the same commit (`docs/design/architecture.md:6-8`, `:61-62`). [R]
- Today's overlay crate cannot just be reused. `dpl-coach-overlay` is devtools-only, Linux X11 first, and "Production builds must not wire it" (`docs/design/architecture.md:84-86`). [R]

**Spike this before relying on it.** In a Windows Terminal issue, engineers reported the style did not always stop focus moving, and pointed to `SW_SHOWNOACTIVATE` at `ShowWindow`. The issue later closed without naming the fix. [V 77] Whether winit also takes activation or capture on a press is unknown. [U]

**A bug this would expose.** The recorder drops our own clicks by checking which process has focus (`crates/engine-capture/src/lib.rs:846-856`). With a non-activating overlay, focus stays on the recorded app, so **overlay taps would be logged as clicks in that app**. Add a hit-test against the overlay rect, or compare `element_from_point(...).get_process_id()` to our pid. [R, I]

### What the hook does today, and its limits

- It keeps only left press and Return, plus three chords (`crates/engine-steps/src/lib.rs:216-226`, `:574-581`). Recording "every click and key" means widening this. [R]
- `rdev` gives `time`, `name` and `event_type`. [V 60] `ButtonPress` has no coordinates and events carry no modifiers, so the code tracks both (`crates/engine-steps/src/lib.rs:190-196`). [R]
- `listen` has no stop API (`crates/engine-steps/src/lib.rs:8-11`). [R]
- **It is old.** The last `rdev` release was 2023-06-26, and the README calls it "a pet project". [V 59, 60] It works here, but it is a replacement risk. [I]
- Internally it uses `SetWindowsHookExA(WH_KEYBOARD_LL…)` and `WH_MOUSE_LL`. [V 62]
- **Hook timing law:** Windows silently removes a low-level hook that times out, with "no way for the application to know", and caps the timeout at 1 second. [V 75] So do no work in the callback. The repo already follows this (`crates/engine-steps/src/lib.rs:8-11`). [R]
- **UI Automation runs on a worker.** Microsoft says to use a separate thread that owns no windows and uses COM MTA. [V 74] Capture x and y in the hook, query on the worker, and expect the UI to have moved slightly by then (a menu may have closed). [I]
- **Elevated windows go dark.** UIPI blocks lower-privilege hooks into higher-privilege processes. [V 110] A reported case: no mouse data while Task Manager has focus. The suggested fix is to watch the foreground window's integrity level and mark that span as "not observed". [V 109] UIAccess would bypass this but needs signing and an install under Program Files. [V 110] Do not ask for it for a personal tool. [I]
- **Passwords.** This is effectively a keylogger. Use `is_password()` to suppress keystrokes in password fields. [V 58, I] Antivirus may flag it. [U]
- **UIA coverage varies.** Standard controls should work. Custom-drawn apps may expose only a window. Coverage of Electron and browser apps needs testing. [I, U]

### Pause and resume

| | Piped frames (current style) | ffmpeg segments, one process per run |
|---|---|---|
| **How** | Stop writing to stdin. Rebase the clock, since `frame_deadline` is computed from `start` (`crates/engine-capture/src/lib.rs:304-306`). [R] | Finish the ffmpeg process on pause, start a new one on resume, join with the concat demuxer at the end. |
| **Video** | Gapless, because output time comes from frame count at `-r 30`. [I] | Gapless after joining with stream copy. [I] Files must have "the same streams (same codecs, same time base, etc.)". [V 78] Same settings give that. [I] |
| **Audio** | **The catch.** The mic is a live second input in the same ffmpeg (`crates/engine-capture/src/lib.rs:1309-1334`). It should keep capturing through a pause, which would desync A/V. [R, I] | Clean: each segment has its own audio. Possible tiny gaps at joins from audio codec framing. Test this. [I, U] |
| **Crash safety** | One long file, lost if ffmpeg dies before closing. [I] | Every finished segment survives. [I] |
| **Recorder** | `start()` and `stop()` only wake and sleep the capture thread, and the recorder is reused (`crates/engine-capture/src/lib.rs:1259-1267`). So pausing is cheap either way. [R] | Same. |

**Recommendation: segments.** Keep a small manifest per segment (first frame index, frame count, wall-clock start). Labels then map through cumulative frame count. [I] Use piped-only pause only for silent recordings. [I]

### Linux and macOS, briefly

- **Linux X11:** works. `rdev::listen` uses X11; AT-SPI and x11rb are already in the workspace, gated to Linux (`Cargo.toml:139-143`). [V 60, R]
- **Linux Wayland, global input:** `rdev::listen` "will not work in Wayland". `grab` uses evdev and is behind `unstable_grab`. [V 60] The GlobalShortcuts portal reports only registered shortcuts, so it is no general click or key log. [V 79, I]
- **Linux Wayland, screen capture:** goes through the ScreenCast portal with a user dialog and PipeWire streams. A restore token skips re-selection, is single-use, and must be refreshed. [V 80] xcap marks Wayland as not fully supported. [V 63] The repo already calls Wayland "the known gap" (`crates/engine-capture/src/lib.rs:13-14`). [R]
- **Linux ARM:** no separate blocker found for capture. Not tested. [U]
- **macOS:** three user-granted permissions. Accessibility: "The process needs to be granted access to the Accessibility API", and without it `listen` callbacks are silently ignored. [V 60] Input Monitoring. [V 81] Screen & System Audio Recording. [V 82] UI inspection needs the macOS accessibility APIs; I did not evaluate a crate for that. [U]

---

## 5. Local speech-to-text and local models

### Speech-to-text

| Option | Size | Accuracy | CPU speed | Timestamps | Licence and packaging |
|---|---|---|---|---|---|
| **whisper.cpp** v1.9.5, 2026-10-06 [V 85] | Base.en-q5_1 59.7 MB, small.en-q5_1 190 MB, large-v3-turbo-q5_0 574 MB. [V 86] RAM: base about 388 MB, small about 852 MB (unquantised). [V 84] | Depends on model. I did not open a primary Whisper figure; a snippet gave large-v3 about 7.2-7.4% WER. [U] | Encode time per chunk, i7-11800H (2022 build): base 1.48 s, small 4.83 s. Newer base.en on Ryzen 7 6800H 0.66 s. [V 87] Encode only, so whole-file speed is lower. [I] | Word-level through `-ml 1`, marked experimental. VAD through `--vad`. [V 84] | MIT. Plain C/C++, no dependencies. [V 84] Prebuilt `whisper-bin-x64.zip` and `whisper-bin-ubuntu-arm64.tar.gz` sit on `b`-tag pre-releases (b5454); the `v` tags carry no assets. [V 85] macOS binary not confirmed. [U] |
| **faster-whisper** | Not stated. [V 88] | Same Whisper models. [I] | Small model, 13 min audio, i7-12700K, 8 threads: whisper.cpp 2m05s, 1049 MB. faster-whisper int8 1m42s, 1477 MB. Int8 batch of 8, 51 s, 3608 MB. [V 88] | `word_timestamps=True`. [V 88] | MIT, but needs Python 3.9 or newer. [V 88] Standalone exes exist from a third party, x86-64 only, with no licence stated. [V 89] |
| **Parakeet TDT v3** through sherpa-onnx | Int8 encoder 622 MB, plus small decoder and joiner. [V 92] | 6.34% average WER, English, Open ASR. [V 93] 25 European languages. [V 93] | RTF 0.325 and 0.118 (v3 and v2) at 2 threads, CPU not named. [V 92] Fast relative to Whisper, per the leaderboard's general finding on TDT decoders. [V 94, I] | Word and segment level. [V 93] | Tool Apache-2.0, model CC-BY-4.0. [V 90, 93] sherpa-onnx v2.0.1, 2026-10-26. [V 91] CLI binary names for each OS not confirmed. [U] |
| **FFmpeg `whisper` filter** | Uses whisper.cpp models. | Same as whisper.cpp. [I] | Same engine. [I] | Output `text`, `srt` or `json`. [V 83] | Needs FFmpeg built with `--enable-whisper`. [V 83] The depot ffmpeg is 8.1.2 (`crates/engine-capture/src/lib.rs:1483-1485`). [R] **Whether that build has the filter is unverified.** [U] |

**Recommendation.** whisper.cpp `whisper-cli` as a depot subprocess, starting with `small.en-q5_1` (190 MB). [I] It matches ADR-009, adds no Python, and has Windows and Linux ARM binaries. If the depot's ffmpeg already has the filter, use it first, because it adds no new tool. [I] Try Parakeet later if accuracy disappoints. It is much larger on disk.

**Aligning speech to frames.** Extract audio with the existing ffmpeg and convert timestamps to frame numbers with `round(t × 30)`. [I] Speech gives "what they said"; it is not label truth.

### "A local AI reads the recording automatically"

**Runtimes**

- **Ollama:** MIT, a local service on `localhost:11434`. Images go base64 in an `images` array on `/api/chat`. [V 95, 96] Fits ADR-009 as a service behind a seam. [I] It needs installing separately. [V 95]
- **llama.cpp:** MIT. `llama-server` gives an OpenAI-compatible API, with a model plus an `mmproj` projector. Its list includes Gemma 3 and 4, SmolVLM, Qwen 2 and 2.5 VL, InternVL and Moondream2. [V 103, 104] Easier to stage in a depot, since it needs no installer. [I]

**Realistic models for a laptop** (download sizes)

| Model | Size | Source |
|---|---|---|
| `moondream` | 1.7 GB | [V 98] |
| `qwen3-vl:2b` | 1.9 GB | [V 100] |
| `qwen2.5vl:3b` | 3.2 GB | [V 99] |
| `qwen3-vl:4b` | 3.3 GB | [V 100] |
| `gemma3:4b` | 3.4 GB | [V 101] |
| `gemma4:e2b` | 4.6-7.5 GB | [V 102] |

Ollama's vision list changes weekly, so recheck it before choosing. [V 97] Check each model's licence: Gemma has its own terms [V 101], and the Ollama pages do not state a licence for Qwen. [V 99, 100]

**What they can do with screen frames**

Describe a screen, summarise a step, read clear text, return JSON. [V 99, 100; I] Qwen3-VL's page claims it can identify on-screen elements and operate interfaces. [V 100] That is a vendor claim.

**What they cannot do reliably**

- **Find small things on a desktop.** In the UI-Vision paper, the best overall model reached 25.5% on element grounding, spatial grounding peaked at 14.9%, and small general models scored about 0-3% (Qwen2.5-VL-7B at 0.85%). [V 105] A GUI-tuned 7B does better, at 52.9 on ScreenSpot-Pro and 60.3 on UI-Vision, but that is still far from exact. [V 107]
- **Keep up with video.** I found no measured CPU-only numbers, so I give none. [U] Expect seconds to minutes per image without a GPU, and test it. [I] Vendor numbers I saw are relative only (SmolVLM needs a 5.02 GB GPU minimum versus 13.70 GB for Qwen2-VL 2B). [V 108]
- **Be a source of truth.** Output is a guess, so it should be stored as a suggestion, never as a label. [I]

**Hardware.** Ollama's pages gave no RAM guidance. [V 95, 96, 102] My rule of thumb: 16 GB of RAM for 2-4B models, usable but slow on CPU; a GPU with 8 GB or more for comfortable use. [I, not sourced]

**The design that follows.** [I]

1. The hooks and UI Automation produce the facts: what was clicked, where, in which window, and when.
2. Pick key frames from those events, not by sampling video.
3. Send a few key frames, the UIA names and the transcript to the model. Ask for a title, step descriptions and a summary.
4. Keep generated text separate from recorded facts, and label it as generated.

---

## Suggested spikes (not done here)

1. **Overlay:** a tiny eframe window with `WS_EX_NOACTIVATE`. Tap it, including by touch, while a menu is open in another app. Does the menu stay open? Does the hook see the touch tap?
2. **Native clip:** 300 frames of plate, cursor, ripple and captions through `egui-wgpu` to ffmpeg, with one JSONL label row per frame. Measure fps, and diff two runs.
3. **Segments:** pause and resume with the mic on, then concat. Check drift and audio clicks at joins.
4. **`ffmpeg -filters`:** does the depot build list `whisper`?
5. **Vision model:** time `qwen3-vl:2b` and `moondream` on 10 real frames on the operator's laptop.
6. **UIA coverage:** run `element_from_point` over Chrome, VS Code, an Electron app, File Explorer and Excel.
7. **Only if polish falls short:** one Remotion composition, firewalled, to confirm offline use and measure real speed and size.

---

## Things I could not verify

- Whether Remotion validates input props at render time.
- Remotion's real render speed on this hardware.
- Remotion's offline behaviour, including `"free-license"` in 5.0.
- Chrome Headless Shell's exact size. The about 90 MB figure comes from a third-party crate's docs.
- Installed size of the full Remotion stack.
- Whether a safe crate for `WS_EX_NOACTIVATE` exists. I did not find one, which is not proof that none does.
- Whether the depot's ffmpeg has the whisper filter.
- CPU-only speed for local vision models.
- wgpu's fallback adapter docs (the page returned 404).

**Search snippets I saw but did not open, so nothing above relies on them as fact:** SourceForge Remotion mirror notes, third-party Remotion render-time claims, third-party Whisper WER summaries, third-party FFmpeg 8 coverage.

---

## Sources (all opened)

### Remotion

1. https://raw.githubusercontent.com/remotion-dev/remotion/main/LICENSE.md
2. https://www.remotion.dev/docs/license/faq (reached by redirect from remotion.pro/faq)
3. https://www.remotion.dev/docs/terms
4. https://www.remotion.pro/license
5. https://www.remotion.dev/docs/license/telemetry
6. https://github.com/remotion-dev/remotion/pull/3750
7. https://www.remotion.dev/docs/5-0-migration
8. https://registry.npmjs.org/remotion/latest
9. https://registry.npmjs.org/@remotion/renderer/latest
10. https://registry.npmjs.org/@remotion/bundler/latest
11. https://registry.npmjs.org/@remotion/compositor-win32-x64-msvc/latest
12. https://registry.npmjs.org/@remotion/compositor-linux-arm64-gnu/latest
13. https://www.remotion.dev/docs/renderer/render-media
14. https://www.remotion.dev/docs/cli/render
15. https://www.remotion.dev/docs/calculate-metadata
16. https://www.remotion.dev/docs/schemas
17. https://www.remotion.dev/docs/composition
18. https://raw.githubusercontent.com/remotion-dev/remotion/main/packages/core/src/resolve-video-config.ts
19. https://www.remotion.dev/docs/miscellaneous/chrome-headless-shell
20. https://raw.githubusercontent.com/remotion-dev/remotion/main/packages/renderer/src/browser/BrowserFetcher.ts
21. https://raw.githubusercontent.com/remotion-dev/remotion/main/packages/renderer/src/browser/get-chrome-download-url.ts
22. https://www.remotion.dev/docs/renderer/ensure-browser
23. https://www.remotion.dev/docs/options/binaries-directory
24. https://www.remotion.dev/docs
25. https://www.remotion.dev/docs/miscellaneous/linux-dependencies
26. https://www.remotion.dev/docs/ssr-node
27. https://www.remotion.dev/docs/using-randomness
28. https://www.remotion.dev/docs/performance
29. https://www.remotion.dev/docs/hardware-acceleration
30. https://www.remotion.dev/docs/brownfield
31. https://nodejs.org/dist/latest-v24.x/
32. https://docs.rs/crate/shirabe/latest/source/src/browser_fetch.rs
33. https://developer.chrome.com/blog/chrome-headless-shell

### HyperFrames, Revideo, Motion Canvas, ThorVG

34. https://github.com/heygen-com/hyperframes
35. https://hyperframes.heygen.com/concepts/determinism
36. https://github.com/heygen-com/hyperframes/issues/294
37. https://registry.npmjs.org/hyperframes/latest
38. https://api.github.com/repos/heygen-com/hyperframes
39. https://github.com/redotvideo/revideo (displayed as midrender/revideo)
40. https://api.github.com/repos/midrender/revideo
41. https://registry.npmjs.org/@revideo/core/latest
42. https://github.com/motion-canvas/motion-canvas
43. https://api.github.com/repos/motion-canvas/motion-canvas
44. https://registry.npmjs.org/@motion-canvas/core/latest
45. https://motioncanvas.io/docs/rendering
46. https://github.com/thorvg/thorvg
47. https://api.github.com/repos/thorvg/thorvg/releases/latest

### egui and wgpu

48. https://docs.rs/egui/latest/egui/struct.RawInput.html
49. https://docs.rs/egui_kittest/latest/egui_kittest/
50. https://docs.rs/egui_kittest/latest/egui_kittest/struct.Harness.html
51. https://docs.rs/egui_kittest/latest/egui_kittest/struct.Node.html
52. https://docs.rs/egui_kittest/latest/egui_kittest/struct.HarnessBuilder.html
53. https://crates.io/api/v1/crates/egui_software_backend
54. https://sotrh.github.io/learn-wgpu/showcase/windowless/

### Capture and input

55. https://crates.io/api/v1/crates/uiautomation
56. https://docs.rs/uiautomation/latest/uiautomation/
57. https://docs.rs/uiautomation/latest/uiautomation/core/struct.UIAutomation.html
58. https://docs.rs/uiautomation/latest/uiautomation/core/struct.UIElement.html
59. https://crates.io/api/v1/crates/rdev
60. https://github.com/Narsil/rdev
61. https://raw.githubusercontent.com/Narsil/rdev/main/src/windows/listen.rs
62. https://raw.githubusercontent.com/Narsil/rdev/main/src/windows/common.rs
63. https://github.com/nashaofu/xcap
64. https://crates.io/api/v1/crates/xcap
65. https://crates.io/api/v1/crates/active-win-pos-rs
66. https://crates.io/api/v1/crates/wineventhook
67. https://docs.rs/wineventhook/latest/wineventhook/
68. https://crates.io/api/v1/crates/winsafe
69. https://docs.rs/winsafe/latest/winsafe/struct.HWND.html
70. https://crates.io/api/v1/crates/windows
71. https://docs.rs/winit/latest/winit/platform/windows/trait.WindowAttributesExtWindows.html
72. https://docs.rs/egui/latest/egui/viewport/struct.ViewportBuilder.html
73. https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles
74. https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-threading
75. https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelmouseproc
76. https://learn.microsoft.com/en-us/windows/win32/winauto/event-constants
77. https://github.com/microsoft/terminal/issues/17168
78. https://www.ffmpeg.org/ffmpeg-formats.html
79. https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html
80. https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.ScreenCast.html
81. https://support.apple.com/guide/mac-help/control-access-to-input-monitoring-on-mac-mchl4cedafb6/mac
82. https://support.apple.com/guide/mac-help/control-access-to-screen-and-system-audio-recording-mchld6aa7d23/mac

### Speech

83. https://raw.githubusercontent.com/FFmpeg/FFmpeg/master/doc/filters.texi
84. https://github.com/ggml-org/whisper.cpp
85. https://api.github.com/repos/ggml-org/whisper.cpp/releases/latest and https://api.github.com/repos/ggml-org/whisper.cpp/releases?per_page=6
86. https://huggingface.co/ggerganov/whisper.cpp/tree/main
87. https://github.com/ggml-org/whisper.cpp/issues/89
88. https://github.com/SYSTRAN/faster-whisper
89. https://github.com/Purfview/whisper-standalone-win
90. https://github.com/k2-fsa/sherpa-onnx
91. https://api.github.com/repos/k2-fsa/sherpa-onnx/releases/latest
92. https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-transducer/nemo-transducer-models.html
93. https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3
94. https://arxiv.org/abs/2510.06961

### Local vision and LLM

95. https://github.com/ollama/ollama
96. https://docs.ollama.com/capabilities/vision
97. https://ollama.com/search?c=vision
98. https://ollama.com/library/moondream
99. https://ollama.com/library/qwen2.5vl
100. https://ollama.com/library/qwen3-vl
101. https://ollama.com/library/gemma3
102. https://ollama.com/library/gemma4
103. https://raw.githubusercontent.com/ggml-org/llama.cpp/master/docs/multimodal.md
104. https://api.github.com/repos/ggml-org/llama.cpp
105. https://arxiv.org/html/2503.15661
106. https://arxiv.org/abs/2507.05673
107. https://huggingface.co/ServiceNow/GroundNext-7B-V0
108. https://huggingface.co/blog/smolvlm

### Windows privilege isolation

109. https://learn.microsoft.com/en-us/answers/questions/2262996/how-can-i-tell-in-c-or-vb-net-if-a-low-level-hook
110. https://learn.microsoft.com/en-us/previous-versions/windows/it-pro/windows-10/security/threat-protection/security-policy-settings/user-account-control-only-elevate-uiaccess-applications-that-are-installed-in-secure-locations

### Repo files read (`D:\projects\devpulselabs`)

`CLAUDE.md`, `docs/design/architecture.md`, `crates/engine-capture/src/lib.rs`, `crates/engine-capture/Cargo.toml`, `crates/engine-steps/src/lib.rs`, `crates/engine-steps/Cargo.toml`, `Cargo.toml` (lines 80-149). I also grepped every `Cargo.toml` and `Cargo.lock` for the crates named above. I did not read `/content` or `.userdata`.
