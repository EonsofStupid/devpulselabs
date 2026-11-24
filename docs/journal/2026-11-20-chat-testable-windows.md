<!-- Preserved draft evidence; see main-reconciliation.md for current disposition. -->
> Historical draft at 2a7d0a5, based on obsolete main 1807459. The captures
> and earlier green runs below do not qualify the reconciled build. The
> build/library-folder project substitution was rejected; current main's
> project/trust code is retained. Current delivery: docs/plans/main-reconciliation.md.

# Chat platform awareness — R308 correction

## Final local verification — 2026-11-21

The correction is built and locally verified. Final gate log
`target/chat-awareness-green-composer.log` is green: 1,403 passed, six ignored,
release rebuilt and renderer smoke passed. Chat-window tests: 61 passed.
Final native scenario: nine steps passed, run
`target/eguipilotai/runs/run-89456-18ea1a8e3701f74c`.
Final popup captures at 1440×900 and 1100×700 were opened and inspected.

Live Codex connected automatically, New chat created a real session with the
Skeptic role, and Send produced an actual reply naming `devpulselabs` and
`wiredfront/viziwiz` from supplied platform context. The live proof image is
`docs/proof/chat-testable-windows/live-codex-platform-reply.jpg`. No project setup
or permission change occurred. Conversation saved and Chat peer closed normally.
The isolated test parent process 109756 remains open; it owns scratch storage
under `target/chat-platform-live/library`.

Live proof caught and fixed duplicate permission IDs, unbounded role drafts
pushing Send off-screen, and unrelated Grok failures appearing in Codex's popup.
Native and unit regressions cover these. The full history below records the
earlier in-progress checkpoints, not the final evidence.

## Done so far

Uncommitted pass in the existing app/chat crates. The operator explicitly
rejected chat-owned project setup: chat must know the platform's project.
`ChatWindowConfig` now receives app-owned `PlatformContext`. Normal startup
opens userdata/chat/conversations automatically and initializes the named
provider adapters against that workspace. Legacy selection intents cannot
replace supplied platform context. The normal project picker is removed.
Missing storage offers Retry in the context bar. Provider prompts receive the
workspace, project label and current shell route without permission changes.

The earlier draft also added a New chat popup, built-in role starters, passive
bounded provider checks, manual refresh and devtools-enabled `cargo xtask dev`.
Roles prepare a draft only after a new session is acknowledged. Existing
per-session drafts remain intact. Prior archives are copied, never deleted.

Before the last recovery refinements: app tests 84 passed, project-store tests
29 passed; window tests had one obsolete project-picker assertion, now corrected.
Full `cargo green` is running (log `target/chat-awareness-green-v3.log`); fmt,
structure/spawn/contract audits and clippy passed before final small edits.
Do not report a full pass yet.

## Exact next step

**Supersedes the earlier step below:** the requested platform-awareness
correction has passed local acceptance. Run `cargo xtask dev -- --chat-open`
for the operator's dev session. Remaining broader product work is Claude's
transport, Grok authentication qualification, Meta Muse identity/integration,
and native session resume; none is reported as delivered. Changes are local
and uncommitted. Before any merge, follow the repository's PR/double-green rule.

Plan step 4: read `target/chat-awareness-green-v3.log`, finish the full gate,
build the devtools app/proof binary, then capture and inspect corrected native
1440×900 and 1100×700 popups. Run `tests/eguipilotai/chat-new.toml`. Exercise a
real provider session from New chat without any project setup. Record failures
honestly and amend this journal/status with actual results.

## Landmines

- Old `docs/proof/chat-testable-windows/new-chat*.png` show the rejected flow.
  Keep them as history; they are not corrected acceptance evidence.
- The prior live process 26816 was stopped; no current live UI proof exists.
- Codex and Grok have transports. Claude detection is not a Claude chat adapter.
  Meta Muse's product identity/integration remains unestablished.
- Native resume remains gated; restored archives are read-only.
- Approved staged debug depot contains Codex 0.163.0 and Grok 0.2.112. Local
  ignored `dptools-deps/sources.local.toml` points to the existing official Codex
  artifact; its prior contents were copied to `target/chat-testable-sources-original.toml`.
- `cargo clean -p dpl-chat-window -p dpl-chat-projects -p devpulselabs -p xtask`
  ran before this full gate. Rebuild the app before any UI claim.
- Chrome retry test now runs in its own test process to avoid races with the
  global library authority. Test-only launch lives in `test_isolation.rs` so
  the app's CLI flag census does not mistake libtest's `--exact` for an app flag.
- Preserve older journals (operator's never-delete law overrides skill cleanup).

## Verify commands

```powershell
cargo green
cargo test -p dpl-chat-projects -p dpl-chat-window -p devpulselabs --features devpulselabs/eguipilotai-devtools
cargo build -p devpulselabs --features eguipilotai-devtools
./target/debug/eguipilotai.exe test devpulse-windows tests/eguipilotai/chat-new.toml
```

Use explicit scratch CONTENT_ROOT, ASSETS_ROOT and USERDATA_ROOT for native
proof and live testing. Normal operator launch is `cargo xtask dev -- --chat-open`.
