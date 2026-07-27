# Chat provider catalog and active-session rail

**Status:** ratified implementation contract  
**Ratified:** 2026-07-27  
**Scope:** AI-provider discovery, configuration entry points, active-session
navigation, and conversation-event presentation

This contract supersedes the fixed provider-row presentation described by the
earlier provider-workspace slice. It does not change the independent native
Chat window, named-provider process boundary, or provider security ADR.

## 1. Information architecture

The AI rail has two different jobs and must not mix them:

1. **AI Providers** is one stable selector. It summarizes the selected
   provider and the currently active provider sessions. Activating it opens
   the provider catalog.
2. **Pinned / Active / Recent** is the session navigator. Provider sessions
   appear here only after discovery, activation, or authored local history
   establishes that they exist.

The rail must not spend permanent vertical space listing every provider.
Unavailable providers remain discoverable in the catalog instead of
disappearing.

The full-width selector displays:

- the selected provider identity and truthful activation state;
- the number of active sessions across providers;
- a disclosure affordance;
- an accessible description explaining that it opens the provider catalog.

The compact 76-point rail displays the selected provider mark and opens the
same catalog. There is one catalog and one selection model at every width.

## 2. Provider catalog

The governed catalog is a transient overlay, not a page or permanent panel. It
contains:

- **Active now** — registered providers with active sessions, including each
  provider's session count and state;
- **Available providers** — every registered built-in adapter, whether ready,
  signed out, unavailable, or not yet qualified;
- **Add a provider** — typed Local and Custom configuration entry points;
- Clyffy as a declared future registered adapter, visibly unavailable until
  its adapter contract is implemented.

The built-in order for this slice is:

1. ChatGPT / Codex
2. Claude Code
3. Gemini
4. Grok
5. Clyffy

The registry, not egui branching, owns labels, icons, status, ordering, and
configuration kind. Adding another built-in adapter is a registry entry.

Selecting a registered provider:

- restores its last active session and draft;
- closes the catalog;
- shows the provider activation ceremony in the conversation when it is not
  connected;
- never fabricates a successful account, session, or turn.

## 3. Profiles and configuration

Provider identity and operator configuration are distinct:

- `ProviderId` identifies a reviewed adapter family such as Grok or Codex;
- `ProviderProfileKey` is an owned identity for an operator-configured
  instance;
- `ProviderConfigurationKind` declares whether the profile is built-in,
  loopback-local, or a custom HTTPS service;
- a typed schema declares the fields accepted by each registered
  configuration kind.

Initial Local and Custom forms support:

- operator-owned display name;
- a loopback HTTP endpoint for Local;
- an HTTPS endpoint for Custom;
- an optional model name;
- validation and an explicit unavailable reason until a compatible transport
  adapter is independently qualified.

The forms cannot accept:

- executable paths;
- command-line arguments;
- environment variables;
- shell commands;
- working directories;
- raw credential values;
- provider-supplied layout, callbacks, or arbitrary code.

Saving a profile records non-secret configuration only. Secrets, when a
future adapter requires them, must use a dedicated credential broker and
never enter Chat history, diagnostics, signals, Turso, or configuration
files.

## 4. Conversation versus diagnostics

Provider protocol events are normalized once and then assigned a presentation
lane:

- **Conversation** — user and assistant messages, plans, tools, diffs,
  approvals, warnings, failures, and meaningful usage summaries;
- **Activity** — session start, transient status, reasoning progress, and turn
  completion;
- **Diagnostics** — bounded unknown/native events and protocol metadata.

Only Conversation events render as transcript rows. Activity updates inform
the header, typing/running state, and trace surfaces. Diagnostic events remain
preserved and inspectable from the provider diagnostics surface.

In particular:

- streamed reasoning fragments never become one `STATUS` card per fragment;
- `EndTurn` never becomes a transcript message;
- ACP `AvailableCommandsUpdate` never dumps its debug representation into the
  conversation;
- hiding an event from the transcript never discards it from the normalized
  history or diagnostic fixture.

## 5. Signals and ownership

Widgets emit typed intents only:

- `OpenProviderCatalog`
- `SelectProvider`
- `OpenProviderConfiguration`
- `SaveProviderProfile`
- `RemoveProviderProfile`
- existing activation, refresh, session, turn, and cancellation intents

The application coordinator owns persistence and adapter activation. Provider
catalog and profile mutations publish typed, bounded provider-domain signals
with provider/profile identity and outcome. Conversation text, endpoints, and
credentials are never signal payloads.

## 6. Slice acceptance

- The full rail contains one AI Providers selector and no fixed provider list.
- The selector reports the current provider and active-session count.
- The catalog exposes ChatGPT/Codex, Claude Code, Gemini, Grok, Clyffy, Local,
  and Custom with truthful state.
- Selecting Grok from the catalog returns to the existing subscription
  activation or ready conversation flow.
- Active sessions remain visible under Pinned / Active / Recent with provider
  identity.
- Local rejects non-loopback endpoints; Custom rejects non-HTTPS endpoints.
- Neither form accepts executable, shell, environment, path, or secret input.
- Raw status, turn-complete, and unknown ACP events do not render as messages.
- Those events remain present in runtime diagnostics.
- EguiPilotAI proves catalog open/close, provider selection, session
  retention, keyboard traversal, and the absence of raw protocol cards.
- A native peer-window capture proves the real Chat viewport, not a root
  screenshot.

## 8. Evidence (2026-07-27)

- `cargo green` (fmt + clippy + full workspace tests + release build) passed
  on this branch; the release exe was rebuilt by the gate.
- EguiPilotAI, real Windows/wgpu process, sequential runs, all passing:
  - `chat-provider-catalog` (authored fixture): catalog opens from the single
    AI Providers selector; Codex/Claude/Gemini/Grok entries, Clyffy, Add
    local, and Add custom all visible without scrolling; Local configuration
    form pre-fills the loopback endpoint with Save enabled.
  - `chat-steam-social` (authored fixture): catalog-first provider selection
    replaces the removed fixed provider rows and hover card.
  - `chat-peer-window` (real state): rewritten to the catalog interaction
    model; full peer-window lifecycle (minimize/restore/show-main/close/
    reopen) still proven.
  - `chat-grok-persisted-ready` (real state): Grok reports Ready through the
    catalog and New chat is enabled.
  - `chat-grok-device-authorization` (grok-auth proof fixture): authorization
    surfaces unchanged.
- `chat-grok-signed-out-readiness` is the paired operator-state proof and is
  mutually exclusive with `chat-grok-persisted-ready`: it can only pass while
  the official Grok client is signed out. On this machine Grok credentials
  persist, so its state-dependent login expectation is unreachable; every
  repaired catalog-sequencing step before it passes. Re-run it at the next
  signed-out operator gate.
- Both Grok scenarios and the peer-window baseline now wait for the child
  viewport's first semantic frame (750 ms) before interacting — the earlier
  failure was this harness race, not a product regression.
- Visual proof: `proof-chat-provider-catalog.png` (native Chat peer viewport,
  not a root screenshot) shows the complete default catalog.
