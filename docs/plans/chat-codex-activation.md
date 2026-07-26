# ChatGPT / Codex activation slice

**Status:** RATIFIED FOR IMPLEMENTATION
**Date:** 2026-07-25
**Governing ADR:** [`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md)
**Product authority:** [`chat-architecture.md`](chat-architecture.md)

## Outcome

The ChatGPT / Codex provider card in the independent DevPULSE Chat window
discovers the official Codex client, asks it for non-secret login status, and
owns one bounded activation path:

```text
ChatGPT / Codex provider card
    -> typed ActivateProvider(codex) intent
    -> app-owned Codex activation coordinator
    -> exact official `codex login` child
    -> OpenAI-owned browser authentication
    -> `codex login status` verification
    -> Ready only for "Logged in using ChatGPT"
```

This slice qualifies ChatGPT subscription activation only. It does not claim
that Codex conversations, tools, approvals, or Council participation are
live. Those capabilities remain disabled until the app-server adapter passes
CHAT-4.

## Provider-owned browser contract

OpenAI documents two Codex authentication methods: ChatGPT subscription
access and usage-based API-key access. The official ChatGPT path is
`codex login`, which opens a browser and returns credentials to Codex. The
official non-secret verification command is `codex login status`.

- <https://learn.chatgpt.com/docs/auth.md>
- <https://developers.openai.com/codex/>

DevPULSE invokes those exact official commands. It does not construct an OAuth
URL, embed a browser, receive a callback, read `auth.json`, use an access
token, accept an API key, or call OpenAI services with Codex credentials.

`Ready` specifically means the status command reports ChatGPT authentication.
An API-key login remains visibly unavailable for the subscription-backed tab.

## Embedded dependency and version boundary

The activation coordinator receives the verified Codex executable from the
target-specific private portable bundle defined by
[`private-tool-depot.md`](private-tool-depot.md).
Production does not search the Codex desktop installation, a user bin
directory, or `PATH`. Shell wrappers, executable paths from configuration,
and all model/UI/route/document/MCP-supplied values are rejected.

The first activation-qualified compatibility band is
`>= 0.145.0, < 0.146.0`, established against the locally installed official
Codex client on 2026-07-25. Expanding the band requires a new local
conformance record.

## State, containment, and acceptance

The state machine, fixed environment allowlist, nonblocking polling, duplicate
launch rejection, owned-child cleanup, setup recovery, and deterministic test
requirements are identical to the named-provider rules in
[`chat-grok-activation.md`](chat-grok-activation.md).

Additional acceptance:

- status parsing distinguishes ChatGPT, API-key, signed-out, and malformed
  responses without retaining raw provider output;
- an already authenticated ChatGPT session starts `Ready` without reading a
  credential file;
- only the exact `codex login` action can start browser authentication;
- successful process exit is followed by `codex login status`; process success
  alone is insufficient;
- EguiPilotAI proves ChatGPT / Codex and Grok expose independent activation
  states and neither falsely enables Send.
