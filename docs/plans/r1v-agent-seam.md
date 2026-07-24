# R1v — the agent seam: axum signals to HTTP endpoints

## 1. Context

The operator's explicit requirement (2026-07-24, verbatim): "chat will be
able to generate, and work with this as well. This means axum signals
getting properly to http end points." The ratified ADR
(`docs/design/estate-and-agent-seam.md`) names the shape: a localhost HTTP
server bridging the typed `Fabric` (tokio broadcast) ↔ HTTP so chat
(Claude/Codex/Grok subscription seats) can drive the editors — `GET /state`,
`POST /command` (validated, doctor-style refusals), `GET /signals` (SSE).
This pass builds that bridge and wires it into the exe. Comp-document
editing over HTTP rides the doc law (agents edit `comp.json` on disk, the
doctor validates); this pass ships the SIGNAL seam: observe state, stream
signals, command navigation — the editor becomes drivable.

## 2. Current state (read in-session)

- `crates/fabric/src/lib.rs` — `Fabric` (Clone, shares channels):
  `publish_shell/subscribe_shell` over `broadcast::channel(64)`;
  `ShellEvent` = RouteChanged{route} · ViziwizSummon · FullBleedRequest(bool)
  · NavigateRequest{route} · WindowRequest{target,action}. `DeckEvent::Beat`.
  No serde anywhere; `Route{hub: HubId(&'static str), sub: SubId(&'static str)}`.
- `crates/app/src/main.rs` — boot builds `Fabric::new()` (323), wires hubs,
  then `hubs: Vec<Box<dyn Hub>>` (359–367) with `hub.spec()` accessible
  (371–377 already iterates specs for chat_routes) before `hubs` moves into
  `Shell::new` (383). CLI flags parse by position scan (~29–140).
- Root `Cargo.toml` — workspace tokio is `default-features = false,
  features = ["sync"]` (additive per-crate features are allowed). No axum,
  no tokio-stream, no tower.
- `dpl-hub` — `HubSpec{id,label,icon,tone,subroutes}`, `SubRoute{id,label,
  blurb}`, `Route`, ids are `&'static str` newtypes.

## 3. Target design

New crate **`dpl-seam`** (`crates/seam`): a localhost-only axum server on a
background thread with its own tokio runtime (the egui thread is never
touched). The app hands it a fabric clone + the hub REGISTRY
(`Vec<&'static HubSpec>`); owned HTTP strings resolve to `'static` routes
ONLY through the registry — an unknown route is refused by name (the doctor
law), never invented.

Endpoints (all JSON):
- `GET /state` → `{ version, route: "hub/sub"|null, hubs: [{id, label,
  subs: [{id, label, blurb}]}] }`. `route` is the seam's last-heard
  `RouteChanged` (it subscribes at spawn).
- `POST /command` — `{"cmd":"navigate","route":"wiredfront/editor"}`
  publishes `NavigateRequest`; `{"cmd":"viziwiz-summon"}` publishes
  `ViziwizSummon`. Valid → `200 {"ok":true}`; unknown route/cmd →
  `422 {"ok":false,"issues":["..."]}` .
- `GET /signals` → SSE: each ShellEvent as one JSON event
  (`{"type":"route-changed","route":"hub/sub"}` etc.), keep-alive pings.

CLI: `--seam-port <n>` (default 7341), `--no-seam` disables. Bind is
`127.0.0.1` ONLY — never a public interface. Serialization of events lives
in dpl-seam (the fabric stays serde-free by design).

## 4. Changes, in execution order

1. Root `Cargo.toml` — members += `crates/seam`; workspace deps +=
   `axum = "0.8"`, `tokio-stream = { version = "0.1", default-features =
   false, features = ["sync"] }`, `tower = "0.5"` (dev, for endpoint tests),
   `dpl-seam = { path = "crates/seam" }`.
2. NEW `crates/seam/Cargo.toml` — deps: axum, tokio (workspace +
   `rt-multi-thread`, `net`, `macros`, `time`), tokio-stream, serde,
   serde_json, dpl-fabric, dpl-hub, tracing; dev-deps: tower (`util`).
3. NEW `crates/seam/src/lib.rs` — `Registry` (resolve + state json),
   `event_json(ShellEvent)`, `router(SeamState)` (the three endpoints),
   `spawn(fabric, registry, port) -> std::thread::JoinHandle<()>` (thread
   named `dpl-seam`, runtime, axum::serve, a task folding shell events into
   `last_route`). Tests: resolver accepts registered + refuses unknown by
   name; POST /command publishes onto a real Fabric (tower oneshot);
   GET /state carries the registry; event_json shapes are pinned.
4. `crates/app/src/main.rs` — parse `--seam-port`/`--no-seam`; after
   `chat_routes` is built (378), spawn the seam with
   `hubs.iter().map(|h| h.spec()).collect()` and the fabric clone; log the
   bound port. One line each, per Law 4's spirit.

## 5. Boundaries

- 🚫 `dpl-fabric` gains NO serde/axum deps — serialization is the seam's.
- 🚫 Never bind beyond 127.0.0.1; no auth this pass means localhost-only is
  the security boundary.
- 🚫 No comp-document mutation endpoints this pass (they ride the doc
  doctor in the R1f follow-up).
- ⚠️ Ask first: exposing deck/beat or capture domains over the seam.
- Out of scope: the estate (parked branch), gen backends, chat-side client.

## 6. Verification

1. `cargo test -p dpl-seam` → resolver/command/state/event tests green.
2. `cargo green` → exit 0 (fmt, clippy, all tests, release exe).
3. Live drill against the REAL exe (sequential, then kill):
   - boot `devpulselabs.exe` normally (background), then
   - `GET http://127.0.0.1:7341/state` → route `home/...`, all five hubs
     with subroutes (wiredfront lists editor+library).
   - `POST /command {"cmd":"navigate","route":"wiredfront/editor"}` → 200;
     a following `GET /state` shows `wiredfront/editor` (the shell heard
     the ask, navigated, and the seam heard the arrival).
   - `POST /command {"cmd":"navigate","route":"nope/nowhere"}` → 422 with
     the route named in issues.
   - `GET /signals` captures the route-changed event JSON.
   - Kill the exe (no orphans — the CLI law).

## 7. Commit plan

Branch `feat/r1v-agent-seam` stacked on `feat/r1u-editor-fidelity`. One
commit: `feat(seam): R1v — axum agent seam (GET /state · POST /command ·
GET /signals SSE) over the fabric`. Push + PR base r1u.

## 8. Evidence

- `cargo test -p dpl-seam` → 5/5 green FIRST compile (resolver refusals by
  name; navigate publishes onto a real Fabric via tower oneshot; 422 +
  issues + publishes-nothing; /state carries registry + heard route; every
  event variant's wire shape pinned).
- Release exe built with the seam wired (`4m 03s` — the one-time axum tree).
- LIVE DRILL against the real exe (2026-07-24, exe booted normally, then
  killed — no orphans):
  - `GET /state` → all five hubs with subroutes; wiredfront lists
    `editor` + `library` first; `route: null` at boot (the seam subscribes
    after the boot arrival — documented behavior, first navigation fills it).
  - `POST /command {"cmd":"navigate","route":"wiredfront/editor"}` →
    `{"ok":true}` AND the app navigated: next `GET /state` returned
    `"wiredfront/editor"` — the full loop (HTTP → fabric NavigateRequest →
    shell navigates → RouteChanged → seam) is closed.
  - `POST navigate nope/nowhere` → HTTP 422 (unit test pins the issues body).
  - `GET /signals` (SSE via curl) captured, in order:
    `{"route":"wiredfront/editor","type":"navigate-request"}` then
    `{"route":"wiredfront/editor","type":"route-changed"}`.
- Deviation: none from §4; the full gate runs in the pre-push hook.
