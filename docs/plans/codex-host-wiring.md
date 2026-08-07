# Chat reaches the AI provider crate — the seam gets a host

Branch: `feat/codex-host-wiring` off `main` after #210.
Completes: `docs/plans/codex-conversation-transport.md` §4 steps 1, 3, 6, 8.

## 1. Context

The operator, 2026-08-06:

> "We now need to ensure that chat is getting access to the crate for our AI
> providers"

#210 merged as `fe0e162`. It is a good commit and it is **half its own plan**.
Measured against `codex-conversation-transport.md` §4:

| step | what it was | shipped in #210 |
|---|---|---|
| 1 | transport deps in `providers/codex/Cargo.toml` | **no** — only `serde_json` |
| 2 | vendored `schema/` | yes |
| 3 | `runtime.rs` — **the child, the worker**, the frames | **frames only** |
| 4 | `ProviderRuntime` impl + notification mapping | yes |
| 5 | `lib.rs` — `pub mod runtime;` | yes |
| 6 | `app/src/chat.rs` — start on activation, drive the SEAM | **no** |
| 7 | unit pins | yes (27 tests) |
| 8 | `codex-turn.toml` live proof | **no** |

The proof is mechanical: `CodexAppServerRuntime::from_parts`,
`APP_SERVER_ARGS` and `initialize_params()` have **zero callers** anywhere in
the workspace. The type's own doc says why — *"The child and worker are
assembled by the host"* — and the host was never written.

So Codex today is a **pure, well-tested translation table that nothing can
run**. `dpl-chat-codex` is already in `crates/app/Cargo.toml:52`, so "access"
at the manifest level exists; what does not exist is a process, a worker, and
a single line of host code that drives the seam.

This pass writes those three things.

## 2. Current state

Ground-truthed in-session 2026-08-06 by reading each file in full.

### 2.1 The seam is complete and unused by any host

`crates/chat/runtime/src/seam.rs:231` — `ProviderRuntime` is
`provider`/`try_send`/`try_event`/`capabilities`. `qualified_capabilities()`
(`:254`) derives the chat capability set from what the provider advertised.
Nothing in `crates/app` names `ProviderRuntime`.

### 2.2 The host is Grok-shaped, not provider-shaped

`crates/app/src/chat.rs:106` — `grok_runtime: Option<dpl_chat_grok::GrokAcpRuntime>`.
A concrete type, not the seam. Alongside it: `grok_runtime_retry`,
`grok_runtime_ready`, `grok_sessions`, `grok_turns`, `grok_permissions`
(`:107`–`:112`). **There is no Codex slot of any kind.**

Every operator intent is gated on Grok and returns otherwise:

- `:1386` `NewSession` — `if provider != grok::PROVIDER { return }`
- `:1399` `SendTurn` — same
- `:1408` `CancelTurn` — same
- `:1431` `ResolveProviderPermission` — same

And `provider_session_id()` (`:2867`) resolves the binding through
`provider_view(dpl_chat_grok::PROVIDER)` — hard-coded, so it answers for Grok
no matter which provider asked.

The projection functions — `poll_grok_runtime` (`:2506`),
`project_grok_update` (`:3024`), `append_grok_event` (`:3124`),
`set_grok_session_idle` (`:3143`), `grok_chat_session_id` (`:2855`) — are
Grok-**named** but structurally provider-neutral: each takes a
`provider_session_id`, looks up a `ChatSessionId`, and calls a
`window.*_provider_*` method that already takes a `ProviderId` argument.

### 2.3 The AI→fabric bridge is already provider-generic

`ai_call_started` (`:2900`) takes `provider: ProviderId`. `ai_first_token`,
`ai_call_finished`, `ai_usage_observed` key off `provider_session_id`. Its
header states the intent outright: *"Provider crates stay fabric-free; the app
owns the translation."* This pass adds a second producer, not a second bridge.

### 2.4 Grok's transport is the shape to mirror, not to copy

`grok/src/runtime.rs:315` — `start(profile)` builds bounded channels, spawns a
named thread, and blocks on an async worker. It uses `async-process`,
`async-channel`, `async-io`, `futures-lite` (`grok/Cargo.toml`).

**Codex must not take those deps.** `CodexAppServerRuntime::from_parts`
already commits to `std::sync::mpsc::{SyncSender, Receiver}`
(`runtime.rs:232`), the frames are newline-delimited JSON rather than ACP, and
blocking reads on a dedicated thread are sufficient. std + the `serde_json`
already present covers it — **zero new dependencies**.

### 2.5 The activation coordinator already owns the executable

`codex/src/activation.rs:69` — `CodexRuntimeProfile { executable, version,
working_directory }`, with `runtime_profile()` at `:338`. The runtime
re-discovers nothing. `profiled_command` (`:619`) is the existing quiet-spawn
+ `env_clear` + allow-list pattern.

### 2.6 Four protocol facts, measured live on this machine

Proved end to end before #210 (`ASSISTANT ANSWER: 'OK'` from a real turn):

1. **stdin must stay OPEN.** Closing it after the handshake killed six
   consecutive probe attempts. This is the single most expensive fact here.
2. **`initialized` is mandatory** — a notification, no id, sent after the
   `initialize` response.
3. **`thread/started` carries the id on the `thread` object**, not at the top
   level (already handled at `runtime.rs:134`).
4. **Responses omit `jsonrpc`** — a frame is a response iff it has `id` and no
   `method`.

### 2.7 The quiet-spawn law applies

`dpl-proc::quiet` (`crates/proc/src/lib.rs:16`) is already a dependency of
`dpl-chat-codex`. The `cargo green` spawn audit fails any file containing
`Command::new` that does not import it.

## 3. Target design

### 3.1 `CodexAppServerRuntime::start(profile)` — the missing half

Added to `providers/codex/src/runtime.rs` beside the mapping it drives:

- `profiled_command`-style child: `codex app-server`, quiet-spawned, piped
  stdio, `current_dir` from the profile.
- **A reader thread** — `BufRead::lines()` over stdout; each line parsed and
  routed: `method` present → `event_for()` → the event channel; `id` present
  and no `method` → a response (only `initialize`'s is acted on).
- **A writer thread** — drains the bounded `Outbound` queue, stamps a
  monotonic JSON-RPC id, writes the line, flushes. **Holds stdin for its
  entire life** (§2.6.1), releasing it only on `Shutdown`.
- The handshake runs on the writer thread before the queue is served:
  `initialize` → await its response → `initialized` → emit
  `RuntimeEvent::Connected { advertised_capabilities() }`.
- A stderr drain thread, so a blocked pipe cannot wedge the child.

`from_parts` stays exactly as it is — it is what keeps the mapping testable
without a process, and its 27 tests depend on it.

### 3.2 The host drives the SEAM, not a second Codex-shaped branch

New in `Shared`:

```rust
/// Live provider runtimes, keyed by provider and driven through ONE path.
/// Grok is deliberately absent — it keeps its bespoke runtime this pass.
runtimes: BTreeMap<ProviderId, Box<dyn ProviderRuntime>>,
seam_sessions: BTreeMap<(ProviderId, String), ChatSessionId>,
seam_turns: BTreeMap<(ProviderId, String), TurnId>,
```

Keyed by `(ProviderId, provider_session_id)` because two providers can and
will mint the same session string, and a collision would cross two operators'
conversations into one transcript.

Five generic functions mirror the Grok five, provider-parameterised:
`poll_seam_runtimes` · `project_seam_update` · `append_seam_event` ·
`set_seam_session_idle` · `seam_chat_session_id`.

**Why generic rather than a Codex copy:** this is the seam's stated purpose —
*"the host then drives every provider through one code path and emits ONE set
of fabric signals"* (`seam.rs:17`). Cursor and Gemini then cost a `start()`
each and no host code. A Codex-shaped copy would make the third provider a
third copy, which is the exact failure the seam was built to prevent.

### 3.3 The intent gates become capability questions

`if provider != grok::PROVIDER { return }` becomes: Grok → the Grok path; a
provider with a live seam runtime → the seam path; neither → the existing
honest warning. The gate stops asking *"are you Grok?"* and starts asking
*"do you have a runtime?"*.

`provider_session_id()` (§2.2) gains a `ProviderId` parameter — its current
hard-coded Grok lookup is a latent defect the moment a second provider has
sessions, and this pass is what makes that moment arrive.

### 3.4 Capability is still earned, never assumed

On `Connected`, the host calls `qualified_capabilities(capabilities)` and
projects the result. Codex advertises `load_session: false, list_sessions:
false`, so it qualifies `StartSession` + `StreamTurns` + `InterruptTurn` and
**not** `ReadSessions`/`ResumeSession`. The composer opens; the session list
does not appear. No hard-coded list, and no Codex special case.

### 3.5 What stays honest

`PINNED_CLI_VERSION` is `0.146.0-alpha.3.1` (the PATH binary) while the
activation coordinator discovers the depot's `0.145.0`. **This pass does not
resolve that** — the passthrough resolver is its own pass. What this pass does
is make it visible: the runtime records the version it actually connected to
and a mismatch with the pin is logged, so the two-binaries problem shows up in
a log line instead of a mystery.

## 4. Changes, in execution order

1. `providers/codex/src/runtime.rs` — `start(profile)`, the three threads, the
   handshake. No new deps.
2. `providers/codex/src/runtime.rs` — unit pins for the frame reader (§6.2).
3. `app/src/chat.rs` — the three `Shared` fields (§3.2) + their initialisers.
4. `app/src/chat.rs` — the five generic seam functions.
5. `app/src/chat.rs` — start the Codex runtime when activation reports Ready.
6. `app/src/chat.rs` — the four intent gates + `provider_session_id`'s
   `ProviderId` parameter.
7. `app/src/chat.rs` — call `poll_seam_runtimes` from the frame loop beside
   `poll_grok_runtime`.

## 5. Boundaries

🚫 **Never touch**

- **Grok's runtime or its host path.** It works. Converging it onto the seam
  is a separate pass, after Codex proves the seam carries two providers —
  the same boundary `codex-conversation-transport.md` §5 set.
- The activation coordinator and the probes. This pass consumes them.
- Credentials. The runtime never reads a token; `codex` owns its own auth.
- The `chat/` → `ai/` rename (`ai-chat-social-separation.md`). It renames
  these very files; doing both at once makes the diff unreviewable.

⚠️ **Ask first**

- Advertising any capability `outbound_for` returns `None` for.
- Approvals — Codex's 10 ServerRequests are an interaction design, not a
  mapping.

**Out of scope:** the passthrough/PATHEXT resolver · `thread/list` ·
`thread/resume` · Gemini · Cursor · Claude.

## 6. Verification

1. `cargo clean -p dpl-chat-codex -p dpl-chat-runtime -p devpulselabs`, THEN
   `cargo clippy --workspace --all-targets -- -D warnings`. Cold, per the
   cold-clippy law — a warm gate hid four errors twice this session.
2. New unit pins beside the existing 27:
   - `a_response_frame_is_told_from_a_notification` — `id` and no `method`
     (§2.6.4).
   - `the_handshake_sends_initialized_after_initialize`.
   - `a_connected_event_carries_the_advertised_capabilities`.
3. `cargo green` → `green ✔`. Includes the spawn audit (§2.7).
4. **The live proof, and the only one that matters:** launch the exe, select
   ChatGPT, type, press Enter, and see a reply. The operator has said "it
   shows active and I cannot chat" since the first message of this program;
   this is that sentence, falsified.
5. Proof shot at 1300×800 of a real ChatGPT reply in the pane.
6. Regressions — Grok must be untouched: `chat-local-histories` ·
   `chat-content-spine` · `chat-provider-tab` · `providers-popup`.

## 7. Commit plan

Branch `feat/codex-host-wiring`. One commit:

```
feat(chat): the host drives the seam — ChatGPT answers in the app
```

`gh pr list` first; merge ONLY on double-green.

## 8. Evidence

Filled during execution. §6.4 is the measure: an assistant message from
ChatGPT, in the app, from a real operator keystroke.

**Recorded before any code:** #210's `from_parts` had zero callers, which is
why "Codex transport landed" and "the operator can chat with ChatGPT" were
both true and both unrelated. The lesson is §2's: a merged PR is not a
delivered capability, and the check that catches it is *does anything call
this?* — not *did it pass CI?*
