# DevPULSE Chat — Steam-Chat AI/Social foundation

**Status:** ACTIVE IMPLEMENTATION PLAN — operator locked
**Date:** 2026-07-25
**Branch:** `codex/chat-steam-shell`
**Product authority:** [`chat-window-product-vision.md`](chat-window-product-vision.md)
**Architecture authority:** [`chat-architecture.md`](chat-architecture.md)

## 1. Outcome

This slice replaces the rejected permanent three-column Chat presentation with
the compact, independently useful DevPULSE Chat peer window.

The accepted primary composition is:

1. a compact top strip with two unmistakable browser-style `AI` and `Social`
   tabs;
2. a strong Steam Friends & Chat-style presence/session sidebar;
3. one dominant conversation and composer surface;
4. a governed overlay host for short-lived UI;
5. optional adjacent native utility windows for sustained previews.

The existing independent peer-window coordinator, launcher, provider-neutral
contracts, bounded histories, renderer registry, and EguiPilotAI lifecycle
evidence are retained. The current permanent work-stage column in
`dpl-chat-window` is rejected presentation evidence and has no compatibility
authority.

## 2. Locked product contract

- Main and Chat are equal native windows in one Rust application.
- Chat remains usable while Main is minimized.
- The Main launcher opens, restores, or focuses Chat; it never hosts a
  conversation.
- `AI` is selected initially. `Social` retains its own selection, draft,
  scroll, unread, and overlay state.
- `AI` and `Social` are persistent top-level mode tabs, not small toolbar
  buttons. They occupy a dedicated row in the Chat chrome, use browser-tab
  proportions and selected-tab continuity, and remain visible in both full
  and compact compositions.
- Social means global people plus project/team conversations.
- The first Social slice has complete typed models and deterministic authored
  fixtures, but no live transport.
- AI providers come from the registry. A provider can be ready, signed out,
  unavailable, experimental, degraded, running, or waiting; the UI never
  invents a successful login, send, stream, or Council action.
- Councils appear like persistent AI group conversations, but remain visibly
  unavailable until independently qualified providers exist.
- Short-lived hover cards, menus, approvals, notifications, and route peeks
  render through one governed overlay coordinator.
- Sustained artifact and route previews use native utility viewports that can
  snap, detach, re-dock, focus, and close without closing their session.
- AI-supplied layout, coordinates, colors, callbacks, shell handles, and
  unrestricted actions are impossible by construction.

## 3. Primary composition

The default Chat viewport is `1040 × 640` logical points with a
`720 × 560` minimum.

```text
+-----------------------------------------------------------------------+
| DevPULSE Chat | search                         | presence | window      |
| +------------------+ +------------------+                            |
| |       AI         | |    Social (5)    |      browser-style tabs    |
+-+------------------+-+------------------+----------------------------+
+------------------+----------------------------------------------------+
| presence/session | conversation                                       |
| sidebar          |                                                    |
|                  | normalized messages and compact tool results        |
| providers/people |                                                    |
| pinned/active/   +----------------------------------------------------+
| recent/projects  | route context | attach | components | model | send |
+------------------+----------------------------------------------------+
       overlays float above their origin; utilities are native peers
```

At widths below 840 points, the 252-point sidebar collapses to a 76-point
presence rail. Provider, person, session, and conversation selection then
opens as an overlay. No responsive composition may introduce a permanent
inspector or third content column.

### AI sidebar

- provider presence for ChatGPT/Codex, Claude, Gemini, and Grok;
- nested sessions grouped as `Pinned`, `Active`, and `Recent`;
- persistent Council entries with truthful qualification state;
- Interaction Library and Settings destinations;
- remembered provider/session selection and scroll state.

### Social sidebar

- global people and presence;
- direct messages and groups;
- projects with project/team conversations;
- unread and activity indicators;
- remembered conversation selection and scroll state.

### Conversation and composer

- conversation remains the dominant surface;
- normalized messages and compact inline interaction results;
- no giant placeholder cards and no empty permanent work stage;
- route context is visible but never implicitly attached or sent;
- drafts are isolated per domain and selected session/conversation;
- attachment, governed component, voice, model, send, and stop controls appear
  only when their capability and transport state are truthful.

## 4. Cross-domain models

### `dpl-chat-core`

Adds `ChatDomain::{Ai, Social}` as the shared stable domain identity.

### `dpl-chat-social`

Owns:

- typed person, project, conversation, and message identities;
- `SocialThreadKind::{Direct, Group, Project}`;
- `Presence::{Online, Away, InFlow, Offline}`;
- transport availability with a required unavailable reason;
- people, projects, conversations, messages, unread counts, and per-thread
  drafts;
- deterministic authored-fixture construction in test/devtools code only.

Production Social begins empty and unavailable. Sending remains disabled with
the exact transport-not-qualified reason.

### `dpl-chat-interactions`

Owns:

- typed interaction identity and provenance;
- capability requirement;
- lifecycle and deterministic ordering;
- `InteractionPlacement::{Inline, Overlay, SnappedUtility, DetachedUtility}`;
- initial content: notice, approval, route result, and artifact preview;
- validation and host-owned action vocabulary.

## 5. Overlay and utility lifecycle

One overlay coordinator owns ordering, exclusivity, origin, and focus return.
Blocking approvals trap focus until dismissed or completed, then restore focus
to the originating semantic control.

Provider hover cards expose only typed host actions:

- new session;
- add to Council;
- account;
- diagnostics;
- provider status details.

`ChatWindowRegistry` records:

- `WindowAttachment::{Snapped, Detached}`;
- snap edge;
- parent Chat window;
- persisted logical offset;
- binding and generation.

A snapped utility opens adjacent to Chat without a taskbar entry. Detaching
recreates it as a decorated taskbar window while preserving its binding and
generation lineage. Re-docking returns it to the recorded anchor. Main-window
minimize does not affect Chat or a utility. Closing a utility does not close
its session.

Chat owns utility declaration. The primary Chat viewport is deferred from
Main; utility viewports are immediate native children of Chat so their
input, replacement, and close lifecycles stay synchronized with the active
peer instead of depending on a sleeping Main viewport. A detach or re-dock
retires the old instrumented viewport lifetime only after its final input
generation commits, then declares the new generation from Chat.
The primary viewport also receives a generation-specific native identity on
reopen. Late callbacks from a retired OS window cannot mutate or close its
replacement, and a closed generation is never allowed to keep pumping the
shared automation runtime.

The initial utility renderer is an artifact/route preview with pin, detach,
re-dock, focus-origin, and close actions.

## 6. Authored fixture and proof

The devtools-only `chat-authored` fixture contains:

- provider presence and nested AI sessions;
- deterministic AI transcript and compact tool result;
- Social people, project/team conversations, messages, unread counts, and
  presence;
- observed route context;
- provider hover content;
- one governed approval;
- one snapped artifact/route preview.

Fixture providers and people never enter production registration.

The Windows proof harness waits for the real `DevPULSE Chat` native surface,
finds it through the existing capture engine, captures that native window,
and exits. A Main/root screenshot does not qualify the peer window.

EguiPilotAI and the native proof harness have deliberately separate jobs:

- EguiPilotAI drives and asserts the real native viewport tree, semantics,
  actionability, focus, committed input generations, and lifecycle. During a
  private devtools session, Main, Chat, and utility callbacks can all pump the
  one shared nonblocking runtime; production builds contain none of this path.
  Every pending action wakes its resolved target and owning parent with an
  immediate request plus a targeted one-millisecond timer, so a closing child
  cannot strand the next root- or sibling-window action.
- `chat-native-proof` runs out of process and captures the discoverable native
  OS surface. This is required because eframe 0.35 does not deliver queued
  screenshot requests for deferred viewports, and the Windows capture engine
  intentionally excludes windows owned by its own process.

The harness enters a DPI-aware process before enumerating or capturing
windows, waits for six identical native-geometry observations, and accepts one
of the deterministic authored states:

```text
cargo run -p devpulselabs --features eguipilotai-devtools \
  --bin chat-native-proof -- \
  --state approval \
  --out docs/proof/chat-steam-approval-native.png
```

Valid states are `approval`, `ai`, `social`, `hover`, `compact`, and
`snapped`. The snapped proof waits for both the real `DevPULSE Chat` native
window and its adjacent native utility, captures each surface independently,
and composes them at their actual OS positions. A utility-only image is not
valid snapped-layout evidence because it cannot prove the Chat chrome, tabs,
or adjacency. These switches only arrange the compile-time devtools fixture;
they cannot activate provider or Social transport.

Required proof states:

1. AI mode;
2. Social mode;
3. provider hover card;
4. blocking approval;
5. compact `720 × 560` composition;
6. snapped native preview.

## 7. Acceptance gates

- Unit tests prove domain isolation, selection and draft retention, Social
  identity, overlay ordering, focus restoration, interaction validation, and
  snap/detach/re-dock transitions.
- egui harnesses cover `720 × 560`, `1040 × 640`, and `1360 × 800` without a
  permanent third column or uncontrolled nested scrolling.
- EguiPilotAI proves domain switching, provider/session and Social navigation,
  hover, approval, route context, utility lifecycle, keyboard traversal, and
  stable semantic IDs.
- Native proof must show both browser-style `AI` and `Social` tabs in every
  Chat composition; snapped proof must include Chat and the adjacent utility
  in one image.
- The real executable proves Main minimized while Chat and a snapped utility
  remain interactive.
- Production Social send remains unavailable with its qualification reason.
- Provider send and activation remain capability-gated without placeholder
  success.
- Format, warning-free lint, focused tests, workspace tests, release build,
  real EguiPilotAI scenario, native-window proof, and the complete local gate
  pass before push.
- The branch is presented with local proof before PR creation or merge.

## 8. Explicitly later

- provider history/store binding beyond the deterministic vertical slice;
- complete revisioned bidirectional route bridge and context export;
- additional governed interaction and utility surface kinds;
- Claude and Gemini subscription activation;
- live conversation protocols for Codex, Claude, Gemini, and Grok (the
  independently gated ChatGPT/Codex and Grok browser-login slices do not
  qualify Send);
- live Social P2P/team transport, media sharing, and relay fallback;
- qualified Council orchestration;
- external Social bridges.

These are planned product milestones, not simulated successes in this slice.
