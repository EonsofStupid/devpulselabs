# Chat provider workspaces — Grok, then ChatGPT/Codex

**Status:** ratified implementation contract
**Ratified:** 2026-07-25
**Scope:** provider activation, sidebar ownership, sessions, live turns, and
local proof for Grok followed by ChatGPT/Codex

This plan corrects the activation-only provider presentation without changing
the independent native Chat-window architecture.

> **2026-07-27 presentation amendment:** provider adapters remain visible and
> truthful, but they now live in the governed provider catalog rather than as
> permanent launcher rows. The rail's stable control summarizes the selected
> provider and active sessions; provider-owned sessions remain in
> `Pinned / Active / Recent`. The canonical replacement contract is
> [`chat-provider-catalog.md`](chat-provider-catalog.md).

## 1. Locked delivery order

This qualification pass changes implementation order, not product scope:

1. Grok Build through its official Agent Client Protocol surface.
2. ChatGPT/Codex through the official local Codex app-server.
3. Stop and test both providers before another provider adapter is implemented.

Claude, Gemini, future providers, Councils, People, and the Interaction Library
remain visible in the authored Chat information architecture. Unsupported
provider actions carry truthful availability state. They are not removed,
hidden, or replaced merely because Grok and ChatGPT/Codex are the first two
adapters being made live.

## 2. Correct navigation ownership (superseded presentation)

The screenshot supplied by the operator on 2026-07-25 remains authority for
session identity, popovers, activation, and density. Its fixed provider-row
presentation is superseded by the 2026-07-27 provider-catalog amendment.

- The strong left rail always includes the AI provider launcher rows:
  ChatGPT/Codex, Claude, Gemini, Grok, and later registered providers.
- A provider row contains its icon, label, truthful account/runtime state,
  unread/activity state, and disclosure affordance.
- Selecting or hovering a provider opens the compact governed provider
  popover over the rail. It contains subscription state, session count,
  `New chat`, `Add to Council`, and `Account`.
- Connecting a provider does not replace or remove its launcher row.
- Provider-owned sessions appear dynamically only after discovery or
  connection establishes that they exist.
- Sessions share the rail's `Pinned`, `Active`, and `Recent` groups and retain
  visible provider identity and live state.
- `New chat`, Councils, People, Interaction Library, and Settings remain
  durable rail destinations.
- Selecting a provider restores that provider's last active session, draft,
  scroll anchor, model/mode state, overlays, and attached utilities.
- Selecting a session never implies cross-provider continuation.
- Signing out keeps the launcher row and truthful recovery state.

Activation progress is shown inside Chat:

```text
AI provider row
  -> open provider popover
  -> official provider login opens
  -> Chat shows Connecting to provider
  -> provider-owned authentication completes
  -> Chat refreshes capability and session state
  -> sessions populate Pinned / Active / Recent
  -> last valid session loads, or New chat remains available
```

Chat never asks for a subscription token. The provider owns credentials and
the system browser. Successful activation does not fabricate a session or a
message.

## 3. Shared provider-runtime boundary

The UI emits provider-neutral intents. The list below is the contract as
ratified 2026-07-25; per-intent shipped status was measured 2026-08-06 and
reconciled in the subsection that follows:

- `ActivateProvider`
- `RemoveProviderWorkspace` *(never built — OPEN GAP, see reconciliation)*
- `RefreshProvider`
- `NewSession`
- `SelectSession`
- `SendTurn`
- `CancelTurn`
- `RetryProvider` *(superseded — see reconciliation)*
- `SignOutProvider` *(superseded — see reconciliation)*

### Intent status — measured 2026-08-06, reconciled 2026-08-07

Measured against `crates/`, the ratified list shipped in four shapes.

**Live as intents** (`ChatWindowIntent` → `HostIntent`, consumed in
`crates/app/src/chat.rs`):

- `ActivateProvider` — 8 references. Intent and host arms
  (`crates/chat/window/src/lib.rs:321`, `:430`), mapped at `:1074`,
  emitted from the provider surface at `:3680`, handled at
  `crates/app/src/chat.rs:1311`.
- `RefreshProvider` — 6 references. Same shape (window `:324`, `:433`,
  `:1083`, `:3709`; host handling `crates/app/src/chat.rs:1363`).
- `NewSession` — live end to end: intent and host arms (window `:330`,
  `:437`, mapped `:1093`), handled at `crates/app/src/chat.rs:1406`, which
  fans into `GrokRuntimeCommand::NewSession`
  (`crates/chat/providers/grok/src/runtime.rs:916`) and the seam's
  `RuntimeCommand::NewSession` (`crates/chat/runtime/src/seam.rs:151`,
  consumed by codex at `crates/chat/providers/codex/src/runtime.rs:73`).
- `SendTurn` — intent and host arms (window `:331`, `:438`, mapped
  `:1094`), handled at `crates/app/src/chat.rs:1418`.
- `CancelTurn` — intent and host arms (window `:336`, `:443`, mapped
  `:1106`), handled at `crates/app/src/chat.rs:1434`.

**Live, but window-local — never a host intent:**

- `SelectSession` — `ChatWindowIntent::SelectSession`
  (`crates/chat/window/src/lib.rs:293`) is handled inside the window at
  `:1006`: `self.runtime.select_session(...)` then `Ok(None)`. Selection
  never crosses the host boundary, because the window's runtime owns
  session truth — a narrower shape than the flat list implies, not a gap.

**Superseded — the capability shipped inverted:**

- `RetryProvider` and `SignOutProvider` were never built as intents (zero
  references in `crates/`). What shipped instead is provider-emitted
  recovery affordances: `RecoveryActionKind::{InstallInstructions, SignIn,
  Retry, UpdateProvider, OpenDiagnostics}` carried on
  `RecoveryAction { kind, label }` (`crates/chat/core/src/lib.rs:566`).
  Providers attach them to their own failure states (grok
  `crates/chat/providers/grok/src/lib.rs:489`, `:505`; codex
  `crates/chat/providers/codex/src/activation.rs:525`), and the window
  renders the first recovery action as the login-area button — a pressed
  `SignIn` or `Retry` maps back into `ActivateProvider`
  (`crates/chat/window/src/lib.rs:3666`). The inversion is truthful in a
  way the intent shape was not: the provider states which recovery it
  needs rather than the UI guessing, so retry and sign-in exist as
  provider-declared affordances, not UI-declared intents. This supersedes
  the two listed intents.

**Open gap:**

- `RemoveProviderWorkspace` — no analogue anywhere in `crates/`; nothing
  removes a provider workspace today. **OPEN GAP**, owned by
  [`loss-recovery-program.md`](loss-recovery-program.md) Arc R4 (ruling
  Q6: the removal implementation follows this reconciled contract).

The app-owned provider coordinator owns the named adapter and maps immutable
events into `ChatRuntime`. Widgets never own a child process, parse a provider
message, or mutate session truth.

Every live profile is pinned to:

- a typed provider/profile identity;
- a discovered, supported executable;
- explicit operator-approved workspace roots selected by application policy;
- a documented protocol and version;
- bounded command, event, output, diagnostic, and attachment sizes;
- explicit cancellation, shutdown, timeout, and child-reaping behavior.

No UI field can supply an executable, argument, environment variable, working
directory, shell command, or artifact path.

The first DevPULSE profile uses the canonical directory from which the
application was launched as its approved coding workspace. It does not reuse
the media/content root. A later named-project registry may select other
predeclared roots, but provider output and conversation text can never replace
the profile root.

## 4. Grok Build contract

### Official surface

The named `GrokAcpRuntime` launches the discovered Grok Build executable with
the fixed `--no-auto-update agent stdio` argument sequence. It uses ACP
protocol version 1 and the official Rust ACP schema/client library.

The handshake is:

1. `initialize` with protocol version 1 and a DevPULSE client identity.
2. Inspect the returned capabilities and authentication methods.
3. Accept only Grok's `cached_token` subscription authentication method.
4. `authenticate` with provider-owned cached authentication.
5. List sessions only when `sessionCapabilities.list` is advertised.
6. Load an existing session only when load is advertised.
7. Otherwise create a session with the application-approved jailed workspace.

Grok is a full coding-agent surface. DevPULSE implements the ACP filesystem,
terminal, permission, and tool-facing client capabilities required for coding
work instead of suppressing them. Capabilities are governed:

- project roots and additional roots are explicit and visible;
- filesystem and terminal requests are normalized into typed actions;
- provider permission requests become Interaction Library approval overlays;
- `Allow once`, `Always allow`, and `Deny` are operator decisions;
- text, diff, plan, tool, and usage results normalize into bounded Chat
  events; image, audio, and video payloads are represented by bounded metadata
  summaries in this pass so encoded media can never flood the event queue;
- promoting qualified media and artifact payloads into attached utility
  windows remains a separately testable follow-on and is not claimed by the
  first Grok runtime;
- MCP servers and app capabilities are selected from named project policy,
  never provider- or prompt-supplied commands;
- unknown updates remain bounded and visible rather than becoming fake
  success.

This is a personal development cockpit, so the policy supports full agent
power. Governance exists to make that power observable and predictable, not
to reduce Grok to plain text chat.

### Session and turn behavior

- `session/list` cursors are opaque and bounded.
- `session/new` creates the provider-native session before Chat creates its
  binding.
- `session/load` replays normalized user and agent events before selection is
  declared ready.
- `session/prompt` streams `session/update` chunks into one stable turn.
- `session/cancel` is sent for Stop and app shutdown when a turn is active.
- A prompt finishes only after its prompt response supplies a stop reason.
- Disconnect, malformed envelopes, protocol mismatch, authentication failure,
  timeout, queue overflow, cancellation failure, and child exit are distinct
  typed failures.

The adapter preserves bounded, redacted unknown ACP updates for diagnostics.
It never emits a fake assistant completion.

### Grok acceptance

- A deterministic in-memory ACP peer proves initialize, cached-token
  authentication, list/new/load, streaming, cancellation, malformed input,
  backpressure, and teardown.
- Production discovery proves the fixed executable and arguments cannot be
  influenced by UI state.
- EguiPilotAI proves provider popover, Connecting, dynamic session discovery,
  New chat, session selection, streaming, Stop, permissions, tool/artifact
  surfaces, recovery, keyboard traversal, and semantic identities.
- A real installed Grok Build executable passes the same lifecycle contract
  before the provider is marked qualified.
- If Grok Build is absent, Chat shows `Not installed`, the official setup
  destination, and hot rediscovery. It does not silently install or run a
  downloaded script.

## 5. ChatGPT/Codex contract

Only after Grok passes locally, implement a named `CodexAppServerRuntime`
against the pinned official Codex app-server protocol.

- Authentication remains ChatGPT-owned.
- The provider workspace and session behavior is identical to Grok at the UI
  contract boundary.
- Codex-specific threads, turns, plans, diffs, tools, approvals, usage, and
  interrupt events normalize behind typed capabilities.
- There is no ACP shim, direct OpenAI API fallback, or reuse of a ChatGPT
  credential outside the official Codex process.
- The same deterministic, real-executable, EguiPilotAI, teardown, and native
  proof gates apply.

## 6. Local-first delivery gate

Each provider stops at a local evidence checkpoint:

1. format and warning-free focused tests;
2. protocol fixtures and lifecycle tests;
3. complete local workspace gate;
4. real native Chat-window proof;
5. operator subscription login and live send/stream/stop test;
6. only then may the branch be pushed or proposed for merge.

No GitHub run substitutes for a missing local gate. No second provider begins
until Grok's deterministic implementation is green; live operator login may
remain an explicit final qualification step when the provider executable is
not installed.
