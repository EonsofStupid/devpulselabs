# The Workbench EDITOR — the one surface, under its real name

Parent: `docs/plans/authoring-charter.md` (ratified thesis) + the ratified mock
`docs/design/mocks/authoring-surface-mock.html` (artifact `a3db4aee`).
Supersedes the naming in every `author-*.md` plan. Branch: off `main`.

## 1. Context

The operator asked, repeatedly and in these words, for the **Workbench
Editor**:

- `2026-07-28 22:20` — *"you also left workbench editors old weird line by line
  editing all rough instead of the visual we discussed — these should be two
  clear separations where you have the raw editor for workbench for new
  templates, elements blocks and then we have the streamlined workflow we
  discussed with the overlays and popup models"*
- `2026-07-28 22:35` — *"Imagine first we have the editor under Workbench
  selected... internally it uses the proper terminology while on the UI it
  keeps simple"*
- `2026-07-29 00:43` — *"the workbench editor its mock reference ... we use
  blocks elements and templates for the **user facing UI** while keeping it
  aligned platform industry code and folder structure"*
- `2026-07-31 01:54` — *"you mixed my fucking wiredfront editor with my
  workbench — the entire point is to get a proper library of templates that
  then drill down to elements and blocks that are well organized then we have
  it simple: new or existing card → click → overlay appears → select → preview
  → save"*

What was built instead is named **Author**, and it stands BESIDE the old rough
editor rather than replacing it. Three defects, all naming/structure, none of
them a rewrite:

1. The surface wears an invented name (`author`).
2. The user-facing vocabulary is **inverted** — the operator's words are hidden
   in machine slugs and invented synonyms are drawn on screen.
3. The old wall-of-text editor is still mounted next to it, so every release
   still shows it and nothing ever reads as finished.

This pass fixes exactly those three. It builds no new feature.

## 2. Current state (ground-truthed this session, file:line)

**Two surfaces are mounted side by side** — `crates/hubs/workbench/src/lib.rs`:

```
:31  pub const AUTHOR: SubId = SubId("author");   // author.rs, 3591 lines — the real one
:33  pub const EDITOR: SubId = SubId("editor");   // lib.rs:750 editor_surface() — the wall of text
:395 } else if route.sub == AUTHOR { self.author.ui(..) }
:397 } else if route.sub == EDITOR { self.editor_surface(ui) }
```

`editor_surface()` (`lib.rs:750-830`) is a flat scrolling list of `label ·
edit ›` text rows over `list_seeds()` + `all_episodes()`. That is the "wall of
text ... links that you dont even realize are links they look like text."

**The engine is already shared — this is NOT two implementations.**
`author.rs:933` — *"**Words** is the REAL `dpl_editor::Editor` over the card's
own document"*; `author.rs:1046, 1630`, and 12 tests open it directly.
`crates/editor` (`dpl-editor`, 1035 lines) is the write-back engine and it
**stays**. Only the standalone *surface* is redundant. (I told you earlier this
pass would delete a crate — that was wrong; the crate is load-bearing.)

**The vocabulary is inverted** — `author.rs:862-902`:

| drawn on screen (invented) | hidden in the slug (the operator's word) |
|---|---|
| Starters | `seeds` |
| Layouts | `templates` |
| Combos | `blocks` |
| Pieces | `elements` |
| Looks | `traits` |
| Interactions | `recipes` |

The ratified mock (`authoring-surface-mock.html:194-199`) says the drawn
buckets are exactly four: **Blocks** (chunks) · **Elements** (atoms) ·
**Effects** (looks/recipes) · **Templates** (slides). Charter §1 says the same.
`library.rs:37-49` (`LibSection`) already uses the correct words — so the
Library wall and the Editor currently disagree with each other.

**The overlay tabs** — `author.rs:938-979`: Words · Background · Movement ·
A quote · Picture. The mock's five are Content · Quote · Media · Interactions ·
Ask AI. Words/Background/Movement are an honest refinement of "Content" and
stay. Picture = Media, A quote = Quote — same things. **Interactions and Ask AI
were never built** (§5).

**One signal exists** — `crates/fabric/src/lib.rs:375` `BuildEvent::CardAuthored
{ show, episode, slide }`, published after the draft write. It is the surface's
only emitted fact and it carries the wrong word. Consumers:
`crates/seam/src/lib.rs`, `crates/ledger/src/lib.rs`,
`crates/app/src/coach_signals.rs` (the `COARSE` coachable catalog as
`build.card-authored`).

**Blast radius, counted:**

| what | rust files | doc files |
|---|---|---|
| `--author-*` flags | 3 (`app/main.rs`, `workbench/lib.rs`, `author.rs`) | 8 |
| `CardAuthored` / `build.card-authored` | 5 | 8 |
| route string `workbench/author` | **0** | 2 |

No crate outside `hubs/workbench` names the surface. `xtask` route audit
(`main.rs:1031`) lists `wiredfront/editor`, not this one.

## 2.1 THE COMPOSITION SCHEMA — what it can say, and the gaps

Operator, 2026-07-31: *"we have far too many gaps in the actual recipe/seed for
the composition."* Correct. Here is the whole document
(`crates/doc/src/lib.rs:19-131`, `SCHEMA_V = 2`):

```rust
struct Slide {
    v: u32,
    template: String,                 // which registered template renders it
    background: Option<Background>,   // MoodBlend{moods[2],motion,ambience} | Mood | None
    blocks: Vec<Block>,               // Text{id,text,font?,size,ink?} | Media{id,as,src,fit?,poster?}
    enter: Option<String>,            // entrance ref
    reveals: Vec<String>,             // the beat grammar (D2)
}
```

Cascade is exactly two levels — episode `Look` → `Slide`, sparse means inherit
(`:695-760`). What is genuinely good and must not be lost: **refs never
values** — a slide names `"orbitron"`, `"ember-drift"`, never a hex or a pixel.
That single property is the reason presets can work at all.

Now the flow (§3.0) against that schema, honestly:

| flow step | schema support | gap |
|---|---|---|
| new / existing card | ✅ `Slide` | — |
| add **text** | ✅ `Block::Text` | — |
| add **picture** | ✅ `Block::Media` | — |
| add **quote** | ⚠️ convention over two Text blocks (`:222`) | **no attribution, no source URL** — charter §2 ratified a real source ref, "never pasted prose" |
| pick a **preset** | ❌ **nothing** | **THE GAP** — a slide cannot say which preset styled it |
| **colour** (always exposed) | ❌ **nothing** | `ink:"tone"` means *use the tone*, but the tone itself is never in the document — it comes from the caller |
| font · size · ink | ✅ `Block::Text` | — |
| background · ambience | ✅ `Background` | — |
| motion | ⚠️ two of them — `Slide.enter` and `Background.motion` | which one is the "motion" knob? |
| save | ✅ | — |

Two further gaps the flow does not hit yet but the charter's law requires:

- **Recipes / interactions** — charter §4's entire law is that a card
  references a registered recipe (`recipe:"explode-reveal"`) so a phantom
  interaction fails validation. **No field on `Slide` or `Block` can hold
  one.** The Effects bucket offers recipes the document cannot reference.
- **Provenance** — no created/modified, no record of what generated a card.
  The compounding-library economics (charter §5) are unmeasurable without it.
- **Title** — `Episode`, `Chapter`, and `SlideRef` carry titles (`:479-496`);
  the slide document itself does not. The filmstrip caption is the folder id.

### 2.2 The architectural fork: a preset is a REF, not a bake

This is the decision everything else hangs on.

**Bake** — picking a preset copies its font/moods/motion/ambience onto the
slide and forgets it. Simple, and it **breaks the spec on reload**: reopen the
card and the Editor cannot know which preset it was, so it cannot know which
4-7 traits to expose. "Only 4-7 knobs" would hold only until you close the lid.

**Ref + sparse overrides** — the slide stores `preset: "neon-intro"` plus only
the traits you actually changed. This is the standard composition pattern
(design tokens, Figma components + overrides, symbol/instance) and it is the
only one where the spec survives a reload:

- reopen → read the preset → look up its `exposes` → show exactly those knobs;
- improve a preset once → every card using it improves;
- the on-disk diff stays tiny and readable;
- it preserves refs-never-values rather than smearing resolved values around.

Cost, stated plainly: the cascade goes from two levels to three (`Look` →
`preset` → `slide overrides`), `resolve()` gains a step, and `SCHEMA_V` goes
**2 → 3**. `load` already refuses newer versions loudly, so old builds fail
honest rather than misreading — which is exactly what the v1→v2 bump was for.

### 2.4 RATIFIED — everything is an ENTITY; presets are entity-scoped

> **2026-10-21 (R285):** these rulings stand and are carried into the
> five-layer model of `docs/research/lesson-slide-schema.md` §5. The v3
> slide below gains identity (`id`, `rev`), a component reference and
> slot-keyed content as `dpl.slide/3` in LS2; the entity presets keep
> `for` and `exposes` exactly as ruled here.

**OPERATOR RULING 2026-07-31: "colour lives per entity."** Colour is one of the
4-7, so the 4-7 are the *selected entity's* traits — which makes the preset
entity-scoped too. You add text, then you style **that text**. The card is
itself an entity (the stage), so the model closes with no special cases:

> Every entity has a preset. Every preset exposes 4-7 traits. Colour is always
> one of them.

| entity | its preset carries | typical `exposes` |
|---|---|---|
| **card** (the stage) | moods · motion · ambience · tone | background · ambience · motion · colour |
| **text** | font · size · ink · tone · enter | font · size · ink · colour · motion |
| **picture** | fit · tone · enter | fit · colour · motion |
| **quote** | font · size · ink · tone · attribution style | font · size · ink · colour |

Today's five presets (`neon-intro`, `glacier-stat`, …) are **card** presets —
they carry moods and ambience, which only the stage has. They are not thrown
away; they are re-filed under the card scope and grown to 10-15. The other
three sets are new data.

Two consequences for §3.1:

- `presets.json` gains a `for` field naming the entity kind, and the build-time
  check becomes per-scope. An `exposes` name must be a knob **that entity
  actually has** — a text preset exposing `ambience` fails the build.
- "10-15" is **per entity kind**, not 10-15 in total.

And one for the schema: `preset` and `tone` are **not** slide-level fields.
They sit on every entity — the slide for the card, each block for its own:

```rust
struct Slide {
  v: 3, template, title: Option<String>,
  preset: Option<String>,          // the CARD entity's preset
  tone: Option<String>,            // the CARD entity's colour
  background: Option<Background>,  // sparse OVERRIDE of the preset
  blocks: Vec<Block>, enter: Option<String>,
  recipe: Option<String>,          // ratified #4 — reserved, unused this pass
  reveals: Vec<String>,
}
Block::Text  { id, preset?, tone?, text, font?, size?, ink? }
Block::Media { id, preset?, tone?, as, src, fit?, poster? }
Block::Quote { id, preset?, tone?, text, attribution, source? }   // ratified #3
```

`Block::Text.size` goes **required → optional** in v3: size now comes from the
preset, and a present value means a deliberate override. That is the single
migration hazard in the bump, so the v2→v3 migration writes the old required
`size` through as an explicit override — no card changes appearance.

### 2.5 Modern practice this schema should adopt

It already does three things right — refs-never-values, a version that refuses
newer, and registry-validated references (the census law). What is missing:

1. **Additive-only evolution with real migrations.** Today v1 documents simply
   fail. One `migrate(v, json) -> Slide` with a test per hop means the four
   authored template docs are not stranded by the v3 bump.
2. **An explicit, tested cascade order** — `Look` → preset → slide, one
   `resolve()`, one table-driven test. Undocumented cascades rot.
3. **Provenance** — `created`/`modified` and a `by` field (`operator` |
   `preset:<id>` | `ai:<model>`), which is what makes charter §5's compounding
   claim measurable instead of rhetorical.
4. **One name per concept** — the discipline §3.2/§3.6 apply to `trait` and
   `seed`, applied to the schema too: `motion` must mean one thing.

## 3. Target design

**One subroute. `workbench/editor`. Labelled "Editor". `AUTHOR` is deleted, not
aliased** — no shim (the no-re-export-shim law).

### 3.0 THE FLOW — the operator's spec, 2026-07-31, verbatim

> *"create a new card or edit existing → click add text → then style, either
> click from the 10-15 different presets where we can then allow them to edit
> the specific traits → save. The concept is that once they pick a preset they
> only have 4-7 traits they can edit including color."*
>
> *"The complexity is behind the scenes. You are creating it so we have it
> streamlined."*

That is the whole surface. Five steps, and **nothing else may be added to the
path**:

```
new card ─┐                    ┌── select an ENTITY (the card, or a thing on it)
          ├─→ add <thing> ─────┤
existing ─┘                    └─→ pick its PRESET ─→ tweak its 4-7 TRAITS ─→ save
                                          │             (colour always one)
                                          └── everything else is decided FOR you
```

Every other section of this plan exists to serve this flow. If a section does
not serve it, it is out of scope.

### 3.1 The preset is the complexity collapser — and it is ENFORCED

A preset is already exactly the right shape in this codebase
(`crates/coldlight/tokens/presets.json`): *"a NAMED BUNDLE OF REFS into the
other registries (fonts/moods/motion) — data composed FROM the generated
styling, zero new code per preset."* Five exist (`neon-intro`,
`glacier-stat`, `arcade-flash`, `warm-editorial`, `fuchsia-signal`) — all of
them **card** presets (§2.4). The spec wants **10-15 per entity kind**.

The rule that makes "only 4-7 traits" real rather than a UI convention: **a
preset DECLARES the traits it exposes.** Two new authored fields — `for` (the
entity kind it styles) and `exposes`:

```jsonc
{
  "id": "neon-intro", "label": "Neon intro", "for": "card",
  "moods": ["ember-drift","deep-lagoon"], "motion": "blend-drift",
  "ambience": "pulse", "tone": "magenta",
  "exposes": ["colour", "background", "ambience", "motion"]  // 4-7, colour mandatory
}
{
  "id": "neon-headline", "label": "Neon headline", "for": "text",
  "font": "orbitron", "size": "display", "ink": "strong", "tone": "cyan",
  "exposes": ["colour", "font", "size", "ink"]
}
```

Everything not in `exposes` is resolved by the preset and **cannot be reached
from the overlay at all** — that is "the complexity is behind the scenes," made
structural. Because presets are cross-validated at build time (an unknown ref
already fails the build), this is enforceable the same way:

- a preset exposing fewer than 4 or more than 7 traits **fails the build**;
- a preset that does not expose `colour` **fails the build**;
- a preset exposing a knob **its entity kind does not have** fails the build
  (a `text` preset exposing `ambience` — the stage has ambience, text does not);
- a trait name not in the knob registry **fails the build**.

The knob registry, drawn from what already exists — no new styling machinery.
The right column is what makes the per-scope check above decidable:

| trait (the knob) | what it swaps | source | entities that have it |
|---|---|---|---|
| **colour** *(always exposed)* | the tone | `Tone` | **all** |
| motion | how it arrives | `Slide.enter` / `MOTION_PRESETS` | all |
| font | the typeface | `cl::FONTS` (13) | text · quote |
| size | the fluid step | the `Fluid` ramp | text · quote |
| ink | weight / emphasis | strong · base · muted · subtle · tone | text · quote |
| background | the mood blend | `cl::MOODS` (10) | card |
| ambience | the ambient loop | none · pulse · drift · rings | card |
| drift | how the moods pan | the old `Background.motion` (§8.3 #6) | card |
| fit | cover or contain | `Media.fit` | picture |

Nine knobs across four entity kinds; **4-7 exposed per preset**, and colour is
in every set. A tray shows only the exposed ones, each drawn as itself (§3.6)
— never a dropdown of words.

### 3.2 The word "trait" means two things — resolved

The operator's "traits" = **the 4-7 editable knobs**. ColdLight's `cl::TRAITS`
= the four base looks (`neubrutalist · claymorphic · cyber · retro`) with axes
like `back_glow.spread` — machinery a layman must never see. Two meanings of
one word is the same disease as §3.3's seed collision, so it is settled the
same way, using the charter's own vocabulary rather than an invented one:

| meaning | the word, from here on |
|---|---|
| the 4-7 editable knobs (operator's word, drawn on screen) | **traits** |
| `neubrutalist`/`claymorphic`/`cyber`/`retro` (internal, in the Effects bucket) | **treatments** — already the word in charter §1 and `docs-history-2026-10-12:docs/plans/pass0-coldlight-treatments.md` |

`cl::TRAITS` → `cl::TREATMENTS`. Generated-code rename in `coldlight/build.rs`
plus its call sites; no token file changes shape.

### 3.3 The front door — "new **or existing**"

The flow's left edge. Today the surface can only mint new drafts; the
existing-slide path lives only in the wall-of-text picker. Deleting that picker
without carrying the path would silently lose capability. So the Editor's front
door gains an **Existing** row in the filmstrip band: the same
`list_template_docs()` + `all_episodes()` union the picker read, drawn as
**`film_tile` thumbnails** (`author.rs:53` — real document miniatures, already
written) instead of text rows. Tapping one opens the same card overlay on that
document.

That is the whole deletion story: the capability moves, the wall of text dies.

### 3.4 The four buckets — slug and label AGREE

`Shelf` is replaced by `Bucket`, and **the machine id and the drawn label are
the same word**. That kills this entire class of bug permanently:

| `Bucket` | slug | label | hint | catalog source |
|---|---|---|---|---|
| `Templates` | `templates` | Templates | slides | `demos()` + the authored slide docs |
| `Blocks` | `blocks` | Blocks | chunks | `blocks()` |
| `Elements` | `elements` | Elements | atoms | `elements()` |
| `Effects` | `effects` | Effects | looks/recipes | `cl::TREATMENTS` + recipes + motion |

Six shelves become four. `treatments` + `recipes` merge into **Effects** (the
mock's hint). Ladder order, coarse to fine — Templates → Blocks → Elements —
matching `LibSection`'s existing order (`library.rs:52`), so a piece is called
the same thing in the Library and in the Editor. `LibSection` keeps its ten
sections (it is the full raw wall); the claim is that the **shared words mean
the same thing**, not that the two lists are identical.

### 3.5 The overlay: ADD, then STYLE — five tabs collapse to two steps

The flow has no tab rail. It has *add a thing*, then *style it*. The five
shipped tabs (`author.rs:938` — Words · Background · Movement · A quote ·
Picture) split cleanly along that line, and the split is a deletion, not a
rewrite:

| shipped tab | becomes |
|---|---|
| Words | **Add → Text** |
| Picture | **Add → Picture** |
| A quote | **Add → Quote** |
| Background | a **trait** (`background`) — exposed by a preset, or decided by it |
| Movement | two **traits** (`motion`, `ambience`) — same rule |

So the overlay is:

1. **ADD** — three kinds, each a big tile drawn as itself. Text is the
   `dpl_editor::Editor` that already exists (`author.rs:933`), unchanged.
2. **STYLE** — the 10-15 preset tiles, each rendered as its own seed (§3.6).
   Tap one, and **only its exposed traits** appear as a tray of 4-7 rows.
3. **Save card.**

Background and Movement stop being places you navigate to and become knobs a
preset either hands you or decides for you. That is the spec's whole point:
after a preset, there are at most seven things left to touch.

### 3.6 A SEED is the default appearance — the property, not a bucket

**OPERATOR RULING 2026-07-31:** a seed is not a fifth bucket. *Every item in
every bucket has a seed — its default appearance.* That is what the tile
previews and what you get when you tap it.

This is the unifying model the surface was missing. Today each bucket draws
itself by a different mechanism, under a different name:

| bucket | how its default appearance is produced today |
|---|---|
| Templates | `demos()` — registered `SlideTemplate`s render themselves |
| Blocks | `blocks()` rendered live on the Library wall |
| Elements | `elements()` rendered live (the DS2 census) |
| Effects | `cl::compose()` resolves a trait to a `Style` |

Four mechanisms, one idea. They become one trait — the only new abstraction in
this pass, and it earns its place by deleting four bespoke tile paths:

```rust
/// The default appearance of a catalog item: what its tile previews, and
/// what lands on the card when you tap it.
pub(crate) trait Seed {
    fn seed_id(&self) -> &str;
    fn seed_label(&self) -> &str;
    /// Uniform across all four buckets — draw the thing as itself.
    fn seed_preview(&self, ui: &mut egui::Ui, rect: egui::Rect);
    /// Bucket-specific: a Template REPLACES the card's document, a Block or
    /// Element is ADDED to it, an Effect is APPLIED to the selection.
    fn seed_apply(&self, slide: &mut dpl_doc::Slide, at: Option<&str>);
}
```

`seed_preview` is why the buckets stop being a wall of names: a tile shows the
thing, never a label for the thing. `film_tile` (`author.rs:53`) already proves
the pattern for cards — this generalises it to the catalog.

**The word collision, resolved without touching the disk.** `seeds/slides/<class>/<id>/slide.json`
(`doc/src/lib.rs:840-885`, four authored documents there — the episode set at seeds/episodes/ is four more, §3.10.1) currently uses "seed" for
*a starting slide document* — which under this ruling is a **Template**, not a
seed. Leaving both meanings alive is exactly the ambiguity that produced this
mess. The code names are corrected to say what they are:

| today | becomes | note |
|---|---|---|
| `struct Seed` | `struct TemplateDoc` | it is an authored template document |
| `list_seeds()` | `list_template_docs()` | |
| `SEED_CLASSES` | `TEMPLATE_CLASSES` | its own doc comment already says it "restates the registry's `TemplateClass` vocabulary" |
| `seed_tree_issues()` | `template_tree_issues()` | |

The **on-disk `seeds/` tree is NOT moved in this pass** — it is git-tracked
product, doctrine §1.3 names the path, and a filesystem migration bundled into
a naming pass is how things break. One constant `TEMPLATE_DOCS_DIR =
"seeds/slides"` holds the old path, with the folder rename named as a
follow-up. That frees the word "seed" completely, at zero filesystem risk.

### 3.7 The signals — designed, not renamed

**OPERATOR RULING 2026-07-31: pre-release — align them fully, no shims.**

The surface emits **publish-only** facts (anti-loop law: the Editor never
subscribes to `build.*`). Today there is exactly one, and it is mis-named. The
aligned set is **one signal per step of the §3.0 flow** — so the signal stream
*is* the flow, and a coach or agent can wait on any point in it:

```rust
pub enum BuildEvent {
    SlidePlaced      { show, episode, chapter, slide },  // Studio — unchanged
    EpisodeChanged   { show, episode },                  // Studio — unchanged
    CardStarted      { slide },                       // new card, or an existing one opened
    CardAdded       { slide, node, kind },         // text · picture · quote (§3.5 ADD)
    CardStyled      { slide, node, kind, preset }, // a preset was picked    (§3.5 STYLE)
    CardAxisChanged { slide, node, axis },         // one of the 4-7 knobs moved
    CardSaved       { show, episode, slide },      // RENAMED from CardAuthored
}
```

> **Vocabulary, 2026-08-11 (`design-system.md` §0.6).** §2.4's ratified ruling
> — *"everything is an ENTITY"* — stands as the OPERATOR'S concept and its
> language is untouched. The **code identifier** is `Node`: in Rust GUI
> `Entity` means an ECS handle (Bevy), and a node in a retained tree is a
> `Node`. Likewise the knobs are **axes**, not traits — `traits.json` is a
> retired second compositor, and an *axis* is defined as one styling property
> channel carrying a merge rule. Fixed here before any row was written,
> because a signal name is a **wire format** that lands in Parquet.

| flow step (§3.0) | signal | catalog id |
|---|---|---|
| new card / edit existing | `CardStarted` | `build.card-started` |
| add text/picture/quote | `CardAdded` | `build.card-added` |
| pick a preset | `CardStyled` | `build.card-styled` |
| tweak an axis | `CardAxisChanged` | `build.card-axis-changed` |
| save | `CardSaved` | `build.card-saved` |

Since presets are entity-scoped (§2.4), the styling signals carry **which
node** was styled (`node` = the block id, or `"card"` for the stage) and
**which kind** it is. Without those, a coach could see that *something* was
styled but never what — and preset usage per entity kind, the measurable form
of the compounding claim, would be unrecoverable from the ledger.

- `CardStarted` fires where `CardAuthored` fires today (`author.rs:1552`, after
  the draft write — emit-with-mutation) — that is what today's signal *actually*
  means.
- `CardStyled { preset }` is the one that carries the product's whole thesis:
  it names which of the 10-15 presets collapsed the complexity. It is also what
  makes preset usage measurable.
- `CardAxisChanged { axis }` carries a knob name from the §3.1
  registry only — a bounded, registered vocabulary, never free text.
- Payloads stay **content-free** in the id space, per the existing law at
  `coach_signals.rs:178`: show/episode/slide names are operator content and
  never reach a `SignalId`; `kind`, `preset`, and `axis` are registered
  vocabularies, so they may.

**Blast radius, exact** — 5 files, all listed, no shims (pre-release ruling):
`fabric/src/lib.rs:360-382` (the enum) · `app/src/coach_signals.rs:181-183,
225-227, 474-490` (dispatch, consts, `COARSE`, tests) · `seam/src/lib.rs:264-290,
970, 993, 1256` (wire shapes `cardAuthored` → the five, plus pinned tests) ·
`ledger/src/lib.rs:640-660` (rows) · `hubs/workbench` (the publishers —
`author.rs:5, 1552`; `studio.rs:228, 234` unchanged).

### 3.8 The seams — named for what they prove

A blind rename produces `--editor-editor`. Each flag is renamed to describe the
fact it proves:

| today | becomes | proves |
|---|---|---|
| `--author-editor` | `--card-open` | the card overlay opens on a fresh card |
| `--card-tab <slug>` | `--card-add <kind>` | the ADD step — text · picture · quote (§3.5; tabs are gone, so the flag that named them goes too) |
| — | `--card-preset <id>` | **NEW** — a preset applied, and the tray showing ONLY its 4-7 exposed traits (§3.1). Without this the spec's core claim is unprovable on the exe. |
| `--author-draft <n>` | `--cards <n>` | the filmstrip with n cards, no overlay |
| `--author-container <kind>` | `--container <kind>` | the band labelled by container kind |
| — | `--card-existing` | **NEW** — the overlay opened on an EXISTING slide (§3.1) |
| `--route workbench/author` | `--route workbench/editor` | the surface itself |

`--card-existing` and `--card-preset` are not decoration: the absorbed
existing-slide path (§3.3) and the "only 4-7 traits" claim (§3.1) are the two
things this pass could lose or fake silently, so each gets its own proof seam.

---

## 3.9 AMENDMENT 2026-08-07 — two depths, and how Studio relates

Operator, verbatim: *"we need to realize we want an advance mode so we have the
capability to grow and expand this … the ability to apply and mix and match
presets on each entity and then save … then the full advance editor where we
can mix and match, extend the options, for the default edit in place. Basically
need to have edit existing, and then the full editor … we do not need 3 meaning
the same."*

### 3.9.1 RULED — three surfaces become two, not one

**OPERATOR RULING 2026-08-07: "studio and editor work together but separate."**

| today | after | why |
|---|---|---|
| **Author** (3,591 lines) — make a card | **deleted** into Editor | §3 already ruled this |
| **Editor** (81 lines) — change a card | **the Editor** | one surface, two depths (§3.9.2) |
| **Studio** (251 lines) — sequence cards into episodes | **stays, separate** | arranging is a different verb from making |

Studio is *not* merged. It is also not independent: its blurb today claims
"blocks → elements → templates → effects" — the Editor's vocabulary — while its
code only publishes `SlidePlaced`. **Working together means sharing the system,
not the surface:** one bucket vocabulary (§3.4), one preset registry (§3.1), one
`Seed` trait for every tile (§3.6), one signal stream (§3.7). Studio's blurb is
corrected to describe sequencing, which is what it does.

The test that keeps them honest: **no styling vocabulary may be defined twice.**
If Studio needs to name a bucket, a preset or a trait, it reads the Editor's
registry — a second enumeration fails the census (§6).

### 3.9.2 The two depths

The flow (§3.0) is unchanged and remains the default. It gains a second depth
*behind* it, not a sixth step in it — §5's boundary holds.

| depth | who it is for | what it is |
|---|---|---|
| **Edit in place** *(default)* | every card, every day | §3.0 exactly: select an entity → its preset carousel → its 4-7 traits → save. Nothing else on the path. |
| **Advanced** *(opt-in)* | growing the system | the same surface opened up: mix and match across ALL entities at once, browse by category, and extend the option sets |

Advanced is reached from the Editor, never navigated to as a separate route —
otherwise it is a fourth surface and this amendment defeats itself.

### 3.9.3 The interaction model — carousels, not trays

Operator: *"categories, overlays, carousel experiences, seamless scrolling,
select and it applies."* That is a correction to §3.5's tiles-and-tray drawing:

- **Select applies immediately.** No confirm step, no Apply button. Tapping a
  preset restyles the entity; the card is the preview (§3.6's `seed_preview`
  law, extended from the tile to the whole stage).
- **Carousels, seamlessly scrolled** — the `carousel` already built
  (`author.rs:2208`, #136) is the pattern, one per entity, not a grid that
  paginates.
- **Categories are a browse axis** alongside the four buckets — the matrix
  facets (`coldlight-matrix.md` §§3-4) are the categories, so this reuses the
  classification rather than inventing a taxonomy.
- **Overlays, not routes.** Depth is an overlay over the card; the card stays
  visible and live underneath.

### 3.9.4 The composition IS the product — the number that justifies all of it

Each entity wears its own preset, so a card is the **product** of its entities'
presets, not one choice from a list:

```
12 card × 13 text × 10 picture × 10 quote = 15,600 compositions
                                          = 187,200 with tone
```

from **45 authored presets** (`editor-presets-mock.html`, red-pen pending).
That is where "diverse visuals" comes from — mixing factors, not a longer list.
The mock's composition section demonstrates it live.

### 3.9.5 OPEN — the styling generator inside the system

Operator: *"the code used to generate the styling needs to be in the system."*

Today generation is **build-time only**: `tokens/*.json` → `build.rs` →
generated Rust consts. That is why the pool cannot grow without a recompile, and
it is the reason Advanced cannot yet *create* — only *combine*.

Making presets runtime-authorable is a **document-layer change**, not a UI one:
an authored preset becomes operator data that must still validate against the
registries (the manifest⇄truth law), which means a new document kind, a
validator, and a place on disk. That is larger than this plan.

**⬜ NOT ANSWERED — staging.** Recommendation unless corrected: Advanced ships
as *mix and match everything that exists* (which the 15,600 already makes
worthwhile), and runtime preset **authoring** is its own plan immediately after.
Shipping both together would put a schema change and a surface rewrite in one
pass, which is how the last three passes went wrong.

### 3.9.6 The knob registry is WRONG in both documents — blocks Commit 4

Found 2026-08-07 by generating the composition surface from the real data:

| source | knobs it lists |
|---|---|
| this plan, §3.1 | colour · motion · font · size · ink · background · ambience · drift · **fit** |
| `editor-presets-mock.html` | colour · motion · **treatment** · font · size · ink · background · ambience · drift |
| what the 45 presets **actually use** | **both** — all 10 picture presets expose `fit`; 15 presets expose `treatment` |

It is **10 knobs**, and each document omits a different one. The per-entity
build check of §3.1 is decided by exactly this table, so **Commit 4 cannot be
written until it is corrected.**

Second, unresolved: §3.2 rules that treatments are *"machinery a layman must
never see"* — yet `treatment` is exposed on the operator's tray by 15 of the 45
presets. Both cannot stand.

**⬜ OPERATOR DECISION REQUIRED (2):**
1. Ten knobs, both tables corrected — or drop one?
2. `treatment` on the tray, or resolved behind the preset per §3.2?

## 3.10 AMENDMENT 2026-08-08 — Commit 2, ground-truthed by reading the files

Every file Commit 2 touches was read in full before this was written:
`crates/doc/src/lib.rs` (2,065) · `crates/slides/src/document.rs` (706) ·
`crates/editor/src/lib.rs` (1,035) · all 8 authored `slide.json` · the one
authored `theme.json`. **Three things §2.5 and §4 assert are wrong**, and each
changes the work.

### 3.10.1 There are EIGHT authored documents, not four

§4 item 3b says "the four authored template docs". There are eight:

```
seeds/slides/static/{headline-clean,stat-callout,terminal-retro,title-neon}/slide.json
seeds/episodes/how-ai-works/{headline,title,stat,terminal}/slide.json
seeds/episodes/how-ai-works/theme.json
```

The episode set was missed because `list_template_docs()` only walks
`seeds/slides`. Migrating four and stranding four would leave the seeded
episode — the one `--clone-seed` uses for every proof — on a stale shape.

### 3.10.2 v1 documents do NOT fail today — and every authored doc IS v1

§2.5.1 says *"Today v1 documents simply fail."* They do not. The only version
check is `doc/src/lib.rs:169`:

```rust
if slide.v > SCHEMA_V { return Err(DocError::Version(slide.v)); }
```

Older is **accepted and never migrated**. And all nine authored documents carry
`"v": 1` while `SCHEMA_V` is 2 — the repo has been running a version behind
since W11 and nothing noticed, because additive-only evolution means an old
document is a valid new one.

That is good news twice over: the property §2.5.1 asks for already holds, and
it means **v3 needs no `migrate()` function** — only a test that pins it.

### 3.10.3 The `size` hazard is smaller than stated — serde already does it

§4 item 3b calls `Block::Text.size` required → optional "the single migration
hazard", requiring a migration that "writes the old required `size` through as
an explicit override".

Serde does exactly that for free: with `size: Option<String>`, a document
carrying `"size": "display"` deserialises to `Some("display")` — which **is**
the explicit override. No card changes appearance and no migration code runs.

Both render paths already have the fallback for the absent case —
`document.rs:375` and `:443` are both
`size_of(&fluid, size).unwrap_or_else(|| fluid.fs_body())` — so `None` needs
only `as_deref()`, not a new branch.

The real hazard is elsewhere and unnamed by the plan: **`dpl-editor` writes
`size` unconditionally** (`editor/src/lib.rs:894,912` mint blocks with
`size: "display".into()`, and its own struct holds `size: String` at `:51`).
If the editor keeps stamping a size on every block it touches, every card the
operator opens silently acquires an override, and the preset can never own size
again. That is the migration hazard, and it is a write-path fix, not a
read-path one.

### 3.10.4 What Commit 2 actually is, then

```rust
pub const SCHEMA_V: u32 = 3;

struct Slide {
  v, template, title: Option<String>,
  preset: Option<String>,          // the CARD entity's preset (§2.4)
  tone:   Option<String>,          // its colour — now ANY of the 53 (477d643)
  coordinate: Option<Coordinate>,  // the matrix point, if pinned directly
  background, blocks, enter,
  recipe: Option<String>,          // ratified #4 — reserved, unused
  reveals,
}
Block::Text  { id, preset?, tone?, text, font?, size?, ink? }   // size now optional
Block::Media { id, preset?, tone?, as, src, fit?, poster? }
Block::Quote { id, preset?, tone?, text, attribution, source? } // ratified #3
```

**`resolve()` gains the middle level** (`lib.rs:764` today, two levels):
`Theme` → preset → entity override. It is currently 20 lines and pure; the
third level keeps both properties.

**Material as `:root` (operator, 2026-08-07)** lands here as the inheritance
rule, not a new field: an entity that names no material inherits its parent's,
so most entities declare nothing and the document stays sparse. Without that,
per-entity styling means every block carrying a full coordinate.

### 3.10.5 Work plan

1. `SCHEMA_V` 3 + the new fields, all `Option`, all `skip_serializing_if`.
   A v1/v2 fixture must round-trip byte-identically — the test that proves
   §3.10.2 rather than assuming it.
2. `Block::Quote` + retire the quote convention (`:222-250`) and `quote_lines()`.
3. `size` → `Option<String>`; `as_deref()` at the two render sites; **stop
   `dpl-editor` stamping a default size** (§3.10.3) — the actual hazard.
4. `resolve()` three-level, with one table-driven cascade test.
5. Provenance (`created`/`modified`/`by`) per §2.5.3.
6. Validate the new refs: preset id, tone against the 53-hue pool, recipe.
7. The eight authored documents re-saved at v3 — all eight, §3.10.1.
8. `cargo green`; §8.

### 3.10.6 Risk the plan does not name

`Theme` also carries `v` and is version-checked the same way
(`lib.rs:739`). It gains `preset`/`tone` too, or a show-level preset is
unexpressible and the cascade has a hole at its top. One authored `theme.json`
exists; it is v1.

## 4. Changes, in execution order

**Commit 1 — the signals (§3.7).**

1. **`crates/fabric/src/lib.rs:360-382`** — `BuildEvent`: `CardAuthored` →
   `CardSaved`; add `CardStarted`, `CardAdded { slide, kind }`,
   `CardStyled { slide, preset }`, `CardAxisChanged { slide, axis }`.
   Doc comment rewritten (it says "authoring surface" today).
2. **`crates/app/src/coach_signals.rs`** — dispatch (`:181`), the five consts
   (`:225`), `COARSE`, and the tests at `:474`.
3. **`crates/seam/src/lib.rs`** (`:264` wire shapes, pinned tests `:970/:993/:1256`)
   + **`crates/ledger/src/lib.rs:640`** — rows and shapes follow.

**Commit 2 — `SCHEMA_V` 3: the composition can express the flow (§2.1-2.4).**

3a. **`crates/doc/src/lib.rs`** — `SCHEMA_V` 2→3. `Slide` gains `title`,
    `preset`, `tone`, `recipe` (reserved). `Block::Text` gains `preset`,
    `tone`, and `size` goes required → optional. `Block::Media` gains
    `preset`, `tone`. **`Block::Quote{id,preset?,tone?,text,attribution,source?}`
    is new** — the quote convention (`:222-250`) is retired, and
    `quote_lines()` with it.
3b. **The v2→v3 round-trip + its test.** ⚠️ CORRECTED by §3.10.2/§3.10.3 —
    read those before implementing this item; the original text below was
    written from assumption and is wrong in two ways.
    - **No `migrate()` is needed.** Older documents already load (`lib.rs:169`
      refuses only `v > SCHEMA_V`), and serde turns a present `"size"` into
      `Some(..)` — which IS the explicit override — by itself. What this item
      ships is the TEST that pins that, not a migration.
    - **There are EIGHT authored documents, not four** — four in
      `seeds/slides/static/`, four in `seeds/episodes/how-ai-works/`, plus one
      `theme.json`. All nine are `"v": 1`. Migrating four strands the seeded
      episode every proof run uses.
    - The real hazard is on the WRITE path: `dpl-editor` stamps `size`
      unconditionally (`editor/src/lib.rs:894,912`). Fix that here or every
      opened card silently acquires an override.
3c. **`resolve()` gains the middle level** — `Theme` → preset → entity
    override — with one table-driven cascade test (§2.5.2). This is also where
    **material-as-`:root`** lands (§3.10.4): an inheritance rule, not a field.
3d. **Provenance** — `created`/`modified`/`by` (§2.5.3).

**Commit 3 — the vocabulary, no behaviour change.**

4. **`crates/doc/src/lib.rs:840-885`** — `Seed` → `TemplateDoc`, `list_seeds()`
   → `list_template_docs()`, `SEED_CLASSES` → `TEMPLATE_CLASSES`,
   `seed_tree_issues()` → `template_tree_issues()`, `TEMPLATE_DOCS_DIR =
   "seeds/slides"`. **No files move on disk.**
5. **`crates/coldlight`** — `cl::TRAITS` → `cl::TREATMENTS` (§3.2), in
   `build.rs`'s generator and every call site. Token files unchanged in shape.

**Commit 4 — the presets (§3.1), the flow's engine.**

6. **`crates/coldlight/tokens/presets.json`** — add the authored `exposes`
   array to each look preset; grow 5 → **10-15** (data only, zero new code per
   preset, per the file's own contract).
7. **`crates/coldlight/build.rs`** — cross-validate `exposes` at build time
   alongside the existing unknown-ref check: **4 ≤ len ≤ 7**, `colour` always
   present, every name in the §3.1 knob registry. A violation fails the build.

**Commit 5 — the Editor surface (§§3.3-3.6).**

8. **`git mv crates/hubs/workbench/src/author.rs editor.rs`** —
   `AuthorSurface` → `EditorSurface`; `Shelf` → `Bucket` (six → four, labels ==
   slugs); pilot ids `workbench.author.*` → `workbench.editor.*`; publish the
   five signals at the existing `dpl_doc::save` call sites.
9. **The ADD/STYLE restructure (§3.5)** — the five-tab rail becomes ADD (text ·
   picture · quote) then STYLE (preset tiles → the exposed-trait tray).
   Background and Movement stop being tabs and become knobs. The `dpl_editor::Editor`
   behind Text is untouched.
10. **The `Seed` trait (§3.6)** — one impl per bucket, replacing four bespoke
    tile paths; every tile becomes `seed_preview`, every tap `seed_apply`.
11. **`crates/hubs/workbench/src/lib.rs`** — delete `pub const AUTHOR`; delete
    `editor_surface()` (`:750-830`) and the `editor:` field (`:106`); `EDITOR`
    dispatches to `EditorSurface`; blurb rewritten; the four `open_author_*`
    methods renamed per §3.8.
12. **The existing-slide path (§3.3)** — `list_template_docs()` +
    `all_episodes()` as `film_tile` thumbnails; tap → overlay on that document.
13. **`crates/app/src/main.rs`** — flags per §3.8, incl. `--card-existing` and
    `--card-preset`.

**Commit 6 — the record.**

14. **`CLAUDE.md`** harness-flag list + **`docs/design/architecture.md`** + the
    `structure_audit()` table in `crates/xtask/src/main.rs` — same commit
    (the same-commit law).
15. **Docs** — `docs-history-2026-10-12:docs/STATE.md` corrected; every `author-*.md` plan stamped
    `SUPERSEDED by workbench-editor.md` (never deleted — the history is why
    this happened); doctrine §1.3 gets the §3.6 note that the `seeds/` folder
    rename is a named follow-up.
16. `cargo green`; exe proofs (§6); PR; **double-green** before merge.

## 5. Boundaries

🚫 **No new crate.** 🚫 The **Interactions** and **Ask AI** steps, the prompt
composer, the entity-first AI law, and the library-count readout (charter
§§3–5) are **NOT in this pass** — named in §8.2 so the gap stays visible.
🚫 **Nothing may be added to the §3.0 path.** Five steps. A sixth step, an
extra rail, a "just one more tab" — all out, by construction.
🚫 `crates/editor` (`dpl-editor`) is not touched — it is the engine behind ADD →
Text.
🚫 **Studio is not merged** (§3.9.1 amendment) — it stays a separate surface
and shares the vocabulary, presets, Seed trait and signal stream. Only its
blurb is corrected, to describe sequencing.
🚫 wiredFRONT's editor (`wiredfront/editor`, the After-Effects video editor) is
a **different product surface** and is not touched, renamed, or merged. Mixing
those two is the error called out on 2026-07-31 01:54.
🚫 No `#[allow]`. 🚫 `/content` untouched.

## 6. Verification

- `cargo green` exits 0 — fmt, clippy, tests, structure phase (which now
  audits the renamed module), release build.
- **THE SPEC TEST (§3.1) — build-time, not a convention.** Every look preset
  exposes **4 ≤ n ≤ 7** traits, one of which is always `colour`, every name in
  the knob registry. A preset that violates any of the three **fails the
  build**. This is the one test that makes "the complexity is behind the
  scenes" a property of the system rather than a promise.
- **The bounded-tray test:** after `seed_apply` of a preset, the trait tray
  offers exactly that preset's `exposes` set — no path reaches an unexposed
  knob. Asserted on the state, so a stray control cannot regress it.
- **Unit:** picking a preset publishes `CardStyled { preset }`; moving a knob
  publishes `CardAxisChanged { axis }` with a registry name;
  `Bucket::ALL` asserts `slug() == label().to_lowercase()` for all four (the
  inversion can never come back); the census test still pins every bucket to a
  live registry.
- **The seed test (§3.6):** every item in every bucket answers `seed_preview`
  and `seed_apply` — a bucket item with no default appearance fails the build.
  Plus: `seed_apply` on a Template replaces the document, on a Block/Element
  adds to it, on an Effect mutates only the selection.
- **The absorption test:** opening a template doc from the existing-row and
  mutating it writes back to exactly that colocated `slide.json` — proving the
  deleted picker's capability survived.
- **Real exe, sequential runs, at 1280×800 AND 1100×700:**
  `--route workbench/editor` (one Editor row in the nav, no Author, no wall of
  text) · `--cards 3` (filmstrip) · `--card-open` · `--card-add text` ·
  **`--card-preset <id>` — the money shot: the tray showing 4-7 knobs and
  nothing else** · `--card-existing` · `--container guide`.
- **Nav proof:** a shot of the Workbench nav showing **Library · Editor ·
  Episodes · Studio · Test Lab** — five rows, no "Author".
- **Flow proof:** one `DPL_LEDGER=1` run of the whole §3.0 path capturing
  `build.card-started` → `card-added` → `card-styled` → `card-axis-changed` →
  `card-saved`, in order. The ledger transcript *is* the spec, replayed.

## 7. Commit plan

One branch `feat/workbench-editor`, off `main`, six commits — each one green
on its own:

1. `refactor(fabric): card signals mirror the authoring flow`
2. `feat(doc): SCHEMA_V 3 — presets by reference, quote as a block, colour per entity`
3. `refactor(vocab): a template document is not a seed; traits are not treatments`
4. `feat(coldlight): a preset declares the entity it styles and the 4-7 traits it exposes`
5. `feat(workbench): the Editor — add, style, save`
6. `docs(editor): supersede the author-* lane; STATE + architecture + audit`

Commit 2 is the load-bearing one: until the schema can hold a preset ref, a
per-entity tone, and a real quote, the flow cannot be built at all.

`gh pr list` first (parallel-session law). PR → `gh pr checks --watch` →
**merge only on double-green** → rebuild `target/release/devpulselabs.exe`
HERE + verify mtime → update `docs-history-2026-10-12:docs/STATE.md`.

## 8. Evidence

To be filled on execution: the six exe shots at both sizes, the nav shot, the
ledger capture, `cargo green` tail, both CI lane ids, and the exe mtime.

### 8.1 Ratification

1. ✅ **RATIFIED 2026-07-31 — a seed is the default appearance of every item in
   every bucket, not a fifth bucket.** Four buckets: Templates · Blocks ·
   Elements · Effects. §3.6 follows, including the rename that frees the word
   (`Seed` → `TemplateDoc`) with the on-disk tree left where it is.
2. ✅ **RATIFIED 2026-07-31 — pre-release: align the signals fully, no shims.**
   §3.7 — one signal per step of the flow, five files, listed.
3. ✅ **RATIFIED 2026-07-31 — THE FLOW (§3.0), typed out once, verbatim.** New
   or existing card → add → preset → 4-7 traits including colour → save. The
   complexity is behind the scenes. §§3.1-3.5 are the design that serves it,
   and §6's build-time preset test is what keeps it honest.

**One thing I will bring back rather than invent:** the flow needs **10-15**
look presets and five exist. The five new-to-ten are *data* (`presets.json` —
font + two moods + motion + ambience + `exposes`), so I will propose the set
composed from the registered fonts and moods and you red-pen the names and
looks. I am not silently authoring taste.

### 8.3 RATIFIED 2026-07-31 — the schema decisions (§2.1-2.3)

| # | decision | ruling |
|---|---|---|
| 1 | preset: ref+overrides vs bake | ✅ **NOT BAKE** — ref + sparse overrides, `SCHEMA_V` 2→3 |
| 2 | colour: per card or per block | ✅ **PER ENTITY** — operator correction; see §2.4 |
| 3 | quote as a real block kind | ✅ **YES** — `Block::Quote{id,text,attribution,source?}` |
| 4 | `recipe` field now, unused | ✅ **YES** — one field today instead of a second bump later |
| 5 | authored card title | ✅ **YES** — `title: Option<String>` |
| 6 | `motion` means two things | ⬜ not answered — taking the recommendation: the **knob** is `Slide.enter`; the background's field becomes `drift`. Say so if you want it the other way. |

### 8.4 The preset sets I will bring back, not invent

Per §2.4 the 10-15 is **per entity kind**, so there are four sets to author
(card · text · picture · quote). All of it is data — refs into the registered
fonts, moods, motion, and tones, plus each preset's `exposes` list. I will
propose all four sets in one table for you to red-pen before any of it is
written. I am not silently authoring taste.

### 8.2 Named follow-ups (NOT this pass — so they stop being invisible)

- **Interactions** and **Ask AI** tabs, the prompt composer, the entity-first
  AI law, the library-count readout — charter §§3–5, never built.
- The `seeds/` → `templates/` **folder** rename on disk (§3.3).
