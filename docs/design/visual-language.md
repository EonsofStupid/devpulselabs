# THE VISUAL LANGUAGE — one source of truth

**Status:** CONSOLIDATION DRAFT, authored 2026-08-04 at operator direction:
*"we do not move forward until we have my vision captured and our
documentation merged into cohesive source of truth."*

This file exists because the visual language is currently spread across
two mocks that **disagree**, a doctrine that **omits** the base the
editor depends on, four reference images with **contested ownership**,
and two ADRs where one is "partially superseded at filing". This
document merges them, names every conflict with evidence, and marks each
one **RULED** or **OPEN**.

**Nothing here invents a ruling.** Where the operator has not spoken, it
says OPEN and stops. Every claim cites the file and line it came from,
read in full in-session 2026-08-04.

---

## 1 · The vision, as the operator has stated it

Direct quotes, newest last. These outrank every inference in this repo.

- **2026-07-29:** *"lets take the time to pull lucide and then properly
  own it as part of coldlight, we will have the SVG engine that is based
  off the airbnb engine airvx and establish baselines for our diverse and
  default polished stylings."*
- **2026-07-29:** *"Lucide for now that we will style in neubrutalist and
  claymorphism variants."*
- **2026-07-29:** *"We should have mocks built first for a clear vision
  to work from."*
- **2026-07-30:** *"clay IS a button"* (retired the port's inference).
- **2026-08-04:** *"The entire purpose of getting icons in this platform
  are to have them properly embedded and get my editor actually looking
  at least close to my mock. the gallery(s)/library(s) are suppose to
  allow us to effortlessly symlink them and then cache."*
- **2026-08-04:** *"align to the mock and get this actually being an
  editor where I have the ability to browse and link outside of the app
  to shares/and drives."*
- **2026-08-04:** *"that reference needs to be clearly associated with
  workbench … the neubrutalist is a styling preset, the same with the
  other two images, and then … the v3 visual style language are what you
  need to align to for this."*

### The reference set — what each image IS

`docs/design/references/` (read as images 2026-08-04):

| File | What it shows | Operator's association |
|---|---|---|
| `editor.png` **(WORKBENCH — operator 2026-08-05, being renamed)** | "WIRED FRONT EDITOR" — deep-navy pro app; CAPTURE/ANNOTATE/EDIT/NARRATE/PREVIEW/EXPORT icon-over-label nav; left SCENES+ASSETS; right INSPECTOR/LAYERS/ACTIONS with form fields; bottom STEPS strip + narration waveform + transport. Restrained cyan/purple glow. A **"COLDLIGHT VOLTAGE" theme picker in the top bar.** | **WORKBENCH.** *Was recorded here as the WiredFRONT target — that was wrong.* |
| `editor-2.png` **(WORKBENCH)** | "DevPULSE Guide Workbench" — same family, more refined; categorized ASSETS with counts; STEP INSPECTOR with sliders + toggle; cyan glow rings on the active element; big cyan circular Play. | **WORKBENCH** — its own name says so. |
| `neubrutalist.png` | "COLDLIGHT DESIGN SUITE v1.0" — dark hatched ground, **extruded** nav plates, and **exactly ONE** saturated hot-pink hero button ("START SYSTEM") with a starburst. | **A styling preset** |
| `claymorphism-cyber.png` | Teal/chrome machine console — riveted metal frame, gauges, bracketed tabs. | **A styling preset — belongs to WORKBENCH** |
| `claymorphism-icons-examples.png` | Deep clay tiles/pills with neon ink. | **A styling preset** |

**THE WIREDFRONT EDITOR'S TARGET IS A MOCK, NOT A REFERENCE SHOT** — operator
2026-08-05: *"its fucking this … EDITOR.PNG IS FUCKING PART OF WORKBENCH IM
RENAMING THAT RIGHT NOW."*

| target | what it governs |
|---|---|
| **`docs/design/mocks/wiredfront-editor-mock.html`** | **the WiredFRONT editor — layout AND treatment** |
| `visual-language-v3-mock.html` | the shared visual language (palette, bases, materials) |
| `editor.png` · `editor-2.png` | **Workbench**, not WiredFRONT |

**This invalidates a derivation.** `visual-language-v3-mock.html:401` claims
*"the two editor shots map to flat wells + liquid-glass/cyber chrome + glow
spotlights (WiredFRONT, Guide Workbench)"* — that sentence is the sole origin
of the aero/`neon-glass` reading for WiredFRONT, and its premise (that those
shots are WiredFRONT) is false. **The v3 mock line must be amended.**

**The load-bearing observation from `neubrutalist.png`:** the saturated
brutalist fill is the **hero** treatment — used *once*, for the primary
action. Everything else on that screen is a dark extruded plate. A
surface where *every* control is a saturated pill is not this reference;
it is drift.

---

## 2 · The palette — and the first contradiction

### C1 — The two mocks disagree on the base palette · **OPEN**

| Slot | `visual-language-v3-mock.html:10` | `wiredfront-editor-mock.html:9` | Code (`coldlight/src/lib.rs:83-87`, main) |
|---|---|---|---|
| well | `#07090e` | `#0a0c10` | `#07090e` |
| base | `#0a0c12` | `#12151c` | `#0a0c12` |
| raised | `#11141d` | *(absent)* | `#11141d` |
| overlay | `#181c27` | `#1a1e28` | `#181c27` |
| hairline | `#232836` | `#2a2f3d` | `#232836` |

The v3 mock's CSS comment says these are *"VERBATIM
crates/coldlight/src/lib.rs"* — and they are. **The editor mock is on a
different, lighter palette.** Two ratified mocks cannot both be the
palette.

**RULED 2026-08-04** — operator: *"for now align to v3 visual then but
technically both should be in the system in the matrix."*

- **v3 is the alignment target now.** It matches shipped code
  (`coldlight/src/lib.rs:83-87`) exactly.
- **The editor mock's palette is NOT discarded.** Per the ruling it
  becomes its own coordinate/preset in the matrix — a second named look,
  reachable later. Its *layout* stays authoritative for the editor
  regardless.
- Neither palette is deleted; the matrix is what lets both exist.

### C7 — A stranded branch would move these exact values · **OPEN / URGENT**

`feat/lucide-m2-migration` commit `f816501` replaces the hand-typed
surface/ink consts with generated ladder refs (`tokens/slots.json`).
Its own commit message records the visible shift: *"surfaces move ΔRGB
≤7, HAIRLINE +27 (brighter), ink::SUBTLE −60 (dimmer)."*

Those are **the numbers the v3 mock ratified verbatim.** Landing that
branch silently re-tints the ratified language. It must be an operator
decision, not a merge side-effect. (The slots work is otherwise correct
and worth landing — main still hardcodes 11 hex literals.)

---

## 3 · The treatments — what is law, what is missing

`crates/coldlight/tokens/traits.json` contains **exactly four** ids:
`neubrutalist · claymorphic · cyber · retro`.

`doctrine.md:33-55` ratifies those four and nothing else:

- **NEUBRUTALIST** (base, actionable) — fill `tone 400`, edge `2px tone
  700`, radius 6, hard `[3,3]` shadow blur 0, painted back-glow `tone
  300`. Ink is `surface::WELL` (dark ink on a bright plate).
- **CLAYMORPHIC** (base, **actionable** since the 2026-07-30 ruling) —
  soft extrusion, directional double shadow, generous radius, hairline
  edge. *Doctrine records that clay's `hover`/`active` state data is
  **still missing** from traits.json — "a gap, not a rule."*
- **CYBER** (stack) — thin cyan edge, cold ink, top wash, ring bloom.
- **RETRO** (stack) — chunky amber edge, r3, scanlines, black-rimmed
  lettering.

### C4 — trait vs preset vs matrix · **RULED 2026-08-04**

**Operator:** *"for now align to v3 visual then but technically both
should be in the system in the matrix."*

**THE MATRIX IS THE SPINE.** `docs/design/coldlight-matrix.md` is the
model; traits are its legacy surface. Both visual languages (v3 and the
editor mock's) are **coordinates in the matrix**, not rivals — neither is
deleted. Align to v3 **now**; the editor-mock palette is expressed later
as its own coordinate/preset rather than discarded.

This supersedes the "three rival vocabularies" framing in the previous
draft. The relationship, from `coldlight-matrix.md` §1, §6, §8:

- **matrix** = the faceted classification (PMEST: ANCESTOR × STRUCTURE ×
  MATERIAL × ATMOSPHERE, + runtime STATE). Ancestor owns `font`;
  structure owns `radius`+`edge.width`; material owns `fill`+`ink`+
  `edge.color`; atmosphere owns `shadows`+`back_glow`+`tints`+`anim`.
- **traits** = the four legacy values, which the matrix **reclassifies**
  (§8): `neubrutalist` → `brutalist·massive·raw·hard-cast`;
  `claymorphic` → `aero·rounded·clay·bloom`; `cyber` →
  `atmosphere:neon`; `retro` → `atmosphere:scanline`. A test is specified
  to prove the resolved style is **byte-identical** — a reclassification,
  not a restyle.
- **preset** = *"a named coordinate plus the facets left free"* (§6).
  So the operator's "styling presets" (the reference images) are
  **named coordinates**. That reading is now confirmed by the ruling.

Already ratified in `coldlight-matrix.md` §11 (do not re-litigate):
eight seeds (`bauhaus · art-deco · brutalist · swiss · aero · memphis ·
constructivist · editorial`); `swiss` not `frutiger`; the ~120-face type
pool as a starting point; `aero` interpreted **no-blur**.

### C2 — "Liquid glass" is NOT a missing base · **RESOLVED**

The previous draft called this a blocker. It was wrong, and the matrix is
why. **The editor's look is already expressible with zero new authored
values.** From `tokens/matrix.json` (read in full 2026-08-04):

| Need (v3 `:401`) | Already authored |
|---|---|
| "liquid-glass chrome" | **`material: glass`** (`:32`) — *"Translucent pane; gloss via banded gradient, never a blur (doctrine)"*; fill `surface OVERLAY`, ink `STRONG`, edge `tone 200` |
| "cyber chrome" | **`material: neon-glass`** (`:36`) — *"Cold translucent skin — cyan ink on a lit edge"*; ink `cyan 100`, edge `cyan 400` |
| "glow spotlights" | **`atmosphere: neon`** (`:56`) — double cyan back-glow (spread 14 + 28) + cyan-500 tint, painted falloff, zero blur |
| "flat wells" | the `WELL` surface slot, unchanged |

The v3 mock proposed liquid glass as a *fifth trait* only because the
four-trait system had no vocabulary for it. Under the matrix it is a
**material**, and it is already there.

**The legality data even picks the ancestor.** Only **`aero`** allows
`glass`/`neon-glass` — `bauhaus`, `art-deco`, `swiss`, `constructivist`
and `editorial` all list them `forbidden`; `brutalist` allows
`neon-glass` but forbids `glass`; `memphis` forbids both.

> **RULED 2026-08-05 (corrected in-session) — the editor's default is
> `aero` wearing the `hacker` flavor.**
>
> The operator first said *"we will default to neo first the lime green"*,
> then corrected it: *"i was wrong to state default is brute neo while that
> will exist as an option i agreed v3 but as aero hacker."*
>
> **Both statements are recorded on purpose** — the correction is the ruling,
> and the first is kept so nobody re-derives it later and thinks the doc
> drifted.
>
> - **DEFAULT: `aero` + `hacker`** — which resolves to the coordinate below.
>   This agrees with the C1 ruling (*align to v3*) and with v3 mock line 401,
>   so the mock and the operator now say the same thing.
> - **`neo-brute` is an OPTION**, not the default — one of brutalist's three
>   seeds (§3b), selectable, not the editor's dress.
> - Per §3b, the authoring unit is **(ancestor, flavor)**, not a bare
>   coordinate. `aero · measured · neon-glass · neon` is what `aero+hacker`
>   RESOLVES to; it is not itself the thing being chosen.
>
> **Editor coordinate — `aero · measured · neon-glass · neon`**
> Checked legal against `matrix.json:91-95` — aero allows structure
> `measured`, materials `glass`/`neon-glass`, atmosphere `neon`.
> That is precisely "flat wells + liquid-glass/cyber chrome + glow
> spotlights." **Needs operator confirmation of the coordinate, not new
> authoring.**

### C2b — The REAL blocker: the matrix has no resolver · **BLOCKER**

`crates/coldlight/src/lib.rs:1216-1254` `pub mod matrix` includes
`matrix_gen.rs` and exposes **`ANCESTORS · STRUCTURES · MATERIALS ·
ATMOSPHERES · RULING_COUNT`** with a census test (8 ancestors × 21
values = 168 rulings, build-enforced). That is **vocabulary and legality
only**.

`coldlight-matrix.md` §7 specifies:

```rust
pub fn resolve(c: &Coordinate, state: TraitState) -> ResolvedStyle;
```

**It does not exist.** There is no `Coordinate` type and no `resolve`.
Every surface still calls the legacy
`compose(tone, base, stack: &[&str], state)` over the four traits — the
exact untyped-stack defect the matrix was designed to close
(`coldlight-matrix.md` §1).

**So the editor cannot wear its coordinate until `resolve()` lands.**
This is the one genuine piece of missing machinery, and it is far better
defined than "author a liquid-glass trait."

### C2c — superseded framing (kept so the reasoning is auditable)

`visual-language-v3-mock.html:401`, verbatim:

> "the two editor shots map to **flat wells + liquid-glass/cyber chrome +
> glow spotlights** (WiredFRONT, Guide Workbench)"

And `visual-language-v3-mock.html:76`:

> "LIQUID GLASS (base, **PROPOSED — not yet trait data**)"

So: **the editor's ratified chrome is liquid-glass/cyber over flat
wells — and liquid glass has never been authored.** It appears in the v3
mock and is entirely absent from `doctrine.md` and `traits.json`.

**Consequence, recorded honestly:** the icon pass shipped on
`feat/editor-icons` (commit `2b0dacc`) skinned the editor's controls with
`cl::brutal` / `cl::treated(neubrutalist)`. Per this section that base is
**wrong for the editor**. The icons themselves are correct and
independent of the base; the *treatment* must be redone once liquid glass
is authored. This is drift introduced by that pass and is named here
rather than left to be discovered.

The v3 mock's own sequence (`:403`) puts it in order:
> V3a (registry, no UI) → V3b+V4 (rail wears icons) → **liquid-glass
> trait pass** → V5 Home showcase.

### C4b — the matrix's own open item · **carried forward**

`coldlight-matrix.md` §11.5 leaves the facet vocabularies (6 structures,
9 materials, 6 atmospheres) as *"a starting point, extended as the work
proves gaps"* — the working rule that vocabularies are approved as
starting points and revised in flight, while only STRUCTURAL decisions
(axis partition, preset-by-reference, legality-as-data) need ratifying up
front. Nothing here needs re-ratifying.

### C3 — `claymorphism-cyber.png` ownership · **RULED 2026-08-04**

- v3 mock `:401`: "the metal machine-console look
  (`claymorphism-cyber.png`) is a candidate flavor for the **Present /
  show-machine** surface, its own later pass"
- **Operator, 2026-08-04: it is associated with WORKBENCH.**

**Operator's word supersedes.** The v3 mock's line is superseded and must
be amended. Recorded here as the ruling.

---

## 3b · FLAVORS cut across ancestors · **operator, 2026-08-05**

> *"the aero morphism is fine but it needs to actually exist and then you need
> to realize that is actually going to become Hacker subpreset for aero"*

The seed names are **not** brutalist's private list. They are a **second axis
— a flavor** — that each ancestor expresses in its own lineage:

| flavor | under `brutalist` | under `aero` |
|---|---|---|
| **neo** | lime plate, 3px black edge, 6px hard shadow | *(to author)* |
| **hacker** | terminal: mono, hairline rule, zero radius, zero shadow | **neon-glass + cyan glow — the `editor.png` look** |
| **retro** | Win95 bevel chrome | *(to author)* |

So `aero · measured · neon-glass · neon` — the coordinate this document
proposed for the editor — **is not a one-off.** It is **`aero` wearing the
`hacker` flavor**, and it should be authored as such rather than as a bare
coordinate. That is what makes it reusable instead of a special case.

**Consequence:** a coordinate is not the authoring unit the operator works in.
The unit is **(ancestor, flavor)**, which RESOLVES to a coordinate. That is
`coldlight-matrix.md` §6's *"a preset is a named coordinate plus the facets
left free"* — flavors are those presets, named consistently across lineages.

**Still OPEN:** `aero` exists in `matrix.json` as vocabulary and legality only
— like every other ancestor, it has **no authored values reaching the runtime**
(the `generate_matrix` payload-discard defect, C2b). *"It needs to actually
exist"* is therefore the same blocker as everything else: `resolve()` plus
payload emission (`docs/plans/matrix-resolve.md`).

## 4 · Surface → treatment assignment

Consolidated from the above. **Bold = operator-stated. Italic = inferred,
needs confirmation.**

| Surface | Base | Stack | Source |
|---|---|---|---|
| **WiredFRONT editor** | **`aero` + `hacker` flavor** → resolves to `aero · measured · neon-glass · neon` | glow spotlights | operator 2026-08-05 + v3 mock `:401` + `matrix.json:91-95` |
| WiredFRONT editor *(option)* | `brutalist` + `neo` flavor on `lime` | — | operator 2026-08-05 — an option, **not** the default |
| **Workbench** | *clay* | **machine-console flavor** (`claymorphism-cyber.png`) | operator 2026-08-04 |
| Rail / hub chips | neubrutalist, per-hub tone | — | v3 mock `:349-368` |
| Passive content cards | claymorphic | — | doctrine `:39` |
| Transient chrome (HUDs, palettes) | *liquid glass* | — | v3 mock `:378` |
| Present / show machine | *OPEN* — its flavor was reassigned to Workbench | — | C3 |

**Present is now unassigned.** The machine-console look was its only
proposed flavor and that has moved to Workbench. **OPEN.**

---

## 5 · The icon language

**Settled and shipping.** `ADR-015` (engine) + the vendored Lucide 1.27.0
pool. Geometry is data; ColdLight brings styling — the visx law. Icons
are treatment-agnostic, so **they survive the C2 base change unaffected.**

Registry: 57 baked ids on `feat/editor-icons` (25 chrome + 32 editor).
Pool on disk: **1,756** SVGs, 706,500 bytes.

### C5 — Pool browsing vs ADR-015 · **OPEN**

ADR-015 **rejected** runtime SVG parsing, reasoning: *"a runtime dep +
cache complexity for icons that never change between builds."* The
operator's 2026-08-04 requirement — *"effortlessly symlink them and then
cache"* — changes that premise.

Measured, so the decision is not a guess:

| | Number |
|---|---|
| Generated Rust per baked icon | **11.8 KB** (283,796 B for 24) |
| Full pool baked as geometry | **~20.7 MB of Rust** — not viable |
| Full pool embedded as text | **706 KB** — trivial |

Design recorded in `docs/plans/wiredfront-editor-residency.md` §3.1
(three tiers: baked chrome / pool-as-text tessellated lazily / operator
links in userdata). **Requires an ADR-015 amendment. Not yet ratified.**

### C6 — Two icon ADRs · **needs a merge**

`ADR-016` was filed 2026-07-30 already "PARTIALLY SUPERSEDED AT FILING" —
a parallel session's duplicate. Its engine decision is dead; its §4
(styled paint vocabulary) and §5 (operator-SVG-same-pipe) are marked
still-live. **Truth about the icon language is split across two files.**
Recommend folding the live parts into ADR-015 and marking 016 fully
superseded.

---

## 6 · Laws that are NOT in question

From `doctrine.md:76-91`, unchanged and binding:

1. **Tooltip law** — every interactive control ships `cl::tip(resp, …)`.
2. **Generated tokens only** — no raw hex, no magic paddings.
3. **Any-shape rendering** — proofs at real window sizes.
4. **Motion respects PilotClock.**
5. **Proof includes feel** — a shot the operator could mistake for the
   old app's polish. *This is the gate the icon pass currently fails.*
6. **No blur, ever** — hard shadows, painted glow, gradient tint.
7. **Proactive buffer** (`doctrine.md:108`) — no scans on the paint
   thread.

---

## 7 · What must be decided before code moves

Ordered by what blocks the most. **C1 and C4 are RULED; C2 dissolved.**

1. **C2b — build `resolve(&Coordinate, state) -> ResolvedStyle`**
   (`coldlight-matrix.md` §7). The matrix's vocabulary and legality are
   generated and build-enforced; the resolver is not written. **This is
   the only missing machinery, and it blocks all editor styling.**
   Lands with the §8 byte-identical migration test so it is proven a
   reclassification, not a restyle.
2. **Confirm the editor coordinate** — proposed
   `aero · measured · neon-glass · neon`, legality-checked against
   `matrix.json:91-95`. One word from the operator; no authoring.
3. **C7 — does the slots branch land**, accepting the ratified-color
   shift to the v3 numbers?
4. **Re-treat the editor** — commit `2b0dacc` skinned it neubrutalist.
   Icons stay; the base becomes the coordinate. Follows 1 + 2.
5. **C5 — amend ADR-015** for pool browsing (measured case in §5).
6. **C6 — fold ADR-016's live parts into ADR-015**, mark 016 superseded.
7. **Present's treatment** is unassigned (C3 consequence).
8. **Express the editor-mock palette as its own coordinate** (the "both
   in the matrix" half of the C1 ruling).

### The sequencing this implies

`resolve()` + migration test → confirm coordinate → re-treat the editor
→ then the deferred passes (icon pool residency, browse reach, the layer
switches in `editor-layer-switches.md`). Styling before machinery is what
produced `2b0dacc`'s wrong base; this order prevents the repeat.

## 8 · Files this document supersedes or amends

- **Amends** `visual-language-v3-mock.html:401` — `claymorphism-cyber.png`
  → Workbench, not Present (C3, operator-ruled).
- **Flags as stale** `wiredfront-editor-mock.html:9` palette (C1) — its
  *layout* remains authoritative.
- **Flags as incomplete** `doctrine.md` — omits liquid glass, which the
  editor depends on.
- **Names as drift** the neubrutalist skinning of editor chrome in
  `feat/editor-icons` (`2b0dacc`) — icons correct, base wrong.
- **Does not touch** `coldlight-matrix.md` — pending C4.
