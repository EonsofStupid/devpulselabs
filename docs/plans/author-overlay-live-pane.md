# Pass — the overlay's live pane (M5b)

The card editor becomes what the mock specifies: a tab rail down the left,
the tab's controls in the middle, and **the card itself, live, on the right**.

**Reference:** R1 — `docs/ops/references/authoring-mock-a3db4aee.html`,
`.m-body` → `.tabs` + `.m-split` → `.m-main` + `.m-live`.
**POA&M:** `docs/plans/authoring-lane-poam.md` — M5b, which that file says
should land BEFORE the four remaining tabs.

## 1. Context — why this pass exists

The operator asked for it in as many words:

> "the card would then appear show live preview as i start to fill it out"

Today the overlay is a single column: a wrapping chip row of seven tabs, then
the tab's body. Two consequences, both real:

- **You cannot see what you are changing.** The Background tab paints ten
  live colour tiles and the card they apply to is nowhere on screen.
- **Content falls below the fold.** The body is capped at 55% of the window
  (#141), so the Background tab's third row scrolls out of sight
  (`author-tab-background.md` §8.3).

It is logged as M5b and sequenced ahead of the remaining tabs because each new
tab otherwise inherits the wrong shape and has to be re-fitted later.

## 2. Current state (ground-truthed in-session, `5214575`)

**The mock's structure**, verbatim from R1:

```js
const split = el("div","m-split");
const main  = el("div","m-main");
const live  = el("div","m-live");
live.innerHTML = '<div class="lvh"><span class="led"></span>as it looks right now</div>';
const lc = el("div","livecard");      // aspect-ratio 16/9
```

with `.m-body{display:flex}`, `.tabs{width:158px}` (a **vertical** rail that
becomes horizontal under 760px), and
`.m-split{grid-template-columns:minmax(0,1fr) minmax(0,306px)}` (one column,
two rows, under 760px).

**Ours** (`author.rs::editor_modal`): title row → `horizontal_wrapped` chip
row of `CardTab::ALL` → `ScrollArea` capped at `room.height() * 0.55` →
footer. One column throughout.

**The Words tab is not a form.** `CardTab::Words` renders the real
`dpl_editor::Editor`, whose `ui()` draws a toolbar, **a stage**, and a tray —
and the stage is the *selection surface*: `ui.interact(stage, …)` then
`map.block_at(pos)` decides whether you selected a text block or the
background. The card there is the control, not a picture of one.

**Thumbnail mode exists** (#147), so a live card can be drawn at pane size
without the crop that would have made this pass pointless before.

## 3. Design — two panes, and one honest exception

**The rail goes vertical**, as in R1: `CardTab::ALL` down the left at a fixed
readable width, falling back to the current wrapping row when the window is
too narrow to afford it. This alone buys back the height that pushed content
below the fold.

**The body splits**: controls left, the card right. The live pane carries
what R1 carries — the words *"as it looks right now"*, the card at 16:9, and
a line of meta under it.

**The exception, stated rather than hidden: Words keeps its own stage and
gets NO live pane.** In every other tab the card is a *preview* — you change
a colour, the card shows it. In Words the card is the *control*: you tap a
line on the slide to select it. Two identical cards side by side, only one of
which responds to a tap, would be worse than one. So:

| tab | shape |
|---|---|
| Words | full width — the editor's own stage, which is the tap target |
| every other tab | controls left, live card right |

This is a deviation from R1's uniform `.m-split`, and it exists because our
Words tab is a tap-the-slide editor where the mock's Content tab was a form.
Recorded in §8 as a deviation, not smuggled in.

**The live card is the real document**, drawn through
`DocSlide::thumbnail_ui` from the editor's own in-memory slide — not a re-read
of the file — so a tapped colour shows up on the next frame rather than after
a save round-trip.

## 4. Scope

**In:** the vertical tab rail with a narrow-window fallback · the two-pane
split for non-Words tabs · the live pane (heading, 16:9 card, meta line) ·
the card fed from the editor's live slide · tests · proof shots.

**Out, and named:**
- **Making Words a form.** That would mean rebuilding the ratified editor
  salvaged in #141. Not this pass, and possibly never — tapping the slide is
  the operator's stated way of working.
- The remaining four tabs. This pass exists so they arrive into the right
  shape.
- The mock's `.bdg` "AI" badge on the Ask-for-help tab — belongs with M10.

## 5. Work plan (atomic, in order — commit at each step that compiles)

1. `Editor::slide()` is already public; add whatever the live pane needs to
   draw from it without a file read (a `look()` accessor if required).
2. The vertical rail + narrow fallback. No split yet; prove the fold is
   already better.
3. The split and the live pane, for non-Words tabs.
4. Tests: the rail lists every tab at both widths; the live pane appears for
   a form tab and not for Words.
5. Proof shots at 1280×800 and 1100×700, §8, POA&M row, `docs/STATE.md`.

## 6. Risks

- **The overlay gets crowded at 1100×700.** Three columns in ~900px of modal.
  Mitigation: the rail collapses to the wrapping row below a threshold
  derived from `panel_width()`, exactly as R1 collapses at 760px; proof at
  1100×700 is the check.
- **Two cards on screen (overlay live pane + the surface's own filmstrip).**
  They show different things — the card being edited versus the sitting's
  drafts — and the scrim separates them. Watch it in the shot.
- **Feeding the pane from the editor's memory couples the surface to editor
  internals.** Mitigation: go through public accessors only; add one if
  missing rather than reaching in.

## 7. Verification / acceptance

- `cargo green` clean.
- A kittest that every `CardTab` is reachable from the rail at a wide size and
  at a narrow one — the gallery gate for tabs.
- A test that a form tab renders the live pane and Words does not.
- Real-exe proof at **1280×800** and **1100×700** with a form tab open: the
  card visible beside its controls, and the Background tab's third row no
  longer below the fold.
- The feel bar: changing a colour must visibly change the card beside it.

## 8. Evidence

### 8.1 The gate

`cargo green ✔`. 23 tests in `dpl-hub-workbench`, 3 new.

`every_tab_is_reachable_from_the_rail_in_both_orientations` — the gallery
gate for tabs, run against BOTH rail orientations: a tab that exists but
cannot be got to is the same defect as one that never paints.
`only_the_words_tab_owns_its_card` pins the deviation below.
`the_live_pane_shows_a_moving_card_and_says_so` — see 8.3.

### 8.2 Real-exe proof

| shot | what it proves |
|---|---|
| `docs/proof/author-live-pane-1280x800.png` | R1's shape: rail left, controls middle, the card live on the right under "as it looks right now" |
| `docs/proof/author-live-pane-1100x700.png` | all three panes still fit at a short window |
| `docs/proof/author-live-pane-words.png` | the exception — Words keeps full width and its own tap-target stage |

The Background tab's rows are now in a pane that scrolls on its own, so the
below-the-fold problem from `author-tab-background.md` §8.3 is closed.

### 8.3 Deviations and findings

**Words gets no live pane, and that is deliberate** (§3). Everywhere else the
card is a preview; on Words it is the control — you tap a line on the slide to
select it. Two cards side by side, only one answering a tap, is worse than
one. The rule is a named predicate, `CardTab::shows_live_card`, so it is
readable and testable rather than an inline `!=`.

**`scope_builder` walked the footer into the band.** The first working
version drew each pane with `ui.scope_builder(max_rect)`; the band had already
claimed its height with `allocate_exact_size`, and the scopes claimed more on
top, so "Everything you type here saves itself." landed mid-band over the
rail. `ui.new_child` does not advance the parent cursor and is the right tool
once geometry is explicit.

**Nested layouts did not work here at all.** The first attempt used
`horizontal_top` + `allocate_ui_with_layout`; it allocated three columns and
painted nothing into them. Explicit rects — the idiom the carousel already
uses in this file — worked immediately. A band whose parts must line up is
easier to reason about as geometry than as a stack of layout negotiations.

**The preview genuinely never rests, and a test proved it.**
`the_live_pane_shows_a_moving_card_and_says_so` first failed with *"Harness::run
exceeded max_steps (4) … Repaint causes: document.rs:120"*. That is the mood
blend animating: the card is a real document render, not a still. `run_steps`
is correct here, and a converging harness would have meant the preview had
stopped being live.

**The pane renders the editor's in-memory slide**, via
`render_thumbnail(… ed.editor.slide(), ed.editor.look() …)` with a clock owned
by `CardEditor` — not a re-read of the file and not a rebuilt `DocSlide`,
either of which would restart the entrance on every keystroke. `Editor::look`
is new so the preview cascades the same way the card does.
