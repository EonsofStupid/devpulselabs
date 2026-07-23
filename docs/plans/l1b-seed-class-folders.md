# L1b — seeds into class folders

Parent: docs/plans/l1-library-doctrine.md (§1.3, §6). Ledger #118.
Worktree: `C:\localdev\dpl-main` on main, after L1a (rebased onto the
Codex EguiPilotAI slice).

## 1. Context

Ratified (doctrine §1.3): *"Clear seed folders = BY TEMPLATE CLASS:
`seeds/slides/static/`, `split/`, `animation/`, `interactive/` — the
registry's own classes; the library wall's sections ARE the folders."*
The operator: *"Clear seed folders i had this in the original app."*
Today `seeds/slides/*` is flat. Pre-release: the layout MOVES, the walk
is rewritten, the compiler surfaces every consumer — no shims.

## 2. Current state (ground-truthed this session)

- Disk: `seeds/slides/{headline-clean,stat-callout,terminal-retro,
  title-neon}/slide.json` — flat. Doc `template` fields: title ×3,
  stat ×1 — every one a static styling card (no split/animation/
  interactive seed exists yet).
- `crates/slides/src/registry.rs`: `TemplateClass{Static,Split,
  Animation,Interactive}` with `label()` returning exactly
  `static|split|animation|interactive` — the vocabulary the folders
  restate. dpl-slides depends on dpl-doc (never the reverse).
- `crates/doc/src/lib.rs:679` `list_seeds() -> Vec<(String, PathBuf)>`
  — one-level read_dir filtering `FILE` (slide.json). `seeds_root()` =
  `find_upward("seeds/slides")`. `Issue{path,message}` already exists
  (the validator's shape). Test `every_seed_loads_and_validates_clean`
  (line 947) destructures the tuple.
- Consumers of `list_seeds`: `crates/hubs/workbench/src/lib.rs:106`
  (`load_seeds` → the SEEDS wall; local `SeedEntry{id,dir,doc,issues}`)
  and `:555` (the editor picker, `for (id, dir)`);
  `crates/slides/src/document.rs:515`
  (`every_seed_renders_through_the_one_path`, destructures the tuple).
- `--clone-seed` uses `list_episodes` (seed EPISODES) — untouched by
  this pass.

## 3. Target design

- Disk: `seeds/slides/<class>/<id>/slide.json`. All four existing seeds
  are static cards → `seeds/slides/static/`. `split/`, `animation/`,
  `interactive/` are created with `.gitkeep` so the class tree is
  visible in the repo from day one.
- `dpl_doc::SEED_CLASSES: [&str; 4] = ["static","split","animation",
  "interactive"]` — the doc layer restates the registry taxonomy as
  folder law (the `kebab_ok` precedent: doc never depends on slides).
  A NEW slides-side test pins the two vocabularies in sync.
- `dpl_doc::Seed { class: &'static str, id: String, dir: PathBuf }`;
  `list_seeds() -> Vec<Seed>` walks the class folders in SEED_CLASSES
  order, ids sorted within a class; a missing class folder is simply
  empty. A slide.json folder sitting DIRECTLY under `seeds/slides/`
  (the old flat layout) is NOT listed — the doctor names it instead.
- `dpl_doc::seed_tree_issues() -> Vec<Issue>`: names (1) a top-level
  entry that is not a class folder — an UNCLASSED seed (folder holding
  slide.json) or an unknown folder name; (2) nothing else — files like
  .gitkeep inside class folders are fine.
- Workbench: the SEEDS wall groups by class (a small `CLASS — n` header
  per non-empty class, the wall's flat cards under each) and surfaces
  `seed_tree_issues()` when non-empty; the editor picker labels seed
  cards `class · id`. (The full sectioned Library wall is L1c.)

## 4. Changes, in execution order

1. **Disk moves** (git mv, copy-never-lose): the four seed folders →
   `seeds/slides/static/<id>/`; create `seeds/slides/{split,animation,
   interactive}/.gitkeep`.
2. **edit `crates/doc/src/lib.rs`** — replace `list_seeds` with the
   class walk (+ `SEED_CLASSES`, `Seed`, `seed_tree_issues`; doc
   comments name the doctrine). Update test
   `every_seed_loads_and_validates_clean` to the new shape + assert
   every listed class ∈ SEED_CLASSES; add
   `seeds_list_by_class_and_flat_layout_is_refused` (scratch tree:
   classed seed listed, flat seed unlisted + named by the doctor) and
   `seed_tree_issues_is_quiet_on_the_real_tree`.
3. **edit `crates/slides/src/registry.rs`** — test
   `the_folder_law_matches_the_registry_taxonomy`: every
   `TemplateClass::label()` ∈ `dpl_doc::SEED_CLASSES` and lengths match
   (4 = 4). (dpl-slides already depends on dpl-doc.)
4. **edit `crates/slides/src/document.rs:515`** — destructure `Seed`
   (`s.id`, `s.dir`).
5. **edit `crates/hubs/workbench/src/lib.rs`** — `load_seeds` carries
   `class`; the SEEDS wall renders per-class headers + tree issues;
   the editor picker labels `class · id`.
6. `cargo green`; proof (§6); commit; push backgrounded.

## 5. Boundaries

🚫 `seeds/episodes/` untouched (a different tier). 🚫 No Library-hub
dissolution, no Workbench/Library wall — L1c. 🚫 Seed slide.json
contents byte-identical (moves only). ⚠️ The doc→slides dependency
direction must not flip (SEED_CLASSES lives in doc for that reason).

## 6. Verification

- `cargo green` exits 0 (new + updated suites).
- Real-exe proof: `--route workbench/seeds --mood 0 --screenshot
  docs/proof/l1b-seed-classes.png --shot-frame 60` — the wall shows the
  STATIC class header over the four seed cards (and no unclassed-seed
  issues).

## 7. Commit plan

`feat(seeds): L1b — class folders (static/split/animation/interactive)`
— moves + doc walk + doctor + consumer updates + proof. Push
backgrounded.

## 8. Executed — evidence

Execution matched the plan, one refinement: `list_seeds`/`seed_tree_issues`
split into root-parameterized internals (`*_in(root)`) so the doctor
tests run on scratch trees instead of the real one. Green: all suites
pass — `seeds_list_by_class_and_flat_layout_is_refused` (classed seed
lists, flat seed refused + named "unclassed seed", mystery folder named
"unknown folder"), `seed_tree_issues_is_quiet_on_the_real_tree`,
`the_folder_law_matches_the_registry_taxonomy` (slides ↔ doc vocab
pinned). Proof: docs/proof/l1b-seed-classes.png — the SEEDS wall shows
"4 documents", the STATIC — section header, and the moved
`seeds/slides/static/<id>/slide.json` paths; zero doctor warnings.
