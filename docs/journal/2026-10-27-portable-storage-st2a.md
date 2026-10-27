# 2026-10-27 — portable-storage: ST2a, a library that isn't connected

Milestone `portable-storage-st2`, pass ST2a. Plan:
`docs/plans/portable-storage.md` §4e, approved 2026-10-27 as two PRs. The
rulings: a new letter mid-session → "Save, then Restart now"; no heartbeat,
so an unplug is noticed only when something saves or fails; `subst` for
proof. Checkout **`D:\projects\devpulselabs`**, branch
`feat/portable-storage-st2a`, PR #300.

## Before this pass, the same session

- ST1b (#299) merged on double-green as `2d75c27`; the exe was rebuilt from
  it. The operator's move (Bring in my existing library… → Move it) is
  still owed.
- ST2's grounding: a census of every `dpl_roots::` call the app can reach
  (22 files). Its findings are §4e's "What the reading found", and they
  shape ST2b.

## ST2a — what landed

- **`dpl-roots`:**
  - `disk::replace_config` (`.new`, then a rename) and the `CONFIG_WRITES`
    counter;
  - `Roots::remember`, which writes a linked library's new path;
  - `reconnect()` and `watch()` (a thread that probes every 2 s);
  - `status()`, `locate()` and `link_to()`;
  - `Publish::{New, Replace}`, threaded through `setup_imported`,
    `setup_linked` and `bring_in`, which now take the program folder and
    only write. The caller opens the library with `initialize`.
- **One window (`main.rs`):**
  - `Root` with the stages `Library` (the `Startup` pages), `Opening` and
    `App`;
  - `open_library()` (the LS2 upgrade);
  - `pilot_runtime(cc)` at creation, and `App::adopt_pilot`;
  - the headless drills exit 2 without a library.
- **The Disconnected page (`storage_startup.rs`):**
  - it names the library and shows where it was last seen;
  - Locate… (Check, then "Open 'X'" for another library);
  - Use a library on this machine (the first screen's choices, with
    `Publish::Replace`, and a ‹ Back);
  - pilot ids under `library.away/*`.
- **Proof:**
  - 34 `dpl-roots` tests and 5 headless page tests;
  - the `st2-disconnected` scenario passed twice;
  - shots in `docs/proof/portable-storage-st2a/`.

## Exact next step

1. Land #300 on double-green, rebuild the exe from main, verify the stamp.
2. Then **ST2b** (§4e steps 5–10). Begin with step 5:
   - `State::Unplugged { last, reason }`, whose path functions return
     `last` instead of panicking;
   - `writable(path)`;
   - `io_failed`.

   Then step 6's census list, then the banner, then kept edits, then the
   reconnect. Ground `dpl-estate`'s `reopen` against fjall 3 before
   relying on it.

## Landmines

- **Python on Windows writes CRLF** in text mode. Open files with
  `newline='\n'` (or `newline=''` to keep what's there); `cargo fmt`
  repairs `.rs` files (`newline_style = "Unix"`).
- **`pilot_track` replaces a widget's accessible label** with the name it
  is given. Give it the visible text, or kittest's `get_by_label` and
  screen readers lose it.
- **`CONFIG_WRITES` is process-wide.** Tests that write configurations in
  parallel count against `reconnect_with`, so its test retries.
- **The ST2a proof** runs in `target/st2-proof/` (prep: the session
  scratchpad's `st2a_prep.sh`). Mount `Q:` with
  `subst Q: D:\projects\devpulselabs\target\st2-proof\drive` and **remove
  it after every run** (`subst Q: /d`).
- **`shell-navigation.toml` is stale.** It looks for a "Test Lab" rail item
  that no longer exists; that failure has nothing to do with ST2a.
- `cargo test -p devpulselabs` rebuilds `target/debug/devpulselabs.exe`
  without devtools. Rebuild with `--features eguipilotai-devtools` before a
  pilot run.

## Verify commands

```sh
cd /d/projects/devpulselabs
git status --short && git log --oneline -4
gh pr list --state open
cargo test -p dpl-roots && cargo test -p devpulselabs storage_startup
subst   # expect nothing listed
```
