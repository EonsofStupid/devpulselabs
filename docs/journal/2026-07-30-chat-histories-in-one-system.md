# 2026-07-30 — H1: local capability is real activation, and the chats are in one system

Plan: `docs/plans/chat-histories-in-one-system.md`. Program map:
`docs/plans/chat-operations-console.md` (link H1).

## What the operator asked for

Twice, and the second quote is what reshaped the pass:

> "i am building a gosh darn private tool where I am allowed to have it
> become convenient to have all the chats available in one system"

> "chat needs to be built so its properly yes its own window but the
> providers that get activated in the chat become available in the platform"

## Done so far

**The defect, in one sentence:** 165 real transcripts were found, decoded,
and indexed at startup, then rendered as a COUNT in an unclickable label,
because every session row was minted from the LIVE runtime registry — which
is empty for history-backed providers.

**The second quote found a second half of it.** `connected_tabs()` says in
its own doc-comment that "a tab is the reward for being connected, so the
predicate is activation and nothing else". Claude registered as
`ProviderRegistration::planned(...)` → `ActivationState::Blocked` → it could
never earn a tab. Adding rows alone, as the first draft of the plan proposed,
would have left the chats homeless in the strip. The plan was amended before
any code was written.

**The model already had every piece.** Nothing was invented:

- `Capability::ReadSessions` and `Capability::PageHistory` were already in
  the enum and had never been qualified by anyone.
- `ActivationState::Experimental { reason }` counts as activated
  (`is_activated()` = `Ready | Experimental`) while still carrying a truthful
  reason string.
- `can_send()` requires `StreamTurns`, which a history-only provider never
  gains — so the composer disables itself with no new gate.

So `with_local_history` (`app/src/main.rs`) qualifies the two history
capabilities and, only when the provider is not ALREADY activated live,
sets `Experimental { reason: "Local transcripts only — live turns are not
qualified yet." }`. Gemini has no history root, so it stays `Blocked` —
correctly, and the pin proves it.

**Ownership inverted, as D1's law requires.** `ChatWindow::new` /
`with_models` now take `catalog: &HistoryCatalog`; the host stores the
catalog in `Shared` and owns every `page()` call. Reading a transcript is IO,
and a surface does not do IO. `HistoryDecoder` and `HistorySource` gained a
`Send` supertrait because `Shared` must be `Send` — a genuine requirement,
not a formality, and nothing in either is legitimately thread-bound.

**The seam mirrors D1 exactly:** `SelectHistorySession` →
`HostIntent::LoadHistoryPage` → `catalog.page()` → `observe_history_page` →
the pane draws `page.events`. Late pages for a transcript already navigated
away from are discarded; the count label at `session_groups` is deleted.

## Verified

- `cargo test -p dpl-chat-window` — **39 passed** (33 before + 6 new).
- `cargo test -p devpulselabs --bins` — **8 passed** (4 before + 4 new
  activation pins).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean, zero
  `#[allow]` attributes added.
- **The criterion that mattered:** `chat-local-histories` against the LIVE
  fixture `chat` — **10/10**. This scenario found ZERO session rows before
  this pass. It now asserts the Claude TAB exists
  (`chat.mode.provider/claude`, which cannot render unless Claude actually
  activated) and the RECENT CHATS heading exists
  (`chat.sidebar.sessions.history`, which renders only when rows exist),
  both in the roster and scoped to the Claude tab.
- Regressions all pass: `chat-content-spine`, `chat-provider-tab`,
  `chat-provider-rows`, `chat-provider-catalog`, `chat-assist-toggle`.
- Proofs on the real exe at 1300×800:
  - `docs/proof/chat-local-histories-native.png` — LIVE path: Claude Code
    **Experimental**, a Claude tab in the strip, RECENT CHATS with a real
    transcript, Gemini still Blocked.
  - `docs/proof/chat-transcript-reader-native.png` — a REAL Claude
    transcript open: banner naming workspace and start time, first-page
    notice, composer disabled reading "Past transcript — read-only".

## Two things the proof shots caught that tests did not

1. **The tab strip had no overflow behaviour.** Once Claude and Grok both
   earned tabs, six 112pt slots plus the title exceeded the row, and the
   window controls — laid out right-to-left in the SAME row — were drawn
   ON TOP of the Social tab. Fixed by giving every slot a share of what is
   left after the title and the right-hand group are reserved, clamped to a
   68pt floor. The old width budget also counted only Content among the
   three always-present tabs, under-budgeting by two slots; that is fixed
   too. Experience-first: this was a regression the moment activation
   started working, and it was invisible to every unit test.

2. **`--chat-proof` is deliberately inert without `--chat-authored`**
   (`main.rs`: a preset "cannot activate a provider or Social transport").
   Special-casing the harness to drop `--chat-authored` silently disabled
   the preset instead of loosening the gate. Reverted in favour of an
   explicit `--live` flag on `chat-native-proof`, leaving the safety gate
   untouched. Authored and live are honest proofs of different things.

## A third defect, caught only by CI

Both hosted lanes went RED on `clippy::sort_by_key` in `history_rows`
after a LOCAL `cargo green` reported `green ✔`. The cause is not the lint
— it is that **cargo caches clippy results per crate**, so a lint already
recorded as clean is not re-run just because the file changed in a way
that did not force a rebuild of that unit. The local gate was reporting a
cached verdict.

**The cure, now a habit:** before trusting a clippy pass on a crate you
just edited, `cargo clean -p <crate>` for the touched crates and re-run.
The hosted lanes always build cold, which is exactly why they caught it
and the warm local gate did not. A green local gate is evidence, not
proof — the double-green law exists for this.

## Landmines

- **The Claude decoder does not recognise newer record types.** The real
  transcript proof shows several `UNKNOWN type=last-prompt / ai-title /
  mode / permission-mode` rows. H1's reader is correct — it renders what
  the decoder yields — but the reading experience is noisy until the
  decoder catches up. **PR #169 is actively rewriting
  `ClaudeHistoryDecoder` (+103 lines)**, so this is deliberately NOT
  touched here; H1 §5 puts decoder-shape changes behind ask-first anyway.
- PR #169 also touches `app/src/chat.rs`'s `Shared` field list and
  `chat/store`. Expect a light textual conflict; rebase on whichever lands
  first. Its window hunks are only in `event_row`, so no logic collision.
- `chat-native-proof` always spawned `--chat-authored`; the new `--live`
  flag is the only way to photograph the operator's real activation.

## Exact next step

**P1 — activated providers become platform capabilities**, the second half
of the operator's directive. H1 made activation TRUE; nothing outside the
chat crates can still SEE it: `ChatRuntime` is a private field of
`ChatWindow` (a surface), the coordinator exposes only `frame`/`attach_*`,
and a workspace grep for `ActivationState`/`Capability::` outside
`crates/chat/` returns just the chat host. P1 must land a fabric-carried
availability signal AND its first real consumer together — a publication
nobody reads proves nothing.

## Verify commands

```
cargo test -p dpl-chat-window                       # 39 pass
cargo test -p devpulselabs --bins                   # 8 pass
cargo clippy --workspace --all-targets -- -D warnings
target\release\eguipilotai.exe test devpulse-windows tests\eguipilotai\chat-local-histories.toml
target\release\chat-native-proof.exe --state history --live --window 1300x800 --out docs\proof\chat-local-histories-native.png
target\release\chat-native-proof.exe --state history --window 1300x800 --out docs\proof\chat-transcript-reader-native.png
```

Discard the first pilot run after any rebuild (warm-up handshake race).
