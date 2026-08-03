# Authoring lane — where it stands, and what to do next

**2026-07-29.** Written as a resume point: read this and `docs/STATE.md` and you
have the whole lane without re-deriving it.

## The branch picture (this was the confusion — resolved)

Squash-merges make a branch's tip a non-ancestor of `main`, so
`git branch --merged` reports landed work as UNMERGED. Everything below was
verified **by content on `main`**, not by ancestry.

**Landed and deleted:** `feat/ds3-ingredient-ladder` (#135),
`feat/author-carousel` (#136), `docs/state-authoring-reset` (#137),
`feat/session-os` (#133), `feat/coach-postcondition`,
`feat/p4-rail-accordion-degrade`, `feat/strict-refusal-witness`.

**Kept on purpose** — the salvage map in
`docs/journal/2026-07-29-authoring-reset.md` points at them:
`feat/authoring-b-gallery` (#128 closed), `feat/authoring-c-editor` (#130
closed).

**Open PR:** **#141** `feat/author-overlay-editor` — the overlay editor.

### There are FIVE GIT WORKTREES

```
/home/hades/Projects/devpulselabs              main          ← work here
/home/hades/Projects/devpulselabs-authoring-a  feat/authoring-a-signals
/home/hades/Projects/devpulselabs-authoring-b  feat/authoring-b-gallery
/home/hades/Projects/devpulselabs-authoring-c  feat/authoring-c-editor
/home/hades/Projects/devpulselabs-charter      docs/authoring-charter
/home/hades/Projects/devpulselabs-ds2          feat/ds2-element-library
```

This contradicts CLAUDE.md's "there is no other worktree", and it is why a
branch checked out elsewhere cannot be deleted or switched to from the main
checkout. It also explains uncommitted work vanishing mid-session: **commit
after every step that compiles.** Do not leave a working tree dirty across a
long task.

Pass A, DS2 and the charter are all content-merged into `main`; their worktrees
are stale and can be pruned by whoever owns them.

## What landed this session

1. **DS3 — the ingredient ladder** (#135). `elements/` (the six atoms, `git mv`,
   34 refs, no shim) · a NEW `blocks/` chunk tier, **empty by design** with the
   census test asserting the emptiness · `templates/` untouched. Library split
   into `elements_wall` + an honest-empty `blocks_wall`. New seam
   `--lib-section <slug>` (because `--focus` pins TEMPLATES).
2. **Plain words** (same PR). `slug()` = machine id, `label()` = what is drawn.
   **Starters · Layouts · Combos · Pieces · Themes · Colors · Styles · Type ·
   Moods**; Eyebrow→Small label, Stat→Big number, Badge→Tag, Bar→Progress bar,
   Divider→Line, Media→Picture. Guarded by `no_chip_speaks_engineering` and
   `no_piece_speaks_engineering`.
3. **The carousel** (#136). Shelf picker over one lane of large cards, all
   census-driven; tap inserts through the same mint path as the blank button.
   Fit-guarded both axes.
4. **Ledger flake** fixed — `LedgerHandle::stalled()` seam pins the
   full-channel behaviour deterministically.
5. **Passes B/C retired** with the salvage map.
6. **The overlay editor** (#141, open). Words is the real
   `dpl_editor::Editor` over the card's `slide.json`; six honest placeholders;
   `--author-editor` seam so it is provable on the exe.

## NEXT, in order

1. **Merge #141** once reviewed.
2. **The filmstrip**, labelled by whichever container is loaded — lesson ·
   guide · episode · deck, read from disk, never compiled in. The container
   model is ratified but nothing implements it yet: `KINDS`/`CONTAINERS` exist
   only in the mock.
3. **The six placeholder tabs**, each its own pass. Background and Movement are
   the big ones (~40 options per group, tapped straight onto the card).
4. **The seed-thumbnail crop** (named in #136): a full slide drawn into a
   card-sized stage overflows because the type ramp has a floor. Needs a
   thumbnail mode in the document renderer — `dpl-slides`, not the surface.

## The reference

Mock artifact `a3db4aee` — dashboard-first IA, shelf picker over one carousel,
plain words throughout, the elements→templates→cards build ladder made visible.
It is the ratified reference for this lane; when code and mock disagree, the
mock is what the operator approved.

## Standing corrections from this session

- **"the UI shows blocks elements and traits you are still not really
  translating that"** — engineering words never reach the screen. Both
  vocabularies, always split.
- **"you still have never made the carousel"** — premades are picked by
  sweeping and tapping, not by typing.
- **Plan before code**, PR not direct push, prove on the real exe at real
  window sizes, and `cargo green` is the gate — never `--no-verify`.
