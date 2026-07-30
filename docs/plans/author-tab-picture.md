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

### 8.1 The gate

`cargo green ✔` — including the structure audit with `dpl-index` newly on the
workbench manifest and **no** `architecture.md` amendment owed, as §2
asserted. 41 tests in `dpl-hub-workbench`, 9 new.

Census: `the_kind_row_is_the_schemas_media_vocabulary` ·
`no_kind_speaks_engineering` · `every_fit_has_a_plain_word` ·
`every_offered_picture_is_a_legal_media_ref` — the last builds a document from
every record the shelf offers and asserts `validate` is silent, which is the
jail proven by construction rather than by hope.

Behaviour: `setting_a_picture_writes_a_valid_media_block` ·
`contain_is_written_because_it_is_not_the_default` (sparse documents) ·
`a_second_picture_replaces_the_first` · `no_picture_clears_it_and_keeps_the_words`
· `an_escaping_ref_would_not_validate` (`/etc/passwd`, `C:\x.png`,
`../../escape.png` all refused — the jail is real, which is *why* passing index
paths through keeps this tab inside it).

### 8.2 Real-exe proof

| shot | what it proves |
|---|---|
| `docs/proof/author-tab-picture-empty.png` | the empty-index line, built and shot FIRST per §6 |
| `docs/proof/author-tab-picture-1280x800.png` | three real thumbnails, `quote-card` ringed as chosen, "No picture" and "How it sits" present, the live pane rendering the card with the picture on it |
| `docs/proof/author-tab-picture-1100x700.png` | the same at a short window |

The populated runs were seeded through `--import-drill`, the app's own import
seam — so the records under test arrived the way real ones do, not by hand.

### 8.3 Deviations and findings

**The proof run found a hang, and it was mine.** `picture_tab` called
`pictures()` every frame — and `pictures()` opens the artifact index, which is
a database connection, not a lookup. The exe never reached its screenshot.
§6 of `author-filmstrip.md` had already written the rule down ("read once into
state, never per frame") and this pass broke it. The shelf is now cached on
`CardEditor` and re-read only by the gesture that changes it, exactly as
`containers.rs` does. Recorded because a per-frame DB open is the kind of
defect that looks like a UI freeze and gets blamed on paint.

**`locate_asset` was not content-root aware — a pre-existing bug in every
drill.** With the hang fixed, every tile still showed "file not found" for
files that were plainly on disk. `dpl_video::locate_asset` probes exe-adjacent
dirs and the cwd, but **not** the root named by `--content-root`, so any drill
pointed at a scratch dir resolves nothing — media *cards* included, not just
this picker.

Fixed in `dpl-video` by reading `--content-root` (and the `CONTENT_ROOT` env
behind it) straight from the process args. That is the same contract
`dpl_engine_capture::explicit_root` implements, and doing it with two lines of
arg parsing honours that function's own note about staying free of a heavy
engine-capture dependency. Scope beyond this plan, and worth having: it makes
every media-bearing drill provable.

**The degrade ladder behaved perfectly while broken**, which is why the bug was
diagnosable rather than mysterious: each tile painted the honest "file not
found" plate instead of a blank or a crash.

**`cover` is not written.** It is the schema's default, so `set_media` stores
`fit` only when it is `contain` — a sparse document, the same discipline the
rest of `dpl_doc` keeps.
