# Pass — the "A quote" tab

Someone else's words, kept with where they came from.

**Reference:** R1 — `docs/ops/references/authoring-mock-a3db4aee.html`. The tab
copy: *"The quote, who said it, and where it came from — kept together so it
can be…"*, and `LAYOUTS` carries `{k:"quote", name:"A quote", d:"someone's
words, with credit"}`.
**POA&M:** `docs/plans/authoring-lane-poam.md` — M5, tab 4 of 7.

## 0. The ratified decision (operator, 2026-07-29)

> **"convention over text blocks"**

A quote is **not** a new `Block` kind. It is three text blocks with reserved
ids. This pass implements that and nothing wider.

## 1. Context — why this pass exists

The tab is one of four still saying "Not built yet." It is the cheapest of
them to make real *and* the one that most needs the decision above written
down, because "a quote" reads like it wants its own block type and does not.

## 2. Current state (ground-truthed in-session, `1eaccdf`)

**Block ids are ALREADY a semantic convention.** Every seed document proves
it:

| seed | ids |
|---|---|
| `headline-clean` | `eyebrow` · `title` · `subtitle` |
| `stat-callout` | `label` · `number` · `caption` |
| `terminal-retro` | `eyebrow` · `title` · `subtitle` |

`Block::Text { id, text, font, size, ink }` — `id` is a free `String`, and the
role is carried by the id while the *look* is carried by `size` + `ink`.

**The document renderer never reads `slide.template`.** Checked: `document.rs`
contains no reference to it. It flows the blocks vertically, sizing each from
its own `size` ref. `slide.template` is declarative metadata — which templates
`demos()` registers (`TemplateId("quote")` among 28) is a separate axis.

**This is why the operator's decision is the only one that works cheaply.** A
`Block::Quote` variant would need renderer support, a schema bump, and a
degrade path. Three text blocks with reserved ids need *none* — they render
today, through the one path, and the live pane already shows them.

**`validate` does not constrain ids** beyond reveals referencing existing
ones. So the convention is ours to define and ours to keep honest.

## 3. Design — three reserved ids, one shape

**The convention** (to be declared in `dpl_doc` beside `ENTERS`, so the
surface cannot drift from it):

| id | role | size | ink |
|---|---|---|---|
| `quote` | the words themselves | `h2` | `strong` |
| `said-by` | who said it | `body-lg` | `tone` |
| `source` | where it came from | `body-sm` | `subtle` |

Sizes and inks are the *defaults the tab writes*; the operator can restyle
them afterwards through the Words tab exactly as with any other block, because
they are ordinary text blocks. Nothing about them is special-cased at render.

**The tab is three fields**, in that order, each a plain-language label:
*What was said · Who said it · Where it's from*. Typing writes through the
editor (a new `Editor::set_quote`, same `mutate` path as `set_background`), so
edits join the undo stack and save themselves.

**Empty fields do not write empty blocks.** A quote with no attribution is a
quote; a document carrying an empty `said-by` text block is a rendering
artefact. Blank fields remove their block.

**`slide.template` is set to `"quote"`** when the first field is filled — it
is the document declaring what it is, which the doctor and any future
template-aware renderer can use. It changes nothing about the paint today, and
the plan says so rather than implying it does.

**The tab reports what it found.** Opening a card that already has these ids
loads them; opening one with a `title`/`subtitle` shape says plainly that this
card is not a quote yet and that filling the fields will add the blocks
alongside what is there. No silent restructuring.

## 4. Scope

**In:** the id convention exposed from `dpl_doc` · the three fields · write
through `Editor::set_quote` · blank-removes-block · `template` set on first
fill · the "not a quote yet" line · tests · proof shots.

**Out, and named:**
- **A `Block::Quote` kind.** Ruled out by the operator's decision, and §2
  explains why it would cost renderer work for no gain.
- **Pull quotes / block quotes as a *look*.** A quote's styling is trait work
  and belongs to the visual-experience lane
  (`docs/plans/poam-visual-experience.md`).
- **Citation metadata beyond a free-text source** (URL, date, page). The mock
  asks for three lines; more is speculation.

## 5. Work plan (atomic, in order — commit at each step that compiles)

1. `dpl_doc::quote_ids()` (and the size/ink defaults) beside `enters()`, with
   a test that the ids are distinct and the defaults resolve through the live
   registries.
2. `Editor::set_quote(words, who, source)` — one `mutate` that adds, updates
   or removes the three blocks and sets `template` on first fill.
3. The tab: three labelled fields over it, reading current values off the
   document.
4. The "this card is not a quote yet" line.
5. Proof shots at 1280×800 and 1100×700 via `--card-tab quote`, §8, the POA&M
   row, `docs/STATE.md`.

## 6. Risks

- **Id collision with an existing block.** A card built from the `stat` seed
  has no `quote` id, but a hand-authored one might. Mitigation: writing
  updates the block with that id if present rather than appending a second —
  and a test covers exactly that.
- **The convention drifting between `dpl_doc` and the tab.** Mitigation: the
  ids and defaults live in `dpl_doc` and the tab reads them, the same fix
  `enters()` made for Movement.
- **Order.** Blocks flow in array order, so an appended `quote` lands under
  whatever is already there. Mitigation: the three are inserted as a group in
  convention order; if the card already has text blocks, the quote group goes
  after them and the shot shows what that looks like.

## 7. Verification / acceptance

- `cargo green` clean.
- A test that filling all three fields produces a document that **validates**
  and carries exactly the three reserved ids.
- A test that clearing a field removes its block and leaves the others.
- A test that an existing block with a reserved id is UPDATED, not duplicated.
- Kittest driving the real fields, as the Background and Movement tabs do.
- Real-exe proof at both window sizes with the live pane showing the quote as
  it is typed — the pane from M5b is what makes this tab legible.

## 8. Evidence

### 8.1 The gate

`cargo green ✔`. 6 new tests — 1 in `dpl-doc`, 5 in `dpl-hub-workbench`.

`the_quote_convention_writes_a_valid_document` (in `dpl-doc`, beside the
validator it has to satisfy) builds a slide from the convention's own
`(id, size, ink)` defaults and asserts `validate` is silent — a default that
failed validation would hand the operator a document the app refuses to load.

In the surface: `a_full_quote_writes_the_reserved_ids_and_validates` ·
`a_blank_line_removes_its_block` ·
`writing_twice_updates_rather_than_duplicates` (the §6 risk, pinned) ·
`a_cards_existing_lines_are_left_alone` ·
`the_tab_reads_the_cards_own_quote`.

### 8.2 Real-exe proof

`docs/proof/author-tab-quote-1280x800.png` and `-1100x700.png`, via
`--card-tab quote`, sequential runs against a scratch content root.

Three plain-language fields — *What was said · Who said it · Where it's from* —
and the M5b live pane beside them rendering the actual quote card. The shot
was taken against a card carrying a real quote, so the pane shows the words on
a `violet-static` backdrop rather than an empty stage.

### 8.3 Findings

**The decision held all the way down.** Nothing about a quote is
special-cased at render, because nothing needed to be: `set_quote` writes
ordinary `Block::Text` values and `document.rs` flows them like any other.
The whole tab is a view over three blocks.

**Blank removes, and that took a test to keep honest.** The obvious
implementation writes an empty string. `a_blank_line_removes_its_block`
asserts the block is gone, because a document carrying an empty text block is
a rendering artefact the operator never asked for.

**Typing does not write per keystroke.** `set_quote` snapshots for undo, so a
per-character write would put fifty entries on the undo stack for one
sentence. `CardEditor` holds the field buffer and writes on change, which is
one undo entry per edit. This was not in §5 and is scope beyond the plan,
recorded here.

**The convention lives in `dpl_doc`, not the surface.** `quote_lines()` sits
beside `enters()` for the same reason: a hand-listed copy in the tab would
drift the day the shape changed and start writing documents the validator
rejects.

**The "already has lines of its own" note is dynamic.** It counts the card's
non-quote block ids, so adding a quote to a title card says so plainly rather
than silently restructuring it — and says nothing at all on a blank card.
