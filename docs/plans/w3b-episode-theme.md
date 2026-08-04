# W3b — episode Theme: the cascade + the folder contract

Parent: docs/plans/authoring-architecture.md (layer 3) and roadmap.md.
Runs BEFORE D2 so the DeckHost is built over the final document shape.
Worktree: `C:\localdev\dpl-main` on main. Ledger #106 prerequisite.

## 1. Context

The operator (2026-07-22): *"the final destination of that styling be
properly cleanly colocated with the slide per folder auto growing as we
adhere to strict patterns."* Ratified shape (checked against how PPTX
masters/themes, Keynote, and Slidev actually work):

- **Cascade in** — an episode gets an authored `theme.json` (a trait built
  from the generated ingredients, refs only). This is the slide-master
  pattern: shared styling lives at the show level.
- **Sparse overrides inline** — a slide states only what differs from the
  show look, in its own `slide.json`. No separate per-slide style file
  (no dominant real-world deck format has one; a parallel file keyed to
  block ids is drift waiting to happen).
- **Two levels, no more** — episode look → slide override. No chains.
- **The folder contract** — reserved filenames per concern, unknown files
  flagged by the doctor, so "auto-growing" stays strict.

Why now: D2 (DeckHost) consumes slides wholesale. Baking the cascade in
first means the deck never learns a pre-cascade shape it must unlearn.

## 2. Current state (all files read in full this session)

- **`crates/doc/src/lib.rs`** (694 lines): `Slide { v, template,
  background: Background, blocks, enter: Option<String> }` — background
  REQUIRED; `Block::Text { id, text, font: String, size: String,
  ink: String }` — font/ink REQUIRED. `validate` checks every ref against
  the live registries (fonts/moods/fluid) plus W1 allowlists
  `INKS`/`AMBIENCES`/`BLEND_MOTIONS`/`ENTERS = ["rise"]`. `save` and
  `save_episode` each hand-roll the SAME atomic tmp→bak→swap dance
  (lines 111–128 and 296–313) — duplicated; a third copy for theme.json
  would be a smell. Episode tier: `Episode`/`Chapter`/`SlideRef`,
  `load_episode`/`save_episode`/`episode_slides`/`validate_episode`
  (dup chapter ids, empty, dangling, nested slide issues path-prefixed,
  dup slide refs). No folder-contract check: unknown files and orphan
  slide folders pass silently.
- **`crates/slides/src/document.rs`** (363 lines): the ONE renderer.
  `render`/`render_traced(ui, rect, &Slide, now, t0)` resolve refs inline
  with degrade guards (`family_of` → Proportional, `size_of` → body,
  `ink_of` → BASE, unknown mood → labeled plate). `rising = slide.enter
  == Some("rise")`. `DocSlide { slide, clock }` wraps a doc with its own
  clock; `DocSlide::ui` calls `render`.
- **`crates/editor/src/lib.rs`** (721 lines): `Editor { dir, slide, clock,
  selection, undo, saved }`. `render_traced(...)` at line 122. Trays read
  concrete `font`/`ink` strings (`text_fields` returns a 4-tuple of
  Strings); `set_text` mutates the tuple; `apply_theme_preset` writes preset refs
  into background + every block font. 4 on-disk tests incl. the marquee
  one-ref-diff.
- **`crates/hubs/workbench/src/lib.rs`** (436 lines): seeds wall builds
  `DocSlide::new(slide)`; `EpisodeView::open` loads each slide into
  `DocSlide::new`; editor picker lists `list_seeds()` ONLY — episode
  slides cannot be opened in the editor today.
- **`crates/coldlight/tokens/presets.json`**: 5 look presets (font, 2
  moods, motion, ambience?) + 3 motion presets. NOTE the motion presets'
  `enter` refs are `enter-rise`/`enter-fade`/`enter-bloom` — a different
  vocabulary than the slide-level `ENTERS = ["rise"]`. Pre-existing
  inconsistency; this pass does NOT touch it (noted for the motion pass).
- **`seeds/episodes/how-ai-works/`**: episode.json (3 chapters) + 4 slide
  folders, each slide fully styled inline. `seeds/slides/` holds the 4
  standalone starters, also fully styled.
- Serde note: all existing `slide.json` files carry every field, so
  making fields `Option` with `#[serde(default)]` is loading-compatible —
  **no schema version bump needed**; `SCHEMA_V` stays 1.

## 3. Target design

**The document folder contract** (written here, enforced by the doctor):

```
seeds/episodes/<id>/
  episode.json      structure: chapters → slide subfolder names
  theme.json         the show's authored trait — refs only (OPTIONAL;
                    absent = the built-in fallback look)
  <slide>/
    slide.json      structure + content + SPARSE style overrides
    (reserved for later passes: ink.json D3, notes.md D4, assets/ X)
seeds/slides/<id>/
  slide.json        standalone starters — stay FULLY specified
                    (self-contained by design; no episode to inherit from)
```

**The Theme** — the trait tier, colocated with the show:

```json
{ "v": 1,
  "font": "michroma",
  "ink": "base",
  "background": { "kind": "mood-blend",
    "moods": ["terracotta-dusk", "solar-flare"],
    "motion": "blend-drift" },
  "enter": "rise" }
```

**The cascade (exactly two levels):** for each stylable field, slide
value if present, else look value. Fields that become optional on the
slide: `background`, `enter`, and per-block `font` + `ink`. `size` stays
REQUIRED on the block — it is structural (an eyebrow is not an h1), not
a show-wide default. `enter: "none"` is added as an explicit "this slide
is static" override (otherwise a slide could never opt OUT of the look's
entrance). `Background::None` already expresses "bare well" explicitly,
so `background` absent = inherit, `"kind": "none"` = deliberately bare.

Resolution is a pure function in `dpl_doc` — `resolve(&Slide, &Theme) ->
Slide` returning a fully-specified document — so the cascade is unit-
testable data logic, and the renderer stays ONE path: it resolves once
at the top of `render_traced` and paints exactly as today. (Cost: one
small clone of a few strings per frame — the renderer already clones
each block's text into `painter.layout` every frame; same order.)

`Theme::fallback()` is the built-in default (known-good refs, proven by a
test against the live registries) used wherever no authored look exists:
standalone seeds, episodes without a theme.json.

**Editor legibility (THE UI LAW):** trays show the RESOLVED value;
tapping a card writes an override (`Some(...)`) exactly as today; a new
**"Show look" chip** in the Font row, Ink row, and Background tray
clears the override (back to `None` on disk) — "this one's different —
put it back" in plain words. Editing text on a sparse slide must NOT
densify styling (serde `skip_serializing_if` keeps absent fields absent).

**Doctor grows the contract:** `validate_episode` additionally flags
(a) a theme.json that names unknown refs, (b) unknown files in the
episode folder or a slide folder (reserved names allowed), (c) leftover
`.tmp`/`.bak` transients, (d) an orphan slide folder no chapter
references.

**Scale story:** restyling a whole show = editing ONE theme.json. A new
slide in a show costs structure + words only — zero styling. A future
concern (ink, notes, assets) claims its reserved filename and the
contract grows by one line, not by renegotiation.

## 4. Changes, in execution order

### Step 1 — `crates/doc/src/lib.rs` (edit)

**1a. Shared atomic writer** — replace the duplicated bodies of `save`
and `save_episode` with one private helper; both become one-liners:

```rust
/// Crash-safe pretty-JSON+LF write: full content lands in a sibling tmp,
/// then swaps in (Windows cannot rename over an existing file, so the
/// old file steps aside as .bak; the bak is removed on success).
fn save_json<T: Serialize>(dir: &Path, file: &str, value: &T) -> Result<(), DocError> {
    std::fs::create_dir_all(dir).map_err(DocError::Io)?;
    let target = dir.join(file);
    let tmp = dir.join(format!("{file}.tmp"));
    let bak = dir.join(format!("{file}.bak"));
    let mut json = serde_json::to_string_pretty(value).map_err(DocError::Parse)?;
    json.push('\n');
    std::fs::write(&tmp, json).map_err(DocError::Io)?;
    if target.exists() {
        let _ = std::fs::remove_file(&bak);
        std::fs::rename(&target, &bak).map_err(DocError::Io)?;
    }
    std::fs::rename(&tmp, &target).map_err(DocError::Io)?;
    let _ = std::fs::remove_file(&bak);
    Ok(())
}

pub fn save(dir: &Path, slide: &Slide) -> Result<(), DocError> {
    save_json(dir, FILE, slide)
}
pub fn save_episode(dir: &Path, ep: &Episode) -> Result<(), DocError> {
    save_json(dir, EPISODE_FILE, ep)
}
```

**1b. Sparse Slide/Block** — exact field changes:

```rust
pub struct Slide {
    pub v: u32,
    pub template: String,
    /// None = inherit the show look's background.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Background>,
    pub blocks: Vec<Block>,
    /// None = inherit; "none" = explicitly static; "rise" = entrance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enter: Option<String>,
}
```

`Block::Text`: `font: String` → `font: Option<String>` and
`ink: String` → `ink: Option<String>`, both
`#[serde(default, skip_serializing_if = "Option::is_none")]`.
`text`, `id`, `size` unchanged. `ENTERS` becomes `&["rise", "none"]`.

**1c. The Theme tier** — new, after the episode tier:

```rust
// ── the Theme tier (W3b) — the show's authored trait ─────────────────────

const LOOK_FILE: &str = "theme.json";

/// The show-level styling trait — `<episode-dir>/theme.json`. Refs only,
/// like every document. Slides inherit these unless they override.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    pub v: u32,
    /// Default font pool id for every text block.
    pub font: String,
    /// Default semantic ink for every text block.
    pub ink: String,
    pub background: Background,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enter: Option<String>,
}

impl Theme {
    /// The built-in default — used when no theme.json is authored
    /// (standalone seeds, showless slides). Known-good refs; a test
    /// proves they resolve through the live registries.
    pub fn fallback() -> Self {
        Self {
            v: SCHEMA_V,
            font: "spacegrotesk".into(),
            ink: "base".into(),
            background: Background::MoodBlend {
                moods: vec!["ember-drift".into(), "deep-lagoon".into()],
                motion: "blend-drift".into(),
                ambience: None,
            },
            enter: Some("rise".into()),
        }
    }
}

/// Load `<dir>/theme.json`; Ok(None) when none is authored (= fallback).
pub fn load_theme(dir: &Path) -> Result<Option<Theme>, DocError> {
    let path = dir.join(LOOK_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(DocError::Io)?;
    let theme: Theme = serde_json::from_str(&raw).map_err(DocError::Parse)?;
    if look.v > SCHEMA_V {
        return Err(DocError::Version(look.v));
    }
    Ok(Some(look))
}

pub fn save_theme(dir: &Path, theme: &Theme) -> Result<(), DocError> {
    save_json(dir, LOOK_FILE, look)
}

/// The look a slide folder resolves against: its parent episode's
/// authored look, else the fallback. (A folder is "in a show" when the
/// PARENT holds an episode.json.)
pub fn theme_for_slide(slide_dir: &Path) -> Theme {
    slide_dir
        .parent()
        .filter(|p| p.join(EPISODE_FILE).is_file())
        .and_then(|p| load_theme(p).ok().flatten())
        .unwrap_or_else(Theme::fallback)
}

/// THE CASCADE — exactly two levels: slide override, else look. Pure;
/// returns a fully-specified document the renderer paints as-is.
pub fn resolve(slide: &Slide, theme: &Theme) -> Slide {
    let mut out = slide.clone();
    if out.background.is_none() {
        out.background = Some(look.background.clone());
    }
    if out.enter.is_none() {
        out.enter = look.enter.clone();
    }
    for b in &mut out.blocks {
        let Block::Text { font, ink, .. } = b;
        if font.is_none() {
            *font = Some(look.font.clone());
        }
        if ink.is_none() {
            *ink = Some(look.ink.clone());
        }
    }
    out
}

/// Name every unknown ref in a look. Empty = clean.
pub fn validate_theme(theme: &Theme) -> Vec<Issue> { /* font via FONTS,
    ink via INKS, background via the same arm as validate's, enter via
    ENTERS — factor the Background arm out of validate into a private
    fn validate_background(bg: &Background, push) so both use it. */ }
```

**1d. `validate` goes present-only** — every check wraps in
`if let Some(...)`; absent fields are legal (inherit). The Background
match moves to the shared `validate_background` helper (1c).

**1e. Folder contract in `validate_episode`** — after the existing
walk, add:

```rust
// The folder contract: reserved names only; everything else is flagged
// so growth stays strict. (Transients mean a crashed save — worth naming.)
const SLIDE_RESERVED: &[&str] = &["slide.json", "ink.json", "notes.md", "assets"];
const EPISODE_RESERVED: &[&str] = &["episode.json", "theme.json"];
```

- Episode dir scan: files not in `EPISODE_RESERVED` → `Issue { path:
  "folder", message: "unexpected file {name:?} (contract: episode.json,
  theme.json, slide folders)" }`; `.tmp`/`.bak` → "leftover transient
  {name:?} from an interrupted save". Subdirectories containing a
  slide.json but referenced by no chapter → "orphan slide folder
  {name:?} — not in any chapter".
- Each referenced slide dir scan: entries not in `SLIDE_RESERVED`
  (same transient carve-out) → flagged, path-prefixed as today.
- Theme: `load_theme(dir)` → parse/version errors become an Issue
  ("theme.json does not load: {e}"); loaded looks run `validate_theme`,
  issues prefixed `look.`.

**1f. Tests** (extend the existing module; existing tests updated for
the Option fields — `sample()` keeps every field `Some(...)` so the
round-trip/validate tests keep their meaning):

- `the_fallback_theme_resolves_through_the_live_registries` —
  `validate_theme(&Theme::fallback())` is empty.
- `theme_round_trips_and_absent_theme_is_ok_none`.
- `resolve_prefers_the_slide_override_and_fills_the_rest` — a slide with
  only `font: Some("orbitron")` on one block: that block keeps orbitron,
  the other inherits, background/enter/ink come from the look; a fully-
  specified slide resolves to itself unchanged.
- `a_sparse_slide_saves_without_the_absent_fields` — on-disk JSON
  contains no `"background"` / `"font"` / `"ink"` keys for absent ones.
- `a_v1_fully_specified_file_still_loads` — the pre-W3b shape (every
  field present) parses identically; no version bump.
- `the_folder_contract_names_strays_orphans_and_transients` — a rogue
  `styles.css` in a slide folder, an unreferenced slide folder, and a
  `slide.json.tmp` are each flagged; reserved names are not.
- `a_bad_theme_ref_is_named_via_the_episode_doctor` — theme.json with
  font "comic-sans" → `look.` -prefixed issue.
- Update `episode_doctrine_names_dangling_dup_and_empty` /seed tests as
  needed for the new checks (seed episode must stay clean — step 5).

### Step 2 — `crates/slides/src/document.rs` (edit)

- `render` and `render_traced` gain the look:

```rust
pub fn render(ui: &mut Ui, rect: Rect, slide: &dpl_doc::Slide,
              theme: &dpl_doc::Theme, now: f64, t0: f64) {
    let _ = render_traced(ui, rect, slide, look, now, t0);
}

pub fn render_traced(ui: &mut Ui, rect: Rect, slide: &dpl_doc::Slide,
                     theme: &dpl_doc::Theme, now: f64, t0: f64) -> RenderMap {
    let slide = &dpl_doc::resolve(slide, look);
    // body unchanged except the Option unwraps below
    ...
}
```

- Inside the (now fully-resolved) body: `slide.background` is
  `Option<Background>` — the `background()` helper takes
  `Option<&Background>`; `None` paints as `Background::None` does today
  (defensive only; resolve always fills it). Block destructuring:
  `font`/`ink` are `Option<String>` — `font.as_deref().unwrap_or("")`
  feeds `family_of` (unknown/empty already degrades to Proportional),
  `ink.as_deref().unwrap_or("base")` feeds `ink_of`. Entrance:
  `let rising = slide.enter.as_deref() == Some("rise");` — unchanged
  (an explicit `"none"` is simply not `"rise"`).
- `DocSlide` carries the look:

```rust
pub struct DocSlide { pub slide: dpl_doc::Slide, look: dpl_doc::Theme, clock: Clock }
impl DocSlide {
    pub fn new(slide: dpl_doc::Slide) -> Self { Self::with_theme(slide, dpl_doc::Theme::fallback()) }
    pub fn with_theme(slide: dpl_doc::Slide, look: dpl_doc::Theme) -> Self { ... }
    // ui() passes &self.theme through to render.
}
```

- Tests: existing ones pass `&dpl_doc::Theme::fallback()`; add
  `a_sparse_slide_renders_with_the_theme_styling` — a slide with no
  background and a font-less block renders through a look without
  panicking (the degrade+cascade proof at the paint layer).

### Step 3 — `crates/editor/src/lib.rs` (edit)

- `Editor` gains `look: dpl_doc::Theme`; `open` sets
  `look: dpl_doc::theme_for_slide(dir)` — a slide inside an episode
  folder automatically edits against its show's look; a standalone seed
  gets the fallback. Render call passes `&self.theme`.
- `text_fields` returns the RESOLVED values plus which are overridden:
  change the return to
  `Option<(String, String, String, String, bool /*font override*/, bool /*ink override*/)>`
  — resolved via `b.font.clone().unwrap_or_else(|| self.look.font.clone())`
  (same for ink). `set_text`'s tuple mutators now write `Some(...)`:
  the closure signature stays `(String, String, String, String)`; inside
  `set_text`, write-back becomes `*font = Some(t.1)` etc. Tapping a
  card = an override, exactly as today.
- **"Show look" chips** (the cascade made legible):
  - Font row: after the font cards, `if chip(ui, "Show look", !font_overridden)`
    → `set_text` clears: add `fn clear_font(&mut self, id)` /
    `clear_ink(&mut self, id)` using `mutate` to set the field `None`.
  - Ink row: same chip after the ink chips.
  - Background tray: a "Show look" chip before the colour rows —
    `mutate(|s| s.background = None)`; the tray's current-mood readout
    resolves through the look when background is None.
  - Enter is not in any tray yet (no motion tray until the motion-preset
    pass) — nothing to add.
- `apply_theme_preset` (the W3 preset apply) now writes `Some(...)` everywhere
  it writes — applying a preset to a slide is an explicit override of
  the whole slide, unchanged in spirit.
- Tests: existing 4 updated for the Option shape (seed() keeps full
  `Some` styling — their assertions keep their meaning, comparing
  `font.as_deref()`). Add:
  - `editing_text_on_a_sparse_slide_does_not_densify_styling` — save a
    slide whose block has `font: None`; edit only the text; the on-disk
    JSON still has no `"font"` key.
  - `show_theme_clears_the_override_on_disk` — override a font, then
    clear via `clear_font`; on-disk block has no `"font"` key and the
    editor's resolved value equals the look's font.

### Step 4 — `crates/hubs/workbench/src/lib.rs` (edit)

- `EpisodeView::open`: load the show's look once —
  `let theme = dpl_doc::load_theme(dir).ok().flatten().unwrap_or_else(dpl_doc::Theme::fallback);`
  — and build rows with `DocSlide::with_theme(slide, theme.clone())`.
- Seeds wall: unchanged (`DocSlide::new` = fallback; standalone seeds
  stay fully specified anyway).
- Editor picker gains a **Shows section** so episode slides are editable
  where the cascade actually lives: after the seeds list, for each
  `list_episodes()` entry, a small header (the episode title) then a
  card per `episode_slides(&dir, &ep)` entry (label
  `"{chapter} · {name}"`), opening `dpl_editor::Editor::open(&r.dir)` —
  the editor auto-detects the episode look via `theme_for_slide`.

### Step 5 — migrate the seed episode (edit 5 JSON files by hand)

- **Create** `seeds/episodes/how-ai-works/theme.json`:

```json
{
  "v": 1,
  "font": "michroma",
  "ink": "base",
  "background": {
    "kind": "mood-blend",
    "moods": ["terracotta-dusk", "solar-flare"],
    "motion": "blend-drift"
  },
  "enter": "rise"
}
```

- **Sparsify the 4 slide.json files** under `seeds/episodes/how-ai-works/`:
  delete every field equal to the look's value; keep every deliberate
  difference as an explicit override (e.g. the stat slide keeps its
  `rajdhani` font and glacier background; `"ink": "tone"` / `"strong"`
  accents stay). The exact per-file diffs fall out mechanically from
  the current files vs the look above; the doctor + tests prove the
  result (validates clean, renders identically).
- `seeds/slides/*` untouched — standalone starters stay fully specified
  by design (§3).

### Step 6 — `docs/plans/authoring-architecture.md` (edit, CORRECTIONS LAW)

Append the look tier to the layer-3 section: the folder contract block
from §3 of this plan, the two-level cascade rule, and "standalone seeds
stay fully specified" — so the architecture doc and the code never
disagree.

## 5. Boundaries

- 🚫 `seeds/slides/*` content unchanged; `crates/coldlight` unchanged
  (presets.json, build.rs, ThemePreset all stay as-is).
- 🚫 No third cascade level, no per-slide theme.json, no template-level
  styling tier (that conversation belongs to Program T).
- 🚫 The `enter-rise`/`rise` vocabulary mismatch between motion presets
  and slide `enter` is NOT resolved here (noted §2; motion-preset pass).
- 🚫 No D2 work; no schema version bump (change is additive-optional).
- 🚫 Codex seat's branch/checkout untouched; work only in
  `C:\localdev\dpl-main` on main (`git branch --show-current` first).
- ⚠️ Renderer signature change touches every caller — the grep-verified
  caller set is: editor line 122, `DocSlide::ui`, document.rs tests.
  Anything else that appears at compile time gets repointed, no shims
  (pre-release law).

## 6. Verification

1. `cargo green` exits 0 (fmt, clippy -D warnings, ALL tests incl. the
   new doc/renderer/editor ones, release build, exe mtime advanced).
2. The seed episode's doctrine is clean post-migration:
   `the_seed_episode_loads_validates_and_flattens` passes with the
   folder-contract checks active (proves theme.json + sparse slides +
   no strays).
3. Cascade proof at the data layer: `resolve` tests (override wins,
   inheritance fills, full slide is a fixed point).
4. Sparse-stays-sparse proof at the editor layer: the two new editor
   tests assert the ON-DISK absence of cleared/never-set fields.
5. Real exe: `--route workbench/episodes` screenshot
   `docs/proof/w3b-episodes.png` — the show renders visually identical
   to the D1 proof (the cascade resolves to the same styling the inline
   values produced). `--route workbench/editor` screenshot
   `docs/proof/w3b-editor-shows.png` — the picker shows the Shows
   section; open an episode slide, the Font row shows the "Show look"
   chip. (Live tap-flow verification still awaits the pilot gesture
   extension — same honest deferral as W4.)

## 7. Commit plan

One commit on main, then a backgrounded push:

```
feat(doc): W3b — episode Theme: the two-level cascade + the folder contract

An episode now carries an authored theme.json (the show's trait — refs
only); slides state only their deliberate overrides. resolve() is the
pure two-level cascade; the renderer resolves once and paints as before.
The doctor enforces the folder contract (reserved names, orphans,
transients, look refs). Editor shows resolved values with "Show look"
to clear an override; picker gains the Shows section. Seed episode
migrated (theme.json + sparsified slides); standalone seeds stay fully
specified by design.
```

`git add`: `crates/doc/src/lib.rs`, `crates/slides/src/document.rs`,
`crates/editor/src/lib.rs`, `crates/hubs/workbench/src/lib.rs`,
`seeds/episodes/how-ai-works/**`, `docs/plans/w3b-episode-looks.md`,
`docs/plans/authoring-architecture.md`, `docs/proof/w3b-*.png`.
