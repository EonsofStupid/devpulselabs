# Plan — chat consolidation: one peer window, no hub route

Status: **DRAFT — awaiting operator ratification.** 2026-07-24. Resolves the drift
flagged by the `design/viziwiz-capture` merge (two chat surfaces coexisting).

## 1. Context — the operator's ruling

Verbatim (2026-07-24): *"chat needs to be merged it was always supposed to be a
window and triggered from status bar steam like experience think of chat as a
portable lite version that also will allow us to load routes from chat."*

So: **chat is a native peer window** (Steam-overlay-like, status-bar-triggered), a
**portable lite** surface that can also **load app routes** — NOT a rail hub route.
The rail-hub chat (`dpl-hub-chat`) is drift to fold in. Memory:
`chat-is-a-peer-window`.

## 2. Current state — ground-truthed this session

- **Peer window exists and is complete-ish** (`crates/chat/window`, 856 LOC):
  header (route display "Main / hub / sub", Show Main, Quit), provider library
  (**already ColdLight trait-styled** — `cl::brutal(tone)` selected chips),
  conversation, `work_stage` (wide-mode right column), composer, activity. Route
  is **observed passively** ("never attached or sent without your action").
- **Status-bar trigger exists**: `main.rs` registers a `WindowAccessory` for
  `dpl_chat_window::WINDOW_KEY`; `ChatWindowCoordinator` (`app/src/chat.rs`) owns
  the peer viewport lifecycle and handles `WindowRequest{OpenOrFocus}`.
- **NAVIGATE seam exists**: `ShellEvent::NavigateRequest{..}` is a fabric event
  (the shell drains it — `docs/plans/navigate-seam.md`). The coordinator currently
  **ignores** it (no-op match arm).
- **The drift**: `main.rs:361` also builds `dpl_hub_chat::ChatHub::with_history(
  runtime.clone(), local_chat_history_catalog())` and registers it as a **rail
  hub** (`Box::new(chat_hub)`). This hub carries the **bounded read-only session
  histories** + its own provider bar. That's the duplicate route to remove.

## 3. Target design

One chat = the peer window. The rail loses its chat route. The peer window absorbs
the two features unique to the hub:

1. **Read-only session histories** → rendered in the peer window's `work_stage`
   (wide) / below the provider library (standard/companion). Thread the
   `HistoryCatalog` into `ChatWindow::new(runtime, route, catalog)`; port the
   hub's `history_indices` read + session-list rendering (bounded, read-only,
   "INDEX CAPPED" note) into a `sessions(ui)` panel.
2. **Load routes from chat** (the new capability): the header's route line gains a
   **"Load route"** affordance, and a compact **route launcher** lists the app's
   hubs/subroutes to jump to. Selecting one pushes a new
   `ChatWindowIntent::NavigateTo(Route)` → `HostIntent::NavigateTo(Route)` → the
   coordinator publishes `ShellEvent::NavigateRequest{route}` (the shell already
   navigates on it). Chat stays "portable lite" — it launches routes, it does not
   embed them.

Then: **remove the rail-hub registration** in `main.rs`; **retire `dpl-hub-chat`**
(delete the crate + its workspace member + dep, since the peer window fully
absorbs its function — pre-release, no shim). `local_chat_history_catalog()` moves
to feeding the coordinator instead of the hub.

## 4. Changes, in execution order

1. `dpl_chat_window`: `ChatWindow::new(runtime, route, catalog)`; add a
   `sessions(ui)` panel (read-only histories) + a `route_launcher`; add
   `ChatWindowIntent::NavigateTo(Route)` and `HostIntent::NavigateTo(Route)`.
2. `app/src/chat.rs`: thread the catalog into `ChatWindow::new`; handle
   `HostIntent::NavigateTo` → `fabric.publish_shell(ShellEvent::NavigateRequest{route})`;
   keep observing route changes.
3. `app/src/main.rs`: drop the `chat_hub` build + `Box::new(chat_hub)` rail
   registration; route `local_chat_history_catalog()` into the coordinator.
4. Retire `dpl-hub-chat`: remove the crate, its workspace member, and the
   `dpl-hub-chat` dep from `crates/app/Cargo.toml`. (If any non-chat code depends
   on it — verify with grep — keep only what's needed.)
5. Docs: fold `docs/plans/chat-architecture.md` to the peer-window-only shape;
   update STATE.md.

## 5. Boundaries

- 🚫 Chat never embeds a route — it LAUNCHES one via `NavigateRequest`. The
  "portable lite" surface stays lite.
- 🚫 Histories stay **read-only** and **bounded** (the existing `HistoryLimits`);
  the peer window never writes provider stores.
- 🚫 No re-export shim for the deleted hub (pre-release, clean).
- 🚫 No `#[allow]`; tooltip law on the new route-launcher + session controls.
- ⚠️ Route observation stays **passive** by default (never auto-attaches the
  current route to a prompt) — the operator's existing guarantee; NavigateTo is an
  explicit tap, distinct from attach.

## 6. Verification

- `cargo green`.
- A `dpl_chat_window` unit test: a `NavigateTo` tap yields
  `HostIntent::NavigateTo(route)`; the sessions panel lists a fixture catalog's
  bounded entries and honors the cap.
- An `app` test / pilot drill: the status-bar accessory opens the peer window; the
  route launcher publishes `NavigateRequest` and the shell route changes; the rail
  no longer offers a chat route.
- Grep proof: zero `dpl_hub_chat` / `ChatHub` references remain after retirement.

## 7. Commit plan

One PR: `refactor(chat): consolidate to a status-bar peer window that loads routes;
retire the rail hub`. Green + exe rebuilt + STATE.md updated.
