# S1 — sessions capture (video + fabric event track)

Parent: docs/plans/s-adr-sessions.md (RATIFIED: native, video-first;
web-playable BY CONSTRUCTION at export, S3). Ledger #111. After X3.
Worktree: `C:\localdev\dpl-main` on main.

## 1. Context

A SESSION is the recorder pointed at the app itself plus a typed event
track — the navigation the DOM replay used to give, without a webview.
S1 is CAPTURE only: Start/Stop from PixVix's Session tab (the surface
exists as an honest placeholder), producing the ratified folder
artifact. S2 (playback + tick-seek) and S3 (web export) follow.

```
inbox/session-<stamp>/
  session.mp4          record_ctl_to pointed at the primary monitor
  events.jsonl         {"ms":…,"type":…,…} — observed fabric traffic ONLY
  session.jpg          poster (poster_for)
  artifact.meta.json   the X1 sidecar, kind "session"
```

## 2. Current state (ground-truthed this session)

- `crates/pixvix/src/lib.rs`: Tab::Session renders a placeholder
  (~l.782). The Video tab's shape is the model: `VideoRec { stop:
  Arc<AtomicBool>, started: Instant, out, done: Arc<Mutex<Option<
  Result<u32,String>>>> }`; `start_video` spawns a named thread running
  `cap::record_ctl_to(out, RecTarget::Monitor(primary), opts, &stop)`,
  writes the poster on ok; `ui()` polls `done` once per frame and
  indexes via `index_capture`. `unique_path` = never-clobber names.
  Pixvix holds `rx: broadcast::Receiver<ShellEvent>` (summons).
- `crates/fabric/src/lib.rs`: `ShellEvent = RouteChanged{route} |
  PixvixSummon | FullBleedRequest(bool)`; `subscribe_shell()` hands a
  fresh receiver (only sees traffic AFTER subscription — exactly the
  session-start semantics we want). No serde on the enum.
- `crates/engine-steps/src/lib.rs:62`: `compact_timestamp()` is
  PRIVATE ("one metadata implementation for every capture vehicle").
- `crates/index/src/lib.rs`: `scan_inbox` recognizes folders ONLY as
  guides (`guide.json`); `file_into_episode`'s folder branch hardcodes
  kind "guide" — a kept session folder would mislabel. `sidecar::
  path_for(dir)` → `artifact.meta.json` already handles folders.
- `crates/hubs/wiredfront/src/lib.rs`: FILTERS = all/image/video/guide;
  `thumb_uri` has no "session" arm.
- pixvix Cargo.toml lacks serde_json.

## 3. Target design

**`crates/pixvix/src/session.rs` (new module)** — the recorder:
- `EventLog`: create(`events.jsonl`) → append one JSON line per
  observed `ShellEvent` with `ms` since start, flushed per line
  (crash-safe: a killed app leaves a valid prefix). Serialization is
  hand-mapped per variant (`route-changed` {hub, sub} · `pixvix-summon`
  · `full-bleed` {on}) — typed enums serialized, NEVER synthesized;
  the only source is a dedicated `subscribe_shell` receiver taken at
  session start.
- `SessionRec`: the VideoRec shape + `rx: ShellReceiver` + `EventLog` +
  `dir`. Start = create `inbox/session-<compact_timestamp()>/` (never-
  clobber via `-2` suffix), spawn the video thread (session.mp4,
  1080p mp4 silent — the Video tab's exact opts), poster session.jpg.
- Pixvix drains `rx` into the log each `ui()` frame while a session
  runs; on thread completion: build the record via
  `record_for_capture(dir, root, "session", "pixvix")`, set
  poster_rel_path, sidecar::write + upsert (status-line degrade, the
  index_capture law). Session tab UI: Start/Stop + elapsed + event
  count + honest status; placeholder text retired.

**`crates/engine-steps`**: `compact_timestamp` goes `pub` (reuse, not
restatement — pixvix already depends on this crate).

**`crates/index`**: `folder_kind(dir)` helper — `guide.json` → guide;
`session.mp4` OR `events.jsonl` → session; else None. `scan_inbox`
folder branch uses it (sessions reindex; a partial session — events
but no finished video — still indexes: files are the truth).
`file_into_episode`'s folder branch uses it too (kind survives keep).

**`crates/hubs/wiredfront`**: FILTERS += ("session", Some("session"));
`thumb_uri` session arm → `<rel>/session.jpg`.

## 4. Changes, in execution order

1. `crates/engine-steps/src/lib.rs` — `fn compact_timestamp` → `pub fn`
   (+ doc line naming the session vehicle as a consumer).
2. `crates/index/src/lib.rs` — add `folder_kind`; use in `scan_inbox`
   + `file_into_episode`; tests: a session folder reindexes as kind
   "session" (with and without session.mp4); keep preserves the kind.
3. `crates/pixvix/Cargo.toml` — + serde_json.workspace.
4. `crates/pixvix/src/session.rs` — NEW: EventLog + SessionRec + line
   serialization; tests: lines are ordered/monotonic/valid JSON; each
   line is flushed (re-open mid-log and parse); folder naming
   never-clobbers.
5. `crates/pixvix/src/lib.rs` — `mod session;`, `session:
   Option<session::SessionRec>` on Pixvix, start/stop/poll wired in
   `ui()` (drain events; completion → poster + sidecar + upsert),
   Session tab UI replaces the placeholder.
6. `crates/hubs/wiredfront/src/lib.rs` — session filter chip + thumb.
7. `cargo green`; drills + proofs (§6); commit; push backgrounded.

## 5. Boundaries

🚫 No playback (S2), no export (S3), no rrweb/DOM anything. 🚫 Events
are observed fabric traffic only — the recorder never fabricates a
line. 🚫 The guide system stays separate. 🚫 Deck/pixvix fabric
domains don't exist yet — the shell domain is the whole feed (the ADR
names the widening as later work, not this pass). ⚠️ The Video tab's
recording path stays byte-identical.

## 6. Verification

- `cargo green` exits 0 (fmt, clippy -D warnings, all suites, release).
- Engine tests (index + session) green within it.
- REAL-EXE DRILL: run the exe, start a Session from the palette,
  navigate hubs, stop; then prove on disk: `inbox/session-*/` holds
  session.mp4 (>0 bytes), session.jpg, artifact.meta.json (kind
  "session"), events.jsonl whose lines parse and carry monotonic `ms`
  with at least one `route-changed`; `dpl-index-cli list session`
  shows the row. (The palette is driven by hand or the existing
  summon; if a hands-off path is impractical this run, the drill uses
  a scripted recorder invocation through the same engine functions and
  the UI is proven by screenshot.)
- Gallery proof shot: the session card with its poster thumb +
  session filter chip.

## 7. Commit plan

`feat(sessions): S1 — session capture (video + fabric event track)` on
main; push backgrounded.

## 8. Executed — deviations recorded (corrections law)

- The hands-off drill became a first-class harness flag:
  `--session-drill <secs>` starts the session through the REAL palette
  handler, performs two real `shell.navigate` hops (the shell publishes
  RouteChanged — the recorder only observes), stops after the window,
  and exits on the recorder's own status verdict. Pixvix gained the pub
  drill seams `session_start_now/session_stop_now/session_active/
  status_text` (the `snap_full_now` precedent).
- Pixvix now stores the `Fabric` (clone) so each session takes a FRESH
  subscription at start.
- FIXED an X3 latent bug the proof shot exposed: the Gallery Keep
  pilot id was shared across cards (`gallery.keep`) and PANICKED as a
  duplicate the moment a second inbox card rendered — now childed per
  card by rel_path.
- Drill evidence (real exe): `inbox/session-20260723-140039/` —
  session.mp4 (181 frames), session.jpg, events.jsonl (2 observed
  route-changed lines, monotonic ms), artifact.meta.json kind
  "session"; `dpl-index-cli list session` shows the row; proof shot
  docs/proof/s1-gallery-session.png (poster thumb + session chip).
