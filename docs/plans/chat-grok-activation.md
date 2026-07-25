# Grok activation slice

**Status:** RATIFIED FOR IMPLEMENTATION
**Date:** 2026-07-25
**Governing ADR:** [`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md)
**Product authority:** [`chat-architecture.md`](chat-architecture.md)

## Outcome

The Grok provider card in the independent DevPULSE Chat window owns one
bounded activation path:

```text
Grok provider card
    -> typed ActivateProvider(grok) intent
    -> app-owned Grok activation coordinator
    -> exact official `grok login` child
    -> xAI-owned browser authentication
    -> Ready only after a successful child exit
```

This slice qualifies subscription activation only. It does not claim that
Grok conversations, sessions, tools, approvals, or Council participation are
live. Those capabilities remain disabled until the ACP adapter passes CHAT-7.

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

## Discovery and version boundary

The activation coordinator resolves only:

1. the official user install location, `%USERPROFILE%\.grok\bin\grok.exe` on
   Windows or `$HOME/.grok/bin/grok` on Unix; then
2. a file named `grok.exe`/`grok` from the process `PATH`.

The UI, model output, route context, documents, configuration files, and MCP
arguments cannot supply an executable path or arguments.

The first activation-qualified compatibility band is
`>= 0.2.112, < 0.3.0`. `0.2.112` was the xAI stable channel version recorded
on 2026-07-25. Unknown, malformed, older, and next-major/minor versions fail
closed with an update action. Expanding the band requires a new local
conformance record.

## State and recovery

- `Not installed`: explain that Grok Build is required and offer
  **Get Grok Build**, which opens the official documentation. DevPULSE never
  downloads or executes an installer.
- `Signed out`: offer **Log in with Grok**.
- `Signing in`: disable duplicate activation and keep the Chat window
  responsive while the provider-owned browser flow is active.
- `Ready`: means only that the current activation command completed
  successfully. Send remains disabled until ACP capabilities are qualified.
- `Failed`: show a bounded, non-secret failure reason and offer **Try again**.
- `Version unsupported`: show the detected version and offer the official
  update documentation.

The runtime does not infer a signed-in account from files. A fresh process
starts in a truthful "not verified by this DevPULSE session" signed-out state
until the operator runs the explicit login action.

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
  signing-in state, and the absence of a false send capability.
- The full local repository gate passes before any branch push.
- A real-account browser login is an explicit operator test because CI and
  deterministic fixtures must never authenticate a personal subscription.
