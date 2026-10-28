# 2026-10-27 — portable-storage: ST5, Locations (ST5a built)

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

## ST5a — built (PR #303)

Steps 1–6 landed on `feat/portable-storage-st5a`, rebased onto #302 (a
parallel session's ST2 follow-through: estate write guard, preference
retries). The amendments and §8 evidence are in the plan. The next step is
**ST5b** (§4f steps 7–9): migrate `sources.json` and `libraries.json` once,
remove `--pin-pack`, and make import by reference produce a location link.

## ST5a — landed

#303 merged as `e60c140` on double-green (head `b661733`: green-linux
2m4s, green-windows 8m26s). The release exe was rebuilt from it
(2026-10-27 16:30:25, stamp `e60c140 · 2026-10-27 20:28z`).

## ST5a′ — asked 2026-10-28, built

The operator, after hearing that a drive pulled mid-session only showed on
the shelf:

> it should be aware if its been started and stopped as well whether linux
> arm or windows or mac please

The plan's §4f ST5a′ (L1–L4) was amended first, on
`feat/portable-storage-st5a-live` from `e60c140`. The changes:
- `dpl-roots` keeps the last look (a snapshot plus a change count), and
  lists and painters read it.
- One app-wide watch, started in `Root::open`, looks every 2 s.
- Composer's own watch is deleted. Its list follows the count and says
  what changed.
- Drives are found through the kernel's mount table on Linux (any
  architecture), under `/Volumes` on macOS, and by the shape of the path
  when the drive is away.

Evidence is in the plan's §8 (ST5a′), and the shots are in
`docs/proof/portable-storage-st5a-live/`.

**New landmine:** a `subst` pilot scenario launched from a background shell
didn't see the letter. Run those proofs in the foreground.
