# DS3 — the ingredient ladder: `elements/` earns its name, `blocks/` becomes the chunk tier

Status: PLANNED — awaiting operator approval. Follows DS2 (element library,
`docs/plans/ds2-element-library.md`) in the design-system build order.
Ratified inputs (operator, 2026-07-28, authoring-mock red-pen session):

> Q1 — "Rename folder → `elements/`, open new `blocks/`."
> Q2 — "Split into two buckets: Looks + Interactions."
> "…aligned with the overlay concepts we use blocks elements and templates
> for the user facing UI while keeping it aligned platform industry code and
> folder structure."

Reference surface: the Author mock (artifact `a3db4aee`), whose bucket row is
`Elements · Blocks · Looks · Interactions · Templates`.

## 1. Context — why this pass exists

DS2 brought the element tier under the Census law but deliberately deferred the
folder question (§3e chose reading **(A)**, "category-as-data, files stay
flat"). That deferral is what produced today's collision:

- `crates/slides/src/blocks/` contains **six atoms**, and DS2 typed them
  `ElementId · ElementSpec · ElementCategory · ElementDemo · elements()`.
  **The types say element; the folder says block.**
- `library.rs::blocks_wall` renders those atoms under a header that literally
  reads `ELEMENTS — the standalone primitives templates compose from`, while
  the enum variant driving it is `LibSection::Blocks`.
- The **chunk tier** — the thing a layman means by "block", a few elements
  composed into one reusable piece — **has no home anywhere in the tree**.

So the user-facing vocabulary the operator ratified (`elements → blocks →
templates`) is a three-rung ladder, and the code currently has rung 1 wearing
rung 2's folder name and rung 2 missing entirely. Every future reader pays that
tax, and the Author surface (Pass B) cannot honestly render a Blocks bucket
that maps to nothing.

This pass makes the folder structure equal the ladder. It does **not** invent
chunk-tier content — see §4.

## 2. Current state (ground-truthed in-session, `7a16b25`)

- **`crates/slides/src/blocks/mod.rs`** — 6 element modules (`badge · bar ·
  divider · eyebrow · media · stat`) + `census`; re-exports
  `census::{elements, CostTier, ElementCategory, ElementDemo, ElementId,
  ElementSpec}` and the six free fns. Module doc opens "Blocks — atomic,
  insertable units."
- **`crates/slides/src/lib.rs`** — `pub mod blocks;` (l.26) and the tier
  doctrine in the crate doc names "**blocks** — atomic, insertable units".
- **Call sites: 26 files, 34 references**, all mechanical `use` paths:
  - 19 `templates/*/mod.rs` (`eyebrow::tracked` ×19, `media_backdrop`/`Media`/
    `Ambient` ×5, `bar::bar` ×2, `divider::rule` ×2)
  - 4 under `infographics/` (`stage.rs`, `stack_layers`, `probability_bars`,
    `donut_share`)
  - `crates/slides/src/document.rs` (`ambient_backdrop`,
    `media_block_backdrop`, `Ambient`)
  - `crates/hubs/workbench/src/library.rs` (`{elements, ElementCategory}`)
  - `crates/hubs/home/src/lib.rs` (`eyebrow::tracked`)
- **`library.rs`** — `LibSection` is an 8-variant enum: `Seeds · Templates ·
  Blocks · Traits · Colors · Presets · Fonts · Moods`. `blocks_wall` (l.376) is
  census-driven and already correct in substance; only its NAME and the enum
  variant feeding it are wrong. `lib.rs` l.8 still carries the stale note
  "Elements sections join later; the chip row leaves room."
- **The structure audit is crate-level only.** `xtask::structure_audit` checks
  dep ceilings, the egui-free census, lateral hub deps, the FFI leaf, manifest
  hygiene, orphan crate folders, harness flags — **no module paths**.
  `docs/design/architecture.md` names `dpl-slides` once, as a SURFACE crate.
  **A module rename inside the crate needs no amendment to either.** (Verified
  by reading `CEILINGS` / `EGUI_FREE` at `crates/xtask/src/main.rs:627-660`.)
- **Promotion rule is live law** (`lib.rs` crate doc + `blocks/mod.rs`):
  "a pattern becomes a block only after it recurs in ≥ 2 templates."
- Working tree: on `feat/session-os`, dirty with untracked `.claude/`. NOTE:
  `git branch -a` fails with `fatal: bad object refs/remotes/origin/feat/session-os`
  — a broken remote ref that must be pruned before branching (§5 step 0).

## 3. Design — three rungs, three folders, one census each

```
crates/slides/src/
  elements/          rung 1 — atoms. 6 modules + census.rs   (was blocks/)
  blocks/            rung 2 — chunks. mod.rs + census.rs     (NEW, empty)
  templates/         rung 3 — whole cards                    (unchanged)
  infographics/      rung 3, charts — registered as templates (unchanged)
```

### 3a. The move

`git mv crates/slides/src/blocks crates/slides/src/elements`, then rewrite
`blocks::` → `elements::` at all 26 call sites and `pub mod blocks;` →
`pub mod elements;` in `lib.rs`. **Hard cutover, one commit — no re-export
shim** (CLAUDE.md: "no re-export shims on moves"). Types are already correctly
named, so only paths change; no type renames anywhere.

### 3b. The new chunk tier, honestly empty

`crates/slides/src/blocks/` is created fresh with `mod.rs` + `census.rs`,
mirroring the element census shape:

```rust
pub struct BlockId(pub &'static str);
pub struct BlockSpec { id, label, made_of: &'static [ElementId], cost_tier, blurb }
pub trait BlockDemo { fn spec(&self) -> BlockSpec; fn paint(&self, ..); }
pub fn blocks() -> Vec<Box<dyn BlockDemo>>   // returns EMPTY, by design
```

**It ships with zero registered blocks, and that is the correct state.** The
promotion rule says a chunk earns the tier by recurring in ≥ 2 templates; DS3
does not get to fabricate three. `made_of: &[ElementId]` is the field that will
later let the Author surface answer "what is this built from" — and is what the
mock's composer reads.

This mirrors exactly how DS2 shipped `ElementCategory::{Container, Control}` —
declared, honestly empty, waiting for promotion.

### 3c. The Library splits its shelf

`LibSection` grows to 9 and the wall functions follow the ladder:

| variant | slug | wall fn | source |
|---|---|---|---|
| `Elements` | `elements` | `elements_wall` (renamed from `blocks_wall`) | `elements::elements()` |
| `Blocks` | `blocks` | `blocks_wall` (NEW) | `blocks::blocks()` → honest-empty copy |

Chip order becomes `Seeds · Templates · Blocks · Elements · Looks · Colors ·
Presets · Fonts · Moods` — ladder order, coarse to fine. The stale `lib.rs` l.8
note ("Elements sections join later") is deleted, because they no longer do.

### 3d. `Traits` → `Looks` (flagged — see §4)

Per the Q2 ruling, the user-facing word for the treatment tier is **Looks**.
`LibSection::Traits` is the only surface still saying "traits" to the operator.
Renaming the variant + slug + blurb is 3 lines and keeps ONE vocabulary across
Library and Author. The authored data (`tokens/traits.json`) and the internal
type names are **untouched** — trait is the correct engineering word, Look is
the correct operator word, exactly as `Slide`/"card" already splits.

## 4. Scope

**In:**
1. `blocks/` → `elements/` move + all 26 call sites + `lib.rs` module decl and
   tier doctrine paragraph.
2. New `blocks/` chunk tier: `mod.rs`, `census.rs`, `BlockId`/`BlockSpec`/
   `BlockDemo`/`blocks()`, returning empty.
3. `LibSection::Elements` added; `blocks_wall` → `elements_wall`; new
   `blocks_wall` painting the honest-empty chunk shelf; chip order; stale note
   deleted.
4. `LibSection::Traits` → `Looks` (variant, slug, blurb only).

**Out — each its own pass:**
- **Promoting any actual block.** Needs a recurrence audit across the 20
  templates first (which composites genuinely recur ≥ 2×). That audit is DS4.
- Author surface Pass B/C (the four-bucket gallery, filmstrip, overlay editor).
- Recipes / interaction presets as a registered tier (`presets.json`).
- `ResolvedStyle` / treatment threading (DS build order §3).
- The `Caps→Tier` fidelity probe honoring `cost_tier` (§5).
- Any physical re-categorisation of elements into subdirs — DS2 §3e reading
  (A) still stands **within** `elements/`; category remains data.

**Flagged for your strike-through:** item 4 (`Traits` → `Looks`) extends the Q2
ruling from the Author gallery to the Library wall. It is the one item here not
literally covered by what you approved. Say the word and I drop it; the rest of
the pass is unaffected.

## 5. Work plan (atomic, in order)

0. **Prune the broken ref** — `git remote prune origin` (or
   `git update-ref -d refs/remotes/origin/feat/session-os`); confirm
   `git branch -a` succeeds. Branch `feat/ds3-ingredient-ladder` off `main`.
1. `git mv crates/slides/src/blocks crates/slides/src/elements` (preserves
   history for all 8 files).
2. Rewrite paths: `blocks::` → `elements::` across the 26 call sites;
   `pub mod blocks;` → `pub mod elements;` in `lib.rs`; update the crate-doc
   tier paragraph to name all three rungs.
3. Reword `elements/mod.rs` module doc — it currently opens "Blocks — atomic,
   insertable units"; it becomes the element tier's doc, and the promotion rule
   sentence moves to the NEW `blocks/mod.rs` where it now belongs.
4. Author `crates/slides/src/blocks/{mod.rs,census.rs}` — the chunk-tier types
   + `blocks()` returning empty, with the promotion rule as its module doc.
5. `library.rs`: add `LibSection::Elements`, reorder `ALL`, rename
   `blocks_wall` → `elements_wall`, write the new `blocks_wall` (honest-empty
   copy reusing the existing empty-shelf phrasing), delete the stale l.8 note.
6. `library.rs`: `Traits` → `Looks` (variant + slug + blurb).
7. Tests: extend the census tests to cover the block census (empty is a valid
   assertion — count 0, ids unique, no panic); add a test asserting
   `LibSection::ALL` slugs are unique and the two walls read different censuses.
8. `cargo green` → PR → §8 evidence → tip `docs/STATE.md`.

## 6. Risks

- **Churn volume.** 34 mechanical edits across 26 files. Mitigated: types
  unchanged, so every edit is a path substitution; rustc catches 100% of misses
  — there is no silent-failure mode. Reviewed file-by-file, not sed-blind.
- **History.** `git mv` (not delete+add) so `git log --follow` survives on all
  8 element files.
- **No-shim law bites.** Any external consumer of `dpl_slides::blocks::*`
  breaks at compile time in the same commit. Verified: the only out-of-crate
  consumers are `hubs/workbench` and `hubs/home`, both in this workspace, both
  in scope.
- **An empty tier reads as unfinished.** Mitigated by the honest-empty copy
  already proven on `ElementCategory::{Container, Control}` — the shelf says
  why it is empty, not merely that it is.
- **Author mock drift.** The mock (artifact `a3db4aee`) is the reference for
  Pass B; if this pass changes vocabulary again, the mock must be republished
  in the same session. It currently matches this plan exactly.

## 7. Verification / acceptance

1. `cargo green` (fmt · spawn+structure audits · clippy `-D warnings` ·
   workspace tests · release build) — **the gate, no `--no-verify`**.
2. `cargo test -p dpl-slides -p dpl-hub-workbench` — the element gallery gate
   still paints all 6 at entrance and rest; new block-census tests pass.
3. `grep -rn "blocks::" crates/ --include=*.rs` returns **only** hits inside
   `crates/slides/src/blocks/` — no stragglers, no shims.
4. `git log --follow crates/slides/src/elements/eyebrow.rs` shows pre-move
   history.
5. **Real-exe proof** (sequential, never parallel): rebuild
   `target/release/devpulselabs.exe`, confirm mtime moved, then
   `--route workbench/library --focus elements --window 1280x800 --screenshot`
   and again `--focus blocks` — the Elements wall shows all 6 shelved by
   category; the Blocks wall shows the honest-empty copy. Both at a REAL window
   size, per the footer-bug law.
6. Acceptance: the three folder names, the three census functions, and the
   three Library chips read the same three words as the Author mock's bucket
   row. No surface says "traits" to the operator; no folder says "blocks" to
   mean atoms.

## 8. Evidence

**Gate.** `env -u CARGO_TARGET_DIR cargo xtask green` → **`green ✔`**
(fmt · spawn audit · structure audit · clippy `-D warnings` · workspace tests ·
release build; exe `target/release/devpulselabs` rebuilt).

**§7.3 — no stragglers.** `grep -rn "blocks::" crates/ --include=*.rs` returns
exactly two hits, both the NEW chunk tier's own API:
`library.rs:445 use dpl_slides::blocks::blocks;` and `library.rs:831` (its
test). Every former element path now reads `dpl_slides::elements::…`. No
re-export shim exists.

**§7.2 — tests.** `cargo test -p dpl-slides -p dpl-hub-workbench`:
26 + 3 passed, 0 failed. New: `blocks::census::tests::the_registry_is_the_census`,
`blocks::census::tests::every_block_paints_at_start_and_at_rest`,
`library::tests::{the_chip_row_is_the_census, elements_and_blocks_are_distinct_shelves,
no_chip_speaks_engineering}`. Pre-existing gates still green
(`elements::census::*`, `registry::the_folder_law_matches_the_registry_taxonomy`).

**§7.4 — history.** `git log --follow` on the six atom files reaches pre-move
history (eyebrow 2, badge 3, stat 3, bar 2, divider 2, media 8 commits).
`elements/census.rs` shows 2 = its complete history (DS2 created it).
**One exception, recorded honestly:** `elements/mod.rs` shows 1 commit — git
paired the old `blocks/mod.rs` with the NEW tier's `blocks/mod.rs` (same
filename) rather than with the moved file. Nothing is lost; the pre-DS3 chain
is fully reachable at `git log -- crates/slides/src/blocks/mod.rs` (6 commits
back to the toolchain commit). A filename reused by a sibling tier is the cost.

**§7.5 — real-exe proof.** The plan's §7.5 was WRONG as written: `--focus`
pins `LibSection::Templates` by design (`workbench/src/lib.rs:137`), so
`--focus elements|blocks` shot the templates gallery. No harness flag reached
any wall but templates. Closed by adding **`--lib-section <slug>`** (see the
scope note below), then:

- `docs/proof/ds3-library-elements-1280x800.png` — `--route workbench/library
  --lib-section elements --window 1280x800`. Chip row reads
  `seeds templates blocks **elements** looks colors presets fonts moods`;
  the wall shelves TYPE (Eyebrow, Stat) · MARK (Badge) · METER (Bar) ·
  DIVIDER (Divider) live from the census.
- `docs/proof/ds3-library-blocks-1280x800.png` — same, `--lib-section blocks`.
  The chunk shelf renders "Nothing here yet — and that is on purpose." with
  the promotion rule stated in plain words.

Both at a REAL 1280×800 window, taken SEQUENTIALLY. Footer confirms build
`2b20cc7`.

**Scope added beyond the approved plan (flagged):** `--lib-section <slug>` —
parsed in `crates/app/src/main.rs`, `WorkbenchHub::set_lib_section` (unknown
slug degrades to the default wall and warns), documented in CLAUDE.md's harness
list. Required because the standing "proof on the real exe" law could not
otherwise be satisfied for this pass: the two walls it delivers were
unreachable. The structure audit's harness-flag census is bidirectional and
passes, so code and CLAUDE.md are pinned in agreement.

**Unrelated pre-existing flake observed:** `dpl-ledger`
`tests::overflow_increments_dropped_instead_of_blocking` failed once under full
workspace parallel load (`dropped=0 handle=0`), then passed 3/3 in isolation
and on the clean re-run. DS3 touches no ledger file. The test asserts a bounded
channel overflows under burst, which does not hold when the writer thread
drains faster than the burst fills — machine-speed dependent. Worth its own fix;
not this pass.

**Not done here:** PR + merge to `main`, and the operator's Windows close-out
(rebuild `target\release\devpulselabs.exe` there, verify mtime, tip
`docs/STATE.md`).
