# Chat CH1 closeout + CH2 read-only sessions

**Status:** IMPLEMENTED — local qualification complete
**Date:** 2026-07-23
**Parent:** [`chat-architecture.md`](chat-architecture.md)
**Runtime boundary:** [`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md)

## Objective

Finish the provider-neutral foundation and replace eager transcript ownership
inside `ProviderRegistration` with a bounded, read-only history store. The
operator must be able to browse deterministic fixtures and existing local
Codex/Claude histories without launching a provider, inferring authentication,
or loading an entire transcript into memory.

This is deliberately a boring data pass. It does not launch Codex or Claude,
send prompts, mutate provider history, read credentials, or create attached
windows.

## Locked implementation patterns

### Qualified capability state

Provider manifests describe intended capabilities. Runtime registrations carry
a separate qualified capability set. UI actions consult only the qualified
set. A provider cannot become capable merely because a manifest or provider
name claims support.

Activation failures expose typed recovery actions with human-readable labels.
Provider-native unknown data is retained through a bounded, redacted payload
type.

### History reader boundary

`dpl-chat-store` owns reusable filesystem mechanics:

- a canonical, read-only root jail;
- depth, file-count, prefix-byte, page-byte, event-count, and line-size limits;
- symlink rejection and canonical containment checks;
- opaque store-owned history locators rather than caller-provided paths;
- newest-first discovery;
- backward cursor paging from the transcript tail;
- explicit truncation and malformed-record diagnostics.

Provider crates own JSONL interpretation through a `HistoryDecoder` contract.
The store never switches on provider names or knows provider schemas.

Provider-native session IDs are retained as metadata, but are not assumed to
be unique storage identities. Multiple history artifacts may belong to the
same native session. The store derives a stable opaque locator from each
root-relative artifact name, checks resulting DevPULSE identity collisions,
and accepts only locators that it generated during bounded discovery.

### Provider readers

`dpl-chat-codex` and `dpl-chat-claude` implement their own decoders. Each
decoder:

- derives the provider-native session ID without accepting a path from UI;
- extracts bounded index metadata from a prefix;
- normalizes known user/assistant/status/tool/result records;
- preserves malformed and unknown records as bounded redacted events;
- never inspects credential files or claims authentication from filesystem
  presence.

Known local roots are resolved once by the application from the operator's
user profile and passed as explicit read-only profiles:

- Codex: `.codex/sessions`
- Claude: `.claude/projects`

An absent root is an unavailable read-only source, not a provider failure.

### UI ownership

The Chat hub owns a `HistoryCatalog` separate from live provider runtime state.
The session rail requests a bounded provider index. Selecting a session requests
one bounded tail page. Conversation rendering uses an event-renderer registry
and a virtualized fixed-row projection; it never instantiates one egui widget
per event in a 50,000-event history.

The hub displays:

- the provider/source owning the session;
- whether the view begins mid-history;
- a deterministic "load earlier" action when a cursor exists;
- explicit malformed/unknown records;
- source errors scoped to the selected provider.

EguiPilotAI profiles own executable and renderer selection only. Named
fixtures own route and deterministic launch arguments. Chat sessions are
actionable semantic nodes, visible transcript events are vital semantic text,
and both retain region breadcrumbs in the pruned agent snapshot.

## Security and failure rules

- No provider file is opened for write.
- No root is accepted from route context, provider output, MCP, transcript
  content, or a UI text field.
- No path is reconstructed from a session ID.
- Symlinks are not followed.
- Oversized records fail as a visible bounded diagnostic instead of allocating
  without limit.
- Changed, removed, rotated, or malformed transcripts affect only that source.
- Store errors contain no transcript content and no credential-shaped values.

## Test matrix

Unit and contract tests:

- manifest capability does not imply qualification;
- typed recovery action rendering;
- bounded native payload truncation;
- jail rejects escape paths and symlinks;
- discovery obeys depth/file/prefix limits;
- cursors page backward without duplicate or lost complete records;
- rotation and missing sessions fail explicitly;
- Codex and Claude fixture normalization;
- malformed/unknown lines survive as diagnostics;
- 50,000 fixture events are requested in bounded pages.

Real artifact:

- all four registered providers remain truthful;
- Codex/Claude local read-only sessions appear only when roots exist;
- provider switching preserves selected session and draft;
- the transcript renders a bounded page and exposes earlier-history state;
- compact layout remains usable;
- Chat controls receive stable EguiPilotAI semantic IDs.

## Qualification evidence

- Bounded store, provider decoder, runtime, renderer, and hub suites pass
  locally.
- Two artifacts sharing one provider-native session ID remain separately
  addressable, with distinct DevPULSE session and event identities.
- A 50,000-record fixture returns one 200-event tail page.
- Real local Codex discovery indexed 37 history artifacts without reading an
  entire transcript into the widget tree.
- Real native wide and compact captures render the bounded history view; the
  narrow transcript toolbar stacks without collision.
- The named `chat` EguiPilotAI fixture returns providers, visible sessions,
  disabled live controls, bounded transcript events, region breadcrumbs, and
  no duplicate-ID, busy-token, or uncontrolled-motion diagnostics.

Local qualification does not authorize live provider launch. The launch
checklist in the runtime ADR remains authoritative.

## Exit

CH1 exits when target and qualified capabilities are separate, recovery actions
are typed, and unknown payloads are bounded.

CH2 exits when fixture and local read-only histories can be discovered and
paged through the native Chat hub with no provider writes, credential
inference, unbounded transcript load, provider-specific UI branch, or whole
history widget tree.
