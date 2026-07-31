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
new card ─┐
          ├─→ add <thing> ─→ pick a PRESET (10-15) ─→ tweak 4-7 TRAITS ─→ save
existing ─┘                        │                    (colour always one)
                                   └── everything else is decided FOR you
```

Every other section of this plan exists to serve this flow. If a section does
not serve it, it is out of scope.

### 3.1 The preset is the complexity collapser — and it is ENFORCED

A preset is already exactly the right shape in this codebase
(`crates/coldlight/tokens/presets.json`): *"a NAMED BUNDLE OF REFS into the
other registries (fonts/moods/motion) — data composed FROM the generated
styling, zero new code per preset."* Five exist (`neon-intro`,
`glacier-stat`, `arcade-flash`, `warm-editorial`, `fuchsia-signal`); the spec
wants **10-15**.

The rule that makes "only 4-7 traits" real rather than a UI convention: **a
preset DECLARES the traits it exposes.** One new authored field:

```jsonc
{
  "id": "neon-intro", "label": "Neon intro",
  "font": "orbitron", "moods": ["ember-drift","deep-lagoon"],
  "motion": "blend-drift", "ambience": "pulse",
  "exposes": ["colour", "font", "size", "ambience"]   // ← 4-7, colour mandatory
}
```

Everything not in `exposes` is resolved by the preset and **cannot be reached
from the card overlay at all** — that is "the complexity is behind the scenes,"
made structural. Because presets are cross-validated at build time (an unknown
ref already fails the build), this is enforceable the same way:

- a preset exposing fewer than 4 or more than 7 traits **fails the build**;
- a preset that does not expose `colour` **fails the build**;
- a trait name not in the knob registry **fails the build**.

The knob set, drawn from what already exists — no new styling machinery:

| trait (the knob) | what it swaps | source |
|---|---|---|
| **colour** *(always exposed)* | the tone | `Tone` / `cl::MOODS` |
| font | the typeface | `cl::FONTS` (13) |
| size | the fluid step | the `Fluid` ramp |
| ink | text weight/emphasis | strong · base · muted · subtle · tone |
| background | the mood blend | `cl::MOODS` (10) |
| motion | how it arrives | `MOTION_PRESETS` |
| ambience | the ambient loop | none · pulse · drift · rings |

Seven knobs total, 4-7 exposed per preset. A tray shows only the exposed ones,
each drawn as itself (§3.3) — never a dropdown of words.

### 3.2 The word "trait" means two things — resolved

The operator's "traits" = **the 4-7 editable knobs**. ColdLight's `cl::TRAITS`
= the four base looks (`neubrutalist · claymorphic · cyber · retro`) with axes
like `back_glow.spread` — machinery a layman must never see. Two meanings of
one word is the same disease as §3.3's seed collision, so it is settled the
same way, using the charter's own vocabulary rather than an invented one:

| meaning | the word, from here on |
|---|---|
| the 4-7 editable knobs (operator's word, drawn on screen) | **traits** |
| `neubrutalist`/`claymorphic`/`cyber`/`retro` (internal, in the Effects bucket) | **treatments** — already the word in charter §1 and `pass0-coldlight-treatments.md` |

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
(`doc/src/lib.rs:840-885`, four authored documents) currently uses "seed" for
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
    CardStarted      { slide },                  // new card, or an existing one opened
    CardAdded        { slide, kind },            // text · picture · quote  (§3.5 ADD)
    CardStyled       { slide, preset },          // a preset was picked     (§3.5 STYLE)
    CardTraitChanged { slide, trait_name },      // one of the 4-7 knobs moved
    CardSaved        { show, episode, slide },   // RENAMED from CardAuthored
}
```

| flow step (§3.0) | signal | catalog id |
|---|---|---|
| new card / edit existing | `CardStarted` | `build.card-started` |
| add text/picture/quote | `CardAdded` | `build.card-added` |
| pick a preset | `CardStyled` | `build.card-styled` |
| tweak a trait | `CardTraitChanged` | `build.card-trait-changed` |
| save | `CardSaved` | `build.card-saved` |

- `CardStarted` fires where `CardAuthored` fires today (`author.rs:1552`, after
  the draft write — emit-with-mutation) — that is what today's signal *actually*
  means.
- `CardStyled { preset }` is the one that carries the product's whole thesis:
  it names which of the 10-15 presets collapsed the complexity. It is also what
  makes preset usage measurable.
- `CardTraitChanged { trait_name }` carries a knob name from the §3.1
  registry only — a bounded, registered vocabulary, never free text.
- Payloads stay **content-free** in the id space, per the existing law at
  `coach_signals.rs:178`: show/episode/slide names are operator content and
  never reach a `SignalId`; `kind`, `preset`, and `trait_name` are registered
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

## 4. Changes, in execution order

**Commit 1 — the signals (§3.7).**

1. **`crates/fabric/src/lib.rs:360-382`** — `BuildEvent`: `CardAuthored` →
   `CardSaved`; add `CardStarted`, `CardAdded { slide, kind }`,
   `CardStyled { slide, preset }`, `CardTraitChanged { slide, trait_name }`.
   Doc comment rewritten (it says "authoring surface" today).
2. **`crates/app/src/coach_signals.rs`** — dispatch (`:181`), the five consts
   (`:225`), `COARSE`, and the tests at `:474`.
3. **`crates/seam/src/lib.rs`** (`:264` wire shapes, pinned tests `:970/:993/:1256`)
   + **`crates/ledger/src/lib.rs:640`** — rows and shapes follow.

**Commit 2 — the vocabulary, no behaviour change.**

4. **`crates/doc/src/lib.rs:840-885`** — `Seed` → `TemplateDoc`, `list_seeds()`
   → `list_template_docs()`, `SEED_CLASSES` → `TEMPLATE_CLASSES`,
   `seed_tree_issues()` → `template_tree_issues()`, `TEMPLATE_DOCS_DIR =
   "seeds/slides"`. **No files move on disk.**
5. **`crates/coldlight`** — `cl::TRAITS` → `cl::TREATMENTS` (§3.2), in
   `build.rs`'s generator and every call site. Token files unchanged in shape.

**Commit 3 — the presets (§3.1), the flow's engine.**

6. **`crates/coldlight/tokens/presets.json`** — add the authored `exposes`
   array to each look preset; grow 5 → **10-15** (data only, zero new code per
   preset, per the file's own contract).
7. **`crates/coldlight/build.rs`** — cross-validate `exposes` at build time
   alongside the existing unknown-ref check: **4 ≤ len ≤ 7**, `colour` always
   present, every name in the §3.1 knob registry. A violation fails the build.

**Commit 4 — the Editor surface (§§3.3-3.6).**

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

**Commit 5 — the record.**

14. **`CLAUDE.md`** harness-flag list + **`docs/design/architecture.md`** + the
    `structure_audit()` table in `crates/xtask/src/main.rs` — same commit
    (the same-commit law).
15. **Docs** — `docs/STATE.md` corrected; every `author-*.md` plan stamped
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
  publishes `CardTraitChanged { trait_name }` with a registry name;
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
  `build.card-started` → `card-added` → `card-styled` → `card-trait-changed` →
  `card-saved`, in order. The ledger transcript *is* the spec, replayed.

## 7. Commit plan

One branch `feat/workbench-editor`, off `main`, five commits — each one green
on its own:

1. `refactor(fabric): card signals mirror the authoring flow` — §4 steps 1–3.
2. `refactor(vocab): a template document is not a seed; traits are not treatments`
   — §4 steps 4–5.
3. `feat(coldlight): a preset declares the 4-7 traits it exposes` — §4 steps 6–7.
4. `feat(workbench): the Editor — add, style, save` — §4 steps 8–13.
5. `docs(editor): supersede the author-* lane; STATE + architecture + audit` —
   §4 steps 14–15.

`gh pr list` first (parallel-session law). PR → `gh pr checks --watch` →
**merge only on double-green** → rebuild `target/release/devpulselabs.exe`
HERE + verify mtime → update `docs/STATE.md`.

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

### 8.2 Named follow-ups (NOT this pass — so they stop being invisible)

- **Interactions** and **Ask AI** tabs, the prompt composer, the entity-first
  AI law, the library-count readout — charter §§3–5, never built.
- The `seeds/` → `templates/` **folder** rename on disk (§3.3).
