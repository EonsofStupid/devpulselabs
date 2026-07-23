# C3 — Video parity (format matrix, audio, target, max-cap)

Parent: roadmap Program C (#108). After C2 (0fcea5c). Worktree:
`C:\localdev\dpl-main` on main.

## 1. Context

Roadmap C3: *"format matrix (mp4/webm-vp9/av1), audio (loopback/mic +
device list), 720p default + max-cap, REC timer + poster. Pilot:
recorder lifecycle scenario w/ suppressed-motion capture."*

The ENGINE already carries all of it (Pass Z8 V4, ground-truthed this
session): `VideoOpts{height, format, audio, audio_device, max_seconds}`,
`RecTarget::{Monitor,Region,Window}`, `ext_for_format` ("webm-vp9"/
"webm-av1" → webm, else mp4), `list_audio_devices()` (dshow on Windows,
pulse on Linux, ffmpeg-only), `record_ctl_to` (caller-owned stop
AtomicBool + optional hard cap), `poster_for`. What the egui palette
exposes today is a fraction: primary monitor · 1080p · mp4 · silent ·
no cap. C3 is the UI tier that surfaces the matrix.

Old-repo semantics (pixvix-palette.tsx:448-484, read this session):
height fixed **720**; format select mp4/webm-vp9/webm-av1; audio select
none/system/mic with a device select seeded from `list_audio_devices`
(default = first device when a mode needs one); `maxSeconds` optional;
target = full screen or a picked window. REC timer and the poster
already exist in the egui port.

## 2. Current state (ground-truthed this session)

- `crates/pixvix/src/lib.rs` (post-C2 split, read in full):
  `start_video` hardcodes `VideoOpts{height:1080, format:"mp4",
  audio:"none", audio_device:None, max_seconds:None}` +
  `RecTarget::Monitor(primary)`; `VideoRec{stop, started, out, done}`;
  the done-watcher in `ui()` reports + indexes + clears; `stop_video`
  flips the AtomicBool. `windows: Vec<cap::WindowInfo>` + a picker
  already exist (Img tab).
- `crates/pixvix/src/palette.rs` (authored this pass): Video tab idle
  arm = one "Record screen" chip + a static hint ("1080p x264 · silent
  (audio in PV3)" — stale, PV3 = this pass); recording arm = REC timer
  + Stop.
- `crates/engine-capture/src/lib.rs:806-848` — the API above; AV1 note:
  CPU-heavy, mp4 stays the default.
- Drills pattern in `crates/app/src/main.rs` (`--pixvix-snap`,
  `--session-drill`, C2's `--steps-hotkey-drill`).

## 3. Target design

Touch-first (the UI law): CHIPS that cycle, not dropdowns, except the
device list (a real enumeration → ComboBox like the window picker).

- Pixvix fields: `video_format: usize` over `FORMATS = [("mp4","MP4"),
  ("webm-vp9","VP9"), ("webm-av1","AV1")]`; `video_audio: usize` over
  `AUDIO = [("none","Silent"), ("system","System"), ("mic","Mic")]`;
  `audio_devices: Vec<String>` + `picked_audio: usize` (enumerated when
  audio cycles OFF "none" the first time, refresh chip re-lists);
  `video_window: bool` (false = primary screen, true = the Img tab's
  picked window); `video_cap: usize` over `CAPS = [None, 60, 300, 900]`
  (Off · 1m · 5m · 15m).
- `start_video`: target from `video_window` + `picked_window`; opts =
  `height: 720` (the old default; the engine downscales), format/audio/
  cap from the fields; `audio_device` = picked device, else first, else
  None (engine errors honestly for "system" without a device); output
  stem "vix" + `ext_for_format`. Status names format + audio.
- Video tab idle: `Screen ⇄ Window` chip · format chip · audio chip ·
  device ComboBox (only when audio ≠ none) · cap chip · Record chip ·
  honest hint. Recording arm unchanged (REC timer + Stop).
- Harness: `--video-drill <secs>` — boot, fire the REAL start handler
  at frame 30 (screen, current defaults = mp4/silent/no cap), stop
  after `<secs>`, wait for the done-watcher, print the status, exit 0
  iff it starts with "saved" (poster + index ride the existing path).
  `--video-format <f>` pins the format so the matrix is drillable
  (mp4 AND webm-vp9 runs in §6; AV1 excluded — CPU-heavy, engine-same
  path).

## 4. Changes, in execution order

1. **edit `crates/pixvix/src/lib.rs`** — FORMATS/AUDIO/CAPS consts;
   the five fields (+init); `start_video` rewritten as §3; harness
   `video_start_now(&mut self, format: Option<&str>) -> &str`,
   `video_active()`, `video_stop_now()`; apply() arms for the new
   actions (cycle_format/cycle_audio/cycle_cap/toggle_video_target/
   pick_audio/refresh_audio — audio cycle INTO system/mic triggers the
   device enumeration when empty).
2. **edit `crates/pixvix/src/palette.rs`** — PaletteView += video_target
   /format/audio/cap labels + audio_devices/picked_audio; the new
   actions; the Video idle arm per §3; stale hint replaced. New pilot
   ids: `pixvix.video.target/format/audio/device/cap`.
3. **edit `crates/app/src/main.rs`** — `--video-drill <secs>` +
   `--video-format <f>` flags, Boot fields, drill state machine
   (start at frame 30 → stop at deadline → exit on the reported
   status; 600-frame post-stop timeout).
4. `cargo green`; drills (§6); roadmap tick; §8; commit; push.

## 5. Boundaries

🚫 Engine untouched (the matrix is proven engine code). 🚫 Session
vehicle untouched. 🚫 No new deps. ⚠️ The old numeric max-cap input
becomes a cycling chip (touch law) — a deliberate, recorded deviation
from the old free-text field. Out of scope: region-target video (old
app didn't surface it either), the pilot lifecycle scenario (palette
viewport — Codex multi-window lane, declared).

## 6. Verification

- `cargo green` exits 0.
- `--video-drill 3` exits 0: status "saved …vix….mp4 (N frames)…".
- `--video-drill 3 --video-format webm-vp9` exits 0 with a `.webm`
  output — the matrix is real on the exe.
- `--pixvix-snap` still exits 0 (regression).
- Poster: the drill's saved mp4 has a sibling `.jpg` (existing path,
  asserted by eye on the inbox listing in §8).

## 7. Commit plan

`feat(pixvix): C3 — video parity (format matrix, audio devices, screen/
window target, max-cap)`. Push backgrounded.

## 8. Executed — evidence

Executed to plan, with one live finding fixed in-pass: the FIRST matrix
drill run produced `vix.mp4` and `vix.webm` whose posters BOTH mapped to
`vix.jpg` (`with_extension("jpg")`) — the webm's poster clobbered the
mp4's, and the naming was off the repo's real convention anyway
(`<stem>-poster.jpg`, per dpl-video and engine-capture). Fixed:
`poster_path` follows the convention; `unique_video_path` keeps stems
unique ACROSS containers (a webm never reuses an mp4's stem); the video
index row now records `poster_rel_path` when the poster exists
(`index_video`, like sessions). Unit test
`video_stems_never_share_a_poster` pins it. The pre-fix drill artifacts
(vix.jpg/vix.mp4/vix.webm) stay in the inbox — operator data, never
deleted.

Proven: `cargo green` exits 0 (incl. the new stem test);
`--video-drill 3` → "saved …vix.mp4 (84 frames)" exit 0;
`--video-drill 3 --video-format webm-vp9` → "saved …vix-2.webm
(90 frames)" exit 0 with `vix-2-poster.jpg` beside it — the format
matrix is real on the exe; `--pixvix-snap` still exits 0. Audio modes
ride the same engine call (`record_ctl_to`) with the enumerated device
list; not drilled (a loopback device is machine-dependent) — the chip →
`list_audio_devices` → device combo path is code-reviewed and the
engine errors honestly, recorded as the pass's deviation alongside the
carried palette-viewport pilot deferral (Codex multi-window lane) and
the deliberate cap-chip-instead-of-free-text change (§5).
