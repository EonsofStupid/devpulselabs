# ADR proposal: Chat provider runtime exception

**Status:** RATIFIED — implementation allowed; live launch remains gated
**Date:** 2026-07-23
**Owner:** DevPULSE operator
**Architecture:** [`chat-architecture.md`](chat-architecture.md)
**Evidence:** [`chat-provider-capabilities.md`](../research/chat-provider-capabilities.md)

## Ratification record

The DevPULSE operator explicitly approved execution of this narrow exception
on 2026-07-23. Ratification authorizes implementation of the named adapter
boundary; it does not waive the launch gates below. The first live adapter is
Grok Build over its documented ACP stdio surface; ChatGPT/Codex follows only
after Grok's local qualification checkpoint. Live launch remains fail-closed
until the selected adapter's supported version range, workspace jail,
redaction, process reaping, and containment checks are implemented and green.

## Decision

DevPULSE has a deliberate pure-Rust, no-production-IPC law. The required Chat
product must use official subscription-backed Codex, Claude, Gemini, and Grok
client surfaces. Those surfaces are provider-owned processes and documented
local protocols.

The proposed decision is:

> Permit a narrow production process/protocol exception for named Chat
> provider adapters. Keep the no-IPC law everywhere else.

Provider-neutral types, read-only history, route context, fake providers, UI
composition, and containment tests may proceed. No production provider process
may be launched merely because the exception is ratified; every required
launch gate must also pass and the governing `AGENTS.md` must retain its
concise pointer.

## Why an exception is necessary

The official rich-client boundaries currently are:

- Codex app-server over JSON-RPC/stdio;
- Claude Code's structured CLI stream;
- Gemini CLI in Agent Client Protocol mode;
- Grok Build in Agent Client Protocol or documented headless mode.

Live sessions require authentication status, streaming turns, tools,
approvals, cancellation, rate limits, and provider-owned history. Direct API
keys would not use the requested subscriptions. Borrowing private OAuth
credentials or scraping terminal presentation would be less secure and less
supportable than the documented client boundaries.

## Allowed boundary

Only dedicated Chat provider adapters and the Chat lifecycle coordinator may:

- resolve an operator-approved named provider executable;
- launch it with protocol-defined arguments and an explicitly selected jailed
  working directory;
- communicate through its documented stdio, ACP, app-server, or future
  official local-client protocol;
- invoke provider-owned login/logout flows;
- receive typed account summaries, sessions, events, approvals, usage, and
  errors;
- service documented coding-agent filesystem and terminal requests inside the
  selected workspace after the corresponding provider permission is resolved
  through the governed Interaction Library;
- terminate and reap the exact child/process tree owned by that adapter;
- enable the runtime only through explicit feature and project configuration.

## Forbidden

- provider executable paths, adapter arguments, or direct host commands
  supplied by model output, documents, transcripts, UI text, MCP arguments, or
  route context;
- arbitrary shell concatenation/interpolation or a generic "run anything"
  adapter. The sole command exception is a typed terminal request received
  from a named provider's documented coding-agent protocol after its governed
  permission decision, confined to an approved workspace and owned/reaped by
  that provider runtime;
- reading, parsing, copying, logging, or persisting provider credentials;
- reusing a provider CLI's OAuth identity to call provider backends directly;
- silently changing provider, account, model, workspace, permission mode,
  executable, environment, or protocol;
- unrestricted filesystem or artifact paths;
- unbounded messages, attachments, queues, logs, or history reads;
- silent fallback between providers or authentication modes;
- provider adapters rendering UI or controlling the shell directly.

## Runtime profile

Every provider profile declares:

- typed provider/profile identities;
- executable resolution policy and supported version range;
- protocol and schema range;
- allowed workspace roots;
- an environment-key allowlist;
- queue, message, attachment, history, and log limits;
- startup, action, idle, cancellation, and shutdown timeouts;
- permission/sandbox defaults;
- account and capability discovery;
- redaction policy;
- process-tree cleanup behavior.

Resolution produces a visible runtime identity before launch. Any mismatch
fails closed. The UI shows provider, account, model, workspace, and permission
mode before a turn begins.

One runtime owner controls process lifetime. Closing a tab, changing route, or
detaching a window cannot orphan a provider. Stop, cancellation, app shutdown,
timeout, crash, and disconnect converge on bounded termination and reaping.

## Containment

```text
approved provider process
          ^
          | bounded documented protocol
          v
provider adapter -> normalized commands/events -> chat runtime
                                                   |
                              store / context / extensions / UI
```

The UI never receives a pipe, process handle, token, raw command, or mutable
environment map. Provider crates never depend on shell/UI crates. Tests use an
in-memory fake provider by default; real executable suites are explicit.

## Rejected alternatives

**Read-only forever:** preserves the current law but cannot deliver live
subscription sessions, approvals, tools, or provider-specific features.

**Direct HTTP APIs and API keys:** fails the subscription requirement, makes
DevPULSE a secret custodian, and loses provider-native session semantics.

**Credential reuse:** rejected for security, terms, and maintainability.

**Embedded provider web apps:** not a stable native session protocol and not a
sound base for route context, accessibility, or deterministic testing.

**One generic command runner:** turns a narrow exception into arbitrary
execution and makes lifecycle/security claims unprovable.

**One universal JSONL dialect:** conceals material provider differences.
DevPULSE instead normalizes display events and negotiates commands.

## Consequences

Benefits:

- subscriptions and credentials remain provider-owned;
- documented rich-client capabilities remain available;
- security review is confined to a small, named boundary;
- adapters evolve and fail independently;
- UI and app-owned state remain provider-neutral.

Costs:

- every adapter needs version-pinned protocol and lifecycle conformance;
- four providers require four real adapters;
- supported versions become a compatibility matrix;
- dependency audits must prove no unrelated production IPC appeared.

## Ratification and launch checklist

- [x] Operator explicitly approves the narrow Chat provider-runtime exception.
- [x] Allowed and forbidden behavior is accepted.
- [x] The first live adapter is chosen: Grok Build ACP, followed by
  ChatGPT/Codex app-server.
- [ ] Grok Build's supported version range is pinned from protocol and
  real-executable conformance evidence.
- [ ] Workspace jail and provider profiles are defined.
- [ ] Secret-redaction and process-reaping tests exist.
- [ ] Production dependency inspection proves containment.
- [x] A concise exception/pointer is added to `AGENTS.md`.

Until every unchecked launch item is complete, live provider launch remains
blocked.

### Activation-only provider slices

Provider-owned authentication may be qualified independently from a live
conversation adapter when it still obeys the complete named-process,
credential, containment, and lifecycle boundary above. Activation success
qualifies only `SubscriptionAuth`/`DeviceCodeAuth`; it cannot silently enable
sessions, turns, tools, approvals, or send.

The first such slices are the bounded official browser-login paths for
[ChatGPT / Codex](chat-codex-activation.md) and
[Grok Build](chat-grok-activation.md). Grok's activation entry is now extended
by the named ACP implementation in
[`chat-provider-workspaces.md`](chat-provider-workspaces.md): browser-login
success restarts ACP but cannot set `Ready` by itself. Codex app-server remains
blocked until its separate implementation and qualification pass.
