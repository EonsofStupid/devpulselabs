# DevPULSE Chat — route-aware multi-agent application platform

**Status:** CANONICAL PLAN — implementation proceeds in the slices below
**Date:** 2026-07-23
**Runtime blocker:** live providers require ratification of
[`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md)
**Provider evidence:**
[`chat-provider-capabilities.md`](../research/chat-provider-capabilities.md)

## 1. Product intent

Chat is not a transcript pane and the provider names are not cosmetic tabs.
It is a first-class, route-aware DevPULSE platform for authenticated coding
agents, durable sessions, explicit context, approvals, tools, artifacts,
extensions, overlays, and attached windows.

Codex, Claude, Gemini, and Grok each retain their own:

- account and authentication state;
- capabilities and version compatibility;
- models, modes, permissions, and quotas;
- session identifiers and history;
- live turns, tools, approvals, and errors;
- per-tab UI state and composer draft.

The app provides one coherent operating environment without pretending the
providers have identical protocols.

### Locked product decisions

1. Provider integration is registry-driven. Adding a provider adapter does not
   add a provider branch to the shell or Chat UI.
2. Provider credentials remain provider-owned. DevPULSE never reads or stores
   tokens.
3. Route changes may update the visible context offer; they never
   automatically send context or a message.
4. Context-triggered UI may become available or peek. It does not steal focus
   or open a new window by default.
5. Provider-native history remains the source of truth. DevPULSE may keep
   bounded app-owned metadata, indexes, drafts, links, and provenance.
6. Session handoff creates a new linked session with an explicit context
   export. A Claude session cannot silently become a Codex session.
7. Additional windows are stable Chat surfaces with typed identities and
   lifecycle, not transient ad-hoc `show_viewport` calls.
8. Unknown provider events remain inspectable; they are never silently
   dropped.
9. Council/orchestration is last. Every provider must work predictably alone
   before providers can coordinate.
10. Unsupported behavior is visible as `Unavailable` or `Experimental` with a
    reason. Placeholder success is forbidden.

## 2. Current reality

The rewrite currently contains:

- a `ChatHub` placeholder;
- `Sessions` and `Council` routes;
- a hardcoded visual list of five provider names;
- a cloned `Fabric` handle used only for navigation.

It does not contain:

- provider/account/session types;
- authentication or executable detection;
- read-only transcript discovery;
- live provider runtimes;
- session paging or virtualization;
- route context capture or consent;
- extensions, overlays, docks, or attached windows;
- provider conformance tests;
- a scalable Chat store.

The prior DevPULSE app is salvage evidence, not a base. Its useful C1
capabilities were a read-only Claude/Codex session browser, a transcript
normalizer that retained unknown lines, an event-renderer registry, bounded
file reads, tabs, and quota presentation. Credential-file presence checks and
hardcoded provider unions must not be ported.

## 3. Design laws

### Boring state flow

```text
provider/fixture input
        |
        v
provider adapter -> typed command/event boundary -> ChatRuntime
                                                    |
                   +--------------------------------+-------------------+
                   |                                |                   |
                   v                                v                   v
               ChatStore                    ContextRegistry     ExtensionRegistry
                   |                                |                   |
                   +------------------------ ChatViewModel -------------+
                                                    |
                                                    v
                                              dpl-hub-chat UI
```

- Commands use bounded MPSC queues.
- Events are immutable typed values with monotonically increasing revisions.
- UI renders a view model; it does not parse provider payloads.
- Provider adapters never call egui or the shell.
- Widgets never own provider process lifetime.
- App state changes on one owner path, not through scattered shared mutation.

### Stable identity

No provider, account, session, turn, event, context item, extension, surface,
or window is identified by a bare UI string.

Required newtypes:

- `ProviderId`
- `ProviderProfileId`
- `AccountId`
- `ChatSessionId`
- `ProviderSessionId`
- `TurnId`
- `ChatEventId`
- `ContextSourceId`
- `ContextSnapshotId`
- `ContextItemId`
- `ChatExtensionId`
- `ChatSurfaceId`
- `ChatWindowId`

Display names, icons, and tones are metadata. They never become identity.
Dynamic identities combine a typed parent and a stable provider/native key.

### Capability truth

The UI asks a provider registry what exists. It does not infer capability from
provider name.

Core capabilities:

- `ReadSessions`
- `PageHistory`
- `LiveTail`
- `StartSession`
- `ResumeSession`
- `ForkSession`
- `StreamTurns`
- `InterruptTurn`
- `SteerTurn`
- `Approvals`
- `Tools`
- `Diffs`
- `Plans`
- `Attachments`
- `Images`
- `Models`
- `Modes`
- `RateLimits`
- `SubscriptionAuth`
- `DeviceCodeAuth`
- `Headless`

Every capability has an availability state:

- `Available`
- `Unavailable { reason }`
- `Experimental { reason }`
- `Blocked { action }`

Provider/version conformance produces this state. The UI does not guess.

## 4. Crate architecture

Each concern owns one crate and exposes a narrow public API.

### `dpl-chat-core`

Provider-neutral types only:

- typed identities;
- capability and activation state;
- account summaries;
- session/turn/event models;
- commands and outcomes;
- normalized event content;
- provider-native bounded payload wrapper;
- redacted diagnostic and error types.

It does not depend on egui, shell, filesystem scanning, subprocesses, or a
specific provider.

### `dpl-chat-store`

Read-only history and app-owned metadata:

- provider session index;
- bounded path-jailed discovery;
- cursor-based history pages;
- incremental tailing;
- app-owned titles, pins, tags, links, drafts, window layout, and context
  provenance;
- schema migration and atomic writes;
- search/index contracts.

It never writes provider homes or provider transcripts.

### `dpl-chat-runtime`

Provider registry and lifecycle coordination:

- provider/profile activation;
- per-session command queues;
- event ingestion and ordering;
- backpressure and overflow diagnostics;
- cancellation and timeouts;
- account/session state machines;
- fake-provider and real-provider registration;
- process lifecycle only after the ADR is ratified.

There is one runtime owner for each activated profile and explicit ownership
for each live session.

### Provider adapters

Separate crates:

- `dpl-chat-codex`
- `dpl-chat-claude`
- `dpl-chat-gemini`
- `dpl-chat-grok`

Each crate:

- owns executable/protocol/version knowledge;
- maps provider-native data to core events;
- exposes native extensions behind capability checks;
- owns conformance fixtures;
- never depends on egui, shell, or another provider.

Codex is the intended first live adapter after ADR ratification because its
app-server is the most complete official rich-client protocol. Claude follows
through its documented structured stream. Gemini and Grok require explicit
installation and qualification.

Local/Clyffy support may become another registered adapter later. It must not
remain a hardcoded fifth exception in the UI.

### `dpl-chat-context`

Route-aware context without hidden data transfer:

- context contributor registry;
- revisioned snapshots;
- item provenance and sensitivity;
- selection and attachment state;
- size estimates and provider compatibility;
- explicit export bundles for turns and handoffs.

### `dpl-chat-extensions`

Declarative UI extension and trigger registry:

- extension manifests;
- placement and ordering;
- route/context/provider predicates;
- permission declarations;
- activation, pin, dismiss, snooze, and restore state;
- typed actions into ChatRuntime or shell surfaces.

Extensions contribute models and actions, not arbitrary egui callbacks from
provider crates.

### `dpl-chat-windows`

Attached/detached surface state:

- stable window identity;
- window kind and binding;
- viewport generation/lifecycle;
- focus/close/restore policy;
- layout persistence;
- EguiPilotAI viewport metadata.

### `dpl-chat-testkit`

Deterministic provider fakes and fixtures:

- scripted account/session/event timelines;
- approval and tool scenarios;
- disconnect/crash/timeout injection;
- large-history generators;
- secret-shaped redaction fixtures;
- cross-provider capability matrices.

### `dpl-hub-chat`

UI composition only:

- converts registered state into zones;
- virtualizes lists and transcripts;
- renders normalized events through a renderer registry;
- delegates provider-native optional panels through safe extension models;
- emits typed user intents.

### Dependency direction

```text
chat-core
   ^
   +-- chat-store
   +-- chat-context
   +-- chat-extensions
   +-- chat-windows
   +-- chat-runtime <-- provider adapters
            ^
            |
       hub-chat <-- app/shell composition

chat-testkit -> all contract consumers in tests only
```

Cycles are forbidden. Provider crates do not depend on Chat UI. Chat core does
not depend on provider crates.

## 5. Provider adapter contract

The public adapter contract is capability-gated. Conceptually:

```rust
trait ProviderAdapter {
    fn descriptor(&self) -> ProviderDescriptor;
    fn capabilities(&self) -> CapabilitySet;
    fn activation(&self) -> ActivationState;
    fn submit(&mut self, command: ProviderCommand) -> CommandAccepted;
    fn try_next_event(&mut self) -> Option<ProviderEvent>;
    fn shutdown(&mut self, deadline: Deadline) -> ShutdownOutcome;
}
```

The exact Rust API may split configuration, runtime, and polling traits, but
these laws remain:

- submission is nonblocking;
- queues are bounded;
- every command has a typed correlation ID;
- events identify provider/profile/session/turn;
- unsupported commands return a typed result immediately;
- shutdown is explicit and testable;
- adapter panic/crash becomes a contained provider failure;
- native payloads are size-limited and redacted.

### Normalized event model

Required normalized content:

- `SessionStarted`
- `SessionMetadataChanged`
- `UserMessage`
- `AssistantMessage`
- `PlanUpdated`
- `ReasoningSummaryUpdated`
- `StatusUpdated`
- `ToolStarted`
- `ToolUpdated`
- `ToolCompleted`
- `DiffUpdated`
- `ApprovalRequested`
- `ApprovalResolved`
- `UsageUpdated`
- `RateLimitUpdated`
- `TurnCompleted`
- `TurnInterrupted`
- `TurnFailed`
- `ProviderWarning`
- `Unknown`

`Unknown` contains a bounded, redacted provider-native representation and a
schema/version tag. Unknown input is visible in diagnostics and preserved in
fixtures.

## 6. Authentication and activation

Provider tabs are always discoverable through the registry, even if their
runtime is unavailable.

Activation states:

- `NotInstalled`
- `VersionUnsupported`
- `SignedOut`
- `SigningIn`
- `Ready`
- `Degraded`
- `Experimental`
- `Failed`
- `Disabled`

Each state has a human-readable reason and zero or more typed recovery actions
such as `InstallInstructions`, `SignIn`, `Retry`, `UpdateProvider`, or
`OpenDiagnostics`.

Rules:

- provider sign-in UI invokes official provider behavior;
- account summaries contain no credentials;
- switching provider tabs never logs another provider out;
- account changes invalidate only the affected profile/session state;
- send is impossible until provider, account, model, workspace, and permission
  state are resolved;
- the UI does not ask users to paste subscription tokens;
- API-key profiles, if ever supported, are a separate explicit product policy.

## 7. Session model

`ChatSessionId` is the app identity. A `SessionBinding` records:

- provider and profile;
- provider-native session ID when one exists;
- workspace identity and jailed root;
- account summary revision;
- created/opened timestamps;
- capability snapshot;
- parent/handoff provenance;
- read-only/live/expired state.

### Provider tab isolation

Each provider tab retains:

- active session;
- selected model/mode/permission;
- composer draft;
- scroll anchor;
- context selection;
- inspector state;
- live turn status;
- open overlays and attached-window links.

Switching tabs must not destroy, merge, or silently pause another provider's
state.

### Unified discovery

DevPULSE may show an aggregated session list with provider badges, but:

- provider-native IDs remain visible in diagnostics;
- sorting/searching never changes ownership;
- selecting a session activates its provider tab;
- cross-provider continuation is never implied.

### Handoff

A handoff:

1. freezes an explicit export from the source session;
2. shows exactly which messages, context items, artifacts, and summaries are
   included;
3. creates a new destination session;
4. records source/destination IDs and content hash;
5. sends only after operator confirmation;
6. never exposes hidden provider state or credentials.

Council uses the same primitive later; it does not get a second secret
handoff mechanism.

### Large history

- session indexes are lazy and incrementally refreshed;
- history is cursor-paged;
- the conversation well is virtualized;
- tailing reads bounded suffixes;
- search uses an app-owned index without rewriting provider data;
- malformed or unknown records remain available as diagnostics;
- loading 50,000 events cannot require 50,000 egui widgets or one monolithic
  JSON allocation.

## 8. Route-aware context

`ShellEvent::RouteChanged` is an ephemeral signal, not the context model.

Each participating hub registers a `ContextContributor` that can produce a
revisioned `ContextSnapshot` for its current route. A snapshot may contain:

- route and subroute;
- selected episode/document/block/artifact;
- focused file or capture;
- active slide/template/preset;
- viewport identity;
- available context actions;
- sensitivity and shareability;
- size estimate;
- source revision and content hash.

### Consent rules

- route changes update the context offer only;
- no context item is attached merely because it is visible;
- the composer shows a context strip listing exactly what will be sent;
- every attached item has source, revision, size, and remove action;
- stale items are marked before send;
- sensitive/unshareable items cannot be attached;
- provider adapters receive an explicit immutable export, not shell access;
- every sent bundle records provenance for replay and handoff.

### Route-aware suggestions

Extensions may use route/context predicates to surface helpful actions:

- "Ask about this episode"
- "Review this diff"
- "Explain this capture"
- "Open the artifact inspector"
- "Start a planning session here"

A trigger may:

- add a passive action;
- show a dismissible peek;
- illuminate an existing dock/tab;
- offer an attached window.

It may not:

- send a prompt;
- launch a provider;
- approve a tool;
- change route;
- steal keyboard focus;
- open a window automatically unless a separate, explicit safe policy allows
  that exact extension.

## 9. UI zoning

Chat owns a predictable set of surfaces. Features extend these surfaces
through registries instead of adding floating layout code.

### Zone A — provider bar

- data-driven provider tabs;
- activation/auth/account state;
- current model, mode, and health;
- provider-local unread/running indicators;
- no hardcoded provider `match` in the UI.

### Zone B — session rail

- new session;
- project/workspace filter;
- search;
- pinned/recent/provider-grouped sessions;
- running and attention states;
- bounded lazy loading.

### Zone C — conversation well

- virtualized normalized events;
- renderer registry by event kind;
- streaming deltas without full-tree rebuild;
- inline tools, diffs, plans, approvals, warnings, and unknown events;
- stable scroll anchoring when history pages prepend;
- explicit jump-to-live behavior.

### Zone D — context inspector

- route context offer;
- attached context and provenance;
- artifacts, plans, diffs, tool details, and provider diagnostics;
- extension-provided inspector sections;
- sensitivity and size budget.

### Zone E — composer deck

- per-provider/per-session draft;
- input and attachments;
- provider model/mode/permission controls;
- capability-driven actions;
- send/stop/steer state;
- keyboard and screen-reader operation.

### Zone F — overlay layer

- approvals;
- account/login progress;
- warnings and recoverable failures;
- command palettes and lightweight context cards;
- extension peeks;
- deterministic stacking and focus return.

### Zone G — attached windows

- detached conversation;
- plan/diff review;
- tool run/output;
- artifact preview;
- provider diagnostics;
- future Council.

### Responsive collapse

1. Collapse the context inspector into a drawer.
2. Collapse the session rail into a session picker.
3. Retain provider identity, conversation, and composer.
4. Never allow overlapping primary zones or nested uncontrolled scrolling.
5. Attached-window layouts use their own declared minimum sizes.

## 10. Extension contract

An extension manifest declares:

- typed extension ID and owner;
- compatible Chat surface placements;
- route/context predicates;
- provider capability requirements;
- priority and mutual-exclusion group;
- required data permissions;
- activation behavior;
- state persistence policy;
- accessibility label/help text.

Initial placements:

- `ProviderBarAction`
- `SessionRailSection`
- `ConversationRenderer`
- `ComposerAction`
- `ContextInspectorSection`
- `Overlay`
- `StatusDock`
- `AttachedWindow`

Extensions produce declarative view models and typed intents. They do not
receive unrestricted `egui::Context`, provider process handles, arbitrary
filesystem access, or shell mutation.

The registry resolves order deterministically:

1. platform-owned mandatory surfaces;
2. active provider surfaces;
3. route-context surfaces;
4. user-pinned extensions;
5. optional suggestions.

Conflicts degrade to one visible diagnostic, never nondeterministic order.

## 11. Shell and Fabric integration

The current `Hub` trait renders only the center surface. Chat needs docks,
overlays, inspector content, status actions, and windows without teaching the
shell about Chat.

Add a generic shell accessory registry. Conceptual surface kinds:

- `StatusAction`
- `Dock`
- `InspectorSection`
- `Overlay`
- `WindowRequest`

The app registers Chat's accessory producer beside the Chat hub. The shell
renders registered accessories generically and never checks `HUB == chat`.

Fabric remains appropriate for ephemeral shell signals:

- route changed;
- navigate request;
- window focus/close notices;
- application lifecycle.

Fabric broadcast is not appropriate for provider jobs, transcript streams, or
large context payloads. Those use bounded MPSC/session stores. Route context
is queried by revision through `dpl-chat-context`, not broadcast as a blob.

## 12. Attached-window lifecycle

Every additional window has:

- `ChatWindowId`;
- `WindowKind`;
- optional provider/session/turn binding;
- optional route/context binding;
- stable egui viewport key;
- generation and lifecycle state;
- focus, close, reopen, and restore policy;
- last valid geometry;
- owner extension/surface.

Lifecycle:

```text
Requested -> Opening -> Visible -> Closing -> Closed
                  \-> Failed
```

Rules:

- the shell/window coordinator owns viewport creation;
- provider adapters request typed intents and never create viewports;
- closing a window never closes its session unless explicitly chosen;
- closing a session gives bound windows a deterministic close/detach choice;
- context triggers offer windows without focus theft;
- layout restoration validates monitor bounds and minimum sizes;
- crash/restart cannot resurrect a stale live-turn handle;
- EguiPilotAI sees stable viewport identity and generation.

The current eframe/wgpu limitation around immediate child-window capture must
remain visible in qualification. Prefer and test a viewport mode that provides
reliable independent discovery/capture; do not claim screenshot support before
the real executable proves it.

## 13. Storage and privacy

### Provider-owned

- credentials;
- provider transcript/rollout files;
- provider config;
- provider session truth.

### DevPULSE-owned

- provider profile metadata without secrets;
- session index/cache;
- titles, pins, tags, and links;
- composer drafts;
- context attachment provenance;
- extension preferences;
- attached-window layout;
- redacted diagnostics;
- fixture and conformance metadata.

App-owned writes are atomic and schema-versioned. Corruption falls back to a
recoverable diagnostic and backup, never provider-data mutation.

Path access uses explicit roots and canonical containment checks. History
readers enforce item/page/byte limits. Provider homes are read-only and only
when the provider adapter/store has a documented need.

## 14. Failure behavior

The following are first-class states with recovery actions:

- provider not installed;
- unsupported version/schema;
- signed out or expired login;
- account changed;
- workspace denied/outside jail;
- capability unavailable;
- session missing/expired;
- malformed/unknown history record;
- queue full/backpressure;
- turn timeout;
- approval expired;
- provider disconnect/crash;
- cancellation failed;
- child cleanup failed;
- context stale/oversized/unshareable;
- extension conflict/failure;
- window open/capture failure.

Failures stay scoped to the affected provider/profile/session/extension.
Another provider tab remains usable.

## 15. Testing strategy

### Contract and unit tests

- typed ID and stable-key behavior;
- capability gating;
- activation/session/turn state machines;
- normalized event ordering and unknown preservation;
- cursor paging and path jail;
- context revision, staleness, and export hashing;
- extension ordering/conflicts/triggers;
- window lifecycle and restoration;
- redaction and bounded payloads;
- fake-provider backpressure, crash, timeout, and cancellation.

### Provider conformance

Every adapter runs the qualification contract in the research record against:

1. deterministic recorded fixtures;
2. malformed/schema-drift fixtures;
3. the real supported executable.

No live adapter is marked qualified from fixtures alone.

### EguiPilotAI real-app scenarios

- provider registry order and unavailable states;
- provider switch preserves active session, draft, and scroll state;
- signed-out login surface and failure recovery;
- route change updates context offer without attaching or sending;
- explicit attach/remove/send provenance;
- large transcript paging and stable scroll anchor;
- live streaming, tools, plan, diff, approval, interrupt, and completion;
- unknown event diagnostic;
- overlay open/dismiss/pin/focus return;
- extension trigger and conflict behavior;
- attached window discover/focus/close/reopen;
- multi-window session preservation;
- provider crash with no orphan process;
- keyboard-only provider/session/context/composer/approval flow;
- accessibility snapshots for all zones and windows.

### Performance gates

- no unbounded channels or collections;
- 50,000-event history remains paged/virtualized;
- streaming updates do not rebuild all rendered history;
- stable idle Chat does not request perpetual repaints;
- route changes do not parse provider history;
- warm steady-state memory is bounded by declared cache budgets;
- real-app CPU/frame overhead is measured before each expansion slice.

### Security gates

- secret-shaped values are absent from state snapshots, logs, traces,
  screenshots, fixtures, and crash bundles;
- provider command construction cannot express a shell;
- executable/profile/workspace selection fails closed;
- context is never sent without an explicit export;
- cancel/crash/app-exit leaves no provider child;
- production dependency inspection contains any ratified IPC exception to the
  named Chat runtime crates.

## 16. Delivery sequence

Each slice ends green locally, proves the real artifact where UI changes, and
is independently reviewable.

### CH0 — architecture and governance

- canonical plan, provider evidence, and runtime ADR;
- update roadmap/census;
- ratify runtime exception before live provider work;
- define generic shell accessory contract.

Exit: no ambiguous provider/auth/IPC/window ownership remains.

### CH1 — provider-neutral foundation

- `dpl-chat-core`;
- `dpl-chat-testkit`;
- provider registry and capabilities;
- normalized events and view-model contracts;
- deterministic fake providers;
- replace hardcoded provider roster with registered descriptors.

Exit: all four providers appear from registry data with truthful fake
activation/capability states.

### CH2 — read-only sessions

- `dpl-chat-store`;
- jailed bounded readers;
- C1 Claude/Codex history adapters;
- session rail, transcript virtualization, renderer registry;
- unknown/malformed event preservation.

Exit: fixture and real local histories can be browsed without provider writes
or credential inference.

### CH3 — route context and extensible shell

- `dpl-chat-context`;
- generic shell accessory registry;
- context strip/inspector;
- extension manifests and passive route-aware actions;
- provenance and export hashing.

Exit: route changes update suggestions; nothing is attached or sent
implicitly.

### CH4 — Codex live adapter

Blocked until ADR ratification.

- Codex app-server lifecycle and handshake;
- ChatGPT-owned authentication status/login/logout;
- threads, turns, streaming, plans, diffs, tools, approvals, usage, interrupt;
- real version-pinned conformance.

Exit: one complete subscription-backed provider works alone and leaves no
orphan process.

### CH5 — Claude live adapter

- documented structured stream;
- Pro/Max-owned authentication state;
- start/resume/continue;
- model/permission/max-turn behavior;
- conformance and normalization.

Exit: Claude works alone without provider-specific UI branches.

### CH6 — Gemini live adapter

- explicit operator install;
- ACP mode and client-mediated filesystem;
- provider-owned Google authentication;
- session/prompt/cancel/mode/model conformance.

Exit: Gemini is qualified or remains truthfully unavailable.

### CH7 — Grok live adapter

- explicit operator install;
- `grok agent stdio` ACP integration;
- subscription login/device flow;
- experimental qualification and version drift handling.

Exit: Grok is qualified, experimental with a reason, or unavailable—never
cosmetically green.

### CH8 — extensions and attached windows

- extension registry and all declared placements;
- `dpl-chat-windows`;
- plan/diff/tool/artifact/detached-session windows;
- layout persistence and EguiPilotAI multi-window proof.

Exit: contextual surfaces can extend Chat without modifying its core layout or
provider adapters.

### CH9 — handoff and Council

- explicit cross-provider export/handoff;
- operator/agent roles;
- turn/cost/time caps;
- reversible Council plan and approval;
- COSTAR templates as data.

Exit: orchestration reuses qualified single-provider primitives and cannot
bypass context consent or approval policy.

## 17. Acceptance contract

The Chat program is not complete until:

- provider tabs come solely from a registry;
- each provider has isolated account/session/draft/mode state;
- every visible action is capability-gated and truthful;
- credentials never enter DevPULSE-owned state or artifacts;
- route context never attaches or sends itself;
- sessions are paged, virtualized, and bounded;
- unknown provider data is preserved and diagnosable;
- extensions use declared placements/triggers and deterministic order;
- overlays return focus predictably;
- attached windows have stable identity, restore safely, and preserve sessions;
- provider failure does not take down other providers or the app;
- cancellation/app exit leaves no child process;
- keyboard and accessibility flows cover all primary surfaces;
- provider adapters pass fixture and real-executable conformance;
- local format, lint, tests, release build, and real-app EguiPilotAI proof pass;
- the branch is reviewed and merged through a PR, never developed directly on
  `main`.

## 18. Deliberately deferred

- Council before all individual providers qualify;
- automatic prompt sending from route/context triggers;
- automatic approval;
- credential import/export;
- unrestricted plugins or arbitrary command adapters;
- hidden cross-provider continuation;
- remote/cloud DevPULSE provider proxy;
- generic browser embedding;
- claiming attached-window visual support without real wgpu proof.

These require new plans or ADR amendments. They are not accidental gaps.
