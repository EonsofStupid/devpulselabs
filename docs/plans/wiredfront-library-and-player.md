# Pass — the editor wears aero+hacker, the library caches, and media previews before it lands

## 1. Context

Three operator requirements, 2026-08-05, verbatim:

> "you must ensure that this editor is faithfully getting styled correct we
> will default to neo first the lime green"

...corrected by the operator the same day, and the correction is the ruling:

> "i was wrong to state default is brute neo while that will exist as an
> option i agreed v3 but as aero hacker"

> "then we need to make sure that the library is easily providing a simple
> browser add folder experience that will ask to cache for thumbnail
> generation"

> "The webplayer was suppose to be able to allow us to have a popup experience
> and see it play if a video or audio or see the image and easily close when
> done or click add to edit so it adds as a new layer automatically"

Three deliverables, in dependency order: **A** the editor's default dress,
**B** add-a-folder with consented thumbnail caching, **C** the preview popup
that can promote what it is showing into the timeline.

**A settles a decision that flipped twice.** `visual-language.md` proposed
`aero · measured · neon-glass · neon` from v3 mock line 401; the operator then
said neo/lime; then corrected to **`aero` + `hacker`**. The correction stands,
and it AGREES with both the C1 ruling (*align to v3*) and the mock — so the
document and the mock now say the same thing rather than disagreeing.

Per `visual-language.md` §3b the authoring unit is **(ancestor, preset)**, not
a bare coordinate: `aero+hacker` RESOLVES to that coordinate. `brutalist+neo`
on `lime` remains a selectable OPTION.

## 2. Current state (read in-session 2026-08-05)

### 2.1 Styling

- `crates/composer/src/lib.rs` paints via `cl::treated(tone, "neubrutalist",
  &[], state)` / `cl::brutal(tone)` — commit `2b0dacc` wired 9 icon call sites
  onto those helpers.
- `tokens/traits.json` `neubrutalist`: fill `{tone:400}` OPAQUE, ink
  `{surface:WELL}` → **dark ink on a bright saturated plate**. That is why the
  proof shots read as pills.
- `docs/design/references/neubrutalist.png` shows the correct usage: **one**
  saturated hero button; every other control is a dark extruded plate.
- The v3 mock §4 (rendered and reviewed) carries the glass stops; the seed
  bench carries Neo-Brute's formula. **v3/aero+hacker is the default; the seed
  bench is the OPTION**, not the target.

### 2.2 Library / browse

- `crates/composer/src/browse.rs` (934 lines) **already has**: `roots()` incl.
  every Windows drive; `add_library`/`remove_library` persisting
  `.userdata/libraries.json`; a shared off-thread `ScanCache`; `warm_cache`
  at boot; deep search capped at 500; `pin_pack`; two-tap `ingest`
  (**copies**); and `THUMBS_PER_FRAME = 8` — the house per-frame work budget.
- **Thumbnails are only generated at IMPORT.** `import::finalize_import`
  calls `dpl_engine_media::write_poster` for `kind == "video"` and writes
  `rec.poster_rel_path`. Browsing a folder generates nothing.
- `write_poster` (`engine-media/src/lib.rs:276`) is an **ffmpeg subprocess per
  file**. That cost is exactly why bulk thumbnailing must be consented to —
  the operator's "ask to cache" instinct is correct, not optional politeness.
- `browse.rs` has a mogrt-only thumb map (`thumbs: HashMap<PathBuf,
  Option<Arc<[u8]>>>`); images paint via `file://` URIs; **video/audio paint a
  text badge** ("VIDEO"/"AUDIO"), because *"video posters are made at import,
  never by probing folders"* (its own comment).

### 2.3 Preview / player

- **No webplayer surface exists.** `grep -rln "webplayer|web_player|WebPlayer"`
  → nothing.
- The machinery does exist: `dpl_video::Player` (`video/src/lib.rs:90`) with
  `texture(ctx)`, `dpl_video::clip` (`clip.rs:242`), and `LottiePlayer`. The
  composer already drives all three for its viewer (`preview`,
  `preview_still`, `preview_lottie`).
- **`add_clip(&Record)`** (`composer/src/lib.rs:906`) is the "add to edit"
  seam: it checkpoints for undo, resolves duration (record → probe →
  fallback), and appends onto the end of the comp. It takes an **indexed
  `Record`**, not a bare path.
- Browse windows are **deferred native viewports** (`show_browsers`), which is
  the house pattern for a peer window.

### 2.4 The blocker

**There is no audio playback anywhere in the workspace.** No `rodio`, no
`cpal`, no `ffplay`. `grep` across every crate and manifest returns nothing.
"see it play if a video or audio" therefore needs a **decision**, not wiring
(§7.1).

## 3. Target design

### A — the editor wears `aero` + `hacker`

The default is the v3 language: **flat wells, liquid-glass/cyber chrome, glow
used as a SPOTLIGHT on the active thing** — the `WORKBENCH-editor.png` read. Resolved
from `matrix.json`, every value authored, nothing hand-picked:

| element | today | aero + hacker |
|---|---|---|
| panel fill | translucent, no edge | `surface RAISED` (via M2 fallback to aero's native `clay`) |
| edge | none | **1px cyan-400**, `measured` radius 4 |
| ink | mixed | **cyan-100** — cold, pale |
| ordinary control | saturated tone-400 pill | dark plate, thin cyan edge, **no glow** |
| the ACTIVE thing | indistinguishable | the cyan glow ring (spread 14 + 28) |
| wells / stage | mood bleeds through | flat `WELL` |

**The correction this makes to today's editor** is the same one either seed
would have made: *stop giving every control the loud treatment.* Today
`neubrutalist.fill = {tone:400}` is opaque and applied to everything, so
nothing reads as selected. Under aero+hacker the loud thing is the **glow on
the active element**, and everything else is a quiet plate.

`brutalist + neo` on `lime` stays authored and selectable — it is an option,
and the seed bench already carries its formula.

It rides `cl::treated` today; when `resolve()` lands (`docs-history-2026-10-12:docs/plans/matrix-resolve.md`) it
becomes the coordinate proper with no visual change.

### B — add a folder, then consent to caching

The flow, in the operator's words, made concrete:

1. **Browse → "★ Add as library"** already exists and persists. Unchanged.
2. **On add, ask**: *"Make thumbnails for the N videos in this folder? It runs
   once and makes browsing instant. Your files are never modified."* — with
   the **file count and an honest size estimate** shown before the operator
   answers, exactly as the two-tap ingest already does.
3. **Yes** → an off-thread worker walks the folder and calls `write_poster`
   per video into a **cache keyed by (path, mtime, size)** under
   `.userdata/thumbs/`, never beside the operator's files (sources are never
   mutated — house law).
4. **No** → the folder still works; video cards keep the honest text badge.
   The offer is re-available from the folder's own bar, never nagged.
5. Progress is **stated, not spun**: "42 of 310 — you can keep browsing."
   The worker is cancellable and survives across Browse windows via the same
   `Arc<Mutex<…>>` shape `ScanCache` uses.

**Cache key includes mtime+size** so a replaced file re-thumbnails and a moved
folder does not orphan its cache. A cache miss degrades to the badge — never
a blocking wait, per the proactive-buffer law.

### C — the preview popup

A **deferred viewport** (the Browse-window pattern), opened by clicking a card
in the Library or a Browse window.

| kind | shows | driven by |
|---|---|---|
| video | plays, with transport | `dpl_video::Player` |
| lottie | plays | `LottiePlayer` |
| image | the image, fit | `egui::Image` |
| audio | **§7.1 decision** | — |

Two actions, both one tap:

- **Close** — Esc, the ✕, or clicking outside. The viewport dies; nothing is
  imported, nothing is written.
- **Add to edit** — the item lands as a **new layer** in the open comp.
  Routing: if the item is already an indexed `Record`, straight to
  `add_clip`. If it is a bare path from a browsed folder, it goes through the
  **one import seam** first (`import_file`, or `import_reference` when
  reference mode is on) and then `add_clip`. **No second import path is
  invented** — that seam is already the only door and it must stay that way.

`add_clip` already checkpoints, so **one undo removes the layer**, and the
popup stays open so several items can be added in a row.

## 4. Changes, in execution order

**A — the editor wears `aero` + `hacker`** (smallest, independently shippable)

1. `crates/composer/src/lib.rs` — `tool_btn` / `tool_cap_btn` / `icon_btn` /
   `icon_word_btn` / `small_btn` / `tag_chip`: quiet dark plate + 1px cyan
   edge + `measured` radius per §3A. **The glow ring is reserved for the
   ACTIVE control only** — that is the whole correction.
2. Same file — `ae_screen`'s panel rects take the flat `WELL`/`RAISED`
   treatment with a hairline edge; no saturated fills anywhere.
3. Proof: `--route wiredfront/editor` at maximized, `1100x700`, `1600x900`;
   compare against `docs/design/references/WORKBENCH-editor.png` and v3 §4's glass
   stops region by region.

**B — the library caches** (needs no new dependency)

4. `crates/engine-media` — `poster_for(src, cache_dir) -> PathBuf` keyed by
   `(path, mtime, len)`; hit returns instantly, miss runs `write_poster`.
5. `crates/composer/src/browse.rs` — a `ThumbJob` (files, done, cancelled)
   behind the existing `Arc<Mutex<…>>` shape; the consent prompt on
   `add_library`; the progress line; the re-offer control.
6. Same file — `card()` paints a cached poster for video when present, and
   keeps the badge when not.
7. Tests: the cache key changes with mtime and with size; a cancelled job
   stops spawning; a miss never blocks the paint thread.

**C — the preview popup**

8. New `crates/composer/src/player.rs` — `Preview { kind, source, player }` +
   `window_ui`, mirroring `browse.rs`'s split (state + body; the Composer owns
   viewport summoning).
9. `crates/composer/src/lib.rs` — `Act::Preview(PathBuf|Record)` and
   `Act::AddToEdit`; `show_previews` beside `show_browsers`; the add path
   routes through the one import seam then `add_clip`.
10. Library + Browse cards gain the tap → preview.
11. Tests: add-to-edit appends exactly one layer and one undo removes it; a
    bare path imports once (not twice) before landing.

## 5. Boundaries

**🚫 Never touch**
- `/content` — operator data.
- **Source files in a browsed folder.** Thumbnails go to `.userdata/thumbs/`,
  never beside the operator's media. Sources are never mutated.
- The **one import seam**. "Add to edit" routes through `import_file` /
  `import_reference`; it does not grow a second door.
- `write_poster`'s ffmpeg args — it is the working poster path.

**⚠️ Ask first**
- **Any audio playback dependency** (§7.1).
- Making thumbnailing automatic (it must stay consented).
- Changing `add_clip`'s placement rule (appends to the end today).

**Out of scope**
- The matrix `resolve()` port — A rides `cl::treated` and converts later.
- Browse reach (address bar / UNC) — `wiredfront-editor-residency.md` D2.
- Editing in the popup. It previews and promotes; it is not a second editor.

## 6. Verification

- `cargo green` exits 0.
- **A:** proof shots at three sizes beside
  `docs/design/references/WORKBENCH-editor.png`;
  the feel bar is the gate — **glow marks the active thing and nothing else**,
  every other control a quiet plate.
- **B:** add a real folder of ~50 videos → the prompt states the count →
  accept → browsing stays responsive while thumbs fill → **second open is
  instant** (cache hit). Touch one file's mtime → only that one regenerates.
  Decline → cards show badges, nothing is written, the offer returns.
- **C:** preview a video (plays), an image (shows), a lottie (plays); Esc
  closes with nothing imported. "Add to edit" → exactly one new layer, one
  `Ctrl+Z` removes it, and the file appears in `content/` exactly once.
- **Break-it:** preview a file whose drive is pulled mid-play → an honest
  message, no hang, no panic.

## 7. Open — needed before C can finish

1. **Audio playback.** Nothing in the workspace can make sound. Options:
   - **`rodio`** — a real runtime dependency; simplest correct playback.
   - **an ADR-009 subprocess** (`ffplay`) — matches the ffmpeg pattern, but
     `ffplay` is not currently in `dptools-deps` and would have to be staged.
   - **waveform-only, no sound** — honest, ships today, and is not what was
     asked for.
   **Recommendation: `rodio`**, as a named dependency decision in
   `architecture.md`. But it is a ceiling change and therefore yours.
2. **Popup as viewport or in-app overlay?** Browse uses deferred **viewports**
   and that is the house pattern. An in-app overlay makes "easily close"
   cheaper (Esc, click-outside) and cannot get lost behind the main window.
   **Recommendation: in-app overlay for preview**, viewports staying for
   Browse — preview is transient, Browse is a workspace.

## 8. Evidence

*(filled during execution)*
