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

**Amended 2026-10-26, after ST1 (#297):** with no parent walk and no
adoption of unmarked folders, the operator's exe no longer reached today's
folders. The operator ruled "Move them next", and, on where (R291):

> stored inside the project properly what part of portable? you let the
> person decide

then "Approve" for ST1b (§4.7). The order is now ST1 (done, #297) →
**ST1b** → ST2 → ST5 → ST3 → ST4 (accounts and the `.aidata` split).

**ST2 approved 2026-10-27** (§4e), as two PRs, ST2a then ST2b:
- Q-ST2-1, a new drive letter mid-session: "Save, then Restart now";
- Q-ST2-2, the heartbeat: "Only when something saves". Nothing runs while
  connected; an unplug is noticed at the next save or failed read under the
  root;
- Q-ST2-3, a temporary `subst` drive letter for ST2b's proof: "Yes, use
  subst".

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

**Moved to ST1b (§4.7), ruled 2026-10-26.** ST4 keeps the split of
`.userdata\` into `.userdata\` and `.aidata\`.

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

### 4.7 Your library moves in (ST1b, ruled 2026-10-26)

After ST1 the app no longer finds today's folders, so the move comes next,
ahead of ST2.
- **The person decides (R291).** The first-launch screen gains a third
  choice, **Bring in my existing library…**:
  - the source is the folder it found (for a build, the project's
    `content\`, `assets\` and `.userdata\`) or a path the person pastes;
  - the destination is the person's pick: **Inside the project** (the
    default for a build), **With the program** (a release folder), or
    **A drive or folder…**.
- **A build attunes to its project.** When the exe sits at
  `<project>\target\{release,debug}\` and `<project>\Cargo.toml` exists,
  the program folder is `<project>`: `roots.toml` and `library\` live
  there (both gitignored since #297), so `cargo clean` never reaches them.
  Exactly two fixed levels are checked; nothing walks. A release folder is
  unchanged.
- **The plan first.** Every top-level item, its size and where it lands:
  `content\` → `library\accounts\me\content\`, `assets\` →
  `…\assets\`, `.userdata\` → `…\.userdata\` whole (the `.aidata`
  split stays in ST4). `cargo xtask move-library --dry-run <source>` prints
  the same plan and touches nothing.
- **Move it:** copy, check each file byte for byte (stronger than a hash,
  and no new dependency), write `library.json` and `roots.toml`, open the
  app. The old folders are left untouched:
  sources are never mutated, which for ST1b replaces §4.5's "label".
  Deleting them is the operator's call. On any failure the new copy is
  removed and nothing else has changed.
- **Drills stay off the library:** the twelve EguiPilotAI fixtures with no
  scratch `CONTENT_ROOT` get one in this pass.

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

### 4d. ST1b execution queue — grounded 2026-10-26

Read in THE checkout at `9ab0c8a` (#297): `crates/roots/src/{lib,setup,
model,disk,handle}.rs`, `crates/app/src/storage_startup.rs`,
`crates/xtask/src/main.rs` (`stage_roots`), `.eguipilotai.toml` and the
runner's environment merge (`egui-pilot-ai-runner/src/lib.rs:286-295`:
profile first, fixture wins). THE checkout's legacy rings hold 358 files
(content 156.6 MB, assets 3.8 MB, .userdata 14.4 MB) and no links.

1. **A build attunes to its project** (`dpl-roots`): `program_dir()` is the
   exe's folder, or `<project>` by `project_of_build` (two fixed levels).
2. **The move as engine code** (`dpl-roots`, `migrate.rs`; the ceiling stays
   serde, serde_json, toml): `has_legacy`, `plan_move` (read-only) and
   `move_in` (copy, byte-for-byte check, identity published last,
   `roots.toml` last of all; any failure removes what it made). `setup.rs`
   shares `create_layout`, `publish_identity` and `write_config` with it.
3. **The first-launch screen:** "Keep my library in this project" for a
   build; **Bring in my existing library…** as its own page — the library
   name, one card saying From, Into and what moves, **Move it** under it,
   **Change where…** for the fields, progress while it copies and checks.
   `--bring-in` opens the page for proof shots.
4. **`cargo xtask move-library --dry-run <source> [<library>]`**; without
   `--dry-run` it refuses.
5. **Drills stay off the library:** each `.eguipilotai.toml` profile has
   `CONTENT_ROOT = "target/eguipilotai/default/content"`; a fixture's own
   wins.
6. **Docs:** ADR-014's amendment, this queue and §8, status, the journal.

Wire fields are implementation details within the approved design:
schema `v = 1`, `mode = "imported" | "linked"`, library identity and
label, linked `last_path`, and active account `me`. Marker fields are
`v`, `id`, `label`, `created`. New-file publication is exclusive; existing
operator data is never overwritten during initialization or staging.

### 4e. ST2 execution queue — grounded 2026-10-27

Read in THE checkout at `2d75c27` (#299): `crates/roots/src/{lib,handle,
discovery,disk,model}.rs`, `crates/app/src/{main,storage_startup}.rs`,
`crates/egui-pilot-ai-{transport,runtime}/src/lib.rs`,
`crates/estate/src/lib.rs`, `.eguipilotai.toml`, and a census of every
`dpl_roots::` call the app can reach (22 files).

What the reading found:
- **The startup screen is its own `eframe` run** (`run_and_return`) with its
  own `egui::Context`. The pilot runtime attaches only to the main app's
  (`main.rs:1321`), and a runtime's handshake belongs to one guard, so
  handing the stdio transport across would need a second handshake. One
  window removes the problem instead of working around it.
- **`last_path` is never written:** a reconnect updates it in memory
  (`handle.rs:155-159`), and `disk` can create `roots.toml` but never
  replace it.
- **Leaving Connected panics:** `content_root()` and its siblings `expect` a
  connected root (`lib.rs:192-195`). At least eight surfaces call them every
  frame: the author dashboard, lessons, the episode picker, the gallery, the
  Shows picker, slide media and backdrops.
- **There is no single save path.** The private crash-safe writers in
  `dpl-doc`, `dpl-comp` and `dpl-workflows` carry most saves, plus about a
  dozen one-off writers. The three document crates' ceilings forbid a
  `dpl-roots` edge.
- **About 25 struct fields cache library paths:** `Editor.dir`,
  `Composer.dir`, `SeamState.root`, `ShowsHub.content_root` and others.
- **The app can't be rebuilt in-process.** The seam thread has no shutdown
  and holds the estate `Arc`, so fjall's lock and port 7341 stay held — the
  shape of known-broken #13.
- **There is no banner or notice API.** The hooks are the status-bar cell
  registry (`shell/src/status.rs:94`) and `ShellEvent`
  (`fabric/src/lib.rs:43`).

**ST2a — at launch: not connected, then reconnected by itself** (one PR)

1. **`dpl-roots`: watch, locate and remember.** The ceiling is unchanged.
   - `disk::replace_config` creates `roots.toml.new` exclusively, syncs it,
     then renames it over `roots.toml`. It refuses links.
   - A reconnect at a new path writes the new `last_path`. This applies to
     linked libraries only: an imported library never rewrites, and a seam
     never writes.
   - `reconnect()` refreshes a clone of the handle outside the lock, then
     swaps it in only if the configuration is unchanged. A 26-letter probe
     never blocks a frame.
   - `watch(wake)` is one background thread (`dpl-roots-watch`) that probes
     every 2 s while the library is not connected; `wake` repaints. It never
     runs under a seam.
   - `locate(folder)` reads the marker there:
     - the same id connects and remembers the path;
     - another library returns `Located::Other { label }` for the screen to
       ask about;
     - nothing is written into the folder.
   - `status()` gives the screen the label, the last path and the reason.
2. **One window** (`main.rs`, `storage_startup.rs`).
   - `prepare()` and `run_and_return` go. `main()` asks `ready()` without a
     window.
   - The three headless drills (`--clone-seed`, `--export-guide`,
     `--brainstorm-drill`) refuse plainly without a library: exit 2, and no
     window.
   - The boot steps that need the library (the LS2 slide upgrade, the AI
     census, `--preset`) move into one `open_library()`. It runs before
     `App::new`, whether the library was ready at launch or opened on the
     screen.
   - There is one `eframe::run_native`. Its root app shows the library
     screens until a library opens, then `App` in the same window.
     `on_exit` is forwarded.
   - The pilot runtime attaches once, at creation, and is pumped every frame
     in both stages. `attach_pilot_runtime` splits in two: the transport and
     the renderer identity at creation; the coach catalog, the fabric tap
     and chat when `App` adopts it.
3. **The Disconnected screen.** The `unavailable` page grows.
   - It says "Your library 'DevPULSE drive' isn't connected." and "Plug it in and
     it reconnects by itself.", with where it was last seen and a quiet
     "Looking…". When the watch finds the library, the app opens with no
     tap.
   - **Locate…** is a path field with **Check**, like Change where… on the
     Bring in page. A different library asks "That's 'Spare library'. Open it
     instead?" before anything changes.
   - **Use a library on this machine** offers the first screen's choices.
     Finishing one replaces `roots.toml`, after a line saying the old
     library won't reconnect by itself any more and that Locate… brings it
     back.
   - **Quit.**
   - A broken configuration keeps the honest "couldn't be opened" page with
     Try again, and the watch retries it too. That covers a malformed file,
     a different library at the path, and two copies.
4. **Proof (ST2a).**
   - **Unit (`dpl-roots`):**
     - the watch step finds the library on another "drive" (temp folders
       standing in, through `refresh_with_candidates`) and writes the new
       `last_path`;
     - an imported library and a seam never write;
     - `replace_config` leaves the old file whole when it can't finish;
     - `locate` never silently adopts another library.
   - **Headless (`egui_kittest`):**
     - the page opens the library when its marker appears;
     - Locate… on another library asks first;
     - Use a library on this machine replaces `roots.toml` only once
       confirmed.
   - **EguiPilotAI on the real exe:**
     - Profile `st2-portable` launches a copy of the devtools exe in
       `target/st2-proof/portable/`. That is its own program folder, so it
       has no seam. It is linked to a library last seen at `P:\library`.
     - Scenario `tests/eguipilotai/st2-disconnected.toml`: with no `P:`, the
       page's nodes are present and Locate… opens its field.
     - While the page waits, the scratch drive folder comes back as `Q:`
       (`subst`, removed after the run). The app must open by itself, in
       the same window, and remember `Q:\library`. (Amended 2026-10-27:
       a new letter proves more than renaming the folder back; Locate…'s
       path is proven headless, since a typed path is machine-specific.)
   - **Shots** at 1440×900 and 1100×700.

**ST2b — mid-session: unplugged, nothing lost, nothing written to the wrong
place** (one PR, after ST2a)

5. **`dpl-roots`: the guard.**
   - A new state, `Unplugged { last, reason }`. The path functions keep
     returning `last`, so per-frame readers find nothing instead of
     panicking.
   - `writable(path)` checks the marker at the current library, at most
     every 500 ms. While unplugged it refuses with "Your library 'DevPULSE drive'
     was unplugged — plug it back in to save."
   - A failed check flips the state to Unplugged and wakes the watch. So
     does `io_failed(path, error)`, for an IO failure under the root: a save,
     or a listing that can't read the content folder.
   - No heartbeat (Q-ST2-2, ruled): nothing runs while connected.
6. **Every writer asks** (the census list).
   - **The three document writers** use an installed guard:
     `set_write_guard(fn(&Path) -> Result<PathBuf, String>)` in `dpl-doc`,
     `dpl-comp` and `dpl-workflows`. That adds no dependency, so the
     ceilings hold.
   - **`dpl-index`:** `Index::open_default`.
   - **App and chat:**
     - ChromeChoices (`main.rs:1719`) and the AI backfill
       (`ai_backfill.rs:80`);
     - the chat policies (`chat.rs:286`, `2387`, `2439`);
     - the room library folders (`chat.rs:4553`);
     - the chat brainstorm (`chat.rs:2665`);
     - the Imagine cache (`imagine` `lib.rs:170`, `206`);
     - the profile and account stores, on each operation.
   - **ViziWiz:**
     - the landing policy (`lib.rs:1466`, `1477`);
     - the start of a capture (`inbox()`, `298`);
     - the freeze file (`editor.rs:50`).
   - **Composer:**
     - its saved state and prefer-reference (`lib.rs:106`, `1611`);
     - the render's temp folder (`lib.rs:5847`);
     - import and promote (`import.rs:164`, `192`, `222`, `290`);
     - `libraries.json` (`browse.rs:308`, `322`).
   - **Present:** `present-screens.json` (`lib.rs:521`).
   - **Workbench:** moves and writes (`lib.rs:1001`, `studio.rs:217`,
     `author.rs:2081`, `2346`).
   - **The seam** refuses changes while unplugged.
   - **The ledger writer** holds its rows until the library is back.
7. **The banner** (`dpl-shell`).
   - `ShellEvent::Library(Unplugged | Reconnected)` drives a full-width
     ColdLight banner under the top bar: "Your library 'DevPULSE drive' was
     unplugged. Plug it back in to save — your open work is kept."
   - A Library status cell sits alongside it.
   - A recording running at the moment of the unplug ends as a failed job,
     with its reason.
8. **Kept edits.** A refused save keeps the work in memory: the card
   editor, lessons, the brainstorm and the comp autosave already hold it. On
   reconnect, each one whose save was refused saves again.
9. **Reconnect.**
   - **The same path** is the usual case, since Windows gives a replugged
     drive its letter back:
     - the estate reopens in place (`Estate::reopen` behind an inner lock,
       so chat, composer and the seam keep their `Arc`);
     - the ledger flushes, kept edits save, and the banner clears.
   - **A new path mid-session (Q-ST2-1, ruled):** kept edits save at the new path,
     because `writable` rebases a path under the earlier location. Then the
     banner offers **Restart now**, and the app restarts on the new letter.
     Carrying on without a restart would mean re-pointing about 25 cached
     paths and making the seam stoppable: a pass of its own, if wanted.
10. **Proof (ST2b).**
    - **Unit:**
      - `writable` refuses while unplugged, rebases, and never returns a
        path on another library;
      - each document crate's guard refuses;
      - the estate reopens.
    - **Real exe (Q-ST2-3, ruled):**
      - a scratch library on a temporary `subst` drive letter, linked;
      - with the app open, `subst /d`, then a typed card edit → the save is
        refused with those words and the banner appears;
      - the same letter back → the edit saves and the banner clears;
      - a different letter → the edit saves there and Restart now appears;
      - shots of the banner at both sizes.
    - **The operator's own USB check** comes last (§6).

**Amended while building ST2b (2026-10-27):**
- **The banner (step 7):** the app sets the shell's banner directly each
  frame, so there is no `ShellEvent::Library`. There is no Library status
  cell either, because it would only repeat the banner.
  - The banner's words and Restart now paint on the top layer, so a sheet
    never covers them.
  - Known limit: a sheet as tall as the window, like the card editor, has
    its top edge, including Done, under the strip. In the Moved state the
    strip is taller, for Restart now's touch height. Restart now is the
    tap expected there, and the work is already saved.
- **Reads (step 5):** a failed read reports itself (`io_failed`) only from
  the author dashboard, and only when its list comes back empty. A full
  library costs nothing, and nothing runs while connected.
- **Writers (step 6):**
  - `dpl-ledger` gets the same installed guard. Rows it can't write wait,
    up to one segment per domain; older ones are dropped and counted.
  - The seam asks `dpl_comp::writable` before a render, so it gains no
    new dependency.
  - The app gains a direct `dpl-comp` edge to install that crate's guard.
- **Kept edits (step 8):**
  - The card editor and lessons save again on reconnect, through the
    `Hub::library_back` hook.
  - The comp autosave retries by itself.
  - The chat brainstorm saves the whole board on its next change.
  - A card's words-to-say (script) are not saved again on reconnect; the
    next change to them saves them.

| Where | Change | Pass |
|---|---|---|
| **new `crates/roots` (`dpl-roots`)** | ENGINE layer, egui-free: `roots.toml`, the marker, the layout, the `Roots` handle (connected/disconnected), the drive check; ceiling serde, serde_json, toml | ST1–ST2 |
| `crates/engine-capture` | `content_root()` / `assets_root()` / `userdata_root()` deleted; its callers move to `dpl-roots` in one pass (no re-export shim) | ST1 |
| the 13 dependents, `crates/app` (boot, the honesty log) | read the `Roots` handle; the Disconnected screen and banner | ST1–ST2 |
| `composer`, `viziwiz` | OS temp → `common\temp`; `composer-state.json` → `.userdata` | ST1 |
| `crates/app` `main.rs` | one window: the library screens and the app in one `eframe` run; the pilot runtime attaches once | ST2a |
| `dpl-doc`, `dpl-comp`, `dpl-workflows` | an installed write guard in each crash-safe writer (no new dependency) | ST2b |
| `dpl-estate` | `reopen` in place, so the `Arc` holders keep working | ST2b |
| `dpl-shell`, `dpl-fabric` | the library banner and status cell; `ShellEvent::Library` | ST2b |
| the census's one-off writers (§4e step 6) | ask `dpl_roots::writable` first | ST2b |
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
| A build's program folder is its project: an exe at `<project>\target\<profile>\` with `<project>\Cargo.toml` uses `<project>` for `roots.toml` and `library\` | the library stored inside the project (R291) | **Ruled** 2026-10-26 (ST1b) |
| Rewrite `roots.toml` (an exclusive `.new`, then a rename) when the library is found at a new path, Located, or replaced by Use a library on this machine | §4.3's "updates the last path"; the old file is never left half-written | Covered by row 1 (ST2a) |
| For ST2b's proof only: a temporary `subst` drive letter over a scratch folder, removed after each run | the only way to unplug and replug a drive letter under a running app without hardware or admin (Windows can't rename a folder while files in it are open) | **Ruled** 2026-10-27 (Q-ST2-3) |

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
  disconnected, and nothing while connected (Q-ST2-2, ruled).
- **An empty card reader or DVD drive** can make Windows show "There is no
  disk in the drive" when a letter is probed. The fix, `SetErrorMode`, is
  FFI, which the workspace denies outside `dpl-lottie-render`. The probe
  runs off the UI thread so it can never freeze the window. ST2a checks this
  on this machine. If a dialog appears, that is a question for the operator
  (an architecture amendment), not a quiet workaround.
- **fjall after a real unplug:** its background workers may fail on a dead
  handle. ST2b's `subst` proof shows whether the estate can be dropped and
  reopened cleanly; a panic there blocks ST2b until fixed.
- **The headless drills without a library** used to open the startup
  window. With one window they exit 2 and say why, which is more honest for
  a command that opens no window.

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

**ST1b, 2026-10-27:**
- **Unit:** `dpl-roots` 27 tests (7 new): a build attunes to its project and
  test binaries, other folder names, a missing manifest and copied exes do
  not; the plan counts each ring and writes nothing; a busy destination, a
  library inside a ring, a ring inside the library and an empty source are
  refused; bringing in copies exactly (every ring's files and bytes equal,
  empty folders too), publishes last and leaves the source unchanged; beside
  the program it is imported; an existing `roots.toml` is never replaced; a
  copy that fails partway (a file held open without sharing, Windows) removes
  what it made, with or without a pre-made empty folder.
- **The page, headless** (`egui_kittest`, the app's tests): the card shows
  the plan, Move it copies into the program's library, writes `roots.toml`
  and opens, and the source is unchanged; a busy destination is named and
  nothing moves.
- **Real exe, scratch only:** a scratch project (`Cargo.toml` stub and the
  exe in its `target/debug/`) and a scratch portable folder, each with
  synthetic rings. Shots in `docs/proof/portable-storage-st1b/`: the first
  screen and the Bring in page at 1440×900 and 1100×700, and the portable
  folder's page ("with the program"). Move it is on screen at 1100×700. No
  file was written by those runs.
- **On THE checkout, read-only:** `cargo xtask move-library --dry-run
  D:/projects/devpulselabs` lists content 63 files (156,600,172 bytes),
  assets 13 (3,785,775), .userdata 282 (14,441,723): 358 files, 174.8 MB,
  into `D:\projects\devpulselabs\library`. Without `--dry-run` it refuses.
- **EguiPilotAI:** the first screen is not reachable — the pilot's runtime
  attaches only in the main app, and its stdio reader can't be handed from
  the startup window to the main one without losing a frame. ST2's
  Disconnected screen lives in the same window, so ST2 needs one
  process-wide transport across both. The `test-lab` fixture, which has no
  `CONTENT_ROOT` of its own, now runs in `target/eguipilotai/default`.
- **Owed to the operator:** the real move — open the rebuilt exe, tap Bring
  in my existing library…, check the card, tap Move it.

**ST2a, 2026-10-27:**
- **Unit:** `dpl-roots` has 34 tests, 7 of them new:
  - a linked library found on another "drive" is written back to
    `roots.toml` once, and the next open finds it there;
  - an imported library and a seam never rewrite;
  - a replace finishes over an interrupted one's `.new`;
  - a replace that can't finish leaves the old file byte for byte
    (Windows, held open without sharing);
  - Locate names this library or another and writes nothing;
  - another library replaces `roots.toml` only with `Publish::Replace`;
  - a background reconnect installs its probe only if no `roots.toml` was
    written meanwhile. This is the one test that touches the process-wide
    handle; other tests' writes count against it too, so it retries.
- **The pages, headless** (`egui_kittest`, 5 tests): the two ST1b tests,
  plus:
  - the Disconnected page names the library and opens with no tap when the
    watch finds it;
  - Locate… on the library itself opens it where it is now, and on another
    library asks first, with `roots.toml` unchanged until the person taps
    Open;
  - Use a library on this machine replaces `roots.toml` only once a choice
    finishes, and the drive's library is left as it was.
- **One window, real exe:** the pilot now attaches at creation and reaches
  both stages.
  - `jobs-board` passes on the main app.
  - `shell-navigation` fails at its third step, on a "Test Lab" rail item
    that no longer exists. The scenario dates from the toolkit's first
    commit, so the failure isn't caused by this pass; it is left as
    found.
- **The Disconnected page, real exe, scratch only** (`target/st2-proof/`;
  prep in the session scratchpad, `st2a_prep.sh`):
  - shots at 1440×900 and 1100×700 in `docs/proof/portable-storage-st2a/`;
  - those runs wrote nothing to either folder.
- **EguiPilotAI `st2-disconnected`: pass, twice**, the second time on the
  final build. With no `P:`, the page's nodes are present and Locate…
  opens its field. Six seconds in, `subst Q: target\st2-proof\drive`; the
  app opened by itself in the same window and wrote
  `last_path = 'Q:\library'`. `Q:` was removed after each run.
  - Captures: `locate-1440x900.png`, `reconnected-on-q-1440x900.png`.
- **Headless drills:** `--clone-seed` without a library printed the reason
  and exited 2, with no window.
- **The empty-drive dialog (§7)** didn't appear in any run. This machine
  has only `C:` and `D:`, so a card reader is still unchecked: the
  operator's USB check covers it.

**ST2b, 2026-10-27:**
- **Unit:**
  - `dpl-roots` has 35 tests, 1 new: pulled out, a failed read notices it;
    writes are refused and readers keep their folders; another library in
    its place is never opened; back on a new letter, a path cached before
    lands there, and `roots.toml` remembers it.
  - `dpl-estate` has 5 tests, 1 new: after a reopen, every holder still
    reads and writes.
  - The tests touching the process-wide handle take turns on a mutex.
- **Real exe, scratch only** (`target/st2-proof/`; scenario
  `tests/eguipilotai/st2-unplugged.toml`, profile `st2-portable`, fixtures
  `st2-author` and `st2-author-1100`):
  - The library is linked as `Q:\library`.
  - A background timeline, counted from the app's start, runs
    `subst Q: /d` at 12 s and `subst Q:` again at 20 s, then `subst Q: /d`
    at 28 s and `subst R:` at 36 s.
  - Each run removed `R:` afterwards; `subst` lists nothing now.
  - The run changes a card's background three times.
- **Passed at both sizes**, the 1440×900 run on the final scenario. The log
  shows the whole sequence: unplugged → back (kept edits saving) →
  unplugged → back somewhere new (Restart now offered, `R:\library`).
- **On disk afterwards:**
  - rev 2 is orchid-haze, saved while connected;
  - rev 3 is deep-lagoon: refused while unplugged, saved when `Q:` came
    back;
  - rev 4 is solar-flare: refused, then saved on `R:` through the rebased
    path;
  - `roots.toml` says `last_path = 'R:\library'`.
- **Shots** in `docs/proof/portable-storage-st2b/`: unplugged, back and
  moved, at 1440×900 and 1100×700.
- **Found and fixed during the proof:**
  - The banner was first a plain top panel, and the card editor's sheet
    covered it (text cut, Restart now hidden). Its words now paint on the
    top layer.
  - A 30 s profile timeout ended the first run partway; `st2-portable` now
    allows 90 s.
  - The first 1100×700 run tapped before the second unplug; the scenario's
    margins were widened.

### ST2 follow-through — grounded 2026-10-27 on `050bdee`

The current execute instruction continues the approved ST2 safety contract.
Upstream #300 and #301 already delivered the two planned passes. This isolated
checkout therefore fixes only two code-grounded gaps, without repeating ST2
or advancing into ST5.

1. Chrome preset persistence must keep its last-successful snapshot unchanged
   after a refused save, so the current in-memory choice retries on reconnect.
2. The estate's existing open database must check an installed, per-instance
   write guard before opening, mutating or flushing. Keep the dependency ceiling
   unchanged. A guard that resolves to a different directory must refuse use of
   the old database handles until explicit reopen/restart; it must never merely
   validate the new directory and then write through old handles. Reopen checks
   the destination before closing the current store. Wire the app's shared
   estate to `dpl_roots::writable`.
3. Regression tests must prove failed preference saves retry, refused estate
   writes/removes/flushes leave stored values unchanged, and a rebased path
   cannot authorize an old open database. Existing startup/reconnect UI is
   unchanged; reuse its committed native proof, run the focused tests and
   full cargo green, then the usual double-green PR gate and isolated release
   rebuild. No operator library or primary checkout mutations.

The changes are additive guard wiring and successful-save bookkeeping;
rollback is reverting the code commit, with no data format migration.

Local evidence: all four new regressions passed in `cargo green` (1,212
tests passed, zero failures, six existing live-tool drills ignored). The
format, Clippy, architecture/contracts and release gates passed; smoke
rendered 16,616 shape pixels, 10,634 text pixels and 60 export frames.
This follow-through changes no pixels and makes no fresh USB drill claim.

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
