# H1 — local capability is real activation, and the chats are in one system

Program map: `docs/plans/chat-operations-console.md` (link H1, §3b).
Branch: `feat/h1-chat-histories` off `main` after #178 lands.

## 1. Context

The operator, plainly:

> "i am building a gosh darn private tool where I am allowed to have it
> become convenient to have all the chats available in one system"

and, on three passes of chat chrome that never delivered it:

> "you have been handwaving me nonstop on this"

and then, naming the structural half of the same problem (2026-07-30):

> "chat needs to be built so its properly yes its own window but the
> providers that get activated in the chat become available in the
> platform"

**Measured 2026-07-30 on this machine: 128 Claude transcripts across 15
projects + 37 Codex transcripts = 165 real chats on disk.** A live-runtime
EguiPilot snapshot (`chat-local-histories`, fixture `chat`) found **ZERO**
session rows in the rail.

This is a private client reading the operator's OWN local transcripts on
their OWN machine. There is nothing to hedge about, and the hedging is
what produced three passes of rail/tab/seam work while the chats stayed
invisible.

### 1.1 What the second quote changes about this plan

The first draft of this plan (superseded here) proposed adding history
rows under a `RECENT CHATS` heading and stopping there. Ground-truthing
the operator's directive found that **rows alone would not have worked**,
and the reason is the directive itself:

`connected_tabs()` (`crates/chat/window/src/lib.rs:1863`) — its own
doc-comment is the law: *"A tab is the reward for being connected, so the
predicate is activation and nothing else."* Claude registers as
`ProviderRegistration::planned(...)` → `ActivationState::Blocked` → it
**never earns a tab**, so its 128 transcripts had no home in the strip,
only a spot in the unscoped roster. The rail was never the whole defect.

**Activation is the gate for everything the operator asked for.** So H1
is now two joined halves: make local capability count as real activation,
then render what that activation unlocks.

## 2. Current state

Ground-truthed in-session against `16e9d9c` (D1's tip).

### 2.1 The decoders are complete

`crates/chat/providers/claude/src/lib.rs:55` — `impl HistoryDecoder for
ClaudeHistoryDecoder` reads `.jsonl`, pulling `cwd` → workspace,
`timestamp` → started_at, first `type:"user"` message → title, and
decodes `user` / `assistant` / `system(init)` / `result` records into
`ChatEventContent`. `CodexHistoryDecoder` is the same shape
(`providers/codex/src/lib.rs:67`).

### 2.2 The sources are registered against the operator's real roots

`crates/app/src/main.rs:1176`, `local_chat_history_catalog()`, registers
`~/.codex/sessions` (guarded by `codex_root.is_dir()`) and
`~/.claude/projects` (guarded by `claude_root.is_dir()`).

`HistoryCatalog::has_source(provider) -> bool` (`store/src/lib.rs:608`)
is the exact predicate "does this provider have transcripts here".

### 2.3 The window ALREADY indexes them

`crates/chat/window/src/lib.rs:548` (inside `with_models`):

```rust
        let mut histories = HashMap::new();
        for view in runtime.provider_views() {
            let provider = view.registration.manifest.descriptor.id;
            if !catalog.has_source(provider) { continue; }
            let entry = match catalog.list(provider, None) {
                Ok(index) => ProviderHistory { index: Some(index), error: None },
                Err(error) => ProviderHistory { index: None, error: Some(error.to_string()) },
            };
            histories.insert(provider, entry);
        }
```

field at `:486` — `histories: HashMap<ProviderId, ProviderHistory>`,
`ProviderHistory { index: Option<HistoryIndex>, error: Option<String> }`
(`:453`).

### 2.4 …and then throws the result away

`self.histories` is consumed in EXACTLY ONE place — the empty-state
branch of `session_groups` (`:2028`):

```rust
            let local_count = self.histories.values()
                .filter_map(|history| history.index.as_ref())
                .map(|index| index.sessions.len())
                .sum::<usize>();
            ...
                RichText::new(format!("{local_count} local histor{} available", ...))
```

A count, in an unclickable label. Every session ROW is minted from
`provider.registration.sessions` — the LIVE runtime registry, which is
empty for history-backed providers.

### 2.5 Claude is registered as `planned`, so it is Blocked

`crates/app/src/main.rs:780` and `:798` (the devtools and non-devtools
arms carry the same list):

```rust
                    dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_claude::MANIFEST),
                    dpl_chat_runtime::ProviderRegistration::planned(dpl_chat_gemini::MANIFEST),
```

`planned()` (`runtime/src/lib.rs:25`) hard-codes `ActivationState::
Blocked { reason: "<label> has a reviewed integration boundary but no
qualified live adapter." }`. True of the LIVE adapter; false of the
provider as a whole, because reading local history is a real, working,
qualified capability sitting right there in `local_chat_history_catalog`.

`activation_registration` (`main.rs:1165`) is the existing helper for
Codex/Grok that projects a coordinator's activation into a registration.

### 2.6 The model already has the exact states this needs

Nothing is invented by this pass:

- `Capability::ReadSessions` and `Capability::PageHistory`
  (`core/src/lib.rs:255,256`) — already in the enum, never qualified by
  anyone.
- `ActivationState::Experimental { reason }` — `is_activated()`
  (`:384`) is `matches!(self, Self::Ready | Self::Experimental { .. })`,
  so `Experimental` **is activated** and still carries a truthful reason
  string that `reason()` (`:388`) surfaces.
- `can_send()` (`runtime/src/lib.rs:52`) requires `is_activated() &&
  StreamTurns && (StartSession | ResumeSession)`. A history-only provider
  has none of those three, so `can_send()` stays **false** and the
  composer stays correctly disabled with no new gate.

### 2.7 The types already match the UI

- `IndexedSession { locator, summary: SessionSummary, started_at,
  modified_millis, source_bytes }` (`store/src/lib.rs:110`). **`summary`
  is the very `SessionSummary` `session_groups` already renders.**
- `HistoryCatalog::page(provider, locator, cursor) -> HistoryPage`
  (`:646`) where `HistoryPage { events: Vec<ChatEvent>, next_cursor,
  truncated_before, ... }` (`:127`). **`events` is the very `ChatEvent`
  the conversation pane already renders.**
- `HistoryLocator(u64)` (`:96`) is `Copy` — a cheap row identity.

Neither half needs a new renderer. This is wiring.

### 2.8 The one obstruction

`ChatWindow::new` / `with_models` take `catalog: HistoryCatalog` **by
value** (`:523`, `:541`), index it, and drop it — no `self.catalog`
field. So the window cannot `page()`, and the host no longer owns the
catalog either (it is moved in at `crates/app/src/chat.rs:262/268` via
`ChatWindowConfig.catalog`).

### 2.9 Construction order blocks the activation change

`main.rs` builds `chat_registrations` (`:773`) → `chat_runtime` (`:814`)
→ and only then `local_chat_history_catalog()` (`:940`, inline in the
`ChatWindowConfig` literal). Deriving activation from the catalog
requires the catalog to be built FIRST. It is a pure move — the function
takes no arguments and reads only the environment.

### 2.10 Nothing in the platform can see a provider

`ChatWindowCoordinator`'s entire public surface is `new`,
`attach_determinism`, `attach_pilot_runtime`, `consume_pilot_close_
requests`, `root_close_requested`, `frame`. `ChatRuntime` is a private
field of `ChatWindow` (`:484`) — a SURFACE. A workspace grep for
`ActivationState` / `Capability::` outside `crates/chat/` returns only
`app/src/chat.rs` and `app/src/main.rs`, i.e. the chat host itself.

So the second half of the operator's directive — activated providers
becoming **available in the platform** — has no seam at all today. That
half is scoped to P1 (§5, Out of scope) rather than smuggled in here,
for the reason D1 established: a publication with no consumer proves
nothing. H1 makes activation TRUE; P1 makes it VISIBLE to the platform
and lands the first consumer in the same pass.

## 3. Target design

### 3.1 Local transcripts are a qualified capability, so they activate

New helper in `main.rs` beside `activation_registration`:

```rust
/// Local transcripts are a real, working capability — not a consolation
/// prize for a missing live adapter. A provider whose history root
/// exists on this machine is ACTIVATED for what it can actually do:
/// read and page its own past sessions. `can_send` stays false because
/// it has no `StreamTurns`, so the composer disables itself with no new
/// gate, and the reason string never over-claims.
fn with_local_history(
    mut registration: dpl_chat_runtime::ProviderRegistration,
    catalog: &dpl_chat_store::HistoryCatalog,
) -> dpl_chat_runtime::ProviderRegistration {
    if !catalog.has_source(registration.manifest.descriptor.id) {
        return registration;
    }
    registration
        .qualified_capabilities
        .insert(dpl_chat_core::Capability::ReadSessions);
    registration
        .qualified_capabilities
        .insert(dpl_chat_core::Capability::PageHistory);
    if !registration.activation.is_activated() {
        registration.activation = dpl_chat_core::ActivationState::Experimental {
            reason: "Local transcripts only — live turns are not qualified yet."
                .to_owned(),
        };
    }
    registration
}
```

Applied to EVERY registration in both arms (`main.rs:773` and `:792`),
not just Claude's: Codex has both a live coordinator and a history root,
and the `is_activated()` guard means a genuinely-live provider keeps its
own state and merely gains the two history capabilities. Gemini has no
history source, so `has_source` is false and it stays `planned` —
untouched, and correctly so.

`local_chat_history_catalog()` moves ABOVE the registration list, and the
same catalog value is threaded into `ChatWindowConfig`. One catalog, one
census, no second directory walk.

**Consequence, which is the point:** Claude satisfies `connected_tabs()`
and earns a tab beside AI and Social, exactly as the directive asks.

### 3.2 The host keeps the catalog; the window borrows it to index

Change both constructors to take `catalog: &HistoryCatalog`. The window's
indexing loop is unchanged (it only calls `has_source`/`list`). The host
stores the catalog in `Shared` and owns every subsequent `page()` call.

This is the D1 law applied again — **surfaces do not do IO** — and it is
the minimum change that makes paging possible. `HistoryCatalog` holds
`Box<dyn HistorySource>` and is not `Clone`, so borrowing is also the
only cheap option.

### 3.3 History sessions become rows, beside live sessions

`session_groups` currently walks `providers[].registration.sessions`.
It gains a second source: for each provider in the passed slice,
`self.histories[provider].index.sessions[].summary`, rendered by the SAME
row code under a `RECENT CHATS` heading per provider, sorted by
`modified_millis` **descending** (the operator is looking for what they
were just doing).

This must be a second source, not a replacement, and it must live OUTSIDE
the `providers.iter().all(|p| sessions.is_empty())` early branch — that
branch is what currently swallows the whole feature. Both call sites
(`:1482` scoped to one tab, `:1487` the unscoped roster) then work with
no further change.

Row identity: `SESSIONS_ID.child(provider).child("history").child(locator)`
→ pilot id `chat.sidebar.sessions/<provider>/history/<locator>`. Live rows
key on `ChatSessionId` and history rows on `HistoryLocator`; both are
stable and the id names which kind it is.

The empty-state count label at `:2028` is DELETED — it exists only
because the rows did not. The error branch beside it is KEPT: a history
source that failed to read is still worth saying out loud.

### 3.4 Selecting a history row reads its transcript

Mirrors D1's content seam exactly:

```
 operator clicks a history row
        ├─ ChatWindowIntent::SelectHistorySession { provider, locator }
        │       (window records the focus, clears the stale page)
        └─ HostIntent::LoadHistoryPage { provider, locator }
                 │
 host: catalog.page(provider, locator, None) ◄──┘
        └─ window.observe_history_page(page) ──► pane renders page.events
```

`HistoryPage.next_cursor` is retained for a later "load more"; H1 renders
the first page only and says so truthfully when `truncated_before` is
set. Paging further is deliberately out of scope — one page proves the
seam and the decode.

### 3.5 The conversation pane

When a history session is focused, `ai_conversation` renders
`self.history_page` events instead of the live event stream, with a
banner naming the source (`<workspace> · <started_at>`) and the composer
disabled with a truthful reason — a past transcript is read-only, and
pretending otherwise would be the same dishonesty as the count label.

### 3.6 Scale

`HistoryLimits::default()` already bounds the index scan; 165 files
indexed at construction is one directory walk, already paid today (and
after §3.1, paid exactly once rather than twice). Rendering is bounded by
the rows in view (the rail already scrolls). `page()` is bounded by the
store's own limits.

## 4. Changes, in execution order

1. `crates/app/src/main.rs` — hoist `local_chat_history_catalog()` above
   the registration lists; add `with_local_history`; wrap every entry of
   both `chat_registrations` arms; pass the hoisted catalog into
   `ChatWindowConfig`.
2. `crates/chat/window/src/lib.rs` — constructors take
   `catalog: &HistoryCatalog`.
3. `crates/app/src/chat.rs` — `Shared` gains `catalog: HistoryCatalog`;
   construction passes `&catalog` then stores it.
4. `crates/chat/window/src/lib.rs` — `ChatWindowIntent::
   SelectHistorySession { provider, locator }`, `HostIntent::
   LoadHistoryPage { provider, locator }`, fields `history_focus:
   Option<(ProviderId, HistoryLocator)>` and `history_page:
   Option<HistoryPage>`, plus `observe_history_page`.
5. `crates/chat/window/src/lib.rs` — `session_groups` renders history
   rows; delete the count label.
6. `crates/chat/window/src/lib.rs` — `ai_conversation` renders a focused
   history page + disabled composer with reason.
7. `crates/app/src/chat.rs` — `apply_host_intent` arm calling
   `catalog.page(...)` and pushing `observe_history_page`.
8. Unit pins (§6.3).
9. `tests/eguipilotai/chat-local-histories.toml` — upgrade the scratch
   scenario written during D1 into the real proof (it currently only
   asserts the window opens).

## 5. Boundaries

🚫 **Never touch**

- The transcripts themselves. This pass is READ-ONLY over
  `~/.claude/projects` and `~/.codex/sessions`. Nothing writes, renames,
  moves, or deletes a transcript. No transcript content is sent anywhere
  — it is decoded locally and drawn locally.
- `/content` — unrelated to this pass and untouched.
- The live provider activation paths (Grok/Codex coordinators). §3.1's
  `is_activated()` guard exists precisely so a live activation is never
  overwritten by a history one.
- `dpl-chat-core`'s filesystem-neutrality: `HistoryPage`/`HistoryLocator`
  come from `dpl-chat-store`, which the window ALREADY depends on. No new
  crate dependency is added.

⚠️ **Ask first**

- Any change to the decoders' output shape.
- Anything that would transmit transcript content off the machine.

**Out of scope**

- **P1 — publishing activation to the platform.** The second half of the
  operator's directive. It needs a fabric-carried provider-availability
  signal AND a first real consumer, or it is a publication nobody reads;
  §2.10 shows there is no consumer today. Its own pass, its own plan,
  immediately after H1.
- Pagination beyond the first page (`next_cursor` retained, unused).
- Gemini (no decoder, no source — and §3.1 correctly leaves it `planned`).
- Writing to or resuming a past session.

## 6. Verification

1. `cargo test -p dpl-chat-window -p devpulselabs` — existing pins plus
   §6.3's new ones.
2. `cargo clippy --workspace --all-targets -- -D warnings` → exit 0, no
   `#[allow]`.
3. New unit pins:
   - `local_transcripts_activate_a_planned_provider` — `with_local_history`
     over a catalog with a Claude source yields `is_activated()`.
   - `a_history_activated_provider_still_cannot_send` — `can_send()` is
     false (no `StreamTurns`).
   - `a_live_activation_is_never_downgraded_by_history` — a `Ready`
     registration stays `Ready` and gains the two capabilities.
   - `a_provider_without_transcripts_is_untouched` — Gemini stays Blocked.
   - `history_sessions_become_rows` — a window built with a fixture
     catalog renders a row per indexed session.
   - `history_rows_sort_most_recent_first` — by `modified_millis` desc.
   - `selecting_a_history_row_asks_the_host_for_its_page` — returns
     `HostIntent::LoadHistoryPage` with the same locator.
   - `a_history_page_replaces_the_previous_one`.
   - `a_focused_history_session_disables_the_composer`.
4. `cargo green` on the server1 lane → `green ✔`.
5. **The live-runtime proof — this is the criterion that matters.**
   `tests/eguipilotai/chat-local-histories.toml` against fixture `chat`
   (NOT `chat-authored`) must assert (a) a Claude tab exists in the strip
   — `chat.mode.provider/claude` — and (b) a session row exists under
   `chat.sidebar.sessions/claude/history/…`. This scenario found ZERO
   rows before this pass; it must find rows after. Discard the first run
   after any rebuild (warm-up handshake race).
6. Proof shot on the real exe at 1300×800 showing the operator's actual
   recent chats listed in the rail.
7. Regressions: `chat-content-spine`, `chat-provider-tab`,
   `chat-provider-rows`, `chat-provider-catalog`, `chat-assist-toggle`.
   **`chat-provider-tab` is the one at risk** — §3.1 changes which
   providers earn a tab, so its expectations may legitimately need
   updating. If they do, that is a plan-visible change, recorded in §8,
   not a quiet edit.

## 7. Commit plan

Branch `feat/h1-chat-histories`. One commit:

```
feat(chat): local capability is real activation — the chats in one system (H1)
```

`gh pr list` first. Merge ONLY on double-green.

**Known parallel work:** PR #169 (AI usage domain) touches
`app/src/chat.rs` `Shared` (`:99`), `chat/store` (`HistoryDecoder` trait,
`JsonlHistorySource`), and `chat/window` (`event_row` only). Its hunks do
NOT overlap `session_groups`, the constructors, or the registration list,
so this is distinct work, not a duplicate. Expect a light textual
conflict on `Shared`'s field list; rebase on whichever lands first.

## 8. Evidence

**§6.5, the measure of this pass: `chat-local-histories` against the LIVE
fixture went from ZERO session rows to 10/10 passing**, asserting both the
Claude tab (`chat.mode.provider/claude` — it cannot render unless Claude
actually activated) and the RECENT CHATS heading
(`chat.sidebar.sessions.history` — it renders only when rows exist), in the
roster and scoped to the tab.

- `cargo test -p dpl-chat-window` — 39 pass (33 + 6 new).
- `cargo test -p devpulselabs --bins` — 8 pass (4 + 4 new activation pins).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean, no
  `#[allow]` added.
- Regressions pass: `chat-content-spine`, `chat-provider-tab`,
  `chat-provider-rows`, `chat-provider-catalog`, `chat-assist-toggle`.
  §6.7 flagged `chat-provider-tab` as at risk; it needed no change.
- Proofs: `docs/proof/chat-local-histories-native.png` (live — Claude
  Experimental, its tab, RECENT CHATS, Gemini still Blocked) and
  `docs/proof/chat-transcript-reader-native.png` (a real transcript, banner,
  disabled composer).

### Deviations from the plan, recorded

1. **Two changes the plan did not foresee, both caught by proof shots.**
   (a) The title-bar tab strip had no overflow behaviour, so a sixth tab was
   drawn underneath the window controls — a real regression the moment
   activation started working. Tabs now share the row's remaining width down
   to a 68pt floor. (b) `chat-native-proof` gained an explicit `--live` flag
   rather than special-casing `history` to drop `--chat-authored`, which had
   silently disabled the preset (`main.rs` gates `--chat-proof` behind
   `--chat-authored` on purpose).
2. **`HistoryDecoder`/`HistorySource` gained a `Send` supertrait**, not in
   the plan: `Shared` must be `Send` to hold the catalog. Nothing in either
   trait is legitimately thread-bound, so this is a genuine requirement
   rather than a workaround.
3. **One extra pin beyond §6.3** —
   `choosing_a_live_session_leaves_the_transcript_reader`.
4. **Known-and-not-fixed:** the Claude decoder emits `UNKNOWN` for newer
   record types (`last-prompt`, `ai-title`, `mode`, `permission-mode`), so a
   real transcript reads noisily. §5 puts decoder-shape changes behind
   ask-first, and PR #169 is actively rewriting that decoder. Reported, not
   duplicated.
