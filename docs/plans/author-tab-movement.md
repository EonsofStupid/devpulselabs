# Pass — the Movement tab

The second of the overlay editor's six placeholders: how a card's words
arrive. Small, because the truth is small — and the point of the pass is to
make that truth visible instead of promising more.

Reference: mock artifact `a3db4aee` (ratified). Follows
`docs/plans/author-tab-background.md`.

## 1. Context — why this pass exists

The Movement tab currently reads:

> "How things come in, get noticed, and go out — tapped on, and shown on the
> card beside you."

Every word of that is a promise. §8.4 of the Background pass already recorded
that the registry cannot keep most of it, and this pass is where the promise
is either kept or corrected. Leaving a placeholder that overstates what
exists is worse than a small tab that is honest.

## 2. Current state (ground-truthed in-session, `e3904bc`)

**The document permits exactly two entrance values.** `dpl_doc`:

```rust
const ENTERS: &[&str] = &["rise", "none"];
```

`validate` rejects anything else, so a tab offering a third would write a
document the app refuses to load.

**The renderer implements exactly one.** `document.rs`:

```rust
let rising = slide.enter.as_deref() == Some("rise");
let fx = cl::motion::ENTER_RISE;
```

`"none"` is the absence of the animation. There is no second code path.

**`Slide::enter` is three-valued, and the third value matters.** `None`
inherits the show's Theme (`Theme::enter`, which the fallback sets to `"rise"`);
`Some("rise")` and `Some("none")` are deliberate overrides. Inheriting is a
real, distinct answer and the editor already treats it that way for fonts and
inks ("Show look" chips).

**The 18 motion effects are NOT slide choices.** `tokens/motion.json` holds
`chart-rise`, `bar-grow`, `gauge-sweep`, `reward-pulse`, `hotspot-pulse` and
the rest — but these are the vocabulary TEMPLATES use internally for their own
elements, and `cl::motion::ENTER_RISE` is the only one the document path
consumes. Offering the other seventeen on this tab would be offering refs the
document cannot hold.

**There are no exit effects at all.** Not in the tokens, not in the schema,
not in the renderer. "How things go out" cannot be honoured, and no amount of
UI work in this pass changes that.

So the honest shelf is **three** choices, not forty:

| choice | document |
|---|---|
| Follow the show | `enter: None` |
| Rise into place | `enter: Some("rise")` |
| Stay still | `enter: Some("none")` |

## 3. Design — three real choices and one honest sentence

**The tab offers the three, drawn from the schema's own list** rather than a
hand-written array, so a fourth entrance added to `ENTERS` shows up here the
day it lands. `dpl_doc` gains a small accessor for its authorised vocabulary;
the surface must not re-declare a list the validator owns, or the two drift
and the UI starts writing documents that fail.

**Each choice previews itself.** Tapping restarts the card's clock so the
entrance actually plays on the live preview beside the tabs — a movement
picker that does not move is a contradiction.

**The gap is stated where it is felt, not buried in a doc.** Under the three
choices, in plain words: things arriving is all that is built; getting
noticed and going out are not, and what has to exist first (an effect
authored in the tokens, a value allowed by the schema, and a renderer that
plays it). This is the same honest-empty move `Kind::not_stored_yet` makes
for containers and `blocks_wall` makes for the empty chunk tier.

**No new tokens are authored in this pass.** Adding `enter-fade` and
`enter-bloom` to the schema is a real improvement and a real pass — it needs
the renderer to select among effects, which today hardcodes `ENTER_RISE`.
Doing it here would mean shipping a registry change, a schema change and a
renderer change under a tab's name.

## 4. Scope

**In:** the three choices from the schema's vocabulary · plain labels · the
clock restart so a pick plays · the honest sentence about what is missing ·
tooltips · tests · proof shots.

**Out, and named:**
- **Authoring more entrances.** `enter-fade` / `enter-bloom` exist as effects
  but no schema value or renderer branch. Own pass: tokens + `ENTERS` +
  a renderer that picks, all together.
- **Exit effects.** Nothing to build on; needs authoring first.
- **Per-block movement.** `enter` is slide-level. Per-block is a schema
  change and its own decision.
- The remaining four tabs.

## 5. Work plan (atomic, in order — commit at each step that compiles)

1. `dpl_doc::enters()` exposing the authorised vocabulary, with a test that
   it matches what `validate` accepts.
2. The tab: three choices over it, labels in plain words, writes through
   `Editor::set_enter` (new, same `mutate` path as `set_background`).
3. The clock restart on pick, so the preview plays what was chosen.
4. The honest sentence.
5. Proof shots, §8, `docs/STATE.md`.

## 6. Risks

- **A three-option tab may read as unfinished.** It is finished; the
  *feature* is small. Mitigation is the sentence — the operator should
  finish this tab knowing exactly what exists and what does not, rather than
  wondering whether the UI is broken.
- **The surface re-declaring the vocabulary.** If it hand-lists
  `["rise", "none"]` it will drift from `ENTERS` and start writing invalid
  documents. Mitigation: the accessor, and a test that ties them together.
- **"Follow the show" is invisible without context.** Inheriting looks the
  same as whatever it inherited. Mitigation: name what it currently resolves
  to, so the choice is legible.

## 7. Verification / acceptance

- `cargo green` clean.
- A test that the tab's choices are exactly the schema's vocabulary plus
  inherit — it fails the day `ENTERS` grows, which is the point.
- Kittest: tapping each choice writes the expected `enter` and the document
  still validates.
- Real-exe proof at 1280×800 and 1100×700 via `--card-tab movement`.

## 8. Evidence

### 8.1 The gate

`cargo green ✔`. 20 tests in `dpl-hub-workbench`, 3 new.

`the_movement_row_is_the_schemas_vocabulary` asserts `dpl_doc::enters()` is
`["rise", "none"]` AND that every entry has an operator word — it fails the
day `ENTERS` grows, which is the point: a new entrance must reach the picker
rather than sit in the schema unoffered.

`picking_a_movement_writes_it_and_still_validates` drives the real widget for
both values and validates each resulting document.
`following_the_show_clears_the_override` pins that inheriting writes
**nothing** — it is an absence in the document and must stay one.

### 8.2 Real-exe proof

`docs/proof/author-tab-movement-1280x800.png` and `-1100x700.png`, via
`--card-tab movement`. The tab shows "Follow the show — rise into place"
(naming what inheriting resolves to), "Rise into place", "Stay still", then
the honest sentence.

### 8.3 Findings

**The shelf is three, and that is the whole truth.** §8.4 of the Background
pass estimated three true entrances from the effects registry. Reading the
schema showed it is narrower still: `dpl_doc::ENTERS` permits exactly
`rise` and `none`, and `document.rs` implements `rise` alone. The other
seventeen motion effects belong to TEMPLATES, for their own elements — the
document path consumes `ENTER_RISE` only. Offering them would offer refs the
document cannot hold and the validator would reject.

**The vocabulary is now exposed, not copied.** `dpl_doc::enters()` returns
the same slice `validate` enforces. A surface hand-listing `["rise","none"]`
would drift the day a third is added and start writing invalid documents.

**"Follow the show" names what it resolves to.** Inheriting is invisible by
nature — it looks like whatever it inherited. The chip reads "Follow the show
— rise into place" via `Editor::theme_enter`, so the default is legible
instead of a blank.

**Picking restarts the clock.** `Editor::set_enter` restarts it so the
entrance actually plays on the preview. A movement picker whose preview does
not move is a contradiction.

**The gap is stated on the tab, not in this file.** "Arriving is all that is
built. Getting noticed and going out are not. Each one needs a movement
authored, the card's file taught to name it, and the player taught to play it
— so they arrive together or not at all." That last clause is the reason this
pass did not add `enter-fade`: it would be a tokens change, a schema change
and a renderer change shipped under a tab's name.
