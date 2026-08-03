# Chat as the operations console — the program map

> "BOTH CHAT READS AND HAS FULL ACCESS TO THE PLATFORM THE WHOLE POINT IS
> THAT CHAT BECOMES A CLEAR POINT OF OPERATIONS TO WORK ON THE CONTENT
> WHETHER ITS RAW OR BUILDING THE OFFICIAL SLIDES, AND
> PRESENTATION/EPISODE" — the operator, 2026-07-30

This is the program map, not a pass plan. It fixes the DEPENDENCY CHAIN
and the laws every link obeys. Each link gets its own 7-section plan
authored immediately before it is built, ground-truthed against the code
as it stands at that moment — never from this map, and never from memory.

## 1. Where Chat actually stands (ground-truthed 2026-07-30, `56e5ec7`)

**Reads:** the `Route` enum naming where the main window is, plus its own
provider/session state. Nothing else. The rail's own words —
"Observed from Main · context not attached" — are literal.

**Acts:** twenty-odd `HostIntent` variants covering provider lifecycle,
sessions, turns, image generation, assist, `NavigateTo(Route)`,
`ShowMain`, `QuitApplication`. **Not one touches content.**

**The decisive fact:** the chat crate tree declares ZERO dependency on
any content crate — not `dpl-doc`, not `dpl-comp`, not `dpl-workflows`,
not the index. Chat cannot see a slide, an episode, or a media file.
Today it is a provider console that happens to know your route.

**What already exists and is real:** `dpl-workflows` ships
`mint_storyboard`, `slide_for_slot`, `comp_for_board`, `validate`,
`load_board`/`save_board`, and stage reports — wired into `dpl-composer`
alone. `dpl-doc` ships `Episode`/`Chapter`/`SlideRef`, `load_episode`,
`save_episode` (crash-safe tmp/bak), `episode_slides` (the flat ordered
walk the deck consumes), `list_shows`, and `EpisodeEntry`. The machinery
the operator wants driven is BUILT. This program is a seam problem.

## 2. The seam that already exists (and the one we extend)

```
  HOST (crates/app/src/chat.rs)                 CHAT WINDOW (surface)
        │                                              │
        │  push truth IN                               │
        │    window.observe_route(route)   ──────────► │   render
        │    on ShellEvent::RouteChanged               │
        │                                              │
        │  ◄──────────  HostIntent  ─────────────────  │  operator acts
        │    apply_host_intent(ctx, shared, intent)    │
```

Content rides this exact shape: an `observe_*` push in, typed
`HostIntent`s out. No new mechanism is invented.

## 3. The laws this program obeys

1. **The seam carries PLAIN DATA, never foreign types.**
   `dpl-chat-core`'s doc-header is a promise: *"deliberately knows nothing
   about egui, provider processes, filesystem discovery, or transport
   protocols."* Content VIEW types therefore live in `dpl-chat-core` as
   owned plain data, and the HOST translates `dpl_doc` truth into them.
   The chat side stays content-SOURCE-neutral exactly as it is already
   provider-neutral. This costs no new crate, no new dep ceiling, and no
   layer risk — and it is why `dpl-chat-window` must NOT grow a `dpl-doc`
   dependency.
2. **Surfaces never do IO.** Disk reads stay in the host/engine layer.
   The window renders what it is handed.
3. **Operator-parity.** Every capability lands as an OPERATOR action
   first; the AI drive tier (D6) only ever sends what a human could have
   clicked. Nothing becomes AI-only.
4. **`/content` is sacred.** Never committed, never authored into, never
   deleted; sources are never mutated. Every mutating link states its
   write set explicitly and writes through `dpl-doc`'s crash-safe
   tmp/bak path, never ad-hoc file writes.
5. **Two libraries stay two.** Workbench library = in-app ColdLight
   ingredients. WiredFRONT library = RAW MEDIA. A link that touches one
   never blurs it into the other.
6. **Experience-first.** Each link is proved on the real exe at a real
   window size, and a link that makes Chat feel worse is a regression
   regardless of what it enables.

## 3b. The finding that reorders this program (2026-07-30)

The operator, plainly: *"i am building a gosh darn private tool where I am
allowed to have it become convenient to have all the chats available in
one system"*. That is legitimate and is the point of the product. It is
also **already 90% built and not wired to the screen**:

- `ClaudeHistoryDecoder` (`crates/chat/providers/claude/src/lib.rs:55`)
  and `CodexHistoryDecoder` are complete, working `.jsonl` decoders.
- `local_chat_history_catalog()` (`crates/app/src/main.rs:1162`)
  registers both against `~/.claude/projects` and `~/.codex/sessions`.
- `ChatWindow::new` (`crates/chat/window/src/lib.rs:548`) indexes that
  catalog into `self.histories` — real sessions, decoded titles.

**Measured on the operator's machine 2026-07-30: 128 Claude transcripts
across 15 projects + 37 Codex transcripts = 165 real chats on disk.**
A live-runtime EguiPilot snapshot (`chat-local-histories`) found
**ZERO** session nodes in the rail.

Cause: `self.histories` is consumed in exactly ONE place — the
empty-state branch of `session_groups` (`:2028`) — where it becomes a
COUNT inside an unclickable label. Every session ROW is minted from
`provider.registration.sessions`, the LIVE runtime registry, which is
empty for history-backed providers. The chats are found, decoded,
indexed, and then shown as a number.

This is not a missing integration or a policy limit. It is a rail wired
to only one of its two sources. **H1 is therefore sequenced NEXT, ahead
of D2** — it is what the operator has asked for repeatedly, and it is
nearer to done than anything else in this map.

## 4. The dependency chain

Each link is a separate pass with its own plan, gate, EguiPilot proof,
journal, and double-green PR. **No link starts before its predecessor is
merged** — the whole point of the order is that each one's model is
proved by use before anything is built on top of it.

| link | scope | why it must come first | mutates? |
|---|---|---|---|
| **D1** | **The content seam + the episode spine, READ-ONLY.** A plain-data content view in `dpl-chat-core`; `observe_content` push from the host; the app translating `dpl_doc` truth (shows → episodes → chapters → slides) into it; the AI rail showing the real spine. | Every later link needs content crossing the seam. Read-only forces the model to be RIGHT before any verb depends on its shape — a wrong model discovered in D3 would cost every link above it. | no |
| **H1** | **The chats in one system.** Local history sessions become first-class ROWS in the rail — grouped by provider, titled from the decoder's `title`/`workspace`/`started_at`, selectable, and readable in the conversation pane. | The operator's actual, repeatedly-stated ask. The decoders, the catalog registration, and the index already exist (§3b) — this wires the rail to the source it already loads, plus a reader for the transcript body. Sequenced ahead of D2 because it is both the highest-value and the nearest-to-done link in this map. | no (read-only over existing transcripts) |
| **P1** | **Activated providers become platform capabilities.** A fabric-carried provider-availability signal, plus the FIRST real platform consumer of it, landed together. | The operator, 2026-07-30: *"chat needs to be built so its properly yes its own window but the providers that get activated in the chat become available in the platform."* Today `ChatRuntime` is a private field of `ChatWindow` — a SURFACE — and the coordinator exposes nothing but `frame`/`attach_*`; a workspace grep for `ActivationState`/`Capability::` outside `crates/chat/` returns only the chat host itself. So the platform cannot see a provider at all. Sequenced after H1 because H1 makes activation TRUE (local capability counts) and P1 makes it VISIBLE — and because a publication with no consumer proves nothing, so the consumer ships in the same pass. | no |
| **D2** | **Content-addressed navigation.** "Open this slide / this episode in Main" — a `HostIntent` carrying a content address, reusing `NavigateTo`'s proven host path. | Proves the intent round-trip on content while still touching nothing on disk. The first content-addressed intent should be the one that cannot corrupt anything. | no |
| **D3** | **The episode spine becomes editable from Chat.** Add / reorder / retitle at the chapter+slide level, written through `dpl_doc::save_episode`. | The container must be authorable before what fills it. This is the first MUTATION, so it lands where the write set is smallest and entirely inside `episode.json`. | **yes** |
| **D4** | **Raw media into slides.** The WiredFRONT raw library readable in Chat, and a media reference placeable into a slide's blocks. | Needs D1's model and D3's proven write path. Placing a REFERENCE never moves or mutates the source — the sources-are-never-mutated law is what makes this safe. | **yes** |
| **D5** | **Workflow stages driven from Chat.** Drive the `dpl-workflows` engine that already exists: mint a storyboard, fill slots, validate, produce slides. | The engine's output IS slides and comps — it must land on a spine that Chat can already read (D1) and author (D3), or its results have nowhere honest to go. | **yes** |
| **D6** | **The AI drive tier.** The provider drives D2–D5 through the fabric, with signals and A2UI. | Operator-parity: nothing is AI-drivable until it is operator-doable. D6 sends exactly the intents D2–D5 already expose — it adds a driver, never a capability. | via D2–D5 |

## 5. What this map does NOT decide

Per-link design (view shapes, intent variants, rail composition, pilot
ids) is decided in that link's own plan, against the code as it stands
then. This map fixes ORDER and LAWS only. If a link's ground truth
contradicts this map, the map is amended in the same commit — the map is
never the authority the code is.

## 6. Status

- **D1** — plan `docs/plans/chat-content-seam.md`. MERGED (#180).
- **H1** — plan `docs/plans/chat-histories-in-one-system.md`. MERGED (#180).
- **P1** — plan `docs/plans/chat-providers-become-platform.md`, governed by
  `docs/design/adr-017-provider-capability-registry.md`. Built and green on
  `feat/p1-provider-availability`.
- The surface P1 feeds is V1–V3 of
  `docs/plans/providers-presence-and-the-brain.md`, against
  `docs/design/mocks/providers.html`.
- D2–D6 — not planned yet, by design.
