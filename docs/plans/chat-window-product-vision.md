# DevPULSE Chat — peer-window product and interaction vision

**Status:** CANONICAL PRODUCT VISION — operator-locked
**Date:** 2026-07-24
**Architecture:** [`chat-architecture.md`](chat-architecture.md)
**Provider evidence:**
[`chat-provider-capabilities.md`](../research/chat-provider-capabilities.md)
**Runtime safety gate:**
[`chat-provider-runtime-adr.md`](chat-provider-runtime-adr.md)

## 1. The product in one sentence

DevPULSE and Chat are one native Rust application with two equal, independent
top-level windows: the main creative application and a persistent agent
workspace that understands the main application's route, can offer or request
navigation, and can render governed AI-generated working surfaces.

Chat is not:

- a hub route in the main window;
- a transcript page inside the shell;
- a temporary overlay;
- a collection of provider logos;
- a webview around another chat product;
- a provider-controlled UI host.

The small in-app Chat affordance is a launcher and status companion. It opens,
restores, or focuses the real Chat window. It never expands into the full
conversation experience.

## 2. Locked correction

The existing provider-neutral contracts, bounded history readers, event
normalization, capability qualification, and deterministic fixtures remain
valid foundations. The embedded `dpl-hub-chat` presentation is rejected as
the product direction.

These decisions outrank the older Chat slice order:

1. The main application window and Chat window are OS-level peers.
2. Minimizing the main window does not minimize, hide, or suspend Chat.
3. Closing either peer does not implicitly close the other. DevPULSE exits
   when the last peer closes or the operator invokes an explicit application
   Quit action.
4. Neither peer is an owned child window of the other.
5. Both peers share one in-process runtime, Fabric, Chat store, provider
   registry, route bridge, and ColdLight design system.
6. The main shell contains only a typed Chat launcher/status accessory. It
   does not own Chat layout.
7. Chat may create additional typed utility windows, but the primary Chat
   window is not classified as an attachment.
8. Route observation is automatic; route mutation and context transmission
   remain explicit, visible actions.
9. AI-generated UI is declarative, schema-bound, capability-gated, and
   rendered by DevPULSE. A provider cannot execute arbitrary egui code.
10. The product must remain useful with one provider, one session, and no
    generated surfaces before orchestration or Council is added.

## 3. What current agent clients teach us

The goal is synthesis, not imitation. Current products consistently separate
conversation from the work it produces and make long-running agent activity
supervisable.

| Reference | Pattern worth adopting | What DevPULSE rejects |
|---|---|---|
| OpenAI Codex app | Project-grouped agent threads, visible progress, parallel work, reviewable diffs, and an agent command-center posture | A developer-only information architecture or provider-specific assumptions |
| Cursor 3 | A high-level agents window with a persistent sidebar, local/cloud state, verification artifacts, and drill-down review | Folding Chat back into an IDE or making files the primary navigation |
| ChatGPT Canvas | Conversation beside a focused, editable work surface with explicit version/change controls | Automatically replacing the primary conversation with a generated canvas |
| Claude Desktop/Cowork | Chat, work, and code modes; persistent artifacts with their own identity and history; explicit deep links | Provider-rendered HTML as an unrestricted application extension mechanism |
| Perplexity Computer | Task-first language, background work, iteration while work continues, and clear connector/action posture | A single opaque “computer” actor that hides provider ownership or approvals |
| Steam desktop client | A durable library rail, compact high-signal status, immersive dark framing, overlays, pinning, and remembered selection | Storefront clutter, game branding, or decorative chrome that steals space from work |

The resulting DevPULSE pattern is:

> A calm, dense agent library on the left; a readable conversation in the
> center; a contextual work stage that appears only when useful; a persistent
> composer and activity shelf; and separate utility windows for work that
> deserves independent focus.

## 4. Native window topology

```text
DevPULSE process
|
+-- AppCoordinator
|   +-- MainWindowCoordinator
|   |   +-- main creative viewport
|   |   `-- Chat launcher/status accessory
|   |
|   +-- ChatWindowCoordinator
|   |   +-- primary Chat viewport
|   |   +-- typed auxiliary Chat viewports
|   |   `-- persisted geometry/focus policy
|   |
|   +-- RouteBridge
|   +-- ChatRuntime
|   +-- ChatStore
|   +-- InteractionRegistry
|   `-- ProviderRegistry
|
`-- shared ColdLight, Fabric, diagnostics, and EguiPilotAI instrumentation
```

### Window roles

| Window | Role | Lifetime |
|---|---|---|
| Main | Creative authoring, capture, library, shows, and presentation | Independent peer |
| Chat | Providers, sessions, conversation, context, tools, and generated working surfaces | Independent peer |
| Launcher | Small main-window accessory that reports Chat status and focuses/opens Chat | Lives with Main |
| Utility | Diff, artifact, plan, tool output, route preview, or focused conversation | Typed child of the Chat subsystem, not an OS-owned child of Main |

### Lifecycle laws

- `ChatWindowCoordinator` is the only owner of Chat viewport creation.
- One stable `ChatWindowId::PRIMARY` identifies the primary Chat viewport.
- Reopening Chat restores its last valid geometry, selected provider, active
  session, drafts, and non-secret view state.
- Geometry restoration clamps to available monitors and declared minimum size.
- Minimize, restore, focus, close, and reopen are explicit state transitions.
- Main-window minimize cannot cascade into Chat.
- Chat can restore and focus Main only after an operator action or an approved
  typed intent.
- Provider activity continues while either peer is minimized, subject to the
  provider runtime policy and explicit cancellation.
- A crash or provider disconnect cannot resurrect a stale window/session
  binding.

The first implementation proof must use the real eframe/wgpu executable. The
exact egui viewport API is selected from measured behavior, not assumed from
documentation. If the selected mode cannot produce two independently
discoverable native windows, the slice stops and the plan is amended.

## 5. Primary Chat window anatomy

The primary window uses a stable five-zone frame:

```text
+--------------------------------------------------------------------------+
| A  Command header: workspace / route / search / account / window actions |
+------------------+--------------------------------+----------------------+
| B  Agent library | C  Conversation workspace     | D  Context work stage|
|                  |                                |                      |
| providers        | messages                       | artifact / diff      |
| sessions         | plans and tool events          | route preview        |
| projects         | approvals and citations        | inspector / form     |
| activity badges  | generated inline interactions | detachable surfaces  |
|                  |                                |                      |
+------------------+--------------------------------+----------------------+
| E  Composer deck: context chips / mode / attachments / send / stop       |
+--------------------------------------------------------------------------+
| F  Activity shelf: running / waiting / approval / failed / completed     |
+--------------------------------------------------------------------------+
```

Zone D is conditional. When it is absent, the conversation expands rather
than leaving an empty inspector. Zone F can collapse to a one-line status
strip while preserving active-task and approval counts.

### A — command header

- DevPULSE Chat identity and current workspace;
- observed main-app route breadcrumb;
- `Open in Main` / `Show Main` action when a route binding exists;
- command palette and global search;
- offline/degraded/provider-runtime health;
- account switcher and application-level window actions.

The header does not become a second navigation tree. It answers: where am I,
what application context is available, and what is running?

### B — agent library

This is the Steam-inspired durable collection surface:

- registry-driven Codex, Claude, Gemini, and Grok provider tabs;
- truthful signed-out, unavailable, experimental, degraded, running, and
  attention states;
- provider-local project/workspace groups;
- pinned and recent sessions;
- filters for active, unread, waiting, failed, and archived;
- type-to-search and keyboard list navigation;
- remembered expansion, selection, and scroll state;
- compact status glyphs with text/tooltips, never color alone.

Each provider retains its own active session, account, model, mode,
permissions, draft, scroll anchor, context selection, and pending work.
Switching providers never merges or destroys those states.

An optional `All activity` lens may aggregate status. It is not a synthetic
provider and cannot send messages.

### C — conversation workspace

- readable content measure instead of full-width transcript lines;
- virtualized, cursor-paged history;
- explicit user, assistant, system, provider, tool, and approval roles;
- streaming without moving earlier content;
- collapsible reasoning summaries, plans, tool calls, diffs, and diagnostics;
- citations, provenance, timestamps, and provider-native identity where
  useful;
- stable scroll anchoring and an explicit `Jump to live` affordance;
- inline interaction surfaces when they are small and conversational;
- no chat bubbles for large plans, diffs, tables, or artifacts.

The conversation is the audit trail. A work surface may move to the context
stage or a utility window, but its origin and outcome remain represented in
the thread.

### D — context work stage

The stage is a typed companion to the conversation, inspired by Canvas and
Artifacts:

- route context and preview;
- editable documents and authored plans;
- diffs and review comments;
- structured tool output;
- images, captures, guides, slides, and other DevPULSE artifacts;
- forms, comparison tables, and generated dashboards;
- provider diagnostics and permission details;
- version history and restore where the surface contract supports it.

Stage surfaces can be pinned, replaced, returned to their originating event,
or detached into a utility window. Replacing a stage surface never destroys
its underlying artifact or thread event.

### E — composer deck

- multiline drafting with predictable keyboard behavior;
- exact attached-context chips with source, revision, size, and remove action;
- model, mode, permission, and provider capability summary;
- attachment and interaction-library actions;
- send, stop, steer, and approval state;
- per-provider/per-session draft persistence;
- disabled actions with a truthful reason;
- no prompt submission caused by route change, tab change, or window restore.

### F — activity shelf

Long-running work must not hide inside a scrolling transcript. The shelf
provides a compact task queue:

- running;
- waiting for operator;
- approval required;
- blocked/degraded;
- completed recently;
- background/monitoring;
- provider and session ownership;
- elapsed time and bounded status text;
- focus, cancel, retry, or open-detail actions when supported.

This borrows Steam's download/status clarity and modern agent clients'
background task supervision without turning the conversation into a log
console.

## 6. Responsive and windowed behavior

The window supports three authored compositions:

### Wide

- agent library, conversation, and context stage visible;
- activity shelf expanded on demand;
- intended for primary work and review.

### Standard

- agent library remains visible;
- context stage becomes a drawer or a detachable window;
- conversation and composer remain stable.

### Companion

- session picker replaces the permanent library rail;
- context stage is detached or modal-by-intent, never overlaid by accident;
- provider identity, conversation, composer, running state, and route context
  remain visible;
- intended to sit beside another application or stay useful while Main is
  minimized.

The implementation uses ColdLight fluid constraints and declared minimums,
not a pile of magic pixel branches. Every composition receives its own
EguiPilotAI scenario and visual baseline.

## 7. Visual direction — ColdLight Ember

The target is “Steam-like in confidence and density, DevPULSE in identity.”

### Palette and depth

- near-black navy base;
- deep teal gradient moving upward and inward from the lower frame;
- diffused orange ember flecks rising sparsely from the bottom edge;
- cool cyan for route/application context;
- warm orange for active work, progress, and intentional emphasis;
- provider accents remain small identity markers, not full-window themes;
- errors, warnings, success, and approvals use semantic ColdLight tokens.

### Material law

- crisp edges and fine borders;
- layered opaque or nearly opaque plates;
- tint gradients and black diffusion scatter;
- no backdrop blur;
- no generic glassmorphism;
- no giant empty hero treatment once a session is active;
- no neon outline around every control;
- elevation is expressed through tone, border, and restrained shadow.

### Ember field

The orange flecks are an environmental backplate, not decoration on controls:

- concentrated near the bottom;
- varied but sparse size and opacity;
- slow upward drift with lateral diffusion;
- no fleck crosses the readable composer text plane at high contrast;
- pointer interaction does not affect the field;
- reduced motion freezes it at a composed phase;
- EguiPilotAI suppression resolves it to a canonical deterministic frame.

### Typography and density

- humanist display face for conversation and section titles;
- monospaced face for paths, commands, code, IDs, timing, and diagnostics;
- compact library rows with a generous conversation line height;
- sentence case for labels;
- uppercase tracked micro-labels only for state or grouping;
- strong focus rings and never color-only selection.

### Motion

- short, purposeful transitions for drawers, stage swaps, and status changes;
- no ambient pulsing on actionable controls;
- streaming status may animate only in its local indicator;
- every finite motion consumes `PilotClock`;
- the ember field registers as ambient motion with a canonical capture phase.

## 8. Bidirectional route bridge

Route awareness is shared application state, not transcript scraping.

### Main to Chat

Main publishes a revisioned `RouteSnapshot`:

- hub and subroute IDs;
- selected artifact/document/capture/episode IDs;
- viewport identity;
- available route actions;
- context contributors and sensitivity;
- revision and content hash.

Chat displays the route breadcrumb and context offer. It does not attach,
summarize, or send route data until the operator selects it.

### Chat to Main

Chat emits a typed `RouteOpenRequest` containing:

- requested route and optional typed selection;
- source provider/session/event/surface;
- whether Main may be restored and focused;
- operator-action or approved-tool provenance;
- correlation ID.

Main validates the route against its registry and responds with
`Opened`, `Unavailable`, `Rejected`, or `Failed`. Providers never receive a
raw Fabric handle.

Generated route cards expose visible actions such as:

- `Preview route`;
- `Open in Main`;
- `Restore Main and open`;
- `Attach current route context`;
- `Return to originating route`.

A model can propose one of these actions. It cannot silently execute it.

## 9. Governed AI Interaction Library

The Interaction Library is how Chat grows without becoming an arbitrary UI
runtime. Providers and DevPULSE features produce declarative specs; the host
validates them and renders known native components.

### Initial surface kinds

- `Notice`
- `Choice`
- `Checklist`
- `Form`
- `Progress`
- `Approval`
- `Plan`
- `Diff`
- `Code`
- `Table`
- `Chart`
- `Artifact`
- `RouteCard`
- `Inspector`
- `Gallery`
- `Toast`
- `Popup`
- `WindowRequest`

Every `InteractionSpec` includes:

- schema version;
- stable `InteractionId`;
- provider/session/turn/event provenance;
- title, purpose, and accessibility label;
- declared placement and size class;
- typed fields and actions;
- capability and permission requirements;
- data sensitivity and retention policy;
- expiry/dismiss policy;
- bounded content and attachment references;
- deterministic ordering key.

### Placements

- `Inline` — small, directly conversational;
- `Stage` — sustained review or editing;
- `Overlay` — brief blocking decision such as an approval;
- `Toast` — non-blocking status with no complex input;
- `UtilityWindow` — independent work deserving persistent space.

### Lifecycle

```text
Proposed -> Validating -> Visible -> Interacting -> Completed
                   \-> Rejected       \-> Dismissed
                                      \-> Expired
```

### Safety and authorship laws

- providers cannot supply arbitrary egui callbacks;
- providers cannot choose raw colors, fonts, z-order, or screen coordinates;
- host tokens, components, layout rules, and accessibility behavior are final;
- every action is typed and capability-checked;
- route mutation, provider launch, file mutation, approvals, and window
  creation remain explicit privileged actions;
- invalid or unsupported specs render a bounded diagnostic, never a blank
  success surface;
- generated surfaces are serializable into fixtures for replay and tests;
- important surfaces retain a thread event and audit outcome.

The library is intentionally useful to both AI and operator. The operator can
invoke the same components directly, pin them, move them to the stage, detach
them, or dismiss them.

## 10. Auxiliary windows and overlays

Utility windows support:

- focused conversation;
- diff review;
- artifact preview/editor;
- plan and task timeline;
- tool output;
- route preview;
- provider diagnostics;
- future Council.

Each has a stable `ChatWindowId`, kind, binding, generation, geometry,
restore/focus policy, and EguiPilotAI viewport identity.

Overlays are reserved for short-lived decisions or launch affordances:

- approval;
- sign-in progress;
- command palette;
- context peek;
- warning/recovery;
- launcher.

An overlay is never the primary Chat experience. It returns focus
deterministically and cannot open a utility window without an explicit typed
action.

## 11. Boring implementation boundaries

The corrected crate direction is:

```text
dpl-chat-core
   ^
   +-- dpl-chat-store
   +-- dpl-chat-context
   +-- dpl-chat-interactions
   +-- dpl-chat-windows
   +-- dpl-chat-runtime <-- named provider adapters
            ^
            |
      dpl-chat-window
            ^
            |
           app
            |
            `-- shell-owned dpl-chat-launcher accessory

dpl-chat-testkit -> contract consumers in tests only
```

Responsibilities:

- `dpl-chat-window` composes the primary Chat viewport and no provider
  process;
- `dpl-chat-windows` owns all Chat window state and lifecycle;
- `dpl-chat-interactions` owns declarative generated-surface contracts;
- `dpl-chat-context` owns observed context, consent, and the route bridge
  models;
- the app owns `ChatWindowCoordinator` and OS lifecycle;
- the shell renders only the registry-provided launcher/status accessory;
- `dpl-hub-chat` is retired as the primary product UI. Any temporary embedded
  route is migration scaffolding and must not become a compatibility promise.

No widget owns a subprocess, store scan, route mutation, or viewport
lifecycle. UI emits intents; coordinators change state; immutable revisions
flow back to UI.

## 12. Corrected build sequence

### CHAT-0 — contract freeze

- land this product vision and reconcile architecture/roadmap/census;
- mark the embedded CH2 screen as rejected presentation evidence;
- preserve provider-neutral/store tests;
- write fixture contracts for window, route, and interaction state.

Exit: no document calls Chat a hub or treats the primary window as an
attachment.

### CHAT-1 — peer-window foundation

- implement `ChatWindowCoordinator`;
- create one real independent native Chat viewport;
- replace the shell Chat route with launcher/status behavior;
- prove open, focus, minimize independence, close, reopen, geometry restore,
  explicit Quit, and last-window exit;
- add primary/launcher typed IDs and EguiPilotAI viewport metadata;
- author the empty/new-session visual shell and three responsive compositions.

Exit: Chat remains fully interactive while Main is minimized, using the real
executable.

### CHAT-2 — bind the proven read-only model

- bind existing registry, capability, history, paging, normalization, and
  renderer work into `dpl-chat-window`;
- remove the embedded three-panel page as product authority;
- add the agent library, conversation workspace, and fixture providers;
- prove large-history virtualization and provider-state isolation.

Exit: real local Codex/Claude histories browse read-only in the peer window
without credential inference.

### CHAT-3 — bidirectional route awareness

- implement revisioned `RouteSnapshot` and `RouteOpenRequest`;
- add route breadcrumb, context offer, route cards, and explicit Main restore;
- prove Main-to-Chat updates and Chat-to-Main navigation;
- add consent, stale-context, and unavailable-route failures.

Exit: Chat and Main feel like one application without hidden context transfer
or automatic navigation.

### CHAT-4 — Interaction Library

- implement schema, validation, registry, placements, and native renderers;
- start with Notice, Choice, Form, Progress, Approval, RouteCard, Artifact,
  Diff, and WindowRequest;
- add deterministic recorded fixtures and accessibility semantics;
- prove inline, stage, overlay, and utility-window movement.

Exit: AI can propose useful native UI without arbitrary layout or privilege.

### CHAT-5 — live Codex

- satisfy every runtime ADR gate;
- integrate app-server account, threads, turns, plans, diffs, tools,
  approvals, usage, and interrupt;
- qualify pinned real executables and teardown.

Exit: one subscription-backed provider works alone end to end.

### CHAT-6 — live Claude

- integrate the documented structured stream and provider-owned Pro/Max auth;
- qualify start/resume, models, permission modes, bounded turns, and teardown;
- map provider-specific capabilities without UI branches.

Exit: Claude works alone using the same window and interaction contracts.

### CHAT-7 — Gemini and Grok qualification

- install only by explicit operator action;
- qualify ACP boundaries independently;
- keep unsupported or experimental behavior visible.

Exit: each provider is truthfully qualified, experimental, or unavailable.

### CHAT-8 — advanced surfaces

- richer artifacts, editable plans, review comments, tool consoles;
- detachable stage surfaces and saved layouts;
- background/monitoring activity;
- provider and route extensions through declared registries.

Exit: adding a surface does not modify the primary layout or provider
adapters.

### CHAT-9 — handoff and Council

- explicit cross-provider export and provenance;
- operator/agent roles and caps;
- reversible orchestration plans and approvals;
- Council only over independently qualified providers.

Exit: orchestration cannot bypass consent, capability, or approval policy.

## 13. Proof matrix

The product direction is not qualified until the real app proves:

| Behavior | Required evidence |
|---|---|
| Main minimized, Chat active | OS-visible peer window remains interactive and provider/session state advances |
| Chat minimized, Main active | Main remains unaffected; Chat restores to the same session and draft |
| Independent close/reopen | Closing either peer preserves the other and restores valid state |
| Last-window exit | Process exits cleanly with no provider child left alive |
| Route observation | Main route revision appears in Chat without sending context |
| Route opening | An explicit Chat action restores/focuses Main and opens the typed route |
| Invalid route | Truthful unavailable/rejected result with no navigation side effect |
| Provider isolation | Switching providers preserves account/session/draft/mode state |
| Generated surface | Fixture spec renders identically inline, staged, and detached where allowed |
| Privileged action | Approval and provenance are visible before route/file/provider/window mutation |
| Multi-viewport semantics | Stable viewport-scoped IDs and deterministic EguiPilotAI discovery |
| Responsive states | Wide, standard, and companion layouts have no overlap or uncontrolled nested scrolling |
| Motion capture | Fifty suppressed captures are byte-identical for the pinned renderer profile |
| Accessibility | Keyboard-only launcher, provider/session switching, composer, stage, approvals, and window movement |
| Scale | Large histories remain paged/virtualized and background activity remains bounded |

## 14. Anti-goals

- rebuilding Slack, Discord, or a browser chat page in egui;
- making Chat subordinate to the main window;
- opening context-triggered windows without consent;
- auto-sending visible route data;
- giving providers unrestricted rendering or shell access;
- provider-specific UI forks in the primary composition;
- full-window provider branding;
- decorative glass, blur, or animation that compromises readability;
- adding Council before single-provider workflows are boring and reliable;
- claiming multi-window support before the real native executable proves it.

## 15. Research references

- [Introducing the Codex app](https://openai.com/index/introducing-the-codex-app/)
- [Meet the new Cursor](https://cursor.com/blog/cursor-3)
- [ChatGPT Canvas](https://help.openai.com/en/articles/9930697-what-is-the-canvas-feature-in-chatgpt-and-how-do-i-use-it)
- [Claude live artifacts](https://support.claude.com/en/articles/14729249-use-live-artifacts-in-claude-cowork)
- [Claude Desktop deep links](https://support.claude.com/en/articles/14729294-open-claude-desktop-with-a-link)
- [Perplexity Computer](https://www.perplexity.ai/products/computer)
- [Steam desktop client update and overlay model](https://store.steampowered.com/news/posts/?feed=steam_client)

These references justify interaction patterns, not visual copying. ColdLight,
the peer-window contract, the provider boundary, and the Interaction Library
remain DevPULSE-authored.
