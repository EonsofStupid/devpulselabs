# The proof deck — the matrix demonstrating itself

**Status: SPEC, awaiting operator go. 2026-07-31.** Parent:
`coldlight-matrix.md`. This is the artifact that proves the matrix is right, or
proves it is wrong. Not a marketing deck — a **test the design has to pass**.

## 0. Why this exists

> **If the matrix cannot produce a genuinely beautiful slide, the matrix is
> wrong.** No amount of clean classification redeems a system whose output is
> mediocre.

So the deck leads with the output, not the theory. The operator's pattern:
*a beautiful slide → a breakdown presented as a series-of-slides-in-one-slide →
resume → and it must demonstrate the categories exist and extend.*

Two claims are on trial, and they are separable:

1. **The categories EXIST** — each facet is genuinely independent, and changing
   one changes exactly one thing about what you see. If a breakdown frame moves
   something the label did not name, the partition (§2 of the matrix) is wrong.
2. **The categories EXTEND** — one authored value multiplies the space rather
   than adding to it. If the count moves by +1 instead of ×N, the model is
   ordinary theming wearing a costume.

## 1. The deck, slide by slide

| # | slide | what it proves | chrome |
|---|---|---|---|
| **1** | **THE SLIDE.** One coordinate, rendered beautifully. Real content, real type, real gradient. | the output is worth having | **none** — no labels, no facet names, no explanation. It stands or it doesn't. |
| **2** | **THE BREAKDOWN.** Slide 1 taken apart as five frames in one frame (§2). | the categories EXIST | one label per frame, naming only the facet that changed |
| **3** | **RESUME.** A second beautiful slide, different ancestor, different content. | slide 1 was not a fluke | none |
| **4** | **THE ANCESTOR ROW.** The *same* structure · material · atmosphere · tone, across all eight ancestors — eight slides in one. | the ancestor facet is real: alone it changes everything | ancestor id only |
| **5** | **RESUME.** A third beautiful slide. | the pattern holds | none |
| **6** | **EXTEND.** Add ONE new atmosphere value; show the count move and sample the new coordinates. | the categories EXTEND | the arithmetic, shown |
| **7** | **THE FLOOR.** A forbidden coordinate, struck out, with its reason. | the space is guarded, not mush | the ruling that forbids it |

The rhythm is deliberate: **1 · 3 · 5 carry no explanation at all.** If the
deck only works when it is narrating itself, it has not proven anything.

## 2. Slide 2 in detail — "a series of slides as one slide"

The operator's phrase, taken literally. One slide frame, five sub-frames in a
row, each adding exactly one facet:

```
┌──────────┬──────────┬──────────┬──────────┬──────────┐
│    1     │    2     │    3     │    4     │    5     │
│ ancestor │ +STRUCT  │ +MATERIAL│ +ATMOS   │ +TONE    │
│  alone   │          │          │          │  = SLIDE 1│
│          │          │          │          │          │
│ complete │ geometry │ surface  │ light    │ colour   │
│ but plain│ moves    │ moves    │ arrives  │ lands    │
└──────────┴──────────┴──────────┴──────────┴──────────┘
   bauhaus   stepped    lacquer     sheen      amber
```

Frame 1 matters most and is the easiest to get wrong: **an ancestor alone must
already be a complete, correct slide** — because it resolves to its natives
(matrix §5). Plain, not broken. If frame 1 looks unfinished, `native` is
under-specified and the model has a hole.

**The honesty rule for this slide:** each step may move ONLY the axes its facet
owns. If `+MATERIAL` visibly changes the geometry, either the render is cheating
or the partition is wrong — and both are findings worth having. This slide is
the actual unit test of §2, drawn.

## 3. Slide 6 in detail — extend

Add one atmosphere value, say `grain`. The arithmetic, shown on the slide:

```
   8 ancestors × 6 structures × 7 materials × 6 atmospheres  =  2,016
   8           × 6            × 7           × 7              =  2,352
   ──────────────────────────────────────────────────────────────────
   one authored value                                        +  336 coordinates
```

Then six of the newly-legal coordinates, rendered. Not described — rendered.
That is the whole argument for the matrix in one frame: **you authored one
thing and got 336.**

The counter-honesty, on the same slide in smaller type: those 336 are
*guaranteed non-contradictory*, not *guaranteed good*. Legality is not taste
(matrix §5). Presets remain curation. Saying this on the slide is what keeps
the deck a proof rather than a sales pitch.

## 4. Content — real, never lorem

Every slide carries real DevPULSE content — a real headline, a real stat, a
real quote with attribution. A demo full of "Lorem ipsum" proves the layout and
nothing about the product. Candidate content: the matrix's own numbers (2,016
coordinates, 24 authored values, 120 faces) — the deck describing itself is
honest content and needs no invention.

## 5. Build order

1. **Mock first, in the artifact** — fast to iterate, and the operator can red-pen
   before any Rust is written. Real OKLCH colour; the 13 repo faces are real,
   the rest named.
2. **Then, and only if the mock is beautiful,** port it to the egui slide
   system as the matrix's acceptance test. A deck that survives the port is the
   proof the implementation matches the design.

Building the Rust first would mean discovering the model is wrong after paying
for it.

## 6. Open — needed before slide 1 can be drawn

1. **Which coordinate opens the deck?** Slide 1 has to be the best thing the
   system can do. Proposal: `art-deco · stepped · lacquer · sheen · amber` —
   Deco is the most immediately striking of the eight and the one least like
   the current app, so it proves range rather than flattering what already
   exists. Alternative: `swiss · measured · paper · none · cyan` — proves the
   restrained end is beautiful too, which is the harder and more useful test.
2. **Deck size** — seven slides as specified, or extend the resume/breakdown
   rhythm further (9, 11)?
3. **Slide 4's fixed coordinate** — which structure/material/atmosphere to hold
   constant while the ancestor varies. It must be one that is *legal for all
   eight*, which is itself a good check on the rulings.
