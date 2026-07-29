# Chat ASSIST MODE — one toggle, one acknowledgement, every provider

**Status:** EXECUTING (operator, 2026-07-28: "plan and do a chat pr …
it needs to be a clear simple acknowledgement that will blanket for all
providers")
**Date:** 2026-07-28
**Origin:** `assist-mode-addendum.md` (folded here §3a; delete with this PR)

## 1. Context

The operator's assist-mode vision: a simple toggle on the route-aware
chat window; ON means platform and app work AS ONE — chat may raise
pop-ups (bake-offs) and preferences get identified. THIS pass ships the
consent foundation only: the toggle, ONE clear acknowledgement that
blankets ALL providers (never per-provider), persistence, and the
machine-legible state — the gate every later proactive-UI pass stands on.
Bake-offs and the preference store are follow-on passes.

## 2. Current state (grounded 2026-07-28, main @ 51997ee; sweep spot-checked)

- `crates/chat/window/src/lib.rs` (4514 lines): pilot-id consts block
  :39-85; `ChatWindowIntent` :215-289 / `HostIntent` :325-373; `apply()`
  :669-817 — the mutate-then-forward arm shape is `ResolveImagineFlush`
  :800-812 (transition interaction → dismiss overlay → clear binding →
  forward). Binding machinery: `register_imagine_flush_prompt` :569-604
  (host-provenance Approval spec, overlay priority 1_000, binding =
  `imagine_flush_consent: Option<InteractionId>` :458);
  `interaction_overlays` :3137-3285 resolves approve/always/reject via a
  per-button if/else-if chain over the bindings (spot-checked
  :3195-3272); the ALWAYS button renders only when
  `always_approve_label.is_some()` (:3213). The composer's first row
  :1929-2014 is the status strip — route chip (`chat.context.route`)
  :1941-1950, then capability buttons. Setter pattern: bare projection
  setters (`set_generation_status` :554). 19 inline unit tests; binding
  template = `provider_permission_preserves_allow_once_always_and_deny_choices` :4382.
- `crates/app/src/chat.rs` (3176 lines): `ImaginePolicy` wiring is the
  precedent end-to-end — boot load under `provider_runtime_enabled`
  :406-424 (authored fixtures get `Default` and never touch Ring-0),
  interaction minting from `next_interaction_id` (seeded 2_000_000_000)
  :528-529, host dispatch `apply_host_intent` :823-1034, persistence in
  `resolve_imagine_flush` :1538-1562 (save only on the persisting
  branch), per-frame projection `project_imagine` :1404-1410. Route
  awareness: `observe_route` :578/:582 → `RouteContextState`
  (crates/chat/context).
- `crates/chat/imagine/src/lib.rs` :193-213: the policy-file shape —
  infallible `load(userdata)` failing closed to `Default`,
  `#[serde(default)]` fields, tmp-then-rename `save`.
- `crates/chat/core` has ZERO deps (pure) — the policy struct must NOT
  land there (serde would break its purity); `dpl-chat-profiles` is
  SQLite provider-profiles only. Decision: policy lives app-side.
- Scenario template: `tests/eguipilotai/chat-imagine-panel.toml`
  (authored fixture; dismisses `interaction-1003`, asserts values);
  authored approval spec precedent `crates/chat/testkit/src/lib.rs:437`.

## 3. Target design

**One toggle, one acknowledgement, zero per-provider anything.**

- **Policy (app-side, `crates/app/src/chat.rs`)**: `AssistPolicy
  { enabled: bool, acknowledged: bool }`, `assist-policy.json` in
  `userdata_root()` — ImaginePolicy's exact load/save shape. Loaded at
  boot under `provider_runtime_enabled`; authored fixtures run in-memory
  defaults and NEVER write disk.
- **Window**: const `ASSIST_ID = PilotId::new("chat.assist.toggle")`;
  field `assist_enabled: bool` + setter `set_assist(bool)`; a plain
  always-enabled chip in the composer status row RIGHT AFTER the route
  chip (assist IS route-awareness made active) — label "⚡ Assist",
  value "on"/"off" in pilot metadata, hover text in layman words.
  NOT capability-gated, NOT provider-gated.
- **Intents**: `ChatWindowIntent::ToggleAssist` → pass-through
  `HostIntent::ToggleAssist { enable }` (window sends the DESIRED state:
  `!assist_enabled`). `ChatWindowIntent::ResolveAssistAcknowledgement
  { interaction, accepted }` → mutate-then-forward (the imagine-flush
  arm shape) → `HostIntent::ResolveAssistAcknowledgement { accepted }`.
- **The acknowledgement** (`register_assist_acknowledgement(interaction)`,
  binding `assist_acknowledgement: Option<InteractionId>`): Host-authored
  Approval overlay, TWO buttons (`always_approve_label: None`). Copy —
  title "Assist mode"; summary: "Chat and the app work as one: chat can
  see where you are — your route, your open document, your storyboard's
  empty slots — and may offer suggestions as pop-ups you approve or
  dismiss. One setting, every provider: this applies across all chat
  providers, current and future. Nothing is sent or changed without your
  action."; approve "Turn on assist — all providers"; reject "Not now".
- **Host flow** (`apply_host_intent` arms): `ToggleAssist { enable:
  false }` → policy.enabled=false, save (persisting mode), project.
  `{ enable: true }` + already `acknowledged` → enabled=true, save,
  project. `{ enable: true }` + NOT acknowledged → mint interaction id,
  `register_assist_acknowledgement` — the toggle does not flip until
  consent. `ResolveAssistAcknowledgement { accepted: true }` →
  enabled=true + acknowledged=true, save, project; `{ accepted: false }`
  → nothing persisted, stays off, project. The acknowledgement is asked
  ONCE ever (acknowledged persists across sessions and providers).
- **Projection**: `project_assist(state)` → `window.set_assist(policy.enabled)`
  at boot + after every assist host arm.
- No fabric changes, no seam changes, no per-provider records anywhere.

### 3a. Folded addendum (operator vision this pass serves)

Assist ON = platform+app as one → later passes raise bake-off pop-ups
(2-3 concrete variant previews via the Interaction Library) whose picks
land as durable preferences (estate) tailoring future defaults;
workbench/workflows is a full peer with slide-granularity storyboarding.
This pass builds ONLY the consent gate those passes require.

## 4. Changes, in execution order

1. `crates/chat/window/src/lib.rs`: `ASSIST_ID` const; `assist_enabled`
   field (+ Default false) + `set_assist`; `assist_acknowledgement`
   binding field; two `ChatWindowIntent` + two `HostIntent` variants;
   `apply()` arms (pass-through + mutate-then-forward);
   `register_assist_acknowledgement` (spec per §3);
   `interaction_overlays`: one `else if` in the approve chain
   (`accepted: true`) and one in the reject chain (`accepted: false`);
   the chip in the composer row after the route chip. Unit tests:
   toggle-crosses-the-typed-host-boundary (off→on sends enable:true; the
   chip value follows `set_assist` only), register-then-resolve binding
   (approve completes the interaction + forwards accepted:true; reject
   forwards accepted:false; binding cleared), acknowledgement-has-no-
   always-button (spec's `always_approve_label` is None).
2. `crates/app/src/chat.rs`: `AssistPolicy` struct + `ASSIST_POLICY_FILE`
   + load/save (ImaginePolicy shape); `assist: AssistPolicy` +
   `assist_persist: bool` on `Shared`; boot load + `project_assist`;
   the two `apply_host_intent` arms per §3; authored branch = defaults,
   no writes.
3. `tests/eguipilotai/chat-assist-toggle.toml` (fixture `chat-authored`):
   dismiss authored approval 1003 → expect `chat.assist.toggle` value
   "off" → click it → expect overlay `interaction-2000000000` approve
   button → click approve → expect toggle value "on".
4. Docs: delete `assist-mode-addendum.md` (folded §3a); STATE tip; §8.

## 5. Boundaries

- 🚫 Never: per-provider assist flags/capability gating; new fabric
  variants; bake-off UI or preference storage (follow-on passes); disk
  writes in authored mode; touching provider crates, chat-core deps, or
  any CEILING-PINNED crate manifest (the structure audit's seven);
  `#[allow]`.
- AMENDED mid-execution (reported): the app crate had no direct serde —
  `serde`/`serde_json` (workspace) added to `crates/app/Cargo.toml` for
  the app-owned policy file. The app is not a ceiling crate; the
  structure audit is unaffected.
- ⚠️ Ask first: assist chip in the top_strip instead of the composer row
  (chosen: composer row, beside the route truth it activates); any
  proactive behavior wired to the flag beyond surfacing state.
- Out of scope: bake-offs, preference store (estate), seam/A2UI exposure
  of assist state, coach triggers.

## 6. Verification

1. `cargo test -p dpl-chat-window` — 19 existing + 3 new, all green.
2. `cargo green` — full gate (structure phase guards the no-new-deps
   claim mechanically).
3. EguiPilotAI `chat-assist-toggle` passes on the devtools exe; the
   chat scenarios that were green stay green (sequential).
4. Real exe truthfulness: with no `assist-policy.json`, the chip reads
   "off"; toggling raises the acknowledgement; approve → "on" AND
   `assist-policy.json` exists with `{"enabled":true,"acknowledged":true}`;
   relaunch → chip restores "on" with NO re-acknowledgement; toggle off
   → instant, persisted.
5. Shots of the acknowledgement overlay + on-state chip at real window
   sizes.

## 7. Commit plan

Branch `feat/chat-assist-mode` → PR → squash. Commit:
`feat(chat): assist mode — one toggle, one blanket acknowledgement (all providers)`.
Deletes the addendum; STATE tip in-PR; session-end trio after merge.

## 8. Evidence (2026-07-28)

- **§6.1**: `cargo test -p dpl-chat-window` 22/22 (19 existing + 3 new:
  `assist_toggle_crosses_the_typed_host_boundary` — the chip never flips
  itself; `assist_acknowledgement_is_one_blanket_consent` — Approval
  spec has NO "always" button, the copy states "all chat providers",
  approve forwards `accepted: true` and clears the overlay;
  `assist_acknowledgement_decline_stays_off`).
- **§6.3 scenarios** (devtools exe, sequential): NEW
  `chat-assist-toggle` (authored: off → tap → acknowledgement overlay →
  approve → on) PASSED; NEW real-runtime pair on scratch rings —
  `chat-assist-enables` PASSED and WROTE
  `target/eguipilotai/assist-rings/.userdata/assist-policy.json` =
  `{"enabled": true, "acknowledged": true}`; `chat-assist-persists`
  (a FRESH launch) PASSED expecting "on" at boot with NO
  re-acknowledgement — §6.4's persist/restore proven mechanically, not
  manually. The six previously-green chat scenarios all stayed green
  (codex-persisted-ready, grok-device-authorization,
  grok-persisted-ready, imagine-panel, provider-catalog, steam-social).
- **§6.5 visual**: `proof-chat-assist-ack.png` (chat-native-proof, new
  `assist` state, 1300x800): the acknowledgement overlay with the
  blanket copy + two buttons, the ⚡ Assist chip beside the
  route-context chip (`wiredfront / editor`) in the composer row.
- **§6.2**: `cargo green` (result recorded at land).
- **Deviations (reported)**: (1) the app crate had no direct serde —
  `serde`/`serde_json` added to `crates/app/Cargo.toml` (§5 amended
  in-flight; not a ceiling crate, structure audit unaffected). (2) The
  in-scenario `capture` step cannot shoot a deferred chat viewport
  (eframe 0.35 limitation, honest runner error) — the visual proof rides
  the chat-native-proof harness instead, gaining a reusable `assist`
  proof state. (3) §6.4's "relaunch" proof became the committed
  `chat-assist-enables`/`chat-assist-persists` scenario pair — stronger
  than the planned manual check.
