# 2026-07-29 — connected-provider tabs (POA&M M3)

Plan: `docs/plans/chat-connected-provider-tabs.md`.
Branch: `feat/m3-connected-provider-tabs` off `faf5132` (M1's merge).

## Done so far

The third clause of the operator's regression report — "once connected
it has its own tab and then sessions had" — is built. A provider that is
activated (`ActivationState::is_activated()`, i.e. `Ready` or
`Experimental`) earns a tab in the title-bar strip beside AI and Social;
entering that tab scopes the AI rail to that provider's sessions; the AI
tab remains the roster and the way back.

- `ChatWindowIntent::SelectProviderTab(ProviderId)` and
  `provider_tab: Option<ProviderId>` on the window. `SelectDomain(_)`
  clears the focus, so AI and Social both return the roster.
- `connected_tabs()` is the tab census; `focused_provider_tab()` is the
  focus **filtered through** the census, so a sign-out drops the tab AND
  the scoping in the same frame with nobody sending an intent.
- `provider_tab_header` replaces the roster while scoped: a
  "‹ All providers" way back, then that provider's own M1 row so its
  status and login stay one tap away.
- `session_groups` was already provider-agnostic — scoping is handing it
  `std::slice::from_ref(&focused)`. Zero change to its body.

Verified: 26/26 `dpl-chat-window` tests (22 + 4 new pins), clippy clean
with `-D warnings` and no `#[allow]`, EguiPilot `chat-provider-tab` 15/15
steps plus `chat-provider-rows` / `chat-provider-catalog` /
`chat-assist-toggle` unregressed, proof shot
`docs/proof/chat-provider-tab-native.png` on the real release exe at
1300×800 showing **AI · Codex · Gemini · Social** with Claude (Blocked)
and Grok (Signed out) correctly tab-less.

`git diff --cached --stat` touches exactly ONE file under `crates/` —
the M2 activation-continuity boundary held again.

## Exact next step

Run `cargo green` on the server1 lane, then land via PR on DOUBLE-GREEN
(`gh pr checks <n> --watch`, both lanes COMPLETED SUCCESS). Then rebuild
`target/release/devpulselabs.exe` HERE **without** `--features
eguipilotai-devtools` — the proof capture needed the devtools build, so
the operator's exe must be re-stamped plain at the end. Update STATE.md
and close M3 in the POA&M.

## Landmines

- **The first EguiPilot run after any rebuild fails the handshake**
  ("target did not answer request …0001 … most likely built without the
  pilot"). It is a warm-up race, not a missing feature — the identical
  invocation passes on the immediate re-run. Cost ~20 minutes of false
  diagnosis today. Always discard the first post-build run.
- **`chat-native-proof` needs the RELEASE exe built WITH
  `--features eguipilotai-devtools`.** The shim spawns the app with
  `--chat-proof <state>`, which is devtools-gated; against a plain
  release build the flag is silently ignored, the Chat viewport never
  opens, and the shim dies with "not discoverable as stable native
  windows within 20 seconds" — which reads like a timing problem and is
  not one.
- **`hidden` cannot prove a node was never minted.** It resolves the
  locator first, so an absent node fails with "locator did not match a
  live node". Use `type = "count"`, `value = 0`. Cost one scenario
  round-trip.
- `PilotId::child()` returns a `PilotPath`, not a `PilotId` — taking
  child ids in a widget helper means the helper takes
  `impl Into<PilotPath>`. `mode_button` was widened accordingly.
- The fixed 112px tab clips "ChatGPT / Codex" to "ChatGPT / Co". Provider
  tabs therefore carry `short_label` ALWAYS (full name on hover), and the
  `compact` threshold now grows by one tab-width per connected provider
  so the search field yields instead of being overrun. Both defects were
  invisible until the proof shot — the experience gate earned its keep.

## Side-find (parked, NOT this spine)

`tests/eguipilotai/chat-peer-window.toml` **fails on `main` itself**,
independent of this pass: step 3 expects
`chat.primary/chat.context.route` to read `wiredfront`, and it reads
`workbench`. Proven by stashing this branch's crate change, rebuilding,
and re-running against unmodified `faf5132` — same failure. The default
landing route appears to have moved under #155/#156. Either the
expectation is stale or the default regressed; someone should decide
which. Not fixed here.

## Verify commands

```
cargo test -p dpl-chat-window
cargo clippy -p dpl-chat-window --all-targets -- -D warnings
cargo build -p devpulselabs --features eguipilotai-devtools
target\release\eguipilotai.exe test devpulse-windows tests/eguipilotai/chat-provider-tab.toml
  (discard the first run after the build; re-run)
cargo build --release -p devpulselabs --features eguipilotai-devtools
target\release\chat-native-proof.exe --out docs/proof/chat-provider-tab-native.png --state ai
```
