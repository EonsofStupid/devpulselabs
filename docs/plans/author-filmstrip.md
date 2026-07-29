# Pass — the container-labelled filmstrip

The authoring surface's second half: a **container bar** that says what you
are building into, and a **filmstrip** along the bottom that shows what you
have started and labels itself with the container that is loaded.

Reference: mock artifact `a3db4aee` (ratified). Lane journal:
`docs/journal/2026-07-29-authoring-lane-resume.md`, NEXT item 2.

## 1. Context — why this pass exists

`author.rs` can start a card and open its editor, but a started card
vanishes from view the moment the overlay closes. There is nowhere that
answers "what have I made in this sitting, and what is it for?"

The mock answers both with one fixed band at the bottom of the surface: a
kind pill, the container's name, a count in that kind's own unit, and a row
of what you have started. The operator's words, recovered from the mock's
own translation comment: **containers are "loaded, never compiled in"**.

The container model has been ratified since the reset and nothing implements
it — `KINDS` and `CONTAINERS` exist only as JS arrays in the mock. This pass
is where they become real, and where the honest gaps in them become visible.

## 2. Current state (ground-truthed in-session, `b17ddc8`)

**The mock's container model** (verbatim from the artifact):

```js
const KIND_TONE={lesson:"cyan",guide:"amber",episode:"magenta",deck:"coral"};
const KINDS=[
  {id:"lesson", label:"Lesson",  unit:"cards", blurb:"teaches one idea, start to finish"},
  {id:"guide",  label:"Guide",   unit:"steps", blurb:"walks through a real task, step by step"},
  {id:"episode",label:"Episode", unit:"cards", blurb:"one show instalment, under a show"},
  {id:"deck",   label:"Deck",    unit:"cards", blurb:"a run of cards for the touchscreen"}
];
```

The strip head is `kindpill · <b>name</b> · N new <unit>`, then `✦ Ask for
help` and `＋ New card`. Empty reads *"Tap something above to drop your first
{unit-singular} in here."* A trailing dashed `＋` tile always closes the film.

**What actually exists on disk** — this is the finding that shapes the pass:

| kind | storage today | enumerated by |
|---|---|---|
| **episode** | REAL — `content/<show>/<episode>/episode.json`, chapters → slide refs | `dpl_doc::list_content_episodes`, `load_episode`, `episode_slides` |
| **guide** | REAL — `inbox/<name>/guide.json` (the `.guide` folder artifact) | `dpl_guide::load_guide` (per-folder; there is **no** `list_guides`) |
| **deck** | NONE — `dpl-deck` is a *runtime host* that plays an episode full-bleed, not a stored container | — |
| **lesson** | NONE — the word appears nowhere in the codebase | — |

So two of the four ratified kinds are backed and two are not. `dpl_doc` also
distinguishes a show (`holds_an_episode`) from a guide folder by exactly this
shape, so the two real kinds never collide.

**The surface** (`author.rs`, after #141): intent bar → shelf picker → one
carousel → blank-card button → status line → overlay editor. `mint_draft`
writes a real `slide.json` under `inbox/authored-draft-N/` and publishes
`BuildEvent::CardAuthored` after the write. Nothing tracks the drafts a
sitting has produced; `next_ix` is a hint, not a list.

**Deps**: `dpl-hub-workbench` already carries `dpl-doc` and
`dpl-engine-capture`. `dpl-guide` is an ENGINE crate and a hub may depend on
engines (architecture.md §1); hubs carry no exact-set ceiling in §2, and the
audit's hub rule is "no lateral hub deps". Adding it needs **no**
architecture.md amendment — asserted here, verified by running the audit.

## 3. Design — the container census, and honesty about the two gaps

**A kind is a registry entry, not a string.** `Kind` gets the same
`slug()`/`label()` split every tier on this surface already uses, plus
`unit()`, `blurb()` and `tone()`. `slug()` is `lesson|guide|episode|deck`
and is never drawn.

**One function enumerates containers** — the Census law:

```rust
fn containers(kind: Kind, root: &Path) -> Vec<Container>
```

- `Episode` → `list_content_episodes` → name `"<show> · <episode>"`, count
  from `episode_slides`.
- `Guide` → scan `inbox/*/guide.json`, load each with `dpl_guide::load_guide`
  → name from the manifest, count = steps.
- `Deck`, `Lesson` → `Vec::new()`, deliberately.

A kind with nothing behind it says so in its own words rather than looking
broken — the same honest-empty move `blocks_wall` makes for the empty chunk
tier. Deck's line says a deck is an episode played on the touchscreen and has
no folder of its own yet; Lesson's says lessons are not stored anywhere yet.

**A census test pins which kinds are backed.** `every_kind_is_backed_or_named`
asserts the backed set is exactly `{episode, guide}`. The day lesson storage
lands, that test fails and forces the kind to be wired at the same time —
this is the promotion rule applied to containers, and it is why the two gaps
are registered rather than omitted.

**The strip labels itself from the loaded container**, never from a constant:
kind pill (tone-coded per `KIND_TONE`), the container's name, and the count
in that kind's `unit()`. With no container chosen, the bar asks for one and
the strip stays hidden — a strip labelled "—" would be a lie.

**What the strip lists is this sitting's drafts**, and the wording says so.
A draft lives in `inbox/`; it is not *in* the container until something
copies it there (`dpl_doc::place_slide`, Studio's lane). The head therefore
reads "N new <unit> — headed for <name>", not "N cards in <name>". Claiming
placement that has not happened is the one thing this band must not do.

## 4. Scope

**In:** `Kind` + `Container` + the census and its test · the container bar
(kind picker, which-one picker) · the filmstrip band (head, film, trailing
new tile, honest empty) · drafts recorded as they are minted so they appear
· the two honest-empty kind lines · a proof seam · tooltips on every control.

**Out, and named:**
- The **dashboard IA** (edit existing / expand the platform / work on
  content). The mock puts it before this surface; it is its own pass, and
  the `← Back` and `Change` controls belong to it.
- **Actually placing** a draft into the container. That is a real mutation
  over `place_slide` and deserves its own pass with its own proof.
- The **holographic gap arrows** (`flagGap`). They belong with the option
  overlays in the tabs pass, where the gap they signal actually occurs.
- **Live mini renders inside the film cards** — that needs the thumbnail
  mode from NEXT item 4; until then a film card carries its name and a
  labelled plate, which is the honest degrade the document renderer already
  uses for an unknown ref.

## 5. Work plan (atomic, in order — commit at each step that compiles)

1. `containers.rs`: `Kind`, `Container`, `containers()`, the census test,
   the honest-empty copy. No UI. Compiles and tests alone.
2. `dpl-guide` onto the workbench manifest; run `cargo xtask structure` to
   confirm no amendment is owed.
3. The container bar in `author.rs`: two pickers, tooltips, remembered
   choice. Strip still absent.
4. The filmstrip band: head from the loaded container, film from the
   sitting's drafts, trailing new tile, empty line.
5. `mint_draft` records the draft so the film fills as cards are started.
6. The proof seam (`--author-container <kind>/<slug>`), CLAUDE.md's flag
   list, and the bidirectional flag census.
7. Proof shots, plan §8, `docs/STATE.md`.

## 6. Risks

- **The strip eats the surface at small heights.** The carousel already
  fights for vertical room and lost once at 1100×700. The band is fixed to
  the bottom, so the carousel's `max_card_h` must subtract it before it
  proposes a card width. Mitigation: same width-proposes/height-disposes
  guard, extended; proof at 1100×700 as well as 1280×800.
- **A guide scan touches every `inbox/` folder.** `inbox/` also holds
  authored drafts. Read once into state (like `load_once`), never per frame.
- **Two of four kinds are empty.** If the empty copy is vague it reads as
  broken. Mitigation: each empty line names *why* and what would fill it.
- **`/content` is operator data.** Enumeration is read-only; drafts continue
  to go to `inbox/` and proof runs point `--content-root` at a scratch dir.

## 7. Verification / acceptance

- `cargo green` clean (fmt · spawn audit · structure audit · clippy -D
  warnings · tests · release build).
- `every_kind_is_backed_or_named` pins the backed set to `{episode, guide}`.
- A census test asserts the kind chips are exactly `Kind::ALL`.
- `no_kind_speaks_engineering` — the drawn strings carry no code words,
  matching `no_chip_speaks_engineering` in `library.rs`.
- Real-exe proof shots at **1280×800** and **1100×700**, sequential runs,
  `--content-root` at a scratch dir: strip labelled by a real container,
  film holding a started card, and the honest-empty line for an unbacked
  kind.
- Tooltip law: every control in the bar and the strip head ships `cl::tip`.

## 8. Evidence

_Filled during the pass._
