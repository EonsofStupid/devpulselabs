# W1 — dpl-doc: the slide document crate + the seed slides

Parent: docs/plans/authoring-architecture.md (ratified). Repo clean at the
UI-law commit. This pass makes the DOCUMENT layer real: slide-as-folder,
refs-never-values, atomic write-back — and authors the first seeds.
Rendering is W2; nothing visual changes this pass.

## 1. Context

The editor (W4) edits a colocated `slide.json` per slide folder. That file
needs a schema, a loader/saver, a validator, and real seed instances to
exist first. Operator requirements bound here: "each slide is its own
folder where it has the structure, content"; edits target the colocated
file; refs never values.

## 2. Current state (ground-truthed this session)

- No document crate exists. `crates/slides` has 18 registered TemplateIds
  (title, stat, callout, …) with hand-authored demos.
- Registries enumerable at runtime: `cl::FONTS` (13 ids), `cl::MOODS`
  (10 names). Fluid token NAMES are collected in `generate_fluid()` but
  only the count is emitted — the names list must be emitted for size-ref
  validation.
- serde/serde_json are not workspace runtime deps (serde_json is a
  coldlight build-dep only).
- Workspace members are listed explicitly in the root Cargo.toml.

## 3. Target design

- **`crates/doc` (dpl-doc)** — depends on serde/serde_json/dpl-coldlight
  only. No egui: the document layer is pure data.
- Schema v1 (serde, tagged enums, kebab-case):
  `Slide { v, template, background, blocks, enter? }`;
  `Background::MoodBlend { moods[2], motion, ambience? } | Mood { mood } |
  None`; `Block::Text { id, text, font, size, ink }`.
  Style values are REGISTRY REFS: font=pool id, mood=mood name,
  size=fluid type name minus `fs-` ("display"), ink ∈
  strong|base|muted|subtle|tone.
- `load(dir)` / `save(dir, &slide)` — colocated `slide.json`, pretty JSON +
  trailing LF; save is crash-safe (tmp file → swap; Windows rename-over
  needs the bak dance).
- `validate(&slide) -> Vec<Issue>` — flags unknown refs (degrade-never-
  crash: the W2 renderer plates unknowns; the validator names them).
  Fonts/moods/sizes check the REAL registries; ink/ambience/motion/enter
  check documented W1 allowlists (replaced by registry checks as W2/W3
  wire them — noted in code).
- `find_upward(rel)` + `seeds_root()` + `list_seeds()` — same exe-dir/CWD
  upward walk the asset loaders use (documented duplication: dpl-doc must
  not depend on dpl-video).
- **4 seeds** under `seeds/slides/<id>/slide.json`: title-neon,
  stat-callout, terminal-retro, headline-clean — each a distinct
  font/mood-blend/ambience composition from the pool.
- coldlight: `generate_fluid` additionally emits
  `pub static FLUID_TOKEN_NAMES: &[&str]`.

## 4. Changes, in execution order

1. Root Cargo.toml: member `crates/doc`; workspace deps
   `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`.
2. coldlight build.rs `generate_fluid`: emit the names array (one line,
   uses the existing `names` vec). Unit test asserts it contains
   "fs-display".
3. Create `crates/doc/Cargo.toml` + `src/lib.rs` (schema, io, validate,
   seeds discovery) + tests: save→load identity; validator flags unknown
   font/mood/size and a non-2 mood-blend; every seed loads + validates
   clean; save leaves no tmp droppings.
4. Author the 4 seed folders.
5. `cargo green`; commit.

## 5. Boundaries

- 🚫 No rendering, no editor UI, no slides-crate changes (W2/W4).
- 🚫 seeds/ is repo content (git-tracked product); `content/` stays
  operator data.
- Out of scope: presets (W3), media blocks in the schema (added with the
  first media seed in W2 — schema enums are #[non_exhaustive]-free and
  additive by design).

## 6. Verification

`cargo green` exits 0 with the new tests (doc round-trip, validator,
seeds-clean, fluid-names). No exe-visual change to prove this pass;
the wall proof lands with W2.

## 7. Commit plan

`feat(doc): W1 — slide-as-folder document crate + the first four seeds`.
Adds crates/doc, seeds/slides/*, coldlight names emission, workspace deps.
No push.
