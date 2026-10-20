---
updated: 2026-10-20
active_lane: chat-first
dirty: false
---

# Status — where DevPULSE Labs is right now

This page is **rewritten, never appended**. History is `git log`, the PRs,
and the tag `docs-history-2026-10-12` (every doc before the one-truth
reorganization).

Read in this order: this page → [`requirements.md`](requirements.md) (the
operator's words) → [`README.md`](README.md) (the map of every doc).

**Updated:** 2026-10-20, main at `1c472a2` (#278). The release exe was
rebuilt 2026-10-20 14:15 from `1c472a2` and its build stamp verified
(`1c472a2 · 2026-10-20 18:13z`). This page lands with RM1.

## Now

**Chat first (R283, ruled 2026-10-19: "Approve, Codex first").** On the
real app, chat does not work end to end (the R283 note has the evidence).
Every step now ends with a real message working in the real app
(`docs/plans/chat-native-program.md` §4.4, "The order"):

| Step | What | State |
|---|---|---|
| CW1 | Codex on the operator's subscription: the operator's own Codex adopted into the depot (OpenAI's signature, its version, its protocol), every server request answered, honest auth health, configurable permissions, plain-language failures, Update Codex | **done** (#278): a real Codex turn answers on the operator's ChatGPT Pro. Four checks are owed on the operator's own clicks: Workspace mode creates a file without a card; Read only declines with a note; the mode survives a restart; Update Codex works |
| RM1 | Roles and rooms as data (R284): the `dpl.role/1` and `dpl.room/1` schemas, 11 built-in roles and 5 rooms citing their research, the operator's own files, the Rooms & roles view | **in PR** |
| CW2 | Claude Code on the operator's subscription (A4): the official binary in the depot, a stream-json adapter, presence from the depot | next |
| CW3 | The shared room, built as the rooms passes: RM2 the room log (saved conversations; tabs as conversations with New, Close, Clear), RM3 participants, the context packet and local endpoints (Ollama first), RM4 Council, RM5 the other rooms | after CW2 (RM4 needs Claude) |
| then | Grok and Gemini auth health (A7/A8), C2–C4 | |

**Rooms and roles** (R284, 2026-10-20): research-backed roles (skeptic,
realist, dreamer and more) and rooms (council, debate, author + critic,
router, brainstorm) for Codex, Claude, Gemini, Grok and local endpoints. They
are authored data, so the library grows without code. The research is
`docs/research/chat-rooms-and-roles.md`. The operator approved the rooms
order, with Council as the first room that runs.

K1, K2 and A1 no longer gate anything: each step moves only the code it
touches, straight to the final layout.

**The #222 port** lands in the background in this PR: the job board, preset
looks and workflow dry run (R282), plus two defects found on the real exe
(the status bar's lanes overlapped at 1100×700; the devtools build had not
compiled since #248).

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
11. **Chat is not whole yet** (2026-10-20). Codex works (CW1, #278). Still
    missing:
    - Claude has no runtime (CW2);
    - there is no shared state, no saved conversation, and no tab close or
      clear (RM2);
    - rooms cannot run yet (RM3, RM4).
12. **A second launch opens a second, half-broken window** instead of raising
    the running one (`FjallError: Locked`, seam port bind os error 10048).
    The operator asked (2026-08-05) for the running window to come to the
    front. This needs its own PR.
13. **No engine shows its startup** (R276). Per-engine states (Starting /
    Ready / Error, plus progress) → D0.2.
14. **Analytics can write but not query.** DataFusion (R241) is not in the
    dependency tree.
15. **The UI still states the old split** (the wiredFRONT blurb's
    "ADR-010 is this hub's law"; the Library card's "Raw material, never app
    ingredients"). → L passes.
16. **`cargo green` never builds the `eguipilotai-devtools` feature**
    (2026-10-19). That is how #248's rename left it uncompilable for two
    months. → D0.5 (gate truth).
17. **Harness drills that print to stdout break EguiPilotAI** (2026-10-19).
    `--run-workflow` and `--dry-run-workflow` print their verdict to stdout,
    which is the pilot's stdio transport, so a fixture cannot use them.
18. **Grok shows "Not installed"** (2026-10-19) although the depot ships
    0.2.112. Presence runs bare names from PATH while the runtimes use the
    depot, and Grok's cached sign-in is not detected. → A7.
19. **#222's open orphans** (each finished per R282, each with its own plan;
    re-checked 2026-10-19):
    - `dpl-estate`'s `Tree::Docs` and `Tree::Manifest`: no reader or
      writer;
    - `engine-media::ink`: kept for a presenter annotate-and-save that never
      landed;
    - `dpl-engine-ae`: no dependent crate;
    - `RemoveProviderWorkspace`: never built (→ the chat program);
    - `promote_asset`: see #5.

## Hazards outside this repo

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

1. **RM1, roles and rooms as data**: in PR. Done means the Rooms & roles
   view lists the library on the real app.
2. **CW2, Claude Code** on the operator's subscription. Done means a real
   Claude turn.
3. **CW3, as the rooms passes**:
   - RM2, the room log. Done means Claude answers about Codex's reply in the
     same conversation, and it is all still there after a restart.
   - RM3, participants and local endpoints. Done means a local model cites
     Codex's reply.
   - RM4, a real Council.
   - RM5, the other rooms.
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
