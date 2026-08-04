# ADR-017 — The provider capability registry is a PLATFORM fact

**Status:** ACCEPTED 2026-07-31 (P1 merged, which ratified it) ·
**AMENDED 2026-08-03: §2's persistence choice was wrong — chat owns its
store; the estate does not hold chat's roster.** See Amendment below. The
capability model (§1, §3–6) is unchanged and stands.

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

### 2. It persists — SUPERSEDED, see the 2026-08-03 amendment

> The original text put the roster in the shared estate's `Tree::Agents`.
> That was wrong for the reason the amendment gives. The REQUIREMENT it
> was serving is still right and still binding:
>
> - a provider that was active last launch is known at boot, before any
>   probe has run — the UI starts truthful instead of empty and correcting;
> - derived facts (a live process handle) are NOT persisted. What persists
>   is what was true and cheap to re-verify.
>
> Only the STORE changed. Chat's own store now holds it.

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
after. They are cheap now and expensive to retrofit, and the seam already
publishes a registry. (Which store persistence uses is amended below.)

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


## Amendment 2026-08-03 — chat owns its store; the estate does not

**Operator ruling:** *"Chat should be its own store."* Research confirms
it, and it corrects a decision this ADR made.

### What was wrong

§2 put the capability roster in the shared estate's `Tree::Agents`,
reasoning that the partition already existed and was already labelled
*"agent (chat provider) session state"*. A reserved partition is not the
same as the right owner, and I did not check the second store.

Ground truth, measured 2026-08-03:

- `grep dpl_estate` across the entire `crates/chat/**` tree returns
  **zero**. Chat never touches the estate.
- `dpl-chat-profiles` keeps its own database —
  `.userdata/chat/provider-profiles.db` (turso) with **its own tokio
  runtime**.
- The only thing writing `Tree::Agents` is `app/src/chat.rs`, on chat's
  behalf, per this ADR.

So the effect of §2 was to spread CHAT'S OWN DATA across two stores while
chat itself owned a third path. That is the failure mode this project has
hit before under a different name: one fact, two homes, guaranteed drift.

### The rule, and why

Domain-driven design is unambiguous: a bounded context gets its own
datastore; sharing is acceptable only WITHIN a context, where cohesion is
high and the coupling is visible to one owner. Chat is unambiguously a
bounded context — its own crate tree, its own vocabulary, its own
lifecycle.

**Chat owns one store. The roster lives there.**

### How the platform still sees it — published language, not shared tables

Integration does not regress, because the platform never read the estate
for this anyway:

```
chat store (private)  ──hydrate at boot──▶  fabric.provider_roster()  ──▶  every surface
```

The shell's Providers popup already reads `fabric.provider_roster()`, not
the estate, so it keeps working unchanged. The seam's wire JSON is
projected from the fabric for the same reason. A reader consumes the
PUBLISHED fact; only chat touches chat's store. This is the standard
bounded-context integration and it is stronger than the shared partition,
because a second writer becomes impossible rather than merely discouraged.

### What this does NOT change

§1 (one writer), §3 (exposed to agents), §4 (capability SET, four
consumers), §5 (duplicate runs), §6 (no credentials, no dispatch) all
stand exactly as written. The model was right; its address was not.

### Consequences

- `Tree::Agents` becomes unused by this ADR. It is NOT repurposed or
  deleted here — the estate is operator data and that is a separate,
  deliberate migration.
- `chat/provider-profiles.db` and the roster converge on ONE chat store.
  That migration moves data the operator already has, so it lands as its
  own pass with its own plan, never as a side effect.
- **A new boundary is created**: "chat's store" becomes a named thing that
  only chat writes. Flagged explicitly because a new boundary is the
  operator's call, not a drive-by.

### Alternatives rejected

- **Leave the roster in the estate and move profiles there too.** Rejected:
  it makes the shared store the owner of a bounded context's private data,
  which is the coupling DDD warns about — and it would put chat's session
  state, transcripts, and persona bindings there next.
- **Three stores, one per concern.** Rejected: cohesion inside chat is
  high and it changes for one set of reasons. Splitting inside a context
  buys nothing and costs a migration each time.