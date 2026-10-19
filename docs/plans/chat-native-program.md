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

**Amended 2026-10-11 (operator: *"update the existing if wrong and ensure
you plan against the existing code to implement canonical consistent naming
and file and folder patterns"*):**
- §3.2a measures what each vendor exposes against what we cover.
- §3.2b lists the naming and layout defects in the existing code.
- §4.3 replaces the "one contract" with:
  - a three-tier `Event` (core · extension · lossless native);
  - the per-provider coverage census as a gate;
  - one identical file pattern per provider;
  - a mechanical rename map;
  - a `Participant` in the model from day one.
- A1–A4 are updated to match, and the Claude fact is corrected (the
  official CLI is installed; only the PATH entry is an impostor).

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
| Anthropic Claude | **Store app `Claude` 2.31226.1.0** | **The official Claude Code CLI IS installed:**<br>• `~\.local\bin\claude.exe` → `2.1.283 (Claude Code)` (Anthropic's native installer)<br>• `~\.vscode\extensions\anthropic.claude-code-2.1.296-win32-x64\resources\native-binary\claude.exe` → `2.1.296`<br>**But the `claude` that PATH resolves first is not Claude Code:** `AppData\Roaming\npm\claude.ps1` is an npm package that errors *"ANTHROPIC_API_KEY not set in .env file"*, and today's presence probe runs exactly that. | Source the native binary into the depot (decision 3); never resolve `claude` from PATH. |
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

### 3.2a What each vendor actually exposes vs what we cover (measured 2026-10-11)

| Vendor | Programmatic surface (evidence) | Our adapter covers today | Not reachable programmatically (stated, not promised) |
|---|---|---|---|
| Codex | `codex app-server` JSON-RPC. Vendored schema `codex-cli 0.146.0-alpha.3.1`: **89 client requests, 70 server notifications** (`crates/chat/providers/codex/schema/{ClientRequest,ServerNotification}.json`). Server requests (approvals) are **not vendored**. | **3 requests** (`thread/start`, `turn/start`, `turn/interrupt`) + `initialize`; **~8 notifications** (`runtime.rs`). Missing: thread list/read/resume/fork, `model/list`, command/file-change items, `turn/diff/updated`, `turn/plan/updated`, MCP, rate limits, review, the `thread/realtime/*` voice notifications. | ChatGPT consumer-app features; Codex is the coding agent, not the ChatGPT app. |
| Claude | Claude Code CLI 2.1.283/2.1.296 (`--help` read in-session): `-p` with `--input-format stream-json --output-format stream-json`, `--include-partial-messages`, `--replay-user-messages`, `-r/--resume`, `-c/--continue`, `--fork-session`, `--session-id`, `--permission-mode`, `--permission-prompt-tool`, `--model`, `--mcp-config`, `--agents`, `--allowed-tools`/`--disallowed-tools`, `--add-dir`, `--settings` | **Nothing** (no runtime; history decoder only) | **Claude Desktop's own features** (Projects, Artifacts, Research, connectors). No programmatic surface found on this laptop. "Wrap" is launch/focus until the A6 probe proves more. |
| Grok | Grok Build `0.2.112`, ACP over `agent stdio`. ACP schema (`agent-client-protocol-schema` in the cargo registry): `initialize`, `authenticate`, `session/{new,load,list,resume,fork,close,delete,prompt,cancel,set_mode,set_config_option,request_permission,update}`, `fs/{read,write}_text_file`, `terminal/{create,output,wait_for_exit,kill,release}` | **About half**: initialize, authenticate, prompt, load/list sessions, cancel, fs read/write, terminals, permissions. Missing: resume, fork, close, delete, set_mode, set_config_option | Grok Bot desktop features; Imagine only through the xAI API key (ADR-011). |
| Gemini | Node bundle `0.54.4`; protocol **unprobed** | Nothing (descriptor only) | Unknown until probed (A8) |

**Group chat does not exist on any vendor side.** A council is DevPULSE
orchestrating separate vendor sessions, feeding each participant the shared,
attributed transcript under a turn policy (round-robin, the operator's
2026-08-03 ruling). It is bounded by each provider's latency and context
window.

### 3.2b Naming and layout defects in the existing code (read 2026-10-11)

| Defect | Evidence |
|---|---|
| **Two parallel type systems for one concept** | Generic `RuntimeCommand/RuntimeEvent/RuntimeFailure/RuntimePermissionOption` (`chat/runtime/src/seam.rs`) **and** Grok's own `GrokRuntimeCommand/GrokRuntimeEvent/GrokRuntimeFailure/GrokPermissionOption` (`grok/src/runtime.rs`). Grok bypasses the generic set; `GrokRuntimeSeam` (`grok/src/seam.rs`) is dead. |
| **One concept, two names** | "How to launch the CLI" is `CodexRuntimeProfile` (Codex) and `GrokLaunchSpec` (Grok). The login process is `CodexLoginExit/Process/Launcher` in Codex but unprefixed `LoginExit/Process/Launcher` in Grok. |
| **"seam" means four things** | The `dpl-seam` agent HTTP crate (`architecture.md:33`); `egui_pilot_ai_protocol::seam` (coach); `chat/runtime/src/seam.rs`; `grok/src/seam.rs`. The naming law (`design-system.md` §0.6) forbids a term with two meanings. |
| **"terminal" is about to mean two things** | `grok/src/terminal.rs` is ACP's agent-requested terminal; the operator's WezTerm TTY (R41) is a different thing. |
| **`chat/core` mixes two boundaries** | AI: `ProviderDescriptor`, `ProviderManifest`, `Capability`, `PresenceSpec/Verdict`, `ActivationState`, `DeviceAuthorizationPrompt`, `AccountSummary`, `RecoveryAction(Kind)`, `ProviderSessionId`, `SessionState/Group/Summary`, `TokenUsage`, `BoundedNativePayload`. Chat: `ChatSessionId`, `ChatEventId`, `ChatEvent*`, `EventPresentation`, `Command*`, `Content*`, `GuidedValue*`. |
| **No shared per-provider file pattern** | Codex: `lib, activation, runtime`. Grok: `lib, runtime, seam, terminal, retry`. Claude, Gemini, Cursor: `lib` only. The chat window is a single 6,959-line `lib.rs`. |
| **Stuttering names** | `dpl_chat_codex::CodexAppServerRuntime`, `dpl_chat_grok::GrokAcpRuntime`: the crate path already says the vendor. |

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
| R1 | Layman-friendly, frictionless, touch-first (75" touchscreen) | `docs-history-2026-10-12:docs/plans/authoring-architecture.md:142-148` |
| R2 | A **window**, from the **status bar**, Steam-like, **portable lite**, **loads routes** | `docs-history-2026-10-12:docs/plans/chat-consolidation.md:8-10` |
| R3 | Route-aware; manage/control the editor; add library elements | `docs-history-2026-10-12:docs/plans/chat-creation-and-workflows-vision.md:13-16` |
| R4 | Grok mode button: create image/video, preview drafts, 30-day flush with consent | same, l.35-40 |
| R5 | Assist = one clear, simple acknowledgement for all providers | `docs-history-2026-10-12:docs/plans/chat-assist-mode.md:3-5` |
| R6 | Guided explainer workflow from chat | `explainer-guided-workflow.md:5-11` |
| R7 | One-click provider selection; sign in inside chat; once connected, **its own tab** with sessions | `docs-history-2026-10-12:docs/plans/chat-provider-easy-select.md:7-9`, `docs-history-2026-10-12:docs/plans/chat-connected-provider-tabs.md:9-10` |
| R8 | **Full access to the platform**, as the point of operations | `docs-history-2026-10-12:docs/plans/chat-operations-console.md:3-6` |
| R9 | **All chats in one system** | `docs-history-2026-10-12:docs/plans/chat-histories-in-one-system.md:10-11` |
| R10 | Providers activated in chat are **available to the whole platform** | `docs-history-2026-10-12:docs/plans/chat-providers-become-platform.md:21-23` |
| R11 | Each provider built **as an individual** | `docs-history-2026-10-12:docs/plans/phase3-analytics-program.md:219-222` |
| R12 | **Piggyback** existing active sessions | same, l.278-279 |
| R13 | Presence awareness at launch; sign-in in the **laptop's own browser** | `docs-history-2026-10-12:docs/plans/providers-presence-and-the-brain.md:12-19` |
| R14 | The known frontier AIs become space for our sessions | same, l.21-22 |
| R15 | The Providers control: ink-bleed hover, thumbnails, provider modal | same, l.24-34 |
| R16 | **Clyffy**: face of the AI; not in group chat; own session, powered by any provider | same, l.239-241; `docs-history-2026-10-12:docs/plans/chat-boundary.md:5-13` |
| R17 | A **secondary cockpit**: load routes, **popup windows**, **task windows** that minimize and restore | `docs-history-2026-10-12:docs/plans/providers-presence-and-the-brain.md:281-285` |
| R18 | **Chat owns its store** | `adr-017-provider-capability-registry.md:156` |
| R19 | **Chat separated from AI**; AI powers the platform; social later | `docs-history-2026-10-12:docs/plans/ai-chat-social-separation.md:10-12`; `docs-history-2026-10-12:docs/plans/chat-boundary.md` |
| R20 | **Commands work, Enter sends**: `/route`, `/model`, `/routines`, with guided values | `docs-history-2026-10-12:docs/plans/chat-boundary.md:5-13` |
| R21 | If ChatGPT is active it **shows active and I can chat** | `docs-history-2026-10-12:docs/plans/codex-conversation-transport.md:10-11,230` |
| R22 | Bleeding-edge transport is fine | same, l.141-142 |
| R23 | Brainstorm: editable visual components; sessions spawn; counterpoints | `docs-history-2026-10-12:docs/plans/brainstorm-and-clyffy-drive-capture.md:9-17` |
| R24 | Clyffy may drive the app, behind manual-approval gates | same, l.19-25 |
| R25 | A true **council** of grok/chatgpt/gemini/claude | `product-vision.md:24-37` |
| R26 | Mocks first | `product-vision.md:307-308` |
| R27 | Work in passes; **new boundaries go to the operator** | `docs-history-2026-10-12:docs/plans/providers-presence-and-the-brain.md:377-381` |
| R28 | Portable; external tools are subprocesses | ADR-009 + 2026-10-10 |

**ORIGIN, 2026-07-21** (`old-app:docs/plans/chat-agent-console-plan.md:15-25`,
`old-app:docs/plans/fabric-plan.md:19-21`; re-read at the lines)

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
crates/chat/renderers/   event rows (message / tool / result / quota / native) — rebuilt, wired in
crates/chat/windows/     peer + utility window lifecycle (unchanged boundary)
crates/chat/core/        CHAT's domain only after A1: ChatSessionId · ChatEvent* · EventPresentation ·
                         Command* (slash commands) · Content* (episode spine) · GuidedValue*
crates/chat/store/       chat's OWN conversation store + the one list of all chats (R18, R9),
                         plus the in-memory chat state from the retired chat/runtime
crates/chat/interactions/ · crates/chat/context/   unchanged boundaries (overlay rules, route context)
```

**Naming inside chat** follows the same rules as AI:
- one module per concept;
- no stuttering (`dpl_chat_window::composer`, not `ChatComposerWindow`);
- vendor names never appear in chat code, which knows only `ProviderId`
  and the `dpl-ai-core` types.

A renderer for the `Native` event tier means a not-yet-mapped vendor event
is still shown, never dropped.

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

**AMENDED 2026-10-11.** The first draft promised "the ONE command/event
contract". A single lowest-common-denominator contract would silently drop
what only some vendors do (Codex diffs and plans, Claude's agents and hooks,
ACP's session fork). That would make "vendor updates are just a seam update"
false. The corrections below are grounded in §3.2a and §3.2b.

#### 4.3.1 The boundary and its crates (names pass the audit: `dpl-` + leaf folder)

```
crates/ai/
  core/          dpl-ai-core          the single source of truth (contract types only — pure, no IO)
  host/          dpl-ai-host          supervisor · registry · depot resolution · update path · fabric publish
  accounts/      dpl-ai-accounts      (moved from chat/accounts)      activation evidence ledger
  profiles/      dpl-ai-profiles      (moved from chat/profiles)      operator-authored profiles
  browser-auth/  dpl-ai-browser-auth  (moved from chat/browser-auth)  sign-in in the laptop's browser (R13)
  imagine/       dpl-ai-imagine       (moved from chat/imagine)       xAI media (R4)
  providers/
    codex/ claude/ grok/ gemini/      dpl-ai-<provider> — one individual each (R11)
```

`chat/providers/cursor` is retired: no operator mandate (R33).
`chat/runtime` is retired:
- its contract (`seam.rs`) becomes `dpl-ai-core::adapter`;
- its registry becomes `dpl-ai-host::registry`;
- its in-memory chat state becomes `dpl-chat-store`.

#### 4.3.2 `dpl-ai-core`: the single source of truth, one module per concept

| Module | Owns |
|---|---|
| `provider.rs` | `ProviderId` (open newtype, per the 2026-08-11 decision B), `Manifest`, `Accent` |
| `capability.rs` | `Capability`, plus the declared-vs-earned qualification (`qualified_capabilities`) |
| `presence.rs` | `PresenceSpec`, `PresenceVerdict` |
| `activation.rs` | `ActivationState`, `DeviceAuthorizationPrompt`, `AccountSummary`, `RecoveryAction(Kind)` |
| `session.rs` | `SessionId`, `SessionState`, `SessionGroup`, `SessionSummary` |
| `turn.rs` | `TurnId`, `TurnInput` (text, attachments, the route context chat offers) |
| `event.rs` | **`Event`, three tiers.** **(1) Core:** turn started/completed, text delta, reasoning delta, tool call, plan, diff, usage, error. **(2) Extension:** capability-flagged events only some vendors emit, typed. **(3) `Native`:** the vendor's original payload, preserved **losslessly** for anything not yet mapped. It is shown and logged, never dropped (bounded by the existing `BoundedNativePayload`). |
| `request.rs` | `Request` (outbound: start session, send turn, interrupt, steer, resume, fork, answer approval) + `SendError` |
| `approval.rs` | `ApprovalRequest`, `ApprovalOption(Kind)`, `ApprovalDecision`. This merges `RuntimePermissionOption*` and `GrokPermissionOption*`. |
| `usage.rs` | `TokenUsage`, `UsageReporting`, `ProviderUsage`, `SessionUsage` |
| `history.rs` | the `HistoryDecoder` port + `DecodedHistoryEvent`, `DecodeOutcome` (each provider implements it) |
| `participant.rs` | `Participant` { Operator · Provider(`ProviderId`) · Clyffy }. Every message carries its speaker from day one, so the council (R25) is not a retrofit. |
| `adapter.rs` | **the port:** `trait Adapter` (what `ProviderRuntime` was), plus `Failure`/`FailureKind` |
| `census.rs` | the protocol **coverage census** types: `ProtocolItem { kind: Request \| Notification \| ServerRequest, name }`, `Coverage { Mapped(&'static str) \| Unused { reason } \| Unmapped }` |

#### 4.3.3 Every provider crate has the SAME files. A mode a vendor lacks is an absent file, never a stub.

```
crates/ai/providers/<vendor>/
  Cargo.toml        dpl-ai-<vendor>
  schema/<vendor-version>/   vendored protocol schema, GENERATED from the exact depot binary
                             (`cargo xtask vendor-schema <vendor>`); a VERSION file names it
  src/
    lib.rs          the `Manifest` const + module declarations — no logic
    launch.rs       `LaunchSpec`: depot binary, args, env allow-list, working folder
                    (merges `CodexRuntimeProfile` + `GrokLaunchSpec`)
    activation.rs   presence probe + sign-in ceremony, through the depot only
    protocol.rs     wire types for this vendor (Codex: JSON-RPC frames from the vendored
                    schema; Grok: the `agent-client-protocol` crate's types, used directly)
    transport.rs    the process's stdio framing — the bytes, nothing semantic
    adapter.rs      `impl dpl_ai_core::Adapter` — the ONLY place vendor ⇄ canonical mapping lives
    census.rs       every protocol item of the pinned version, classified (Mapped / Unused+reason)
    history.rs      `impl HistoryDecoder` for the vendor's transcripts
    usage.rs        the vendor's usage dialect
    tty.rs          the vendor's own TUI in our WezTerm (R41 terminal mode)
    desktop.rs      desktop-app wrap — ONLY once the A6 probe proves a capability
    terminal.rs     (ACP vendors) agent-requested terminals; distinct from tty.rs
```

Type names are **unprefixed inside a provider crate** and read through the
path: `dpl_ai_codex::Adapter`, `dpl_ai_grok::Adapter`. This follows the Rust
API Guidelines (no stuttering), and the same shape means a reader of one
provider can read them all. The supervisor's restart policy (today's
`grok/src/retry.rs`) moves to `dpl-ai-host::retry` because it is not
vendor-specific.

#### 4.3.4 The coverage census: what makes "a vendor update only needs our contract to cover it" real

- **Every item is classified.** Each provider's `census.rs` lists every
  request, notification and server request of its pinned protocol version:
  `Mapped` (to which canonical item), or `Unused { reason }`.
- **A test fails on drift.** It regenerates the item list from the vendored
  schema and **fails when any item is unclassified**: a new vendor method, or
  one removed while still mapped. `cargo green` runs it.
- **A vendor update reads as a red list.** Staging a new vendor version
  (A3) regenerates the schema. The census then prints exactly what is new,
  changed or gone. That list is the update's to-do, and nothing reaches the
  operator half-mapped.
- **Today's baseline (§3.2a)** is roughly 12 of 159+ Codex items mapped and
  about half of ACP. The census makes that gap visible and enforced instead
  of hidden.
- **Approvals.** Codex's server-request schema (approvals) is vendored with
  the rest, and every server request is answered (A2).

#### 4.3.5 Rename map (A1, mechanical, no shims; callers move in the same commit)

| Today | Becomes |
|---|---|
| `dpl_chat_runtime::seam::{ProviderRuntime, RuntimeCommand, RuntimeEvent, RuntimeUpdate, RuntimeFailure(Kind), RuntimeSendError, RuntimeCapabilities, RuntimeSessionSummary, RuntimePermissionOption(Kind)}` | `dpl_ai_core::{Adapter, Request, Event, Event(core tier), Failure(Kind), SendError, Capability set, SessionSummary, ApprovalOption(Kind)}` |
| `GrokRuntimeCommand/Event/Failure(Kind)/SendError/PermissionOption(Kind)/SessionUpdate/SessionSummary/AcpCapabilities` | **deleted.** Grok maps onto the core types in its `adapter.rs` (this retires the Grok-only host path) |
| `GrokRuntimeSeam` | deleted (dead) |
| `CodexAppServerRuntime`, `GrokAcpRuntime` | `dpl_ai_codex::Adapter`, `dpl_ai_grok::Adapter` |
| `CodexRuntimeProfile`, `GrokLaunchSpec` | `dpl_ai_codex::LaunchSpec`, `dpl_ai_grok::LaunchSpec` |
| `CodexActivationCoordinator`, `GrokActivationCoordinator` | `dpl_ai_<vendor>::activation::Coordinator` |
| `CodexLogin{Exit,Process,Launcher}`, Grok `Login{Exit,Process,Launcher}` | `dpl_ai_<vendor>::activation::{LoginExit, LoginProcess, LoginLauncher}` |
| `CodexHistoryDecoder`, `ClaudeHistoryDecoder` | `dpl_ai_<vendor>::history::Decoder` |
| `CodexUsage`, `ClaudeUsage`, `GrokUsage` | `dpl_ai_<vendor>::usage::Usage` |
| `GrokRuntimeRetryState`, `GrokRuntimeFailureDisposition`, `GrokVerificationOrigin` | `dpl_ai_host::retry::{RetryState, FailureDisposition, VerificationOrigin}` |
| `dpl_chat_runtime::{ProviderRegistration, ProviderView, RegistryError}` | `dpl_ai_host::registry::{Registration, View, RegistryError}` |
| AI types in `dpl_chat_core` (§3.2b list) | `dpl_ai_core::<module>` per §4.3.2 |
| `dpl_chat_store::{HistoryDecoder, DecodedHistoryEvent, DecodeOutcome, UsageReporting, ProviderUsage, SessionUsage}` | `dpl_ai_core::{history, usage}` |

**Naming law (`design-system.md` §0.6).**
- **Industry terms** (meaning (b)): adapter (ports and adapters),
  transport, protocol, session, turn, request, event, capability, manifest,
  presence, tty.
- **Repo terms** (meaning (c)): `census` (the architecture.md census law),
  `activation`, `participant`. A1 adds a definition row for each to
  §0.6 **before** the code that uses them.
- **"seam" is not used anywhere under `crates/ai/`.**

#### 4.3.6 The three ways in, per provider crate (R41)

| Mode | What it is | Per vendor (probed first, honest about limits) |
|---|---|---|
| **CLI (background)** | The vendor CLI as a supervised subprocess from the depot, driven over its protocol | Codex `app-server` JSON-RPC · Claude Code `stream-json` · Grok `agent stdio` (ACP) · Gemini headless (multi-turn to be probed) |
| **Desktop wrap** | Attach to, or launch and focus, the vendor's installed desktop app; reuse its signed-in session (R12 piggyback) | Codex desktop: its local **`app-server` daemon** is running; attaching to it is the candidate. Claude Desktop and Grok Bot: **probe first** what can be attached or driven. Only proven capabilities are shown (R36). **Terms gate (2026-10-18):** driving Anthropic's own desktop app from DevPULSE sits near the Consumer Terms' "automated or non-human means" clause. Nothing beyond launch/focus is built for it without a clean reading of the terms, put to the operator first. |
| **Terminal (TTY)** | The vendor's own terminal UI in **our WezTerm**, portable, from the depot, a subprocess with a DevPULSE-owned config | `codex` · `claude` · `grok` interactive TUIs. Opened from chat's UI ("Open in terminal"), never by typing a command (R37). |

**Updates that keep subscriptions working (R40)**, the per-provider update
path:
1. **Compatibility is proven, not hard-coded.** Each provider's census
   (§4.3.4) classifies its whole protocol surface. On staging a vendor
   version, a **compatibility probe** runs against that exact binary: the
   handshake, then schema regeneration (`cargo xtask vendor-schema
   <vendor>`, e.g. `codex app-server generate-json-schema`), then the
   census diff. The result is written into the depot manifest beside the
   payload. This replaces stale constants like `>=0.145,<0.146`.
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

#### 4.3.7 Subscriptions done right, one shared conversation, roles and routines (R279, R280)

**AMENDED 2026-10-18** from a sourced research pass. The vendors' terms were
re-verified at their sources the same day, and the evidence and links are in
`docs/requirements.md` R279–R280.

**What the vendors allow. This is binding on every provider pass.**

| Vendor | Allowed | Not allowed |
|---|---|---|
| OpenAI | Codex `app-server` (OpenAI's first-class embedding path), with the CLI managing its own ChatGPT login | DevPULSE holding tokens; exposing the subscription as an API to other tools; rotating accounts |
| Anthropic | The **unmodified** `claude` binary, with the operator signing in through Anthropic's own flow in a real terminal. Use stays "ordinary, individual". | DevPULSE collecting, storing or intermediating Claude credentials or tokens; a DevPULSE-owned "Sign in with Claude" |
| xAI | `grok agent stdio` (ACP), as documented | Account sharing (no third-party clause was found) |
| Google | **Gemini CLI on a paid Gemini API key**, funded by the subscription's Developer Program credits (AI Pro $10/month, Ultra up to $100/month, usable "towards Gemini API"), or the free API tier. The key is entered in the CLI's own setup. | Driving Antigravity or any Google consumer OAuth from DevPULSE (Antigravity terms §6). Revisit if Google ships official ACP support. |

**Auth health (one state machine for every provider).** States:
- Ready;
- Limited (until the reset time);
- Stale (a "signed in but rejected" login, which the CLI must restart);
- Needs sign-in (a terminal opens on the vendor's own sign-in command).

Built on each CLI's real signals:
- **Codex:** `account/read`, `account/rateLimits/read` and `…/updated`;
  turn errors `Unauthorized` and `UsageLimitExceeded`. After
  `codex login --device-auth` the supervisor **restarts app-server**
  (openai/codex#47456).
- **Claude:** `claude auth status` (exit code); stream-json
  `rate_limit_event` and `error: authentication_failed`.
- **ACP vendors:** `authenticate` / auth_required.

There are no retry loops: a 429 backs off until `resetsAt`. This lives in
`dpl-ai-host` with per-vendor signal mapping in each provider's
`activation.rs`. It lands with A2 (Codex), A4 (Claude) and A7/A8 (Grok,
Gemini).

**One shared conversation (the room).**
- **The transcript.** DevPULSE owns the room's transcript in `dpl-chat-store`
  (C1): who spoke, their role, the text, and a summary of their tool calls,
  never the payloads.
- **Native sessions.** Each vendor keeps its **native** session (a Codex
  thread, a Claude `--session-id`, an ACP session).
- **Deltas.** Before a participant's turn, the host sends only what is new
  since it last spoke: Codex via `thread/inject_items`, Claude and ACP
  vendors as a user message.
- **A local stdio "room" MCP server** is given to every participant
  (`room.read(since)`, `room.note`, a shared scratchpad).
- **A scribe role** keeps a rolling digest. The digest replaces the full
  history when a participant nears its context limit (Codex
  `ContextWindowExceeded`, ACP `usage_update`, Claude result `usage`).

**Roles and routines.**
- **Roles.** Each participant gets a role prompt. The UI shows a roster
  with role chips and @-mentions, and DevPULSE moderates who speaks.
- **Routines, built in this order.**

| # | Routine | Why first (evidence in R279) |
|---|---|---|
| 1 | **Council: select, never blend.** Parallel answers, anonymized ranking (no model ranks its own), then a chairman **selects** and lightly edits. A cheaper mode runs "best model × N". | Voting and selection carry the gains of debate. Judge selection beat Mixture-of-Agents synthesis. LLM judges prefer their own outputs. |
| 2 | **Author plus a cross-vendor critic** with fixed lenses (correctness, security, tests) | Verification failures dominate multi-agent failures. Single executors do better on sequential work. |
| 3 | **Router**: @-mention, or task-type rules; escalate to the Council only for judgment-heavy questions | Routers cut cost 2× or more. Multi-agent setups lose on sequential tasks. |

**Not built yet:** open-ended multi-round debate and layered answer blending
(MoA synthesis).

**"MoE" in this program** means agent-level orchestration. Model-internal
experts cannot be changed in vendor models. A local open-weight MoE model
(llama.cpp, a depot subprocess) is a terms-free candidate for the scribe or
router role, and belongs to R277's lab.

### 4.4 Passes (each: plan doc, 7 sections + §8; PR; double-green; real-exe proof; exe rebuilt; STATE)

| Pass | Scope | Done means |
|---|---|---|
| **P0** | This plan approved (with §4.6) | Operator approval |
| **D0** | The laptop's deps (`d0-laptop-deps.md`, approved): startup check, honest media, camera, portable release | As that plan states |
| **K1 = design-system 3′** | The operator's ordering law: the naming merge + `coldlight` module split, pixel-identical | `cargo green`; before and after shots at 1100×700 and maximized |
| **K2** | ColdLight token extensions for chat (`roles.json` chat surface, icons, caret/busy motion) | Generated; census tests; Library walls show them |
| **A1 = the AI boundary, canonical** | Create `crates/ai/*` (§4.3.1). Move accounts, profiles, browser-auth, imagine and providers/* under it. Split `chat/core`, `chat/runtime` and `chat/store` (§3.1, §4.3.5). Apply the **rename map** and the **same-files pattern** (§4.3.3) to every existing provider. Author `dpl-ai-core` per §4.3.2: the three-tier `Event`, `Participant`, the `census` types. Move presence, usage and backfill out of the app. Add the §0.6 definition rows. **No shims; behaviour unchanged.** `architecture.md` + the xtask audit are amended in the same commit. | `cargo green`; existing chat drills unchanged; real-exe shot; the structure audit shows chat ↛ provider crates; a grep shows no `seam` under `crates/ai/` and no vendor name under `crates/chat/` |
| **A2 = host + Codex done right** | `dpl-ai-host` supervisor. Codex CLI mode: depot-only presence and sign-in; **the server-request schema vendored and every server request answered**; turn deadline. **The Codex census** classifies all 89 requests, 70 notifications and the server requests (Mapped / Unused+reason), and the core history/resume/models/diff/plan items are mapped. `cargo xtask vendor-schema codex`. | **A real turn on this laptop** with the operator's login; the census test is green on the pinned version; orphan check clean |
| **A3 = the update path** | Probe (handshake + `vendor-schema` + **census diff**) + manifest + "Update" control. Codex is moved from the depot's 0.145-alpha to the operator's current version through it. | Update proven end to end, with the census red-list shown for the version jump. A refusal is proven against a deliberately incompatible build kept in a scratch depot. |
| **A4 = Claude Code** | The official Claude Code native binary (installed at `~\.local\bin\claude.exe` 2.1.283; never the PATH impostor) sourced into the depot. A `stream-json` adapter (both directions, `--include-partial-messages`, `--resume`/`--fork-session`, `--permission-prompt-tool` for approvals). A census over its documented flags and message types. Presence and sign-in through the depot. | Real Claude turn |
| **A5 = Terminal mode** | Portable WezTerm **installed** into the depot (new tool id, `terminal` family); "Open in terminal" for Codex, Claude and Grok | Launches the depot binary with our config; closes clean |
| **A6 = Desktop wrap** | Probe each desktop app (Codex daemon attach, Claude Desktop, Grok Bot); build only what the probe proves | Each proven capability demonstrated; unproven ones stated |
| **C1 = chat store** | Conversations persisted under `.userdata/chat/` (document law: `v`, refuse newer, tmp → bak → rename); the one list across DevPULSE + vendor history (through AI's decoders) | Quit and relaunch; history is there; vendor chats in the same list |
| **C2 = chat UI** | `chat/window` rebuilt as in §4.2, on ColdLight, colocated. **This real egui window is the mock the operator red-pens (R26, R38).** | Shots at 1100×700 and maximized; a pilot scenario drives every control; a grep proves zero raw colour/frames in `crates/chat/**` |
| **C3** | Commands with guided values; popup + task windows (R17, R20, R32) | Pilot scenario per command; minimize/restore proof |
| **C4** | Platform reach: AI available platform-wide; Assist; content actions (R5, R8, R10) | A non-chat surface runs a turn; round-trip proof |
| **A7 / A8** | Grok onto the host (retire its separate path). Gemini with Node **installed** into the depot, **on a paid Gemini API key** funded by the subscription's Developer Program credits, never on Google consumer OAuth (§4.3.7, R280). Each gets §4.3.7 auth health. | Real turns; an auth-health state shown for each |
| **C5 / C6 / later** | Grok media (R4); Clyffy (R16); **the room and routines (§4.3.7)**: the shared conversation, then Council (select, never blend; R25), then Author plus critic, then Router; brainstorm (R23); Clyffy drive (R24) | Each gets its own plan. The Council comes after Codex and Claude each work on their own (R33). |

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
| 6 | Provider order | Codex → Claude Code → Grok → Gemini (on a paid API key funded by Developer Program credits, R280); then the room and the routines of §4.3.7, Council first (R33). **Amended 2026-10-18:** the operator approved the §4.3.7 design. |

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

**Tooling and gate (A1–A3):**
- `cargo xtask vendor-schema <vendor>` regenerates `schema/<version>/` from
  the exact staged depot binary.
- Each provider's census test runs inside `cargo green`, so an unclassified
  protocol item fails the gate on both CI lanes.
- The structure audit gains two checks:
  - no `chat/*` → `ai/providers/*` edge;
  - no vendor name under `crates/chat/`.

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
- The exe is rebuilt in THE checkout and its mtime checked; `docs-history-2026-10-12:docs/STATE.md` and
  `docs-history-2026-10-12:docs/ops/resume/CURRENT.md` are updated.

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
- **Measured 2026-10-11 (§3.2a, §3.2b amendment):**
  - **Codex protocol.** Parsed the vendored `ClientRequest.json` and
    `ServerNotification.json` (`oneOf` → `method`): **89 requests, 70
    notifications**. A grep of `codex/src/runtime.rs` finds 3 requests sent
    and about 8 notifications mapped.
  - **ACP protocol.** A grep of `agent-client-protocol-schema` (cargo
    registry) gives the method list in §3.2a. A grep of
    `crates/chat/providers/grok/src/*.rs` shows the methods used.
  - **Claude Code CLI.** `claude.exe --version` returns `2.1.296 (Claude
    Code)` (VS Code extension native binary) and `2.1.283 (Claude Code)`
    (`~\.local\bin`). `--help` was read for the stream-json, resume, fork,
    permission, model, MCP and agents flags.
  - **Naming defects.** A `pub struct|enum|trait` inventory of
    `chat/{core,runtime,store,accounts,profiles}` and `providers/{codex,grok,claude}`
    (§3.2b), plus a grep of `docs/design` for the four meanings of "seam".
  - **Naming audit rule.** `crates/xtask/src/main.rs:1063-1090`: crate =
    `dpl-` + a name ending in the leaf folder, so `dpl-ai-codex` for
    `crates/ai/providers/codex` passes.
- R1–R35 were re-read at their cited lines; R36–R41 are from 2026-10-10/11.
