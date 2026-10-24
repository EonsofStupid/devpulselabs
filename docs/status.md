---
updated: 2026-10-21
active_lane: lesson-schema
dirty: false
---

# Status — where DevPULSE Labs is right now

This page is **rewritten, never appended**. History is `git log`, the PRs,
and the tag `docs-history-2026-10-12` (every doc before the one-truth
reorganization).

Read in this order: this page → [`requirements.md`](requirements.md) (the
operator's words) → [`README.md`](README.md) (the map of every doc).

**Updated:** 2026-10-21, main at `4ee638b` (#279). The release exe was
rebuilt 2026-10-20 15:46 from `4ee638b` and its build stamp verified
(`4ee638b · 2026-10-20 19:45z`). This page lands with LS1.

## Now

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
| LS1 | Component contracts: `dpl.component/1`, a `component.json` beside each of the 28 templates, the gallery demo built from each contract's first example, the Workbench reading the contracts, the `contracts` phase of `cargo green` | **in PR #280** (CI waiting: the estate runners are offline) |
| LS2a | Slides by id (`dpl.slide/3`): its own id and revision, blocks as the component's slots, the library folder, Layout and Starter taps that start from what was tapped, read-only seeds, the one-time upgrade of the six v2 drafts with backups | **draft PR #281**, stacked on LS1 |
| LS2b | Presets by reference: the five looks are card presets (`for`, `exposes`, `tone`, and a font default for their lines, ruled 2026-10-21); a slide stores the look's id; the cascade is slide → look → show | **draft PR #282**, stacked on LS2a |
| LK1 | Linked looks (R286): the look chip and its menu (your changes, Undo all my changes, Stop following / Follow the look again), a dot on every value that is yours with a comparison and a one-tap way back, try-before-keep, More controls, one revision per typing burst, Undo on every tab | **built**, stacked on LS2b |
| LK2a | Your own looks as files (`dpl.look/1`, the `Looks` registry passed to every surface that paints): Save as a new look, your looks first in the tray and in the Library, the missing-look state | **built**, stacked on LK1 |
| LK2b | Changes that spread: Make these the look's new style ("changes 12 other slides", before and after, one Undo), your version of a built-in with Back to the built-in, Open the look (in the card editor, ruled 2026-10-22) | |
| LK3 | Text, quote and picture looks (the approved sets) with the same chip, dots and stop-following, per block | |
| LS3 | Lessons (`dpl.lesson/1`) and episodes built from lessons (`dpl.episode/2`), by reference, with "Update available" | |
| LS4 | Components render their slots and variant axes | |
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
12. **A card made from a layout shows the layout's words, not its look**
    (2026-10-21, LS2a). The document renderer paints every slide as a plain
    stack, so until text looks (LK3) and component rendering (LS4) land,
    the lines all share one size.
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

## Hazards outside this repo

- **Both estate CI runners are OFFLINE** (2026-10-21): `devpulse-runner-win-01`
  and `devpulse-runner-lnx-01`. PR #280's lanes have queued since 15:11Z.
  Nothing merges until the operator brings Proxmox VM 119 and LXC 121 back
  and both lanes pass (the double-green law).

- **`D:\localdev\platform_devpulse`'s `cargo xtask run-dev`** (often a
  parallel Claude session) runs `taskkill /F /T` on **every** `xtask.exe` at
  startup. A `cargo green` that dies silently with exit 1 was killed by it:
  check its `run-dev.log`, then re-run. Never touch that project.
- **A screenshot's pixel size is the window times the attached display's
  scale.** `--window <w>x<h>` is in points, and the footer shows the `dpr`.
  This laptop has run on a 150% display and on a 2560×1440 monitor at 100%.
  Compare runs by the footer's `dpr`, never by pixel size across days.
- **The Windows CI lane takes 40 to 70 minutes** on the estate runner (#276:
  1 h 6 min). A slow run is not a hung one; check the job's steps first.
- **The second clone, `D:\localdev\devpulselabs`**, is unused. Never work
  there, and never push its local branches (`wip/c1-paused`, `park/…`).

## Next, in order

1. **LS1, component contracts**: in PR. Done means every template has a
   contract that the gallery demo is built from, and the contracts gate is
   green.
2. **LS2–LS6**, each with its own plan (the table under Now).
3. **Chat**, when the operator returns to it:
   - the Codex-smoothness pass (known broken #11), for approval;
   - CW2, Claude Code;
   - RM2–RM5, the rooms.
4. Grok and Gemini auth health.
5. The rest of the incomplete work, each with unit tests and real-run
   drills: the eight dependency PRs (#268–#275: ACP 2→3.4, arrow and parquet
   59→60, zip 8→9, jsonschema 0.49→0.58, thorvg-sys 0.3→0.4, egui 0.36.2),
   adopted properly; #222's open orphans; the known-broken list.
6. P2, then P3 for operator approval; the L passes; D0.2 onward.
7. **The knowledge engine (R275–R277)**: its own plan, from the 2026-10-13
   sourced research (one store with a project id on every row; keyword
   search first, embeddings one datapoint; `llama-server --embedding` as a
   depot subprocess; honest per-engine startup; an LLM lab in Workbench
   `/experiments`).
