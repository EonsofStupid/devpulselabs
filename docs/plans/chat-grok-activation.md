# Grok activation slice

**Status:** REAL AUTHORIZATION PERSISTED — POST-LOGIN VERIFICATION SUPERVISOR IN PROGRESS
**Date:** 2026-07-26
**Governing ADR:** [`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md)
**Product authority:** [`chat-architecture.md`](chat-architecture.md)
**Extended by:** [`chat-provider-workspaces.md`](chat-provider-workspaces.md)

## Operator correction

The 2026-07-26 operator account test exposed a second boundary error: the
visible authorization window was launched by `dpl-engine-browser`, which
always enables a Chrome DevTools Protocol remote-debugging port and performs
navigation through CDP. A visible window does not make an automation-driven
browser a human browser. xAI's Cloudflare boundary rejected that session.

This contract now makes the browser split explicit:

- `dpl-engine-browser` and the bundled Chromium payload are automation
  infrastructure for EguiPilotAI, capture, and authored browser drills;
- provider authentication opens the validated xAI device URL in the
  operator's operating-system default browser;
- DevPULSE never attaches CDP, injects input, reads browser state, manages
  cookies, or claims ownership of the resulting human authentication window.

The previous app-owned Chromium implementation and its deterministic proof
remain useful evidence for the Chat ceremony and browser-action semantics,
but they are superseded as real-account authentication evidence.

The 2026-07-25 slice proved executable resolution and the Grok ACP workspace,
but it did **not** implement the commissioned activation experience. It
launched opaque `grok login`, discarded the provider's authorization prompt,
and delegated navigation without retaining the device ceremony in Chat. The
new system-browser handoff does not restore that incomplete presentation:
Chat still owns the code, Copy/Reopen actions, trace, readiness transition,
and session focus.

Grok is the first complete provider activation. ChatGPT/Codex follows after
this exact experience is manually accepted. Claude and Gemini do not become
live merely because their provider descriptors exist.

## Required outcome

The Grok provider card in the independent DevPULSE Chat window owns one
bounded activation path:

```text
click Grok
    -> select Grok and make activation the dominant conversation content
    -> no qualified connection: show one Connect Grok action
    -> exact official `grok login --device-auth` child
    -> parse only the bounded xAI verification URL + short user code
    -> retain both as provider-authored guided values in bounded memory
    -> render the exact code, one-click Copy, guidance, and terminal trace
       inside the Grok conversation
    -> automatically open the URL in the computer's default browser
    -> operator completes xAI-owned authentication
    -> login child reports completion
    -> start the named Grok ACP runtime
    -> cached subscription authentication succeeds
    -> write non-secret activation evidence to the Chat Turso ledger
    -> select or create the Grok conversation, focus the composer, and enable
       normal chat
```

The login child, system-browser handoff, ACP runtime, and Turso ledger have
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

The accepted URL is handed to the operating system's default-browser service
on a bounded worker, never to `dpl-engine-browser`, CDP, a webview, or an
embedded Chromium surface. The worker accepts only the exact validated Grok
device endpoint already represented by `DeviceAuthorizationPrompt`.
Successful dispatch publishes a redacted `SystemBrowserDispatched` fact;
dispatch failure moves the attempt to a retryable browser failure.

DevPULSE cannot observe whether the operator closes the system browser and
does not publish a browser-closed fact. Closing or abandoning the page cannot
manufacture success. The official `grok login --device-auth` child continues
polling xAI and remains the sole authority for authorization completion.

This narrow default-browser handoff is not a general URL launcher. Model
output, documents, transcripts, route state, application configuration, MCP
arguments, and external signal payloads cannot supply its URL.

## Activation state machine

```text
SignedOut
  | Activate Grok
  v
SigningIn (device child starting)
  | validated accounts.x.ai URL + code
  v
AwaitingAuthorization (code visible; system browser requested)
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
  Grok conversation; automatically open the computer's default browser, expose
  one-click Copy and Reopen, and retain a compact expandable activation trace.
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

## Durable subscription ownership and verification supervision

The 2026-07-26 operator test completed xAI authorization, then exposed a
DevPULSE lifecycle race:

```text
official login child exits success
    -> DevPULSE starts or reuses an ACP child immediately
    -> first ACP authentication reports a retryable authorization failure
    -> DevPULSE prematurely projects SignedOut
```

Independent probes against the exact bundled `0.2.112` executable proved that
the provider credential was already durable:

- `grok --no-auto-update models` reported an active grok.com login;
- the exact ACP protocol-v1 `initialize -> cached_token -> authenticate`
  exchange succeeded under the same compiled environment allowlist;
- no credential file was opened, copied, parsed, or logged by DevPULSE.

The open-source `0.2.112` Grok Build implementation confirms the intended
boundary. Device authorization calls `AuthManager::update`, which writes
`auth.json` before the login function returns. The write is owner-only,
flushes and syncs before publication, and uses an atomic temporary-file
replacement when storage permits. Grok Build also owns token refresh and
multi-process credential reconciliation. The reviewed source revision is
[`b41c75a`](https://github.com/xai-org/grok-build/commit/b41c75a578f98bddbd326ab02cd53618451d97ee).

Therefore:

- DevPULSE must never copy Grok credentials into Turso, `.userdata`,
  `dptools-deps`, a release directory, a project, or an application-owned
  "portable auth" file.
- The application and bundled executables are portable. Authentication remains
  private to the operating-system user profile by default. Moving DevPULSE on
  the same machine retains login; moving it to another user or machine
  intentionally requires authorization again.
- DevPULSE passes only the compiled provider environment allowlist. It includes
  the official `GROK_HOME` override plus the platform home variables, but never
  invents or retargets `GROK_HOME`.
- Turso remains a non-secret evidence ledger. It may remember that an ACP
  handshake succeeded, but startup always revalidates the current provider
  credential.

One runtime generation may exist at a time. Beginning interactive login first
requests a bounded, nonblocking shutdown of any startup-probe ACP generation.
The runtime handshake must observe that shutdown even while `initialize` or
`authenticate` is pending; the UI thread never waits for a provider process.

After the official login child succeeds, DevPULSE creates a new post-login
verification generation. Retryable launch, transport, timeout, and
authentication failures remain visibly `Verifying` and use the bounded
backoff sequence `100 ms, 250 ms, 500 ms, 1 s, 2 s`. Each retry is a typed,
redacted provider fact with its ordinal and delay. No retry writes `failed` or
`signed_out` evidence.

The failure boundary is:

- a startup probe that does not advertise `cached_token` is definitively
  `SignedOut`;
- a post-login or recovery probe that momentarily lacks `cached_token` retries
  before becoming `SignedOut`;
- a cached-token authentication rejection retries before becoming
  `SignedOut`;
- launch, timeout, and disconnect failures retry before becoming `Failed`;
- protocol mismatch, unsupported behavior, invalid commands, and backpressure
  fail immediately because repetition cannot repair an incompatible contract.

Once `Ready`, the ACP child stays alive and Grok Build performs its own token
refresh. A later top-level disconnect begins the same bounded recovery
supervision and reloads provider sessions after reconnect. Login is requested
again only after the provider can no longer establish cached subscription
authentication within the bounded recovery policy.

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
- Browser-auth tests prove only named provider endpoints can open, system
  dispatch never runs on the UI thread, automatic dispatch is deduplicated,
  explicit Reopen is allowed, and no CDP/browser-automation dependency exists.
- Turso tests prove ready/signed-out replacement, persistence across reopen,
  and the absence of token/code/URL columns.
- Chat-window tests prove that a signed-out Grok card exposes one semantic
  login button in the conversation and produces only
  `ActivateProvider(grok)`.
- App coordinator tests prove provider actions are rejected for non-Grok
  providers and forwarded only to the Grok coordinator; provider facts share
  one attempt identity and never contain a code or authorization URL.
- EguiPilotAI proves provider selection, one activation action, visible
  device-code state, browser-reopen action, verifying state,
  ACP-qualified session actions, and the absence of a false Send capability
  before the session transport is ready.
- A real-account native proof requires the operator to click Grok, complete
  xAI authentication in the computer's default browser, return
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
  activation/runtime, the now-superseded app-owned browser boundary, Turso
  evidence, and Chat window.
- `chat-grok-device-authorization.toml` passed all fourteen steps against the
  real independent Chat viewport, including visible and copyable code,
  collapsed/expanded activation trace, enabled browser-handoff action, and
  disabled Send. This proves UI semantics rather than real-account transport.
- Two real native-window captures were byte-identical. The reviewed proof is
  `proof-chat-grok-authorization.png`.
- The conversation-owned ceremony is captured independently in
  `proof-chat-grok-conversation-activation.png`; the fixture does not force a
  provider hover card over the ceremony.
- No real account was authenticated by automation. The remaining operator
  gate is the explicit xAI ceremony and one real Grok turn.
- The subsequent operator test reached xAI's Cloudflare block page because
  the visible Chromium proof still carried CDP remote-debugging and automated
  navigation. That result invalidated app-owned Chromium as an authentication
  transport and triggered the system-browser correction above.
- The corrected `dpl-chat-browser-auth` crate has no dependency on
  `dpl-engine-browser` or `dpl-tools-deps`; four focused tests prove strict
  endpoint validation, asynchronous system dispatch, automatic deduplication,
  explicit Reopen, and bounded failure projection.
- The corrected real independent Chat viewport passed all fourteen steps in
  `chat-grok-device-authorization.toml`. The reviewed native-window proof is
  `proof-chat-grok-system-browser-auth.png` and visibly identifies the
  computer's default browser as the authorization surface.
- A real provider-owned cached credential was independently qualified with
  the exact embedded Grok executable: its `models` command and ACP
  `initialize` plus `cached_token` authentication succeeded under the same
  compiled environment allowlist used by DevPULSE.
- The remaining application failure was isolated to the process boundary:
  converting a configured `std::process::Command` into
  `async_process::Command` resets the latter's internal stdio-configuration
  flags, so spawn replaced the intended ACP pipes with inherited handles.
  DevPULSE now reasserts piped stdin/stdout and null stderr after conversion;
  a Windows regression test launches a real child and proves both ACP handles
  remain owned by the runtime.
- The real independent Chat viewport now reaches Grok `Ready` after a fresh
  DevPULSE process, selects Grok, and exposes an enabled `New Grok chat`
  action without another authorization ceremony. The durable
  `chat-grok-persisted-ready.toml` scenario preserves that positive
  ready-state contract. Absence of a login node is not encoded as a `hidden`
  expectation because the current EguiPilotAI `hidden` matcher requires a
  live semantic node; the ready state and enabled session action are the
  supported, user-visible proof.
