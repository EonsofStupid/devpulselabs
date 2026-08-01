# P1 — the provider capability registry

Governed by `docs/design/adr-017-provider-capability-registry.md`.
Activation cross-links into the platform's configuration stores, and A2UI,
workflows / mixture-of-experts routing, A/B duplicate runs, and Clyffy's
training capture all bind to its shape — which is why the registry carries a
capability SET.

The mock is `docs/design/mocks/providers.html`. This pass builds the registry
and its three projections; the surface it feeds is V1–V3 of
`docs/plans/providers-presence-and-the-brain.md`.


Program map: `docs/plans/chat-operations-console.md` (link P1).
Branch: `feat/p1-provider-availability` off `main` after #180 lands.

## 1. Context

The operator, 2026-07-30:

> "chat needs to be built so its properly yes its own window but the
> providers that get activated in the chat become available in the
> platform"

H1 delivered the first half: a provider whose transcripts are on this
machine is now genuinely activated (`Experimental` + `ReadSessions`/
`PageHistory`), earns a tab, and its chats are rows. **This pass delivers
the second half — making that activation legible to the rest of the app.**

The distinction that matters, and the reason this is not a one-liner:
today the platform can see that *something happened* to a provider. It
cannot see what any provider can *do*.

## 2. Current state

Ground-truthed in-session against `989e145` (H1's tip).

### 2.1 Chat owns the runtime, and it is a SURFACE

`ChatRuntime` is a private field of `ChatWindow`
(`crates/chat/window/src/lib.rs:484`). `ChatWindowCoordinator`'s entire
public surface is `new`, `attach_determinism`, `attach_pilot_runtime`,
`consume_pilot_close_requests`, `root_close_requested`, `frame`. There is
no accessor by which anything outside can ask what is registered.

A workspace grep for `ActivationState` / `Capability::` outside
`crates/chat/` returns only `app/src/chat.rs` and `app/src/main.rs` — the
chat host itself. **Zero platform surfaces can see a provider.**

### 2.2 The fabric's provider channel is a LIFECYCLE stream, not a roster

`ProviderEvent` (`crates/fabric/src/lib.rs:248`) is fourteen variants of
one activation attempt's progress — `ActivationRequested`,
`DeviceCodeReceived`, `RuntimeStarted`, `Ready { attempt, client_version,
auth_method }`, `Failed { stage, class, retryable }`, `Cancelled`,
`Expired`. Every variant except `Selected` and `ProfileConfigured` carries
an `ActivationAttemptId`.

Two consequences:

1. **A history-activated provider never appears on it at all.** H1's
   providers activate by having transcripts on disk; no attempt runs, so
   no `Ready` is ever published. The channel cannot answer the operator's
   question by construction.
2. **A broadcast stream has no past.** `subscribe_providers()` returns a
   `broadcast::Receiver`, so a surface that subscribes late hears nothing
   about providers that activated before it existed. A roster question
   ("what is available *now*") cannot be answered by replaying a stream
   nobody kept.

### 2.3 Six subscribers already exist, and none of them learns a capability

`app/src/chat.rs:549`, `app/src/coach_signals.rs:53`, `app/src/main.rs:669`,
`crates/ledger/src/lib.rs:105`, `crates/seam/src/lib.rs:800`,
`crates/shell/src/lib.rs:251`.

The shell's is the telling one (`shell/src/lib.rs:488`):

```rust
        while let Ok(st) = self.providers_rx.try_recv() {
            self.signal_count += 1;
            self.push_activity(st.ms, activity_provider_line(&st.ev));
        }
```

It turns the event into **a line of text in an activity feed**. That is
telemetry, and it is the whole of the platform's relationship with
providers today.

### 2.4 The fabric already has the exact pattern this needs

THE JOBBOARD (`fabric/src/lib.rs:476`):

```rust
    /// THE JOBBOARD — the fold of the jobs stream (+ each row's latest-wins
    /// progress counter), shared by every Fabric clone.
    board: Arc<std::sync::Mutex<Vec<JobRow>>>,
```

`publish_job` folds each event into the board **before** notifying, so
that (`:571`) "the consumer reads a board that is already true". `JobRow`
is plain data (`:154`). Every `Fabric` clone shares the `Arc`, so any
surface holding a fabric can read the fold without having heard the
stream.

**P1 is that pattern applied to providers.** No new mechanism is invented,
exactly as D1 and H1 invented none.

## 3. Target design

### 3.1 A roster of what each provider can DO

New plain data in `dpl-fabric` beside `JobRow`:

```rust
/// What one provider can do for the platform, right now.
///
/// A capability SET, not a pair of booleans: workflows route a `StageKind`
/// to whoever can take it, A/B runs name every provider that can take the
/// SAME beat, and training capture needs a stable subject. None of those
/// can be expressed by `can_send` (ADR-017 §4).
pub struct ProviderCapability {
    pub id: ProviderId,
    pub label: String,
    /// Activated at all — H1's `Experimental` counts, and carries a reason.
    pub activated: bool,
    /// The truthful limitation when activated but partial.
    pub note: Option<String>,
    /// The qualified set, in `dpl_chat_core`'s existing vocabulary.
    pub capabilities: BTreeSet<Capability>,
}

impl ProviderCapability {
    /// Derived, never stored — the booleans are a reading of the set.
    pub fn can(&self, capability: Capability) -> bool {
        self.activated && self.capabilities.contains(&capability)
    }
}
```

The set is the stored form; booleans are derived. Storing the reading
instead of the fact is what made the first draft unable to serve routing.
`Capability` is carried as-is from `dpl-chat-core` — the vocabulary is
already provider-neutral plain data and mirroring it would create exactly
the drift a second registry causes. `ActivationState` is deliberately NOT
carried: it is a chat LIFECYCLE type with fourteen shades a platform
consumer has no use for, so it flattens to `activated: bool` plus the
truthful `note`. Carry the fact, not the state machine.

`dpl-fabric` already imports `ProviderId` from `dpl-chat-core`
(`fabric/src/lib.rs:12`), so `Capability` crosses on the same edge.

**Verified against the enforced law:** `docs/design/architecture.md:65`
carries a RATIFIED named exception — *"dpl-fabric → dpl-hub +
dpl-chat-core + dpl-intent: facts carry Route, provider types, and the
intent gate's mode/cause/channel. The fabric is a side rail, not a layer
violation."* So P1 adds **no new dependency edge**, and the `structure`
phase of `cargo green` (`crates/xtask/src/main.rs`, `structure_audit`)
has nothing to fire on. Neither the doc nor the audit table needs
amending in this pass — which is the test of whether a design fits the
stack rather than bending it.

### 3.2 The roster is a fold, held by the fabric

```rust
    roster: Arc<std::sync::Mutex<Vec<ProviderCapability>>>,
```

- `Fabric::publish_provider_roster(Vec<ProviderCapability>)` — replaces
  the fold wholesale (the chat host always knows the complete truth, so
  there is no partial-update case to get wrong) and then notifies.
- `Fabric::provider_roster() -> Vec<ProviderCapability>` — the read any
  surface can make with no subscription and no history.

Wholesale replacement rather than per-row folding is the honest choice
here: unlike jobs, which arrive independently, the provider set is one
consistent snapshot minted from one registry.

### 3.3 The chat host publishes it; the surface still never does IO

`ChatWindowCoordinator` publishes the roster whenever the provider views
could have changed — at construction (H1's trigger-0 lesson: `--chat-open`
raises no request) and after any `apply_host_intent` that touches
activation, sign-out, or session state.

The window gains ONE accessor, `provider_capabilities()`, returning the
plain-data vector built from `self.runtime.provider_views()`. This does
not leak the runtime and does not make the surface do IO — it is the same
shape as `content_episode_ids()`, which D1 established.

### 3.3b It persists, and agents can see it (ADR-017 §2–3)

Both land WITH P1 because both are cheap now and expensive to retrofit,
and because the four consumers assume them:

- **Persistence.** `Tree::Agents` already exists in the estate
  (`crates/estate/src/lib.rs:35`, *"Agent (chat provider) session state for
  the HTTP seam"*). The host writes the roster with `put_json` on change
  and reads it at boot, so a provider that was active last launch is known
  BEFORE any probe runs — the UI starts truthful instead of empty and
  correcting itself a second later. Live process handles are never
  persisted; only what was true and is cheap to re-verify.
- **Seam projection.** `crates/seam/src/lib.rs:72` already exists to serve
  *"the registry as wire JSON — what an agent reads to learn the map."* The
  capability registry joins that map. An agent that cannot see which
  experts exist cannot route to them.

### 3.4 The first consumer, landed in the same pass

**The shell's status area names what the platform can do.** A chip reading
`AI · Codex, Claude` with a tooltip naming each one's limit ("Claude Code
— local transcripts only"), and nothing at all when the roster is empty.

This is deliberately the smallest honest consumer:

- It proves the roster reaches a surface that is NOT chat.
- It is read-only, so it cannot corrupt anything (D1's discipline: prove
  the model by use before any verb depends on it).
- It answers the operator's own question at a glance.

A publication with no consumer proves nothing, which is why the consumer
is not deferred to a later pass.

### 3.5 What this deliberately does NOT do

It does not let a hub *send* to a provider, and it does not route. Those
are CONSUMERS (ADR-017 §6) and they wait until the shape is proved by use —
the same reason D1 was read-only before D3 could edit.

It also holds no credential, ever. The registry says "this provider can
stream turns"; it never holds a token, cookie, or key. Presence is probed
via the vendor's own tool.

## 4. Changes, in execution order

1. `crates/fabric/src/lib.rs` — `ProviderCapability` (capability SET),
   the `roster` field, `publish_provider_roster`, `provider_roster`.
2. `crates/chat/window/src/lib.rs` — `provider_capabilities()` accessor
   projecting `runtime.provider_views()` into the plain-data set.
3. `crates/app/src/chat.rs` — publish at construction and after every
   activation-affecting host intent; write `Tree::Agents`; seed the roster
   FROM `Tree::Agents` at boot before the first probe.
4. `crates/seam/src/lib.rs` — the roster joins the wire-JSON registry.
5. `crates/shell/src/lib.rs` — read `provider_roster()` and render the chip.
6. Unit pins (§6.3).
7. `tests/eguipilotai/platform-sees-providers.toml` — a LIVE-fixture
   scenario asserting the chip names Claude on this machine.

## 5. Boundaries

🚫 **Never touch**

- `dpl-fabric` must NOT gain a dependency on `dpl-chat-runtime` or
  `dpl-chat-window`. It sits under every surface; the roster is plain data
  precisely so the layer stack stays right. The gate's `structure` phase
  enforces this.
- The activation paths themselves. P1 REPORTS activation; it never
  changes who is activated.
- Transcripts, `/content` — untouched, as in H1.

⚠️ **Ask first**

- Any surface that would SEND to a provider (that is D5/D6).
- Any change that would make the roster writable from outside the chat
  host — one publisher is what keeps it truthful.

**Out of scope**

- Per-capability affordances in hubs.
- The control, the roster popup, and the modal — V1/V2/V3 of
  `docs/plans/providers-presence-and-the-brain.md`, against
  `docs/design/mocks/providers.html`.

The roster IS derived state and the live projection always wins; persistence
decides only what the platform shows before the first recompute lands, never
what is true.

## 6. Verification

1. `cargo test -p dpl-fabric -p dpl-shell -p dpl-chat-window -p devpulselabs`.
2. `cargo clean -p` the touched crates, THEN
   `cargo clippy --workspace --all-targets -- -D warnings` — H1's lesson:
   a warm clippy pass can report a cached verdict, and both hosted lanes
   went red on one.
3. New unit pins:
   - `an_empty_roster_reads_empty` — no publish, no panic, no chip.
   - `the_roster_is_readable_without_subscribing` — publish, then read
     from a fabric CLONE that never subscribed (the whole point of §2.2).
   - `a_history_only_provider_is_in_the_roster_but_cannot_send` —
     `can(PageHistory)` true, `can(StreamTurns)` false.
   - `the_roster_survives_a_restart` — write, reopen the estate, read back
     the same set (ADR-017 §2: truthful at boot, before any probe).
   - `an_unactivated_provider_can_nothing` — `can()` is false for every
     capability regardless of what is qualified.
   - `the_seam_projects_the_roster` — the wire JSON names the provider and
     its capability set.
   - `publishing_a_roster_replaces_the_previous_one` — no accumulation.
   - `the_shell_names_the_activated_providers`.
4. `cargo green` → `green ✔`, then MERGE ONLY ON DOUBLE-GREEN.
5. **The live proof:** `platform-sees-providers` against fixture `chat`
   asserts the shell chip names Claude — a provider activated ONLY by
   having transcripts on this machine. That is the operator's sentence,
   mechanised.
6. Proof shot of MAIN (not Chat) at a real window size showing the chip.
7. Regressions: `chat-local-histories`, `chat-content-spine`,
   `chat-provider-tab`, plus any shell/status scenario.

## 7. Commit plan

Branch `feat/p1-provider-availability`. One commit:

```
feat(platform): activated providers become platform capabilities (P1)
```

`gh pr list` first — #169 touches `crates/fabric/src/lib.rs` heavily
(+210 lines at `:444`, plus `Fabric` struct and `impl` hunks), so this
plan's §4.1 WILL conflict textually. Rebase on whichever lands first and
re-read the struct before editing.

## 8. Evidence

§6.5 is the measure: a surface outside the chat crates naming a provider that
exists only because the operator has transcripts on disk.

### Ground truth as found (2026-08-01, `2459cdf`)

The plan was ground-truthed against `989e145`; main had moved to `2459cdf`
(#180 D1+H1, #183 ADR-017, #184, #185, #189, #190). Every anchor in §2 and
§3 still held, with ~4 lines of drift: `board` at `fabric/src/lib.rs:480`
(plan said 476), `publish_job` at `:572` (571), `ProviderId` import at `:12`
as stated. **No design change was needed** — the fabric's JOBBOARD pattern
and the ratified `dpl-fabric → dpl-chat-core` exception are exactly as §3.1
described, so P1 added no dependency edge and the `structure` audit had
nothing to fire on.

### Two decisions the plan did not settle

1. **`ProviderId` is `&'static str`** (`chat/core/src/lib.rs:47`, via
   `static_id!`), so a persisted roster CANNOT be deserialized back into
   one — not without leaking. Restore therefore RESOLVES stored id strings
   against the ids this build actually registers, and drops the rest. Same
   for capabilities: a stored slug this build does not know is dropped, never
   invented. This is the doctor law the seam already speaks, applied to the
   estate. Pinned by `a_provider_this_build_no_longer_knows_is_dropped_not_
   resurrected` and `an_unknown_stored_capability_is_dropped_not_invented`.

2. **`Capability` had no stable wire name** — only `label()`, a human string
   free to be reworded. Persistence and the seam both need a name that does
   not change when a caption does, so `Capability::slug()` / `from_slug()` /
   `ALL` were added beside it in `dpl-chat-core`. Putting them there rather
   than in the app crate is the §3.1 reasoning applied again: a second
   stringify table is exactly the mirror that drifts. Pinned exhaustively by
   `every_capability_slug_round_trips_and_is_unique`.

### Publishing is change-detected

Activation is mutated from **fourteen** `set_provider_activation` call sites
plus the provider-event drain, so any enumeration of "activation-affecting"
intents is one new call site away from being silently wrong.
`republish_roster` compares the projection against the last published one and
publishes only on a difference, from the single funnel every frame passes
through. A steady state costs one comparison of ~4 small rows.

### Pins (all green)

`cargo test -p dpl-fabric -p dpl-chat-core -p dpl-shell -p dpl-chat-window
-p dpl-seam -p devpulselabs`

| pin | crate | proves |
|---|---|---|
| `an_empty_roster_reads_empty` | fabric | no publish, no panic, no chip |
| `the_roster_is_readable_without_subscribing` | fabric | §2.2 — a late CLONE reads the fold |
| `a_history_only_provider_is_in_the_roster_but_cannot_send` | fabric | `can(PageHistory)` ✓, `can(StreamTurns)` ✗ |
| `an_unactivated_provider_can_nothing` | fabric | qualified ≠ available |
| `publishing_a_roster_replaces_the_previous_one` | fabric | wholesale, no accumulation |
| `the_roster_answers_who_can_take_a_beat` | fabric | the routing question the SET exists for (ADR-017 §4) |
| `every_capability_slug_round_trips_and_is_unique` | chat-core | the stored vocabulary is exhaustive + stable |
| `an_unknown_capability_slug_is_dropped_not_invented` | chat-core | the label is not the wire name |
| `the_roster_survives_a_restart` | app | ADR-017 §2 — truthful at boot, before any probe |
| `a_provider_this_build_no_longer_knows_is_dropped_not_resurrected` | app | resolve, never leak |
| `an_unknown_stored_capability_is_dropped_not_invented` | app | refuse by name |
| `no_estate_is_not_an_error` | app | derived state degrades to empty, never to a failed launch |
| `the_seam_projects_the_roster` | seam | an agent can read which experts exist (ADR-017 §3) |
| `the_shell_names_the_activated_providers` | shell | §6.5, in a unit |
| `the_shell_says_nothing_when_no_provider_is_activated` | shell | silence, not an empty claim |

### The live proof — PASSED

`tests/eguipilotai/platform-sees-providers.toml`, fixture `chat` (live
runtime, no authored models), asserting `root/shell.status.providers` — the
ROOT viewport, not `chat.primary` — is visible and its text names Claude.
The chip cannot render unless a provider is genuinely activated, and Claude
activates on this machine only by having transcripts on disk.

Run on the Linux lane under `devpulse-cloud` (software Vulkan / lavapipe),
all five steps green:

```
outcome: passed
  wait 2500ms                                        ✔
  visible  root/shell.status.providers               ✔
  text     root/shell.status.providers ~ "Claude"    ✔
  text     root/shell.status.providers ~ "AI ·"      ✔
  capture  platform-sees-providers (viewport root)   ✔ "capture stabilized and written"
```

Proof shot: `docs/proof/platform-sees-providers.png` — MAIN at 1440×900 on
the wiredFRONT hub, status bar reading **`AI · Claude Code`**. The capture
is scoped to the `root` viewport explicitly: the `chat` fixture opens the
peer window too, and an unscoped capture cannot know which of two live
viewports is the evidence.

The chip reads `Claude Code` alone rather than `Claude Code, Codex` — this
machine has `~/.claude/projects` transcripts and no `~/.codex/sessions`, so
naming only Claude is the truthful answer, not a partial one.

### Regressions — pre-existing, NOT caused by this pass

`chat-local-histories`, `chat-content-spine`, `chat-provider-tab`, and
`shell-navigation` all report `failed` on this Linux box. They fail
**identically on `origin/main` (`2459cdf`) with none of this pass's code**,
verified by rebuilding the baseline exe from a clean checkout and running
the same four under the same Xvfb display. `shell-navigation` fails on its
FIRST step (`visible root/shell.rail/workbench`) and on a hover-driven
flyout — an environment/timing characteristic of this headless lane, not
anything a status-bar chip could touch.

Recorded rather than silently skipped: this pass did not fix them and does
not claim they are green. They need running on a lane with a real display
(`dpl-win-laptop` / `dpl-linux`) before the PR merges, and the fact that
`platform-sees-providers` PASSES on the same box is what makes its result
meaningful — the harness works here; those four scenarios are separately
unwell.
