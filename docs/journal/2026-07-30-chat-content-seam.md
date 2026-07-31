# 2026-07-30 — the content seam (D1) + the finding that reorders the program

Program map: `docs/plans/chat-operations-console.md`.
Plan: `docs/plans/chat-content-seam.md` (7 sections + §8).

## Done

**D1 — Chat reads the episode spine.** Before this pass Chat's entire
view of the platform was a `Route` enum; the chat crate tree declared no
dependency on any content crate. Now:

- `dpl-chat-core` gained plain-data content views (`ContentEpisodeId`,
  `ContentEpisode`, `ContentSlide`, `ContentChapter`, `ContentSpine`,
  `ContentView`) and a third domain, `ChatDomain::Content`. The seam
  carries OWNED DATA — no `PathBuf`, no `dpl_doc` types — so the core's
  doc-header promise ("knows nothing about … filesystem discovery")
  stays true and `dpl-chat-window` gained NO content dependency.
- The host translates `dpl_doc` truth (`all_episodes` → `load_episode` →
  `episode_slides` → `dpl_doc::load` for each template) and pushes it in
  via `observe_content` / `observe_content_spine`, mirroring the
  `observe_route` seam that already existed.
- `ChatDomain::Content` is a UNIT variant, so `ChatDomainStates::get/
  get_mut` stay `const fn`. This is not a reversal of M3's rejection of
  `ChatDomain::Provider(ProviderId)` — that variant's cost was entirely
  in the DATA it carried.
- Rail = the census (shows, then seed templates) + Refresh. Board = the
  focused episode's chapters and slides, labelled `<chapter> · <name>`
  (the deck's own convention — a `Slide` document has no title).

Verified: 33/33 `dpl-chat-window` (7 new pins) · 10/10 `dpl-chat-core`
(4 new) · clippy `-D warnings` clean, no `#[allow]` · exactly three
crates touched · `grep dpl-doc crates/chat/**/Cargo.toml` → EMPTY ·
`chat-content-spine` 16/16 steps · `chat-provider-tab`,
`chat-provider-rows`, `chat-provider-catalog`, `chat-assist-toggle` all
unregressed · proof `docs/proof/chat-content-spine-native.png` shows the
Content tab, the seed episode in the rail, and a REAL spine (3 chapters ·
4 slides) on the board.

## THE FINDING — read this before D2

The operator: *"i am building a gosh darn private tool where I am allowed
to have it become convenient to have all the chats available in one
system"*, and *"you have been handwaving me nonstop"*. Both fair.

Measured on this machine 2026-07-30: **128 Claude transcripts across 15
projects + 37 Codex transcripts = 165 real chats on disk.** A
live-runtime EguiPilot snapshot (`chat-local-histories`, fixture `chat`
— NOT the authored fixture) found **ZERO** session nodes in the rail.

The machinery is all built and NOT wired to the screen:

- `ClaudeHistoryDecoder` / `CodexHistoryDecoder` are complete `.jsonl`
  decoders (titles, workspace, timestamps, user/assistant/result).
- `local_chat_history_catalog()` (`crates/app/src/main.rs:1162`)
  registers both against `~/.claude/projects` and `~/.codex/sessions`.
- `ChatWindow::new` (`:548`) indexes that catalog into `self.histories`.

`self.histories` is then consumed in exactly ONE place — the empty-state
branch of `session_groups` — where it becomes a COUNT inside an
unclickable label. Every session ROW is minted from
`provider.registration.sessions`, the LIVE runtime registry, empty for
history-backed providers. **The chats are found, decoded, indexed, and
shown as a number.**

H1 is therefore sequenced NEXT, ahead of D2. It is small: `IndexedSession
.summary` is the SAME `SessionSummary` the rail already renders, and
`HistoryCatalog::page()` returns the SAME `ChatEvent` the conversation
pane already renders. The catalog is a constructor parameter that is
consumed and never stored, so paging must go through the host — exactly
the seam D1 just built.

## My own failure worth recording

M1 (provider rail), M3 (provider tabs), D1 (content seam) — three passes
of chrome around a chat system, and I never once ran the LIVE fixture to
ask whether the chats themselves appear. Every verification I wrote asked
"does my new widget render", never "does the product's purpose work". One
live-runtime snapshot found it immediately, and I only ran it because the
operator pushed. Authored fixtures prove composition; they cannot prove
the product works. Run the live fixture too.

## Landmines (cost real time today)

- **The disk hit 99% (11 GB free of 953 GB) and silently failed a build**
  — `error: There is not enough space on the disk (os error 112)` — while
  `cargo … | tail -3` masked cargo's nonzero exit, so it first read as a
  SUCCESSFUL build producing a stale exe. **Never pipe cargo to tail
  without capturing `$?` separately.** Freed 24.3 GB by removing
  `target/debug/incremental` (pure regenerable cache); left
  `target/debug/deps` and BOTH `dptools-deps` depots alone.
- **Content was read only on the `WindowRequest`/`OpenOrFocus` event, but
  `--chat-open` never raises it** — the Content tab would have been
  permanently empty on that path. Fixed by seeding the census at
  CONSTRUCTION (trigger 0); the plan's §3.4 trigger set was amended too,
  because the PLAN was wrong and would have handed D2 a false model.
- **A missing content root early-returned and suppressed the SEED
  census** — seeds live in `seeds/episodes/`, outside any content root,
  and `all_episodes` unions the two. A fresh install would have shown
  Chat as blind. A missing root is now a NOTE beside a real census.
- `chat-native-proof` validates its `--state` list INDEPENDENTLY of
  `ChatProofMode::parse` — a new state must be added in BOTH or the shim
  rejects it.
- A pilot key holding the `"<show>/<episode>"` address would fake a
  hierarchy level; addresses split into two keys (`split_address`).

## Exact next step

Land D1 on double-green. Then H1 with its own 7-section plan: local
history sessions become first-class rows, selectable, with the transcript
body readable in the conversation pane via a host-side `page()` seam.

## Verify commands

```
cargo test -p dpl-chat-core -p dpl-chat-window
cargo clippy -p dpl-chat-core -p dpl-chat-window -p devpulselabs --all-targets -- -D warnings
grep -n "dpl-doc" crates/chat/*/Cargo.toml crates/chat/*/*/Cargo.toml   # must be EMPTY
cargo build -p devpulselabs --features eguipilotai-devtools
target\release\eguipilotai.exe test devpulse-windows tests/eguipilotai/chat-content-spine.toml
  (discard the first run after any rebuild — the warm-up handshake race)
target\release\eguipilotai.exe test devpulse-windows tests/eguipilotai/chat-local-histories.toml
cargo build --release -p devpulselabs --bin devpulselabs --bin chat-native-proof --features eguipilotai-devtools
target\release\chat-native-proof.exe --out docs/proof/chat-content-spine-native.png --state content
```
