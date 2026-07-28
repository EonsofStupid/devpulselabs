# Authoring charter — the editor, the AI, and the library that grows itself

**Status: RATIFICATION DRAFT (2026-07-28).** The north-star contract for the
authoring EXPERIENCE — how a person (layman or operator) builds a show, and how
using the AI compounds the library instead of spending it. The catalog
ARCHITECTURE is owned by [`../design/design-system.md`](../design/design-system.md)
(elements → treatments → recipes → templates → fidelity); this charter defines
the **surface and the workflow** over it. The reference spec is the interactive
mock [`../design/mocks/authoring-surface-mock.html`](../design/mocks/authoring-surface-mock.html)
(the After-Effects-editor pattern: the mock IS the spec, ratified before egui).

Gate: every authoring-surface pass moves this charter's workflow forward and
leaves surfaces clean against `design-system.md` + `doctrine.md`.

---

## 0. The thesis

> **Use the AI → it registers the correct entity FIRST → the library compounds →
> next time it's already there.** You don't buy slides; you buy permanent
> capability. Every dollar and minute spent authoring leaves a reusable,
> registered entity behind.

The whole product bends around this loop. A one-off slide is a cost; a
registered recipe is an asset. The editor's job is to make the *asset* the
natural output of *every* action — so the library grows monotonically as a
byproduct of use, and the operator's leverage rises with each ask.

---

## 1. Four words, one comp — the user's whole vocabulary

The person sees **four buckets** and nothing else of the internal taxonomy:

| The user taps… | …which internally is |
|---|---|
| **Blocks** | assembled chunks — compositions above an element, below a slide (the Container tier + clusters) |
| **Elements** | the atoms — the DS2 census (`eyebrow · stat · badge · bar · divider · media`, sorted by Type/Mark/Meter/Divider/Media) |
| **Effects** | everything you *apply* — treatments (semantic/composite), recipes (interactions), motion, fidelity |
| **Templates** | full slide designs — the template registry (`title_card`, `quote`, `big_number`, …) |

The 7 census categories, semantic-vs-composite treatments, cost tiers, and
`SCHEMA_V` validation stay **invisible plumbing that sorts the raw into those
four words**. Simple front; rigorous engine. No jargon reaches a tile face —
just a live preview and a friendly name.

## 2. The authoring surface is OVERLAY-driven

No permanent side panels for authoring. The person clicks **New card** or an
**existing card**, and a focused **popup overlay** opens with a tab rail:

- **Content** — the guided, *constrained* fields (title/value, caption, tone).
  A tone can only come from the pool; text has a max; a fraction is a bounded
  stepper. You physically cannot enter something off-pattern.
- **Quote** — what the card is quoting, captured as a real **source ref**
  (quote · attribution · URL), never pasted prose.
- **Media** — paste a link or drop an image/video; it is **copied to the inbox
  and content-jailed** (ADR / three-ring law), never hotlinked, sources never
  mutated.
- **Interactions** — give the card a behavior from the recipe shelf.
- **Ask AI** — hand the card (or the gap) to the AI (§4).

Placement is **tap OR drag** — every library piece is both (touch build:
tap + long-press-drag, ≥48px targets, no hover-only reveals). What comes out
the back of every overlay is **clean, schema-valid, on-pattern authored data**
(`dpl_doc::Slide`, `SCHEMA_V`, every ref resolving) — visible live in a
**Captured** readout. The person authors *data*, never code, and cannot produce
an invalid slide.

## 3. Click-the-raw → a paste-ready prompt (the unique system)

The library is also a **prompt composer**. Clicking raw pieces + typing an
**intention** assembles a clean, structured brief the operator can paste into
chat — so the AI gets precise inputs, not vibes. The composed prompt carries the
card's captured data (content, treatment, quote, media) plus the intention and
any *wanted-but-missing* interaction, flagged `NEW`.

The embedded **chat becomes context-aware**: it knows which card is open, and
offers the **workflows and skills** relevant to it (add interaction, restyle,
generate variants). "Copy prompt" or "Generate" in place.

## 4. The architecture-first AI law (the crux — enforceable, not a convention)

> When the AI must create something that does not yet exist, it creates the
> **correct entity FIRST — registered as its proper kind** (recipe / preset /
> effect / element) — and only THEN the composition that uses it.

This is a **law**, not prompt etiquette, because the census makes "correct
entity" *checkable*:

- A recipe that isn't in the `dpl-slides` recipe registry **does not exist**
  (Census law, `architecture.md` §4). An element absent from `blocks::elements()`
  **does not exist**. A treatment not in `tokens/treatments.json` **does not
  resolve**.
- So a generated slide that references `recipe:"explode-reveal"` **fails
  validation** until that recipe is a registered, unit-tested, cost-tiered
  entity. The AI literally cannot hand back a slide with a phantom interaction.

The worked example (in the mock): **"slides explode on contact and reveal the
answers"** → the AI's plan runs in order — **① register the `explode-reveal`
recipe** (pure state machine, tested, `cost_tier`) → **② build the card** that
wires it → **③ library grows +1**. The recipe is now reusable forever.

## 5. The compounding library (the economics)

Every AI ask leaves a **registered, reusable entity**. The **Library count ticks
up** and the new entity appears in its bucket immediately. Over time the editor
and the catalog thicken with the operator's own recipes, presets, and blocks —
built once, by AI, on the operator's dime, and owned forever. This is the
$-and-time maximization: the marginal ask gets cheaper because last month's ask
is this month's building block.

## 6. How it sequences onto the build order

This charter rides the existing program; it invents no architecture it doesn't
already have a home for:

1. **Spine — DONE.** DS2 element census (PR #114) makes "correct entity"
   checkable for elements; the template registry already does for templates.
2. **Effects = design-system step ④** — the `Recipe` trait + registry in
   `dpl-slides`; the starter recipes (`reveal/toggle/flip/match/poll/peek`) plus
   the operator's AI-grown ones. Treatments = step ③. This charter is the
   *reason* those steps exist as first-class registries.
3. **Overlay editor** — rides the ratified After-Effects editor
   (`w1`/`r1u`/`wiredfront-editor-mock.html`) + the composer insert seam
   (`w2`/`w6` media windows + import) — the overlay is a new authoring shell
   over the same import/validate spine.
4. **Prompt composer + entity-first AI** — a new pass over the **C-series agent /
   touchpoint seam** and the **chat window product vision** (`chat-*`): the
   context-aware chat, the prompt assembler, and the entity-first generator that
   commits registrations before compositions.

Near-term first pass to name in its own plan doc: **the overlay authoring shell**
(New/Edit card → tabbed popup → guided capture), because it's the surface every
other piece hangs off and it reuses spine that already exists.

## 7. Non-negotiable guardrails

- **Census law** — nothing unregistered can be referenced; the AI registers
  before it composes.
- **Content-jail** — media is copied into the working root, never hotlinked;
  sources are never mutated (three-ring law).
- **Schema-valid capture** — `dpl_doc::Slide` + `SCHEMA_V`; a card that can't
  validate can't save.
- **Layman never sees internals** — the four words are the whole surface;
  categories/tiers/composite-treatments stay hidden.
- **Touch-first** — tap + long-press-drag, ≥48px targets, tooltips on controls,
  no hover-only paths (the 75″ prop is a first-class target).
- **ColdLight only on surfaces** — colour from tones, size from Fluid, no blur,
  hard shadows (`doctrine.md` owns the mechanics).

## 8. Open decisions (for ratification)

1. **Blocks vs Elements — the exact line.** Proposal: Elements = atoms (census);
   Blocks = assembled multi-element chunks below a full slide (Container tier +
   clusters). Confirm before the palette shelves both.
2. **Where the composed prompt goes.** Clipboard (paste into any chat) vs
   direct-to-embedded-chat (the aware chat runs it in place). Proposal: both —
   Copy for portability, Generate for the in-app path.
3. **Starter recipe set** the AI can compose without inventing — likely the
   design-system §3 set (`advance/toggle/flip/match/poll/peek`); everything else
   is AI-grown-then-registered.
4. **How "skills" surface** in the aware chat — a fixed workflow menu per card
   kind, vs discovered from the registries. Proposal: discovered, so new entities
   auto-appear as skills (the compounding loop extends to the chat itself).

## 9. Acceptance

This charter is ratified when the operator signs off §§1–5 and the four open
decisions (§8). Thereafter every authoring-surface pass cites it, moves its
workflow forward, and proves on the real exe (overlay screenshots at real window
sizes; a live AI-entity-first round that ends with the library count moved and
the new entity registered + census-tested).
