# Pass — aligning the authoring surface

Five tabs and three censuses were built in a day, each correct on its own and
each slightly different from the others. This pass makes them one shape.

**Reference:** R1 — `docs/ops/references/authoring-mock-a3db4aee.html`.
**POA&M:** `docs/plans/authoring-lane-poam.md`.
**Operator direction (2026-07-30):** *"I do not care if I have to rewrite code
to align and have consistent patterns."*

## 1. Context — why this pass exists

The lane moved fast and the patterns drifted while it did. Nothing here is
broken; everything here is **inconsistent**, which is worse over time — the
next person cannot learn the surface once and apply it everywhere, and every
new tab has to pick which of three precedents to copy.

This is not tidying. Two of the four findings below are the kind of drift that
produced real defects earlier in this lane: a rule written in one module and
not followed in the next (the per-frame index open), and a vocabulary declared
twice (the reason `enters()` and `quote_lines()` exist at all).

## 2. Current state (ground-truthed in-session, `ef65438`)

Four concrete inconsistencies, all introduced by this lane.

### 2.1 `pretty()` exists twice

```
crates/hubs/workbench/src/containers.rs:27  pub(crate) fn pretty(id: &str) -> String
crates/hubs/workbench/src/backgrounds.rs:30 fn pretty(id: &str) -> String
```

Identical bodies. `author.rs` already imports the `containers.rs` one;
`backgrounds.rs` grew its own instead. This is exactly the duplication the
lane fixed twice at the document layer (`enters()`, `quote_lines()`) and then
reintroduced one layer up.

### 2.2 Two different `Kind` enums

```
crates/hubs/workbench/src/containers.rs:40  pub(crate) enum Kind   // lesson · guide · episode · deck
crates/hubs/workbench/src/pictures.rs:35    pub(crate) enum Kind   // image · video
```

Both `pub(crate)`, both in one crate, both called `Kind`, meaning entirely
unrelated things. `author.rs` refers to them as `crate::pictures::Kind` and
(via import) `Kind` — a reader has to know which is which from context.

### 2.3 "A thing you can pick" has three shapes

| module | type | fields |
|---|---|---|
| `backgrounds.rs` | `Choice` | `slug`, `label` |
| `containers.rs` | `Container` | `id`, `kind`, `name`, `count`, `dir` |
| `pictures.rs` | `Picture` | `rel_path`, `show`, `name` |

Three names for the machine id (`slug` / `id` / `rel_path`) and two for the
drawn word (`label` / `name`). The lane's own law is `slug()` = machine id,
`label()` = what is drawn — and the data types do not follow it.

### 2.4 Tab functions take different things

```rust
fn background_tab(ui: &mut Ui, editor: &mut dpl_editor::Editor)
fn movement_tab  (ui: &mut Ui, editor: &mut dpl_editor::Editor)
fn quote_tab     (ui: &mut Ui, ed: &mut CardEditor)
fn picture_tab   (ui: &mut Ui, ed: &mut CardEditor)
```

Split by accident, not design: the first two predate `CardEditor` holding
per-tab state. A sixth tab has two precedents to choose between, and the
choice is invisible from the call site.

**Also noted, and deliberately NOT changed here:** the census functions return
`Vec<Choice>`, `Vec<Container>` and a `Shelf` struct. That difference is
*real* — `Shelf` carries `more` and `unavailable`, which the others have no
concept of. Aligning it would mean inventing state the other two do not have.
§4 says so rather than forcing symmetry.

## 3. Design — one vocabulary, one shape, one signature

**One `pretty()`.** It moves to a `words.rs` module beside the censuses that
use it, since it belongs to none of them in particular. `containers.rs` and
`backgrounds.rs` both import it.

**`Kind` becomes two named things.** `containers::Kind` → `ContainerKind`;
`pictures::Kind` → `PictureKind`. Neither is "the" kind, so neither keeps the
bare name. Both stay in their own modules; only the type name changes.

**One pickable shape, following the lane's own law.** Every "thing you can
pick" carries:

```rust
slug:  String   // the machine id — the ref written into the document
label: String   // what the operator reads
```

`Container` keeps `count` and `dir`; `Picture` keeps `show`. Those are real,
type-specific facts. What changes is that the *shared* two fields are named
the same everywhere, so `x.slug` and `x.label` mean the same thing in all
three.

**One tab signature.** All four take `&mut CardEditor`. Background and
Movement reach the editor through `ed.editor`, exactly as Quote and Picture
already do. A sixth tab now has one precedent.

## 4. Scope

**In:** the four alignments above, their call sites, and the tests that name
the changed types.

**Out, and named:**
- **Forcing the census return types to match.** `Shelf`'s `more` and
  `unavailable` are real state the other two lack. Symmetry for its own sake
  would mean inventing fields.
- **Touching document or editor APIs.** `Editor::set_*` is already one shape
  and one mutation path; nothing there drifted.
- **The remaining tabs.** On tap and Ask for help are blocked for reasons the
  POA&M states; this pass does not unblock them.
- **Renaming anything the operator reads.** This is entirely internal
  vocabulary. Not one drawn string changes, and the `no_*_speaks_engineering`
  tests should stay green untouched — which is itself the proof.

## 5. Work plan (atomic, in order — commit at each step that compiles)

1. `words.rs` with the single `pretty()`; both censuses import it; delete both
   copies.
2. `Kind` → `ContainerKind` / `PictureKind`, with call sites and tests.
3. `Choice`/`Container`/`Picture` converge on `slug` + `label`.
4. `background_tab` and `movement_tab` take `&mut CardEditor`.
5. `cargo green`, proof shots re-taken at both sizes to prove nothing moved on
   screen, §8, POA&M row, `docs/STATE.md`.

## 6. Risks

- **A rename that changes behaviour.** Mitigation: every step is
  mechanical, `cargo green` runs between them, and the drawn-string tests are
  the tripwire — if a `no_*_speaks_engineering` test fails, something reached
  the screen that should not have.
- **Proof shots differing.** They must NOT differ. Re-taking them at both
  sizes and comparing against the committed ones is the acceptance test for
  "internal only".
- **Churn against other lanes.** This touches one crate the authoring lane
  owns. Other lanes are halted; `gh pr list` before starting anyway.

## 7. Verification / acceptance

- `cargo green` clean.
- The full workbench suite passes **unchanged in count** — no test is deleted
  or weakened, only renamed where it names a renamed type.
- `no_chip_speaks_engineering`, `no_piece_speaks_engineering`,
  `no_kind_speaks_engineering`, `no_option_speaks_engineering` all still pass
  untouched.
- Real-exe proof at 1280×800 and 1100×700 for a tab of each former shape
  (Background, Picture) showing **no visible change**.

## 8. Evidence

### 8.1 The gate

`cargo green ✔`. **38 tests, up from 37** — nothing deleted or weakened; the
one addition is `an_id_becomes_a_sentence_not_a_slug`, covering the now-single
`pretty()` which previously had no test in either copy.

All four drawn-string tests pass **untouched**:
`no_chip_speaks_engineering` · `no_piece_speaks_engineering` ·
`no_kind_speaks_engineering` · `no_option_speaks_engineering`. That is the
tripwire for "internal only" — a rename that reached the screen would trip
one of them.

### 8.2 What changed

| # | Before | After |
|---|---|---|
| 1 | `pretty()` in `containers.rs` AND `backgrounds.rs` | one `words.rs`, imported by both and by `author.rs` |
| 2 | two `pub(crate) enum Kind` meaning different things | `ContainerKind` · `PictureKind` |
| 3 | `slug`/`label` vs `id`/`name` vs `rel_path`/`name` | every pickable thing carries `slug` + `label` |
| 4 | two tab signatures | all four take `&mut CardEditor` |

A fifth fell out of #4: the test modules had two different ways to build a
`CardEditor`. There is now one `card_editor()`, shared — a test can no longer
construct the surface's state differently from the surface.

### 8.3 The acceptance test: nothing moved

`docs/proof/author-alignment-1280x800.png` against the committed
`author-tab-background-1280x800.png`: same rail, same "The colour — 10 to
choose from", same tile grid and selection ring, same live pane, same footer.

A byte comparison is **not** the right instrument here and was tried first: the
app picks a random backdrop mood per boot, so pixels differ legitimately
(`violet-static` versus `deep-lagoon` in the footer). The comparison is
structural, and that is stated rather than glossed.

### 8.4 Findings

**The duplication had a pattern.** `pretty()` was copied one layer above the
exact place the lane had already paid twice to stop copying vocabulary
(`dpl_doc::enters`, `dpl_doc::quote_lines`). The lesson generalises: a helper
used by two censuses belongs beside the censuses, not inside one of them.

**Renaming `Kind` was the riskiest step and a blunt regex nearly broke it** —
`dpl_index::Record.kind` is *their* field, not ours, and a global rename
touched it. Caught immediately by the compiler; worth noting because the same
trap waits for anyone renaming a common word across a module that also
consumes a foreign struct.

**Nothing in the document or editor layers drifted.** `Editor::set_*` was
already one shape through one `mutate`. The drift was entirely in the surface,
which is where five things got built in a day.
