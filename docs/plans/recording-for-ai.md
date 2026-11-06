# Recording for AI — the program plan (RA)

**Status: DRAFT, for the operator's approval.** R294. Rulings (2026-11-03): "Next,
right after ST3"; a recording holds screen video, app timeline and clicks,
transcribed voice and typing; out come skills, WiredFRONT workflows, training data
and proposed features; "Local AI automatically, cloud on my tap".

**Rulings 2026-11-04:**
- Q-RA-1, the clip renderer: "Native renderer".
- Q-RA-2, where training runs: the operator asked for the better move, and would do
  the Rust work if it paid more. **Python, behind one command** (§4.5).
- §5b's four items: signed off, and "configurable" (§4.3).
- ST3: "Leave it parked for now".

**APPROVED 2026-11-05,** with RA1 (§4.6), "Approve, start RA1a". Rulings:
- Q-RA-3, what the video holds across apps: "The whole monitor".
- Q-RA-4, old captures once the old recorders go: "Keep the readers".
- Q-RA-5, the overlay: "Hide it from recordings, tiny crate". One leaf crate holds
  both Windows calls (don't take focus; leave the overlay out of capture), as
  `dpl-lottie-render` holds its FFI. That bends §4.2's "no new product crate", by
  ruling. Whether capture exclusion works with this capture method is proven in
  RA1b, not assumed.

**Evidence behind it:**
[`research/recording-for-ai/training-python-vs-rust.md`](../research/recording-for-ai/training-python-vs-rust.md)
(measured here), and the
[demonstrations](../research/recording-for-ai/demonstrations-to-skills.md) and
[clips and capture](../research/recording-for-ai/clips-and-capture.md) digests.

## 1. Context

The operator wants to press record in an overlay, work across real apps, pause
and resume, clean it up, and have that recording **feed and extend what the AIs
can do**. The recording isn't the product; it is raw input. It becomes
machine-scripted events (R294: deterministic code, not CPU instructions), which
drive generated clips that re-create what happened, and from those come real
connectors, skills and routines.

Today the app has three separate recorders and no path from a recording to any
AI. Every existing AI plan has AI driving this app; none has the operator
teaching AI by recording other apps.

## 2. Goal and non-goals

**Goal:** one recorder with one flow, where finishing a recording measurably gives
an AI a capability it didn't have, shown by a score rather than claimed.

**Non-goals:**
- Sending anything to a cloud AI without the operator's tap.
- Recording anyone without their knowledge: the overlay always shows it.
- Generating a skill per recording by default. Evidence says untested generated
  skills don't help (§7).
- Training a model from scratch. Tuning open models is what's proven here.

## 3. Ground truth (read 2026-11-03)

- **Three recorders:** Video (`viziwiz/src/lib.rs:884` →
  `engine-capture/src/lib.rs:1434`), Session (`viziwiz/src/session.rs:195`) and
  Steps (`engine-steps/src/lib.rs`).
- **Across apps:** only Steps follows focus. Video locks its crop at start
  (`engine-capture/src/lib.rs:1454-1478`). Session records the primary monitor.
- **Overlay:** only Steps has a REC pill (`viziwiz/src/palette.rs:669`). Closing
  the palette hides Video's and Session's controls while they keep recording
  (`viziwiz/src/lib.rs:1225`).
- **Pause and resume:** none. ffmpeg gets piped frames and a stop flag
  (`engine-capture/src/lib.rs:1361-1391`).
- **Clean up:** four separate places, no redaction.
- **On disk:** `guide.json` has no timestamps or typed text
  (`engine-steps/src/lib.rs:155-168`). Session's `events.jsonl` holds this app's
  events only. No transcription exists.
- **To AI:** no path. `tailor()` takes a bible and has test callers only
  (`workflows/src/lib.rs:503`).
- **Built twice:** three REC displays, three step structs, two `parse_events`.

**Measured 2026-11-04:** labels from EguiPilotAI's semantic tree trained a local
vision model from 37.9% to 90.8% on controls it never saw. Details and limits are
in the training doc. Real third-party apps are unmeasured.

## 4. Design

### 4.1 The flow

```
Overlay: record, pause, resume, stop
   │  one recording: video, app timeline, clicks + what they hit, typing, voice
   ▼
Clean up: one page. Trim, delete parts, blur. Nothing has left this machine.
   ▼
Local AI reads it (automatic). Cloud AI only on a tap, after review.
   ▼
Scripted events: the recording as code. Replays with no AI; AI only fills gaps.
   ├─▶ Generated clips: re-create the events, exact labels on every frame
   ├─▶ Dataset and tuning: a model that scores better, or it doesn't ship
   ├─▶ Skills and connectors the AIs load, tested by replay first
   ├─▶ WiredFRONT workflows
   └─▶ Proposed features, for the operator to approve
```

### 4.2 Consistency: where this is built

No new product crate. **One recorder replaces the three**, in the existing
capture boundary (`engine-capture`, `engine-steps`, `viziwiz`), so there is one
REC display, one step format and one events format. The review page lives in
WiredFRONT. Training is a depot tool, never a crate (the ADR-009 pattern).

### 4.3 What a recording records

**Structure comes first.** Published tests found video alone the weakest input,
and removing images, UI annotations or key logs each cost 14–22 points of task
success (Work2Skill).

- One timeline, anchored to video frames, with pauses producing no frames.
- For each action: the frame **before the mouse moved**, so no future leaks in.
- A **selector bundle** per click, from UI Automation where it exists: id, type,
  name, parent path, process, window class, box, and a small image crop.
- The alignment error is **measured** by injecting a known click. No published work
  reports it.

**Privacy is prevention first.** Redaction misses things (the best published
screenshot detector reached 0.753 mAP). So: per-app "never record" rules, password
fields masked via UI Automation's password flag, and a plain off switch for
typing. Then blur on review. Recordings live in the library, which git never
tracks.

**Configurable (ruled 2026-11-04).** Each stream can be turned on or off (video,
clicks, typing, voice), with defaults the operator sets. The speech model and the
model being tuned are chosen, not hard-coded.

### 4.4 Passes (each one PR, double-green, real-exe proof)

| Pass | What you get | Done means |
|---|---|---|
| **RA1 One recorder** | Overlay record, pause, resume, stop, across apps, always showing it's on. Format v1. | A recording across two real apps, paused and resumed, opens with all streams aligned. The old recorders are migrated and deleted. |
| **RA2 Clean up** | One review page: trim, delete parts, blur. | A password typed mid-recording never reaches disk. |
| **RA3 First real score** | Recording to dataset to tuned local model, as a depot tool, with a before/after score in the app. | A score on **real third-party apps**, labelled from UI Automation, on questions never trained on. |
| **RA4 Scripted events** | Recording to replayable script: selectors, parameters, checks. AI fills gaps only. | A recorded task replays with no AI, and verifies by reading state back rather than trusting the screen. |
| **RA5 Generated clips** | The script drives clips from the native renderer (Q-RA-1), with exact labels per frame. | Used as test fixtures first. They train only if they lift the score on held-out real recordings. |
| **RA6 What the AIs get** | Skills (`.agents/skills` and `.claude/skills`), connectors, workflows, proposed features. | Each skill passes a replay test before it ships, and an agent finishes more tasks with it than without. |

RA3 comes before the scripted events and clips because it produces a number
first, and every later pass is judged by that number.

### 4.5 Python for tuning, Rust for everything else (Q-RA-2)

The operator offered to do the Rust work if it paid more. It doesn't, for tuning:

- **Rust's payoff is a small, self-reliant install** (R289). That comes from
  *running* the tuned model in Rust, which doesn't need a training stack.
- **Tuning in Rust means porting** each new model's architecture and tuning layer
  by hand, indefinitely, in a field that keeps releasing models.
- **Measured:** about 12× slower on one small model, on Vulkan. Burn on CUDA is
  unmeasured.

So tuning is a replaceable tool behind one command: dataset in, adapter and score
out. The app knows nothing about Python, so Burn can take over later if it earns it.

**Unverified:** running a tuned Qwen3.5 from Rust (a merged model through a local
runtime or Candle). RA3 checks this before relying on it.

### 4.6 RA1 execution queue, grounded 2026-11-05

Read on `plan/recording-for-ai`, by a census of the capture crates and everything
that consumes their output. Paths are under `crates/`.

What the reading found:

- **Keepable:** the frame loop, `frame_deadline` and `finish_recording`
  (`engine-capture/src/lib.rs:304,273`), the one `rdev` hook and chords
  (`engine-steps/src/lib.rs:216,545`), `EventLog`, the `land_*` policy.
- **Duplicated:** REC displays (`viziwiz/src/palette.rs:329,447,677`), `parse_events`
  (`guide/src/lib.rs:166`, `hubs/wiredfront/src/lib.rs:184`), step structs
  (`engine-steps/src/lib.rs:155`, `edit.rs:27`, `guide/src/lib.rs:31`).
- **Pause needs segments.** Video time is frames ÷ 30, so not feeding frames leaves
  no video gap, but the mic is a second live input in that ffmpeg
  (`engine-capture/src/lib.rs:1309,1320-1334`) and keeps recording speech the
  operator thinks is paused. So pause finishes the process, and resume spawns
  another. A zero-frame segment errors today (`:285`), so drop it instead.
- **The clocks don't agree.** Event `ms` starts before ffmpeg
  (`viziwiz/src/session.rs:41`) and stamps at UI drain, ignoring `Stamped.ms` (`:83`).
  v1 needs one recorder-owned frame counter. Nothing measures the audio start offset.
- **Typing needs no new crate:** `rdev` already computes each key's name. Only
  `uiautomation` is new.
- **Consumers to keep working:** the index's `folder_kind` (`index/src/lib.rs:851`),
  the gallery (`hubs/wiredfront/src/lib.rs:895-908`), the session player (`:184,980`),
  Composer's Guide review (`composer/src/lib.rs:2003-2017,6055`), `--export-guide` and
  DG1 (`guide/src/lib.rs:470,495`), and four drills. Removing a flag fails the census
  unless `CLAUDE.md` changes too (`xtask/src/main.rs:1737`).

Bugs this must fix, not inherit:

- **Rescan hides organized captures.** `scan_inbox` isn't recursive
  (`index/src/lib.rs:878-891`), reindex soft-clears unseen inbox rows (`:480`), and
  every bucket lands in a subfolder (`landing/src/lib.rs:187`). The files survive,
  but their rows don't come back.
- **A frozen video is reported as saved.** Any non-timeout DXGI error ends xcap's
  thread, the loop keeps writing the last frame (`engine-capture/src/lib.rs:1392-1409`),
  and the cached slot stays dead until restart (`:1348`). UAC or a lock screen likely
  triggers it (untested). Over a long recording across apps, that would silently waste
  the recording.
- **`update_guide` loses data:** it drops `session`, `guideVideo` and `replayGif` from
  exported guides and writes non-atomically (`engine-steps/src/edit.rs:146-157`).

**RA1a: the format and engine** (one PR). Format v1 under the document law,
segments, one clock, a stopped-frames alarm, the rescan fix. **Proves:** a paused
recording opens aligned on a scratch root.

**RA1b: the overlay** (one PR). One pill with timer, pause, resume and stop, the
non-activating window, and a hit-test so overlay taps aren't logged as clicks (with
focus staying put, `engine-capture/src/lib.rs:854` stops filtering them). The new
leaf crate (Q-RA-5) holds the two Windows calls; `architecture.md` §1, §3 and §4 and
the audit's `LINTS_OVERRIDE_ALLOWED` and `EGUI_FREE` change in the same commit
(`xtask/src/main.rs:1626,1493`). **Proves:** focus stays on the recorded app, and the
pill is absent from the recorded frames, at both window sizes.

**RA1c: the streams** (one PR). App timeline, clicks with selector bundles, voice,
and typing **with its masks in the same PR**: typing doesn't ship before the masks,
which moves them up from RA2. **Proves:** the injected-click alignment error, and a
password never reaches disk.

**RA1d: one recorder** (one PR). Consumers move to v1; the old tabs, drills, flags and
duplicates go; `CLAUDE.md` changes with them. The readers of old guides and sessions
stay (Q-RA-4). **Proves:** old captures still open.

Recording is the **whole monitor** (Q-RA-3): one monitor, chosen when recording
starts. The app timeline lets cleanup and the dataset crop to each app later.

## 5. Wiring

| Where | Change | Pass |
|---|---|---|
| `engine-capture`, `engine-steps`, `viziwiz` | one recorder, format v1, pause and resume; the old three deleted | RA1 |
| `viziwiz` | the non-activating overlay, with a hit-test so overlay taps aren't logged as clicks | RA1 |
| `hubs/wiredfront` | the review page, the score | RA2, RA3 |
| depot (`dptools-deps`) | whisper.cpp; the training tool and its Python runtime | RA1, RA3 |
| `workflows` | a capture as input to `tailor()` | RA6 |

### 5b. Reach outside the repo, for sign-off

| What | Why | Proposed |
|---|---|---|
| One FFI exception for `WS_EX_NOACTIVATE`, named in `architecture.md` | so tapping the overlay doesn't steal focus from the app being recorded; no safe crate does this | **Ruled** 2026-11-04 |
| Safe-API crates for capture and input (`xcap`, `rdev`, `uiautomation`) | window following, global clicks and keys, what was clicked | **Ruled** 2026-11-04 |
| Global key capture | it can see passwords; hence §4.3's prevention and off switch | **Ruled** 2026-11-04 |
| whisper.cpp in the depot, starting with `small.en-q5_1` (190 MB) | local transcription; first check if the depot's ffmpeg has the `whisper` filter | **Ruled** 2026-11-04 |
| A Python training runtime in the depot (about 4 GB) | tuning; Rust has no equivalent | **Ruled** 2026-11-04 (Q-RA-2) |
| Model downloads from Hugging Face (Qwen3.5-2B, Apache-2.0, 4.5 GB) | the model being tuned | **Ruled** 2026-11-04 |

## 6. Proof

- **Unit:** the format, alignment, pause and resume, masks.
- **EguiPilotAI on the real exe**, at 1440×900 and 1100×700.
- **Real recording of real apps**, on a scratch library, only with the operator's
  consent, since it captures their desktop. A disposable app under the test's
  control comes first.
- **The score** is the proof for RA3 onward: held-out questions, before and after,
  with the raw results committed.

## 7. Risks

- **Clips may not help training.** GUI-Perturbed found random variation gave no
  average lift and more samples made it worse (one model, low-rank tune). Targeted
  synthesis gained about 11 points (LearnWeak). Hence fixtures first, and window
  size and zoom before themes.
- **Generated skills can hurt.** Curated skills gained 16.2 points and self-written
  ones lost 1.3 (SkillsBench, terminal tasks). Work2Skill's demonstration-induced
  skills did help on GUI tasks. Hence replay tests before shipping.
- **AI repairs can cheat.** One case study documents weakened assertions and deleted
  tests. Any repair is a proposed diff that can't touch its own checks.
- **The numbers so far may not reach real apps.** RA3 exists to find that out early.
- **Dropped narration value.** No study measures spoken narration. Measure it: skill
  tests with narration on and off.
- **Windows evidence is thin.** Most benchmarks are Linux or web. UI Automation ids
  aren't guaranteed stable, per Microsoft, hence selector bundles.
- **Training time.** 47 to 99 minutes per run on slow kernels; the recording flow
  must not wait on it.

## 8. Evidence

The spike's numbers and scripts are in
[`research/recording-for-ai/`](../research/recording-for-ai/training-python-vs-rust.md).
Pass evidence lands here as each one ships.

**RA1a, 2026-11-06:**
- **Unit:**
  - `dpl-index` has 24 tests, 1 new: organized captures, `-poster.jpg`, an unsaved
    guide, a recording folder as one item, and nothing cleared on a second rescan.
  - `dpl-engine-capture` has 13 tests, 4 new: an ended capture against an idle
    screen, where it stopped, and the pause, resume and stop states. The clock is
    the video timeline, and the description round-trips and refuses a newer one.
- **Real screen,** approved by the operator for this run (screen only, deleted
  after). `capture-cli record` on the 2560×1440 primary monitor, 6 s, paused 2–4 s:
  - two segments, frames 0–47 (1.567 s) and 47–107 (2.000 s);
  - the clock held at frame 47 through the pause;
  - each segment's video length matched its frame count.
  - **Found:** the first segment starts about 0.43 s after Record, which is how long
    the capture and ffmpeg take to start. The timeline stays exact, but the first
    moment isn't captured. RA1b's pill can show that it's getting ready.
- **Not yet proven:** the microphone across a pause (screen-only run), and an ended
  capture on real hardware (a lock screen mid-recording).

## Why the native renderer (Q-RA-1)

The operator named Remotion, then chose native on the research. Only re-enactment
gives exact labels. Neither renderer can redraw another company's app, so both
animate over real screenshots. The native one writes each frame's label as it draws
and reuses the ffmpeg path. Remotion has no per-frame label output and needs Node,
Chromium and a React copy of the UI. Its licence was not the blocker.

## Questions for the operator

None open. Approved 2026-11-05.
