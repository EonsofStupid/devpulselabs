# 2026-10-27 — portable-storage: ST5, Locations (started)

Milestone `portable-storage-st5`. Plan: `docs/plans/portable-storage.md`
§4f, approved 2026-10-27 with the rulings Q-ST5-1/2/3 in the Status block.
Checkout **`D:\projects\devpulselabs`**, branch
`feat/portable-storage-st5a`, from main `050bdee` (#301).

## Before this pass

- **ST2 is done:**
  - ST2a (#300, `32e3746`): one window, the Disconnected page, the watch,
    Locate….
  - ST2b (#301, `050bdee`): `writable` guards, the banner, kept edits, the
    estate reopened in place, Restart now.
  - The exe was rebuilt from `050bdee` (09:47:19).
- **Owed by the operator:** the move (Bring in my existing library…) and a
  real USB unplug/replug check.

## ST5a — the queue (§4f steps 1–6)

1. **`dpl-roots` `locations.rs`:**
   - `.userdata/locations.json` holds an id, a label, a path (relative to
     the library when on its drive) and a sample of file names and sizes
     (Q-ST5-1: nothing is written into the folder);
   - `add`, `list` (online/offline), `locate` (sample match, or the person
     confirms), `remove` (the record only);
   - `resolve("locations/<id>/…")` wired into `locate_asset`;
   - an in-memory cache, so the per-frame resolve doesn't read the file;
   - a watch that probes other drive letters only while some location is
     offline.
2. **Index:** linked rows keyed `locations/<id>/…` with
   `location = "linked:<id>"`; a recursive scan per location; the
   `rel_path_of` absolute fallback removed.
3. **Posters** go in `common/cache/locations/<id>/…`, never into the
   operator's folder.
4. **`slides/src/elements/media.rs`:**
   - offline is told apart from missing;
   - authoring shows the poster plus "On 'X' — plug it in";
   - Run shows the poster only (Q-ST5-3).
5. **The surfaces:**
   - Browse's "★ Add as library" becomes "Add as a location";
   - Composer's "your packs" becomes Locations, with online/offline,
     Locate… and Remove;
   - pilot ids and tooltips on the new controls.
6. **Proof:**
   - unit tests;
   - a `subst` drive of short videos added as a location, picked into a
     card, unplugged (the poster shows) and back on another letter (it
     plays);
   - shots at both sizes.

ST5b (steps 7–9) comes after: migrate `sources.json` and `libraries.json`
once, `--pin-pack` becomes `--add-location`, and import by reference
becomes a location reference (no symlink, no silent copy).

## Landmines

- **`subst` proofs:** remove every letter after a run (`subst` should list
  nothing).
- **Python edits:** write the scripts with the Write tool and open files
  with `newline=''`. Shell heredocs mangle backslashes, and text mode
  writes CRLF.
- **Media resolve is per frame:** `locate_asset` is called every frame from
  slides and backdrops, so location resolution must be in memory.

## Verify commands

```sh
cd /d/projects/devpulselabs
git status --short && git log --oneline -3 && gh pr list --state open
subst   # expect nothing listed
cargo test -p dpl-roots
```
