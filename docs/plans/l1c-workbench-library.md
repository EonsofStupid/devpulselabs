# L1c — Workbench/Library rebuilt; the Library hub dissolves

Parent: docs/plans/l1-library-doctrine.md (§1.1, §6). Ledger #118.
Worktree: `C:\localdev\dpl-main` on main, after L1b (a5e6a65).

## 1. Context

Ratified (doctrine §1.1): *"The Library hub DISSOLVES into Workbench →
Library. One ingredient home: seeds, templates, blocks, elements, and
the vocabulary walls (presets · moods · fonts · color pool) become
sections of Workbench's Library. The top-level hub leaves the rail."*
The operator: *"Workbench/library is where I can access all of the raw
ingredients using that editor to turn them into tailored pieces."*
Pre-release: the hub crate is DELETED, not shimmed.

## 2. Current state (ground-truthed this session)

- `crates/hubs/library/src/lib.rs` (read in full): `LibraryHub{demos,
  focus}` + `set_focus`; five walls — `templates_gallery` (stateful:
  live demos, tap-to-solo focus, replay), `pool_wall`, `presets_wall`,
  `fonts_wall`, `moods_wall` (stateless fns over cl registries).
  Subroutes: components/templates/moods/fonts/presets.
- Its ONLY consumers: `crates/app` (Cargo dep, `main.rs:219-221`
  `LibraryHub::default()` + `--focus` → `set_focus`, `:240` registration
  in the hub vec).
- `crates/hubs/workbench/src/lib.rs`: subroutes SEEDS(default) · EDITOR
  · EPISODES · STUDIO · LIBRARY(placeholder) · TOKENS(placeholder) ·
  TEST_LAB. `seeds_wall` (L1b class sections + tree doctor),
  `editor_surface`, `episodes_surface`; `ui()` dispatches, `else` →
  placeholder. Cargo already deps dpl-slides (DocSlide).
- `default_route` = the FIRST subroute (dpl_hub lib.rs:58).
- `--session-drill` frame-60 hop navigates to `workbench::SEEDS`
  (main.rs:500-503).
- Workspace members list carries both hub crates (root Cargo.toml).

## 3. Target design

- **Workbench subroutes become LIBRARY(default) · EDITOR · EPISODES ·
  STUDIO · TEST_LAB** — the doctrine pipeline order (ingredients →
  tailor → compose → studio). SEEDS and TOKENS dissolve (TOKENS was an
  empty placeholder; its job — tones/spacing visualized — is already
  covered by the pool/presets/fonts/moods sections).
- **`crates/hubs/workbench/src/library.rs` (new module)** — THE
  ingredient library: a section chip row
  `SEEDS · TEMPLATES · COLORS · PRESETS · FONTS · MOODS`
  (`LibSection` enum, default Seeds) dispatching to:
  - SEEDS: the L1b class-sectioned seeds wall (moved verbatim from
    `seeds_wall`, header re-worded "SEED SLIDES — the raw ingredients");
  - TEMPLATES: the live gallery + tap-to-solo focus + replay (moved
    from the Library hub; demos lazy-init `Option<Vec<Box<dyn
    SlideTemplate>>>` so `#[derive(Default)]` survives);
  - COLORS / PRESETS / FONTS / MOODS: the four stateless walls, moved
    verbatim (tone refs change Violet→SPEC amber automatically since
    they read the local SPEC).
- **`WorkbenchHub`** gains `lib_section`, `demos`, `focus` +
  `pub fn set_focus(&mut self, id)` (sets focus AND
  `lib_section = Templates`). `seeds`/`tree_issues` fields stay (used
  by the SEEDS section).
- **`crates/hubs/library` is DELETED** (git rm): workspace member out,
  app Cargo dep out, registration out (rail: 5 houses). `--focus`
  repoints to `workbench.set_focus`; the drill's frame-60 hop repoints
  `SEEDS` → `LIBRARY`.
- Blocks/elements sections: FUTURE (Z7-era work) — the chip row leaves
  room; not invented in this pass.

## 4. Changes, in execution order

1. **create `crates/hubs/workbench/src/library.rs`** — `LibSection`
   enum + `chip_row`; move `templates_gallery` (as a method taking
   `&mut Option<Vec<...>>` + `&mut Option<String>` or a small
   `TemplatesView` state struct), `pool_wall`, `presets_wall`,
   `fonts_wall`, `moods_wall` from the library hub (code verbatim
   except SPEC/tone references now the workbench SPEC).
2. **edit `crates/hubs/workbench/src/lib.rs`** — `mod library;`
   SPEC subroutes → LIBRARY(first) · EDITOR · EPISODES · STUDIO ·
   TEST_LAB; delete SEEDS/TOKENS consts + their SubRoutes; `ui()`
   dispatch: LIBRARY → `library_surface` (chip row + section dispatch;
   SEEDS section calls the existing seeds-wall body); add
   `lib_section/demos/focus` fields + `set_focus`. Module doc updated.
3. **edit `crates/app/src/main.rs`** — drop `dpl_hub_library` (use,
   default, set_focus, registration); `--focus` → workbench.set_focus;
   drill hop 60 → `dpl_hub_workbench::LIBRARY`; comment updates.
4. **edit `crates/app/Cargo.toml`** — remove dpl-hub-library dep.
5. **edit root `Cargo.toml`** — remove the member + workspace dep
   entry.
6. **git rm -r `crates/hubs/library`**.
7. `cargo green`; proofs (§6); commit; push backgrounded.

## 5. Boundaries

🚫 The walls' rendering logic moves VERBATIM — no visual redesign in
this pass. 🚫 No Shows rail entry (L1d), no session playback move
(L1e). 🚫 EDITOR/EPISODES/STUDIO/TEST_LAB surfaces untouched. ⚠️ The
editor picker's seed listing (L1b `class · id`) stays as is.

## 6. Verification

- `cargo green` exits 0.
- `--route workbench/library --mood 0 --screenshot
  docs/proof/l1c-library-seeds.png --shot-frame 60` — the chip row +
  SEEDS section with the STATIC class header.
- `--focus title --route workbench/library --mood 0 --screenshot
  docs/proof/l1c-library-focus.png --shot-frame 60` — the TEMPLATES
  section solo'd on the title template ("title" = TitleCard's id).
- `--open-rail workbench --route workbench/library --mood 0
  --screenshot docs/proof/l1c-rail.png --shot-frame 90` — the rail
  shows FIVE houses (no Library), Workbench unfolded WITHOUT
  Seeds/Tokens, Library first.

## 7. Commit plan

`feat(workbench): L1c — the ingredient Library; the Library hub
dissolves` — new module + workbench/app/Cargo edits + crate removal +
3 proofs. Push backgrounded.

## 8. Executed — evidence

Execution matched the plan (walls moved verbatim; `templates_gallery`
became a free fn over `&mut [Box<dyn SlideTemplate>]` + `&mut
Option<String>` — no state struct needed). Green first run. Proofs:
docs/proof/l1c-library-seeds.png (chip row + SEEDS section, STATIC
header); docs/proof/l1c-library-focus.png (« Title card · static ·
replay — the live template solo'd via `--focus title`);
docs/proof/l1c-rail.png (FIVE houses — Library gone from the rail;
Workbench unfolded showing Library·Editor·Episodes·Studio·Test Lab —
Seeds and Tokens dissolved; the accordion + the new Library surface in
one frame).
