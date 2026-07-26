# DevPULSE Chat — route-aware peer-window multi-agent platform

**Status:** CANONICAL PLAN — implementation proceeds in the slices below
**Date:** 2026-07-24
**Product and visual authority:**
[`chat-window-product-vision.md`](chat-window-product-vision.md)
**Runtime gate:** the narrow named-provider exception is ratified, but live
launch remains fail-closed on the safety checklist in
[`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md).
**Provider evidence:**
[`chat-provider-capabilities.md`](../research/chat-provider-capabilities.md)
**Active composition slice:**
[`chat-steam-social-foundation.md`](chat-steam-social-foundation.md)

## 1. Product intent

Chat is not a transcript pane, a hub route, or a cosmetic row of provider
tabs. It is a first-class, independently focusable native window in the same
DevPULSE process as the main creative application. The two windows are equal
peers over one shared runtime. Chat remains available while Main is minimized
and owns authenticated coding agents, durable sessions, explicit context,
approvals, tools, artifacts, declarative interactions, and utility windows.

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

1. Main and Chat are independent OS-level peer windows. Neither is the owned
   child of the other, and minimizing or closing one does not implicitly
   minimize or close the other.
2. The main shell owns only a typed Chat launcher/status accessory. It does
   not render the primary Chat experience.
3. `ChatWindowCoordinator` is the sole owner of primary and utility Chat
   viewport lifecycle.
4. Provider integration is registry-driven. Adding a provider adapter does not
   add a provider branch to the shell or Chat UI.
5. Provider credentials remain provider-owned. DevPULSE never reads or stores
   tokens.
6. Route changes may update the visible context offer; they never
   automatically send context or a message.
7. Chat may request a typed main-app route only through an explicit operator
   action or separately approved tool intent. Providers never receive Fabric
   or a shell navigation handle.
8. Context-triggered UI may become available or peek. It does not steal focus
   or open a new window by default.
9. Provider-native history remains the source of truth. DevPULSE may keep
   bounded app-owned metadata, indexes, drafts, links, and provenance.
10. Session handoff creates a new linked session with an explicit context
   export. A Claude session cannot silently become a Codex session.
11. Additional windows are stable Chat surfaces with typed identities and
   lifecycle, not transient ad-hoc `show_viewport` calls.
12. AI-generated UI is a validated declarative `InteractionSpec`, rendered
    from host-owned components. Providers cannot inject arbitrary egui code.
13. Unknown provider events remain inspectable; they are never silently
   dropped.
14. Council/orchestration is last. Every provider must work predictably alone
   before providers can coordinate.
15. Unsupported behavior is visible as `Unavailable` or `Experimental` with a
    reason. Placeholder success is forbidden.

## 2. Current reality

The Chat branch currently contains:

- provider/account/session/event identities and capability contracts;
- a data-driven provider registry and deterministic fake providers;
- bounded, path-jailed read-only history discovery and paging;
- transcript normalization that preserves malformed and unknown input;
- renderer, context, extension, runtime, testkit, and window-state crates;
- the status-bar-triggered peer window (`dpl-chat-window`), which carries the
  read-only session histories and the lite route launcher.

The contracts, stores, readers, normalization, and deterministic tests are
useful evidence. The earlier `ChatHub`-owned embedded three-panel presentation
incorrectly made Chat subordinate to the main shell; it has been retired and
its two unique features (bounded read-only session histories and route
loading) were folded into the peer-window product defined in
[`chat-window-product-vision.md`](chat-window-product-vision.md). Chat is now
the peer window only — there is no rail hub route.

The current peer-window foundation contains an independently discoverable
primary native viewport, `ChatWindowCoordinator`, and launcher lifecycle
evidence. Its permanent library/conversation/work-stage composition is
rejected presentation: this slice keeps the native infrastructure and replaces
the presentation with the compact AI/Social shell.

The rewrite still does not contain:

- typed Social people/project/conversation models;
- isolated AI/Social view state;
- governed overlay coordination with focus restoration;
- snapped/detached/re-docked utility-window attachment state;
- bidirectional route snapshots and typed route-open outcomes;
- the governed Interaction Library;
- live, version-qualified provider runtimes;
- native capture proof for the new Chat and utility surfaces.

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
               ChatStore                    ContextRegistry     InteractionRegistry
                   |                                |                   |
                   +------------------------ ChatViewModel -------------+
                                                    |
                                                    v
                                             dpl-chat-window UI
                                                    |
                                                    v
                                          ChatWindowCoordinator
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

The current live-adapter order is Grok Build first through its official ACP
surface, followed by ChatGPT/Codex through app-server. The operator tests both
before another provider is exposed. The detailed contract is
[`chat-provider-workspaces.md`](chat-provider-workspaces.md). Claude and
Gemini remain later independent qualifications.

Local/Clyffy support may become another registered adapter later. It must not
remain a hardcoded fifth exception in the UI.

### `dpl-chat-context`

Bidirectional route-aware context without hidden data transfer:

- context contributor registry;
- revisioned snapshots;
- item provenance and sensitivity;
- selection and attachment state;
- size estimates and provider compatibility;
- explicit export bundles for turns and handoffs;
- typed route-open requests, provenance, validation, and outcomes.

### `dpl-chat-social`

Social-domain contracts without a network implementation:

- typed person, project, conversation, and message identities;
- `SocialThreadKind::{Direct, Group, Project}`;
- `Presence::{Online, Away, InFlow, Offline}`;
- people, projects, conversations, messages, unread counts, selections, and
  per-conversation drafts;
- explicit transport availability with a required unavailable reason.

Production Social begins empty and unavailable. Deterministic people,
projects, conversations, and messages are supplied only by the named
devtools/test fixture.

### `dpl-chat-extensions`

Feature-owned extension and trigger registry:

- extension manifests;
- placement and ordering;
- route/context/provider predicates;
- permission declarations;
- activation, pin, dismiss, snooze, and restore state;
- typed actions into ChatRuntime or shell surfaces.

Extensions contribute models and actions, not arbitrary egui callbacks from
provider crates.

### `dpl-chat-interactions`

Governed AI/operator surface vocabulary:

- versioned `InteractionSpec` schemas;
- stable interaction identity and provenance;
- typed fields, actions, permissions, and outcomes;
- `Inline`, `Overlay`, `SnappedUtility`, and `DetachedUtility` placements;
- initial notice, approval, route result, and artifact preview content;
- validation, bounds, expiry, dismissal, and retention;
- host-owned native renderers and accessibility semantics;
- fixture serialization and deterministic replay.

It accepts declarative content. It never accepts provider-supplied egui
callbacks, raw colors, fonts, z-order, coordinates, or unrestricted shell
actions.

### `dpl-chat-windows`

Primary and utility-window state:

- stable window identity;
- window kind and binding;
- snapped/detached attachment, anchor edge, parent, and persisted offset;
- viewport generation/lifecycle;
- focus/close/restore policy;
- layout persistence;
- EguiPilotAI viewport metadata.

`ChatWindowId::PRIMARY` is the persistent peer Chat window. Utility windows
may bind to sessions, turns, interactions, routes, or artifacts. The app-owned
`ChatWindowCoordinator` is the sole viewport creator.

### `dpl-chat-testkit`

Deterministic provider fakes and fixtures:

- scripted account/session/event timelines;
- approval and tool scenarios;
- disconnect/crash/timeout injection;
- large-history generators;
- secret-shaped redaction fixtures;
- cross-provider capability matrices.

### `dpl-chat-window`

Primary Chat viewport composition only:

- converts registered state into zones;
- virtualizes lists and transcripts;
- renders normalized events through a renderer registry;
- delegates optional panels through safe extension/interaction models;
- emits typed user intents.

It does not implement `Hub`, own provider processes, mutate routes, or create
viewports.

### `dpl-chat-launcher`

Main-shell accessory only:

- reports Chat open/running/attention state;
- emits open, restore, and focus intents;
- remains registry-provided and shell-generic;
- never expands into a transcript or owns Chat state.

The former `dpl-hub-chat` UI was migration scaffolding. It has been retired
(pre-release, no shim): the peer window (`dpl-chat-window`) absorbed its
read-only session histories, and route loading now happens in the main app via
an explicit `NavigateTo` intent that the coordinator publishes as
`ShellEvent::NavigateRequest`. It defines no public compatibility surface.

### Dependency direction

```text
chat-core
   ^
   +-- chat-store
   +-- chat-context
   +-- chat-extensions
   +-- chat-interactions
   +-- chat-windows
   +-- chat-runtime <-- provider adapters
            ^
            |
       chat-window <-- app/ChatWindowCoordinator
                          |
                          `-- shell renders chat-launcher generically

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

All registered provider launchers remain visible in the persistent rail with
truthful runtime/account state. Selecting or hovering one opens a governed
provider popover. Provider-owned sessions populate the shared `Pinned`,
`Active`, and `Recent` groups only after connection or discovery establishes
them. Implementation order never removes provider choices or other Chat
destinations.

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

The first activation-only slices are independently governed by
[`chat-codex-activation.md`](chat-codex-activation.md) and
[`chat-grok-activation.md`](chat-grok-activation.md). They qualify
provider-owned browser login only. `Ready` authentication does not qualify
conversation Send, sessions, tools, or Council.

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

### Provider workspace isolation

Each provider workspace retains:

- active session;
- selected model/mode/permission;
- composer draft;
- scroll anchor;
- context selection;
- inspector state;
- live turn status;
- open overlays and attached-window links.

Switching workspaces must not destroy, merge, or silently pause another
provider's state. Every launcher renders provider identity and truthful
runtime/account status. The provider popover owns `New chat`, Council, and
Account actions; established sessions populate the shared grouped session
rail and carry provider identity.

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
Main and Chat share a revisioned route bridge. Observation and mutation are
separate contracts.

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

Main publishes the current snapshot whether Chat is visible, minimized, or
closed. Chat may show the route breadcrumb and context offer immediately, but
opening, attaching, or sending remain separate actions.

### Consent rules

- route changes update the context offer only;
- no context item is attached merely because it is visible;
- the composer shows a context strip listing exactly what will be sent;
- every attached item has source, revision, size, and remove action;
- stale items are marked before send;
- sensitive/unshareable items cannot be attached;
- provider adapters receive an explicit immutable export, not shell access;
- every sent bundle records provenance for replay and handoff.

### Chat-to-Main navigation

Chat may emit a typed `RouteOpenRequest` containing:

- requested route and optional typed selection;
- provider/session/event/interaction provenance;
- whether Main may be restored and focused;
- operator-action or approved-tool provenance;
- correlation ID.

Main resolves the request against the same route registry used by the shell
and returns `Opened`, `Unavailable`, `Rejected`, or `Failed`. A provider never
receives Fabric, a hub instance, or an unchecked navigation callback. Model
output may propose a route card; only an explicit typed action can execute it.

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
- offer a utility window.

It may not:

- send a prompt;
- launch a provider;
- approve a tool;
- change route;
- steal keyboard focus;
- open a window automatically unless a separate, explicit safe policy allows
  that exact extension.

## 9. Primary-window zoning

The canonical visual, responsive, and interaction specification is
[`chat-window-product-vision.md`](chat-window-product-vision.md). Architecture
protects these responsibilities:

1. mode strip — DevPULSE Chat identity, `AI`/`Social`, search, notifications,
   account/presence, and window actions;
2. presence/session sidebar — AI providers and sessions or Social people,
   DMs, groups, and projects;
3. conversation — virtualized normalized events and compact inline
   interactions;
4. composer — explicit route context, attachments, component library,
   model/capability, send, stop, and steer;
5. overlay host — hover cards, pickers, menus, approvals, notifications, and
   context peeks;
6. optional utility viewports — sustained artifact, route, diff, plan, and
   tool surfaces.

The primary viewport always has two persistent content columns: sidebar and
conversation. No composition may reserve a permanent work stage, inspector,
or activity column. At widths below 840 points, the 252-point sidebar
collapses to a 76-point presence rail and selection opens through the overlay
host. Responsive compositions retain active identity, conversation, composer,
route context, and running state without uncontrolled nested scrolling.

Approvals, sign-in progress, recovery, command palette, and passive context
peeks use the overlay layer. The main-shell launcher is not part of this
window composition.

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
- `Overlay`
- `UtilityWindow`

Extensions produce declarative view models and typed intents. They do not
receive unrestricted `egui::Context`, provider process handles, arbitrary
filesystem access, or shell mutation.

AI/operator-generated working surfaces use `dpl-chat-interactions`, not this
feature-extension contract. Extensions declare who can contribute; interaction
specs declare the concrete surface instance.

The registry resolves order deterministically:

1. platform-owned mandatory surfaces;
2. active provider surfaces;
3. route-context surfaces;
4. user-pinned extensions;
5. optional suggestions.

Conflicts degrade to one visible diagnostic, never nondeterministic order.

## 11. Application, shell, and Fabric integration

Chat is not a `Hub`. The app owns `ChatWindowCoordinator` beside the main
shell coordinator. Both use shared in-process services while retaining
independent native viewport lifetimes.

The main shell receives Chat only through the generic accessory registry.
Conceptual launcher surface kinds:

- `StatusAction`
- `Dock`
- `OverlayLauncher`
- `WindowFocusRequest`

The app registers Chat's launcher/status producer beside other accessories.
The shell renders it generically and never checks `HUB == chat`. Activating it
emits a typed open/restore/focus intent to `ChatWindowCoordinator`.

Fabric remains appropriate for ephemeral shell signals:

- route changed;
- validated navigate request/outcome;
- window focus/close notices;
- application lifecycle.

Fabric broadcast is not appropriate for provider jobs, transcript streams, or
large context payloads. Those use bounded MPSC/session stores. Route context
is queried by revision through `dpl-chat-context`, not broadcast as a blob.

## 12. Peer and utility-window lifecycle

The primary Chat window has a declared singleton identity and peer lifetime.
Every Chat window has:

- `ChatWindowId`;
- `WindowKind`;
- optional provider/session/turn binding;
- optional route/context binding;
- `WindowAttachment::{Snapped, Detached}` for utilities;
- anchor edge, parent window, and persisted offset for snapped utilities;
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

- `ChatWindowCoordinator` owns viewport creation;
- the primary Chat viewport is not an owned child of Main;
- minimizing or closing Main cannot minimize or close Chat;
- application exit occurs on last-peer close or explicit Quit;
- provider adapters request typed intents and never create viewports;
- closing a window never closes its session unless explicitly chosen;
- snapped utilities open adjacent to Chat without a taskbar entry;
- detaching recreates a utility as decorated while retaining its binding and
  generation lineage;
- re-docking restores the recorded anchor edge and offset;
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
- primary and utility-window discover/focus/close/reopen;
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

The original CH1 provider-neutral contracts and CH2 read-only backing store
are retained as implementation evidence. Their embedded hub presentation is
not retained. Product execution now follows the corrected `CHAT-*` sequence
from
[`chat-window-product-vision.md`](chat-window-product-vision.md):

### CHAT-0 — contract freeze

- reconcile product vision, architecture, roadmap, and census;
- mark the embedded CH2 screen as rejected presentation;
- retain and freeze provider/store compatibility evidence;
- define deterministic window, route, and interaction fixtures.

Exit: no active plan calls Chat a hub or calls its primary viewport an
attachment.

### CHAT-1 — peer-window foundation

- app-owned `ChatWindowCoordinator`;
- one real independently discoverable primary Chat viewport;
- main-shell launcher/status accessory;
- independent minimize, focus, close, reopen, geometry restore, explicit Quit,
  and last-window exit;
- frozen native peer-window and launcher evidence;
- stable EguiPilotAI viewport-scoped identity.

Exit: Chat remains interactive while Main is minimized in the real native
executable.

### CHAT-1A — Steam-Chat AI/Social foundation

- compact `AI`/`Social` mode strip and isolated domain state;
- Steam Friends & Chat-style presence/session sidebar;
- dominant conversation and per-domain/per-conversation composer;
- typed Social people, projects, conversations, presence, messages, and
  unavailable transport;
- governed overlay coordinator and initial interaction vocabulary;
- snapped, detached, and re-docked utility-window lifecycle;
- devtools-only authored fixture and native Chat/utility capture proof.

Exit: the accepted compact AI, Social, hover, approval, compact, and snapped
preview states are proven without placeholder transport success.

### CHAT-2 — read-only sessions in the peer window

- bind the existing provider registry, bounded readers, paging, normalization,
  and renderer contracts into `dpl-chat-window`;
- bind into the compact AI sidebar and conversation without structural layout
  branches;
- prove provider view-state isolation and large-history virtualization.

Exit: fixture and real local histories browse read-only in the peer window
without provider writes or credential inference.

### CHAT-3 — bidirectional route awareness

- revisioned route snapshots and context contributors;
- explicit context strip/stage with provenance;
- typed route-open request/outcome;
- Chat-driven Main restore/focus on explicit action;
- stale, unavailable, rejected, and failed route behavior.

Exit: route changes update Chat without implicit attachment or sending, and
Chat can explicitly open a valid route in Main.

### CHAT-4 — Interaction Library

- `dpl-chat-interactions` schema, validation, registry, and native renderers;
- initial notice, approval, route result, and artifact preview;
- inline, overlay, snapped-utility, and detached-utility placement;
- deterministic fixtures, accessibility, and privileged-action gates.

Exit: AI can propose useful native surfaces without arbitrary UI or shell
privilege.

### CHAT-5 — Grok and ChatGPT/Codex live qualification

Implementation is authorized only after every runtime ADR launch gate is
green.

- follow [`chat-provider-workspaces.md`](chat-provider-workspaces.md);
- Grok Build first: ACP v1 lifecycle, cached provider-owned authentication,
  session list/new/load, streaming prompt, cancellation, and teardown;
- local and operator checkpoint before starting ChatGPT/Codex;
- Codex app-server lifecycle, ChatGPT-owned authentication, threads, turns,
  streaming, plans, diffs, tools, approvals, usage, and interrupt;
- persistent provider launcher popovers and dynamic provider-session
  navigation;
- real version-pinned conformance and child teardown for both providers.

Exit: both subscription-backed providers work independently, leave no orphan
process, and have been tested before any additional provider is exposed.

### CHAT-6 — Claude live adapter

- documented structured stream;
- Pro/Max-owned authentication state;
- start/resume/continue;
- model/permission/max-turn behavior;
- conformance and normalization.

Exit: Claude works alone without provider-specific primary-layout branches.

### CHAT-7 — Gemini qualification

- explicit operator installation;
- ACP boundaries and provider-owned authentication;
- session/prompt/cancel/mode/model conformance;
- experimental qualification and version-drift handling.

Exit: Gemini is qualified, experimental with a reason, or unavailable after
the two-provider checkpoint.

### CHAT-8 — advanced surfaces

- richer artifacts, reviewable plans/diffs, tool consoles, and comments;
- detachable context stage and saved utility-window layouts;
- background/monitoring activity;
- declared feature extensions and route triggers.

Exit: adding a surface does not modify provider adapters or the primary
window's structural layout.

### CHAT-9 — handoff and Council

- explicit cross-provider export/handoff;
- operator/agent roles;
- turn/cost/time caps;
- reversible Council plan and approval;
- COSTAR templates as data.

Exit: orchestration reuses qualified single-provider primitives and cannot
bypass context consent, route validation, capability, or approval policy.

## 17. Acceptance contract

The Chat program is not complete until:

- Main and Chat are independently discoverable native peer windows;
- Chat remains interactive when Main is minimized;
- closing/minimizing either peer follows the declared policy without
  implicitly closing/minimizing the other;
- the main shell exposes only a launcher/status accessory, not embedded Chat;
- provider tabs come solely from a registry;
- each provider has isolated account/session/draft/mode state;
- every visible action is capability-gated and truthful;
- credentials never enter DevPULSE-owned state or artifacts;
- route context never attaches or sends itself;
- explicit Chat route actions resolve through typed validation and outcomes;
- sessions are paged, virtualized, and bounded;
- unknown provider data is preserved and diagnosable;
- extensions use declared placements/triggers and deterministic order;
- generated interactions are schema-bound, host-rendered, and privilege-gated;
- overlays return focus predictably;
- primary and utility windows have stable identity, restore safely, and
  preserve sessions;
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
- provider-supplied egui callbacks or arbitrary generated layout;
- hidden cross-provider continuation;
- remote/cloud DevPULSE provider proxy;
- generic browser embedding;
- claiming peer/utility-window visual support without real wgpu proof.

These require new plans or ADR amendments. They are not accidental gaps.
