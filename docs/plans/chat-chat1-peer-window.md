# CHAT-1 Implementation and Qualification Plan

Status: implemented and locally qualified on 2026-07-24
Branch: `codex/chat-platform`
Depends on: `chat-window-product-vision.md`, `chat-architecture.md`,
`eguipilotai.md`

## 1. Outcome

CHAT-1 replaces Chat-as-a-hub with one app-owned, independently repainting
native Chat peer window. Main retains only a generic launcher/status
accessory. Closing or minimizing Main does not close or minimize Chat.
Closing Chat preserves provider and view state for a later reopen. The
process exits only after the last visible peer closes or after explicit Quit.

This slice authors the durable window and UI foundation. It does not pretend
that a provider can send messages before its adapter and authentication gates
qualify.

## 2. Measured eframe 0.35 facts

The implementation is selected from the checked-in dependency version rather
than assumed from examples:

- `show_viewport_deferred` creates a native `winit` window through eframe's
  viewport integration and gives it an independent repaint callback.
- eframe does not attach an operating-system owner/parent handle when it
  creates a secondary window. The egui parent relation is render/lifecycle
  metadata, not an OS ownership relation.
- deferred viewports can repaint while the root viewport is sleeping,
  occluded, hidden, or minimized.
- the root close request can be canceled with
  `ViewportCommand::CancelClose`.
- secondary close requests become viewport input state. The application
  closes a secondary viewport by no longer declaring it.
- `ViewportCommand::{Visible, Minimized, Focus}` provide explicit
  hide/restore/focus behavior.

Consequences:

1. Primary Chat uses a deferred viewport, not PixVix's immediate palette
   pattern.
2. Main-close is intercepted only while Chat remains a live peer. Main is
   hidden and can be restored.
3. If hidden Main is the only remaining event-loop root and Chat closes, the
   coordinator explicitly closes the root so the process exits.
4. The Chat viewport is shown on the Windows taskbar and keeps ordinary
   decorations. It is neither always-on-top nor an overlay.

## 3. Boring ownership boundaries

### App

The app owns `ChatWindowCoordinator` beside `Shell` and `PixVix`.

The coordinator alone:

- creates and removes Chat viewports;
- translates generic shell window requests into lifecycle changes;
- observes route signals without attaching or sending route context;
- restores/focuses Main after an explicit Chat action;
- applies last-peer and explicit-Quit policy;
- transfers immutable viewport state into the Chat UI.

### `dpl-chat-windows`

This crate owns the deterministic model:

- `ChatWindowId::PRIMARY`;
- lifecycle and generation;
- last valid logical geometry;
- peer visibility and exit decisions;
- typed intents and transition errors.

It remains independent of egui and operating-system APIs.

### `dpl-chat-window`

This new crate owns primary Chat composition:

- command header;
- agent library;
- conversation/new-session workspace;
- conditional work stage;
- composer;
- activity shelf;
- wide, standard, and companion responsive compositions;
- stable semantic controls and typed UI intents.

It does not create viewports, publish Fabric signals, navigate Main, scan
stores, or launch providers.

### Shell

Shell gains a generic window-accessory registry. An accessory is data:

- stable app-window key;
- label, glyph, tone, tooltip, and semantic identity.

The status grid renders registered accessories and emits a generic typed
window request. It contains no Chat branch and no Chat transcript.

### Fabric

Fabric carries the small ephemeral request:

`WindowRequest { target, action: OpenOrFocus }`

It does not carry transcript, provider, or context payloads.

### EguiPilotAI

`ContextPilotExt` gains the deferred-viewport sibling of the current
immediate wrapper. It:

- declares the stable typed viewport key and class;
- forwards queued native commands from the declaration owner, because a
  minimized deferred child cannot repaint its own callback to restore itself;
- leaves rendering and lifecycle with the app.

The primary Chat key is `chat.primary`. Reopen advances its discovered
lifetime generation while its declared key stays stable.

## 4. Initial behavior

Chat starts closed during ordinary launches and open under the named
`chat-peer` fixture or `--chat-open`.

The Main footer accessory:

- shows Chat;
- restores it if minimized;
- focuses it if already visible;
- reports a truthful planned-provider state.

The primary Chat window:

- shows Codex, Claude, Grok, and Gemini from the provider registry;
- preserves provider-local selection and drafts;
- labels every unqualified adapter as planned/blocked;
- displays the current Main route as observed context only;
- offers `Show Main` and `Quit DevPULSE` window actions;
- keeps send unavailable while the selected provider cannot send;
- provides no fake sign-in, fake session, or fake streaming behavior.

## 5. Responsive compositions

The same zones recompose at declared thresholds:

- Wide (`>= 1180` logical points): agent library, conversation, and work
  stage are simultaneous columns.
- Standard (`>= 760`): agent library and conversation remain; the work stage
  becomes an explicit tab/rail surface.
- Companion (`< 760`): one primary surface at a time, compact provider tabs,
  route strip, conversation, composer, and activity shelf.

No state is duplicated across layouts. Threshold selection is a pure
function with boundary tests.

## 6. Visual contract: ColdLight Ember

- near-black navy base;
- deep-teal vertical/diagonal tint in the conversation well;
- deterministic sparse amber ember flecks rising from the lower quarter;
- crisp hairlines and hard action shadows;
- clay passive planes;
- cyan for app/route context;
- provider accent only for provider identity;
- amber for attention and lifecycle;
- no blur, translucent web glass, webview, HTML, CSS, or JavaScript.

The ember field is derived from fixed geometry hashes. It does not animate in
CHAT-1, so captures are deterministic.

## 7. State and lifecycle rules

Primary:

`Closed -> Requested -> Opening -> Visible -> Closing -> Closed`

- Opening a visible window issues restore/focus commands and does not create
  a second viewport.
- Reopening a closed window increments generation exactly once.
- Closing preserves selected provider, drafts, observed route, and last
  valid geometry.
- Geometry updates accept finite positive values only and clamp the reopen
  size to the declared minimum.
- The 1120 × 640 logical default fits a 1536 × 864 desktop at 125% scaling,
  including native decorations; Chat must not debut partly off-screen.
- Main close while Chat is live becomes Main hidden.
- Main close while Chat is not live exits normally.
- Chat close while Main is visible closes Chat only.
- Chat close while Main is hidden closes the root and exits.
- Explicit Quit closes the root regardless of peer visibility.

## 8. Semantic contract

Stable nodes:

- `shell.window.chat`
- `chat.header.route`
- `chat.header.show-main`
- `chat.header.quit`
- `chat.providers`
- `chat.providers/<provider-id>`
- `chat.workspace.new-session`
- `chat.composer`
- `chat.composer.send`
- `chat.activity`

Every interactive control has a tooltip and an actionability state. Dynamic
provider IDs use the declared parent plus stable provider key.

## 9. Proof matrix

### Pure/model

- primary singleton identity;
- legal and illegal lifecycle transitions;
- reopen generation;
- geometry validation and restoration;
- last-peer decisions;
- responsive boundary selection;
- provider selection and draft preservation.

### egui harness

- all four provider controls are discoverable;
- planned provider cannot send;
- three responsive compositions render without overlap;
- launcher and Chat controls have stable semantic IDs;
- deterministic ember paint produces a stable primitive count.

### Real native executable

Named fixture: `chat-peer`

1. start the real wgpu executable with Chat open;
2. discover root and `chat.primary`;
3. verify the route-aware header and select a blocked provider with real
   pointer input;
4. minimize and restore the native Chat child;
5. minimize Main and prove Chat remains visible and actionable;
6. restore Main through Chat's `Show Main` control;
7. close Chat, reopen it through the Main launcher, and rediscover its
   provider controls;
8. request Chat capture and receive the explicit deferred-capture capability
   failure before the run starts;
9. stop the session cleanly.

No claim is made from a root-only screenshot. The eframe 0.35 Windows wgpu
backend did not deliver queued screenshot requests for the background deferred
Chat viewport. Discovery/input/lifecycle remain qualified and deferred capture
is recorded as an explicit backend limitation.

## 10. Local release gate

Before push:

- formatting;
- warning-free workspace lint;
- workspace tests;
- release build;
- focused Chat window/model tests;
- EguiPilotAI scenario against the real executable;
- full `cargo green` from this worktree's own target directory;
- review staged diff for unrelated files and production devtools leakage.

The repository pre-push gate remains enabled. GitHub is confirmation, not the
first place the branch learns whether it builds.
