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

### V1 — Backdrop repaint policy: spend the frames where they matter

**Ruling first (30e35e7, 2026-07-22; `docs/plans/port-backdrop.md`):** the
backdrop is SHELL-OWNED (`crates/backdrop`), painted app-wide under EVERY
route — that commit explicitly retired "B1's Home-subroute misread." The
shape does not reopen here: no route ever owns the backdrop, and no route
goes without it.
**Scope:** what the operator is cutting is the PER-FRAME spend, not the
paint. The one shell-owned backdrop gains a REPAINT policy: on the
Home/start route it runs its animated loop; on every other route it holds
a STATIC mood wash — the same mood-derived paint, zero per-frame
animation, zero repaint requests at idle. A mode of the one backdrop,
never route-scoped ownership.
**Deliverables:** the repaint policy inside the shell-owned backdrop
(which mode paints is decided per route; ownership never moves), a static
`mood_wash` paint path in `dpl-backdrop` or coldlight; mood state stays
global — the topbar mood tiles keep working everywhere (state stays,
animation doesn't).
**Evidence gate:** kittest/contract pinning "non-Home routes request no
animation repaints" AND "every route still paints the shell backdrop";
before/after idle repaint or CPU numbers in the pass plan §8; proof shots
of Home (animated) vs two other hubs (static wash) at 1100x700 and
1280x800.
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

### V3 — The icon language (REDIRECTED to Lucide; the engine is LANDED)

**Redirect (operator, 2026-07-29, verbatim): "Lucide for now that we
will style in neubrutalist and claymorphism variants."** The personal
SVG library becomes a LATER intake through the SAME pipeline — deferred,
not dropped. **Directive (operator, 2026-07-29, verbatim): "lets take
the time to pull lucide and then properly own it as part of coldlight,
we will have the SVG engine that is based off the airbnb engine airvx
and establish baselines for our diverse and default polished stylings."**

**M1 — LANDED (#163, double-green; ADR-015
`docs/design/adr-015-svg-icon-engine.md`; plan
`docs/plans/lucide-icon-engine.md`):** Lucide 1.27.0 vendored wholesale
(1,756 SVGs + LICENSE + VERSION); the zero-dep `dpl-svg` engine (the
icon subset of SVG → stroked meshes, corpus-tested against the whole
pool every run); `tokens/icons.json` as the 8th authored source (24
semantic ids, geometry baked at build); paint baselines ink / tone /
brutal; `LibSection::Icons`, the library's 10th wall
(`--lib-section icons`, proof `docs/proof/lucide-icons-gallery.png`).

**MOCK GATE (operator, 2026-07-29, verbatim): "We should have mocks
built first for a clear vision to work from."** The vision mock is
`docs/design/mocks/visual-language-v3-mock.html` — live ColdLight
ladders (the JS port of the build.rs curves), the pinned lucide@1.27.0
geometry, the ratified bases + stacks, the operator-named mixes
(neubrutalist/retro, claymorphism/cyber, cyber/retro), the PROPOSED
liquid-glass base, the icon grid in every treatment, and the V3b/V4
rail preview. The operator's reference set is `docs/design/references/`:
`aero-morphism.png` (formerly claymorphism-icons-examples.png, renamed
in #211; deep rim-lit clay, NEON-INK pills —
the nav-is-clay-buttons question, RESOLVED by the 2026-07-30 doctrine
ruling "clay IS a button"), `neubrutalist.png` (comic energy: bevel
slabs, halftone, starburst — all paintable), `claymorphism-cyber.png`
(metal machine-console — candidate Present-hub preset, own pass),
`WORKBENCH-editor.png`/`WORKBENCH-editor-2.png` (flat wells + neon +
glow-spotlight chrome for WiredFRONT / Guide Workbench). **NO STYLING
pass in this program
executes until the operator ratifies (or redlines) the mock.** That
gate holds M2 (the 40-call-site migration that makes rail/chrome
visibly wear Lucide), every treatment-variant pass, and V4/V5. Liquid
glass, once ratified, is its own trait-data pass.

**Remaining, in order (all behind the mock gate):** M2 migration
(`docs/plans/lucide-icon-engine.md` §4 M2); styled icon variants as
trait DATA; V3b — the rail wears them (pairs with V4); the
operator-SVG intake through the same generator (its own later pass;
sources never mutated, two-libraries law).
**Depends on:** the operator ratifying the vision mock.

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
| V1 | Backdrop repaint policy — animate on Home, static wash elsewhere (shell-owned, app-wide per 30e35e7) | not started | — |
| V2 | Mood card backgrounds (generated ingredient) | not started | V1 paint path |
| V3 | Icon language — Lucide, owned (redirected) | **M1 LANDED (#163)**; M2 + variants held | **the MOCK GATE** |
| V4 | Rail links become clear buttons | not started | the MOCK GATE (pairs with V3b) |
| V5 | Home showcase — clay + brutal, fast | not started | V1–V4; the MOCK GATE |

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
