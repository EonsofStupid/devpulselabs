# The ColdLight MATRIX — a faceted classification for the look system

**Status: DEFINITION DRAFT, 2026-07-31.** The operator's model, written down so
it can be schema'd. Parent: `doctrine.md` (the treatment mechanics) +
`architecture.md` §4 (the census law). Consumed by
`docs/plans/workbench-editor.md` §3.1 (presets).

---

## 0. The principle, named correctly

**S.R. Ranganathan**, *Colon Classification* (1933) — **faceted
classification**. A thing is not filed at one place in one tree; it is a
**coordinate** across independent facets that combine. His five fundamental
categories are **PMEST**: Personality · Matter · Energy · Space · Time.

That is exactly the operator's ask: *"we take the complexity to the
background."* A hierarchy of hand-authored looks grows linearly with effort. A
matrix grows **multiplicatively** from a small authored vocabulary — and the
person picking never sees the axes, only a coordinate that already looks right.

> **~22 authored facet values → ~900 valid coordinates.**
> Nobody authors 900 looks. Nobody browses 900 looks. The matrix holds them and
> the presets name the good ones.

## 1. What already exists (ground truth, do not rebuild)

`crates/coldlight/src/lib.rs:455` —
`compose(tone: Tone, base: &str, stack: &[&str], state: TraitState)`. It
already resolves **per-axis**, and the axis rules are already right:

| axis kind | axes | rule |
|---|---|---|
| override | `fill` `edge` `radius` `ink` `font` | last trait that names it wins |
| additive | `shadows` `back_glow` `tints` `anim` | every trait's contribution concatenates |

`traits.json` has four traits carrying a crude two-value facet, `role`:

| trait | role | what it really is under the matrix |
|---|---|---|
| `neubrutalist` | base | the **Brutalist** ancestor |
| `claymorphic` | base | a **material** (soft extruded plane) — never an ancestor |
| `cyber` | stack | an **atmosphere** (neon bloom) |
| `retro` | stack | an **atmosphere** (CRT scanline) |

**The defect the matrix fixes:** `stack: &[&str]` is untyped. Nothing prevents
`compose(tone, "neubrutalist", &["claymorphic"], …)` — two contradictory bases,
silently last-wins. Facets replace the untyped list with **one value per
facet, exactly**, which is checkable at build time.

## 2. The five facets (PMEST, mapped)

| Ranganathan | our facet | it answers | owns these axes |
|---|---|---|---|
| **Personality** | **ANCESTOR** | what lineage is this? | typography (`font`) + the default coordinate for all below |
| **Space** | **STRUCTURE** | what geometry? | `radius`, `edge` |
| **Matter** | **MATERIAL** | what is it made of? | `fill`, `ink` |
| **Energy** | **ATMOSPHERE** | what light/motion is on it? | `shadows`, `back_glow`, `tints`, `anim` |
| **Time** | **STATE** | idle / hover / active | the state overlay (already exists) |

The partition is **total and disjoint** — every existing axis belongs to
exactly one facet. That is what makes the matrix implementable without touching
the compositor's maths: the facets are a *classification laid over machinery
that already works*.

`STATE` is runtime, never authored. A coordinate is the four authored facets.

## 3. ANCESTOR — the seeds (Personality)

Five lineages. Each sets typography and a **native default** for the other
three facets, so an ancestor alone is already a complete, correct look.

| id | name | period | structural instinct | material instinct | atmospheric instinct |
|---|---|---|---|---|---|
| `bauhaus` | **Bauhaus** | 1919–33, Weimar/Dessau | rectilinear; primitives (circle/square/triangle); asymmetric grid; zero radius | flat unmodulated colour planes | **none — anti-atmospheric.** No shadow, no depth. Ornament is the enemy. |
| `art-deco` | **Art Deco** | 1920s–30s | symmetric, stepped/ziggurat, vertical emphasis, chamfered | lacquer, chrome, gold leaf, inlay | directional sheen + hard geometric shadow |
| `brutalist` | **Brutalist** | *béton brut*, 1950s–70s (web ~2010s) | massive blocks, thick edges, zero radius | raw, exposed, unfinished | hard offset cast shadow, never blurred |
| `swiss` | **Swiss / International** | Zürich–Basel, 1950s– | the mathematical grid; strict columns, flush-left ragged-right, generous margin, restrained radius | flat, neutral, paper | minimal — clarity over effect |
| `aero` | **Aero** | Frutiger Aero, ~2004–13 | generous radius, pill forms | glass, gloss, translucency, wet | bloom, reflection, specular highlight |

**OPERATOR CLARIFICATION 2026-07-31:** the fourth seed is the **grid/technical**
lineage — so it is `swiss` (International Typographic Style: Müller-Brockmann,
Hofmann, Univers/Helvetica, the mathematical grid), **not** `frutiger`. Adrian
Frutiger is a typographer *inside* that movement; naming the seed after him
would file a movement under a person. `aero` stays its own seed — it is the
glossy 2000s look, unrelated to Swiss clarity despite sharing his name.

**Typography is the ancestor's, always.** It is the most identity-carrying
choice a movement makes, so `font` is owned by ANCESTOR and is not a free knob
in the other three facets.

### 3.1 `aero` collides with the no-blur law

`doctrine.md` says *no blur, hard shadows*. Aero is constitutionally about
blur, gloss and translucency. Either (a) `aero` is interpreted **no-blur** —
gloss via banded gradients and hard specular bands rather than Gaussian blur;
or (b) the doctrine takes a named exception for one facet value.
**Recommendation: (a)** — it keeps one law, and it is honest on a 75″ panel
where real blur costs fill rate.

### 3.2 THE GAPS — what five seeds cannot reach

The five cover the **geometric modernist sans** territory thoroughly, and they
are all descended from roughly one argument (ornament vs function, 1919–1980).
That is a coherent spine, and it is also the shape of what is missing. Ranked
by how much genuinely new ground each opens:

| gap | movement | what it is the ONLY source of | verdict |
|---|---|---|---|
| 1 | **Memphis / Postmodern** (Sottsass, 1981–87) | the **playful, anti-grid** register — squiggles, terrazzo, clashing pastel, deliberate asymmetry. It is the explicit *rebellion against* Bauhaus and Swiss, so nothing descended from them can produce it. | **strongest gap.** Everything you have is serious. |
| 2 | **Constructivist** (Rodchenko, El Lissitzky, 1917–30s) | the **diagonal, agitational** register — dynamic diagonals, photomontage, red/black, urgency. Bauhaus is orthogonal and calm; Constructivism is tilted and loud. | **strong.** The best ancestor for anything that must feel urgent. |
| 3 | **Editorial / Didone** (Bodoni, Didot; fashion-press classicism) | **serif.** All five of your seeds are sans-serif lineages — the matrix as it stands cannot produce a serif voice at all. | **structural hole**, easy to miss because it is an absence. |
| 4 | **Art Nouveau / Arts & Crafts** (1890–1910) | the **organic, curvilinear** register — botanical, hand-drawn, asymmetric curves. Every one of your five is geometric. | real, but furthest from this product. |
| 5 | **Terminal / teletype** | the monospace grid, box-drawing, no-antialias structural language. | defensible for a dev-tools product; `scanline` covers the *look*, not the *structure*. |

**What is NOT a gap** — worth saying, because the instinct to add them is
strong and wrong: `cyber`, `retro`, vaporwave, Y2K chrome, neumorphism. Those
are **atmospheres and materials**, not lineages. Filing them as ancestors would
break the matrix's multiplication — the ancestral list must stay small and
orthogonal, because every value in every other facet has to be ruled on by
every ancestor (§5). Five to eight seeds is the workable range; twenty is not.

**Recommendation:** take gaps 1–3 (`memphis`, `constructivist`, `editorial`),
giving **eight seeds** — playful, urgent, and serif all become reachable, and
the ruling burden stays manageable. Defer 4 and 5.

## 4. The dependent facets

### STRUCTURE (Space) — owns `radius`, `edge`

| id | geometry |
|---|---|
| `rectilinear` | zero radius, hard corners, right angles |
| `stepped` | chamfered / ziggurat, vertical emphasis |
| `massive` | thick heavy edge, large block, zero radius |
| `measured` | restrained radius, strict grid, generous margin |
| `rounded` | generous uniform radius |
| `pill` | fully rounded ends |

### MATERIAL (Matter) — owns `fill`, `ink`

| id | surface |
|---|---|
| `matte` | flat, unmodulated |
| `raw` | exposed, unfinished, concrete |
| `lacquer` | deep high-gloss |
| `metallic` | chrome / gold sheen |
| `paper` | neutral flat, print-like |
| `glass` | translucent |
| `clay` | soft extruded plane *(today's `claymorphic`)* |

### ATMOSPHERE (Energy) — owns `shadows`, `back_glow`, `tints`, `anim`

| id | light / motion |
|---|---|
| `none` | no shadow, no glow |
| `hard-cast` | hard offset shadow, zero blur |
| `sheen` | directional specular band |
| `bloom` | soft outer glow |
| `neon` | saturated glow bloom *(today's `cyber`)* |
| `scanline` | CRT wash *(today's `retro`)* |

**5 ancestors × 6 structures × 7 materials × 6 atmospheres = 1,260 coordinates
from 24 authored values.** Not all are valid — §5.

## 5. Not every coordinate is legal — and that is checkable

A faceted system that permits everything produces mush. `bauhaus` + `glass` +
`bloom` is not a look, it is a contradiction — Bauhaus is *defined* by the
refusal of that. So each ancestor declares, per facet, three sets:

```jsonc
{
  "id": "bauhaus", "label": "Bauhaus",
  "font": "spacegrotesk",
  "native":    { "structure": "rectilinear", "material": "matte", "atmosphere": "none" },
  "allowed":   { "structure": ["measured","massive"], "material": ["paper","raw"],
                 "atmosphere": ["hard-cast"] },
  "forbidden": { "material": ["glass","lacquer","metallic","clay"],
                 "atmosphere": ["bloom","neon","sheen"] }
}
```

- **native** — what the ancestor IS. One value per facet. An ancestor alone
  resolves to its natives, so it is always a complete look.
- **allowed** — defensible departures.
- **forbidden** — contradictions. Referencing one **fails the build**, the same
  way an unknown ref already does.

Every facet value must appear in exactly one of the three sets for every
ancestor — a value left unclassified fails the build. That is what stops the
matrix from rotting as values are added: adding a material forces every
ancestor to rule on it.

**This is the census law applied to the matrix.** Legality is data, checkable,
and not a matter of taste at call time.

## 6. What a PRESET becomes

A preset stops being a hand-authored bundle of taste. **A preset is a named
coordinate plus the facets left free:**

```jsonc
{
  "id": "neon-intro", "label": "Neon intro", "for": "card",
  "coordinate": { "ancestor": "brutalist", "structure": "massive",
                  "material": "raw", "atmosphere": "neon", "tone": "magenta" },
  "moods": ["ember-drift","deep-lagoon"], "drift": "blend-drift",
  "exposes": ["colour","background","ambience","motion"]
}
```

And the flow's central claim falls straight out of the model:

> **The 4-7 exposed traits are the axes left free after the coordinate pins the
> rest.** Picking a preset *is* pinning a point in the matrix. The complexity in
> the background is literally the 1,260 coordinates you did not have to look at.

## 7. The schema

```rust
/// A point in the matrix. STATE is runtime, never authored.
pub struct Coordinate {
    pub ancestor:   AncestorId,     // bauhaus | art-deco | brutalist | frutiger | aero
    pub structure:  StructureId,
    pub material:   MaterialId,
    pub atmosphere: AtmosphereId,
    pub tone:       Tone,
}

/// Replaces `compose(tone, base, stack: &[&str], state)`. One value per facet,
/// exactly — the untyped stack, and the two-contradictory-bases bug, are gone.
pub fn resolve(c: &Coordinate, state: TraitState) -> ResolvedStyle;
```

Resolution order — ancestor first, then the three facets against the existing
per-axis rules (§1), then the state overlay. **The compositor's maths does not
change**; only what is allowed to enter it does.

Generated from `tokens/matrix.json` by `build.rs`, beside the existing
generators. Build-time checks: total-and-disjoint axis partition · every facet
value classified by every ancestor · no coordinate referencing a forbidden
value · every `id` kebab (the naming law, `architecture.md` §4).

## 8. Migration from the four traits

No behaviour change, so it lands quietly:

| today | becomes |
|---|---|
| `compose(tone,"neubrutalist",&[],s)` | `ancestor:brutalist · structure:massive · material:raw · atmosphere:hard-cast` |
| `compose(tone,"claymorphic",&[],s)` | `ancestor:aero · structure:rounded · material:clay · atmosphere:bloom` |
| `compose(tone,base,&["cyber"],s)` | `… atmosphere:neon` |
| `compose(tone,base,&["retro"],s)` | `… atmosphere:scanline` |

A test pins each of the four legacy calls against its coordinate and asserts
the `ResolvedStyle` is **byte-identical** — so the matrix is proven to be a
reclassification, not a restyle.

## 9. Open for ratification

1. ✅ **RATIFIED 2026-07-31** — the grid/technical seed is **`swiss`**, not
   `frutiger`; `aero` remains its own seed.
2. §3.1 — `aero` rendered **no-blur** (recommended), or a doctrine exception?
3. §3.2 — take the three named gaps (`memphis` · `constructivist` ·
   `editorial`) to eight seeds, or ship five and revisit?
4. §4 — the facet vocabularies: 6 structures, 7 materials, 6 atmospheres.
   Add, cut, or rename before any of it is written.
5. §5 — native/allowed/forbidden as the legality law. Note the cost, honestly:
   **every ancestor must rule on every facet value**, so eight seeds × 19 values
   = 152 rulings to author. That is the price of a matrix that cannot rot, and
   it is why the ancestral list must stay short.
