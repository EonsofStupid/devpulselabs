# Pass — the Background tab

The first of the overlay editor's six placeholder tabs to become real: what
sits behind the words, tapped straight onto the card.

Reference: mock artifact `a3db4aee` (ratified). Lane journal:
`docs/journal/2026-07-29-authoring-lane-resume.md`, NEXT item 3.

## 1. Context — why this pass exists

The overlay editor (#141) shipped seven tabs. **Words** is the real
`dpl_editor::Editor`; the other six say what they will hold and admit they
are not built. That was the right call then and is a debt now: the mock's
copy for this one reads

> "Two colours blended, something moving, or one flat colour — around forty
> to choose from, tapped straight onto the card."

Background is the tab to do first. It is the one that changes how a card
LOOKS the instant you tap it, it maps onto a document field that already
exists, and it needs no new document schema.

## 2. Current state (ground-truthed in-session, `28346ab`)

**The document already models this.** `dpl_doc::Background`:

```rust
pub enum Background {
    MoodBlend { moods: Vec<String>, motion: String, ambience: Option<String> },
    Mood { mood: String },
    None,
}
```

So a background is already "one colour", "two blended", or "nothing", with an
optional ambience above it. No schema change is owed.

**The registries hold less than the mock promises — this is the finding.**

| ingredient | registry | how many | ids |
|---|---|---|---|
| moods | `cl::MOODS` (`tokens/moods.json`) | **10** | ember-drift · deep-lagoon · orchid-haze · botanical-night · solar-flare · glacier-signal · violet-static · terracotta-dusk · jade-current · fuchsia-arc |
| ambience | `slides::elements::Ambient` | **3** | rings · drift · pulse |
| blend motion | `cl::MOTION_PRESETS` | **3** | calm-rise · soft-fade · bloom-in |

Ten moods, not forty. The mock's number is aspirational, and the honest
reading is that the count comes from **combination**, not from a shelf of
forty authored things: 10 flat + 45 pairs × 3 motions × 4 ambience choices is
far more than forty, and all of it is real.

**Moods carry no operator-facing label.** `tokens/moods.json` entries are
`{ name, stops, scrim, accent }` — no `label`, no `blurb`. The names are
already evocative rather than technical ("ember-drift", "jade-current"), so
`pretty()` covers the plain-words law without authoring new data. This is
worth stating because the DS3 rule is normally "author a label"; here the
authored name IS the operator's word.

**The stage renders all of this already.** `document.rs::render_inner` paints
`Background::MoodBlend` via `cl::mood_blend` and the ambience via
`ambient_backdrop`. Nothing new needs painting — the tab picks refs, and the
existing one render path shows them.

**Thumbnail mode just landed** (`docs/plans/slide-thumbnail-mode.md`), so a
grid of small live previews is now possible without cropping. Before it, this
tab could not have shown what it was offering.

## 3. Design — pick a colour, then optionally a second and a movement

**Every option is a live miniature, not a swatch.** The card next to it is
the real document; the option tiles are the real backdrop painted through the
same path. A picker that shows an approximation of what you will get is the
thing this whole lane exists to avoid.

**Three rows, in the order a person decides:**

1. **The colour** — 10 mood tiles + a "nothing" tile. Tapping one writes
   `Background::Mood`. This alone is a complete answer.
2. **Blend a second** — the same 10, offered only once a first is chosen.
   Tapping one promotes to `Background::MoodBlend` with a default motion.
   Tapping the chosen one again drops back to flat.
3. **Something moving** — the 3 ambiences + "still", and (when blended) the
   3 blend motions.

Rows 2 and 3 stay collapsed until row 1 is answered, so the first decision is
never buried under two it depends on.

**The count is stated, not hidden.** The tab says how many colours there are.
When the registry grows, the number grows with it — a census, like every
other shelf on this surface.

**Writes go through the document.** Tapping edits the open card's
`dpl_doc::Slide` and saves, exactly as the Words tab does. The live preview
beside the tabs re-renders because it reads the same document.

## 4. Scope

**In:** the Background tab's three rows drawn from the real registries · live
miniature tiles · writing `Background` into the open card and saving · the
count stated · tooltips on every tile · tests · proof shots.

**Out, and named:**
- The other five tabs. Movement is next and is a bigger pass (see §6).
- **Authoring more moods.** If ten is too few, the answer is new entries in
  `tokens/moods.json` — a data pass, not a code one. This pass makes the
  shortfall visible rather than papering over it.
- Per-block backgrounds. `Background` is a slide-level field; making it
  per-block is a schema change and its own decision.
- The holographic gap arrows. They belong with a scrolling option grid; this
  tab's rows fit without scrolling, so inventing the gap to signal would be
  backwards.

## 5. Work plan (atomic, in order — commit at each step that compiles)

1. A `backgrounds.rs` census in the workbench: the mood list, the ambience
   list, the motion list, each with `slug()`/`label()`, plus the test that
   the drawn row IS the registry.
2. The Background tab's row 1 (colour) over that census, tiles painted as
   live miniatures. Writes + saves.
3. Rows 2 and 3 (blend, movement), collapsed until row 1 is answered.
4. The proof seam — `--card-tab <slug>` so a tab other than Words can be
   photographed on the exe.
5. Proof shots, §8, `docs/STATE.md`.

## 6. Risks

- **Ten miniatures painting live in one overlay.** Each is a real render.
  Measure before assuming it is fine; if it is not, the fix is the texture
  cache deliberately deferred in the thumbnail pass, with evidence.
- **The mock says forty and the shelf will say ten.** The operator ratified
  the mock. Mitigation: say the real number plainly and name the authoring
  task; do NOT pad the shelf with generated variations to hit a number.
- **Movement is the harder tab and this pass shapes it.** The 18 motion
  effects are mostly element-level (`chart-*`, `bar-grow`, `gauge-sweep`) and
  only three are true entrances (`enter-fade`, `enter-rise`, `enter-bloom`).
  **There are no exit effects at all**, so the mock's "how things come in,
  get noticed, and go out" cannot be honoured by the registry as it stands.
  Named here so that pass starts from the truth rather than rediscovering it.

## 7. Verification / acceptance

- `cargo green` clean.
- A census test: the colour row is exactly `cl::MOODS`, the ambience row
  exactly the `Ambient` variants — a mood added to the tokens appears without
  a code change.
- A test that tapping writes a valid `Background` and the document still
  passes `dpl_doc::validate`.
- `no_option_speaks_engineering` over every drawn string.
- Real-exe proof at **1280×800** and **1100×700**: the tab open, the tiles
  live, and the card beside it showing the pick.
- The feel bar: tapping a colour must change the card visibly and instantly.

## 8. Evidence

_Filled during the pass._
