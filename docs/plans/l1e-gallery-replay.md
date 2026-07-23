# L1e — the Gallery plays sessions in place

Parent: docs/plans/l1-library-doctrine.md (§1.2, §6). Ledger #118.
Worktree: `C:\localdev\dpl-main` on main, after L1d (afb648c).

## 1. Context

Ratified (doctrine §1.2): *"Replays play IN PLACE in the wiredFRONT
Gallery — tapping a session card opens the player right there; Sessions
stays as the dedicated list+player."* The operator's original sting:
*"im not really sure where you are watching these sessions."* Today a
session card in the Gallery is inert (only Dump/Keep act on it); the
player exists only behind the SESSIONS subroute.

## 2. Current state (ground-truthed this session, wiredfront lib.rs)

- `SessionPlay{name,player,poster,events}`; `SessionsView{rows,open,
  status}`; `open_session(rel)` builds the play state INTO the sessions
  view (also the `--open-session` seam).
- `sessions_surface` lines 590-793: the ENTIRE player (back row ·
  16:9 stage with frames/poster/honest label · play/pause + mm:ss ·
  the event-tick seek rail · partial-session text track) lives inline
  under `if let Some(play) = view.open`. Pilot ns "sessions".
- `gallery_surface`: cards return `CardAct{Dump,KeepToggle,KeepTo}`;
  the thumb allocates with `Sense::hover` (inert); session thumbs show
  session.jpg via `thumb_uri`.
- `WiredfrontHub{gallery, sessions}`.

## 3. Target design

- **Extract the player verbatim** into `fn session_player_ui(ui, play:
  &mut SessionPlay, status: &str, back_label: &str, ns: &'static str)
  -> bool` (true = back tapped). `sessions_surface` calls it with
  ("‹ sessions", "sessions"); behavior byte-equivalent.
- **`build_session_play(rel) -> (SessionPlay, String)`** extracted from
  `open_session` (which now calls it); the status string rides
  alongside instead of mutating a view.
- **`WiredfrontHub.replay: Option<(SessionPlay, String)>`** — the
  Gallery's in-place player. `gallery_surface` renders it FIRST when
  set (ns "gallery.replay", back "‹ gallery" → clears it); else the
  wall as today.
- **`CardAct::Open`**: a session card's THUMB becomes tappable
  (`Sense::click`, pilot `gallery.play/<rel>`, "play ›" on the meta
  row); firing it sets `replay`. Other kinds' thumbs stay inert.
- **Harness**: `pub fn open_replay(&mut self, rel)` +
  `--open-replay <rel>` in main.rs (the `--open-session` pattern),
  booting wiredfront/gallery.

## 4. Changes, in execution order

1. **edit `crates/hubs/wiredfront/src/lib.rs`** — extract
   `session_player_ui` + `build_session_play`; add `replay` field +
   `open_replay`; `CardAct::Open` + the session-thumb tap in
   `artifact_card`; the replay branch + Open handling in
   `gallery_surface`.
2. **edit `crates/app/src/main.rs`** — `--open-replay <rel>` flag →
   Boot → `wiredfront.open_replay(rel)` pre-registration.
3. `cargo green`; proofs (§6); commit; push backgrounded.

## 5. Boundaries

🚫 The SESSIONS surface stays byte-equivalent in behavior (same pilot
ids, same flow). 🚫 No ClipPlayer changes. 🚫 Dump/Keep flows
untouched. ⚠️ Only ONE player may live at a time per surface — the
replay slot and the sessions slot are separate structs; both alive is
fine (different routes render on different frames; pilot namespaces
differ).

## 6. Verification

- `cargo green` exits 0.
- `--open-replay inbox/session-20260723-140039 --route
  wiredfront/gallery --mood 0 --screenshot
  docs/proof/l1e-gallery-replay.png --shot-frame 90` — the Gallery
  route showing the in-place player: "‹ gallery" back, a live frame or
  poster on the stage, the tick rail with 2 coral ticks.

## 7. Commit plan

`feat(gallery): L1e — sessions play in place (the shared session
player)` — wiredfront + main.rs + proof. Push backgrounded. Program L
completion notes ride in the same push (roadmap + doctrine §6 ticks).

## 8. Executed — evidence

Execution matched the plan (one small addition: the tappable session
thumb also carries a "play ›" hover hint instead of a meta-row label —
less layout churn, same honesty). Green first run; the SESSIONS surface
is behaviorally identical through the shared `session_player_ui`.
Proof: docs/proof/l1e-gallery-replay.png — the wiredfront/gallery route
rendering the in-place player: "‹ gallery" back, the session name,
"2 events", a LIVE decoded frame (Pause showing, 00:00/00:06), and the
seek rail with both coral event ticks. Program L (#118) is COMPLETE
with this pass.
