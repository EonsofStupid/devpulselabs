# D1 — episode documents: the tier over slide folders

Parent: docs/plans/roadmap.md Program D (the show machine). Ledger #106.
Worktree: main. First pass of the Deck/Present runtime — the DATA tier the
DeckHost (D2) will drive. No deck UI this pass.

## 1. Context

W1/W2 gave us slide documents (each slide its own folder + slide.json,
refs-never-values, rendered by the one document renderer). A SHOW is a
sequence of those slides grouped into chapters. D1 adds the EPISODE tier —
`episode.json` naming an ordered chapter→slide structure over slide
subfolders — plus its validator and a seed episode rendered on a wall
(gallery gate). D2 then builds the DeckHost that navigates it.

## 2. Current state (ground-truthed this session)

- `dpl_doc`: Slide/Background/Block schema, load/save/validate (per slide
  folder), Issue, find_upward/seeds_root/list_seeds. SCHEMA_V=1.
- 4 slide seeds under seeds/slides/*.
- Old-repo episode.json (census/deck-raw): id/slug/show, chapters, slides[]
  with beat/steps; doctrine = chapter-lock + ≤3 interactive + 15–24 band +
  dup/dangling integrity. We port the STRUCTURE now; show-kind doctrine
  becomes authored data in a later pass (noted, not dropped).

## 3. Target design

**An episode is a FOLDER:**
```
seeds/episodes/<episode-id>/
  episode.json          the structure (refs to slide subfolders)
  <slide-name>/slide.json   each slide, the existing Slide document
```

`episode.json` (serde, versioned):
```json
{ "v": 1, "id": "how-ai-works", "title": "How AI actually works",
  "chapters": [
    { "id": "intro", "title": "The setup", "slides": ["title", "headline"] },
    { "id": "cost",  "title": "What it costs", "slides": ["stat"] }
  ] }
```

- `Episode { v, id, title, chapters: Vec<Chapter> }`,
  `Chapter { id, title, slides: Vec<String> /* subfolder names */ }`.
- `load_episode(dir)` / `save_episode(dir, &ep)` — colocated episode.json,
  same atomic tmp→bak→swap + LF as slides; refuses a newer `v`.
- `validate_episode(dir, &ep) -> Vec<Issue>` — the structural doctrine
  (platform-free, the old doctor's core): duplicate chapter ids, empty
  episode/chapter, each slide ref resolves to `<dir>/<name>/slide.json`
  that loads AND validates clean (its issues bubble up path-prefixed),
  duplicate slide ref within the episode. Show-kind doctrine (chapter-lock,
  budgets) = a later authored-doctrine pass; a TODO names it.
- `episode_slides(dir, &ep) -> Vec<SlideRef>` where
  `SlideRef { chapter, name, dir, index }` — the FLAT ordered walk the deck
  (D2) consumes; step-0..N grammar rides on top in D2.
- `episodes_root()` = find_upward("seeds/episodes"); `list_episodes()`.
- **Seed episode** `seeds/episodes/how-ai-works/` with 3 chapters reusing
  copies of the 4 slide docs (title/headline/stat/terminal) as slide
  subfolders — a real, validate-clean show.
- **Workbench EPISODES subroute**: list episodes; for the selected one,
  render its slides in order as a labeled 16:9 strip (chapter headers +
  each slide live via the document renderer) — the gallery gate for
  episode documents. Read-only (editing = W-track / D2).

Scale story: an episode adds no per-slide code — it references slide
folders that already render through the one path; a new show is a folder +
JSON.

## 4. Changes, in execution order

1. `crates/doc/src/lib.rs` — Episode/Chapter/SlideRef, load/save/validate
   episode, episode_slides, episodes_root/list_episodes; tests: round-trip,
   newer-version refusal, dangling/dup/empty all named, a nested bad slide
   bubbles path-prefixed, the seed episode loads+validates clean and
   flattens to the right ordered slide list.
2. Author `seeds/episodes/how-ai-works/episode.json` + 4 slide subfolders
   (copied slide.json from the existing seeds, retoned into a coherent
   show).
3. `crates/hubs/workbench` — EPISODES subroute + wall (episode picker →
   ordered slide strip).
4. `cargo green`; exe proof `--route workbench/episodes`; commit; push.

## 5. Boundaries

🚫 No deck navigation / reveal steps / presenter (D2+). 🚫 No editing of
episodes (read-only wall). 🚫 No show.json tier or show-kind doctrine yet
(authored-doctrine pass). 🚫 Codex branch/worktree untouched. seeds/ is
git-tracked product.

## 6. Verification

green exits 0 incl. the episode tests (the seed episode flattens to the
authored slide order; a doctored dangling ref is named). Real exe:
`--route workbench/episodes --screenshot docs/proof/d1-episodes.png` shows
the seed episode's chapters + slides rendered in order.

## 7. Commit plan

`feat(doc): D1 — the episode document tier + a seed show + episodes wall`
on main, pushed.
