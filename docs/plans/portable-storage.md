# Portable, linked storage — the plan (ST)

**Status: APPROVED 2026-10-25** (the operator's answers, Claude Code session):
- Q1 "Everything, as one folder";
- Q2 "Local profiles";
- Q3 "AI vs app state";
- Q4 "Disconnected screen";
- **Q5 "Both"** — Locations (link folders in place) AND a clean move of the
  whole library to a bigger drive (§4.2: "Move it to a drive or folder"
  covers drive → drive too, copy → verify → label, never delete);
- Q6 "Link first, accounts last" — ST1 → ST2 → ST5 → ST3 → ST4.

**§5b signed off, all four:** `roots.toml` beside the exe; the marker check
on each drive while disconnected; `library.json` written into a linked
library; and removing the parent walk, the CWD lookup and the OS temp use.
With it, the operator defined the word (R289, 2026-10-25):

> portable means just self-reliant while being able to attune to that space
> with the understanding it will likely be opened on different machines

The design rule that follows:
- **self-reliant:** the program folder plus its library carry everything
  they need — no install, no registry, no machine-wide state;
- **attunes:** on each machine it adapts — finds its library on whatever
  drive letter it got, picks up that machine's screens, notices the
  depot's tools — without writing anything machine-specific into the
  library.

Ruled 2026-10-24 (LS4 round 2): "Plan it now, build after LS4a". It is its
own program because it changes where every document lives.

The operator's words (R287, 2026-10-24, the storage part):

> we have it so the storage is linked and the portable app just disconnects
> if on another machine but can be reconnected i.e users plugs external and
> gets it connected again.  We want to also allow them to grow the portable
> folder cleanly and decide if its linked or improted and stored with the
> whole program but have our structure clean, .userdata .aidata and the
> common ones properly organized nested clean for each account.  devpulse.app
> and this will eventually be tied into my dev version of devpulse

**R288 (2026-10-24)** sharpens it: *"I can have an external plugin and have
it effortlessly link so i can have videos, and all link efficiently to
inside my slides"*. The external drive's videos must link into slides in
place: fast (a cached poster, never a re-copy), shown as offline when the
drive is away, never broken. That makes Locations (ST5) central, not
last — see Q6. An export copies linked media in (ruled 2026-10-24,
`docs/plans/export-player.md`).

It builds on what is already ruled:
- **ADR-014**, the three-ring root: `content\` (confirmed), `assets\`
  (working) and `.userdata\` (state) as siblings, root-relative paths,
  additive migrations;
- **R83 and docs-one-truth §4.3, Locations:** existing folders join the
  library in place, show as offline when missing, with Locate… / Remove;
- **D0 DQ1:** a `roots.toml` beside the exe, with no parent walk and no CWD.

Grounded 2026-10-24 by a read-only pass over `feat/ls4a-component-engine`.

## 1. Context

- **One owner resolves the root, once.** `dpl_engine_capture::content_root()`
  (`crates/engine-capture/src/lib.rs:93-131`) is cached in a `OnceLock`. The
  order is:
  1. `--content-root`;
  2. `CONTENT_ROOT`;
  3. the first of `<exe>/content`, `content/` up to four parents, or
     `./content` that exists;
  4. otherwise it creates `<exe>/content`.

  `assets_root()` and `userdata_root()` are its siblings (`:182-223`), and
  the rings are made only at boot (`:246-267`).
- **A missing drive is never noticed.** With no flag, an unplugged drive
  makes the search **silently pick another candidate, or create an empty
  library beside the exe**. With the flag, boot only logs a warning
  (`main.rs:110`). Mid-session there is no check at all: each save fails on
  its own IO error (`import.rs:170`).
- **State is spread out.**
  - Inside `content\`: the library (slides, lessons, looks), shows,
    elements, `inbox\`, and also `composer-state.json`, which is app state in
    the confirmed ring.
  - In `.userdata\`: the index, the estate, the analytics Parquet, chat
    (`accounts.db`, `provider-profiles.db`, roles and rooms), caches,
    policies, browser profiles.
  - In `assets\`: captures.
  - Beside the exe: the tool depot, the Lottie helper, seeds.
  - Elsewhere: OS temp (`composer`, `viziwiz`) and the CWD (the provider's
    working folder, `experiments\`).
- **There is no account or user** beyond each provider's own sign-in
  evidence. `.userdata` exists as a ruled layout; **`.aidata` does not
  exist**.
- **Nothing watches drives.** There is no watcher and no volume identity.
  The only "offline" idea is pinned pack sources (`packs.rs:21-24`).
- **Folder-finders disagree.** `status.md` known-broken records four of them.
  The capture engine reads `shows/<show>/episodes/<ep>` while `dpl-doc`
  writes `<show>/<episode>`.

## 2. Goal / non-goals

**Goal:** the operator's library goes where they choose:
- **with the program** (imported: inside the portable folder);
- **linked** (on an external drive or any folder), and then:
  - started on a machine without it, the app says so plainly and waits;
  - plugging the drive back in reconnects it, on any drive letter;
  - pulled out mid-session, nothing is lost and nothing is written to the
    wrong place.

The folder is laid out cleanly per account, with app state, AI data and
creative material each in its own place. It can be switched between linked
and imported, and grown by adding existing folders in place (Locations).

**Non-goals:** devpulse.app sync or sign-in, and any network (recorded as
the direction; nothing is built toward it until planned); cloud storage
providers; sharing a library between two machines at once.

## 3. Ground truth (read 2026-10-24)

The full report, with file:line for each claim, is the grounding pass in the
session of 2026-10-24. The points that shape the design:

- The `OnceLock` root makes reconnect-on-a-new-drive-letter impossible as
  it stands. The root has to become a **handle that can change state**
  (connected → disconnected → connected elsewhere), and every IO site asks
  it.
- **13 crates depend on `dpl-engine-capture`**, most only for the roots.
  Giving the roots their own engine crate frees them from `xcap`, `image`
  and `webp`.
- **The structure audit rejects `-sys` crates** (`xtask:1461,1490-1500`), so
  reading a Windows volume serial through `windows-sys` would fail the gate.
  Recognising the drive by a **marker file inside the root** needs no new
  dependency and works the same on Linux.
- **Writes outside the rings** are named in the external-reach inventory:
  #29 (the parent walk, then CWD), #30 (the provider CWD) and #31 (OS temp).
  ST1 closes #29 and #31; #30 is the chat plan's.

## 4. Design

### 4.1 The layout (ST1)

```
<library>\                      the portable data root
  library.json                  marker: its id, schema, label, created
  common\                       shared by every account on this library
    cache\  temp\               regenerable (the deletability contract)
  accounts\<account>\
    account.json                id, name, created (local; room for a
                                devpulse.app link later, nothing more)
    content\                    Ring 2 — confirmed (unchanged inside)
    assets\                     Ring 1 — working (captures)
    .userdata\                  Ring 0 — app state: index.db, estate\,
                                settings and policies, profiles\,
                                composer-state.json (moved out of content)
    .aidata\                    what AI made or learned: chat (rooms,
                                roles, provider profiles, activation
                                evidence — never credentials), the
                                analytics ledger and backfill, the imagine
                                cache, chat memory (R275) when it lands
```

The program folder (exe, `dptools-deps\`, the Lottie helper, `seeds\`)
holds no operator data. Seeds stay embedded material (DQ7).

### 4.2 Linked or imported (ST1 + ST3)

- **`roots.toml` beside the exe (DQ1)** says where the library is:
  - `mode = "imported"`: the library is the program folder's own
    `library\`;
  - `mode = "linked"`: it is anywhere else, recorded by the marker's
    `id`, the last path it was seen at, and its label.

  There is no parent walk and no CWD lookup. `--content-root` /
  `CONTENT_ROOT` stay as the drill seams, and a seam is never written to
  `roots.toml`.
- **First launch with no `roots.toml`:** a one-screen choice — "Keep my
  library with the program" or "My library is on a drive or folder…". It
  is never chosen silently.
- **Switching (ST3):**
  - **Bring it into the program folder:** copy, verify, switch, then label
    the source `…moved-to-program`, never deleted (ADR-014 law 4).
  - **Move it to a drive or folder:** the same steps the other way.

  Each step shows the plan as a sheet first, like LS3b2's move.

### 4.3 Disconnected, then reconnected (ST2)

- **At launch, a linked library that is not there** opens the
  **Disconnected** screen. It says "Your library 'DevPULSE drive' isn't
  connected. Plug it in and it reconnects by itself." It offers
  **Locate…** (point at where it is now), **Use a library on this
  machine** (imported, or another linked one) and **Quit**.

  Nothing else opens, so nothing is written into an empty stand-in. That is
  today's silent bug.
- **Recognising the drive:** every few seconds while disconnected, the app
  looks for `library.json` with the right id:
  1. at the last path;
  2. at the same path on every other drive letter (Windows) or mount point
     (`/media/<user>/*`, `/run/media/<user>/*`, `/mnt/*`).

  It only checks those exact places — no deep scan. A match reconnects and
  updates the last path.
- **Pulled out mid-session:** the roots handle checks the marker before
  writing, and an IO failure under the root triggers the same check.
  - Saves are refused in plain words ("Your library was unplugged — plug it
    back in to save"). What is open stays open, and typing is kept in
    memory.
  - The Disconnected banner appears, and on reconnect the kept edits save.

### 4.4 Accounts (ST4)

- An **account** is a local profile: a name and an id. There is one by
  default ("Me"). More can be added, one is active at a time, and switching
  reloads everything under it.
- `common\` holds only what is safe to share: regenerable cache and temp.
- No sign-in. `account.json` leaves room to be linked to a devpulse.app
  identity when that is planned.

### 4.5 Today's folders move once (ST4)

- **A read-only dry run first** (`cargo xtask move-library --dry-run`, as
  in LS3b2). It lists every file and where it lands:
  `content\` → `accounts\me\content\`, and `.userdata\` splits into
  `.userdata\` and `.aidata\`.
- **Then the move,** shown as a sheet in the app and never at launch:
  copy, verify, label, never delete. THE checkout's
  `D:\projects\devpulselabs\{content,assets,.userdata}` is operator data,
  so it moves only on the operator's own tap.

### 4.6 Growing it: Locations (ST5)

- This is R83 as docs-one-truth §4.3 designed it. An existing folder joins
  the library in place: its path and its library-style marker are
  recorded, and it is indexed without copying.
- A missing location shows **offline**, with Locate… / Remove. Remove only
  removes the record.
- Locations replace pinned pack sources and Browse libraries (two lists
  become one).

## 5. Wiring

### 4c. ST1 execution queue — grounded 2026-10-25

The operator authorized reading this untracked plan from
`D:/projects/devpulselabs` and implementing in an isolated checkout under
`D:/localdev/devpulselabs`. Base: `530dd17` (RL #296). The source checkout
and its operator data remain untouched. No open storage PR was found.

1. Add `dpl-roots`, with only serde/serde_json/toml. Versioned
   `roots.toml` and `library.json`, validated typed library/account ids,
   imported `<program>/library` and linked paths, final account layout,
   and a stateful `Roots` handle. Test identity rather than trusting a
   remembered path. Candidate checking takes explicit paths for tests;
   production enumeration checks only the same suffix under drive/mount
   roots. No recursive scan. Ambiguous duplicate ids must not pick one.
2. Preserve CLI then environment scratch-root overrides (including sibling
   ASSETS_ROOT/USERDATA_ROOT overrides); they never write roots.toml or
   markers. Move root helpers and every caller out of engine-capture,
   with no re-export. Video reads the same resolver, never parent/CWD
   discovery. Root paths are snapshots from the handle, not separately
   cached OnceLocks. ST2 owns live store teardown/reopen and guarding all
   previously opened IO handles; ST1 does not claim hot-unplug safety.
3. Gate startup before opening stores, upgrading slides or running drills.
   No config presents the approved native first-launch choice. Missing,
   mismatched, malformed or future-version configuration must leave data
   untouched and show a recoverable startup error; never an empty fallback.
   ST2 adds automatic polling, live disconnection and retained-edit saves.
4. New libraries use `accounts/me/{content,assets,.userdata,.aidata}` and
   `common/{cache,temp}`. Existing legacy trees are only opened via the
   explicit drill/dev seam until ST4's shown migration. Move composer
   state writes into .userdata, reading old state as a fallback without
   moving/deleting it. Composer render scratch and ViziWiz freeze output
   use the common temp root. Chat/analytics migration remains ST4.
5. Add `xtask stage-roots <program-dir> <existing-library>`: validate its
   marker and create configuration without clobbering any file. Never
   adopt or migrate an unmarked legacy library. Register the crate,
   dependency ceiling and egui-free census together; amend ADR-014.
6. Verify root/config/marker/layout round trips; wrong/missing/malformed
   markers; changed drive; duplicate ids; no-write failure; no-clobber;
   explicit overrides and shared video lookup. Run formatting, relevant
   tests, structure and app checks, then real native startup shots at
   1440x900 and 1100x700 where the build environment permits. Record exact
   failures rather than claiming unrun gates or later ST passes complete.

   The workspace test gate must supply private scratch CONTENT_ROOT,
   ASSETS_ROOT and USERDATA_ROOT values to its child test processes. Existing
   UI/default-store tests previously relied on ambient repo discovery;
   removing that discovery must not make tests read an operator library.
   Direct cargo-test invocations of those consumers use the same explicit
   scratch seams. Root-engine tests inject their own contexts.

Wire fields are implementation details within the approved design:
schema `v = 1`, `mode = "imported" | "linked"`, library identity and
label, linked `last_path`, and active account `me`. Marker fields are
`v`, `id`, `label`, `created`. New-file publication is exclusive; existing
operator data is never overwritten during initialization or staging.

| Where | Change | Pass |
|---|---|---|
| **new `crates/roots` (`dpl-roots`)** | ENGINE layer, egui-free: `roots.toml`, the marker, the layout, the `Roots` handle (connected/disconnected), the drive check; ceiling serde, serde_json, toml | ST1–ST2 |
| `crates/engine-capture` | `content_root()` / `assets_root()` / `userdata_root()` deleted; its callers move to `dpl-roots` in one pass (no re-export shim) | ST1 |
| the 13 dependents, `crates/app` (boot, the honesty log) | read the `Roots` handle; the Disconnected screen and banner | ST1–ST2 |
| `composer`, `viziwiz` | OS temp → `common\temp`; `composer-state.json` → `.userdata` | ST1 |
| chat, analytics, imagine | their files move to `.aidata` | ST4 |
| `crates/xtask` | `stage-roots` (THE checkout's `roots.toml`), `move-library [--dry-run]`; the structure audit's `EGUI_FREE` and `CEILINGS` rows | ST1, ST4 |
| docs | `architecture.md` (the new crate, same commit as the audit), ADR-014 amendment (the layout), requirements R287, status | each pass |

### 5b. Reach outside the repo — for sign-off (`external-reach-inventory.md`)

| What | Why | Proposed |
|---|---|---|
| Write `roots.toml` beside the exe | where the library is | Confirm |
| Read `library.json` at one path on each drive letter or mount point, every few seconds while disconnected | to recognise the drive | Confirm |
| Write `library.json` into a linked library (on an external drive) | the drive's identity, without `-sys` deps | Confirm |
| A linked library outside the program folder | the feature itself | Confirm |
| Remove the parent walk and CWD lookups (inventory #29), and OS temp (#31) | the portable folder never reaches outside itself | Fix (ST1) |

## 6. Proof

- **Unit:** layout and marker round-trips; `roots.toml` choosing
  imported/linked; the drive check finding a marker on a different "drive"
  (temp folders standing in); the handle's states; the dry-run plan touches
  nothing; the move is copy, verify, label, and puts everything back on a
  failure.
- **Real exe:** a scratch library on a temp folder linked by `roots.toml`;
  renamed away → the Disconnected screen; renamed back → reconnects with no
  restart; moved to another path with the same marker → found there. Shots
  at 1440×900 and 1100×700.
- **The operator's own drive:** one check on a real USB drive, on their
  machine and at their tap — unplug, plug into another port, reconnect.
- **Release:** `cargo green`, both lanes, the exe rebuilt from `main`.

## 7. Risks

- **Every IO site moves to the handle,** the widest change since the rings.
  Mitigated by one owner crate, a compile-time migration (the old functions
  deleted, so the compiler finds every caller), and ST1 changing no
  behaviour beyond the walk.
- **Operator data moves.** Only by a shown plan and a tap, copy before
  label, never delete, and a dry run first.
- **A second app instance** (known-broken #13, `FjallError: Locked`) gets
  worse on a slow drive. That needs its own fix first, or ST2 handles it
  plainly.
- **Polling cost:** a stat on each drive letter every few seconds while
  disconnected, and nothing while connected.

## 8. Evidence

**ST1 local verification, 2026-10-25:** implemented in the operator-authorized
isolated worktree, based on RL #296. Final `cargo green` passed: 1,187 tests,
zero failures, six existing live-tool drills ignored; release and renderer
smoke passed. The 20 root-engine tests cover marker/config resolution,
bounded drive recovery, refusal and no-clobber behavior. Twelve debug/release
captures were inspected at the two required sizes, and the release app's
actual imported-library button created the layout and opened the main app
in the same process. Details and boundaries:
`docs/proof/portable-storage-st1/REPORT.md`. No operator data was migrated.

## Questions for the operator

- **Q1. What moves together when the library is linked?** The whole thing —
  content, working files, app state and AI data — as one folder on the
  drive (recommended: the rings are one unit today), or only the content,
  with app state staying beside the program?
- **Q2. What is an account?** A local profile — a name, several allowed,
  one at a time, each with its own library inside the same portable folder
  (recommended) — or one person per library, with "accounts" meaning
  something else (devpulse.app sign-ins later)?
- **Q3. `.aidata` vs `.userdata`:** AI data = chat (rooms, roles, provider
  profiles, activation evidence — never logins), the analytics ledger, the
  imagine cache and chat memory; app state = the index, the estate,
  settings, browser profiles. Right split?
- **Q4. When the drive is missing at launch:** the Disconnected screen that
  waits and reconnects by itself, with Locate… and "Use a library on this
  machine" (recommended), or open the app read-only on what is cached?
- **Q5. "Grow the portable folder cleanly":** is that Locations — adding
  existing folders in place, offline when missing (R83) — or something
  else, like moving to a bigger drive?
- **Q6. Order:** ST1 the layout and `roots.toml` → ST2 disconnect and
  reconnect → **ST5 Locations (linked video folders on the drive, R288)**
  → ST3 linked ↔ imported → ST4 accounts, `.aidata`, and moving today's
  folders? Or ST4 earlier, so the layout is final before anything links?
