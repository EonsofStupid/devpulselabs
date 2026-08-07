# CAPTURE — brainstorm-triggered A2UI chat + Clyffy graduated drive

**STATUS: PROPOSED — nothing here is ratified until the operator rules.**
This is a vision-capture + research document (2026-08-06), not a pass
plan. When a slice is ratified it gets its own 7-section plan doc.

## 1 · Operator vision, verbatim (2026-08-06)

> "what I am looking for is cutting edge patterns for brainstorming
> first right when i type in chat brainstorming it literally will
> trigger a2ui addition components that will populate with list that I
> can edit they will be visualized. So one will be a storyboard yes but
> it might have a template to make it brainstorming so its taking the
> massive idea and hten breaking down. However we also have triggers
> and events that allow us to have new sessions created seamlessly and
> then processes that feed them all back into grounding where we can
> even have other providers assist in offering counter points."

> "The other clear task is that we are suppose to have the ability to
> allow Clyffy to have full control to impersonate the operator as we
> explore ways to track and train and at first have guards/gates that
> require manual approval on keyparts as we evolve the whitelist to
> cover the majority of actions even for the lazier vibecoders that
> will have workflows captured ahead of time to keep them safe because
> we plan ahead for it"

Relation to the ratified estate: brainstorming sits UPSTREAM of the
EX1 pipeline (intake → bible → storyboard → production) — it is the
stage where the massive idea gets broken down before intake captures
it. Counterpoints extend the assist-mode/bake-off preference system
from output comparison to idea challenge. Clyffy drive is the R2c
command channel grown into a trust ladder, governed by the
operator-parity law (everything AI does must be operator-doable in the
editor first).

## 2 · Research: the brainstorm surface (web sweep 2026-08-06)

### 2.1 The protocol shape — A2UI is the right skeleton

Google open-sourced **A2UI** (Dec 2025): agents emit a declarative
JSON stream of `surfaceUpdate` (a FLAT list of components with ID
references — flat chosen because LLMs generate and patch flat lists
reliably), `dataModelUpdate` (state, separate from structure), and
`beginRendering` (names the root). Components bind to state by path
(`{"path": "/board/items/3/title"}`); user edits mutate the bound
data model LOCALLY with no agent round-trip; the agent only hears
named semantic actions (`confirm`, `expand`, `promote`) with the bound
context attached. Security model: **UI is data, not code** — the agent
can only reference a client-controlled component catalog.
(github.com/google/A2UI)

Why this fits us exactly:
- The trusted catalog IS the ColdLight ingredient library — the agent
  composes registered ingredients, never invents pixels. Brand control
  and safety in one move.
- Flat-list + separate data model maps cleanly onto egui immediate
  mode: rebuild from the component list each frame, bind values into
  the data model.
- We do NOT need the wire protocol itself in v1 — the provider's tool
  call carries the payload (Vercel-pattern: tool name = component
  selector, streamed partial args = the progressively-populating
  list). A2UI is the vocabulary to imitate, not a dependency.

Adjacent confirmations: **AG-UI** (CopilotKit) syncs one shared typed
state document via `STATE_SNAPSHOT` + RFC 6902 JSON Patch deltas, with
run/step lifecycle events and first-class human-in-the-loop
interrupts; **MCP Apps** (SEP-1865, Anthropic+OpenAI, Jan 2026)
pre-registers UI templates reviewed by the host before tool execution.
Both reinforce: pre-registered templates, patch-based state, auditable
structured traffic.

### 2.2 The brainstorm interaction — diverge, then converge, on a template

From FigJam AI, Miro AI, Whimsical AI, tldraw make-real, and Luminate
(CHI '24), five load-bearing patterns:

1. **Template-first**: the "brainstorming" trigger instantiates a
   phased scaffold (title, phase lanes, starter prompts) that the
   agent then populates — the template shapes both the human's and the
   agent's behavior. The operator's "storyboard with a brainstorming
   template" is literally this: same board machinery, brainstorm
   phases instead of beats.
2. **Explicit diverge→converge machinery**: idea explosion (1→N) and
   synthesis (cluster-by-theme, N→1) as separate named operations —
   not one "brainstorm" blob. The storyboard is the terminal
   convergent artifact: clusters become ordered beats.
3. **Recursive breakdown with structure-as-context** (Whimsical):
   every generated item carries a "break this down" affordance; the
   expansion prompt includes ancestors/siblings; the operator steers
   by EDITING THE STRUCTURE (seed items), not by re-prompting.
4. **AI output as editable peer objects**: generated items are
   indistinguishable from hand-made ones — editable, movable,
   deletable — and edits feed subsequent generations.
5. **Operations wired to selections** (Jambot): AI verbs (ideate /
   cluster / summarize / rewrite) apply to the current selection, and
   the operation chain stays visible and re-runnable.

### 2.3 Sessions, grounding, counterpoints

- **Blackboard as the grounding document**: one schema'd, versioned
  document every spawned session reads at start and writes back to
  ONLY through validated patch operations (PatchBoard, arXiv
  2605.29313; MetaGPT's shared pool). The same JSON-Patch machinery
  serves UI state sync AND multi-session grounding — one mechanism,
  two duties.
- **Trigger → scoped sub-session with a contract** (Anthropic's
  orchestrator-worker system): spawn with (task description, expected
  output format, fresh context); sessions return DISTILLED results,
  never raw transcripts; effort-scaling rules decide 1 vs N spawns;
  the parent synthesizes and commits the patch.
- **Counterpoint = enforced role asymmetry + separate judge**: pinned
  opposing briefs (advocate / skeptic), heterogeneous providers per
  role, a distinct judge step that writes the resolution to the
  blackboard. The literature warning: naive "two models chat" debate
  homogenizes (agents converge and lose facts) and often loses to
  self-consistency at equal compute — so **disagreement-gated**: run
  providers independently first, escalate to a debate round only when
  answers materially disagree or confidence is low. That is also what
  makes counterpoints affordable.
- **Event-sourced session lifecycle in the UI**: spawned sessions
  surface inline in chat as started → steps → finished + patch
  applied, giving the operator interrupt/approve/steer points — the
  JobBoard vocabulary already speaks this.

### 2.4 Proposed shape (PROPOSED), mapped to the estate

| Piece | Seam it lands on |
|---|---|
| "brainstorming" keyword trigger in chat | chat intent chain (`ChatWindowIntent → HostIntent`), same mount as the imagine flow |
| Inline populating/editable components | the M3 `InteractionContent::GuidedPrompt` seam generalized: `InteractionContent::Surface { components, data }` against the ColdLight catalog (⊞ Component entry point) |
| Brainstorm board document | new `BrainstormDoc v1` beside bible/workflow/storyboard (documents-not-code-paths, schema + pins, doctrine law 3) |
| Diverge/converge/breakdown verbs | semantic actions on the surface → provider calls; each mutation a validated patch, undo-stack integrated (operator-parity: every verb is also a hand edit) |
| Promote to storyboard | `distill(&BrainstormDoc) -> intake pre-fill / BibleDoc draft` — the EX1 tailor precedent, pure + deterministic |
| Trigger-spawned sessions | jobs system (JobBoard facts) + fresh provider contexts; results return as patches to the brainstorm doc |
| Provider counterpoints | assist-mode bake-off machinery, upgraded with pinned advocate/skeptic briefs + judge synthesis, disagreement-gated |

## 3 · Research: Clyffy graduated drive (web sweep 2026-08-06)

The operator's ruled rollout shape (§1) — manual gates on key parts →
evolving whitelist → tracked/trained approvals → pre-captured safe
workflows — matches the 2025-26 state of the art almost exactly. The
research names the mechanisms that make each stage work.

### 3.1 The five load-bearing patterns

1. **Canonical action signatures make the whitelist LEARNABLE.**
   (Claude Code's shipping loop: every gated action normalizes to
   `Tool(specifier)` before the prompt; "yes, don't ask again"
   synthesizes a rule into a settings file; compounds decompose into
   per-primitive rules.) For Clyffy: every app action gets a canonical
   verb + normalized-argument form (`Timeline.TrimClip(project:*)`,
   `Publish.Upload(channel:youtube)`), and every ledger entry records
   `{signature, offered generalization, task context, risk tier,
   decision (allow-once | allow-always | deny | edited-then-allowed),
   decider, timestamp, outcome, undone-within-N?}`. The whitelist is
   then DERIVED: promote a pattern to auto-allow after K consistent
   approvals with zero post-hoc undos, at the granularity the
   approvals establish. Persistence scales with risk class.
2. **Deny-first, layered evaluation — learned rules can never widen
   the floor.** (Claude Code, Progent arXiv 2504.11703, Invariant.)
   Order: policy hooks → deny → ask → allow → mode default; deny
   always wins; operator-authored denies live in a scope the learner
   cannot write to; a tiny hard-coded circuit-breaker list fires even
   in full-trust mode. The learned whitelist only narrows the ask-set
   inside a hand-set safe region.
3. **Tier actions by reversibility × blast radius; tier bounds what
   is learnable.** Consensus four tiers: T1 read-only (auto + log) ·
   T2 reversible internal writes (auto + audit) · T3 external reach
   (staged/delayed commit; each pattern needs explicit ratification) ·
   T4 irreversible — publish, delete, spend (PERMANENTLY ask-gated; no
   approval history ever promotes it — Operator's trained
   confirmations, GitHub's never-auto-merge). Key insight:
   reversibility is BUILDABLE — the app's undo journal moves actions
   down a tier, enlarging the delegable space more cheaply than
   smarter gating. Our undo stack + crash-safe document writers are
   already this.
4. **Pre-vetted building blocks are the vibecoder envelope.** (GitHub
   Agentic Workflows' "safe outputs": the unattended agent gets NO raw
   capabilities — only a closed vocabulary of pre-approved,
   parameterized operations; anything else is structurally impossible,
   not merely denied.) The operator's "workflows captured ahead of
   time" is exactly this: Clyffy-for-novices composes only registered
   workflow blueprints (the D6 presets-as-ingredients law already
   shapes these); power operators get the wider rule surface.
5. **Watch → attend → unattend, with out-of-band approval at the
   top.** Operator/RPA's ladder: watch mode first (agent acts, human
   present, everything ledgered — this IS the training-data capture
   phase); unattended only for learned-allowed tiers with exception
   escalation. Devin's refinement: the agent's own confidence is a
   second gate orthogonal to tier — low confidence forces an ask even
   on whitelisted actions, and confidence-vs-outcome correlation is
   measurable from the ledger. For T4, borrow CIBA's shape in-app: the
   approval arrives via a surface the agent cannot forge (a native
   dialog, never a chat message), names the exact action, and mints a
   single-use expiring capability — both identities ledgered.

### 3.2 Proposed mapping to the estate (PROPOSED)

| Piece | Seam it lands on |
|---|---|
| Action signatures + ledger | the signal ledger (`--ledger`/`DPL_LEDGER`) grown into an approval ledger; Fjall for the durable per-project store |
| Clyffy's action vocabulary | operator-parity law makes this cheap: every AI-drivable verb already exists as an operator action → the intent/undo vocabulary IS the signature census |
| Risk-tier registry | census-shaped (the coldlight/format-profile archetype): every drivable verb registered with its tier; `validate()` refuses unregistered verbs |
| Gates/approvals UI | bake-off popup machinery + a dedicated approval surface (NOT chat transcript — the unforgeable-channel rule) |
| Pre-captured safe workflows | registered workflow blueprints (D6); Clyffy-unattended may only mint runs of registered blueprints |
| Watch-mode training capture | jobs facts + ledger entries per driven action; replayable per the drill/EguiPilot harness precedent |

### 3.3 Sources (primary)

Claude Code permissions (code.claude.com/docs/en/permissions) ·
OpenAI Operator system card · Devin 2.1 confidence gating · GitHub
Agentic Workflows "safe outputs" · Progent (arXiv 2504.11703) ·
Invariant Labs trace guardrails · Knight Institute autonomy levels
(L1 Operator → L5 Observer) · CIBA async approval (OIDC) · MindStudio
four-tier action risk framework · FINOS AIR agent-decision audit.

## 4 · Open questions for ratification

1. Is the brainstorm surface a CHAT-inline surface (transcript
   components), a peer-window board (chat-is-a-peer-window precedent),
   or both with one document underneath?
2. v1 provider scope for counterpoints: which providers get the
   advocate/skeptic briefs first?
3. Does `BrainstormDoc` promote into the EX1 intake (pre-filling the
   8 fields) or straight to a bible draft?
4. Sequencing: does the brainstorm surface ride BEFORE EX1's M2/M3, or
   after the explainer pipeline proves the render leg end-to-end?
5. Clyffy T4 census: which actions are permanently ask-gated no matter
   the approval history? (Proposed floor: anything touching /content
   sources, publish/upload, delete outside undo reach, spend.)
6. Where does the approval ledger live — the existing signal ledger
   grown, or a sibling store beside it (Fjall)?
7. Promotion constant K (approvals with zero undos before auto-allow)
   and per-tier persistence windows — operator's call.
