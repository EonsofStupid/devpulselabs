# W11 — Media → slide: any image / PDF / video / replay becomes a slide

Gate: `docs/plans/wiredfront-experience.md`. Composes with — never re-does —
`w10-slide-association.md` (the slide→episode gesture; W11 mints the slide,
W10 associates it), `w6-frictionless-media-in.md` + `w7-image-layers.md`
(the media shelf + import seam), `w2-doc-renderer.md` (the document renderer).
Ground-truthed in-session 2026-07-27.

## 1. Context

Operator ask: *"right now we have no templates that can properly allow me to
easily add an image, or pdf, or video, session replay etc and have it become
a slide for me to have available for the episode and stream."*

Exactly right. W10 guides a **finished** slide into an episode, but nothing
makes a slide **out of raw media**. The media painter that would render such
a slide already exists and is proven live (the gallery gate) — it is simply
unreachable from operator data. W11 closes that: one document block, one
generic template, one "Make slide" gesture on the media shelf that hands off
to W10.

Ratified this session (operator, two forks):
- **PDF & replay mapping = "explode + reuse."** A multi-page PDF rasterizes
  to N page-images on import (subprocess, ADR-009), each becoming its own
  image media-slide. A session replay becomes a video media-block on the
  session's existing light `.mp4` (W5 `render_replay`). Zero new render code
  — reuses the image and video paths exactly.
- **Scope = Library action + episode chips.** One "Make slide" tap on the
  media shelf mints the slide.json AND surfaces the KEEP-style "add to
  episode" chips, proving the whole loop on the real exe. The chip mechanics
  delegate to W10's association gesture, not a parallel reimplementation.

## 2. Current state (ground-truthed 2026-07-27)

- **The document schema can only hold text.** `dpl_doc::Block` has one
  variant, `Text` — `crates/doc/src/lib.rs:80-97`. The comment admits the
  gap: *"media lands with the first media seed, W2"* (`:78`). It never landed.
  `SCHEMA_V = 1` (`:16`); `load` refuses newer versions honestly.
- **The media painter exists but is unreachable from data.** `slides::blocks::
  media::Media` renders `Image` / `Video` / `Anim` with the honest
  degradation ladder (frames → poster → labeled plate) —
  `crates/slides/src/blocks/media.rs:24-108`. But it is a **code enum keyed
  on `&'static str`**, internal to templates; it has **no PDF and no replay
  arm**. `TitleCard.media: Option<Media>` proves it paints live via the
  gallery (`crates/slides/src/templates/title_card/mod.rs:37,50-55`).
- **No template is a generic media frame.** `registry::demos` is 18
  text/chart/interactive templates — `crates/slides/src/registry.rs:80-99`.
  None takes an operator's imported asset. Adding one = its module + ONE line
  here (the documented rule, `:8-9`).
- **The document renderer resolves refs → paint in one path** but only knows
  `Block::Text` today — `crates/slides/src/document.rs`. A new block needs an
  arm here.
- **The media shelf + import seam already exist (W6/W7).** Assets land in
  `content/inbox/` with posters + index write-through
  (`crates/composer/src/import.rs:6,73`); the shelf's one outside seam is
  `Composer::add_from_library` (STATE, W6). Slide/comp folders are minted
  no-clobber via the `inbox/<name>-<n>` pattern (`crates/composer/src/lib.rs:
  806-863`).
- **Episodes list slide subfolder names.** `Episode { v, id, title, chapters
  }`, `Chapter { slides: Vec<String> }` — `crates/doc/src/lib.rs:337-351`;
  slides live at `content/<show>/<episode>/<slide>/slide.json`. Seed shape:
  `seeds/episodes/how-ai-works/episode.json`. `save_episode`
  (`doc/src/lib.rs:374-376`) exists but has no production caller — **W10
  wires it**, and W11 reuses W10's gesture rather than call it directly.
- **PDF: zero handling** anywhere in `crates`. **Replay:** exists only as a
  session's light `.mp4` (W5 `render_replay`); no bridge to a slide.

**THE GAP:** the document layer cannot express media, no template renders it
from data, and the shelf offers no "→ slide" action. Everything downstream
(episode format, deck, posters, index) is ready.

## 3. Design & ratified decisions

Three additive pieces, each riding an existing pattern:

### 3.1 `Block::Media` in `dpl_doc` (data, refs-not-values)
```
Media {
  id: String,
  src: String,        // content-relative asset ref, NEVER an absolute path
  fit: Option<String>,// "cover" (default) | "contain"
  poster: Option<String>, // content-relative; video/replay degradation still
}
```
Bump `SCHEMA_V → 2`. `load` already refuses newer versions honestly, so old
builds fail loud, not silent. `validate` gains a media arm: `src` must resolve
under the content root (the jail law from R1w) — an escaping/absolute path is
an `Issue`, never a load. Refs-not-values holds: `src` names an asset the
library owns, resolved at paint time.

### 3.2 A `media_frame` template
One module `crates/slides/src/templates/media_frame/mod.rs` + ONE line in
`registry::demos`. It renders a full-bleed media stage by reusing
`media_backdrop` — thin, because the painter already exists. Extend
`slides::blocks::media::Media` and `media_backdrop`:
- add a **`PdfPage`**? — **NO.** Per the ratified fork, PDF never reaches the
  renderer as a PDF: it is exploded to page-images at import, so each slide
  carries an ordinary `Image` src. No new renderer arm for PDF.
- **replay** reuses the existing `Video` arm pointed at the session light
  `.mp4`. No new arm.
So the ONLY renderer change is teaching the painter to accept a **runtime
path from the document** (today it takes `&'static str`). The document
renderer's `Block::Media` arm resolves `src` → the shelf path → `media_backdrop`.

### 3.3 The "Make slide" gesture (the one new seam)
A shelf action `Composer::make_slide_from_library` (sibling of
`add_from_library`), plus its UI tap on the media shelf. It:
1. mints a slide folder no-clobber (the `inbox/<name>-<n>` pattern);
2. writes `slide.json` = `{ v:2, template:"media", background:none,
   blocks:[ media{ id, src } ] }`, `src` relative to the content root;
3. for a **PDF**, first rasterizes to N page-images via a subprocess
   (ADR-009 — a real rasterizer, e.g. `pdftoppm`/`mutool`, resolved the
   embedded-deps way; **never a crate PDF parser**), minting N image slides;
4. for a **replay**, points `src` at the session's light `.mp4`
   (W5 `render_replay` output), poster from the existing session poster;
5. surfaces KEEP-style "add to episode" chips that call **W10's association
   gesture** (`save_episode` via W10) — W11 does not reinvent association.

Quiet-spawn law (W8): the rasterizer subprocess goes through `dpl-proc::quiet`
or the green audit fails.

## 4. Scope & blast radius

**In:** `Block::Media` + `SCHEMA_V` bump + `validate` arm (`dpl_doc`); the
`media_frame` template + registry line + document-renderer arm + a runtime
path into `media_backdrop` (`dpl_slides`); `make_slide_from_library` + the
PDF rasterize helper + the shelf tap + episode chips delegating to W10
(`dpl_composer` / workbench). One media rasterizer as an embedded-dep
subprocess.

**Out (explicitly):** slide→episode association mechanics (W10 owns them);
live/interactive replay embeds (the "one paged block" fork was declined);
paged-PDF-in-one-slide; editing a media slide's crop/timing in the editor
(a later pass); PDF text extraction of any kind.

**Blast radius:** `SCHEMA_V` bump touches every `slide.json` reader — but old
seeds stay `v:1` and load unchanged (only media slides are `v:2`); a v2 slide
on an old build fails loud by design. The renderer arm is additive (unknown
block → the existing degrade path). No episode-format change.

## 5. Task plan (atomic)

1. `dpl_doc`: add `Block::Media`, bump `SCHEMA_V→2`, `validate` jail arm,
   round-trip + version-refusal tests.
2. `dpl_slides`: teach `media_backdrop` a runtime (owned `String`) path
   alongside the `&'static str` demo path; no behavior change for existing
   callers.
3. `dpl_slides`: `media_frame` template module + `registry::demos` line +
   the gallery renders it live (gallery gate).
4. `dpl_slides::document`: `Block::Media` render arm (resolve `src` → paint).
5. `dpl_composer`: `make_slide_from_library` (mint folder, write v2 slide.json).
6. `dpl_composer`: PDF rasterize helper — embedded-dep subprocess via
   `dpl-proc::quiet`; explode to N image slides. Honest error if the tool is
   absent (naming it), never a crash.
7. Replay path: `make_slide_from_library` accepts a session id → video media
   slide on the light `.mp4` + session poster.
8. Workbench media shelf: the "Make slide" tap + KEEP-style episode chips
   that call W10's association gesture.
9. Harness seam `--make-slide <asset-rel>` (headless mint, mirrors
   `--save-comp`) for SEQUENTIAL real-exe proof.

## 6. Risks

- **PDF rasterizer as a new C-dep.** Mitigate: ADR-009 subprocess only,
  resolved through embedded-deps (never a crate); absent → honest labeled
  failure, feature degrades, app never crashes. Confirm the exact tool +
  staging before task 6 (parallels ffmpeg/aerender).
- **Schema bump ripple.** Mitigate: media slides only are v2; every existing
  seed stays v1 and must still load (regression test over all `seeds/`).
- **Two "Library" surfaces** — the Workbench *vocabulary* Library
  (`workbench/src/library.rs`) vs the W6 *media shelf*. The gesture belongs on
  the **media shelf** (where real assets live). Confirm the exact shelf module
  + `add_from_library` call site before task 8.
- **W10 dependency.** The episode chips need W10's association gesture. If W10
  hasn't landed the production `save_episode` caller yet, W11 tasks 1–7 + 9
  stand alone (a mint-only pass); task 8's chips wait on W10 or mint into a
  scratch episode behind the same seam.

## 7. Verification target

`cargo green` (fmt + clippy + quiet-spawn audit + tests + release build), then
SEQUENTIAL real-exe drills:
- `--make-slide assets/generated/dpl-poster.webp` → a v2 media slide folder
  with valid `slide.json`; the Library gallery renders `media_frame` live.
- A real multi-page PDF → N image slides, page order preserved, byte-checked
  page images; screenshots at `--window 1280x800` and `1600x1000`.
- A real session id → a replay video slide on its light `.mp4`.
- The media shelf tap mints + the episode chip lands the slide via W10 (once
  W10 is present).
Proof shots + ffprobe/sqlite evidence filed in §8.

## 8. Evidence

Implemented 2026-07-27. First built on the `devpulse_labs` divergent
snapshot, then re-applied onto live `main` (branch `feat/w11-media-to-slide`)
— the four coach/estate/W10a-diverged files hand-merged, the rest clean.
Release-exe rebuild + screenshot proof on real indexed content is the
operator's Windows-side close-out.

**Code (all landed, additive):**
- `dpl_doc`: `Block::Media { id, as, src, fit, poster }`, `SCHEMA_V 1→2`,
  `Block::id()`, jail predicate `rel_ok`, `validate` media arm; `resolve`
  skips media. `crates/doc/src/lib.rs`.
- `dpl_slides`: `media_block_backdrop` runtime path (refactored the image/
  video ladders into shared `paint_image`/`paint_video`); the `media_frame`
  template + one `registry::demos` line; the `document.rs` media pre-pass
  (full-bleed, reveal-honoring, text-flow untouched).
- `dpl_video`: `locate_asset` also probes a `content/` root (bridges the
  content-relative artifact path space; assets/ still resolve first).
- `dpl_composer`: `make_slide_from_library` / `mint_media_slide` /
  `media_slide_for` / `make_slides_from_pdf` / `make_slide_from_rel`;
  `import::rasterize_pdf` (pdftoppm subprocess, quiet-spawn). `dpl-proc` dep
  added.
- `dpl_editor`: 5 text-only helpers now skip media blocks (`let … else`).
- `wiredfront`: shelf card "Make slide" chip + `LibAction` + `slide_note` +
  the `--make-slide` hub seam. `app/main.rs`: `--make-slide <rel>` wired
  through all 4 boot sites.

**Gate (Linux, this mirror):** `cargo check --workspace` clean; `cargo fmt
--check` clean; `cargo clippy` clean (no `#[allow]`); tests — dpl-doc 35,
dpl-slides 20, dpl-composer 39, dpl-editor 15, dpl-deck 6, present 2 — all
green. The `#[ignore]` `rasterize_pdf_explodes_ordered_pages` drill PASSES
against real `pdftoppm` (2-page PDF → 2 ordered PNGs).

**Real binary (debug exe, headless, scratch `--content-root`):**
- `--make-slide inbox/poster.webp` → `inbox/poster/slide.json` (v2,
  `template:"media"`, `{kind:media, as:image, src:"inbox/poster.webp",
  fit:cover}`). Exit 0, printed "1 slide(s) minted".
- `--make-slide inbox/report.pdf` (real 2-page PDF) → `inbox/report-1/` +
  `inbox/report-2/` media slides, pages rasterized + imported as
  `inbox/report-{1,2}.png`. Page order preserved.
- `--route wiredfront/library --window 1280x800 --screenshot` renders the
  media shelf (the "Make slide" surface).

**Deferred to the Windows checkout (operator):** rebuild
`target\release\devpulselabs.exe` + move mtime; screenshot the media-shelf
"Make slide" chip on a REAL indexed image/video card and the Workbench
Library gallery's "Media frame" card, at real window sizes; land via PR.
The precise `fit:"contain"` letterbox (data carried, renders as cover today)
and the W10 episode-association hop from the shelf note are the named
follow-ups.
