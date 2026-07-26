# Grok activation slice

**Status:** IMPLEMENTED AS THE ACTIVATION ENTRY TO THE GROK ACP WORKSPACE
**Date:** 2026-07-25
**Governing ADR:** [`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md)
**Product authority:** [`chat-architecture.md`](chat-architecture.md)
**Extended by:** [`chat-provider-workspaces.md`](chat-provider-workspaces.md)

## Outcome

The Grok provider card in the independent DevPULSE Chat window owns one
bounded activation path:

```text
Grok provider launcher
    -> typed ActivateProvider(grok) intent
    -> app-owned Grok activation coordinator
    -> exact official `grok login` child
    -> xAI-owned browser authentication
    -> restart the named Grok ACP runtime
    -> cached subscription authentication succeeds
    -> Ready only after the ACP connection is qualified
```

The activation child remains a narrow provider-owned browser boundary. A
successful login child exit is not itself `Ready`; it causes a fresh ACP
connection attempt. Sessions, turns, tools, approvals, and cancellation are
owned by the separately tested `GrokAcpRuntime` defined in the extension plan.
Council participation remains unavailable.

## Provider-owned browser contract

xAI documents Grok Build as its coding client, `grok login` as the explicit
sign-in command, browser authentication on first launch, and
`grok agent stdio` as the ACP boundary for future rich integration:

- <https://docs.x.ai/build/overview>
- <https://docs.x.ai/build/cli/reference>
- <https://docs.x.ai/build/cli/headless-scripting>

DevPULSE launches only the named Grok Build executable with the literal
argument `login`. It does not construct an OAuth URL, embed a browser, receive
a callback, read an auth file, inspect a token, or call xAI services with
provider credentials. The system browser and all credential/session handling
remain owned by Grok Build and xAI.

## Embedded dependency and version boundary

The activation coordinator receives the verified Grok executable from the
target-specific private portable bundle defined by
[`private-embedded-dependencies.md`](private-embedded-dependencies.md).
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
- `Signing in`: disable duplicate activation and keep the Chat window
  responsive while either cached authentication or the provider-owned browser
  flow is active.
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
- Standard input/output/error are not attached to Chat or retained in logs.
- One coordinator owns at most one login child.
- Duplicate login requests are rejected.
- Every frame polls nonblocking completion.
- Drop, application shutdown, cancellation, or replacement kills and reaps
  the exact owned child.
- UI and provider-neutral runtime types never receive a process handle.

## Acceptance

- Unit tests cover version parsing, compatibility rejection, deterministic
  executable resolution, fixed command construction, duplicate launch
  rejection, success/failure mapping, and owned-child cleanup.
- Chat-window tests prove that a signed-out Grok card exposes one semantic
  login button and produces only `ActivateProvider(grok)`.
- App coordinator tests prove provider actions are rejected for non-Grok
  providers and forwarded only to the Grok coordinator.
- EguiPilotAI proves provider selection, hover-card login action, visible
  connecting state, ACP-qualified session actions, and the absence of a false
  send capability before a session transport is ready.
- The full local repository gate passes before any branch push.
- A real-account browser login is an explicit operator test because CI and
  deterministic fixtures must never authenticate a personal subscription.
