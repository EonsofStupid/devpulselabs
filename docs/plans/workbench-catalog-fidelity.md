# Cycle 2 — Workbench catalog fidelity

## 1. Context

Operator ask: Library shows real categories, templates, overlays/blocks —
not hollow placeholders. Gated by `experience-inventory.md` (authored this
pass). Cadence: plan → implement → EguiPilot → squash-merge → tip STATE.

## 2. Current state (read in-session)

- Library at `workbench/library`; 7 chips; no Blocks.
- 18 live templates via `dpl_slides::demos()`; flat gallery.
- Seeds: 4 static; split/animation/interactive folders empty — no headers.
- EguiPilot `[fixtures.library]` still `--route library/templates` (stale).
- Blocks paint exists: `dpl_slides::blocks::{eyebrow,badge,stat,ambient_*}`.

## 3. Target design

1. Inventory doc (done companion).
2. `LibSection::Blocks` + `blocks_wall` live swatches of existing paint fns.
3. `templates_gallery` sectioned by `TemplateClass` (empty Animation honest).
4. `seeds_wall` always emits all `SEED_CLASSES` headers + empty copy.
5. Fix fixture → `workbench/library` + `--focus title`; scenario + proof PNG.

## 4. Changes (execution order)

1. Docs: inventory + this plan.
2. `library.rs` / `lib.rs`: Blocks chip + walls; class sectioning.
3. `.eguipilotai.toml` + `tests/eguipilotai/workbench-library.toml`.
4. Green → PR → evidence §8 → tip STATE.

## 5. Boundaries

- Out: new template renderers; Elements chip; Studio fidelity; W10; seed
  authoring into empty classes; inventing animation demos.
- ColdLight tokens only; tooltip law on new chips/controls.

## 6. Verification

1. `cargo test -p dpl-hub-workbench -p dpl-slides` (touched) + `cargo green`.
2. EguiPilot `devpulse-cloud` scenario green + proof PNG.
3. Empty classes show honest copy; unknown `--focus` still degrades.

## 7. Commit plan

Branch `feat/workbench-catalog-fidelity`. Squash-merge + STATE tip.

## 8. Evidence

- Inventory: `docs/plans/experience-inventory.md`.
- Blocks chip + live swatches; Templates/Seeds sectioned by class with
  honest empty copy; EguiPilot fixture fixed to `workbench/library`.
- `cargo clippy -p dpl-hub-workbench --all-targets -- -D warnings` clean.
- EguiPilot `devpulse-cloud` `tests/eguipilotai/workbench-library.toml`
  **passed** — run `run-925585-18c6012f97460325`; proof
  `docs/proof/workbench-library-templates.png`.
- `env -u CARGO_TARGET_DIR cargo xtask green` ✔.
