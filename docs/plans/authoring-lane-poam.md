# Authoring lane — POA&M

**The one page for this lane.** What we are building, what is done, what is
left, and what each remaining thing is waiting on. Every authoring pass
updates its row here in the same PR that does the work.

**Updated:** 2026-07-29 · **Status:** M1–M4 closed, M5 in flight

---

## 0. The goal

> Turn an intention into a finished card without the operator ever meeting
> the machinery.

Say what you want, sweep a shelf of real premades, tap one, and edit it in an
overlay — on a touchscreen, in plain words, with every option drawn live
rather than described. Behind it, everything the operator picks is a
registered ingredient, so the library compounds instead of accumulating
one-off slides.

**Done means:** an operator builds a run of cards for a lesson, guide,
episode or deck, start to finish, without typing a ref, seeing a slug, or
learning what a template is.

---

## 1. References — attached, and cited by every pass

| # | Reference | What it settles |
|---|---|---|
| R1 | **`docs/ops/references/authoring-mock-a3db4aee.html`** — the ratified mock, saved into the repo (artifact `a3db4aee-9c6c-433c-a50d-35848a54d7ac`). Open it in a browser. | THE ratified surface. Dashboard-first IA, shelf picker over one carousel, the container-labelled filmstrip, the seven-tab overlay with its live-preview pane, plain words throughout. **When code and mock disagree, the mock is what the operator approved.** Committed rather than linked because an artifact URL can rot and every pass has to open it cold. |
| R2 | `docs/plans/authoring-charter.md` | The thesis, the four-word vocabulary, the architecture-first AI law, the guardrails. §9: *every authoring-surface pass cites this*. |
| R3 | `docs/plans/authoring-architecture.md` | The layer stack (document → presets → editor), the UI law ("the stack is invisible"), the pass breakdown. |
| R4 | `docs/design/doctrine.md` + the `coldlight-system` skill | Treatments, paint vocabulary, the tooltip law, the feel bar. |
| R5 | `docs/journal/2026-07-29-authoring-reset.md` | Why Passes B/C were retired and what was salvaged. |
| R6 | `docs/journal/2026-07-29-authoring-lane-resume.md` | The branch/worktree picture and the ordered NEXT this POA&M supersedes. |

**Standing operator corrections** (from R6, still binding):

1. *"the UI shows blocks elements and traits — you are still not really
   translating that"* — engineering words never reach the screen. Both
   vocabularies, always split (`slug()` / `label()`).
2. *"you still have never made the carousel"* — premades are picked by
   sweeping and tapping, not by typing.
3. **Reverse the IA** — declare intent at a dashboard first, *then* reach the
   pieces.
4. **The container is configurable** — lesson · guide · episode · deck,
   loaded from disk, never compiled in.

---

## 2. Milestones

| ID | Milestone | Status | Evidence |
|---|---|---|---|
| **M1** | The spine — a draft card is a real, schema-valid document | ✅ closed | #124 |
| **M2** | The ingredient ladder + plain words | ✅ closed | #135 |
| **M3** | The carousel — sweep and tap the premades | ✅ closed | #136 |
| **M4** | The overlay editor opens on a real file | ✅ closed | #141 |
| **M5** | **The card tabs become real** (7 tabs) | 🔶 **3 of 7** | #149, #151 |
| **M6** | The container spine — filmstrip + thumbnails | ✅ closed | #146, #147 |
| **M7** | Placing cards into the container | ⬜ not started | — |
| **M8** | The dashboard — intent declared first | ⬜ not started | — |
| **M9** | The platform lane — build the ingredients themselves | ⬜ not started | — |
| **M10** | Assist — paste a recipe, get elements → templates → cards | ⬜ not started | — |

---

## 3. Work done

Each row: what landed, and where the evidence is.

| Pass | PR | Plan | What it bought |
|---|---|---|---|
| Ingredient ladder (DS3) | #135 | `ds3-ingredient-ladder.md` | `elements/` (six atoms) + an empty-by-design `blocks/` chunk tier, census-tested. Plain words: Starters · Layouts · Combos · Pieces · Looks. Seam `--lib-section`. |
| The carousel | #136 | — | Shelf picker over one lane of large sweepable cards, all census-driven. Endless-scroll as a checkbox. Fit-guarded both axes. |
| Overlay editor | #141 | — | Scrim modal, seven plain-word tabs; **Words** is the real `dpl_editor::Editor` over the card's own `slide.json`. Seam `--author-editor`. |
| Container filmstrip | #146 | `author-filmstrip.md` | `containers.rs` census (4 kinds), the container bar, the band labelled by what is loaded. `start_card` as the ONE authoring path. Seams `--author-container`, `--author-draft`. |
| Thumbnail mode | #147 | `slide-thumbnail-mode.md` | `Fluid::thumbnail` — a miniature is a scaled full stage. Carousel cards and film tiles show whole slides instead of crops. |
| Background tab | #149 | `author-tab-background.md` | Ten live-rendered colours + blend + ambience, written through `Editor::set_background`. Seam `--card-tab`. |
| Movement tab | #151 | `author-tab-movement.md` | Three real choices from the schema's own vocabulary; the shortfall stated on the tab. |

**Infrastructure repaired along the way:** #144 restored the Linux CI lane
(red since #142); the Windows lane was fixed by #142 after #139 broke it.
Both incidents produced the **merge-only-on-double-green** law (#145).

---

## 4. Work remaining

Ordered by what unblocks the most. Each row says what it is waiting on, so
nothing here is a mystery when picked up cold.

### M5 — the four remaining tabs

| Item | Waiting on | Size | Note |
|---|---|---|---|
| **A quote** | nothing | S | The quote, who said it, where it came from. Needs a `Block` kind or a convention over text blocks — decide first. |
| **Picture** | nothing | M | `Block::Media` already exists (schema v2, W11). Needs a file picker and the sources law (never mutate; capture NEW). |
| **On tap** | **the interaction tier does not exist** | L | `Shelf::Interactions` ships empty for the same reason. Needs the `Recipe` trait + registry (charter §6.2) before a tab can offer anything. |
| **Ask for help** | **M10** | L | This is the assist path's entry point; it should not ship before the thing it hands off to. |

### M5b — the live preview panel (R1, `.m-split` / `.m-live`)

**Named here because it is easy to miss.** The mock's overlay is *two panes*:
tabs and fields on the left, a **live card preview on the right**, so you see
the card change as you fill it in. Today only the Words tab has a preview (the
editor draws its own stage); Background and Movement have none, and the
operator's own words were *"the card would then appear show live preview as i
start to fill it out"*.

Waiting on: nothing. Should probably land **before** the remaining tabs, since
each new tab otherwise inherits the wrong shape. Also fixes the
below-the-fold problem noted in `author-tab-background.md` §8.3.

### M7 — placing cards into the container

The filmstrip says *"N started here, not put in yet"* and means it. Drafts
live in `inbox/`; `dpl_doc::place_slide` exists and Studio already uses it.
Waiting on: nothing. Size M. This closes the loop the filmstrip opened.

### M8 — the dashboard

Operator correction #3, and the only ratified screen with nothing built.
Three intents: *edit existing · expand the platform · work on content*, plus
a recents row. Waiting on: nothing. Size M. The `← Back` and `Change`
controls deferred in `author-filmstrip.md` §4 belong to it.

### M9 — the platform lane

The mock's second lane: build the ingredients themselves (piece · combo ·
look · interaction), with the build ladder made visible. Waiting on: M9
overlaps the interaction tier (see On tap). Size L.

### M10 — assist

*"go to the chat window and toggle assist to paste the recipe and my natural
language … generating first the necessary elements and then the templates and
then the actual cards."* Waiting on: the chat assist mode exists (#126); the
**entity-first generator** does not, and the charter's §4 architecture-first
AI law is the hard part — registrations must commit before compositions.
Size XL. This is the lane's endgame.

### Authoring debt (data, not code)

| Gap | Where | Consequence today |
|---|---|---|
| **10 moods** | `tokens/moods.json` | The mock promises "around forty"; the tab honestly says ten. |
| **1 entrance** | `dpl_doc::ENTERS` + `document.rs` | Movement offers three choices total. |
| **No exit effects** | nowhere | "How things go out" cannot be offered at all. |
| **No lesson / deck storage** | nowhere | Two of four container kinds show an honest-empty line. |
| **Empty `blocks/` tier** | `dpl-slides` | Combos shelf is empty by design until a pattern recurs in ≥2 templates (promotion rule). |

Each is a **data/authoring pass**, not a code pass. Code that would paper
over them was deliberately not written.

---

## 5. The standing finding

Three passes running, **the ratified mock promised more than the registries
hold** — and each time the honest move was the same: show the real number,
name the gap, refuse to pad.

| Mock says | Reality | What we did |
|---|---|---|
| 4 container kinds | 2 stored on disk | Registered all four; the unbacked two say why. Census test pins the backed set. |
| "around forty" backgrounds | 10 moods | Heading reads off `colours().len()`. Forty is reachable by *combination*, not by a shelf of forty. |
| "come in, get noticed, go out" | 1 entrance, 0 exits | Three choices; the shortfall stated on the tab. |

**This is a feature of the process, not a failure of the mock.** The mock is
the operator's intent; the registries are what exists. The gap between them
is the backlog, and writing it down is how it stops being invisible.

---

## 6. The working habit (from here on)

1. **Cite the reference in §1 of the plan doc.** Which of R1–R6, and which
   part. A pass that cannot name its reference is not ready to start.
2. **Ground-truth before designing.** Read the real registries and schema
   in-session. Every surprise above came from this step; none came from
   reasoning about the mock.
3. **Update this file in the same PR.** Move the milestone row, add the Work
   Done row, delete or amend the Remaining row. The PR is not done until the
   POA&M matches reality.
4. **Journal only at a lane boundary** — a milestone closing, a decision
   reversed, or context about to be lost. Per-pass detail belongs in the
   plan's §8, not a new journal file.
5. **When the mock and the registry disagree, say so in §8** and add a row to
   §4's authoring-debt table. Never pad a shelf to match a promise.
6. **`docs/STATE.md` gets the one-paragraph outcome**; this file gets the
   status; the plan gets the evidence. Three files, three jobs, no overlap.
