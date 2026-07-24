# Chat provider capability research

**Evidence date:** 2026-07-23
**Scope:** subscription-backed coding agents for the DevPULSE Chat platform
**Status:** research record; every live adapter still requires version-pinned
real-executable conformance

This document records the official integration surfaces that informed
[`chat-architecture.md`](../plans/chat-architecture.md). It is intentionally
separate from the product plan: provider commands and capabilities will
change, while DevPULSE's typed UI and state contracts should remain boring and
stable.

## Findings

1. DevPULSE must be a client of official provider applications and protocols.
   It must not extract credentials, imitate private OAuth clients, or call
   subscription backends using another application's tokens.
2. Authentication stays provider-owned. DevPULSE may initiate a documented
   login/logout action and show a non-secret account summary, but it never
   reads, stores, copies, or logs provider credentials.
3. Codex app-server is the strongest first rich adapter. It explicitly exists
   to power clients and exposes accounts, threads, turns, approvals, models,
   usage, rate limits, and streaming events.
4. Claude Code exposes documented programmatic and streaming modes, session
   resume, models, permission modes, and bounded turns. It needs a dedicated
   adapter rather than terminal scraping.
5. Gemini CLI's Agent Client Protocol mode is its correct future client
   boundary. Direct third-party use of the services behind Gemini CLI with the
   CLI's OAuth identity is explicitly outside the allowed boundary.
6. Grok Build exposes an ACP agent mode and subscription sign-in. Its surface
   is newer and must remain experimental until qualified.
7. A normalized event model is valuable for the UI. A universal provider
   command dialect is not: capability negotiation must keep real differences
   visible.

## Evidence matrix

| Provider | Official client boundary | Subscription authentication | Session surface | Local evidence | Initial state |
|---|---|---|---|---|---|
| Codex | `codex app-server`, JSON-RPC over stdio | ChatGPT sign-in managed by Codex | Native thread/turn lifecycle, history, streaming, approvals, usage | `codex 0.145.0-alpha.18` installed | Candidate |
| Claude | Claude Code documented stream JSON | Claude App Pro/Max login, Console, or enterprise | New/resumed sessions, streaming, permission modes, bounded turns | `claude 2.1.218` installed | Candidate |
| Gemini | Gemini CLI ACP mode | Provider-owned Google login and supported alternatives | ACP new/load session, prompt, cancel, mode/model controls | Not installed | Pending |
| Grok | `grok agent stdio` ACP mode; documented headless JSON | Grok/eligible subscription browser or device login | ACP client lifecycle plus headless streaming | Not installed | Experimental |

Installed-version evidence is environmental, not a product promise. A provider
profile declares supported versions and the adapter verifies the protocol or
schema at every handshake.

## Codex

Official sources:

- [Using Codex with your ChatGPT plan](https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan)
- [Codex app-server README](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)

The app-server contract includes initialization and capability negotiation;
account read/login/logout and rate limits; thread start/resume/fork/read/list;
turn start/steer/interrupt; incremental item and turn events; approvals; model
discovery; and generated schemas.

DevPULSE should speak that documented protocol. Terminal text and
`codex exec --json` are not the primary interactive integration.

Codex owns ChatGPT authentication and token refresh. DevPULSE renders only
typed account/plan state returned by app-server and invokes its documented
login/logout requests. Provider-native events without a normalized equivalent
remain available as bounded diagnostic payloads.

## Claude

Official sources:

- [Claude Code setup](https://docs.anthropic.com/en/docs/claude-code/getting-started)
- [Claude Code CLI reference](https://docs.anthropic.com/en/docs/claude-code/cli-usage)

Claude Code documents Claude App Pro/Max login, programmatic print operation,
JSON and stream-JSON output, resumed sessions, model selection, permission
modes, maximum turns, and continuation.

DevPULSE should launch only an approved Claude executable and consume its
documented structured stream. The adapter must preserve unknown events,
surface the actual permission mode, and treat every optional behavior as a
negotiated capability.

Claude owns credentials. DevPULSE must not decide that a user is authenticated
because a particular credential file exists.

## Gemini

Official sources:

- [Gemini CLI repository and authentication overview](https://github.com/google-gemini/gemini-cli)
- [Gemini CLI ACP mode](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/acp-mode.md)
- [Gemini CLI configuration](https://github.com/google-gemini/gemini-cli/blob/main/docs/reference/configuration.md)
- [Gemini CLI terms and privacy boundary](https://github.com/google-gemini/gemini-cli/blob/main/docs/resources/tos-privacy.md)

Gemini CLI's ACP mode is a documented JSON-RPC-over-stdio client surface. It
supports authentication, new/load session, prompt, cancel, session mode, and
model operations, and provides a client-mediated filesystem service.

DevPULSE may act as an ACP client to the official Gemini CLI. It must never
harvest Gemini CLI OAuth credentials, reuse those credentials to call Google
services, or imitate a private OAuth client.

Gemini is not installed in the current environment. Installation is an
explicit operator action; qualification begins only after its executable,
version, and ACP behavior are recorded.

## Grok

Official sources:

- [Grok Build overview](https://docs.x.ai/build/overview)
- [Grok Build CLI reference](https://docs.x.ai/build/cli/reference)
- [Introducing Grok Build](https://x.ai/news/grok-build-cli)

Grok Build documents interactive use, streaming headless output, provider
login/logout, device authentication, and `grok agent stdio` as an ACP agent.
The product was introduced as an early beta for eligible subscription users.

DevPULSE should integrate only through documented ACP/headless surfaces and
provider-owned sign-in. Until a pinned version passes real executable
conformance, Grok remains visibly `Experimental`, not silently equivalent to
the other tabs.

## Authentication contract

Every adapter obeys the same boundary:

- the provider application owns credentials and refresh behavior;
- DevPULSE stores only typed, non-secret account summaries and activation
  state;
- tokens, cookies, refresh secrets, credential paths, and credential contents
  never enter app state, traces, fixtures, screenshots, logs, or crash reports;
- login/logout are explicit provider actions with visible progress and errors;
- provider, account, model, workspace, and permission mode are visible before
  send;
- unsupported flows remain disabled with a truthful reason;
- no authentication state is inferred merely from the filesystem.

## Qualification contract

An executable being present does not make an adapter qualified. Every
supported version must pass:

1. executable discovery and version reporting;
2. protocol/schema handshake and unknown-version rejection;
3. signed-out, signing-in, signed-in, expired, and failed account states;
4. list/open/start/resume and bounded history paging when supported;
5. streaming text, plan, diff, tool, and status events where supported;
6. approval request and response where supported;
7. cancellation, timeout, crash, disconnect, and normal shutdown;
8. bounded queues and backpressure;
9. unknown event preservation;
10. secret and path redaction;
11. child-process reaping after cancel, crash, and parent exit;
12. truthful `Unsupported` results for every absent capability.

Recorded fixtures are required for deterministic tests, but they do not
replace real-executable qualification.
