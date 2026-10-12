# CHAT + AI — the plan (chat under chat, providers and subscriptions under AI)

**Status:** **APPROVED 2026-10-11** (operator: *"execute"*). The §4.6
decisions are adopted as recommended:
1. the AI layout, with existing crates moved;
2. the order D0 → K1 → K2 → A1 → A2 → C1 → C2 → …;
3. Claude Code as Anthropic's native binary;
4. WezTerm installed portable into the depot;
5. vendor versions taken from local installs first, with a per-vendor OK
   for downloads;
6. Codex → Claude Code → Grok → Gemini.

Rewritten 2026-10-11 in THE checkout (`D:\projects\devpulselabs`). It
supersedes every earlier version of this file and the `wip/c1-paused`
amendments.

**What this plan obeys** (the operator's own law, read in full 2026-10-10):
- `docs/design/architecture.md`: the layers, ceilings, §4 laws and §5
  new-crate checklist.
- `docs/design/design-system.md`: the §0.6 naming law and the §7 ordering
  law.
- `docs/design/doctrine.md`: tooltip law, generated tokens only, any-shape,
  no blur, PilotClock.
- `docs/eguipilotai-devpulse-authoring.md`.
- ADR-009: external capability is a subprocess.

## 1. Context — the rulings

Operator, 2026-10-10/11, verbatim:

- *"it should properly build chat from scratch inside the fucking app like i
  asked from the start … properly look at my requirements for it and get it
  actually planned and implemented."*
- *"no fucking pullimng my fucking web that are not even intended for tyhis"*
- *"No I want chat to have its own ui colocated with its feature"* · *"you are
  such an asshole for not extending my existing boundaries"*
- *"i want chat properly authored under chat yes but the fucking
  subscriptions, the fucking providers get captured and authored under AI and
  their respected api need to be easily updated per update so the
  subscriptions always work realistically and again this is allowed so if we
  need to drive their cli in the background or have it so we wrap claude
  desktop, grok, codex, and even have their terminal also available via our
  tty wezterm, and so forth"*
- *"FULLY UI DRIVEN"* · *"RESPECTM Y FUCKING PRACTICES AND STRUCTURE"* · *"If
  we need to install or patch we are in pre-release do not reward hack"*

**The shape this fixes:**

| Boundary | Owns | Does not own |
|---|---|---|
| **CHAT** — `crates/chat/*`, extended | Chat's **own UI, colocated with the feature** (the window and its controls, rows, messages, composer); conversations; the one list of all chats; route context; the status-bar launcher hookup | Any provider, subscription, sign-in, CLI, or vendor protocol |
| **AI** — `crates/ai/*` | **Every provider and every subscription:** presence, sign-in, accounts, activation, the vendor adapters (background CLI, desktop-app wrap, terminal), the supervisor, **per-version updates**, usage. **AI powers the whole platform**, not just chat. | Any chat UI |

## 2. Goal / non-goals

**Goal.**
- **Chat:** a native chat window, opened from the status bar and driven
  entirely through its UI, in which the operator connects their
  subscriptions, converses (Codex and Claude Code first), and sees every
  chat in one list.
- **Providers:** they keep working when vendors ship updates. Each one is
  reachable three ways: driven in the background, attached through its
  desktop app, or opened as its own terminal UI in our WezTerm.
- **Portable, real, proven:** everything is portable (depot beside the exe)
  and proven on the real exe.

**Non-goals:**
- No web UI and no HTML mock.
- No chatmaton, automaton or Shippin.
- No new boundary beyond the AI one the operator asked for.
- No social transport (*"social to extend too later"*).
- No credentials held by DevPULSE: auth stays in each vendor's own
  app/CLI profile.

## 3. Ground truth (read or probed in-session, 2026-10-10/11)

### 3.1 What exists in `crates/chat/*` today, and where it belongs

| Crate today | What it is (audit) | Belongs to |
|---|---|---|
| `chat/window` (6,959 lines) | The whole chat window. **0** ColdLight role/style calls, 32 raw `Color32`, 22 bespoke frames, ~30 stock buttons, ~40 controls without `tip()` | **CHAT**: rebuilt in place, with its own UI components colocated in it |
| `chat/windows` | Peer-window lifecycle registry (no egui) | **CHAT** |
| `chat/renderers` | Event-renderer registry, unused, paints with a deleted-to-be frame | **CHAT**: rebuilt on ColdLight, wired into the window |
| `chat/interactions` | Overlay/interaction rules | **CHAT** |
| `chat/context` | Route context offered to a turn (half-used) | **CHAT** |
| `chat/social` | Social model, no transport | **CHAT** (parked: later) |
| `chat/store` | Read-only vendor-history discovery + Codex/Claude decoders + usage dialects | **Split.** Vendor decoders and usage → **AI** (each provider knows its own format). Chat's own conversation store + the one-list index stay **CHAT**. |
| `chat/core` | Mixed: provider identity, capabilities, activation/presence, token usage, **and** chat events + slash commands | **Split.** Provider/activation/usage vocabulary → **AI core**; chat events/commands stay **CHAT core** |
| `chat/runtime` | Provider registry + `ProviderRuntime` seam + in-memory chat state | **Split.** Registry + seam → **AI**; chat state → **CHAT** |
| `chat/accounts` | Activation-evidence ledger (Turso) | **AI** |
| `chat/profiles` | Operator-authored provider profiles (Turso) | **AI** |
| `chat/browser-auth` | Opens the system browser for sign-in | **AI** |
| `chat/imagine` | xAI REST image/brainstorm client | **AI** (Grok's media capability) |
| `chat/providers/{codex,claude,grok,gemini,cursor}` | The provider individuals | **AI**: `cursor` has no mandate and is retired (R33) |
| `chat/extensions` | Dead (no dependents) | Retired |
| `chat/testkit` | Fixtures | Split with the crates it serves |
| `app/src/presence.rs`, `ai_usage.rs`, `ai_backfill.rs` | Presence probes (PATH!), usage accounting, history backfill | **AI** (the app keeps wiring only) |
| `shell/src/providers.rs` | The Providers control UI in the status bar | Stays in the shell (UI that **consumes** AI) |

### 3.2 Vendors on this laptop (probed 2026-10-11)

| Vendor | Desktop app | CLI | Notes |
|---|---|---|---|
| OpenAI Codex | **Store app `OpenAI.Codex` 26.1007.2314.0** (+ `CodexBeta` 26.727). It runs its own **`codex app-server` daemon 0.162.1** (`~/.codex/packages/app-server-daemon/…`, `--listen unix://`). | Depot `0.145.0-alpha.18`; system `0.160.1` (`AppData\Local\Programs\OpenAI\Codex\bin`) | The depot copy only passes today's stale gate (`>=0.145,<0.146`). |
| Anthropic Claude | **Store app `Claude` 2.31226.1.0** | **The `claude` on PATH is NOT Anthropic's Claude Code.** It is an npm package that errors *"ANTHROPIC_API_KEY not set in .env file"*. Today's presence probe runs exactly that. | The official Claude Code CLI must be sourced into the depot. |
| xAI Grok | **"Grok Bot" 0.63.0** (publisher SpaceXAI), `C:\Program Files\Grok Bot\Grok Bot.exe` | Depot Grok Build `0.2.112`; `~/.grok` exists | |
| Google Gemini | — | Depot `0.54.4` is a **Node bundle** | Needs a Node runtime in the depot. |
| WezTerm (our TTY) | **Not installed** | — | Must be **installed** (portable) into the depot (R36). |

**Other facts:**
- **Codex:** server-initiated JSON-RPC requests (approvals) are never
  answered (`codex/src/runtime.rs:263-267`), so a turn can hang. There is no
  turn deadline.
- **Presence and sign-in** run bare names from PATH with the full
  environment (`app/src/presence.rs:47,165`), while the runtime uses the
  depot.
- **Conversations are never persisted.**

### 3.3 The design system (the operator's law, read 2026-10-10)

- ColdLight is the token, style and paint layer: `Intent`
  {Chrome, Hero, Quiet, Danger} → `binding(surface, …)`, `ramp`, `space`,
  `Fluid`, `draw_icon`, `tip`, motion + PilotClock.
- `roles.json` has **no `chat` surface**.
- The icon registry lacks send, attach, mic, pin, user, plus, check,
  minimize, maximize and settings. No caret or busy motion exists.
- `design-system.md` §7 ordering law: step **3′** (`Tone→Accent`,
  `Treated→Styled`, delete the trait path and bespoke frames, split
  `coldlight/src/lib.rs`) *"goes first — every later step writes call
  sites, and writing them twice is the drift this revision exists to
  stop."*

## 4. Design

### 4.1 Requirements ledger. Every requirement is the operator's words; nothing else is built.

**R1–R28** (each quote re-read at its cited line, 2026-10-10)

| R | Requirement | Source |
|---|---|---|
| R1 | Layman-friendly, frictionless, touch-first (75" touchscreen) | `authoring-architecture.md:142-148` |
| R2 | A **window**, from the **status bar**, Steam-like, **portable lite**, **loads routes** | `chat-consolidation.md:8-10` |
| R3 | Route-aware; manage/control the editor; add library elements | `chat-creation-and-workflows-vision.md:13-16` |
| R4 | Grok mode button: create image/video, preview drafts, 30-day flush with consent | same, l.35-40 |
| R5 | Assist = one clear, simple acknowledgement for all providers | `chat-assist-mode.md:3-5` |
| R6 | Guided explainer workflow from chat | `explainer-guided-workflow.md:5-11` |
| R7 | One-click provider selection; sign in inside chat; once connected, **its own tab** with sessions | `chat-provider-easy-select.md:7-9`, `chat-connected-provider-tabs.md:9-10` |
| R8 | **Full access to the platform**, as the point of operations | `chat-operations-console.md:3-6` |
| R9 | **All chats in one system** | `chat-histories-in-one-system.md:10-11` |
| R10 | Providers activated in chat are **available to the whole platform** | `chat-providers-become-platform.md:21-23` |
| R11 | Each provider built **as an individual** | `phase3-analytics-program.md:219-222` |
| R12 | **Piggyback** existing active sessions | same, l.278-279 |
| R13 | Presence awareness at launch; sign-in in the **laptop's own browser** | `providers-presence-and-the-brain.md:12-19` |
| R14 | The known frontier AIs become space for our sessions | same, l.21-22 |
| R15 | The Providers control: ink-bleed hover, thumbnails, provider modal | same, l.24-34 |
| R16 | **Clyffy**: face of the AI; not in group chat; own session, powered by any provider | same, l.239-241; `chat-boundary.md:5-13` |
| R17 | A **secondary cockpit**: load routes, **popup windows**, **task windows** that minimize and restore | `providers-presence-and-the-brain.md:281-285` |
| R18 | **Chat owns its store** | `adr-017-provider-capability-registry.md:156` |
| R19 | **Chat separated from AI**; AI powers the platform; social later | `ai-chat-social-separation.md:10-12`; `chat-boundary.md` |
| R20 | **Commands work, Enter sends**: `/route`, `/model`, `/routines`, with guided values | `chat-boundary.md:5-13` |
| R21 | If ChatGPT is active it **shows active and I can chat** | `codex-conversation-transport.md:10-11,230` |
| R22 | Bleeding-edge transport is fine | same, l.141-142 |
| R23 | Brainstorm: editable visual components; sessions spawn; counterpoints | `brainstorm-and-clyffy-drive-capture.md:9-17` |
| R24 | Clyffy may drive the app, behind manual-approval gates | same, l.19-25 |
| R25 | A true **council** of grok/chatgpt/gemini/claude | `product-vision.md:24-37` |
| R26 | Mocks first | `product-vision.md:307-308` |
| R27 | Work in passes; **new boundaries go to the operator** | `providers-presence-and-the-brain.md:377-381` |
| R28 | Portable; external tools are subprocesses | ADR-009 + 2026-10-10 |

**ORIGIN, 2026-07-21** (old app's `docs/plans/chat-agent-console-plan.md:15-25`,
`fabric-plan.md:19-21`; re-read at the lines)

| R | Requirement | Operator, verbatim |
|---|---|---|
| R29 | Chat is the **guided UI over the agent CLIs running in the background** | *"have claude code run in the background of chat where chat is just the guided UI experience for me"* |
| R30 | Subscriptions through the vendor CLI | *"we can literally trigger the cli for subscription"* |
| R31 | Click the provider's tab, see its sessions in the **extended sidebar** | *"once activated I can simply click on the tab and see the session in the extended sidebar"* |
| R32 | Built to evolve; **additional windows** | *"…have additional windows generated"* |
| R33 | **Claude Code and Codex first**, each working well; Council after | *"council comes after we get claude code and codex individually working well"* |
| R34 | Tabs for grok, chatGPT(codex), claude, gemini; route awareness | *"…It will have tabs … for grok, chatGPT(codex), claude, gemini…"* |
| R35 | It must **feel optimized**: memory and load | *"does not feel optimized — the memory, the way things load."* |

**2026-10-10/11**

| R | Requirement | Operator, verbatim |
|---|---|---|
| R36 | **No reward hacking**: install or patch for real | *"If we need to install or patch we are in pre-release do not reward hack"* |
| R37 | **FULLY UI DRIVEN** | *"FULLY UI DRIVEN"* |
| R38 | **Chat's own UI colocated with the chat feature**; extend existing boundaries; no web | *"No I want chat to have its own ui colocated with its feature"*, *"…not extending my existing boundaries"* |
| R39 | **Providers and subscriptions captured and authored under AI** | *"the subscriptions, the fucking providers get captured and authored under AI"* |
| R40 | **Each provider's API easily updated per vendor update**, so subscriptions keep working | *"their respected api need to be easily updated per update so the subscriptions always work realistically"* |
| R41 | **Three ways in, all allowed:** drive the CLI in the background · wrap the desktop app (Claude Desktop, Grok, Codex) · the vendor's terminal in **our TTY, WezTerm** | *"drive their cli in the background or have it so we wrap claude desktop, grok, codex, and even have their terminal also available via our tty wezterm"* |

**R37, made concrete:**
- Every operator action is a visible, tooltipped control:
  - connect, sign in or out, update a provider;
  - choose its mode (background / desktop / terminal), project folder and
    model;
  - Assist, approve or decline;
  - open, rename and find chats;
  - load routes, minimize and restore task windows.
- Nothing requires a config file, env var, CLI flag or terminal command.
- Harness flags are for drills only.
- A capability without a working control is hidden.

### 4.2 CHAT — its own UI, colocated in its feature (R38)

Chat's UI components live **inside chat's crates, next to the feature that
uses them**. They are not moved to a shared library and not invented as a
new crate:

```
crates/chat/window/src/
  lib.rs            ChatWindow — composes the screen from the modules below
  launcher.rs       status-bar launcher (the WindowAccessory hookup)
  tabs.rs           provider tabs (R7, R31, R34) — presence dot, unread badge
  sidebar.rs        the extended sidebar: that provider's sessions, general + per project (R31, R9)
  conversation.rs   message list, streaming text + caret, tool/result rows
  composer.rs       text field, Enter sends (R20), chips (Assist, Attach, Media, model), route chip (R3)
  commands.rs       /route /model /routines typeahead with guided values (R20)
  approvals.rs      agent requests: Approve / Decline, deadline (R24, R5)
  status.rs         plain-words provider states + what to do (R13, R21)
  tasks.rs          task-window strip, minimize/restore (R17, R32)
crates/chat/renderers/   event rows (message / tool / result / quota) — rebuilt, wired in
crates/chat/windows/     peer + utility window lifecycle (unchanged boundary)
crates/chat/store/       chat's OWN conversation store + the one list of all chats (R18, R9)
```

**Paint law (the operator's design system):**
- Every component paints through ColdLight: `Intent` →
  `binding("chat", …)` → `Styled`/`dressed`, `ramp`, `space`/`Fluid`,
  `draw_icon`, `tip`.
- Zero raw `Color32`, zero bespoke `Frame::new`, zero stock `egui::Button`,
  zero Unicode-glyph icons, zero magic px.
- Every control carries `tip()` and a `PilotId`.

**Token data extends ColdLight's own files**, its existing boundary:
- a `chat` surface in `roles.json`;
- the missing icons, from the vendored Lucide pool (ADR-015);
- `caret-blink` and `busy` effects in `motion.json`, registered with
  PilotClock.

**Chat consumes AI and nothing else of it:**
- it asks AI for providers, their state and their sessions;
- it sends turns through AI;
- it stores its own conversations.

### 4.3 AI — providers and subscriptions (R39, R40, R41)

```
crates/ai/
  core/        dpl-ai-core      provider identity · subscription/activation model · capabilities
                                · the ONE command/event contract · usage — pure, no IO
  host/        dpl-ai-host      the supervisor: start on first use, health, restart w/ backoff,
                                idle stop, no orphans (Job Object on Windows), every server
                                request answered, turn deadline; presence + sign-in through the
                                depot only; publishes activation on the fabric (R10) so the
                                WHOLE platform can use a connected provider
  accounts/    dpl-ai-accounts  (moved from chat/accounts)   activation evidence
  profiles/    dpl-ai-profiles  (moved from chat/profiles)   operator-authored profiles
  browser-auth/ dpl-ai-browser-auth (moved from chat/browser-auth) — sign-in in the laptop's browser (R13)
  imagine/     dpl-ai-imagine   (moved from chat/imagine)    xAI media (R4)
  providers/
    codex/ claude/ grok/ gemini/   dpl-ai-<provider> — one individual each (R11)
```

**Each provider crate holds up to three adapters (R41)**, behind the one
contract in `ai/core`:

| Mode | What it is | Per vendor (probed first, honest about limits) |
|---|---|---|
| **CLI (background)** | The vendor CLI as a supervised subprocess from the depot, driven over its protocol | Codex `app-server` JSON-RPC · Claude Code `stream-json` · Grok `agent stdio` (ACP) · Gemini headless (multi-turn to be probed) |
| **Desktop wrap** | Attach to, or launch and focus, the vendor's installed desktop app; reuse its signed-in session (R12 piggyback) | Codex desktop: its local **`app-server` daemon** is running; attaching to it is the candidate. Claude Desktop and Grok Bot: **probe first** what can be attached or driven. Only proven capabilities are shown (R36). |
| **Terminal (TTY)** | The vendor's own terminal UI in **our WezTerm**, portable, from the depot, a subprocess with a DevPULSE-owned config | `codex` · `claude` · `grok` interactive TUIs. Opened from chat's UI ("Open in terminal"), never by typing a command (R37). |

**Updates that keep subscriptions working (R40)**, the per-provider update
path:
1. **Compatibility is proven, not hard-coded.** Each adapter declares the
   protocol surface it uses (methods, notifications, server requests). On
   staging a vendor version, a **compatibility probe** runs against that
   exact binary: handshake, schema regeneration (e.g. `codex app-server
   generate-json-schema`), and a diff against the adapter's surface. The
   result is written into the depot manifest beside the payload. This
   replaces stale constants like `>=0.145,<0.146`.
2. **One "Update" control per provider**, in AI's provider panel and reached
   from chat (R37). It shows the depot version, the newer versions found
   (on this machine first: the vendor's own installed copy, e.g. Codex
   desktop's newer CLI; an official vendor download only with the
   operator's per-vendor OK, recorded in the external-reach inventory). It
   stages the new version, runs the probe, and **switches only if the probe
   and a real turn pass**. Otherwise it **keeps the last working version**
   and says in plain words what changed.
3. **Schema-pin tests** are regenerated from the staged binary in the same
   PR as any adapter change.

### 4.4 Passes (each: plan doc, 7 sections + §8; PR; double-green; real-exe proof; exe rebuilt; STATE)

| Pass | Scope | Done means |
|---|---|---|
| **P0** | This plan approved (with §4.6) | Operator approval |
| **D0** | The laptop's deps (`d0-laptop-deps.md`, approved): startup check, honest media, camera, portable release | As that plan states |
| **K1 = design-system 3′** | The operator's ordering law: the naming merge + `coldlight` module split, pixel-identical | `cargo green`; before and after shots at 1100×700 and maximized |
| **K2** | ColdLight token extensions for chat (`roles.json` chat surface, icons, caret/busy motion) | Generated; census tests; Library walls show them |
| **A1 = the AI boundary** | Create `crates/ai/*`. Move accounts, profiles, browser-auth, imagine and providers/* under it. Split `chat/core`, `chat/runtime` and `chat/store` along the §3.1 lines. Move presence, usage and backfill out of the app. **No shims; behaviour unchanged.** `architecture.md` + the xtask audit are amended in the same commit. | `cargo green`; existing chat drills unchanged; real-exe shot; the structure audit shows chat ↛ provider internals |
| **A2 = host + Codex done right** | `dpl-ai-host` supervisor; Codex CLI mode: depot-only presence and sign-in, server requests answered, turn deadline, real compatibility probe | **A real turn on this laptop** with the operator's login; orphan check clean |
| **A3 = the update path** | Probe + manifest + "Update" control; Codex updated from the depot's 0.145-alpha to a current version through it | Update proven end to end; refusal proven against a deliberately incompatible build kept in a scratch depot |
| **A4 = Claude Code** | The official Claude Code CLI sourced into the depot (not the PATH impostor); `stream-json` adapter; presence and sign-in through the depot | Real Claude turn |
| **A5 = Terminal mode** | Portable WezTerm **installed** into the depot (new tool id, `terminal` family); "Open in terminal" for Codex, Claude and Grok | Launches the depot binary with our config; closes clean |
| **A6 = Desktop wrap** | Probe each desktop app (Codex daemon attach, Claude Desktop, Grok Bot); build only what the probe proves | Each proven capability demonstrated; unproven ones stated |
| **C1 = chat store** | Conversations persisted under `.userdata/chat/` (document law: `v`, refuse newer, tmp → bak → rename); the one list across DevPULSE + vendor history (through AI's decoders) | Quit and relaunch; history is there; vendor chats in the same list |
| **C2 = chat UI** | `chat/window` rebuilt as in §4.2, on ColdLight, colocated. **This real egui window is the mock the operator red-pens (R26, R38).** | Shots at 1100×700 and maximized; a pilot scenario drives every control; a grep proves zero raw colour/frames in `crates/chat/**` |
| **C3** | Commands with guided values; popup + task windows (R17, R20, R32) | Pilot scenario per command; minimize/restore proof |
| **C4** | Platform reach: AI available platform-wide; Assist; content actions (R5, R8, R10) | A non-chat surface runs a turn; round-trip proof |
| **A7 / A8** | Grok onto the host (retire its separate path); Gemini with Node **installed** into the depot | Real turns |
| **C5 / C6 / later** | Grok media (R4); Clyffy (R16); Council (R25), brainstorm (R23), Clyffy drive (R24) | Each gets its own plan |

### 4.5 External reaches this plan adds (to `external-reach-inventory.md`)

- **WezTerm portable**: installed into the depot.
- **Codex desktop's local app-server daemon**: attached to.
- **The Claude Desktop and Grok Bot apps**: wrapped.
- **The official Claude Code CLI**: sourced into the depot.
- **A Node runtime** for Gemini.
- **Vendor download sources**: only with per-vendor operator OK.

Each is listed before it is relied on.

### 4.6 Decisions for the operator (R27)

| # | Decision | Recommendation |
|---|---|---|
| 1 | Crate layout under AI as in §4.3 (`dpl-ai-*`, the existing chat crates moved, not rewritten) | **Yes** |
| 2 | Order: D0 → K1 (your §7 law) → K2 → A1 → A2 → C1 → C2 → … | **Yes**. Chat UI (C2) waits for a real Codex turn (A2), so the UI is built against something that works. |
| 3 | Official Claude Code CLI into the depot: Anthropic's native binary, or the npm package (which needs Node) | The **native binary**, so no Node dependency for Claude |
| 4 | WezTerm as our TTY: portable build installed into the depot | **Yes** |
| 5 | Where new vendor versions come from | This machine's installed copies first; official downloads per vendor with your OK |
| 6 | Provider order | Codex → Claude Code → Grok → Gemini; Council last (R33) |

## 5. Wiring

**Edges:**
- `chat/*` → `ai/core` (types) and, through the app host, `ai/host`.
- **No `chat` crate depends on a provider crate.**
- `ai/providers/*` → `ai/core`, `dpl-proc`, `dpl-tools-deps` (engine,
  egui-free).
- `ai/host` → `ai/core`, `ai/providers/*`, `ai/accounts`, `ai/profiles`,
  `ai/browser-auth`, `dpl-fabric`.
- `dpl-fabric`'s provider facts move from `dpl-chat-core` to `dpl-ai-core`
  (exception §3 amended).
- `shell/providers.rs` reads AI's published state.

**Depot:**
- new ids `wezterm` (terminal family) and `claude` (provider);
- `node` if Gemini is approved.

**Rules for every new or moved crate:**
- the §5 checklist;
- `architecture.md` + the xtask audit amended **in the same commit**;
- no re-export shims; callers move.

## 6. Proof (every pass)

**R36, for every item below.**
- No check passes by being skipped.
- No mock provider stands in for a real turn, and no fixture is shown as
  live.
- Drills exit non-zero on failure.
- Gaps are named in §8.

Each pass:
- `cargo green` locally, then **both** CI lanes green before merge.
- Real-exe shots at **1100×700 and maximized**.
- Pilot scenarios for every new control (R37).
- An orphan-process check before and after.
- **Real turns on this laptop** with the operator's logins. CI cannot hold
  vendor credentials; that is stated, never faked.
- The exe is rebuilt in THE checkout and its mtime checked; `STATE.md` and
  `CURRENT.md` are updated.

## 7. Risks

- **Vendor protocol churn** is exactly R40's job. The probe-gated update
  path keeps the last working version.
- **Desktop apps** may expose little to attach to. A6 probes first and ships
  only what is proven.
- **A1 moves many crates at once.** It is behaviour-neutral by design; the
  existing drills and census tests catch drift.
- **K1 is large** (358 `Tone` sites). It is the operator's ordering law,
  proven pixel-identical.
- **Experience regression.** Anything today's chat does that the rebuilt
  one does not yet do is listed in that pass's plan.
- **Parallel lanes.** `gh pr list` before every pass.

## 8. Evidence

- **Read in-session 2026-10-10:**
  - `architecture.md` (full);
  - `design-system.md` §0.6, §1, §6, §7;
  - `doctrine.md` §27-92;
  - `roles.json`, `traits.json`;
  - the ColdLight/slides/shell/pilot component inventory;
  - the current chat code audit.
- **Probed 2026-10-11:**
  - Store apps `OpenAI.Codex` 26.1007.2314.0, `OpenAI.CodexBeta`
    26.727.4816.0 and `Claude` 2.31226.1.0;
  - "Grok Bot 0.63.0" (SpaceXAI) at `C:\Program Files\Grok Bot`;
  - WezTerm not installed;
  - the PATH `claude` is a non-Anthropic npm package (*"ANTHROPIC_API_KEY
    not set in .env file"*);
  - the Codex desktop daemon is running (0.162.1, `--listen unix://`).
- R1–R35 were re-read at their cited lines; R36–R41 are from 2026-10-10/11.
