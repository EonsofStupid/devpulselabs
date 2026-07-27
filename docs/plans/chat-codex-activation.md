# ChatGPT / Codex activation slice

**Status:** IMPLEMENTED — real-account login ceremony remains the operator gate
**Date:** 2026-07-25 (ratified) · 2026-07-27 (implementation landed)
**Execution plan:** [`chat-codex-activation-pass.md`](chat-codex-activation-pass.md)
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

## 2026-07-27 local evidence

- The post-login `codex login status` verification moved off the UI thread:
  a `CodexStatusProbe`/`CodexStatusRequest` seam runs the official non-secret
  command on the bounded `devpulse-codex-login-status` worker while the
  coordinator projects a visible `Verifying` state. The UI thread never waits
  on a provider process.
- Fifteen `dpl-chat-codex` tests pass, including the previously untestable
  transitions: SigningIn → Verifying → Ready for "Logged in using ChatGPT";
  API-key status refuses Ready and lands SignedOut with one SignIn action;
  a status-probe failure fails closed with one Retry action; a failed login
  exit never starts verification; duplicate login and rediscovery are
  rejected while a login child or verification is live.
- Activation evidence now mirrors Grok: attempts are minted per activation,
  `ActivationRequested`/`ClientStarted`/`AuthorizationCompleted`/`Ready`/
  `Failed` provider facts feed the activation trace, and the Turso ledger
  records ready/signed-out/failed rows (`auth_method: chatgpt`; failure
  classes `authorization_launch`, `authorization_process`,
  `status_verification`). The boot probe outcome is recorded at startup; a
  fresh process logs `loaded non-secret Codex activation evidence
  state=Ready version=0.145.0 method=chatgpt`.
- `RefreshProvider` now rediscovers Codex; the window's `Verifying` control
  and the `Verifying`/`SignedOut` ceremony copy are provider-generic.
- EguiPilotAI `chat-codex-persisted-ready.toml` passed against the real
  Windows exe: the catalog shows Codex `Ready` and Grok `Ready` as
  independent real states, and with Codex selected both `New chat` and
  `Send` stay disabled — Ready qualifies `SubscriptionAuth` only, exactly
  as the ADR's activation-only clause requires. The five existing chat
  scenarios stayed green.
- Native proof `proof-chat-codex-activation.png` (real peer viewport,
  `--state codex`): the "Connect Codex" ceremony with the single
  "Log in with ChatGPT" action and disabled Send.
- Merge repair recorded: PR #94 (`ProviderEvent::ProfileConfigured`) and
  PR #93 (exhaustive coach-signal matches) merged independently green but
  broke the devtools build together; the coach catalog now names
  `providers.profile-configured` and all seven coach-signal contract tests
  pass.
- The remaining operator gate is the explicit real-account ceremony: sign
  the official Codex client out, click "Log in with ChatGPT", complete
  OpenAI browser authentication, and watch SigningIn → Verifying → Ready
  with the activation trace populated. Deterministic fixtures never
  authenticate a personal subscription.
