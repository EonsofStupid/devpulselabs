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

The UI emits provider-neutral intents:

- `ActivateProvider`
- `RemoveProviderWorkspace`
- `RefreshProvider`
- `NewSession`
- `SelectSession`
- `SendTurn`
- `CancelTurn`
- `RetryProvider`
- `SignOutProvider`

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
