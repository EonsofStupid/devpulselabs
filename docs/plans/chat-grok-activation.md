# Grok activation slice

**Status:** IMPLEMENTED AND LOCALLY QUALIFIED — OPERATOR ACCOUNT TEST PENDING
**Date:** 2026-07-26
**Governing ADR:** [`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md)
**Product authority:** [`chat-architecture.md`](chat-architecture.md)
**Extended by:** [`chat-provider-workspaces.md`](chat-provider-workspaces.md)

## Operator correction

The 2026-07-25 slice proved executable resolution and the Grok ACP workspace,
but it did **not** implement the commissioned activation experience. It
launched opaque `grok login`, discarded the provider's authorization prompt,
and delegated navigation to the system browser. That implementation is
superseded by this contract.

Grok is the first complete provider activation. ChatGPT/Codex follows after
this exact experience is manually accepted. Claude and Gemini do not become
live merely because their provider descriptors exist.

## Required outcome

The Grok provider card in the independent DevPULSE Chat window owns one
bounded activation path:

```text
click Grok
    -> select the provider and inspect its activation evidence
    -> no qualified connection: show one Activate Grok action
    -> exact official `grok login --device-auth` child
    -> parse only the bounded xAI verification URL + short user code
    -> render the code and progress inside the Grok provider surface
    -> automatically open the URL in DevPULSE's app-owned Chromium window
    -> operator completes xAI-owned authentication
    -> login child reports completion
    -> close the DevPULSE authorization browser
    -> start the named Grok ACP runtime
    -> cached subscription authentication succeeds
    -> write non-secret activation evidence to the Chat Turso ledger
    -> select or create the Grok conversation and enable normal chat
```

The login child, authorization browser, ACP runtime, and Turso ledger have
separate owners. A successful login-child exit is **never** `Ready`; it moves
to verification and causes a fresh ACP connection attempt. Only the ACP
`cached_token` authentication response can establish `Ready`.

Sessions, turns, tools, approvals, and cancellation remain owned by the
separately tested `GrokAcpRuntime` defined in the extension plan. Council
participation remains unavailable until multi-provider council policy is
qualified.

## Device authorization and browser contract

xAI documents Grok Build as its coding client, `grok login` as the explicit
sign-in command, `--device-auth` as its URL-and-code flow, and
`grok agent stdio` as the ACP boundary for future rich integration:

- <https://docs.x.ai/build/overview>
- <https://docs.x.ai/build/cli/reference>
- <https://docs.x.ai/build/cli/headless-scripting>

DevPULSE launches only the named Grok Build executable with the literal
arguments `login --device-auth`. It captures a bounded stderr stream solely to
extract:

- an HTTPS URL under the exact `accounts.x.ai/oauth2/device` endpoint;
- the provider-issued short user code already present in that URL.

All other output is discarded and never logged. DevPULSE does not construct
an OAuth request, receive a callback, read an auth file, inspect a token, or
call xAI services with provider credentials. Grok Build owns device polling,
credential persistence, and refresh. xAI owns the authorization page.

The accepted URL is opened through `dpl-engine-browser` in a visible,
app-owned, persistent provider-auth profile. It is not sent through
`egui::OpenUrl`, the operating-system default browser, or an operator's normal
browser profile. Browser launch and navigation run off the egui thread.
Closing the browser does not manufacture success; successful ACP verification
closes the owned browser automatically.

The browser executable comes only from the complete, verified Chromium tree
under the release-local `dptools-deps/browsers/` family. Provider
authorization does not silently switch to an installed or `PATH` browser.

This is an attached authentication surface controlled by Chat. It is not a
general webview and cannot navigate to model-, document-, transcript-, or
route-supplied URLs.

## Activation state machine

```text
SignedOut
  | Activate Grok
  v
SigningIn (device child starting)
  | validated accounts.x.ai URL + code
  v
AwaitingAuthorization (code visible; app Chromium open)
  | official login child exits success
  v
Verifying (ACP process starting; cached_token not yet accepted)
  | ACP authenticate succeeds
  v
Ready
```

Any invalid URL, malformed code, browser-launch failure, login-process failure,
ACP rejection, timeout, disconnect, or version mismatch fails closed with one
bounded recovery action. No intermediate state enables Send, New chat, tools,
or Council.

## Turso activation evidence

Chat owns a dedicated local Turso database under
`.userdata/chat/accounts.db`. It records only:

- provider identity;
- `ready`, `signed_out`, or `failed`;
- official client version;
- authentication method (`device_code` or `cached_token`);
- last validated and updated timestamps;
- a bounded public failure classification when applicable.

It never stores device codes, authorization URLs, access tokens, refresh
tokens, cookies, provider auth files, raw provider output, or transcript
content.

The ledger makes startup and recovery intelligible, but it is not an
authentication oracle. A `ready` row is advisory until the current process
completes the real ACP cached-authentication handshake. A failed handshake
updates the row to `signed_out`; a successful handshake replaces it with new
validated evidence.

## Embedded dependency and version boundary

The activation coordinator receives the verified Grok executable from the
target-specific private portable bundle defined by
[`private-tool-depot.md`](private-tool-depot.md).
Production does not search the provider home or `PATH`. The UI, model output,
route context, documents, application configuration, and MCP arguments cannot
supply an executable path or arguments.

The first activation-qualified compatibility band is
`>= 0.2.112, < 0.3.0`. `0.2.112` was the xAI stable channel version recorded
on 2026-07-25. Unknown, malformed, older, and next-major/minor versions fail
closed with an update action. Expanding the band requires a new local
conformance record.

## State and recovery

- `Not installed`: explain that the DevPULSE Grok payload is missing from the
  private release bundle and expose packaging diagnostics.
- `Signed out`: offer **Log in with Grok**.
- `Signing in`: disable duplicate activation while the exact device child
  starts.
- `Awaiting authorization`: keep the short code and progress visible in the
  Grok provider surface; allow the owned authorization window to be reopened.
- `Verifying`: the login ceremony completed and the real ACP cached-token
  handshake is in progress. Send remains disabled.
- `Ready`: means the named ACP child negotiated protocol v1, advertised
  `cached_token`, completed cached subscription authentication, and projected
  its qualified capabilities. A login-process exit cannot set this state.
- `Failed`: show a bounded, non-secret failure reason and offer **Try again**.
- `Version unsupported`: show the detected version and offer the official
  update documentation.

The runtime never reads an auth file or token. When Grok Build is installed, a
fresh DevPULSE process performs one bounded ACP cached-authentication probe. A
successful probe becomes `Ready`; an authentication rejection becomes
`Signed out` and exposes the explicit login action.

## Process containment

- The child is started without a shell.
- Program, arguments, working directory, and inherited environment keys come
  from the compiled Grok profile.
- Standard input and output are null. Standard error is read on a dedicated
  bounded worker solely for validated device authorization material.
- One coordinator owns at most one login child.
- Duplicate login requests are rejected.
- Every frame polls nonblocking completion.
- Drop, application shutdown, cancellation, or replacement kills and reaps
  the exact owned child.
- UI and provider-neutral runtime types never receive a process handle.

## Acceptance

- Unit tests cover version parsing, compatibility rejection, deterministic
  executable resolution, the exact device-auth argument vector, strict xAI URL
  and code parsing, output bounds, duplicate launch rejection, success/failure
  mapping, and owned-child cleanup.
- Browser-auth tests prove only named provider endpoints can open, browser
  work never runs on the UI thread, duplicate windows are rejected, and
  close/reopen lifecycle is bounded.
- Turso tests prove ready/signed-out replacement, persistence across reopen,
  and the absence of token/code/URL columns.
- Chat-window tests prove that a signed-out Grok card exposes one semantic
  login button and produces only `ActivateProvider(grok)`.
- App coordinator tests prove provider actions are rejected for non-Grok
  providers and forwarded only to the Grok coordinator.
- EguiPilotAI proves provider selection, one activation action, visible
  device-code state, browser-reopen action, verifying state,
  ACP-qualified session actions, and the absence of a false Send capability
  before the session transport is ready.
- A real-account native proof requires the operator to click Grok, complete
  xAI authentication in the DevPULSE-owned Chromium window, return
  automatically to Chat, create a session, and exchange one real turn.
- The full local repository gate passes before any branch push.
- A real-account browser login is an explicit operator test because CI and
  deterministic fixtures must never authenticate a personal subscription.

## 2026-07-26 local evidence

- A real Windows portable depot staged Codex `0.145.0-alpha.18`, Grok
  `0.2.112`, Chromium `150.0.7871.129`, and ffmpeg `8.1.2` under one
  target-specific `dptools-deps/` manifest and passed production resolver
  verification.
- Chromium's manifest covers its complete 309-file, 439,360,895-byte tree;
  its fixed PE version is checked without launching the browser.
- Forty focused tests passed across the dependency resolver, Grok
  activation/runtime, app-owned browser boundary, Turso evidence, and Chat
  window.
- `chat-grok-device-authorization.toml` passed all eight steps against the
  real independent Chat viewport, including visible code, enabled owned-browser
  action, and disabled Send.
- Two real native-window captures were byte-identical. The reviewed proof is
  `proof-chat-grok-authorization.png`.
- No real account was authenticated by automation. The remaining operator
  gate is the explicit xAI ceremony and one real Grok turn.
