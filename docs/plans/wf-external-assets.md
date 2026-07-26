# WF — external assets in place: reference imports + cross-platform libraries

Program: wiredFRONT media intake QoL. The operator's paid AE presets / image
folders must be reachable and usable WITHOUT duplicating them into the app.
Ground-truthed 2026-07-26 on Linux/aarch64 (`crates/composer`).

## 1. Context

Browse + import were a walled garden: (a) every pull **copied** into
`content/`, doubling large preset libraries; (b) `browse::roots()` was
Windows-only (`USERPROFILE` + `A:`–`Z:`), so on Linux/macOS a Browse window
had nowhere to go. Gate: `wiredfront-experience.md` friction rows for media
bin / organize — effortless pull of the operator's own libraries.

## 2. Current state (ground-truthed 2026-07-26)

- ONE import seam: `composer::import::import_file` — copy + poster + index
  (`import.rs`). Pack shelf pins folders in place (W3) but individual file
  pulls always copied.
- Folder-symlink under `/content` already REJECTED (W3/W4 — rescan blow-up).
- `browse::roots()` — Windows drives + USERPROFILE only; empty on unix.
- No persistent "my folders" list beyond themeforest pack pins
  (`elements/sources.json`).

## 3. Target design

1. **`import_reference(src)`** — single-FILE symlink into `content/inbox` or
   `content/elements/referenced/`, indexed like any artifact (`source=
   "reference"`). Shared `finalize_import` with copy path. Fallback to copy
   when the OS denies symlinks (Windows without privilege). Never folder
   symlinks.
2. **Persisted preference** — `.userdata/prefer-reference` sentinel; Project
   panel chip `⧉ copy` ⇄ `⌁ in place`; `handle_import` branches so Explorer
   drops AND Browse pulls honor the mode.
3. **Cross-platform `roots()`** — home standard folders (`HOME` /
   `USERPROFILE`; `Movies` = macOS Videos), Windows drives, unix mounts
   (`/Volumes` `/mnt` `/media` `/`).
4. **Operator libraries** — `★ Add as library` in Browse; persisted
   `.userdata/libraries.json`; shown FIRST in every Browse quick-jump bar.

## 4. Changes (executed)

1. `import.rs` — `import_reference`, `symlink_file` (unix/windows),
   `finalize_import`; unit `reference_symlinks_the_source_in_place`.
2. `lib.rs` — `prefer_reference` + Project chip + `handle_import` branch.
3. `browse.rs` — cross-platform `roots()`, `operator_libraries` /
   `add_library` / `remove_library`, `★ Add as library` chip; unit
   `roots_are_populated_cross_platform`.

## 5. Boundaries

- No pack-shelf rewrite; pins stay `sources.json`.
- No folder symlinks into content.
- No assets/-ring migration; intake stays content/.
- No AE parsing — `.aep` remains an element artifact.

## 6. Verification

- `cargo green` (with in-repo `target/` — Cursor's injected
  `CARGO_TARGET_DIR` under `/tmp` breaks `locate_asset`'s 4-level walk).
- Unit: symlink reference; roots populated on unix.
- Drill (recorded in commit): `--import-drill` in reference mode lands
  `content/inbox/<name>` → real source via symlink.
- `green-linux` on the PR before squash-merge.

## 7. Commit plan

Branch `feat/wf-external-assets`: (1) reference import + toggle;
(2) cross-platform roots + operator libraries; (3) plan + rustfmt.
PR → green-linux → squash-merge → tip STATE.

## 8. Evidence (2026-07-26)

- Reference mode end-to-end: `--import-drill` with prefer-reference ON →
  `content/inbox/mypreset.png` is a symlink resolving to the source
  (commit `feat(composer): reference AE presets/images IN-PLACE`).
- Browse roots non-empty on Linux; `★ Add as library` persists under
  `.userdata/libraries.json` (commit `feat(composer): browse ANY folder
  cross-platform + add your own libraries`).
- Tests: `reference_symlinks_the_source_in_place`,
  `roots_are_populated_cross_platform`.
