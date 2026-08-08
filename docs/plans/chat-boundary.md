# BND1 — the chat boundary (one PR, POAM-tracked)

## 1 · Context

Operator's asks (verbatim, 2026-08-07/08): "Why are all of my commands
missing why is enter not properly responding to send… /route for the
ability to load dynamically the routes from the platform… but also
/model /routines etc only we will even allow it to trigger guided
seeded values intuitive suggestions if necessary preventing out of best
practices values and inputs" · "as soon as one subscription comes live
Clyffy becomes active. That is its own unique session that can be
toggled to use any provider as powering it" · "get this authored so the
chat boundary and AI boundary are truly separate."

The boundary statement this pass makes true: **window = render +
recognize · core = command vocabulary + validation + seeds · host =
dispatch + IO · providers = transport.** No ratatui/wezterm second UI
stack (assessment ratified-by-silence 2026-08-08, memorized): the
engine/front-end split already exists — the chat core is EGUI_FREE by
census; this pass builds the missing COMMAND LAYER between the two
boundaries.

**Decisions (inline per the no-dialogs ruling):**
- D1 `CommandSpec` census lives in `dpl-chat-core` (EGUI_FREE holds —
  plain data): `{ slug, label, blurb, args: Vec<CommandArg { id, ask,
  kind: RoutePick|ProviderPick|WorkflowPick|Text, required }>,
  privileged: bool }`. Six v1 commands: /route /model /routines
  /brainstorm /strict /behavioral. Nothing else enumerates commands.
- D2 Recognition: ONE registry-driven parser replaces
  `parse_operator_slash` + `parse_brainstorm_trigger` (deleted, not
  wrapped). Whole-buffer at submit, provenance law verbatim; unknown
  `/x` stays ordinary text. Word-triggers (`brainstorming`) stay as
  registry ALIASES of /brainstorm.
- D3 Privileged dispatch unchanged: /strict //behavioral resolve to
  `OperatorSlash { command, synthetic }` — `input_is_synthetic` read
  at the composer, `resolve_operator_slash` refuses synthetic BEFORE
  the gate. Every census command inherits the synthetic refusal.
- D4 Enter honesty: `submit_key && !enabled` currently discards the
  consumed key silently (window:3119) while the reason string already
  exists (:3091-3105). Fix = a window-local transient status line
  under the composer painting THAT string (composer `self.status`
  precedent). No new copy.
- D5 Guided seeds = `GuidedValue<T>` verbatim (core:118-205 — built
  for this, its Route/Session/Ai sources unused until now). Args seed
  from live censuses: /route from the seam registry's full hub/sub
  table, /model from the roster, /routines from
  `dpl_workflows::workflows()`. Out-of-census args refuse NAMING the
  census (the --brainstorm-drill message shape).
- D6 /route rides `NavigateTo` — its FIRST emitter ever (the routes
  field is dead weight behind a count badge today; orphans get
  completed). `ChatWindowConfig` widens to carry the full hub/sub
  census (`dpl_seam::Registry` shape, 'static specs).
- D7 /routines v1 = LIST the workflow census inline + navigate to the
  Workflows tab to run. Chat-triggered runs need the cross-window seam
  — named LATER slice (R2c), not smuggled in.
- D8 Clyffy = completing the declared `CLYFFY_PROVIDER` FutureAdapter
  row (profiles:17,180-188): a runtime-level HOUSE SLOT beside the
  providers (the SessionProviderMismatch guards forbid living inside
  one — runtime:231,279), first real occupant of the orphaned
  `SessionGroup::Pinned`, activation = any `PresenceVerdict::SignedIn`
  ("subscription live" honestly collapses to this today —
  AccountSummary.plan is dead), backing swapped via /model through a
  `ChatSessionId → (ProviderId, provider_session_id)` BINDING (the
  provider is a current binding, never part of the identity), history
  = the first DevPULSE-WRITTEN transcript JSONL under userdata, paged
  by the existing catalog via a house decoder.
- D9 Typeahead strip: no precedent exists anywhere — new machinery.
  Mount = `egui::Area` + `Order::Foreground` at the composer rect
  (provider_hover_overlay shape); arrow/Tab/Escape keys via a
  pre-TextEdit event-retain pass mirroring `take_composer_submit`'s
  documented law (consume-before-widget; never `consume_key` — it
  ignores shift); `Event::Ime`/`Event::Text` untouched.

## 2 · Current state (three Opus sweeps 2026-08-08 @ 842f481, every
sweep spot-checked — §8)

- **Recognition today**: two competing hand parsers (window:433,:447)
  over 3 commands; `OperatorSlashCommand` is a bare 2-variant enum in
  the WINDOW crate. No /route //model //routines anywhere (grep zero).
- **The Enter defect precisely**: `take_composer_submit` (:107-126)
  consumes bare Enter pre-TextEdit; `enabled`/`reason` computed
  (:3086-3105, five honest reasons authored); `submitted =
  send.clicked() || (submit_key && enabled)` (:3119) — the reason is
  hover-only on a disabled button; the keyboard path has NO surface.
  No test covers `submit_key && !enabled`.
- **Routes**: `window.routes` read ONCE (a count label :2960);
  `NavigateTo` has ZERO emitters; the FULL hub/sub census lives in
  `dpl_seam::Registry` (resolve("hub/sub") with named refusals,
  hubs_json) — chat gets hub-defaults only (main.rs:1067).
- **Store truth**: `dpl-chat-store` is a READ-ONLY jail over vendor
  JSONL (Codex + Claude sources only); session ids are FNV hashes of
  (provider, path); live ids are a host counter; NOTHING persists a
  DevPULSE-owned session. `Pinned` is declared-never-assigned outside
  a test fixture. `seam_sessions`/`seam_turns` key by (ProviderId,
  String); runtime guards reject provider mismatch (runtime:231,279).
- **Presence/activation**: probe_all → per-vendor verdicts; SignedIn →
  `Experimental` (non-destructive apply, app/chat.rs:425-466);
  roster published change-driven + persisted to `Tree::Agents`.
  `Capability::SubscriptionAuth` qualifies only via Codex/Grok
  activation coordinators; `AccountSummary.plan` never populated.
- **Clyffy today**: `CLYFFY_PROVIDER` + a FutureAdapter catalog row
  ("declared but not qualified") — nothing else.
- **GuidedValue**: full provenance/confidence/sensitivity API; ONLY
  production use is Grok's device-auth prompt. Sources Route/Session/
  Ai and confidence Suggested are unused. KNOWN LEAK (rider, not this
  pass's silent fix): window:4199 puts a TransientSecret user_code
  into pilot NodeMetadata with no sensitivity check.
- **Proof estate**: `chat-enter-sends.toml` exists (fill+Enter, no
  focus-viewport — headless-safe); chat-brainstorm 6/6; deferred
  viewports cannot self-capture (native proof is the pixel contract);
  ChatProofMode + chat_native_proof carry DUPLICATED state lists;
  harness-flag census is CLAUDE.md-bidirectional. Side-find TO RULE
  ON (parked, never silent): dpl-chat-{cursor,context,windows,
  extensions,renderers} are absent from EGUI_FREE.

## 3 · Target design

1. **Core: the command layer.** `CommandSpec`/`CommandArg` census +
   `commands()` + `command(slug)` + one `recognize(draft) ->
   Option<CommandDraft>` (whole-buffer; aliases; unknown-/x → None) +
   `validate_args(spec, values, censuses) -> Result<_, Vec<Issue>>`
   refusing by naming the census. Seeds ride `GuidedValue<String>`
   with the dormant Route/Provider/Session sources. Pure, EGUI_FREE,
   pinned (census + negative + provenance tests).
2. **Window: recognize + render.** Registry lookup replaces both
   parsers; the strip (D9) offers census commands filtered by the live
   draft with seeds visible; Enter honesty (D4) paints the existing
   reason; the composer emits `Command(CommandDraft)` intents —
   /strict //behavioral still emit `OperatorSlash` (D3 byte-identical).
3. **Host: dispatch.** `HostIntent::Command` arm: /route →
   validate against the widened route census → NavigateTo (first
   emitter); /routines → project the workflow census inline +
   NavigateTo the Workflows tab; /model → Clyffy binding swap;
   /brainstorm → StartBrainstorm (existing).
4. **Runtime: the house slot.** `HouseSession { id, title: "Clyffy",
   group: Pinned, backing: Option<ProviderId>, events }` beside
   `providers`; activation derived from presence (any SignedIn =
   active; none = Blocked with the sign-in recovery); sends resolve
   through the binding to the backing runtime; every ChatEvent keeps
   the provider_id that produced it (history is provider-tagged per
   event ALREADY — continuity is representable); the profiles row
   flips truthful. House transcript JSONL written under userdata +
   `ClyffyHistoryDecoder` so the catalog pages it like any vendor.

Scale story: command #7 = one census line + one dispatch arm; the
strip, seeds, refusals, and synthetic gate are paid for once; the
binding table is the seam Clyffy-drive (graduated trust) later rides.

## 4 · Changes, in execution order — see §9 POAM

## 5 · Boundaries

- 🚫 `/content`; chat-triggered workflow RUNS (D7 later slice, R2c);
  the Clyffy graduated-trust drive (its own arc); the GuidedValue
  sensitivity leak (named rider — fix window:4199 in BND1-M1 as ONE
  guarded line, or park with a dated STATE line); EGUI_FREE census
  gaps (side-find parked for operator ruling); provider convergence
  (Grok's bespoke runtime stays).
- ⚠️ Ask first: any new crate; any dep beyond window/core's current
  sets; touching the determinism gate's shape; deleting the
  FutureAdapter row instead of completing it.
- Rebase at milestone boundaries; journal per milestone; double-green
  before merge; shared-checkout law: check `git branch --show-current`
  before ANY git op in `C:\localdev\devpulselabs`.

## 6 · Verification

V1 command registry census (count-justified, unique, validated,
negative lookup) · V2 no parallel enumeration (parsers deleted,
grep-proof) · V3 whole-buffer provenance across all six (+ unknown /x
stays text) · V4 out-of-census args refuse naming the census, refusal
mutates nothing · V5 seeds carry GuidedValue provenance; operator
override flips source · V6 synthetic refusal survives + extends to the
whole census (4 existing mint pins unchanged) · V7 THE ENTER-HONESTY
PIN (state test: refused Enter records the painted reason, no intent
emitted; kittest: reason reachable as node value without hover) · V8
strip offers only census commands (registry-derived, no second table)
· V9 Clyffy inert without presence / active on one SignedIn (the
FutureAdapter row becomes truthful) · V10 backing swap preserves the
transcript (same ChatSessionId, events in order, per-event provider
tags; /model ghost refused, binding untouched) · V11 [NEW]
`ChatProofMode::Commands` (BOTH duplicated lists amended together) +
native PNGs at 1280x800 AND 1100x700, READ, baselined · V12 [NEW]
pilot `chat-command-strip.toml` (chat-enter-sends shape: no
focus-viewport, no capture on the deferred viewport; paired negative
asserts the refusal as a node value) · V13 existing chat scenarios
stay green (chat-peer-window is a known warm-up race — re-run before
calling red) · V14 structure audit (EGUI_FREE holds for core; new
flags land in CLAUDE.md same commit) · V15 cargo green all phases ·
V16 double-green (`gh pr checks --watch`; a running check is red) ·
V17 operator exe + STATE (the BS1-parked item struck).

**THE BND1-PR CHECKLIST** (each row checked with evidence in the PR):
- [ ] ONE `commands()` census; both hand parsers GONE (grep-proof).
- [ ] Whole-buffer recognition at submit; unknown `/x` stays text.
- [ ] Every refusal names its census; a refused command mutates nothing.
- [ ] Every census command refuses under `input_is_synthetic`.
- [ ] No silent submit path — existing reason strings painted for
      keyboard as well as mouse; no new copy.
- [ ] `GuidedValue` reused as-is; no second provenance vocabulary.
- [ ] Clyffy presence-gated; the "declared but not qualified" row
      becomes true.
- [ ] Proof-state censuses amended together; new flags in CLAUDE.md
      same commit.
- [ ] Pilot semantic nodes; no capture on the deferred viewport.
- [ ] Native PNGs both sizes, READ.

## 7 · Commit plan

Branch `feat/chat-boundary` (cut from 842f481, carries BS1). Commits
per milestone, gate-clean in-lane; PR DRAFT at M1-end; squash-merge on
double-green after M4. Journals `docs/journal/chat-boundary-m<N>.md`.

## 8 · Evidence & provenance

Deep-plan fan-out 2026-08-08, three Opus sweeps (store/session +
presence + GuidedValue; composer/routes/typeahead + determinism;
verification draft), all against feat/chat-boundary @ 842f481. Fable
spot-checks, confirmed verbatim: GuidedValue API (core:118-205 read
in-session), SessionProviderMismatch guard (runtime:229-237), Pinned
assigned only in a test fixture, CLYFFY_PROVIDER + FutureAdapter row
(profiles:17,180-188), chat-enter-sends.toml exists. Sweep
contradictions adopted: no session persistence exists; NavigateTo has
zero emitters; no typeahead precedent anywhere; "subscription live" =
SignedIn only; stale core:76 GuidedValue cite corrected to core:150.
Operator ratifications: the three verbatim asks (§1); "go" 2026-08-08;
D1-D9 RECOMMENDED-inline per the no-dialogs ruling, standing unless
overruled.

## 9 · POAM (the living tracker)

**Objective:** typing `/` in chat reveals the real command surface;
every command carries guided seeded values and refuses bad input by
naming what IS valid; Enter never dies silently; and the moment one
provider is signed in, Clyffy sits pinned at the top of the sessions —
one conversation, any backing.

| M | Scope (files) | Exit criteria | Status |
|---|---|---|---|
| BND1-M1 · The command layer | chat/core: CommandSpec+census+recognize+validate+seeds; window: parsers replaced, Enter honesty line, intents; app: dispatch arms (/route via widened route census → NavigateTo, /routines list+navigate, aliases); guarded fix or dated park of the window:4199 sensitivity leak | V1-V7 green in-lane | **DONE 2026-08-08** — 16+45+37 tests incl. THE ENTER-HONESTY PIN; NavigateTo's first emitter live; sensitivity leak parked with a dated line below |
| BND1-M2 · The strip | window: typeahead Area + key-retain pass + seed display; ChatProofMode::Commands (both lists); native PNGs both sizes READ | V8, V11 green | not started |
| BND1-M3 · Clyffy | runtime house slot + binding + presence gate; house transcript write + decoder; /model swap; profiles row truthful; Pinned occupied | V9, V10 green | not started |
| BND1-M4 · Close-out | pilot scenarios (+negative), existing suite re-run, rebase, green, double-green merge, exe, STATE | V12-V17; PR merged | not started |

### §9.1 Deviations log
- 2026-08-08 (M1): `/strict please` behavior CHANGED deliberately — the
  old parser let it fall through as an ordinary message; the registry
  recognizes it and refuses honestly ("/strict takes no arguments").
  The audited privileged mapping stays whole-buffer-exact
  (`privileged_command` requires empty rest); pinned in
  `slash_parse_is_exact_and_whole_buffer`.
- 2026-08-08 (M1): the window:4199 GuidedValue sensitivity leak
  (TransientSecret user_code into pilot NodeMetadata) is PARKED, not
  fixed — it predates BND1 and deserves its own guarded line with a
  test; carried in STATE as a named open.
