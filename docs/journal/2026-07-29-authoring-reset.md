# Authoring passes B and C — superseded, and what to salvage

**2026-07-29.** Decision record for PRs #128 (Pass B) and #130 (Pass C), both
closed unmerged. Written so the work in them is reusable rather than lost.

## What happened

Passes B and C were built to the FIRST authoring mock — a four-bucket gallery
over a comp filmstrip. The operator then red-penned that mock twice, and both
corrections cut underneath what those branches had built:

1. **Reverse the IA.** Declare intent at a dashboard first (edit existing ·
   expand the platform · work on content), *then* reach the pieces. B/C are
   gallery-first, with the intent bar as a header strip.
2. **Translate the vocabulary.** "the UI shows blocks elements and traits you
   are still not really translating that … elements the eyebrow would be
   confusing". B/C's `enum Bucket { Blocks, Elements, Effects, Templates }` is
   the code's own vocabulary drawn straight onto the screen.

Two further rulings landed after those branches were written:

- **Effects splits** into Looks (traits) and Interactions (recipes) — one
  bucket in B/C maps to two unrelated tiers with different storage and
  lifecycles.
- **The container is configurable** — lesson · guide · episode · deck, loaded
  not compiled in. B/C's "comp filmstrip" also collides with `crates/comp`,
  the wiredFRONT AV composition, against the two-editors-never-mix law.

Both branches are also stale against `main`: they predate the signal-ledger and
chat-assist merges, so their diffs read as deletions of both.

DS3 then renamed `blocks/` → `elements/`, which breaks
`dpl_slides::blocks::elements()` at `author.rs:147` on both branches.

## The call

**Rebuild, don't retrofit** — for the palette layer only. Retrofitting would
mean rewriting the bucket enum, the filmstrip's container model, the IA order,
and every drawn string, on a base that also needs rebasing past two merges.
That is not a retrofit; it is the same rebuild with a harder starting point.

The replacement landed in PR #136: a shelf picker over one carousel, six
shelves, census-driven, plain words throughout.

## Salvage map — what is still worth taking from #130

The overlay editor was NOT superseded. The ratified mock still calls for it,
with tabs and a live preview beside the fields. These pieces port largely as-is:

| from `feat/authoring-c-editor` | why it survives |
|---|---|
| `editor_modal` — scrim `egui::Modal`, tab rail | the mock's overlay editor, same shape |
| `CardEditor` wrapping the REAL `dpl_editor::Editor` over the draft's `slide.json`, edits auto-saving | the honest path: editing a card edits its document. Do not reinvent this. |
| `open_editor(dir, title)` | opening by directory, not by index |
| `DraftCard` | the minted-draft record behind a filmstrip card |

What to leave behind: `Bucket`, `Palette`, `palette_rail`, `bucket_items`
(replaced by `Shelf` + the carousel), `comp_deck` (replaced when the filmstrip
lands, labelled by the loaded container), and the header's
"Library entities count" (an `EntityRegistered` fact that does not exist yet —
STATE defers it to the pass that actually registers entities, and no fabricated
fact should ship before it).

Tabs need renaming on the way over: Content · Quote · Media · Interactions ·
Ask AI becomes **Words · Background · Movement · A quote · Picture · On tap ·
Ask for help**.

## Open after this

- Port the overlay editor per the salvage map, onto the carousel surface.
- The filmstrip, labelled by whichever container is loaded.
- The seed-thumbnail crop named in #136 — a thumbnail mode in the document
  renderer, `dpl-slides`.

Branches `feat/authoring-b-gallery` and `feat/authoring-c-editor` are kept, not
deleted, so the salvage map above has something to point at.
