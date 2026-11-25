# LS4 — components render their slots and variant axes

**Status:** APPROVED 2026-10-24 — Q1 "A: Render tree now, phased"; Q3 "Add
props"; Q4 "Approve, show me shots"; Q2 answered with new requirements
(verbatim below, R287), and §4b approved in round 2: Q5 "Yes, all three"
(styling levels: layout only by default, "save as my template" as LS4d, a
one-off variant on a slide or a lesson's use with LS4a); Q6 "Plan it now,
build after LS4a" (portable linked storage: its plan written while LS4a is
built, landing after LS4a and before LS4b/LS4c). **Order:** LS4a → run a
lesson → portable storage → LS4b → LS4c (with touch sub-events) → LS4d.

**2026-11-25 (R311):** LS4a is on main. LS4b and LS4c are executed as
milestones M2 to M5 of [move and organize](organize-code-and-data.md),
which also holds their mechanism (a body painter per component) and the
seven steps each component follows. Component folders moved in M1: see
`docs/design/design-system.md` §4.7.

> configurable the user can save it as a template, or do layout, or a one
> off variant for that specific lesson/slide remember we want to be able to
> have slides even have sub events/slides for push to trigger on my
> touchscreen and have this have two modes so when i am ready to run the
> lesson we have the script for the teleprompter, and we have it so the
> storage is linked and the portable app just disconnects if on another
> machine but can be reconnected i.e users plugs external and gets it
> connected again.  We want to also allow them to grow the portable folder
> cleanly and decide if its linked or improted and stored with the whole
> program but have our structure clean, .userdata .aidata and the common
> ones properly organized nested clean for each account.  devpulse.app and
> this will eventually be tied into my dev version of devpulse

(The operator's answer to Q2 in the Claude Code session of 2026-10-24; to be
recorded as R287 in `docs/requirements.md` with the next docs commit.)
Grounded on the stack top (`feat/ls3b2-move-episodes`, 8cd3d0e) by a
read-only pass; its key claims re-checked by hand (the design system's
status and build order, the missing step-3′/4/5 code).

## 1. Context

- **R285** (2026-10-20): the schema lets lessons mix and match slides and
  refresh visuals; the research's §5 rule 4: "A new component is a contract
  file plus its paint function." LS1 gave every template a contract; LS2–LS3
  made slides, looks, lessons and episodes real. **LS4 is the pass where a
  slide is drawn by its component.**
- **The design system** (`docs/design/design-system.md`, RATIFIED 2026-07-27,
  revised 2026-08-09; R285 banner 2026-10-21) already decides the shape:
  a component's slots render into a `Node` tree (§4.1), laid out by a
  `LayoutSpec` (`Stack`/`Split`/`Grid`/`Free`, §4.2), resolved by a pure
  cascade with a parent level (§4.3), painted by ONE renderer; "no template
  hand-paints", and the catalog moves over in one big-bang pass (§4.6, §6.3).
  Its build order puts the naming merge (3′), the matrix binding (3″), the
  effects-to-tracks move (3‴), recipes (4) and the fidelity probe (5) BEFORE
  the schema/renderer (6a/6b) — **none of 3′–5 has landed**.

## 2. Goal / non-goals

**Goal:** a slide renders as its component says — its slots in the
component's layout, its variant axes applied — everywhere a slide is drawn
(deck, editor, Author, Library, episode wall, lessons), with every value the
slide and its looks set still honoured, and the editor still able to tap any
line.

**Non-goals:** render locks and Lottie slots (LS5); the teaching components
and lints (LS6); point-of-use variants on a lesson's use.

## 3. Ground truth (read 2026-10-24)

- **The component and variant are never read when painting.** All four
  render entry points funnel into `document::render_inner`: one left-aligned,
  vertically-centred column of text/quote lines in block order; pictures
  paint as a full-stage backdrop; **data blocks are never painted**. Title
  and stat look roughly right only because their blocks happen to be stored
  top-down; **agenda, compare and every list/data component lose their main
  content** (`docs/status.md` known-broken #12).
- **The 28 templates hand-paint the gallery demos** (`SlideTemplate::ui`):
  their own struct fields, hard-coded fonts/sizes/inks, no wrapping, no
  shrink-to-fit, no thumbnail sizing, no background, no reveals — they ignore
  every look, line look, per-line entrance and size the slide sets (LK1–LK3b
  would regress if their paint were used as it is). Three already read a
  variant axis as a field (`TitleCard.align`, `BigNumber.density`,
  `SplitPanel.accent`), but only demos ever get values.
- **A slide has no place for props** (`quiz.select`, `choice.answer`;
  `tone` is the slide's own). `Block::Media` and the templates' `Media` don't
  match (fit, poster, frame, entrance are lost one way, anim the other).
- **The editor depends on the flat flow:** hit-testing by block rect,
  `editor.line/<id>` tap targets, the selection outline, trays by block kind.
  A data block today gets a full-stage rect, so on an agenda card **any tap
  selects the list and the background cannot be selected** — a live bug.
  `set_quote` switches the component to `quote@1` but leaves the old blocks.
- **The `contracts` gate** stops a contract shrinking but not a NEW required
  slot; one contract per id (a `title@1` slide and a `title@2` contract
  cannot coexist).

## 4. Design — three ways to do it (Q1)

**A. The ratified shape, phased (recommended).** Build the render tree and
the one renderer now, and move the catalog over in three slices:

| Pass | What lands |
|---|---|
| **LS4a: the engine** | `dpl-doc` gains the render tree (egui-free: `Node`, `LayoutSpec`, `Slot`) and `expand(slide, contract) → Node` (slots into the component's layout, variant axes applied, props from a new `props` map on the slide); `dpl-slides` paints the tree (resolve → measure → paint), keeping everything `render_inner` does today — looks, line looks, sizes, wrapping, shrink to fit, entrances, reveals, thumbnail sizing — and returning a rect per slot (and per list item) so the editor keeps every tap target. **`Stack` is today's flow, proven pixel-identical** on every seed and test slide before anything else changes. The data-block hit-test bug and the `set_quote` leftover are fixed here. The text components move first: title, section, quote, stat, callout, media. |
| **LS4b: lists and data** | agenda, features, timeline, split, compare, stats, code, the eight infographics — their content painted at last; list/data items selectable in the editor (and the first item editor). |
| **LS4c: interactive** | checklist, reveal, choice, quiz, cost-meter, predict, drag-match as node trees + recipes (the design system's step 4, lifted out of the templates): live on the deck stage, at rest in thumbnails and the editor; the deck's edge tap zones made to give way. Each migration deletes the template's bespoke `ui()`. **R288 (2026-10-24):** the gestures are the point — drag to point and release to connect, slide away to reveal, push-to-trigger sub-events on the touchscreen — and they must work in the exported player (EX, after LS4c). |

The naming merge (3′) and the fidelity probe (5) stay their own passes,
after LS4 — the new code uses today's names (`Tone`), which the merge then
renames once with everything else.

**B. The ratified order first.** 3′ (naming merge + module split), 3″, 3‴,
4 (recipes) and 5 (fidelity) land before LS4, then LS4 = 6a/6b as written.
Cleanest call sites; the list/data cards stay blank for several more passes.

**C. Narrow and fast.** Each template gains a document paint path that takes
the resolved slots and returns slot rects — no render tree. Fixes the blank
cards soonest; contradicts §4.6 ("no template hand-paints") and keeps 28
bespoke painters to restyle later.

**Common to all (Q2–Q4):**
- **Fidelity to the slide:** components honour everything the slide and its
  looks set (fonts, sizes, inks, colours, line looks, entrances, reveals);
  the component decides only layout and what goes where.
- **Props:** `dpl.slide/3` gains an optional `props` map (validated against
  the contract), so `quiz.select` and `choice.answer` have a home; `tone`
  stays the slide's own.
- **Seeds and existing slides look different** where the flat column was
  hiding the component (agenda shows its list; title honours `align`):
  fresh proof shots at real window sizes, shown before merge.

## 4b. Amendment from the Q2 answer (R287) — awaiting the operator

**Styling is configurable, at three levels** (replaces "layout only"):
1. **Layout only** — the component places things; every value the slide and
   its looks set still shows. The default.
2. **Save it as my template** — a card's component with its styling (look,
   line looks, sizes, variant axes, props) saved as the operator's own
   template in the library (`dpl.template/1`, like LK2a's own looks), offered
   beside the built-in layouts on Author's shelf; a slide started from it
   follows it by reference, with the same chip, dots and stop-following as a
   look.
3. **A one-off variant for this lesson or slide** — on the slide itself
   (`variant`/`props`, already per slide), or for one lesson's use of a
   shared card without touching the card (`Use.variant`/`Use.overrides`, the
   research's point-of-use rule), set from the Lessons page.

Levels 1 and 3 land with LS4a (the engine reads the use's variant when a
lesson shows the card); level 2 lands as **LS4d** after the three slices, so
templates save what the engine can already paint.

**The rest of R287 is beyond LS4** and gets its own plans:
- **Sub-events on the touchscreen** — a slide holding sub-slides or events a
  push triggers (beyond today's reveals): with LS4c, where interactive
  components become node trees and recipes.
- **Two modes: build, and run the lesson** — run a lesson (not only an
  episode) on the deck, with the teleprompter reading each card's script: a
  short pass after LS4a (the deck and Presenter already read through the
  episode walk; a lesson is one chapter of it).
- **Portable, linked storage** — the content folder linked (an external
  drive) or imported into the program's folder; the app disconnects cleanly
  when it is absent and reconnects when it is plugged in; a clean nested
  structure per account (`.userdata`, `.aidata`, …). Its own program with its
  own plan — it changes where every document lives, and it touches paths
  outside the repo, so it is inventoried for sign-off first
  (`docs/plans/external-reach-inventory.md`).
- **devpulse.app / the dev version of devpulse** — recorded as the
  direction; nothing is built toward it until it is planned.

## 4c. LS4a — the steps (the work queue)

Grounded on `feat/ls3b2-move-episodes` (the renderer, the title card's paint,
the contract types) on 2026-10-24.

1. **A layout is data in the contract** (`component.json`'s optional
   `layout`, `schemas/component.v1.schema.json`, `dpl_doc::component::Layout`):
   `kind` (`stack` in LS4a), `align` (`left`/`center`, or `{"axis": id}` to
   read a variant axis), `slots` in reading order, each with an optional
   `place` (`backdrop`) and a default `size` and `ink`, and `when` rules
   (`{axis, is, align?, slots}`) that adjust them per variant. Validated
   (slots, axes, options, sizes and inks all real); a contract that gains a
   layout raises its version (the superset gate already allows it).
2. **Slot defaults sit in the cascade** between the slide's looks and the
   theme: a line's own value → its line look → **its slot's default** → the
   card look's font → the theme. The component gives hierarchy (a title is
   big, an eyebrow is small and toned); nothing the slide or its looks set
   is overridden (Q2/Q5 level 1).
3. **The renderer reads the layout** (`document::render_inner`, the one path):
   the slide's variant over each axis's default (and, when a lesson shows
   the card, the use's variant over that — step 6); lines in the layout's
   slot order, then any line the layout does not name in its own order (so
   nothing vanishes); left or centred; everything it does today kept. **A
   slide with no layout paints exactly as today** — the same code path with
   today's defaults — pinned by a test over every seed and test slide. A
   data block no longer claims the whole stage (the editor can select the
   background on a list card again); it is painted in LS4b.
4. **The six text components get layouts:** title (align axis), section,
   quote, stat (density axis), callout, media. Each migrated template's
   `ui()` draws its example slide through the renderer — its bespoke paint
   is deleted, so the Library and Author's shelf show exactly what a card
   started from it looks like.
4b. **Amendment (2026-10-24, the feel bar — reground on the after shots):**
   step 4 alone made the Library plainer than the old gallery. The bespoke
   paint drew things a stack of lines cannot: the callout's raised card,
   sheen and tone bar; the quote's oversized mark and the rule above its
   attribution; the section divider's big number top-left, full-width rule
   and bottom-pinned title; the media caption pinned to the bottom. A
   regression is not done (the experience law), so the layout grows the
   decorations **as data** — the design system's element categories
   (`container`, `mark`, `divider`), painted by the one renderer, so every
   real slide gets them too:
   - layout `container: "card"` (the column sits in a raised card: hairline,
     sheen, a bar in the card's colour); `anchor: top|middle|bottom` (where
     the column sits); `mark: "quote"` (an oversized, dim mark top-left);
   - slot `place: "top"` (out of the column, pinned top-left) and
     `rule: half|full` (a divider in the card's colour above the slot).
   Found on the way: **the renderer never painted the card's colour**
   (`slide.tone` — the quote's Colour prop, and the card look's Colour knob
   since LK1). The "tone" ink now reads the line's colour → the card's →
   the background's accent. The gallery card paints on the bare well, like
   the 22 templates not yet moved. Not restored: the callout's moving
   backdrop pattern (an `anim` picture slot never reached a real slide —
   picture slots are LS4b).
5. **Props on a slide:** `dpl.slide/3` gains an optional `props` map,
   validated against the contract (kind, options); `slide_from_example`
   copies an example's props (Q3).
6. **A one-off variant** (Q5 level 3): the card editor gains a **Layout** row
   — one chip row per variant axis (Alignment: Left · Centered), with the
   same dot-and-give-back as any value; a lesson's use gains `variant`
   (`dpl.lesson/1`, optional), set on the Lessons page with **Just this
   lesson**, merged over the card's own when the lesson or an episode shows
   it.
7. **The contracts gate** refuses a new required slot too.
8. **Proof:** unit tests (layout validation, the cascade order, slot order
   and alignment, the no-layout identity over every seed, props, the use's
   variant, the data-block hit-test); EguiPilotAI (the Layout row, Just this
   lesson); shots of every changed seed and the six components at 1440×900
   and 1100×700 (`--screenshot` for animated surfaces), shown before merge
   (Q4); the LK and LS scenarios re-run.

## 5. Wiring (option A)

| Where | Change | Pass |
|---|---|---|
| `crates/doc` (+ a `node` module), `schemas/slide.v3.schema.json` | the render tree, `expand`, `props`; egui-free (structure ceiling unchanged) | LS4a |
| `crates/slides/src/render/` (was `document.rs`) | the one renderer over the tree; `Stack` identity; slot/item rects | LS4a |
| `crates/slides/src/components/*` | each migrated template's layout as data; its `ui()` deleted | LS4a–c |
| `crates/editor` | hit-testing over slot/item rects; the data-block bug; `set_quote` | LS4a–b |
| `crates/deck`, Workbench surfaces | unchanged callers (they all go through the renderer) | — |
| `crates/xtask` contracts gate | a new required slot is refused too | LS4a |
| docs (requirements R285, status, journal, design-system §7 status) | the state | all |

## 6. Proof

- **Pixel identity:** every seed and test slide renders identically through
  `Stack` before any component moves (the design system's own gate).
- **Unit:** expand per component (slots, axes, props, missing required slot →
  an honest plate); slot rects stable across beats; the editor hit-tests a
  list item and the background on an agenda card.
- **Real exe:** EguiPilotAI over the editor (tap a slot, change it), the deck
  (beats, reveals), Author (Layouts shelf card = the minted card); shots at
  1440×900 and 1100×700 (animated surfaces by `--screenshot`).
- **Release:** `cargo green`, both lanes, the exe rebuilt from `main`.

## 7. Risks

- **The biggest pass in the program** — the renderer every surface uses.
  Mitigated by the identity gate first, then one component family at a time.
- **Look regressions:** the templates' hard-coded styling must not leak in;
  the identity gate and the LK scenarios re-run catch it.
- **Ordering:** option A writes `Tone` call sites the naming merge will
  rename later (the cost the design system's ordering law names).
- **Operator content:** slides whose component the flat column happened to
  render will change; a read-only census over content (like LS3b2's dry run)
  reports which before merge.

## 8. Evidence

**LS4a** (branch `feat/ls4a-component-engine`, 2026-10-24):

- **Unit:** dpl-doc 102 (layout validation incl. the decorations and the
  place/entity rule, the axis and `when` merge, the cascade order with slot
  font, props, a use's variant, the new-required-slot rule), dpl-slides 42
  (reading order and centring, the no-layout flow, the data block claiming
  no stage, a template's values as a slide, the section's pinned number and
  bottom title, the callout's lines inside its card, shrink-to-fit with the
  quote's mark), dpl-editor 17, dpl-hub-workbench 65, dpl-deck 16; clippy
  `-D warnings` clean on the four touched crates.
- **EguiPilotAI** (devtools exe, scratch root `target/eguipilotai/ls4a`):
  `layout-make` (Author → Title card → Layout: Centered → Put it in) and
  `lessons-layout` (the lesson's Layout → Left → "Same as the card" shows)
  pass at 1440×900; `layout-make-1100` and `lessons-layout-1100` pass at
  1100×700. On disk: the card's `slide.json` holds `variant.align =
  "center"`, the lesson's use `variant.align = "left"`; the lesson's
  thumbnail paints left, the editor centred.
- **The LK and LS scenarios re-run** on the LS4a devtools exe, each over its
  scratch root: all 23 pass on the first try (author-start-from-layout,
  editor-look-by-reference, linked/own/spread/line/picture looks and their
  `-1100`s, the LS3a lessons chain, the LS3b1 episodes chain, the LS3b2
  move and its `-1100`). The two `library` fixtures have no scratch root
  and were not run.
- **The feel bar** (before = the release exe at main `b69944a`, after = the
  devtools exe; `docs/proof/ls4a-*.png`): the first after-shots failed it —
  the callout lost its card, the quote its mark and rule, the section its
  composition, the stat its centring — so step 4b was added (amendment
  above). After 4b: title, quote, section, callout and media match the old
  anatomy; the stat is centred again. **Trade-offs to show the operator:**
  - type is the card's real font now (the theme's, or the card look's) where
    the old gallery drew the default face;
  - the callout's drifting stripes and the stat's rings are gone from the
    gallery — they were an `anim` picture slot no real slide ever received
    (`slide_from_parts` drops it); picture slots are LS4b;
  - the card's colour now paints "tone" lines on every slide that has one
    (a card look's colour since LK1, a component's Colour prop) — before,
    those lines took the background's accent (e.g. Neon intro: amber, was
    amber-ember).
- **Found, not LS4a:** at 1100×700 Author's "Put them in" sits below the
  fold (the harness cannot scroll, so `layout-make-1100` stops at Done).

## Questions for the operator (round 2, from R287)

- **Q5.** The three styling levels as §4b reads them — layout only by
  default; "save as my template" (LS4d); a one-off variant on a slide or on
  one lesson's use (LS4a) — right?
- **Q6.** Order after LS4a: run-a-lesson mode, then LS4b/LS4c (with touch
  sub-events), then LS4d; and portable linked storage planned as its own
  program — before or after LS4?

## Questions for the operator (round 1, answered)

- **Q1.** Which way: **A** (the ratified render tree now, in three slices:
  engine + text components → lists/data → interactive), **B** (the design
  system's naming/recipes/fidelity steps first, then LS4), or **C**
  (each template paints slides directly, no render tree)?
- **Q2.** Components honour everything the slide and its looks set (fonts,
  sizes, colours, line looks, entrances) and decide only the layout — yes?
- **Q3.** Add an optional `props` map to slides for settings like a quiz's
  single/multi choice?
- **Q4.** Seeds and existing slides will look different where the flat
  column hid the component (an agenda shows its list; a title honours its
  alignment) — approve, with fresh shots shown before merge?
