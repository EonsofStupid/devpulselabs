# 2026-08-06 — Chat reaches the AI provider crate

Plan: `docs/plans/codex-host-wiring.md` · Branch: `feat/codex-host-wiring`
Completes: `docs/plans/codex-conversation-transport.md` §4 steps 1, 3, 6, 8.

## Done so far

**The finding that defined the pass.** #210 ("Codex conversation transport")
merged as `fe0e162` and was **half its own plan**. It shipped the schema, the
notification mapping, the `ProviderRuntime` impl and 27 tests — and no child,
no worker, and no host. `CodexAppServerRuntime::from_parts`, `APP_SERVER_ARGS`
and `initialize_params()` had **zero callers** in the workspace. Codex was a
pure, well-tested translation table that nothing could run.

The check that catches this class of thing is *does anything call this?* —
not *did it pass CI?*

**The transport** (`providers/codex/src/runtime.rs`, +344):

- `CodexAppServerRuntime::start(profile)` — three threads: reader (stdout →
  `event_for`), writer (owns stdin, serves the queue), stderr drain.
- `classify()` — a frame is a response iff it has `id` and no `method`.
  Responses omit `jsonrpc`, so the discriminator has to be structural.
- **stdin stays OPEN for the runtime's whole life.** Closing it after the
  handshake makes `app-server` exit; that cost six probe attempts to learn.
- `Connected` is emitted when `initialize` is ANSWERED, not when the process
  spawns. A child that dies first reports its last stderr line instead of
  going quiet.
- **Zero new dependencies** — std threads + the `serde_json` already there.
  Grok's `async-process`/`async-channel` stack was not copied; `from_parts`
  had already committed to `std::sync::mpsc`.
- `activation::profiled_command` became `pub(crate)` so the app-server child
  and the login probes cannot drift into different environments.

**The host** (`app/src/chat.rs`, +583): `Shared` gained `runtimes:
BTreeMap<ProviderId, Box<dyn ProviderRuntime>>` plus `seam_sessions` /
`seam_turns` keyed by `(ProviderId, String)` — two providers can mint the same
session string, and an unkeyed map would splice two conversations into one
transcript. Then `drive_codex_runtime` · `poll_seam_runtimes` ·
`take_seam_event` · `project_seam_event` · `project_seam_update` ·
`append_seam_event` · `set_seam_session_idle` · `send_seam_command` ·
`send_seam_turn` — all provider-parameterised, so the next provider costs a
`start()` and no host code.

The intent gates stopped asking *"are you Grok?"* and started asking *"do you
have a runtime?"*. Grok keeps its bespoke path untouched (its convergence is
its own pass).

**A latent defect fixed on the way:** `provider_session_id()` resolved through
`provider_view(grok::PROVIDER)` regardless of who asked — harmless only while
Grok was the sole provider with live sessions. It now takes a `ProviderId`.

## The proof

`crates/chat/providers/codex/examples/codex_turn.rs` drives the SAME runtime
the app drives, through the SAME trait the host calls:

```
executable: target\release\dptools-deps\providers\codex\0.145.0-alpha.18\codex.exe
version:    0.145.0
connected; advertised RuntimeCapabilities { load_session: false, list_sessions: false }
thread:     019fd746-ff90-7673-9c91-d972246a2bf8
usage:      19255 of 258400
stop:       completed
ASSISTANT ANSWER: "WIRED"
```

An example rather than a test on purpose: it needs a real login and the
network, and a test that needs either is one CI will eventually run.

**The UI witness** (`tests/eguipilotai/chat-codex-persisted-ready.toml`,
re-run live and PASSING): Codex Ready → type a draft → **Send is enabled**.
Before this pass that button's tooltip read *"This provider's live turn
transport is not qualified."* And in the app's own log, under the pilot:

```
provider runtime connected provider=codex can_send=true
                           qualified=[StartSession, StreamTurns, InterruptTurn]
```

Getting there required correcting two claims I had asserted without checking
— both found only by RUNNING it:

1. I called `chat-peer` an authored fixture. It is not: `.eguipilotai.toml`
   passes `--chat-open` with no `--chat-authored`, so the real app-server
   child starts. Its old `expect disabled` assertions were pinning LIVE
   behaviour this pass changes.
2. I then asserted `enabled` on an EMPTY composer. Send needs four things and
   only one is the transport: not reading history · a session selected ·
   `can_send()` · **a non-empty draft**. An empty composer is disabled with
   "Type a message first." however qualified the provider is — that assertion
   would have passed before and after, proving nothing.

The code was right both times; the assertions were wrong.

**The live run found a real defect the unit tests could not.** The first turn
reported `usage: 0 of 0`. The pinned schema said why: `ThreadTokenUsage` is
**nested** — `tokenUsage.total.totalTokens` and `tokenUsage.modelContextWindow`
— and the adapter read `totalTokens` at the top level, which parses cleanly and
returns zero forever. This is precisely what pinning a schema is for: guessed
field names fail silently, pinned ones fail loudly. Fixed, pinned by two tests,
and re-proved live.

## Exact next step

Land this (double-green), then the operator's call between:

1. **Widen the seam's `Usage`** — Codex reports a full input/cache/output
   split and `RuntimeUpdate::Usage` carries only context occupancy, so the
   split is DROPPED. It is not folded (a folded total cannot be taken apart
   again); it is honestly absent. Restoring it means widening the seam and
   touching Grok's seam ADAPTER — a design decision, not a fix.
2. **The passthrough resolver** — `PINNED_CLI_VERSION` is `0.146.0-alpha.3.1`
   (the PATH binary) while the app runs the depot's `0.145.0`. Both work for
   the four methods used, and the mismatch is now logged rather than silent.
   Passthrough first, depot fallback second, and the fallback stated aloud.
3. **`ai-chat-social-separation.md`** — the `crates/ai/` rename, which renames
   these very files.

## The blocker this pass hit (not its own)

`green-windows` went red on #213 for a reason #213 did not cause:
`dpl_index::tests::a_folder_symlink_is_unlinked_never_recursively_deleted`.
`crates/index` is not in this diff; it was last touched by #211. **Main was
red for everyone** — #213 was simply the first Windows lane to run against it.

`Index::dump` unlinked symlinks with `remove_file`, but a Windows DIRECTORY
symlink needs `remove_dir`, so a referenced FOLDER could never be dumped on
Windows. Fixed on its own branch as **#214**, which must land before #213 can.

Why every local gate missed it, and the reason worth keeping: `try_symlink`
returns false on a Windows machine without Developer Mode, and both symlink
tests then return early **while still reporting `ok`**. A local `cargo green`
never executed the assertion — a skip wearing a pass's clothes. The skip
messages now read `SKIPPED (NOT PROOF)`.

## Landmines

- **stdin.** Closing it kills `app-server`. The writer thread holds it for its
  whole life; only `Shutdown` or a dead pipe releases it.
- **Nested usage.** See above. Any new field read from this protocol gets its
  path confirmed against `schema/`, never guessed.
- **The example needs the depot staged beside it.** `CodexRuntimeProfile::
  discover` resolves `dptools-deps/manifest.json` relative to the exe, so
  `target/debug/examples/` needs it. A junction works — **and must be removed
  afterwards**: a recursive delete through one previously wiped `target/`.
- **Grok is untouched by design.** Its runtime, retry state and permission
  handling still run on the bespoke path. Do not "tidy" the two together
  without a pass that says so.
- **Approvals are declined, not allowed.** No seam provider advertises them;
  the host answers `option_id: None` rather than act for the operator.
- Cold clippy caught a `while_let_loop` a warm run would have missed. Again.

## Verify commands

```
cargo clean -p dpl-chat-codex -p dpl-chat-runtime -p devpulselabs
cargo clippy -p dpl-chat-codex -p devpulselabs --all-targets -- -D warnings
cargo green                                   # → green ✔
cargo test -p dpl-chat-codex                  # 33 tests
cargo run -p dpl-chat-codex --example codex_turn -- "Reply with exactly: OK"
```
