# Codex conversation transport — ChatGPT you can actually talk to

Branch: `feat/codex-transport` off `main` after #206 and #207 land.
Seam: `dpl_chat_runtime::seam::ProviderRuntime` (landed in #207).

## 1. Context

The operator, first message of the session and repeatedly since:

> "if i have chatGPT active already it should clearly show its active and i
> can chat"

It shows active. You cannot chat. That has been true the whole time and it
is the oldest open ask in the program.

The app already *knows* the operator is signed in — boot logs
`Codex activation evidence state=Ready version=0.145.0 method=chatgpt`, and
A1's probe reports `SignedIn`. What it has is a doorbell, not a phone line:

```
grok/src/    lib.rs  runtime.rs  retry.rs  terminal.rs   ← ACP transport
codex/src/   lib.rs  activation.rs                       ← login detection only
```

Nothing anywhere qualifies `StreamTurns` for Codex, so `can_send()` is false
and the composer correctly refuses. Honest, and useless.

**This pass gives Codex a runtime that implements the seam, so "ChatGPT is
active" means the operator can talk to it.**

## 2. Current state

Ground-truthed in-session, 2026-08-04, on the operator's own machine.

### 2.1 The seam is built and Grok already wears it

`crates/chat/runtime/src/seam.rs` (#207): `ProviderRuntime` —
`provider` · `try_send(RuntimeCommand)` · `try_event() -> Option<RuntimeEvent>`
· `capabilities()`. A runtime is a CHANNEL, never a blocking call.
`grok/src/seam.rs` wraps `GrokAcpRuntime` by translation.

`seam::qualified_capabilities(RuntimeCapabilities)` is the ONE function that
turns what a provider advertised into what it earned. Codex qualifying
`StreamTurns` therefore requires no new capability logic — only a runtime
that connects.

### 2.2 Codex does NOT speak ACP — measured, not assumed

`codex --help` on this machine (`codex-cli 0.146.0-alpha.3.1`,
`C:\Users\jessa\.local\bin\codex.cmd`) lists no `acp` subcommand. Copying
Grok's ACP client would not have worked. The two candidates are:

| subcommand | what it is | fit |
|---|---|---|
| `codex app-server` | JSON-RPC over stdio — what the IDE integrations drive | **yes** |
| `codex mcp-server` | exposes Codex as a TOOL to another agent | wrong direction |

`mcp-server` is rejected on direction: it makes Codex a tool the operator's
agent calls, not a conversation the operator has.

### 2.3 The protocol publishes its own schema

`codex app-server generate-json-schema --out <dir>` emits **39 JSON Schema
files**, including `ClientRequest`, `ServerNotification`, `ServerRequest`.
Measured surface: **89 callable methods · 70 notifications · 10 server→client
requests**.

This is the strongest ground truth available for a wire protocol — the
contract is machine-readable and versioned, so the adapter can be pinned
against it rather than written from a blog post.

### 2.4 Every seam variant has a Codex counterpart

The mapping fell out of the schema with no gaps:

| seam | Codex app-server |
|---|---|
| `Connected { capabilities }` | `initialize` |
| `NewSession` | `thread/start` → `thread/started` |
| `LoadSession` | `thread/resume` |
| `ListSessions` | `thread/list` |
| `Prompt` | `turn/start` → `turn/started` |
| `Cancel` | `turn/interrupt` |
| `SessionUpdate(AssistantText)` | `item/agentMessage/delta` |
| `SessionUpdate(ThoughtText)` | `item/reasoning/textDelta` · `…/summaryTextDelta` |
| `SessionUpdate(Plan)` | `turn/plan/updated` · `item/plan/delta` |
| `SessionUpdate(Tool)` | `item/commandExecution/*` · `item/mcpToolCall/progress` |
| `SessionUpdate(Usage)` | `thread/tokenUsage/updated` |
| `TurnFinished` | `turn/completed` |
| `PermissionRequested` | `item/commandExecution/requestApproval` (ServerRequest) |
| `ResolvePermission` | that request's response |
| `Failure` | `error` · `warning` |
| `Stopped` | process exit |

**This is the pass's most important finding.** The neutral vocabulary was
designed from Grok/ACP and maps cleanly onto a completely unrelated protocol
with nothing left over and nothing missing. That is the test an abstraction
either survives or fails, and it survived — so the seam is confirmed by use
rather than by assertion.

Two methods land free for later work: `account/usage/read` and
`account/rateLimits/read` are exactly what the group-chat billing popup
(program map §8.3) needs.

### 2.5 What Grok's runtime already establishes as house shape

`grok/src/runtime.rs` (1927 lines): `async-process` child, `async-channel`
command/event queues, a worker task, `retry.rs` for backoff, `terminal.rs`
for the process. Deps: `async-channel`, `async-io`, `async-process`,
`futures-lite`, `serde_json`, `dpl-proc`, `agent-client-protocol`.

Codex needs the same minus ACP: the frames are plain JSON-RPC, so
`serde_json` covers the wire.

### 2.6 The activation coordinator already gives us the executable

`CodexRuntimeProfile { executable, version, working_directory }`
(`codex/src/activation.rs:69`), discovered and version-gated. The runtime
does not re-discover anything.

## 3. Target design

### 3.1 `CodexAppServerRuntime` — one more spoke, no new crate

`crates/chat/providers/codex/src/runtime.rs`, mirroring Grok's file layout so
the two read the same. It implements `ProviderRuntime` directly (Grok needed
an adapter only because its own vocabulary predated the seam; Codex has no
legacy vocabulary to translate).

**No new crate. No new boundary.** This aligns to the existing provider-crate
pattern — `providers/<name>/{lib,activation,runtime}.rs`.

### 3.2 Transport

`codex app-server` spawned via `dpl-proc`'s quiet-spawn (spawn-audit law),
stdio JSON-RPC, one worker task, bounded command and event channels. The UI
thread only ever calls `try_send` / `try_event`.

### 3.3 Half-implemented is the plan, and it is scoped honestly

Operator: *"i dont care if its half implemented i want the bleeding edge
pattern."* So this pass takes the conversation spine and **declares the rest
unqualified rather than faking it**:

**In:** `initialize` · `thread/start` · `turn/start` · `item/agentMessage/delta`
· `item/reasoning/*Delta` · `turn/completed` · `turn/interrupt` ·
`thread/tokenUsage/updated` · `error`.

**Out, and NOT advertised:** approvals (the 10 ServerRequests), `thread/list`,
`thread/resume`, `turn/steer`, MCP, review, plans, file-change items.

Because `qualified_capabilities()` derives from what the runtime reports,
declining to advertise means the surface will not offer it. A capability the
adapter cannot honour is never claimed — that is the seam's guarantee, and
this pass is its first real exercise.

### 3.4 The experimental flag is stated, not hidden

`app-server` is marked `[experimental]` by the vendor. The runtime records
the CLI version it connected to and surfaces it. If `initialize` fails or the
schema shape is unrecognised, the provider stays unqualified with a truthful
reason — never a silent downgrade.

### 3.5 The schema is pinned

The generated `ClientRequest`/`ServerNotification` schemas are vendored under
`crates/chat/providers/codex/schema/` with the CLI version they came from,
and a test asserts the methods this adapter depends on still exist. When
Codex moves, the test fails with the method name rather than a runtime
mystery.

This is the "one clear type" discipline applied to an inbound protocol: we do
not trust a wire, we pin it.

## 4. Changes, in execution order

1. `providers/codex/Cargo.toml` — `async-channel`, `async-io`,
   `async-process`, `futures-lite`, `dpl-chat-runtime`.
2. `providers/codex/schema/` — vendored `ClientRequest.json`,
   `ServerNotification.json` + `VERSION`.
3. `providers/codex/src/runtime.rs` — the child, the worker, the frames.
4. …the `ProviderRuntime` impl + the notification→`RuntimeEvent` mapping.
5. `providers/codex/src/lib.rs` — `pub mod runtime;`.
6. `app/src/chat.rs` — start the runtime on activation and drive it through
   the SEAM, not a Codex-shaped branch. Grok's bespoke polling is left alone
   in this pass; converging it is its own step (§5).
7. Unit pins (§6.3).
8. `tests/eguipilotai/codex-turn.toml` — the live proof.

## 5. Boundaries

🚫 **Never touch**

- Grok's runtime. It works. Converging `poll_grok_runtime` onto the seam is a
  SEPARATE pass, after Codex proves the seam carries two providers.
- The activation coordinator and the probes. This pass consumes them.
- Credentials. The runtime never reads a token; `codex` owns its own auth,
  and `account/chatgptAuthTokens/refresh` is a ServerRequest we do NOT
  implement in this pass.
- `mcp-server` (§2.2, wrong direction).

⚠️ **Ask first**

- Anything that would make Codex's capability set differ from what
  `initialize` actually reports.
- Implementing approvals — it is a real interaction design, not a mapping.

**Out of scope:** approvals · session list/resume · steer · billing surfaces
(the methods exist; the popup is §8.3's pass) · Gemini · Cursor.

## 6. Verification

1. `cargo clean -p dpl-chat-codex -p dpl-chat-runtime`, THEN
   `cargo clippy --workspace --all-targets -- -D warnings`. The cold-clippy
   law: this session it caught four errors a warm run hid, twice.
2. New unit pins:
   - `an_unconnected_runtime_qualifies_nothing`
   - `a_notification_maps_to_exactly_one_runtime_event` — over recorded
     frames, not a live child.
   - `an_agent_message_delta_becomes_assistant_text`
   - `a_turn_completed_ends_the_turn`
   - `an_unrecognised_notification_is_kept_as_Other_not_dropped` — an unknown
     frame is evidence (seam law).
   - `the_pinned_schema_still_declares_every_method_we_call` — fails with the
     method name when Codex moves.
3. `cargo green` → `green ✔`; merge ONLY on double-green.
4. **The live proof, and the only one that matters:**
   `tests/eguipilotai/codex-turn.toml` against the LIVE fixture — select
   Codex, type, press Enter, and assert an assistant message arrives. The
   operator has said "it shows active and I cannot chat" since the first
   message; this scenario is that sentence, mechanised.
5. Proof shot at 1300×800 of a real ChatGPT reply in the pane.
6. Regressions: `chat-local-histories`, `providers-popup`, `chat-provider-tab`,
   and any Grok scenario — Grok must be untouched.

## 7. Commit plan

Branch `feat/codex-transport`. One commit:

```
feat(chat): Codex conversation transport — the second spoke on the seam
```

`gh pr list` first.

## 8. Evidence

Filled during execution. §6.4 is the measure: an assistant message from
ChatGPT, in the app, from a real operator keystroke.

**Recorded now, before any code:** the seam was designed against Grok/ACP and
mapped onto Codex's app-server with no gaps (§2.4). If that had NOT held,
this plan would have been an argument for changing the seam rather than for
using it.
