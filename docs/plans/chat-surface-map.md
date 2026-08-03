# The Chat surface map — what EXISTS today

Not a plan. An inventory, ground-truthed against the code on
`feat/p1-provider-availability` (2026-08-02) so that no later pass has to
guess what is already there — which is exactly the failure this document is
being written to stop.

**This is the Rust/egui app** (`crates/chat/*`, painted with egui in
`crates/chat/window`). It is not the Svelte platform. Every path below is in
this repo.

## 1. The crates, and what each one owns

| crate | loc | owns |
|---|---:|---|
| `chat/core` | 878 | provider-neutral identities + state contracts. NO egui, NO processes, NO IO |
| `chat/runtime` | 770 | the provider registry and provider-neutral chat state |
| `chat/window` | **6031** | the whole painted surface — every rail, pane and overlay |
| `chat/windows` | 624 | peer/utility window lifecycle (open, geometry, close policy) |
| `chat/store` | 939 | bounded read-only transcript discovery + paging |
| ~~`chat/renderers`~~ | 419 | **DEAD — zero consumers.** Deleted by X1 (`crate-consolidation.md`) |
| `chat/social` | 419 | Social-domain state — **no live transport** |
| `chat/context` | 153 | route-context offers and per-provider attachments |
| `chat/interactions` | 396 | governed interaction contracts (approvals, A2UI) |
| ~~`chat/extensions`~~ | 254 | **DEAD — zero consumers.** Deleted by X1 (`crate-consolidation.md`) |
| `chat/profiles` | 556 | operator-authored provider profiles |
| `chat/accounts` | 401 | activation evidence in local **Turso** |
| `chat/imagine` | 385 | the Imagine generation adapter (ADR-011) |
| `chat/browser-auth` | 387 | system-browser handoff |
| `chat/testkit` | 499 | deterministic fixtures; production never depends on it |
| providers | | `codex` 1405 · `grok` 3370 · `claude` 293 · `cursor` 53 · `antigravity` 54 · `gemini` 45 |

The shape to notice: **`window` is 6031 lines — bigger than every other chat
crate combined.** That is where the operator experience lives and where any
UI pass lands.

## 2. The surfaces the window paints

Every one of these is a real function in `chat/window/src/lib.rs`:

```
top_strip          the route-aware header + domain switcher
ai_sidebar         provider tabs + session groups + local history rows
content_sidebar    shows → episodes census
social_sidebar     conversations
ai_conversation    the transcript
content_board      the chapter → slide spine
social_conversation
history_transcript a past transcript, read-only
composer           the input + send/stop + model line
compact_picker_overlay
provider_catalog_overlay
provider_hover_overlay
generation_overlay
interaction_overlays
inline_interactions
```

**Domains:** `Ai` · `Content` · `Social` (`ChatDomain`, `chat/core:13`).
Content is its own domain deliberately — Chat is the point of operations FOR
the content, not a place content is filed under a provider.

## 3. What Chat can ask the platform for

The whole seam, as `HostIntent` — 24 variants:

**providers** `ActivateProvider` `OpenProviderAuthorization` `OpenProviderSetup`
`RefreshProvider` `ProviderSelected` `SaveProviderProfile`
`CopyProviderAuthorizationCode` `ResolveProviderPermission`
**sessions/turns** `NewSession` `SendTurn` `CancelTurn` `LoadHistoryPage`
**content** `LoadContentSpine` `RefreshContent`
**generation** `RequestImageGeneration` `SaveGenerationDraft`
`DiscardGenerationDraft` `ResolveImagineFlush`
**assist** `ToggleAssist` `ResolveAssistAcknowledgement`
**shell** `NavigateTo` `ShowMain` `QuitApplication` `OpenUtility`
**operator** `OperatorSlash`

**Not one of these mutates content.** Reading the spine is D1; editing it is
D3 and unbuilt.

## 4. The named, testable surfaces

51 pilot ids exist, so most of the surface is already addressable by a
scenario. The provider-facing ones:

```
chat.sidebar.providers      chat.sidebar.sessions      chat.sidebar.sessions.history
chat.mode.ai/content/social/provider
chat.provider.login         chat.provider.setup        chat.provider.refresh
chat.provider.catalog       chat.provider.configuration(.display-name/.endpoint/.model/.save)
chat.provider.authorization-browser/-code/-copy        chat.provider.activation-trace
chat.composer(.send/.stop/.voice/.media/.attachment/.provider-model)
chat.context.route          chat.assist.toggle         chat.utility.open
```

## 5. What is REAL vs what only looks real

| | state |
|---|---|
| peer window, geometry, close policy | **real** |
| provider registry, tabs, activation states | **real** |
| local transcripts as session rows (165 on this machine) | **real** (H1) |
| content census + chapter/slide spine, read-only | **real** (D1) |
| provider roster on the fabric → estate → seam | **real** (P1) |
| presence probes + vendor sign-in | **real** (A1/A2) |
| composer: Enter sends, Shift+Enter newline | **real** |
| **Social** | **no transport** — typed state only |
| **live turns** | qualified for NO provider yet; Claude is transcripts-only |
| **content editing from Chat** | not built (D3+) |
| **Council / bake-off** | not built — exists only in the operator's words and ADR-017 §6 |
| **provider carousel** | not built |

## 6. Where the operator's asks land on this map

| the ask | where it lives |
|---|---|
| rail stays; activating a provider gives a small clear label | `ai_sidebar` + `chat.mode.provider` |
| carousel of subscriptions, opened FROM the rail | new overlay beside `provider_catalog_overlay` |
| all chats in one system | `ai_sidebar` session groups — done |
| chat as operations console | `content_sidebar` + `content_board`, read-only today |
| route-aware control of the library | `chat.context.route` exists; the verbs do not |
| Council / mixture-of-experts | nothing yet; needs `HostIntent` variants |
| moka / Dragonfly | `chat/store` paging + `chat/accounts` (Turso today) |
| DuckDB | not present in the chat tree at all |

## 7. How to SEE this window — the capture recipe

**The pilot harness cannot screenshot the chat.** `chat.primary` is a deferred
viewport and `eframe 0.35` never delivers a queued screenshot request for one;
the scenario fails with exactly that message. This is a sibling of the
`MotionPolicy::Suppressed` limitation and it affects every peer window.

So the frame comes off X11 directly:

```bash
Xvfb :99 -screen 0 1800x1100x24 &
export DISPLAY=:99 WGPU_BACKEND=vulkan LIBGL_ALWAYS_SOFTWARE=true \
       VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.json
cargo build -p devpulselabs --features eguipilotai-devtools
./target/debug/devpulselabs --chat-open --mood 0 &      # +--chat-authored for interactions
sleep 30                                                # presence sweep + first paint
ffmpeg -f x11grab -video_size 1800x1100 -i :99 -frames:v 1 -y shot.png
```

`tests/eguipilotai/chat-surface-capture.toml` drives the same states through
the pilot and is the place to add steps; it just cannot produce the pixels.

### 7.1 What the 2026-08-03 capture shows

Six providers registered — Codex and Claude *Experimental*, Antigravity,
Cursor, Gemini and Grok *Blocked*. And three facts no reading of the source
made obvious:

- **`More providers…` is clipped mid-word** by the scroll boundary. It is the
  last child of the scroll area, and six rows already overflow it.
- **`PINNED`, `ACTIVE` and `RECENT CHATS` do not appear at all.**
  `session_groups()` runs after `provider_roster()` and there is no height
  left for it.
- **The `🧠 Providers` control is in the SHELL STATUS BAR**, bottom-right of
  the main window — not in the chat rail. That is V1/V2/V3 as shipped.

Colours sampled off that frame, for anything reproducing the surface:
rail `#071115` · conversation `#050c0f` · composer `#0a0c12` ·
selected row `#0b3034` on `#14696c` · chevron `#334f58` ·
cyan `#01b7cf` · amber `#f6a91b` · lime `#97d030` · violet `#bba1ff` ·
magenta `#ff7ce4`. The exe renders with egui's default `Ubuntu-Light`.
