# ADR-017 — The provider capability registry is a PLATFORM fact

**Status:** PROPOSED 2026-07-31. Filed BEFORE P1 is built, deliberately —
P1's planned shape was wrong for this and would have had to be rewritten.
Merging P1 ratifies this ADR.

## Context

H1 (`951fb2a`) made local capability count as real activation: a provider
whose transcripts are on this machine is `Experimental` with
`ReadSessions`/`PageHistory`, earns a tab, and its chats are rows. That
answered *"the chats in one system"*.

The operator then named what activation is actually FOR (2026-07-31):

> "this is going to also cross link into the main configuration stores for
> the platform right so I need to make sure you understand that when a
> provider subscription becomes active it's active in the platform a 2ui
> the evolving UI all of that comes into play it also allows all of this to
> then properly have our workflows where depending on the objective it may
> be a full workout individuals skills and mixture of experts required and
> then hit each one of those beats from the moment of brainstorming in
> processing it for actual development"

and where it ends:

> "we're going to have you know A and B test … where we even have
> duplicates running in parallel so then we can pick whatever one that
> relates to us … this is how we do our proper tuning and fine tuning that
> can later then be used for llm training so we cannot be 100% relying on
> all just Frontier AI where we have the general conversational aspect the
> managed by are finally tuned version with clyffy"

### What P1 was planned as, and why that was wrong

`docs/plans/chat-providers-become-platform.md` scoped the roster to two
booleans (`can_send`, `can_read_history`) folded onto the fabric, with a
shell status chip as its consumer. That is a chat feature wearing a
platform's clothes. It cannot answer *"who can do X"*, it does not persist,
it is invisible to the agent seam, and it captures nothing. Every consumer
below would have forced it to be rewritten — and it is the thing they all
depend on, so the rewrite would have been the expensive kind.

**This ADR is filed first because the registry's SHAPE is load-bearing for
four separate consumers that do not exist yet.**

## Decision

**Provider capability is ONE platform fact with ONE writer and four
consumers.** Not a chat detail that other things peek at.

### 1. One writer

The chat host is the only writer. It owns the runtime, the activation
coordinators, and the probes, so it is the only place that can be
authoritative. Every other crate READS. A second writer is how a registry
starts lying.

### 2. It persists — the config store cross-link

The estate ALREADY reserves the partition: `Tree::Agents` —
*"Agent (chat provider) session state for the HTTP seam"*
(`crates/estate/src/lib.rs:35`). Activation writes there via
`put_json`/`get_json`, so:

- a provider that was active last launch is known at boot, before any
  probe has run — the UI starts truthful instead of empty and correcting;
- the seam and the app read ONE store rather than two views of it.

Derived facts (a live process handle) are NOT persisted. What persists is
what was true and cheap to re-verify.

### 3. It is exposed to agents

`crates/seam/src/lib.rs:72` already describes its job: *"The registry as
wire JSON — what an agent reads to learn the map."* The capability registry
belongs in that map. An agent that cannot see which experts exist cannot
route to them.

### 4. The four consumers, and what each needs from the shape

| consumer | the question it asks | what the shape must carry |
|---|---|---|
| **A2UI / the evolving UI** | "what may I offer right now?" | capability presence, so a surface renders an affordance only when something can honour it — and REMOVES it when the provider signs out |
| **Workflows / mixture-of-experts** | "who can take this beat?" | capability→provider lookup, per `StageKind`. `dpl-workflows` already has `WorkflowDoc`/`Stage`/`StageKind`/`Beat` — the operator's "beats" is literally the type. Routing needs SKILLS, not booleans |
| **A/B duplicate runs** | "run this beat on N providers at once" | the ability to name a SET of providers that can all take the same beat, and to keep their outputs distinguishable |
| **Training capture (Clyffy)** | "which answer did the operator keep?" | a stable provider identity on every captured run, so preference data has a subject |

**Consequence for the type.** A boolean pair cannot serve rows 2–4. The
registry carries a **capability SET** per provider (the existing
`dpl_chat_core::Capability` vocabulary — `ReadSessions`, `PageHistory`,
`StreamTurns`, `Tools`, `Images`, `Video`, `Plans`, …), plus identity, a
truthful limitation note, and the activation state. Booleans like
`can_send` become derived helpers, never the stored form.

### 5. Duplicate runs are a first-class capability of the platform

The operator's A/B-with-parallel-duplicates is not a test harness bolted
on. It is how tuning data is produced, and it is the reason the platform
must not be 100% frontier-dependent: **the captured preferences are what
fine-tunes Clyffy.** So:

- a beat dispatched to N providers is a normal dispatch, not a special mode;
- every run carries provider identity + the capability set it was selected
  for, or the capture cannot be attributed later;
- the operator's PICK is the label. Capturing the pick is the point.

This does not get built by this ADR. It is recorded here so the registry is
not shaped in a way that makes it impossible.

### 6. What stays out

- The registry states CAPABILITY, never CREDENTIALS. It says "this provider
  can stream turns"; it never holds a token, cookie, or key. Presence is
  probed via the vendor's own tool (see the Providers program map §2.1).
- The registry does not dispatch. Routing and MoE selection are consumers,
  landed later, on top of a shape proved by use.

## Consequences

**P1 is re-scoped** — see `docs/plans/chat-providers-become-platform.md`,
amended in the same commit as this ADR. Its first consumer stays small (the
shell naming what is available) because a publication with no consumer
proves nothing, but its TYPE is now the platform shape, not a chip's
convenience.

**Order changes.** Persistence and the seam projection land WITH P1, not
after. They are cheap now and expensive to retrofit: `Tree::Agents` already
exists, and the seam already publishes a registry.

**The layer law is unaffected.** `docs/design/architecture.md:65` already
ratifies `dpl-fabric → dpl-chat-core`, so the capability vocabulary crosses
with no new dependency edge and nothing for the `structure` audit to fire
on. This is the test of a design that fits the stack rather than bending it.

**What this ADR forbids:** any surface writing capability, any capability
type that cannot express a SET, and any persisted credential. Each of those
is cheap to avoid now and structural to undo later.

## Alternatives rejected

- **Keep P1 thin, widen later.** Rejected: four unbuilt consumers all bind
  to this shape, and the widening lands after they exist — which is the
  expensive order. H1's own lesson is that the cost is paid where the shape
  is wrong, not where it is discovered.
- **A second registry for platform use, beside chat's.** Rejected: two
  registries drift, and the operator's whole point is that activation is
  ONE fact.
- **Put routing in the registry.** Rejected: it is a consumer. D1
  established read-only-before-verbs and it held; the same discipline
  applies.
