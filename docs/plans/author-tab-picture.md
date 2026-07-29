# Pass — the "Picture" tab

An image or a video on the card, chosen from what the operator already has.

**Reference:** R1 — `docs/ops/references/authoring-mock-a3db4aee.html`. The tab
copy: *"An image or a video on the card"*; `PIECES` carries
`{k:"media", name:"Picture", d:"an image or a video, dropped in"}`.
**POA&M:** `docs/plans/authoring-lane-poam.md` — M5, tab 5 of 7.

## 1. Context — why this pass exists

The schema has carried media since W11 and the renderer paints it. The one
thing missing is a way for an operator to *choose* one, which is why the tab
still says "Not built yet."

## 2. Current state (ground-truthed in-session, `1eaccdf`)

**The block exists and is fully specified.** `dpl_doc::Block::Media`:

```rust
Media {
    id: String,
    #[serde(rename = "as")] show: String,   // "image" | "video" — AUTHORED, never guessed
    src: String,                            // content-relative, e.g. "inbox/report/page-01.png"
    fit: Option<String>,                    // "cover" (default) | "contain"
    poster: Option<String>,                 // video's degradation rung 2
}
```

`MEDIA_SHOWS = ["image","video"]`, `MEDIA_FITS = ["cover","contain"]`.

**The jail is a validator law, not a convention.** `rel_ok` rejects an empty
path, a leading `\`, anything containing `:`, an absolute path, and any `..`
component. `validate` reports *"src escapes the content root"*. The renderer
never follows a ref off the reservation.

**The renderer already degrades.** `media_block_backdrop` paints a missing
file as a labelled plate; a video's ladder is frames → poster → plate. So a
picker cannot produce a crash, only a plate.

**There is already an index of everything the operator has.**
`dpl_index::Index::open_default()` (at `.userdata/index.db`) with
`recent(limit, location)` returning `Record { rel_path, location, kind, name, … }`
where `rel_path` is **already content-root-relative** ("inbox/pix.png") and
`kind` is `"image" | "video" | "guide" | …`.

**That fit matters:** an index-backed picker cannot violate the jail, because
every `rel_path` it offers is already relative and inside the root. No file
dialog, no path munging, no way to author an escaping ref.

**`dpl-index` is not yet a workbench dep.** It is an ENGINE crate and
`dpl-seam`, `dpl-viziwiz`, `dpl-composer` and `hubs/wiredfront` all take it, so
a hub taking it is established and needs no `architecture.md` amendment —
asserted here, verified by running the structure audit.

**Sources are never mutated** (CLAUDE.md). This pass only ever *references* an
existing file by its relative path. It copies nothing, converts nothing, and
writes no bytes outside the card's own `slide.json`.

## 3. Design — pick from what you have, see it immediately

**The picker is the index, not a file dialog.** A grid of what the operator
already has, newest first, each tile a live thumbnail of the actual file. Two
filters in plain words: *Pictures* and *Videos* (`kind`), because those are
the two things `show` can be.

**Tapping writes the block** through a new `Editor::set_media(...)`, same
`mutate` path as `set_background` — undo and save for free. `show` is set from
the record's `kind` (**authored, never guessed from the extension**, which is
the schema's own rule and the reason the doctor can name a mismatch).

**Fit is two plain words**: *Fill the card* (`cover`) and *Show all of it*
(`contain`). Default `cover`, as the schema says.

**An empty index says why, and what to do.** If nothing is indexed the tab
does not show an empty grid: it says the operator has no pictures in their
content folder yet and names how they get there (capture, import). Same
honest-empty move as `Kind::not_stored_yet`.

**A missing file is shown as missing.** If a record's file has since gone, the
tile paints the renderer's own labelled plate rather than a blank — the tab
degrades the same way the card will.

**Remove is a first-class control.** *"No picture"* clears the block, mirroring
the Background tab's *"Nothing"*.

## 4. Scope

**In:** `dpl-index` on the workbench manifest · the index-backed grid with
image/video filters · live thumbnails · `Editor::set_media` · the fit choice ·
"no picture" · the empty-index line · tests · proof shots.

**Out, and named:**
- **A file dialog / drag-and-drop from outside the content root.** It would
  mean copying files in (a capture concern) or authoring an escaping ref (a
  jail violation). Importing is `--import-drill`'s lane, not a tab's.
- **Poster selection for videos.** The schema carries `poster`; choosing one
  means picking a frame, which is a video-tooling pass.
- **Cropping, or per-block media.** `Media` is a block in the flow; cropping is
  editing an asset and this pass mutates no sources.
- **Reindexing from the tab.** If the index is stale that is the index's
  problem; `reindex` exists and belongs to whoever owns it.

## 5. Work plan (atomic, in order — commit at each step that compiles)

1. `dpl-index` onto the workbench manifest; run `cargo xtask structure` to
   confirm no amendment is owed.
2. A `pictures.rs` census: open the index, list image/video records, map each
   to a plain-word choice. Honest-empty when there is none. Tests, no UI.
3. `Editor::set_media(record) / clear_media()`.
4. The tab: filters, grid of live thumbnails, fit choice, "no picture".
5. Proof shots at 1280×800 and 1100×700 via `--card-tab picture`, §8, the
   POA&M row, `docs/STATE.md`.

## 6. Risks

- **The index may be empty on a fresh machine**, which is also the state every
  proof run starts in. Mitigation: the honest-empty line is the *first* thing
  built and the first thing photographed; the populated shot needs a scratch
  content root with a real image and a `reindex`.
- **Thumbnail cost.** A grid of decoded images is heavier than ten mood
  gradients. Mitigation: measure at 1280×800; if it is slow, cap the grid and
  `log` the cap rather than silently truncating.
- **`kind` and the file's real type disagreeing.** The schema deliberately
  authors `show` rather than sniffing. Mitigation: write `show` from the
  record's `kind` and let the doctor name a mismatch, exactly as designed —
  do not add a second guess here.
- **Video in a live preview.** The live pane renders a real document; a video
  block there could try to play. Mitigation: check what
  `media_block_backdrop` does at thumbnail size before wiring it, and if it
  plays, prefer the poster/plate in the pane.

## 7. Verification / acceptance

- `cargo green` clean; structure audit passes with the new dep and no
  amendment.
- A test that every offered choice writes a `src` that **passes `rel_ok`** —
  the jail proven by construction, not by hope.
- A test that a written document validates, and that `show` matches the
  record's `kind`.
- A test that clearing removes the block.
- Real-exe proof at both sizes: the grid with at least one real picture, the
  card in the live pane showing it, and a separate shot of the empty-index
  line.

## 8. Evidence

_Filled during the pass._
