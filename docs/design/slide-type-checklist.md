# The slide-type checklist — the census we are working toward

**Status: RESEARCHED DRAFT, 2026-07-31.** Sources in §6. Parent:
`coldlight-matrix.md`. This is the **type** axis; the matrix is the **look**
axis. They are orthogonal and must not be confused.

## 0. The two axes

> **A slide TYPE is what it says. A slide COORDINATE is how it looks.**

| axis | field | vocabulary | owned by |
|---|---|---|---|
| **type** | `Slide.template` | the template registry (this doc) | `dpl-slides` |
| **look** | the coordinate | ancestor · structure · material · atmosphere · tone | `dpl-coldlight` |

`~50 types × 2,016 coordinates` is the real product surface, and neither axis
needs to know about the other. That separation is why this checklist can be
completed methodically without touching the matrix work.

## 1. What is ALREADY registered (ground truth, `registry.rs:68`)

**28 templates.** Not a blank sheet — considerably further along than the
gap list below might suggest.

`title_card` · `media_frame` · `quote` · `section_divider` · `agenda` ·
`big_number` · `callout_card` · `split_panel` · `compare_vs` · `feature_grid` ·
`timeline` · `checklist` · `reveal_list` · `choice_poll` · `quiz_panel` ·
`stats_grid` · `code_block` · `cost_meter` · `predict_rank` · `drag_match`
— then the infographic tranche — `donut_share` · `gauge_dial` ·
`radial_progress` · `bar_compare` · `line_trend` · `probability_bars` ·
`stack_layers` · `cost_meter_face`

## 2. The checklist

✅ registered · ⬜ gap · ◐ partially covered by a neighbour

### A · FRAME — slides that structure the deck itself

| | type | carries |
|---|---|---|
| ✅ | `title_card` | opener: title, subtitle, presenter, date |
| ✅ | `agenda` | the contents, optionally with timings |
| ✅ | `section_divider` | resets attention; a title and nothing else |
| ⬜ | `recap` | key takeaways — the summary slide |
| ⬜ | `call_to_action` | one directive + the benefit |
| ⬜ | `end_card` | outro / thanks / subscribe / contact |
| ⬜ | `appendix` | denser reference data, backup for Q&A |

### B · STATEMENT — one idea, no evidence

| | type | carries |
|---|---|---|
| ✅ | `big_number` | one figure and its caption |
| ✅ | `quote` | quotation + attribution + source |
| ✅ | `callout_card` | one framed statement |
| ⬜ | `statement` | a full-bleed sentence — the thesis slide |
| ⬜ | `definition` | a term and its meaning |

### C · EVIDENCE — data

| | type | carries |
|---|---|---|
| ✅ | `line_trend` `bar_compare` `donut_share` `probability_bars` | the chart tranche |
| ✅ | `gauge_dial` `radial_progress` `cost_meter` `cost_meter_face` | the meter tranche |
| ✅ | `stats_grid` | the metric grid / bento dashboard |
| ⬜ | `data_table` | **rows and columns.** Nothing covers this at all |
| ⬜ | `waterfall` | bridge from one figure to another |
| ⬜ | `before_after` | two states, one change |

### D · STRUCTURE — relationships

| | type | carries |
|---|---|---|
| ✅ | `compare_vs` `split_panel` | two-way comparison |
| ✅ | `timeline` | events over time |
| ✅ | `feature_grid` | discrete related items |
| ✅ | `stack_layers` | layered composition |
| ⬜ | `matrix_2x2` | **two axes, four quadrants** — the most iconic strategy slide |
| ⬜ | `process_steps` | numbered sequence with direction |
| ⬜ | `hierarchy` | org / tree / decomposition |
| ⬜ | `funnel` | narrowing stages |
| ⬜ | `cycle` | a loop with no start |
| ⬜ | `venn` | overlap between sets |

### E · MEDIA

| | type | carries |
|---|---|---|
| ✅ | `media_frame` | image or video on the stage |
| ✅ | `code_block` | source with syntax colour |
| ◐ | `split_panel` | image + text side by side |
| ⬜ | `picture_grid` | contact sheet / gallery |
| ⬜ | `terminal` | console session — **dev-tools native, and this is a dev tool** |
| ⬜ | `screenshot` | a UI still with callout pins |

### F · TEACHING — for the Lesson and Guide containers

| | type | carries |
|---|---|---|
| ✅ | `quiz_panel` | the knowledge check |
| ✅ | `checklist` | things to tick |
| ⬜ | `objective` | what you will be able to do — opens a lesson |
| ⬜ | `step` | one instruction in a guide, with its ordinal |
| ⬜ | `worked_example` | the problem solved in view |
| ⬜ | `gotcha` | the common mistake, named |
| ⬜ | `resources` | where to go next |

### G · ENGAGEMENT — for the Episode and Deck containers

| | type | carries |
|---|---|---|
| ✅ | `choice_poll` `predict_rank` `drag_match` `reveal_list` | the interactive tranche |
| ⬜ | `hook` | the cold open — the first three seconds |
| ⬜ | `lower_third` | name/title bar over video |
| ⬜ | `chapter_card` | episode chapter marker |

## 3. The finding

**28 registered · ~50 in the census · ~22 gaps.** But the *shape* of the gap
matters more than the count:

| category | registered | gap | read |
|---|---|---|---|
| evidence | 9 | 3 | **over-served** — 8 of 28 templates are infographics |
| engagement | 5 | 3 | well served |
| structure | 4 | 6 | thin on classic relationship diagrams |
| frame | 3 | 4 | **no way to end a deck** — no recap, no CTA, no end card |
| statement | 3 | 2 | fine |
| media | 2 | 3 | no terminal, in a developer tool |
| **teaching** | **2** | **5** | **the biggest hole** |

> **Lesson and Guide are two of the four container kinds, and teaching is the
> thinnest tranche in the registry.** The app can chart, meter and quiz — it
> cannot yet state an objective, give a numbered step, or work an example.
> That is where the methodical pass should start, not with more infographics.

Second finding: **there is no `data_table`.** A table is the most ordinary slide
in business communication and nothing in 28 templates covers rows and columns.

## 4. Suggested build order

1. **Teaching tranche** (`objective` · `step` · `worked_example` · `gotcha` ·
   `resources`) — unblocks the Lesson and Guide containers, which are half the
   product's stated surface.
2. **Frame closers** (`recap` · `call_to_action` · `end_card`) — today a deck
   cannot end.
3. **`data_table`** — the conspicuous ordinary absence.
4. **Structure diagrams** (`matrix_2x2` · `process_steps` · `hierarchy` ·
   `funnel` · `cycle` · `venn`) — the largest tranche, and the one that most
   benefits from being built as one family with shared geometry.
5. **Media** (`terminal` · `screenshot` · `picture_grid`).
6. **`waterfall` · `before_after` · `statement` · `definition` · `appendix` ·
   `hook` · `lower_third` · `chapter_card`.**

## 5. How this meets the matrix

Every template renders through the coordinate — a template never names a
colour, a font, or a shadow. So:

- adding a **template** adds one row to the product surface;
- adding a **facet value** multiplies every row.

The proof deck (`matrix-proof-deck.md`) should therefore draw its slides from
**different template types**, not just different coordinates — otherwise it
only proves half the system.

## 6. Sources

- SlideModel, *12 Types of PowerPoint Slides* — https://slidemodel.com/types-of-slides/
- Deckary, *PowerPoint Layout Ideas: 9 Proven Structures* — https://deckary.com/blog/powerpoint-layout-ideas
- Deckary, *Consulting Presentations: MBB Slide Design & Structure Guide* — https://deckary.com/blog/pillar-consulting-presentations-guide
- SlideUpLift, *Types of Slides* — https://slideuplift.com/blog/types-of-slides/
- Visme, *What Is an Agenda Slide?* — https://visme.co/blog/agenda-slide/
- SlideModel, *Learning Objectives Examples* — https://slidemodel.com/learning-objectives-examples/

Consulting sources converge on "roughly twenty slide types account for ~80% of
layouts"; the ~50 here is deliberately the fuller census, because this product
also serves lessons, guides and episodes — not only business decks.
