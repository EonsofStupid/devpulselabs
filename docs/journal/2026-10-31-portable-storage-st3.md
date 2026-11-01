# 2026-10-31 — portable-storage: ST3, your library moves where you choose

## Before this pass

- **Main is `711fc72`** (#309, ST5b), on top of `bbac4f5` (#311: the repo is
  public, and CI runs on GitHub-hosted runners).
- **Main is protected:**
  - PRs, both checks (`green-windows`, `green-linux`) and resolved
    conversations are required, admins included;
  - the branch must be up to date with main.
- **Hosted lanes from cold caches** took 47 min (Linux) and 64 min
  (Windows) on #309.
- **The release exe was rebuilt** from `711fc72` (stamp
  `711fc72 · 2026-11-01 02:21z`).

## Rulings (2026-10-31)

- **Q-ST3-1, where "Move my library…" lives:** "A Library cell in the
  status bar".
- **Q-ST3-2, when the copy runs:** "Restart, copy, then open".
- **The old copy:** the question named it as "kept, renamed and never
  deleted".

## The plan

`docs/plans/portable-storage.md` §4g, steps 1–5. It was grounded on
`0dc211d` by a read-only census, and its file:line references are in §4g.
The design:
- `plan_switch` and `switch` in `dpl-roots` keep the library's id.
  - Everything is copied except `common/temp`, then checked byte for byte.
  - Locations are rebased; the marker is published last; `roots.toml` is
    replaced.
  - The old copy is retired: `library.json` → `library.moved.json`, and
    the folder is labelled `…moved-to-…`.
- The Library cell and card in `dpl-shell`.
- "Move it" writes `move.toml` beside `roots.toml`, then Restart now. At
  launch, a "Moving your library" page runs the move before anything opens
  the library.

## Exact next step

§4g step 1, the engine. Read `crates/roots/src/migrate.rs` (copy helpers)
and `setup.rs` (`publish_identity`, `write_config`, `mode_for`). Then add
the switch beside `migrate.rs`, moving the shared helpers so both moves use
them, without a re-export.

## Landmines

- **`initialize` is not for switching open stores mid-session**
  (`roots/src/lib.rs:164`). That is why the copy runs at launch.
- **An old copy with the same id can be found again** by the drive-letter
  probe, Locate… or setup. Retiring its marker is what prevents it.
- **Locations stored with `..`** resolve somewhere else after a move, so
  rebase them in the copy before publishing.
- **The real-exe proof needs a scratch exe copy and `--no-seam`**, like the
  `st2-portable` profile: a seam never writes `roots.toml`.
- **Run proof-shot exes sequentially**, each with a scratch library. Never
  open the operator's real library.
- **`cargo test` outside the gate needs `CONTENT_ROOT`** set to a scratch
  folder.

## Verify commands

- `git log --oneline -1 origin/main` should be `711fc72` or later. Run
  `gh pr list` first, because parallel sessions move main.
- `grep -ao "[0-9a-f]\{7\} · [0-9-]* [0-9:]*z" target/release/devpulselabs.exe`
  should print `711fc72 · …`.
- `grep -n "4g. ST3" docs/plans/portable-storage.md` finds the plan.
