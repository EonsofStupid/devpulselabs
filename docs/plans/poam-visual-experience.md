# POAM — the visual-experience program (backdrops, moods, icons, buttons)

Plan of Action & Milestones, ratified from the operator's directive
(2026-07-29, verbatim):

> "the animated background is for the landing the start page. home. then we
> have all of these moods that can be used for cards that we can easily
> create a background from that way we are not [spending] resources on
> animated/3d backgrounds except where it matters to me. I literally have
> years of svgs and content we can utilize for icons to go with our icon
> only mode for the left side bar. we want to have actual clear buttons for
> the links. and we want to demonstrate unique fast loading visuals with
> neubrutalist and claymorphism."

## Objective

Spend animation where it matters (the Home/start page), make everything
else CHEAP and instant (static mood-derived paint), put the operator's own
SVG library to work as the rail's icon language, make every link read as a
real button, and let the two ratified base treatments — NEUBRUTALIST and
CLAYMORPHIC — carry a visibly unique, fast-loading identity.

Ground truth (read this session):

- `crates/shell/src/lib.rs` `central()` paints `self.backdrop.paint(...)`
  under EVERY route — the animated backdrop runs everywhere today. That is
  the resource spend the operator is cutting.
- `cl::MOODS` holds ten authored colours (the Background-tab census, #149);
  `mood_gradient` + `cl::MOTION_PRESETS` mixes already exist — the raw
  material for static card backgrounds is authored and generated.
- Rail icons today are Phosphor font glyphs (`cl::icon::*`); there is no
  SVG pipeline in the workspace (`crates/vector` is mesh geometry, not SVG
  parsing). NOTE: the `landing` crate is the capture-landing POLICY, not
  the start page — the start page is the Home hub.
- Rail rows (`house_tile`/`sub_item`) are quiet hover rows, not buttons;
  the NEUBRUTALIST actionable treatment (hard offset shadow, crisp edge,
  press state — `launcher_chip` is the in-repo exemplar) exists but the
  rail does not wear it.

## Milestones

### V1 — Backdrop policy: animation is a Home privilege

**Scope:** the animated backdrop (loops + mood drift) paints ONLY on the
Home/start route. Every other route gets a STATIC mood wash — one painted
gradient from the session mood, zero per-frame animation, zero repaint
requests at idle.
**Deliverables:** a backdrop policy in the shell (route-aware), a static
`mood_wash` paint path in `dpl-backdrop` or coldlight; the topbar mood
tiles keep working everywhere (state stays, animation doesn't).
**Evidence gate:** kittest/contract pinning "non-home routes request no
animation repaints"; before/after idle repaint or CPU numbers in the pass
plan §8; proof shots of Home (animated) vs two other hubs (static wash) at
1100x700 and 1280x800.
**Depends on:** nothing. First mover.

### V2 — Mood-derived card backgrounds (a generated ingredient)

**Scope:** a card-background ColdLight ingredient generated from the mood
registry: ten flat washes + pair blends (the #149 combination math), each a
static paint — so ANY card can wear a mood background for free.
**Deliverables:** the ingredient authored as trait/element data (never
hand-rolled per-card), regenerated cleanly, surfaced in the manifest so the
Workbench editor offers it (gallery-gate law); presets feed the one
generated ingredient library — no one-offs.
**Evidence gate:** manifest shows the ingredient; a Workbench card and a
Home card both wearing mood backgrounds in proof shots; zero animation cost
demonstrated (no repaint requests from the card).
**Depends on:** V1's static wash paint path (shared code).

### V3 — The operator's SVG library becomes the icon language

**Scope:** ingest the operator's years of SVGs into an icon library that
the rail's icon-only mode (and beyond) draws from.
**Deliverables, in order:**
1. **Intake + curation:** operator drops source SVGs into a sources
   location; sources are NEVER mutated (house law) — derived assets file
   NEW. Two-libraries law respected: raw SVGs are source media; what the
   app consumes is a GENERATED in-app ingredient derived from them.
2. **ADR — the SVG→egui path:** decide rasterize-at-build/import (textures)
   vs tessellate-to-mesh (feeding `crates/vector`); any new crate or
   subprocess helper goes through the architecture checklist (ADR-009
   pattern if a helper binary is warranted). This ADR gates the rest.
3. **The icon registry:** icons become a generated, named registry (like
   status cells: adding one = adding one entry), so `HubSpec.icon` and
   other consumers can reference operator icons by id with the Phosphor
   glyphs as fallback.
4. **The rail wears them:** icon-only mode renders operator icons.
**Evidence gate:** ADR merged; registry test (unknown icon id fails the
build or degrades honestly); rail proof shots in icon-only mode with
operator icons at 1100x700.
**Depends on:** operator supplying/pointing at the SVG set (open input);
V4 pairs naturally (same surface).

### V4 — Actual clear buttons for the links

**Scope:** rail house tiles and subroute rows stop being quiet hover rows
and become REAL buttons — the NEUBRUTALIST actionable treatment: crisp
edge, hard offset shadow, hover lift, press-onto-shadow, tone accent —
clearly tappable in BOTH icon-only and expanded modes. Tooltip law holds
(every control tips in layman words).
**Deliverables:** button treatment applied via the trait system (authored
trait data, not bespoke paint); accordion behavior (one open house, unfold,
p4 scroll) unchanged; pilot metadata unchanged.
**Evidence gate:** proof shots both rail modes at 1100x700 + 1280x800;
feel-bar check against the original app's polish; existing rail kittest
contracts stay green.
**Depends on:** nothing hard; lands best with V3 step 4.

### V5 — The fast-visuals showcase: neubrutalism + claymorphism, demonstrably quick

**Scope:** the Home/start page becomes the demonstration surface: CLAY
(passive, soft-extruded) carries content cards over the animated backdrop;
NEUBRUTALIST (actionable) carries every control; CYBER/RETRO stack where
tone wants them. Everything painted — no blur, no textures fetched, no
load spinners: first frame is the finished look.
**Deliverables:** Home composed from V2 card backgrounds + V3 icons + V4
buttons; any missing trait variants authored as trait DATA in
`tokens/traits.json` and regenerated — never hand-rolled.
**Evidence gate:** cold-start proof (launch → first frame shot at
`--shot-frame` low N shows the full look, nothing popping in late);
maximized AND 1100x700 shots; the operator's feel bar.
**Depends on:** V1–V4.

## Sequencing & status

| # | Milestone | Status | Blocked on |
|---|-----------|--------|------------|
| V1 | Backdrop policy — animation only on Home | not started | — |
| V2 | Mood card backgrounds (generated ingredient) | not started | V1 paint path |
| V3 | SVG icon library + registry + rail | not started | operator SVG intake; ADR |
| V4 | Rail links become clear buttons | not started | — (pairs with V3) |
| V5 | Home showcase — clay + brutal, fast | not started | V1–V4 |

Every milestone is its own pass (or passes): plan-before-code
(`docs/plans/<pass>.md`, 7 sections + §8), the ColdLight gate before UI is
touched, PR → double-green → merge, proof on the real exe at real window
sizes, STATE.md updated.

## Risks / open questions

- **SVG intake location + volume** — where the operator's SVG years live
  and how many are curated for icons is the V3 gating input. Sources are
  never mutated; `/content` is never committed.
- **SVG rendering dependency** — a parser/rasterizer is a real dependency
  decision (crate vs ADR-009-style helper); the V3 ADR decides it against
  `docs/design/architecture.md`, amending the audit table in the same
  commit if a crate is added.
- **Backdrop coupling** — mood state feeds the topbar tiles and the rail
  unfold strip; V1 must decouple "mood as state" (kept everywhere) from
  "mood as animation" (Home only) without breaking either consumer.
- **Concurrent sessions** — the authoring lane (Background/Movement tabs,
  its own POAM branch) touches moods and ambients; V1/V2 must re-check
  `gh pr list` before authoring (adopt, don't duplicate).
