# 2026-11-03 — recording for AI: the ask, the rulings, the ground truth

Handoff for the program after ST3. The quotes below move into
`docs/requirements.md` as R294 when the plan's PR opens; until then this file
is their safe copy.

## The operator's words (verbatim)

> so this needs to be consistent where we are truly building this so the complexity is handled so the Ui makes it quality of life easy.  We clearly get the overlay indication so we can click record on different apps pause resume and then be able to clean up at the end have it fed to AI to develop a new tailored workflow/training data

> you understand that I want actual recordings, that are tuned and created to feed and extend the AIs capabilities and features

> I TOLD YOU CUSTOM MADE REMOTION CLIPS THAT HAVE IT RECREATE THE EVENTS IN MACHINE CODE SCRIPTED EVENTS WE HAVE IT DEVELOP ACTUAL CONNNECTS, SKILLS ROUTINES WE DO THis aligned to real research we have it built so it is tailore to dynamically generate the files and the data video

## Rulings (AskUserQuestion, 2026-11-03)

- **When:** "Next, right after ST3". Ahead of ST4 and AgentHub.
- **What one recording holds:** "Screen video", "App timeline and clicks",
  "Your voice, transcribed", "What you type", plus the custom-clips quote above.
- **What comes out:** "Skills your AIs load", "WiredFRONT workflows", "Training
  data export", "New app features, proposed".
- **Before a recording reaches an AI:** "Local AI automatically, cloud on my
  tap".

## What the words mean (from the operator's own earlier messages)

**"Machine code" and "scripted events" mean deterministic code doing the work so
AI doesn't have to.** They do not mean CPU instructions.
- 2026-09-23: "we offset the costs by having scripted events so the AI just
  neecs to know oh if i run this and then generate the raw json…"
- 2026-10-10: "all the unspoken that is robust machine code that ensures
  functionality"; "dynamically triggered via machine code that will appear as AI
  but actually leverage the codebase as a vectorDB … once your project gets to a
  decent size that becomes your LLM and then frontierAI just has to write the
  new code and fill the gaps".

**Remotion:** no Claude Code message on this machine mentions it before today,
though the operator says it was told before. Taken from today's answer. The only
repo mention is `docs/research/lesson-slide-schema.md:256`, on schema-driven
props. The plan must confirm: Remotion itself, or clips made the Remotion way?
It is Node and a headless browser; the app is pure Rust and egui, and web
material is otherwise refused.

## The working reading, to confirm in the plan

1. A real recording: screen, app timeline, clicks, typing, voice.
2. It is cleaned up, then distilled into a **scripted event timeline**: the
   recording as code, deterministic and re-runnable.
3. That script drives **generated clips** that re-enact the events. Every frame
   has exact labels because the script made it, and it can be varied (timing,
   theme, data) to make more data. Private content can be left out by
   construction.
4. Out of that come connectors, skills and routines the AIs use, WiredFRONT
   workflows, a training dataset, and proposed features.

## Ground truth (read 2026-11-03, `main` capture crates)

- **Three recorders:** Video (`viziwiz/src/lib.rs:884` →
  `engine-capture/src/lib.rs:1434`), Session (`viziwiz/src/session.rs:195`) and
  Steps (`engine-steps/src/lib.rs`).
- **Across apps:** only Steps follows focus. Video locks its crop at start
  (`engine-capture/src/lib.rs:1454-1478`). Session is primary monitor only.
- **Overlay:** the palette is always on top (`viziwiz/src/lib.rs:1240`). Only
  Steps has a REC pill (`viziwiz/src/palette.rs:669`). Closing the palette hides
  Video's and Session's controls while they keep recording
  (`viziwiz/src/lib.rs:1225`). No non-activating window yet: clicking Mark likely
  takes focus from the app being recorded (inferred, not proven;
  `docs/plans/port-viziwiz.md:362` still says VERIFY).
- **Pause and resume:** none, anywhere.
- **Clean up:** four separate places, no redaction. Save card
  (`viziwiz/src/palette.rs:577`), Composer's Guide tab
  (`composer/src/lib.rs:5725`), comp timeline trim, gallery Dump.
- **On disk:** `guide.json` has screenshots, action, click and title but no
  timestamps or typed text (`engine-steps/src/lib.rs:155-168`). Session has an
  mp4 and `events.jsonl` of this app's own events only. No transcription exists.
- **To AI:** no path. Nothing takes a capture as input; `tailor()` takes a bible
  (`workflows/src/lib.rs:503`) and has test callers only. "Training data" is not
  in the code.
- **Built twice:** three REC displays, three step structs, two `parse_events`.
- **Plans:** every AI plan has AI driving this app. None has the operator
  recording other apps for AI to learn from.
- **Consistency:** the operator's point. One recorder in the overlay, in the
  existing capture crates, replacing the three.

## Parked ST3, and the operator's next ask (2026-11-03)

The operator called the library move a distraction and asked to compare Python
and Rust for training and "get this actually producing results". ST3 is parked
unmerged on `feat/portable-storage-st3` (`23f526f`), to land or drop on their
word. This work is on `plan/recording-for-ai`.

## Measured on this machine (2026-11-04)

Scratch only: `D:\.usr\Temp\claude\d--localdev-devpulselabs\d0ec6155-b7fb-480f-b3ea-5b7ca10f26c7\scratchpad\ml-bench\`.
RTX 5090 Laptop, 24 GB VRAM, 63 GB RAM.

- **The global Python's torch is CPU-only** (`2.14.1+cpu`). A separate venv in
  the scratch folder has `2.14.1+cu130`. The global Python was left alone.
- **PyTorch vs Burn,** same network, bytes, batch order and optimizer, three
  seeds, on a clean GPU:
  - PyTorch 2.14.1 on CUDA: 66.8–70.9 steps/s, final MSE 0.0049–0.0051, 805 MB.
  - Burn 0.21 on Vulkan with `fusion` and `autotune`: 5.8 steps/s, final MSE
    0.0049–0.0058. About 12× slower, and it learned about as well.
  - Without `fusion` and `autotune`, Burn ran at 2 steps/s. They are opt-in.
  - Burn's CUDA backend needs `cuda_runtime.h` from NVIDIA's full toolkit
    (`CUDA_PATH`), so it is untested here.
- **The dataset is the machine-code idea working.** EguiPilotAI's semantic tree
  labels real frames: 96 frames, 16 screens, 6 window sizes, 1,251 labelled
  controls, 59 distinct names, no failures, no AI and no hand labelling. Labels
  were checked by drawing them on a frame. Limits: an empty library, and many
  screens only tag their 10 shared-menu controls.
- **Qwen3.5-2B, LoRA rank 16, 3,000 steps, 99 minutes, 7.7 GB peak.** Hit means
  the answer landed inside the real rectangle:
  - same screens at an unseen window size, 150 questions: 21.3% → 94.7%, median
    miss 140 → 3.9 px;
  - unseen screens, 126 questions: 23.8% → 100%, median miss 330 → 0.5 px.
    **Mostly memorized shared menus; not evidence of generalizing.**
  - The only truly unseen control was `+ New lesson`, 6 questions, already 100%
    before training.
  - The tuned model repeats its answer instead of stopping, so these "after"
    numbers need the one-line re-score.
- **What this proves:** it learned this app's layout. **Not yet shown:** finding
  a control it has never seen, or working in another app.
- **Re-scored, reading one line:** the same numbers held.
- **Never-seen controls** (15 of 49 withheld, 1,200 steps, 47 minutes): 37.9% →
  90.8%, median miss 68 → 4.6 px, 87 questions.
- **Foreign-look pages** (24 generated pages, 288 questions, that model): 58.3% →
  100%, median miss 33 → 4.7 px. **An easy test,** not evidence about real apps.
- All of this, with its caveats, is in
  `docs/research/recording-for-ai/training-python-vs-rust.md`.

## Research digests

`scratchpad\research-demonstrations.md` (71 sources) and
`scratchpad\research-remotion-capture.md`, both in the folder above's parent.
They go into `docs/research/` with the plan. The second recommends a native clip
compositor over Remotion. **The operator named Remotion, so that is their call.**

## Approved 2026-11-05, and the exact next step

- **The plan:** `docs/plans/recording-for-ai.md`, approved with RA1, on PR #312.
- **Rulings:** native renderer; tuning in Python behind one command; §5b signed off
  and configurable; the whole monitor; keep the old readers; hide the overlay from
  recordings, with a tiny crate. ST3 stays parked.
- **Next:** land #312 on double-green (it's docs only), then **RA1a**, the format and
  engine (§4.6). Branch `feat/recording-ra1a` from main.
  - First, read `crates/engine-capture/src/lib.rs:270-420,1280-1440`,
    `crates/viziwiz/src/session.rs` and `crates/index/src/lib.rs:470-500,840-900`.
  - Then write the v1 format under the document law
    (`docs/design/architecture.md:149-150`).
- **RA1a must fix, not inherit:** the non-recursive rescan that hides organized
  captures, and the frozen-video loop that saves a dead stream as fine (§4.6).

### RA1a progress, on `feat/recording-ra1a` (from main `95a6516`)

1. **Done:** Rescan walks the organized inbox, `-poster.jpg` rides its video, and
   an unsaved guide stays out (`index/src/lib.rs`, a new test, 24 index tests).
2. **Done:** an ended screen capture stops the recording and says where, and the
   dead recorder slot is dropped (`pull_latest`, `Recorded::capture_lost_at` and
   `lost_note`, a new test).
3. **Done:** `engine-capture/src/recording.rs`: `Recording` (state and frame clock),
   `Manifest` (v1, saved after each segment), and `record()`. `record_monitor_ctl`
   takes a keep-going check and an optional clock. The index knows a recording folder.
   `capture-cli record <dir> <s> [<pause> <resume>]` takes `FFMPEG=<depot ffmpeg>`.
   - The real-screen proof was operator-approved (screen only, deleted after):
     segments 0–47 and 47–107, the clock held through the pause.
   - About 0.43 s of startup isn't captured.
4. **Landed:** #313 merged as `fd61108` on double-green (Linux 14m57s, Windows
   19m17s). The release exe was rebuilt from it (stamp `fd61108 · 2026-11-06 08:45z`).
   RA1b is branched from it, as `feat/recording-ra1b`.

**Next: RA1b, the overlay** (§4.6). First read `viziwiz/src/lib.rs:1225-1320`
(the palette viewport and the Steps pill), `palette.rs:325-349,443-470,669`, and
`xtask/src/main.rs:1493,1626` (`EGUI_FREE`, `LINTS_OVERRIDE_ALLOWED`). Model the new
leaf crate on `crates/lottie-render`. Its two Windows calls are `WS_EX_NOACTIVATE`
and `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)`; whether the second hides
the pill from xcap's capture is the open question, proven with `capture-cli`.

**RA1b grounding (2026-11-06), simpler than planned:**
- **Capture exclusion needs no new unsafe.** egui has
  `ViewportCommand::ContentProtected(bool)` (`egui-winit-0.36.1/src/lib.rs:1914`).
  winit 0.30.13 implements it as `SetWindowDisplayAffinity(hwnd,
  WDA_EXCLUDEFROMCAPTURE)` (`winit/src/platform_impl/windows/window.rs:1029`). Send
  it to the pill's viewport. Prove xcap honours it with `capture-cli`.
- **No-activate keeps ADR-009's subprocess model.** winit doesn't expose
  `WS_EX_NOACTIVATE` for app windows (only internally). Windows allows
  `SetWindowLongPtrW(GWL_EXSTYLE)` from another process at the same integrity level;
  only `GWLP_WNDPROC` is refused across processes, and SetWindowDisplayAffinity is
  the call that must come from the owner.
  - So the tiny crate can be a spawned helper like `dpl-lottie-render`: it finds the
    pill by process id and title (`EnumWindows`) and adds `WS_EX_NOACTIVATE` with
    `SWP_FRAMECHANGED`. No unsafe in the app process.
  - Verify it on a real window before relying on it.
- **Still owed in RA1b:** the hit-test (`engine-capture/src/lib.rs:854`), so taps
  on the pill aren't logged as clicks in the recorded app.

The release exe was rebuilt from main `95a6516` on 2026-11-05 (stamp
`95a6516 · 2026-11-05 23:40z`).

### RA1a design notes, from reading `engine-capture/src/lib.rs:1275-1430`

- **Frozen video, the cause.** `record_monitor_ctl` reads frames with
  `while let Ok(f) = rx.try_recv()` (`:1392`) and `recv_timeout` (`:1396`), so it never
  sees `Disconnected` when xcap's capture thread dies. It keeps writing `last` forever.
  - **Fix:** on `Disconnected`, end the segment, evict that monitor's `RECORDERS` slot
    (`:1348`) so the next recording recreates it, and report "the screen capture
    stopped at mm:ss; what was recorded until then is kept".
  - An idle screen legitimately sends no frames (WGC is change-driven), so only
    `Disconnected` counts as dead, never silence.
- **Segments.** Each segment is one ffmpeg process with its own audio input:
  `finish_recording` (`:273`) per segment.
  - Pause: drop stdin, finish, record the segment. Resume: spawn the next, rebase
    `start` and `tick` (`:1378-1379`), and drain the channel first (`:1358`).
  - A segment with zero frames is dropped, not an error (`:285`).
- **One clock.** One recorder-owned frame counter runs across segments. Every event
  records `frame` plus `ms` on the recorder's clock, never the UI drain time
  (`viziwiz/src/session.rs:83`).
- **Format v1** (a new folder kind, under the document law):
  - `recording.json`: `v: 1`, `kind: "recording"`, the monitor (id, width, height),
    30 fps, streams on or off, and `segments[]`, each with file, first frame, frames,
    start ms, seconds and audio.
  - `segment-NNN.mp4`, `events.jsonl` and `poster.jpg` beside it.
  - The index's `folder_kind` (`index/src/lib.rs:851`) learns it.
- **The rescan fix** lands first, with a test, since organized captures already
  vanish: `scan_inbox` (`index/src/lib.rs:878-891`) walks every bucket, and reindex
  (`:480`) clears only rows whose file is really gone.

## Earlier next step (superseded)

Land ST3 only on the operator's word. Then: the research digests in
`docs/research/recording-for-ai.md`, the program plan
`docs/plans/recording-for-ai.md`, R294 in `docs/requirements.md`, and a draft PR
so the operator can read the plan on GitHub before approving.

## Landmines

- **Windows global input and window tracking** may need FFI, which the workspace
  denies outside `dpl-lottie-render`. That is an architecture question for the
  operator, not a quiet workaround.
- **New dependencies** (speech-to-text, Node and Remotion, a local model) are
  external reach and need sign-off (`docs/plans/external-reach-inventory.md`).
- **The repo is public.** Recordings and training data must never land anywhere
  git tracks.
- **Remotion's licence** applies to companies above a size; check it.
