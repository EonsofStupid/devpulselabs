# 2026-10-27 — portable-storage: ST2b, unplugged mid-session

Milestone `portable-storage-st2`, pass ST2b. Plan:
`docs/plans/portable-storage.md` §4e steps 5–10, with the amendments made
while building it (after step 10). Checkout **`D:\projects\devpulselabs`**,
branch `feat/portable-storage-st2b`, PR #301. ST2a landed as #300
(`32e3746`); the exe was rebuilt from it (`32e3746 · 2026-10-27 13:03z`).

## ST2b — what landed

- **`dpl-roots` (`guard.rs`):**
  - `State::Unplugged { last, reason }`, so the path functions return
    `last` instead of panicking;
  - `writable(path)`: an identity check at most every 500 ms, refusal while
    unplugged, and a rebase from earlier places this session;
  - `io_failed`, `is_unplugged`, `generation`, `library` and `label`.
- **Writers:**
  - an installed guard (`set_write_guard`, `WriteGuard`) in `dpl-doc`,
    `dpl-comp`, `dpl-workflows` (through `dpl_doc::writable`) and
    `dpl-ledger`, which holds rows that can't be written;
  - direct `dpl_roots::writable` calls in the index, the chat stores,
    Imagine, ViziWiz, Composer, Present, the room library, the backfill and
    `ChromeChoices`;
  - the seam's render asks `dpl_comp::writable`.
- **`dpl-estate`:** the stores sit behind a `RwLock<Option<Stores>>`, so
  `reopen` keeps every `Arc` holder working.
- **`dpl-shell`:** `LibraryNotice::{Unplugged, Moved}`, on a top-layer
  strip with Restart now; `library_back` reaches the hubs.
- **`dpl-hub`:** a default `Hub::library_back`. The Workbench saves the
  card editor and lessons again.
- **The app:** `App::watch_library` each frame; `RESTART` spawns the exe
  again after `run_native` returns.

## Exact next step

1. Land #301 on double-green, rebuild the exe from main, verify the stamp.
2. Then **ST5, Locations** (R288): write its §4 queue against the real
   files first. Ground `composer/src/browse.rs`, whose `libraries.json`
   holds absolute paths; they don't follow a drive-letter change.

## Landmines

- **The `subst` proof** (`tests/eguipilotai/st2-unplugged.toml`): mount
  `Q:` before the run, start the background timeline (12 / 20 / 28 / 36 s
  from the app's start), and **remove `R:` after every run**. Re-prep
  `target/st2-proof/` each time (the session scratchpad's `st2b_prep.sh`),
  then write `roots.toml` with `last_path = 'Q:\library'`.
- **Shell heredocs mangle backslashes in Python code.** Write the scripts
  with the Write tool and open files with `newline=''`.
- **A sheet as tall as the window sits under the banner's strip** (the
  card editor's Done in the Moved state). This is recorded as a known
  limit in §4e.

## Verify commands

```sh
cd /d/projects/devpulselabs
git status --short && git log --oneline -4 && gh pr list --state open
cargo test -p dpl-roots && cargo test -p dpl-estate
subst   # expect nothing listed
```
