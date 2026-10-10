# Program — Chat, built natively in this app from the operator's requirements

**Status:** APPROVED TO PROCEED on recommendations — 2026-10-10. Asked for
rows 22/46 and Q1/Q2/Q3/Q11, the operator answered *"what the fuck whatever
fine"*. Read as: build it methodically on the recommendations in §4.9. Every
adopted recommendation stays **revisitable** — the operator can override any
row at any time, and each pass plan re-states the ones it depends on.
Authored 2026-10-10.

## 1. Context

**The ruling that opens this program (operator, 2026-10-10):**
*"it should properly build chat from scratch inside the fucking app like i
asked from the start and not just blindly copy that one but properly look at
my requirements for it and get it actually planned and implemented."*

"That one" is **chatmaton** (formerly `automaton`, the `mato-*` crates), and
with it **Shippin**. This program therefore **supersedes**:

- `docs/plans/unified-ai-kernel-automaton-merge.md` and
  `docs/journal/2026-08-11-unified-ai-kernel.md`. The operator did say, on
  2026-08-11, *"we want to merge the two as a complete right"* (merge plan
  l.6). Today's ruling replaces that, and the record keeps both.
- The unmerged `origin/feat/providers-carousel` branch's imported designs:
  the `crates/seat` "seat" model (from Shippin's NAMING-AND-SSOT), the
  Claude-Code scoping model, and the ChatGPT/Claude composer census.

**How requirements are sourced.** A requirement in this program must trace to
the operator's own words, quoted verbatim with file:line (§4.1). Agent-written
plans that call themselves "operator-locked" or "ratified" without a quote are
**not** requirements here. The main ones are `chat-window-product-vision.md`,
`chat-architecture.md`, `chat-steam-social-foundation.md` and
`chat-provider-catalog.md`. Their unsourced items are listed for challenge in
§4.8 and are not built unless the operator confirms them.

## 2. Goal / non-goals

**Goal.** A chat that lives in DevPULSE and meets the operator's needs:
- a native egui peer window, opened from the status bar;
- it signs in to the operator's own AI subscriptions and lets them actually
  talk;
- every chat is kept in one place;
- it is route-aware and can operate on the platform's content;
- the providers it activates power the whole platform;
- it is layman-friendly and touch-first.

It is built as clean in-repo crates, with each vendor CLI run as a
subprocess from the portable depot.

**Non-goals.**
- 🚫 Any dependency on, copy of, or sidecar to chatmaton, automaton, Shippin
  or clyffy repos.
- 🚫 A webview or browser-hosted chat.
- 🚫 Vendor binaries resolved from PATH.
- 🚫 Holding vendor credentials in DevPULSE (auth stays in each vendor CLI's
  own profile).
- 🚫 Social transport this program (operator: *"we have social to extend too
  later"*).
- 🚫 A second UI stack.
- 🚫 Inventing a feature the ledger in §4.1 does not carry.

## 3. Ground truth (read-only sweeps + in-session checks, 2026-10-10)

**Size.** The current chat is 22,119 LOC across 20 `crates/chat/**` crates.
The app adds `crates/app/src/chat.rs` (5,310 lines), `presence.rs`,
`ai_usage.rs` and `ai_backfill.rs`, and the shell adds `providers.rs` (604).

**What works:**
- **Codex** runs end to end through the `ProviderRuntime` seam
  (`codex app-server` JSON-RPC, three std threads).
- **Grok** runs through a **separate Grok-only path** (ACP; `GrokRuntimeSeam`
  is dead code).
- The peer window, its status-bar trigger and `NavigateRequest` route loading
  are wired. Evidence: `app/src/main.rs:1112-1121`; `chat.rs:1089-1092`,
  `1718-1725`.

**What is broken or missing:**
- **Conversations are never persisted.** They live in an unbounded in-memory
  `Vec` (`chat/runtime/src/lib.rs:25,297`). Only vendor-written history
  (`~/.codex/sessions`, `~/.claude/projects`) survives, read-only.
- **The Codex version gate is wrong.** It accepts only `>=0.145,<0.146`
  (`codex/src/activation.rs:10-11`), while the vendored schema and the
  operator's install are 0.146.
- **Codex server requests are never answered.** Server-initiated JSON-RPC
  requests (approvals) are treated as notifications (`codex/src/runtime.rs:263-267`),
  so a turn that needs approval can hang. There is no turn timeout.
- **Presence and sign-in run from PATH; runtime runs from the depot.**
  Presence and sign-in launch bare `codex`, `claude`, `grok`, `cursor-agent`
  and `gemini` from PATH with the full environment (`app/src/presence.rs:47,165`),
  but the runtime uses the depot. The two can disagree.
- **Claude, Cursor and Gemini cannot converse.** Claude has a history
  decoder only; Cursor and Gemini are descriptors.
- **Clyffy is inert** (`chat.rs:1655-1665`).
- **Two crates are dead:** `extensions` and `renderers` have no dependents.
- **Blocking work runs on the UI thread:** boot-time CLI spawns
  (`main.rs:990-994`), synchronous history listing, and Turso `block_on`.
- **The provider working directory is the process CWD** (`main.rs:990`).
- **There is no test that sends a real turn and checks for an assistant
  reply.** The only live-turn proof is a manual example.
- **There is no depot on this machine** (`dptools-deps/sources.local.toml`
  and every staged payload are absent). Depot-resolved providers cannot run
  here until it exists.
- **The working tree carries uncommitted 10-09 Codex work** (a placebo
  `ProcessManager`) that fails fmt and clippy (external-reach row 22).

**The code's shape.** `chat/window` is one 6,959-line file and the app host
is 5,310 lines. Both mix AI, content, social, brainstorm, imagine and
commands.

## 4. Design

### 4.1 Requirements ledger — the operator's words (each verified at the cited line)

| R | Requirement | Operator, verbatim (date · source) |
|---|---|---|
| R1 | Layman-friendly, frictionless, touch-first; for a 75" touchscreen | *"i want it layman friendly and frictionless not some high dev useless UI … to have on my large 75\" touchscreen"* (07-22 · `authoring-architecture.md:142-148`) |
| R2 | Chat is a **window**, triggered from the **status bar**, **Steam-like**, a **portable lite** surface that **loads routes** | *"chat needs to be merged it was always supposed to be a window and triggered from status bar steam like experience think of chat as a portable lite version that also will allow us to load routes from chat."* (07-24 · `chat-consolidation.md:8-10`) |
| R3 | Route-aware; can manage/control the editor and add library elements | *"this chat client needs to be properly route aware, so when I am working under WiredFRONT editor you can literally have chat then manage and control, add from the library of elements…"* (07-27 · `chat-creation-and-workflows-vision.md:13-16`) |
| R4 | Grok mode button to create image/video; preview drafts; 30-day cache flush with a consent prompt | *"…we have a mode button to trigger create video or create image … the cache will get flushed every 30 days if nothing is saved. We flag the user…"* (07-27 · same file l.35-40) |
| R5 | Assist = one clear, simple acknowledgement covering all providers | *"it needs to be a clear simple acknowledgement that will blanket for all providers"* (07-28 · `chat-assist-mode.md:3-5`) |
| R6 | Guided explainer workflow driven from chat (first prompt → bible → storyboard → video) | *"…work with chat that will have a guided system where the first prompt captures the necessary context … create the bible and storyboard…"* (07-28 · `explainer-guided-workflow.md:5-11`) |
| R7 | **Easy one-click provider selection**; sign-in to the subscription **inside chat**; once connected it gets **its own tab** with its sessions | *"you took away my ability to easily select a provider … it triggers inside the chat the ability to log in to subscription"* · *"once connected it has its own tab and then sessions had"* (07-29 · `chat-provider-easy-select.md:7-9`, `chat-connected-provider-tabs.md:9-10`) |
| R8 | Chat has **full access to the platform**, as the point of operations on content, slides and episodes | *"BOTH CHAT READS AND HAS FULL ACCESS TO THE PLATFORM THE WHOLE POINT IS THAT CHAT BECOMES A CLEAR POINT OF OPERATIONS…"* (07-30 · `chat-operations-console.md:3-6`) |
| R9 | **All chats in one system** | *"…convenient to have all the chats available in one system"* (07-30 · `chat-histories-in-one-system.md:10-11`) |
| R10 | Providers activated in chat become **available to the whole platform** | *"…the providers that get activated in the chat become available in the platform"* (07-30 · `chat-providers-become-platform.md:21-23`) |
| R11 | Each provider is built **as an individual** module (major ones first, extensible) | *"build them clearly as individuals please … one for grok one for gemini one for each major…"* (07-30 · `phase3-analytics-program.md:219-222`) |
| R12 | **Piggyback** existing active sessions | *"if the user already has active sessions we should be able to piggly back off them."* (`phase3-analytics-program.md:278-279`) |
| R13 | **Presence awareness at launch** (ChatGPT, Claude, Grok, Gemini); can start a subscription sign-in in the **laptop's own browser**, separate from the built-in browser | *"…we can absolutely have devpulse labs despite being portable have that awareness … open the laptops browser that needs to be separate from the built in browser"* (07-31 · `providers-presence-and-the-brain.md:12-19`) |
| R14 | The known frontier AIs listed become **space for our sessions** | *"…We want to have all of that become space available for our sessions."* (same file l.21-22) |
| R15 | The Providers control: a brain + "Providers" with a slow **ink-bleed** hover (5.6–6.3 s, 2-colour from a 4–5 pool, global preset); click opens a popup of thumbnails with dynamic tooltips, which open a modal provider summary | (07-31 · same file l.24-34) |
| R16 | **Clyffy** is the global and per-session face of the AI; not in group chat; becomes active when one subscription is live; its own session, powered by any provider | *"Clyffy will be our global/per session face … Clyffy will not exist in the group chat."* (08-03 · same file l.239-241) · *"as soon as one subscription comes live Clyffy becomes active. That is its own unique session that can be toggled to use any provider as powering it"* (08-07/08 · `chat-boundary.md:5-13`) |
| R17 | Chat is a **secondary cockpit**: route awareness, load routes, **popup windows**, **task windows** that minimize and restore | *"the chat becomes a secondary cockpit … have addition popup windows appear and evolve the chat and have task windows that can easily be minimized and brought back."* (08-03 · same file l.281-285) |
| R18 | **Chat owns its store** | *"Chat should be its own store."* (08-03 · `adr-017-provider-capability-registry.md:156`) |
| R19 | **Chat separated from AI**; AI powers the whole platform; social later; **the UI can stay like this** | *"i need chat to be separated from AI since it also powers the platform … and we have social to extend too later"* (08-04 · `ai-chat-social-separation.md:10-12`) · *"get this authored so the chat boundary and AI boundary are truly separate."* (08-08 · `chat-boundary.md`) |
| R20 | **Commands work and Enter sends**: `/route` (dynamic routes), `/model`, `/routines`, with guided seeded values that prevent bad inputs | *"Why are all of my commands missing why is enter not properly responding to send… /route … /model /routines etc … guided seeded values intuitive suggestions…"* (08-07/08 · `chat-boundary.md:5-13`) |
| R21 | If ChatGPT is active it **shows active and I can chat** | *"if i have chatGPT active already it should clearly show its active and i can chat"* · *"it shows active and I cannot chat"* (08-04 · `codex-conversation-transport.md:10-11,230`) |
| R22 | Bleeding-edge transport is fine even if half-implemented | *"i dont care if its half implemented i want the bleeding edge pattern."* (`codex-conversation-transport.md:141-142`) |
| R23 | Brainstorm: typing brainstorm triggers editable visual components; new sessions spawn; other providers offer counterpoints | (08-06 · `brainstorm-and-clyffy-drive-capture.md:9-17`) |
| R24 | Clyffy may drive the app as the operator, behind **manual-approval gates** on key parts, with a whitelist that grows | (08-06 · same file l.19-25) |
| R25 | Council: log into grok / chatgpt / gemini / claude and pull them into a **true council**; workflows with schemas the AI must fulfil | *"…log into to eventually grok chatgpt gemini claude subscriptions that can be then pulled into a true council…"* (07-30 · `product-vision.md:24-37`) |
| R26 | Mocks first | *"We should have mocks built first for a clear vision to work from."* (`product-vision.md:307-308`) |
| R27 | Work in passes; new boundaries go to the operator for input | *"we do it in passes … if new boundaries are being created we tell the operator so they can give their input or just select recommendations."* (08-03 · `providers-presence-and-the-brain.md:377-381`) |
| R28 | Portable app; external tools are subprocesses | ADR-009 + operator 2026-10-10 (*"portable app with its deps as proper sub processes"*) |

### 4.2 Shape — two in-repo families (R11, R18, R19, R10)

```
crates/ai/                      ← powers the PLATFORM (chat is one consumer)
  core        provider identity (open ProviderId), capability vocabulary,
              activation state, the ONE command/event contract, usage
  host        the supervisor: owns every provider subprocess (start on
              first use, health, restart w/ backoff, stop on idle/exit,
              no orphans), publishes activation on the fabric (R10)
  providers/
    codex     ChatGPT/Codex individual  — `codex app-server` (JSON-RPC)
    grok      Grok individual           — `grok agent stdio` (ACP)
    claude    Claude individual         — `claude` stream-json
    gemini    Gemini individual         — transport decided in its pass
crates/chat/                    ← the CHAT product (consumes ai/*)
  model       conversations, threads, sessions, Clyffy house session,
              council (later)
  store       chat's OWN store (R18) + read-only vendor-history index → R9
  commands    /route /model /routines … with guided values (R20)
  window      the egui peer window — split into modules, built to the
              approved mock (R26), never a 7k-line file
```

- Every provider is resolved through `dpl-tools-deps` (depot only) for
  **every** use: presence, sign-in and runtime. One binary, one truth. This
  fixes the PATH/depot split.
- Auth stays in each vendor CLI's own profile. "Piggyback" (R12) means using
  the vendor CLI's existing login. A ChatGPT browser tab or desktop app being
  signed in does **not** sign in the `codex` CLI; the UI must say so in plain
  words.
- One provider path for all providers. No Grok-only special case.
- The app host (`crates/app/src/chat.rs`) shrinks to wiring.

Folder and crate names follow the naming law, and the new crates go through
architecture.md's new-crate checklist plus the xtask audit table **in the
same commit**.

### 4.3 The supervisor (`ai/host`) — the real version of what the 10-09 work faked

| Requirement | Mechanism |
|---|---|
| Start lazily | The provider process starts on first use (a send, or opening its sessions), never at boot. Presence is a short, bounded probe on a worker thread. |
| Readiness | Each transport's own handshake: Codex `initialize`/`initialized`; ACP `initialize`/`authenticate`. Never "spawned = ready". |
| Health | The handshake, plus an exit watcher. A crash yields an honest `Failed(reason)` in the UI. |
| Restart | Backoff (absorbs Grok's `retry.rs`), with a cap. Then it stops and says so. |
| Idle | Stop after an idle window with no live turn. The window length is a constant in `ai/host` and shown in the UI. |
| Exit | Every child is killed when the app exits, including on crash. Windows: one Job Object with kill-on-close. Linux: `PR_SET_PDEATHSIG` / a process group. |
| Timeouts | Every turn has a deadline. Every server→client request is **answered** (approve/decline/expire), never dropped. |
| UI thread | Nothing blocks it. Spawns, probes, store I/O and history listing all run on workers, with results polled. |

The Windows Job Object needs a Win32 binding. Whether to accept that
dependency is decision **Q9**.

### 4.4 Chat's own store (R18, R9)

- **What is saved:** every DevPULSE conversation (user turns, assistant
  turns, tool and approval events, provider, model, route context, usage).
  It lives under `.userdata/chat/`, beside the exe: portable, never OS temp,
  never a vendor folder.
- **How vendor history joins it:** Codex and Claude transcripts are
  **indexed read-only** into the same list, so "all chats in one system" is
  one list. Vendor folders are never written. Grok and Gemini join if their
  CLIs keep local history (verified per pass).
- **Format:** decision **Q8**. The recommendation is append-only JSONL per
  conversation plus a small index (inspectable, crash-safe, no engine). The
  alternative is Turso, which is already in the binary via `dpl-index`.

### 4.5 The window (R1, R2, R7, R17, R20, R21, R26)

Built to an **approved HTML mock** (`docs/design/mocks/chat-mock.html`) that
the operator red-pens **before** any window code (R26).

The mock covers:
- the status-bar trigger;
- the peer window at real sizes;
- **one-click provider selection** with in-chat sign-in (R7);
- a connected provider's sessions, shaped by **Q2**;
- the composer, where Enter sends;
- the command typeahead `/route /model /routines` (R20);
- route-aware context (R3);
- popup and task windows that minimize and restore (R17);
- the Assist acknowledgement (R5);
- the plain-words states: not installed, signed out, signing in, ready,
  busy, failed — each with **why** and **what to do**;
- tooltips on every control;
- touch targets for the 75" screen (R1).

ColdLight only (the `coldlight-system` gate). The Providers control (R15)
gets its own mock stage.

### 4.6 Platform reach (R8, R10, R3, R24)

- **Platform-wide activation.** Activation is published on the existing
  fabric, so any platform surface can ask "which providers are ready" and run
  a turn through `ai/host` (R10).
- **Chat acting on content.** Content actions (add a library element, place
  a slide, load a route) are typed intents through the shell/fabric seams
  that already exist (`NavigateRequest`, composer and library intents). They
  are never raw access.
- **Gating.** With Assist on (R5), they run under the one blanket
  acknowledgement. Destructive or irreversible ones always show what will
  change and require a tap (R24's "manual approval on key parts").
  Confirmation is decision **Q5**.

### 4.7 Passes (proposed order — the operator sets it, R27; confirm as Q10)

Each pass gets its own plan doc (7 sections + §8), a PR, double-green CI,
real-exe proof at real window sizes, a rebuilt exe and a STATE update.

| Pass | Scope | Done means |
|---|---|---|
| **C0** | Clean ground: park and discard the 10-09 placebo work; mark the superseded docs; apply external-reach answers; **stand up the depot on this machine** | Tree clean; `cargo green` on Windows; depot resolves codex + grok |
| **C1** | Chat mock + experience inventory for chat | Operator approves the mock |
| **C2** | `ai/core` + `ai/host` + **Codex individual** (version gate fixed, server requests answered, turn timeout, depot-only presence/sign-in) | A pilot scenario **sends a real turn and asserts an assistant reply** (none exists today) |
| **C3** | `chat/store` + `chat/model` + window core to the mock: select provider, sign in, converse, Enter sends, persisted, all chats in one list | Restart the app and the conversation is still there; vendor history in the same list |
| **C4** | **Grok individual** onto the same path (retire the Grok-only host path); device-code login in the system browser | Grok real-turn scenario green |
| **C5** | **Claude individual**; **Gemini individual** (transport probe first) | Real-turn scenarios per provider |
| **C6** | Route awareness, `/route /model /routines` with guided values, popup and task windows | Pilot scenarios per command; window minimize/restore proof |
| **C7** | Platform reach: platform-wide activation, Assist blanket acknowledgement, chat content actions (Q5) | A non-chat surface runs a turn; content action round-trip proof |
| **C8** | Grok image/video mode + drafts + 30-day flush consent (R4; Q3 decides the API key) | Draft previewed, kept or ignored; flush prompt proven |
| **C9** | Clyffy house session (R16) | Active after the first subscription goes live; provider switchable |
| **C10** | Council (R25), brainstorm (R23), Clyffy drive with gates (R24) | Each gets its own plan when reached |

### 4.8 Agent-authored items NOT carried as requirements (challenge or confirm)

From `chat-window-product-vision.md` (Codex-authored, "operator-locked" with
no quote):
- two-column-only layout and the 1040×640 / 720×560 sizes;
- the AI/Social mode strip;
- the "ColdLight Ember" palette;
- the 18-kind Interaction Library;
- snap/detach utility windows;
- the CHAT-0…9 order;
- "fifty byte-identical captures".

From `chat-architecture.md`:
- 15 "locked" decisions;
- "triggers may never send/navigate" (in tension with R8/R5);
- Clyffy as an adapter (contradicts R16).

Others:
- `/strict` and `/behavioral` commands (the operator named `/route /model
  /routines`);
- the two-button approval overlay;
- bake-off popups;
- the "no-dialogs ruling" (no source);
- the subscription-only "law" (derived, not quoted);
- **Cursor and Antigravity** as providers (no operator mandate).

### 4.9 Decisions the operator must make before C2

| Q | Question | Recommendation |
|---|---|---|
| Q1 | Providers and order. You named **Grok, ChatGPT, Claude, Gemini**. Cursor/Antigravity have no mandate: drop them? | Grok + ChatGPT first (proven transports), then Claude, then Gemini. Drop Cursor/Antigravity until you ask. |
| Q2 | A connected provider's sessions: **its own tab** (07-29) or a **sidebar that extends down** with general + per-project sessions (08-03, on the unmerged branch)? | Mock both in C1; you pick on the mock. |
| Q3 | API keys: subscription-only, or API keys too? (ADR-011 already uses `XAI_API_KEY` for imagine; 08-03 unmerged: *"the API all stays"*) | Subscriptions for chat; the xAI key only for R4 image/video; nothing else until you say. |
| Q4 | Clyffy: confirm R16 as the whole spec for now (no fine-tune, no group-chat presence)? | Yes; C9. |
| Q5 | Chat content actions under Assist: blanket, or a tap on destructive/irreversible ones? | Blanket for reversible; a tap for destructive (R24). |
| Q6 | The Grok agent asking to run programs or write files (ACP terminal/file): allow, ask each time, or off? | Ask each time, jailed to the project root, never PATH-anything without approval. |
| Q7 | Provider working folder (today = wherever the app was launched). | An explicit project root chosen in chat ("general" vs per-project), never CWD. |
| Q8 | Store format. | Append-only JSONL + small index. |
| Q9 | Accept a Win32 binding for the Job Object (no-orphan guarantee)? | Yes; `windows-sys`, one module in `ai/host`. |
| Q10 | Pass order §4.7. | As listed. |
| Q11 | Existing code (§5.1): rewrite vs absorb. Laws on record: *"why delete why not work and get this built correctly"* (`crate-consolidation-review.md:194-195`). | Proven leaves are **absorbed by name** into the new crates; monoliths are rewritten; dead crates are retired (or absorbed if you want their idea). Confirm per row. |
| Q12 | `chat-window-product-vision.md` / `chat-architecture.md` stop being canonical (AGENTS.md:26-29 points at them)? | Yes; §4.1 replaces them. |

## 5. Wiring

### 5.1 Disposition of today's code — every element, no silent drops (confirm per row)

| Today | → | Disposition |
|---|---|---|
| `chat/core` (identities, capabilities, events, slash list) | `ai/core` + `chat/commands` | Absorb: provider-neutral parts → ai/core; commands → chat/commands |
| `chat/runtime` + `seam.rs` (`ProviderRuntime`, `qualified_capabilities`) | `ai/core` / `ai/host` | Absorb: the seam is the one contract |
| `providers/codex` (app-server transport, activation, env allowlist, history decoder, usage, schema-pin tests) | `ai/providers/codex` | Absorb and fix: version gate, server-request answering, turn timeout, depot presence |
| `providers/grok` (ACP runtime, retry, terminal, device-code) | `ai/providers/grok` + `ai/host` (retry) | Absorb onto the one path; delete the Grok-only host path and dead `GrokRuntimeSeam` |
| `providers/claude` (history + usage decoders) | `ai/providers/claude` | Absorb; runtime built in C5 |
| `providers/gemini` (descriptor) | `ai/providers/gemini` | Rebuilt in C5 |
| `providers/cursor` (descriptor) | — | Retire per Q1 |
| `chat/store` (read-only vendor history) | `chat/store` | Absorb as the vendor-index half; the own-store half is new |
| `chat/window` (6,959 lines) | `chat/window` (modules) | **Rewrite** to the mock; reuse paint helpers by name where they match |
| `chat/windows` (peer lifetime registry) | `chat/window` | Absorb |
| `chat/interactions` | `chat/window` | Absorb if the mock uses it; otherwise retire |
| `chat/browser-auth` | `ai/host` | Absorb (system browser for sign-in, R13) |
| `chat/accounts`, `chat/profiles` (Turso) | `chat/store` / `ai/core` | Decided by Q8; activation evidence becomes the activation record |
| `chat/context` (route context, half-used) | `chat/model` | Absorb (R3) |
| `chat/imagine` (xAI REST) | `ai/providers/grok` (media) | Absorb in C8 per Q3 |
| `chat/social` (no transport) | — | Park (R19: later) |
| `chat/extensions`, `chat/renderers` (no dependents) | — | Retire, or absorb the idea if you want it |
| `chat/testkit` | `chat/*` test support | Absorb |
| `app/src/chat.rs` (5,310), `presence.rs`, `ai_usage.rs`, `ai_backfill.rs` | `ai/host` + thin app wiring | Rewrite; the backfill `HOME`-only bug fixed |
| `shell/src/providers.rs` (roster UI) | stays in shell | Rebuilt to R15 in its own mock stage |

### 5.2 Edges

- New: `ai/*` ← `dpl-proc`, `dpl-tools-deps`, `dpl-fabric`.
- New: `chat/*` ← `ai/core`, `dpl-hub`, `dpl-coldlight`.
- Unchanged: app → `chat/window`, `ai/host`.
- No crate outside `ai/providers/*` names a vendor.
- Architecture.md and the xtask structure audit are amended in the same
  commit as each new crate.

## 6. Proof (every pass)

- `cargo green` locally, then **both** CI lanes complete and green before
  merge (`gh pr checks <n> --watch`).
- A **real-turn pilot scenario per provider**: send, then assert that the
  assistant text arrives. This is the first time this proof will exist.
- **Persistence proof:** quit, relaunch, and the conversation is there.
- **Supervisor proof:** after app exit, zero orphaned provider processes
  (process list before and after).
- Proof shots at **real window sizes** (`--window <w>x<h>`), never only
  maximized. Comparison against the approved mock.
- `target\release\devpulselabs.exe` rebuilt here and its mtime checked;
  `docs/STATE.md` updated.

## 7. Risks

- **No depot on this machine.** C0 is blocked until the Windows payloads'
  source is known (external-reach row 46).
- **Vendor protocol churn.** `codex app-server` is an alpha surface. The
  schema-pin tests stay and are re-pinned per version bump.
- **Gemini multi-turn may not exist** headless. C5 probes it first and says
  so plainly if it is one-shot.
- **Experience regression law.** Anything the current window does that the
  new one does not yet do is listed in each pass plan. Nothing disappears
  silently.
- **Parallel lanes.** `gh pr list` before each pass; adopt, don't duplicate.
- **Scale.** This is a multi-week program. It is cut into passes so each
  lands green and usable.

## 8. Evidence

- 2026-10-10: four read-only sweeps (current chat code; operator chat
  requirements; code external reach; docs/CI external reach).
- In-session checks: debug build on Windows (compiles); clippy fails on the
  uncommitted `process_manager.rs:83`; structure audit passes; every
  verbatim quote in §4.1 (R1–R27) and the §4.9 Q11 law re-read at its cited
  line; `dpl-lottie-render` with no args exits 1 with its usage text.
- Filled per pass.
