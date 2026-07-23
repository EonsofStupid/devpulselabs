# L1d — the Shows rail entry (per-show pages)

Parent: docs/plans/l1-library-doctrine.md (§2, §5.1, §6). Ledger #118.
Worktree: `C:\localdev\dpl-main` on main, after L1c (80f7887).

## 1. Context

The operator: *"we want then to have on the left hand side a new rail
Shows — under that will be EffingAI, DevPulse News, Eons of Stupidity.
The content used for social media related posts, all of that properly
organized under here with their gallery, and organization."* Ratified
(§5.1): **a show's page = Episodes · Gallery · Social**; social's disk
home is `content/<show>/social/`. X3's content tree finally becomes
VISIBLE and navigable per show.

## 2. Current state (ground-truthed this session)

- No Shows presence anywhere; Present's picker is the only window into
  `content/`. Rail: Home · Present · Workbench · wiredFRONT · Chat.
- `dpl_doc`: `list_shows(root)`, `list_content_episodes(root)`,
  `load_episode(&dir)` (→ `.title`), `episode_slides(&dir, &ep)`
  (→ `.chapter`, `.name`, `.dir`).
- `dpl_engine_capture::content_root()` — the root (workbench:561 uses
  it).
- `HubSpec.subroutes: &'static [SubRoute]` — static; main.rs already
  leaks CLI route strings ("a harness-only allocation" precedent), so a
  hub MAY leak boot-discovered subroutes once at construction.
- Thumbnail law (wiredfront:515): `file:///` with THREE slashes,
  backslashes → forward.
- Tone Violet + glyph squares-four freed by L1c's dissolution.
- On disk today: `content/devpulse-news/how-ai-works/` (one real
  episode); no effingai/eons-of-stupidity folders yet.

## 3. Target design

- **New crate `crates/hubs/shows` (dpl-hub-shows)** — deps: egui,
  dpl-hub, dpl-coldlight, dpl-doc, dpl-engine-capture.
- **Boot-dynamic subroutes, leaked once**: the three RATIFIED stream
  categories always list (EffingAI · DevPulse News · Eons of
  Stupidity — ids `effingai`, `devpulse-news`, `eons-of-stupidity`),
  then any OTHER show found by `list_shows` at boot appends (kebab id
  as label). A new folder needs an app restart — honest boot-discovery,
  no shims. `ShowsHub::new()` builds and `Box::leak`s the SubRoute
  slice + a HubSpec (tone Violet, new `cl::icon::SHOWS` = squares-four).
- **The show page** (route sub = the show id): header (label + counts),
  a chip row `episodes · gallery · social` (per-show section state in a
  HashMap keyed by show id), sections:
  - EPISODES: one card per `list_content_episodes` entry of this show —
    episode id, its `episode.json` title, chapter·slide count via
    `episode_slides`. Empty → "no episodes yet — clone a seed in
    Workbench → Episodes".
  - GALLERY: every media file under `content/<show>/` walked
    recursively (images png/jpg/jpeg/webp/avif/gif render as thumbs via
    the file:/// law; videos mp4/mov/mkv/webm render an honest label
    card), wrapped grid, rel-path captions. Machine files
    (episode.json, slide.json, *.meta.json, events.jsonl) skipped.
  - SOCIAL: rows over `content/<show>/social/` (name + kind); empty →
    "no social content yet — files under content/<show>/social/ appear
    here" (the ratified disk home, stated on the surface).
- **Registration**: rail order Home · **Shows** · Present · Workbench ·
  wiredFRONT · Chat (the doctrine map §3). Root + app Cargo entries.
- Cross-hub jumps (tap an episode → Present/Editor) need a fabric
  navigate event — deliberately NOT this pass (S1's EventLog match is
  exhaustive; a new ShellEvent variant is its own small pass).

## 4. Changes, in execution order

1. **edit `crates/coldlight/src/lib.rs`** — `pub const SHOWS: &str =
   "\u{E464}";` (squares-four, freed) beside the other rail glyphs.
2. **create `crates/hubs/shows/Cargo.toml`** (the wiredfront pattern,
   deps per §3).
3. **create `crates/hubs/shows/src/lib.rs`** — `ShowsHub::new()`
   (discover + leak spec), `Hub` impl dispatching every subroute to
   `show_page(ui, show_id, label)`; the three section fns; unit tests:
   ratified categories always present + in order, a discovered show
   appends, `media_kind` classification, machine files skipped.
4. **edit root `Cargo.toml`** — member + workspace dep.
5. **edit `crates/app/Cargo.toml`** — dep.
6. **edit `crates/app/src/main.rs`** — register
   `ShowsHub::new()` after Home.
7. `cargo green`; proofs (§6); commit; push backgrounded.

## 5. Boundaries

🚫 No fabric changes, no cross-hub navigation events (future pass).
🚫 Present/Workbench/wiredFRONT untouched. 🚫 Nothing writes under
`content/` — the Shows hub is READ-ONLY. ⚠️ Leaked subroutes are a
once-per-boot allocation (the main.rs precedent) — never per-frame.

## 6. Verification

- `cargo green` exits 0 (new tests included).
- `--route shows/devpulse-news --mood 0 --screenshot
  docs/proof/l1d-show-page.png --shot-frame 60` — the show page:
  EPISODES section with the how-ai-works card (title + counts).
- `--open-rail shows --route shows/effingai --mood 0 --screenshot
  docs/proof/l1d-rail-shows.png --shot-frame 90` — SIX houses, Shows
  unfolded listing EffingAI · DevPulse News · Eons of Stupidity;
  EffingAI's page shows the honest no-episodes state.

## 7. Commit plan

`feat(shows): L1d — the Shows rail (per-show Episodes · Gallery ·
Social)` — new crate + icon + registration + 2 proofs. Push
backgrounded.

## 8. Executed — evidence

Execution matched the plan (one fix mid-pass: `MediaKind` doesn't sort,
`walk_media` sorts by path only; one polish: the empty-state's "→"
glyph isn't in the UI face — swapped for "›"). Green; the three new
tests pass (ratified order pinned, discovered show appends, media/
machine-file classification). Proofs: docs/proof/l1d-show-page.png —
DevPulse News: "1 episode · content/devpulse-news", chips, the
how-ai-works card ("How AI actually works · 3 chapters · 4 slides");
docs/proof/l1d-rail-shows.png — SIX houses, Shows under Home unfolded
(EffingAI selected · DevPulse News · Eons of Stupidity, violet mood
strip), EffingAI's honest no-episodes state.
