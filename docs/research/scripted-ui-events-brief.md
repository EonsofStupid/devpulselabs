# Scripted UI events — focused standards research brief

2026-11-02. Operator clarification R295. Proposed research scope; no new
protocol, implementation or production boundary is ratified by this brief.
Parent evidence: [guided workflow review](guided-workflow-standard-review.md).

**Adoption assessment:** [how this could become a standard](ui-procedure-standard-adoption.md)
supplies the researched implementation/adoption strategy. It narrows this brief:
first test a UI-domain profile over existing workflow semantics, independent
implementation and reusable distribution terms; a new event language is not
currently justified.

## Product contract: easy by default, inspectable when needed

The user should be able to **Record → Describe the goal → Review the draft →
Try it → Use or publish it**. AI extracts steps, proposes parameters, suggests
triggers and drafts explanations. The guided UI asks concrete questions only
where intent or outcomes cannot be inferred reliably. Users do not need to
author JSON, selectors, state machines or event wiring.

Advanced/devtools UI progressively exposes targets, trigger rules, parameters,
before/after observations, branches, timing, trace playback, budgets and failed
checks. It edits the SAME artifact as the friendly UI. There is no separate
expert-only workflow implementation. Technical complexity belongs in the
engine; uncertainty and errors should produce understandable recovery choices.

The recording can be fed into authoring directly or selected from a saved
project. Changes regenerate a proposed procedure revision and a preview.
Testing reveals what would happen and what proves completion. Publishing
derives a website guide; using the procedure offers recorded watch, live AI
demonstration, human follow and execute, as described in the parent review.

## Research question

What minimum interoperable contract lets an A2UI action initiate a reusable,
state-checked app procedure and receive small, truthful UI updates, without a
model deciding every step? A2UI is one binding; equivalent native controls,
authorized agent requests and explicitly enabled app-state triggers must be
able to use the same execution contract.

## Command and fact distinction

Proposed path:

`A2UI interaction → validated intent → typed command → workflow executor →
observed outcome → domain signal → component data update`

A request to save is not a fact that saving succeeded. Preserve the existing
[signal doctrine](../design/signal-doctrine.md): commands use direct typed
calls/reliable delivery; domain facts publish after the corresponding mutation.
The Rust fabric stays per-domain and serde-free. External serialization belongs
at versioned boundaries, not in a second global string-event bus.

AI may propose or revise a trigger/step from current context, but the runtime
validates the revision and its authority before activation. A feedback message
cannot silently install actions or claim an outcome. Existing authorized
procedures can run without new model inference or repeated authoring approval.

## Initial standards grounding

- [A2UI actions](https://a2ui.org/concepts/actions/) already distinguish local
  renderer functions from events routed to the agent. Local behavior need not
  make a network round trip. Research which workflow operations fit a trusted
  catalog function and which should route to a deterministic backend; an
  agent-facing route need not imply an LLM inference for every interaction.
- [W3C SCXML](https://www.w3.org/TR/scxml/) already specifies state-machine
  execution and event-driven transitions. Compare its execution semantics,
  guards and history with our procedure needs before inventing equivalent
  behavior. This does not propose importing XML or a scripting engine into
  the native app.
- [CloudEvents](https://github.com/cloudevents/spec/blob/main/cloudevents/spec.md)
  defines event records and contextual metadata. Evaluate an external mapping
  for observed facts; an event envelope alone does not define permissions,
  executable UI actions or successful task completion.
- Existing MCP Tasks/Skills and A2UI presentation bindings are covered in the
  parent review. Preserve them where applicable; identify a demonstrated gap
  before proposing a protocol extension.

## What the proposed profile must settle

| Topic | Research output |
|---|---|
| Identity and types | Versioned procedure/action/capability identities, typed parameters and catalog bindings. |
| Trigger semantics | User/agent/app provenance, scope, edge vs level, debounce, reentrancy, deduplication and prevention of self-trigger loops. |
| Execution | Preconditions, outcome predicates, sequencing, bounded waits/retries, cancellation and resume. |
| State evidence | Instance/window scope, freshness, correlation and explicit unknown/unsupported states. |
| Authority | Which triggers may start which effects; capability limits remain enforced by the host. |
| Efficient feedback | Reusable component templates, subscribed state deltas, bounded optional AI text and deterministic fallback. |
| Delivery reliability | Backpressure; coalesce transient progress but preserve outcome records; reconnect and recovery semantics. |
| Learning/publishing | Same step identity across live execution, observed practice and hosted media; public derivatives omit private trace content. |
| Conformance | Two adapters interpreting the same procedure and fault cases consistently; calls/tokens, latency and payload measurements. |

“Optimized signals” means fewer unnecessary model calls and state transfers,
typed routing, reuse and bounded work. A compact encoding alone is insufficient.
Do not promise exactly-once external effects unless the target supports the
required transaction or deduplication contract.

## Next research artifact

Produce a comparative gap matrix and a small normative draft with worked event
traces: normal completion, duplicate trigger, wrong window, delayed outcome,
cancellation, restart and dynamic revision. Pair it with mockups of the guided
authoring flow and advanced inspector. Then validate the draft through the
parent review's native vertical slice and second adapter. This brief separates
the standards research workstream; it does not fork a repository or publish a
standard.
