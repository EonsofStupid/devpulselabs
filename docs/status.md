---
updated: 2026-10-30
active_lane: portable-storage
dirty: false
---

# Status — where DevPULSE Labs is right now

This page is **rewritten, never appended**. History is `git log`, the PRs,
and the tag `docs-history-2026-10-12` (every doc before the one-truth
reorganization).

Read in this order: this page → [`requirements.md`](requirements.md) (the
operator's words) → [`README.md`](README.md) (the map of every doc).

**Updated:** 2026-10-30. Main has ST5b (#309: Locations become the one
list) on top of `bbac4f5` (#311: public repository, hosted CI). The release
exe is rebuilt from main after the merge.

**Public repository setup (R293).** Public visibility is verified and both
estate runner registrations are removed. Green now targets GitHub-hosted
Windows 2025 and Ubuntu 24.04, with pinned actions and read-only PR tokens.
Every PR runs both required checks, including documentation-only changes.
Secret scanning, push protection, dependency alerts/security updates and private
vulnerability reporting are enabled. All external fork workflows need approval.
Main protection requires PRs, resolved conversations and both green checks;
force pushes and deletion are forbidden. The setup PR must pass both hosted
lanes before merge; see the PR checks for delivery evidence and
[`runbook/ci-runners.md`](runbook/ci-runners.md) for the operating policy.

**CI artifact storage** (#310, ruled 2026-10-30): the private-repo storage
cap filled on 2026-10-29, and every Windows lane went red at the exe upload
while the gate was green. PR runs no longer upload; main uploads, kept 1
day, and #311 kept that policy on the hosted lanes. The cap applies only to
private repos, so it no longer applies now the repo is public.
`docs/plans/ci-exe-upload.md`.

## Now

**Project chat implementation (R292).** The operator authorized the reviewed
design with **resume**. PC1a is implemented on isolated `feat/chat-project-storage`
from `e60c140`, now integrating `855f674`: project-owned archives, recoverable
drafts, native project selection and bounded machine discovery.
Before integration, full `cargo green` passed (1,265 tests,
six existing ignored); feature checks passed 53 window and 11 host tests.
Native A/B/A switching, restart, separate drafts and offline transcript reading
passed at 1440×900 and 1100×700, including with the main window hidden.
Evidence: [`proof/chat-project-storage/README.md`](proof/chat-project-storage/README.md).
PR #305 is merged. PC1b is in draft PR #308 for provider-native resume,
stable conversation links and standalone launch. Its native/standalone adapters
remain unqualified and disabled pending the outstanding real-provider evidence.
No operator provider homes or credentials are changed by development. The
scope includes supported standalone AI apps and automatic workspace organization;
neither scope nor this implementation authorization needs another confirmation.
Storage's ST5b is done (#309).

**Portable storage (ST).** ST1 landed (#297): the library has one owner
(`dpl-roots`), `roots.toml` beside the program, a `library.json` marker, a
first-launch choice and a startup error with Try again / Quit; no parent
walk, no CWD. ST1b landed (#299, R291): a build keeps `roots.toml` and its
library inside its project, and the first screen's **Bring in my existing
library…** copies today's `content\`, `assets\` and `.userdata\` into a
new library the person places — checked byte for byte, the old folders left
exactly as they were. ST2a (#300): the library pages and the app share one
window, so EguiPilotAI reaches both. A linked library that isn't plugged in
shows "Your library '…' isn't connected", is looked for every two seconds,
and opens by itself on any drive letter, which `roots.toml` then
remembers. Locate… and Use a library on this machine are there too.
ST5a (#303), **Locations**: in the Browse window, "Add as a location"
links any folder in place, for example a drive of videos. Nothing is
copied, and nothing is written inside it. Its videos and pictures join the
media shelf and the pickers with cached posters. While the drive is away
they show "offline — plug it in", never broken, and the drive is found
again on any letter. Composer lists Locations with Locate… and Remove.
**ST5a′ (#304)** (asked 2026-10-28: "aware if its been started and stopped …
linux arm or windows or mac"):
- **One app-wide watch** looks every 2 s while the library is open. A drive
  pulled or plugged back in mid-session shows within seconds, on the shelf
  and in Locations, with a plain status line.
- **A drive changed while the app was stopped** shows at launch.
- **Lists and painters read the last look,** never the drive.
- **Drives are found** on other drive letters (Windows), through the
  kernel's mount table (Linux, ARM too) and under `/Volumes` (macOS).
**ST5b** (ruled 2026-10-29: "Put everything on the shelf", "The whole
folder"), one list:
- **The old lists move in once.** The pinned packs and Browse's libraries
  become Locations, plugged in or not. Their lists are kept in
  `.userdata/moved-to-locations/`.
- **Indexed when seen.** A location waits until its folder is seen, then
  the app indexes its videos and pictures onto the shelf with posters,
  with the Editor open or not.
- **Pin pack, the pack shelf and `--pin-pack` are gone.** Ingest pack
  stays.
- **Import by reference links the file** where it lives, as part of its
  folder's location, with no symlink and never a hidden copy. The timeline
  shows it as "clip.mp4 · on 'folder'".
- **Fixed on the way:** an index locked by another writer was being set
  aside as "corrupt". Index work now takes turns, and a locked database is
  never quarantined.
ST2b (#301) covers a library pulled out mid-session:
- every write asks first, so nothing lands on whatever drive takes its
  letter;
- saves are refused in plain words, with an amber banner, and open work
  stays in memory;
- back where it was, the kept edits save and the estate reopens;
- back on another letter, they save there, and the banner offers Restart
  now.

**ST2 follow-through:** a failed Chrome preset save stays pending and retries
when storage returns. The shared estate now receives the library write guard
before open, mutation, explicit flush and reopen; an old open database cannot
be authorized by a rebased path. This covers API calls, not Fjall background
work accepted earlier. Development is isolated under
`D:/localdev/devpulselabs/.worktrees/portable-storage-st1`; the operator's
checkout, library and release are untouched by this follow-through.

**The operator's own move is owed:** open the exe, tap Bring in my existing
library…, read the card (358 files, 174.8 MB into
`D:\projects\devpulselabs\library`), tap Move it. Plan:
`docs/plans/portable-storage.md` §4.7 and §4e.

**Lessons, slides and components (R285, ruled 2026-10-21).** The schema
comes first, so lessons can mix and match slides and visuals can be
refreshed without touching content. The research is
`docs/research/lesson-slide-schema.md`.
- **Five layers:** look, component, entity preset, slide, and lesson or
  episode. Lessons build episodes.
- **Pin content, float presentation:** a lesson pins a slide's revision and
  offers "Update available"; looks, presets and components float.

| Pass | What | State |
|---|---|---|
| LS1 | Component contracts: `dpl.component/1`, a `component.json` beside each of the 28 templates, the gallery demo built from each contract's first example, the Workbench reading the contracts, the `contracts` phase of `cargo green` | **done** (#280) |
| LS2a | Slides by id (`dpl.slide/3`): its own id and revision, blocks as the component's slots, the library folder, Layout and Starter taps that start from what was tapped, read-only seeds, the one-time upgrade of the six v2 drafts with backups | **done** (#281) |
| LS2b | Presets by reference: the five looks are card presets (`for`, `exposes`, `tone`, and a font default for their lines, ruled 2026-10-21); a slide stores the look's id; the cascade is slide → look → show | **done** (#282) |
| LK1 | Linked looks (R286): the look chip and its menu (your changes, Undo all my changes, Stop following / Follow the look again), a dot on every value that is yours with a comparison and a one-tap way back, try-before-keep, More controls, one revision per typing burst, Undo on every tab | **done** (#283) |
| LK2a | Your own looks as files (`dpl.look/1`, the `Looks` registry passed to every surface that paints): Save as a new look, your looks first in the tray and in the Library, the missing-look state | **done** (#284) |
| LK2b | Changes that spread: Make these the look's new style ("changes 12 other slides", before and after, one Undo), your version of a built-in with Back to the built-in, Open the look (in the card editor, ruled 2026-10-22) | **done** (#285) |
| LK3a | Line looks (ruled 2026-10-23): per-line entrances; the 14 text and 6 quote looks by reference between a line's own values and the card's look; a line's colour painted; per line the chip, menu, look row, dots, Colour and Movement rows, stop/follow again; shrink to fit; the Library shows them | **done** (#286) |
| LK3b | Picture looks made real (ruled "Approve wider LK3b", 2026-10-23): fit painted for real (it was ignored), a frame in the picture's colour, the picture's own entrance; the 5 picture looks; picture controls on tap and on the Picture tab; the Library shows them | **done** (#287) |
| LS3a | Lessons (`dpl.lesson/1`, ruled "Approve, two PRs", 2026-10-23): every revision a save replaces is kept and readable; a lesson's uses point at library cards and pin a version; the Workbench's Lessons page (New lesson, add from your library, reorder, take out, a job per card, Newer version → Compare → Update / Not now, Update all, Edit — the lesson you edit from takes the change, the others ask); Author builds into a lesson; a seed episode's slides are read-only | **done** (#288) |
| LS3b1 | Episodes built from lessons (`dpl.episode/2`, ruled 2026-10-24: lessons float, two PRs, an edit from a show shows in it, "Episode — Chapter"): one walk every surface reads (deck, Presenter, episode wall, editor picker, Studio, Building into, shows hub, chat spine), the theme from the episode, cards named by their own words; cloning a seed makes a real episode; placing into a chapter adds a use; the "copy to my shows" control made tappable | **done** (#289) |
| LS3b2 | Your existing episodes move into the library ("Two PRs, dry run first", 2026-10-24): a read-only dry run (`cargo xtask upgrade-episodes --dry-run`; on the operator's content: 0 shows to move), then "Move into the library…" on the Episodes page — the plan as a sheet, **Move it** — with the old file kept and everything put back on a failure; never at launch | **done** (#290) |
| LS4a | The component engine (R287, ruled 2026-10-24: the render tree, phased; props; shots before merge; three styling levels): a component's layout is data in its contract — slots in reading order with their size, ink, font and tracking, an alignment fixed or from a variant axis, per-variant adjustments, and decorations painted in the card's colour (a card, a mark, a slot pinned to the top, a rule, where the column sits); title, section, quote, big number, callout and media drawn by it everywhere, the gallery included; the card's colour painted at last; props on a slide; the editor's Layout row; Layout for one lesson only on the Lessons page | **done** (#291) |
| LS4b | Lists and data: agenda, features, timeline, split, compare, stats, code and the infographics painted; list items selectable | |
| LS4c | Interactive components as node trees and recipes, with touch sub-events | |
| RL | Run a lesson (R287, ruled 2026-10-25, one pass): **Run** a lesson (the Lessons page, or from a card) or an episode (the episode wall, the Present hub) — locked, the pinned cards, the slides OS full screen and the presenter opening at once in its own window; touch for the big screen (tap anywhere for the next beat, the left edge back, swipe, long press for the navigator); the presenter drives (big Back / Next, tap a beat to jump, Swap screens); window places kept per machine. **On tap** (the card sheet's tab, and the Lessons editor): each line at the start or on a tap, the teleprompter's words per tap written to the card's `script.mdx` as you type, a preview at any tap; each lesson card says "1 tap · script". `--run-lesson <id>` | **done** (#296) |
| ST | Portable, linked storage (R287, R288): a library with the program or linked on a drive, recognised on any letter, a Disconnected screen that reconnects by itself, linked video folders (Locations), per-account `.userdata` / `.aidata` | **approved** 2026-10-25, reach signed off (R289) — `docs/plans/portable-storage.md`. ST1 **done** (#297); ST1b **done** (#299: a build's library in its project; Bring in my existing library…, R291); ST2 approved 2026-10-27 as two PRs — ST2a **done** (#300: one window; the Disconnected page reconnects by itself, on any letter); ST2b **done** (#301: unplugged mid-session — refused saves, the banner, kept edits, Restart now; follow-through #302); ST5 approved 2026-10-27 — ST5a **done** (#303: Locations linked in place, offline posters, found on any letter); ST5a′ **done** (#304: aware while running and across restarts, Windows/Linux incl. ARM/macOS); ST5b **done** (#309, ruled 2026-10-29: one list — packs and libraries move in and are indexed when seen, reference imports link in place); next ST3 → ST4 |
| AH | AgentHub (R290, ruled 2026-10-26): the app finds the AIs installed on the machine and links to them; a hub showing each one and what it is working on; each AI arrives briefed, warmed up and resumable; Clyffy brings up the task's UI to work on it with other AIs and other people; Council | after ST, before LS4b — its own program plan, for approval |
| EX | Export a player (R288, ruled 2026-10-24): one folder — the player, the lesson, its videos copied in — that opens straight into Run mode; after LS4c | ruled direction — `docs/plans/export-player.md` |
| LS4d | Your own templates ("save it as my template") | |
| LS5 | Render locks for exports; Lottie slots | |
| LS6 | The teaching components and their lints | |

**Chat is parked after RM1** (#279; R283 order). What comes next there:
1. A pass to make Codex smooth. It was offered on 2026-10-20 and is not yet
   approved; the findings are known broken #11.
2. CW2 (Claude Code).
3. RM2–RM5, the rooms.

The shared-state research of 2026-10-20 settled the design:
- **No driving the desktop apps.** The app's room log is the truth.
- **Codex** gets it injected through `thread/inject_items`.
- **Claude** runs one persistent stream-json session.
- **Optionally**, a local MCP "room" server lets the desktop apps join as
  guests.

| Step | What | State |
|---|---|---|
| CW1 | Codex on the operator's subscription | **done** (#278). Four checks are owed on the operator's own clicks: Workspace mode creates a file without a card; Read only declines with a note; the mode survives a restart; Update Codex works |
| RM1 | Roles and rooms as data (R284), and the Rooms & roles view | **done** (#279) |
| CW2 | Claude Code on the operator's subscription | after the Codex pass |
| CW3 | The rooms passes: RM2 room log and tabs, RM3 participants and local endpoints, RM4 Council, RM5 the rest | after CW2 |

K1, K2 and A1 no longer gate anything: each step moves only the code it
touches, straight to the final layout.

**`docs/plans/docs-one-truth.md`** (approved 2026-10-12): P1 merged (#262).
P2 (one design doc per subject; ADRs into `adr/`) and P3 (`design/library.md`)
wait behind chat. P2 inherits:
- **#222's R8**, the coach unification program and ADR-019: folded into the
  coach design doc as a proposal for approval (operator, 2026-10-19);
- **two ADR-011s**: `docs/plans/rendering-stack.md` still titles itself
  ADR-011, which is Imagine generation (`docs/design/adr-011-imagine-generation.md`);
- **`docs/plans/wiredfront-library-and-player.md`**: parts A and C landed
  (#211); part B (the library caches) never landed and is
  `media-experience-program.md` Arc A.

**D0, laptop dependencies** (`docs/plans/d0-laptop-deps.md`): D0.0, D0.1a
(#258) and D0.1b (#260) are done; D0.1c is folded into the L passes; D0.2
(startup check), D0.3/D0.4 (camera), D0.5 (gate truth) and D0.6 (portable
release) are still to come.

## Known broken (verified 2026-10-12 unless dated)

1. **A Gallery rescan hides material that is still on disk.**
   - `reindex` scans only the top level of `content/inbox`
     (`crates/index/src/lib.rs:449-474`, `:823-860`).
   - Every row is stamped `location:"inbox"` (`:873`).
   - Hidden: ViziWiz bucket captures, renders, elements, pack ingests,
     `.mov`/audio/Lottie imports. → L1.
2. **"Link in place" silently copies when Windows refuses a symlink.**
   - `import_reference` falls back to a full copy and still records
     `source:"reference"` (`crates/composer/src/import.rs:230-234`).
   - Developer Mode was turned on 2026-10-12, and the app can now create
     symlinks unprivileged. The silent copy must still go. → L passes and
     D0.2.
3. **The library is about 15 parallel systems, not one.** WiredFRONT and
   Workbench see different material. → `docs-one-truth.md` §3, L passes.
4. **Four folder-finding walkers disagree** (content root, `locate_asset`,
   `dpl-doc` seeds, a dead `dpl-comp` copy). → L passes.
5. **The shipped bundle shares `assets/` with the working ring.**
   `promote_asset` would delete a shipped backdrop (latent: it is reached
   only from `tests/promote_drill.rs`). DQ7 ruled: separate them. → L passes.
6. **The editor status bar is too short for its two lines.**
   `status_h = 58.0` in `crates/composer/src/lib.rs`. Line 2 straddles the
   frame at 1600×900 and 1280×800.
7. **A Lottie renderer that exists but fails is re-opened every frame.** → D0.2
   follow-up.
8. **`content/artifacts.db.corrupt`** (2026-07-24) is unexplained. Never
   delete it; investigate it read-only.
9. **The release zip ships without the tool depot.** → D0.6.
10. **Six ignored drills do not run against the depot.** → D0.5.
11. **Chat is not whole yet, and Codex is not smooth** (verified
    2026-10-20 with a headless probe turn, run A/B).
    - **The depot's Codex is incomplete.** Adoption copied only `bin/`.
      The Codex package (layout v1) also has `codex-resources/`, which holds
      `codex-command-runner.exe` and `codex-windows-sandbox-setup.exe`, and
      `codex-path/`, which holds `rg.exe`. Without them, a sandboxed shell
      command fails ("sandbox helper is missing") and asks to run
      unsandboxed. The operator's full install ran the same turn with no
      card.
    - **Noise:** each unmapped app-server notification becomes an UNKNOWN
      card, about 14 per short turn (`item/started`, `item/completed`,
      `thread/status/changed`…). The work Codex did is not shown in plain
      words.
    - **The wrong folder:** Codex works in the app's current directory (the
      build folder when the exe is double-clicked), and `thread/start` sends
      no `cwd`.
    - **Nothing carries over:** there is no `thread/resume` or `thread/list`,
      so every launch starts a new thread.
    - **Still to build:** Claude has no runtime (CW2); there is no shared
      state, no saved conversation, and no tab close or clear (RM2); rooms
      cannot run yet (RM3, RM4).
12. **A list or data card loses its main content** (2026-10-21, LS2a).
    Since LS4a (2026-10-24) title, section, quote, big number, callout and
    media are drawn by their component; agenda, features, timeline, split,
    compare, stats, code and the infographics still paint only their words
    until LS4b, and the interactive ones until LS4c. A picture slot that
    holds a moving pattern (the callout's stripes, the big number's rings)
    reaches no slide yet (LS4b).
13. **A second launch opens a second, half-broken window** instead of raising
    the running one (`FjallError: Locked`, seam port bind os error 10048).
    The operator asked (2026-08-05) for the running window to come to the
    front. This needs its own PR.
14. **No engine shows its startup** (R276). Per-engine states (Starting /
    Ready / Error, plus progress) → D0.2.
15. **Analytics can write but not query.** DataFusion (R241) is not in the
    dependency tree.
16. **The UI still states the old split** (the wiredFRONT blurb's
    "ADR-010 is this hub's law"; the Library card's "Raw material, never app
    ingredients"). → L passes.
17. **`cargo green` never builds the `eguipilotai-devtools` feature**
    (2026-10-19). That is how #248's rename left it uncompilable for two
    months. → D0.5 (gate truth).
18. **Harness drills that print to stdout break EguiPilotAI** (2026-10-19).
    `--run-workflow` and `--dry-run-workflow` print their verdict to stdout,
    which is the pilot's stdio transport, so a fixture cannot use them.
19. **Grok shows "Not installed"** (2026-10-19) although the depot ships
    0.2.112. Presence runs bare names from PATH while the runtimes use the
    depot, and Grok's cached sign-in is not detected. → A7.
20. **#222's open orphans** (each finished per R282, each with its own plan;
    re-checked 2026-10-19):
    - `dpl-estate`'s `Tree::Docs` and `Tree::Manifest`: no reader or
      writer;
    - `engine-media::ink`: kept for a presenter annotate-and-save that never
      landed;
    - `dpl-engine-ae`: no dependent crate;
    - `RemoveProviderWorkspace`: never built (→ the chat program);
    - `promote_asset`: see #5.
21. **At 1100×700 Author's "Put them in" sits below the fold**
    (2026-10-24, found by the LS4a scenarios): the Building-into bar is the
    last thing on the page, so a short window has to scroll to reach it.

## Hazards outside this repo

- **Public CI uses hosted runners.** Both estate registrations were removed
  on 2026-10-30. Never reconnect them to public PR workflows. Merge requires
  successful hosted Windows and Linux checks (the double-green law).

- **`D:\localdev\platform_devpulse`'s `cargo xtask run-dev`** (often a
  parallel Claude session) runs `taskkill /F /T` on **every** `xtask.exe` at
  startup. A `cargo green` that dies silently with exit 1 was killed by it:
  check its `run-dev.log`, then re-run. Never touch that project.
- **A screenshot's pixel size is the window times the attached display's
  scale.** `--window <w>x<h>` is in points, and the footer shows the `dpr`.
  This laptop has run on a 150% display and on a 2560×1440 monitor at 100%.
  Compare runs by the footer's `dpr`, never by pixel size across days.
- **A cold hosted build takes longer than a cached build.** Inspect the job's
  active step before treating a slow run as hung.
- **The second clone, `D:\localdev\devpulselabs`**: the operator authorized
  an isolated ST1 worktree here on 2026-10-25. Do not push its unrelated
  local branches (`wip/c1-paused`, `park/…`) or change the source checkout.

## Next, in order

1. **Portable, linked storage** (`docs/plans/portable-storage.md`): ST1,
   ST1b, ST2a/b, ST5a, ST5a′ and ST5b are done. Next is **ST3** (§4.2:
   switch a library between the program folder and a drive), then ST4. The operator's own USB drive check (§6) is owed, and
   their two pinned themeforest folders index the first time that drive
   is plugged in.
2. **Owed on the operator's own setup:** the move (open the exe → Bring in
   my existing library… → Move it); then Run a lesson (#296) on the 75″
   with the script on the laptop, Swap screens, touch to advance.
3. **AgentHub** (R290), after ST: its own program plan, for approval. Its
   plan sets how it meets chat's queue (item 5); the Council still comes
   after Codex and Claude Code each work on their own (R33).
4. **LS4b** (lists and data) → **LS4c** (the gestures: drag to connect,
   slide away to reveal) → **EX** (export a player,
   `docs/plans/export-player.md`) → **LS4d** (your templates), then LS5 and
   LS6 — the order ruled 2026-10-24 (R287, R288).
5. **Chat**, when the operator returns to it:
   - the Codex-smoothness pass (known broken #11), for approval;
   - CW2, Claude Code;
   - RM2–RM5, the rooms.
6. Grok and Gemini auth health.
7. The rest of the incomplete work, each with unit tests and real-run
   drills: the eight dependency PRs (#268–#275: ACP 2→3.4, arrow and parquet
   59→60, zip 8→9, jsonschema 0.49→0.58, thorvg-sys 0.3→0.4, egui 0.36.2),
   adopted properly; #222's open orphans; the known-broken list.
8. P2, then P3 for operator approval; the L passes; D0.2 onward. The twelve
   EguiPilotAI fixtures with no scratch `CONTENT_ROOT` get one before any
   of them runs again.
9. **The knowledge engine (R275–R277)**: its own plan, from the 2026-10-13
   sourced research (one store with a project id on every row; keyword
   search first, embeddings one datapoint; `llama-server --embedding` as a
   depot subprocess; honest per-engine startup; an LLM lab in Workbench
   `/experiments`).
