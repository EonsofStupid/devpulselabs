# Determinism modes behind the intent gate — dpl-intent's first consumer

Status: planned this pass (2026-07-28), executing under the ratified rulings
in `docs/plans/signal-ledger-and-lesson-foundation.md` §9.2/§9.3 and the
working notes in `docs/journal/2026-07-28-next-phase-notes.md` §B.

## 1. Context

Phase 1.5 shipped `OperatorIntent` — the single-use, non-serde proof — with
no consumer. The ruling it encodes: determinism is a dynamic mode
(**behavioral** default for measurement, **strict** escalated for evidence),
and escalation is authorized by the **provenance of the keystroke**, never by
a pattern in a string. This pass builds the mode, the two authorized
channels, the policy override, and the audit trail.

Deliberately NOT in this pass: the pinning that strict mode will eventually
drive (fixed clock, pinned RNG, locked adapters) lands with replay; the
protocol/pilot_status surfacing of the mode lands when a consumer needs to
read it over the pipe. This pass is the switch, its guards, and its record —
so that when the pinning arrives, the authorization question is already
settled law with tests.

## 2. Ground truth (read in-session, 2026-07-28)

- `OperatorIntent` / `IntentChannel` exist in `dpl-intent` (∅ deps, ceiling-
  pinned by the structure audit).
- The chat composer's submit is the Send-button site in
  `crates/chat/window/src/lib.rs` (~line 2049): the draft becomes
  `ChatWindowIntent::SendAiTurn`-shaped intents → `ChatWindow::apply` →
  `HostIntent` → `apply_host_intent` in `crates/app/src/chat.rs`, which holds
  the fabric. Both intent enums derive `Clone` — a non-`Clone` proof cannot
  ride them.
- The pilot already knows injected input: `SharedState.synthetic_input`
  (per-viewport set), but it is **consumed** by `coach_pass` and has no
  public query. `egui_pilot_ai::handle(ctx) -> Option<PilotHandle>` exists,
  so a free function can degrade to `false` when no plugin is installed
  (production builds).
- The shell inspector is a permanent right panel with cards (SIGNALS card
  verified live this session); the shell holds a fabric handle and opens its
  signal ears at construction.
- Model output enters the window via provider facts
  (`observe_provider_fact`), a path that never touches the submit parse —
  the "model output is never re-parsed" law holds structurally.

## 3. Target design

### 3.1 The gate owns the mode (`dpl-intent`)

`DeterminismGate`: a cloneable handle (std `Arc<Mutex>`, crate stays ∅-dep)
over one state machine:

- `mode() -> DeterminismMode { Behavioral, Strict }`
- `escalate(OperatorIntent) -> IntentChannel` — consumes the proof (the
  only way in; the channel returns for the audit record).
- `deescalate() -> Deescalation { Cleared, RefusedEvidence, AlreadyBehavioral }`
  — no proof needed, but only provenance-checked call sites exist.
- `force_evidence()` — policy, no proof: "evidence forces strict" (§9.2).
  Evidence-forced strict cannot be de-escalated by the operator; the run is
  the evidence.
- `strict_cause() -> Option<StrictCause { Operator(IntentChannel), Evidence }>`
  — evidence outranks operator in the answer.

The gate returns facts; it never publishes. Callers own the fabric edge, so
the crate stays dependency-free and its ceiling (∅) stands.

### 3.2 The INTENT fabric domain

`IntentEvent { DeterminismChanged { mode, cause } · IntentRefused { action,
reason } }`, stamped like every fact domain. `dpl-fabric` gains a dependency
on `dpl-intent` to reuse `DeterminismMode`/`StrictCause`/`IntentChannel`
rather than mirroring them (fabric already carries `dpl-hub` and
`dpl-chat-core` types for the same reason — architecture.md §3 gets the
edge recorded). Consumers land with the domain, per doctrine: a seam
envelope (`intent_json`, pinned), activity-feed lines, and doctrine law 11.
The coachable-signal catalog does NOT forward this domain — mode changes
are not lesson material, and nothing that flows toward an agent should
narrate the escalation surface.

### 3.3 The slash channel (keystroke provenance)

At the Ai composer's Send click — the operator's input-submit event — the
trimmed draft is parsed for a slash command (`/strict`, `/behavioral`)
BEFORE it can become a message. Recognition emits
`ChatWindowIntent::OperatorSlash { command, synthetic }` (draft cleared,
nothing sent to any provider) → `HostIntent::OperatorSlash` →
`apply_host_intent`, which is the ONE slash-channel mint site:
`OperatorIntent::slash_command()` → `gate.escalate` → publish
`DeterminismChanged`. The `synthetic` flag is read at the click from the
pilot (below); a synthetic submit publishes `IntentRefused { action,
"synthetic-input" }` and mints nothing — an agent that fills the composer
with `/strict` and clicks Send gets a recorded refusal, not a mode change.
Unrecognized `/…` text stays an ordinary message. Model output cannot reach
this path (§2, structural).

### 3.4 The UI second-acknowledgement channel (shell inspector card)

A DETERMINISM card in the inspector: shows the mode and its cause; a
two-step control (`arm strict` → `confirm strict`) whose confirm mints
`OperatorIntent::ui_acknowledged()`; a single-step `return to behavioral`
when strict is operator-demanded; a locked label when evidence-forced.
Every activation checks input provenance first; a synthetic click publishes
`IntentRefused` and does nothing else. The shell receives the gate by an
`attach_determinism` setter (no constructor churn) and publishes on its own
fabric handle.

### 3.5 The provenance query (`egui-pilot-ai`)

`SharedState` gains `synthetic_frame`: a per-viewport set maintained at the
raw-input hook (inserted when this frame's input includes queued injection,
removed otherwise) — unlike `synthetic_input`, it is never consumed by the
coach pass. Public: `PilotHandle::frame_input_synthetic(viewport)` and the
free `egui_pilot_ai::input_is_synthetic(ctx)` (current viewport; `false`
when no plugin is installed, which is every production build — real input
is the only input a production build can receive).

### 3.6 Policy: evidence forces strict

The `--screenshot` harness path calls `gate.force_evidence()` at boot and
publishes the fact once the fabric exists. Self-proving: any proof shot
that includes the inspector shows `strict · evidence` in its own pixels.

## 4. Changes (execution order)

1. `dpl-intent`: `DeterminismMode`, `StrictCause`, `Deescalation`,
   `DeterminismGate` + state-machine tests. Crate stays ∅-dep.
2. `dpl-fabric`: `IntentEvent` domain (+ `dpl-intent` dep), channel,
   publish/subscribe, round-trip test.
3. `dpl-seam`: `intent_json` + SSE merge + pinned envelope test.
4. `egui-pilot-ai`: `synthetic_frame` + queries + test.
5. `dpl-chat-window`: slash parse at submit, `OperatorSlash` intents, parse
   unit tests.
6. `crates/app`: gate at boot; chat `Shared` carries it; `apply_host_intent`
   mints/refuses/publishes; `--screenshot` forces evidence.
7. `dpl-shell`: DETERMINISM card, intent ear + feed lines,
   `attach_determinism`.
8. Docs: doctrine law 11, architecture.md §3 fabric edge, STATE.md.

## 5. Boundaries

- No protocol change; the pilot pipe does not carry the mode this pass.
- No pinning behavior (clock/RNG/adapters) — that is replay's pass.
- The gate never touches the chat transcript, provider output, or any
  replayed trace; those paths cannot reach the mint sites by construction.
- `dpl-intent` stays dependency-free (ceiling ∅, audit-enforced).
- Social composer slash support deferred; the Ai composer is the ruling's
  "operator's text buffer".

## 6. Verification

- Unit: gate matrix (escalate consumes; evidence outranks; de-escalation
  refused under evidence; channels recorded); slash parse (exact command,
  leading whitespace, non-initial `/strict` inert); fabric round-trip; seam
  envelope pins; pilot synthetic-frame query (injected → true, real →
  false, absent plugin → false).
- Live (devtools session, agent-driven): `pilot_run` fills the composer
  with `/strict` and clicks Send → expect `IntentRefused ·
  synthetic-input` on the SSE stream and the feed, and the mode still
  behavioral — the refusal path is exactly what an agent CAN witness.
  The operator happy path (real click) is left for the operator's hands.
- Harness: one `--screenshot` run whose shot shows `strict · evidence` on
  the DETERMINISM card.
- Gate: `cargo green` (structure audit holds dpl-intent at ∅ deps).

## 7. Commit plan

One PR: `feat(intent): determinism modes behind the operator intent gate`.

## 8. Evidence

Ground truth in §2 was read from the working tree and the live Part A
session this same day, not from memory. The provenance law wording follows
the operator's 2026-07-28 rulings verbatim where it matters ("recognized at
the operator's input-submit event, from the operator's own text buffer,
before that text ever becomes a message").

Harness proof (2026-07-28): `--screenshot` at 1280×832 forces evidence;
`docs/proof/determinism-evidence-2026-07-28.png` shows the DETERMINISM
card `strict · evidence` / "locked — this run is evidence" and the feed
line `determinism · strict · evidence`. Unit matrix green across
`dpl-intent`, fabric round-trip, seam envelope, slash parse, and
synthetic-frame query.