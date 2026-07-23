# S-ADR — sessions, video-first and native (RATIFIED)

Operator, 2026-07-23: "get it fully planned out and yes we have to go
native." rrweb was webview tech; it retired with the webview. Ledger
#111.

## The decision

A SESSION is a video plus a typed event track, captured natively:

```
content/inbox/session-<stamp>/
  session.mp4            the recorder's capture of the app itself
  events.jsonl           timestamped fabric events (route changes,
                         deck beats, pixvix actions) — one JSON per line
  session.jpg            poster (existing poster_for path)
  artifact.meta.json     the X1 sidecar, kind "session"
```

- **Video**: the EXISTING recorder (`cap::record_ctl_to`) pointed at
  the app's own window — no new capture tech, ADR-009 ffmpeg subprocess
  law unchanged.
- **Events**: a session recorder subscribes to the fabric (the shell
  domain today; deck/pixvix domains as they grow) and appends
  `{ms_since_start, event}` lines. Typed enums serialized — never
  screen-scraped, never synthesized.
- **Playback** (wiredFRONT SESSIONS subroute): the dpl-video player
  (B1's machinery) + a seek rail whose TICKS are the events — tap a
  route-change tick, the video seeks there. The event track is the
  navigation the DOM replay used to give, without the webview.
- **Filing**: sessions are inbox artifacts like every capture — X1
  write-through (kind "session"), Gallery card via the poster, dump
  works today, keep works via X3. The guide system stays separate
  (operator's line) — sessions are recordings, not slide generators.

## Passes (after X3; independent of D4/D5)

- **S1 capture**: session recorder (start/stop from PixVix's Session
  tab — the surface already exists), events.jsonl writer, sidecar +
  index write-through. Engine tests: event lines are ordered,
  monotonic, valid JSON; stop is crash-safe (partial session still
  plays).
- **S2 playback**: SESSIONS subroute player + event-tick seek rail.
  Honest fallbacks (no events file = plain player).
- **S3 polish**: chapter-ing long sessions by route, trim-to-clip via
  the existing media pipeline (a session clip can then be kept into a
  show like any video).

## Boundaries

🚫 No rrweb/DOM anything. 🚫 No auto-slide generation from sessions
(that is the guide system's job, and they are separate). 🚫 Events are
observed fabric traffic only — never synthesized. Pilot scenarios ride
the capture surfaces already instrumented; new toolkit needs (if any)
go to the Codex lane.
