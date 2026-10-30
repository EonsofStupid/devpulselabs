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

## ST5a′ — landed

#304 merged as `855f674` on double-green (head `865ebc4`: green-linux
1m59s, green-windows 10m17s). The release exe was rebuilt from it
(2026-10-28 15:41:45, stamp `855f674 · 2026-10-28 19:39z`).

## ST5b — ruled 2026-10-29, built

Grounded on `bfc8e33` (#305, a parallel session's PC1a). The operator's
`content/elements/sources.json` pins `D:\webwork\themeforest\video` and
`…\audio`. Neither is on any drive letter today, so they move in offline.
Rulings:
- Q-ST5b-1, "Put everything on the shelf";
- Q-ST5b-2, "The whole folder".

Built per §4f B1–B5:
- locations wait to be indexed;
- `adopt_old_lists` runs at `open_library`;
- one app-wide indexer (Composer's `index_waiting`, driven by the app)
  works one location at a time;
- the pack shelf, Pin pack, Browse libraries and `--pin-pack` are deleted;
- `import_reference` makes a location link.

**Found while proving and fixed:** `Index::open` set a *locked* database
aside as corrupt. Index work now takes one turn per process.

**Landmines:**
- **`subst` proofs:** run them in the foreground.
- **Pilot timing:** the app takes 1–12 s to start, depending on how warm
  the build is. Time drive changes from the lists moving in, not from the
  script's start.
- **`cargo test` outside the gate needs `CONTENT_ROOT`** set to a scratch
  folder. Without it, Composer's tests panic on "Choose where to keep your
  library".

## ST5b — the CI storage quota (2026-10-29/30)

PR #309 (head `683b380`) passed `cargo green` on both lanes, but
`green-windows` failed at `upload the windows exe` with "Artifact storage
quota has been hit".
- **First response:** the operator approved deleting old exe uploads. 19
  were deleted and 3 kept (79 MB).
- **The block stayed:** twelve attempts over 8 hours still hit the quota.
  GitHub bills private-repo storage by average use over the month, so
  deleting doesn't undo October's count.
- **The ruling:** the operator chose "Upload only from main, 1 day". #310
  merged as `e3f8c8b` on double-green (run 41081620853: upload skipped,
  both jobs success).
- **Then the repo went public (R293, same day).**
  - The estate runners were deregistered, so ST5b's rebased push had no
    runner. A hand-started run waited with nothing to pick it up and was
    cancelled.
  - #311 (`bbac4f5`, a parallel session's PR) moved both lanes to
    GitHub-hosted runners, kept #310's policy and protected main. The
    branch must be up to date with main; both checks and resolved
    conversations are required, admins included.
  - ST5b was rebased on it. The storage cap applies only to private repos,
    so it no longer applies.

**Landmine:** a push whose event GitHub drops gets no run. Check
`gh run list --branch <b>` for the new head sha before waiting on
`gh pr checks`.
