# S2 — sessions playback (player + event-tick seek rail)

Parent: docs/plans/s-adr-sessions.md (RATIFIED). After S1 (51bc34b).
Ledger #111. Worktree: `C:\localdev\dpl-main` on main.

## 1. Context

S1 produces the session artifact (session.mp4 + events.jsonl +
session.jpg + sidecar). S2 makes it WATCHABLE in the app: the
wiredFRONT SESSIONS subroute lists sessions and plays one, with a seek
rail whose TICKS are the recorded events — tap a route-change tick,
the video seeks there. That event track is the navigation the DOM
replay used to give, without a webview.

## 2. Current state (ground-truthed this session)

- `crates/video/src/lib.rs` is the LOOP player only: `-re -stream_loop
  -1` rawvideo pipe → one shared frame slot → texture; reader RESPAWNS
  forever (a backdrop never ends); no seek, no pause, no duration.
  Decode is cover-cropped 960×540 (`DECODE_W/H`, `FRAME_BYTES`).
  `set_ffmpeg_path` injection; poster path conventions.
- `crates/hubs/wiredfront`: SESSIONS renders the generic placeholder;
  GalleryView/list machinery exists; no dpl-video or serde_json dep.
- The S1 artifact on disk: events.jsonl lines
  `{"ms":n,"type":"route-changed","hub":…,"sub":…}` (also
  pixvix-summon, full-bleed{on}).
- Harness precedent: `--present <id>` reaches into a hub before
  registration (main.rs).

## 3. Target design

**`crates/video/src/clip.rs` (new)** — the seekable CLIP player,
ADR-009 shapes throughout:
- `clip_pipe_args(path, start_secs)`: `-re -ss <t> -i … -an -vf
  <the same cover scale> -f rawvideo -pix_fmt rgba -` — `-ss` BEFORE
  `-i` (fast seek), NO `-stream_loop` (a clip ends).
- `parse_duration(stderr)`: the clip's length from `ffmpeg -i`'s
  stderr banner ("Duration: HH:MM:SS.cc") — no ffprobe dependency,
  pure + unit-tested. `clip_duration(path)` runs it (None degrades
  honestly).
- `ClipPlayer`: `open(path)` (probes duration, starts playing at 0),
  `toggle()` (pause = kill pipe + freeze base at `position()`; play =
  respawn at base; after EOF, toggle replays from 0), `seek(t)` (kill
  + respawn at t — seeking always plays), `position()` (= base +
  wall-clock while playing; `-re` paces at native rate), `texture()`
  (the loop player's upload pattern), `ended()` (reader sets it on
  EOF; NO respawn — the loop player's respawn is a backdrop law, not
  a clip law). Each (re)spawn takes a fresh shared slot; the old
  thread sees stop and exits.

**`crates/hubs/wiredfront` SESSIONS surface:**
- List: `Index::list(Some("session"), None)` cards (poster thumb via
  the Gallery's `thumb_uri` law, name, captured_at, event count).
- Player view: 16:9 stage painting `ClipPlayer::texture` (poster until
  frames land); a control row (play/pause chip, mm:ss position /
  duration); THE RAIL — tappable full-width strip seeking
  proportionally, with one TICK per parsed event; tick hover shows
  `type → hub/sub`; tapping a tick seeks to its `ms`. Rail span =
  duration, else the last event's ms (honest fallback), else 1s.
- Partial session (no session.mp4): the event list renders as tappable
  text rows labelled "partial session — event track only"; no crash,
  no fake video.
- No events.jsonl / bad lines: plain player, no ticks (each line
  parses independently; bad lines skip).
- PilotIds: `sessions.open.<rel>`, `sessions.play`, `sessions.rail`,
  `sessions.back`.

**Harness:** `--open-session <rel_path>` — WiredfrontHub gains
`open_session(rel)` (loads the view through the same code the tap
uses); main.rs wires it pre-registration (the `--present` pattern) and
boots the route to wiredfront/sessions.

## 4. Changes, in execution order

1. `crates/video/src/clip.rs` — NEW module (+ `pub mod clip;` in
   lib.rs; DECODE constants + FRAME_BYTES + ffmpeg_program get
   `pub(crate)` visibility as needed). Tests: pipe args carry `-ss`
   before `-i` and no `-stream_loop`; parse_duration on a real banner
   + garbage; position math (paused position is frozen).
2. `crates/hubs/wiredfront` — Cargo: + dpl-video, serde_json.
   SESSIONS surface (list → player, rail, ticks, fallbacks) replacing
   the placeholder branch for SESSIONS only.
3. `crates/app/src/main.rs` — `--open-session <rel>` + Boot wiring +
   route to wiredfront/sessions when present.
4. `cargo green`; proofs (§6); commit; push backgrounded.

## 5. Boundaries

🚫 No export (S3), no chaptering/trim (S3). 🚫 The loop player's
backdrop behavior stays byte-identical (clip.rs is additive). 🚫 No
event synthesis — the rail renders exactly the parsed lines. ⚠️ The
seek engine kills/respawns ffmpeg — never more than one live pipe per
player (fresh shared per spawn, old thread exits on stop).

## 6. Verification

- `cargo green` exits 0 with the new suites.
- Real-exe proof: `--open-session inbox/session-20260723-140039
  --route wiredfront/sessions --screenshot docs/proof/
  s2-session-player.png --shot-frame 90` — the shot shows the video
  stage (a real decoded frame or the poster), the play control, the
  rail, and 2 event ticks.
- List proof: `--route wiredfront/sessions --screenshot
  docs/proof/s2-sessions-list.png` — the S1 session card.

## 7. Commit plan

`feat(sessions): S2 — playback with the event-tick seek rail` on
main; push backgrounded.

## 8. Executed — evidence

Execution matched the plan (no design deviations). Proofs:
docs/proof/s2-session-player.png — a LIVE decoded frame mid-playback
(00:02/00:06, Pause showing, magenta progress fill, 2 coral event
ticks, "2 events"); docs/proof/s2-sessions-list.png — the session
card with poster thumb. dpl-video clip tests green (fast-seek args,
duration banner parse incl. N/A refusal, frozen/clamped paused
position).
