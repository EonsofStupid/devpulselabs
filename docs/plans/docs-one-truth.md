# One truth — the docs become one organized set, and the library becomes one

**Status:** PLAN, for operator approval. No file is moved or deleted before
approval. Grounded 2026-10-12 at `aa81b7d` (three read-only sweeps of the
whole repo, with the key claims re-checked by hand).

## 1. Context

**The operator, 2026-10-12:**

> "CHECK THIS REPO I HAVE TYPED THIS OUT OVER AND OVER AND OVER AND OVER WE
> CLEARLY NEED TO ORGANIZE AND PROPERLY GET A FUCKING DOCUMENTATION
> ORGANIZED I AM IN PRE-RELEASE STOP PARALLEL STOP LEAVING OLD STOP DOING
> LEGACY AND FUCKING IT IS ALL ONE GOD DAMNIT"

The same demand was already made on **2026-08-04**, and it was never
finished: *"we do not move forward until we have my vision captured and our
documentation merged into cohesive source of truth."*
(`docs/design/visual-language.md:4`, an unfinished "CONSOLIDATION DRAFT").

On **2026-08-05** the operator made the library ruling that this plan
carries: *"we are in pre-release and need to get this actually aligned to
best practices and get this properly authored so we can actually have
content be a seamless mixture of content we embed, import, or symlink once
we run it on a system"* (`docs/plans/content-provenance.md:7`). That plan
was never executed.

Why it keeps happening: every pass wrote a new doc and left the old one in
place. A session reads one doc and misses the operator's ruling in another.
The D0.1c roots design (2026-10-12) did exactly that. It walled the shipped
material into a separate silo, against the 08-05 ruling, because that ruling
lives in one file.

## 2. Goal / non-goals

**Goal:**
- **Every subject has exactly one canonical doc.**
- **One requirements ledger** holds every operator quote verbatim.
- **One short status file.**
- **Finished, superseded and stale docs are deleted.** Git keeps them.
- **References follow in the same PR.** Every reference to a moved or
  deleted doc is updated: CLAUDE.md, AGENTS.md, skills, hooks, the xtask
  gate, and code comments.
- **The one-library design is written down.** It covers embedded,
  imported and linked material, one index, and one way to find files. That
  is the law the code passes then build to.

**Non-goals for the docs passes (P1–P2):**
- No code behaviour changes.
- No operator quote is edited.
- `/content` is never touched.

The code that makes the library one is its own program (§4.4), and each of
its passes is planned and approved separately.

## 3. Ground truth (2026-10-12, `aa81b7d`)

**The docs today:**
- **325 markdown files, 63,013 lines.** 209 of them are plans (45,793
  lines).
- **Only 74 files (19,981 lines) are still the governing truth.** The other
  251 (43,032 lines) are finished pass plans and journals (211), superseded
  (24) or stale (16).
- **Four files compete for "where are we":**
  - `CLAUDE.md:3-5` says "read STATE first".
  - `STATE.md:3-6` says it is *not* the resume truth.
  - `ops/SESSION.md` is frozen at 2026-08-10.
  - `roadmap.md` still names the superseded chat docs as canonical.
  - Only `ops/resume/CURRENT.md` lines 1–81 are current.
- **Four design-system docs each claim to be the truth:** `design-system.md`,
  `coldlight-matrix.md`, `doctrine.md` and `visual-language.md`.
- **There is no library doc.** Its authority is split across ADR-014,
  `l1-library-doctrine.md`, `wiredfront-experience.md` §7,
  `guided-landing-charter.md`, `experience-inventory.md`, and the
  unexecuted `content-provenance.md`.
- **ADRs are scattered.** They live in both `design/` and `plans/`, and
  ADR-011 is used twice.
- **The only requirements ledger covers chat and AI only.**
  `chat-native-program.md` §4.1 (R1–R41) misses the library, linking,
  WiredFRONT/Workbench, the editor and preset rulings, and the visual
  language.
- **About 150 distinct operator quotes are scattered over the docs.** The
  library and linking quotes (*"effortlessly symlink them and then cache"*,
  *"browse and link outside of the app to shares/and drives"*, *"do both …
  symlinked … option to ingest"*, *"embed, import, or symlink"*) each live
  in only one or two files.

**The library today (code):**
- **About 15 separate library-like systems; only one is the index.**
  - **Through the index:** ViziWiz captures, plus composer
    import/reference/ingest/render/export.
  - **Bypassing the index:** Browse roots, the pack shelf, COMPS/RENDERS
    disk scans, seeds, episodes/shows, the Shows hub walk, Author drafts,
    compiled presets/tokens, the shipped `assets/`, the Imagine cache, the
    Ring-1 `assets/captures`, and the legacy `shows/*/episodes` capture
    tree.
- **A rescan hides the operator's material.**
  - `reindex` (`index/src/lib.rs:449-474`) scans only the **top level** of
    `content/inbox` (`:823-860`).
  - Every row is stamped `location:"inbox"` (`:873`), and any row not found
    at the top level is soft-cleared.
  - So a Gallery "rescan" hides ViziWiz bucket captures, renders, elements,
    pack ingests, and `.mov`/audio/Lottie imports. *(Re-checked by hand.)*
- **"Linked" is a silent copy on this laptop.**
  - `import_reference` (`composer/src/import.rs:230-234`) falls back to a
    full copy when Windows refuses the symlink, and still records
    `source:"reference"`.
  - This laptop refuses: Developer Mode is off, and file and folder
    symlinks are denied. *(Probed 2026-10-12. Junctions work.)*
- **One `rel_path`, two resolvers.** WiredFRONT joins paths onto the
  content root. Workbench and slides use `locate_asset`, which tries
  exe/repo roots first (`video/src/lib.rs:280-317`). The same `src` can
  resolve to a different file.
- **WiredFRONT and Workbench see different material.**
  - Slides minted in WiredFRONT never appear in Workbench.
  - A Kept guide disappears from Workbench.
  - Pictures are cut to the newest 61 *before* filtering.
  - "Elements" means different things in each house.
- **Legacy and dead code** (about 24 items, each with file:line in the
  sweep):
  - `find_episode_dir` + `capture-cli` (the old `shows/*/episodes` layout);
  - `promote_asset` (no runtime caller; would delete shipped backdrops);
  - `comp::find_upward` (dead);
  - index adoption of `artifacts.db`;
  - 9 never-clobber copies, 4 media-kind classifiers, 3 thumbnail
    resolvers, 2 COMPS listers, 2 render paths;
  - two poster conventions (`<stem>.jpg` vs `<stem>-poster.jpg`);
  - a per-frame disk walk in the Shows hub.

**Constraints a move must respect** (each re-checked):
- `xtask` `docs_status` reads `docs/eguipilotai-playwright-review.md` by
  exact path (`crates/xtask/src/main.rs:390`). It is part of `cargo green`.
- The harness-flag census reads the `- Harness flags` block in `CLAUDE.md`
  (`main.rs:1153`). That block stays where it is.
- `design/architecture.md` is gate law. It is amended only together with
  the xtask audit table.
- `.claude/ledger/session-start.sh:16-21` prints STATE, SESSION and CURRENT
  in full every session. `:28` picks the newest `docs/journal/*.md`.
- `.cursor/hooks/*` read the same files and SESSION's frontmatter.
- Skills name doc paths: milestone-loop (dated journals), session-os,
  coldlight-system (`doctrine.md`, `pass0-…`), eguipilotai, port-capture,
  egui-build, true-verify, slide-catalog-pass.
- Code comments in `crates/` name 43 docs, and 3 of those references are
  already dangling.
- `chat-native-program.md:179-209` cites line ranges in 18 docs, and
  `external-reach-inventory.md` cites file:line in code.
- `green.yml` skips `docs/**`, so a docs PR proves both lanes through
  `workflow_dispatch`.

## 4. Design

**Operator rulings, 2026-10-12** (this plan approved: *"Approve"*):
- **Old docs: delete them, tagging first.** The tag
  `docs-history-2026-10-12` is created, every quote is moved into
  `requirements.md`, and then the files are deleted. There is no archive
  folder and no SUPERSEDED leftovers.
- **Status: one short `status.md`.** It replaces STATE, CURRENT and
  SESSION. Hooks and CLAUDE.md read it.
- **Linking: real Windows symlinks, through Developer Mode.**
  - The operator turns on Developer Mode (Settings → System → For
    developers). That is a system setting the agent never changes.
  - Symlink evaluation on this laptop (queried 2026-10-12): local→local
    and local→remote are ENABLED, so a library link to a share or another
    drive resolves. remote→local and remote→remote are DISABLED, which
    matters only if the library itself sits on a share.
  - The library **never silently copies**. A refused link is an honest
    error.
  - D0.2's startup check verifies that a symlink can be created and
    resolved on this machine.
- **DQ7: separate the shipped material from the working `assets\` now**
  (ruled earlier the same day).

### 4.1 The doc set (target)

```
README.md                    the repo front door (what it is, how to build, where the docs are)
docs/
  README.md                  THE MAP: one line per doc, what it is the truth for
  requirements.md            THE operator ledger: every quote verbatim, dated, by subject (R-ids)
  status.md                  NOW: what works, what is broken, what is next (rewritten each pass)
  product.md                 the one product: the houses, what each is for, how they connect
  design/                    the laws, ONE doc per subject
    architecture.md          layer law (gate-enforced; unchanged path)
    library.md               ONE library: roots, rings, embedded/imported/linked, index, finding files
    design-system.md         ColdLight: tokens, roles, matrix, paint laws, icons, the RULED register
    wiredfront.md            the raw editor: contract, mock, workflows
    workbench.md             the build house: library walls, editor, studio, presets, color
    chat-ai.md               chat + AI providers/subscriptions (from chat-native-program)
    coach.md                 the coach seam
    capture.md               ViziWiz, sessions, guides
    signals.md               the signal doctrine
    release.md               portable release + tool depot
    eguipilotai.md           the harness
  adr/                       numbered decisions only, one folder, numbers made unique
  plans/                     ONLY passes in flight or next; a landed plan is deleted at merge
  runbook/                   ci-runners, sccache, adobe-exit
  research/                  live reference research only
  proof/                     proof images
  journal/                   the newest handoff only (the hooks read it)
```

**Rules that keep it one:**
- **One home per quote.** A quote lives **only** in `requirements.md`.
  Every other doc cites R-ids and never re-quotes. A ruling that replaces
  an earlier one marks the old R-id *superseded by* the new one in the same
  place, so contradictions can't hide in two files.
- **No "SUPERSEDED" banners.** When a doc's subject gets a new truth, the
  old doc is deleted in the same PR, and the canonical doc absorbs whatever
  is still law.
- **A plan's lifetime.** A pass plan lives in `plans/` while it is in
  flight. At merge its evidence goes into the PR and `status.md`, and the
  plan is deleted. Git and the PR keep it.
- **`status.md` stays short.** It is rewritten, not appended: now / known
  broken / next, under 200 lines. History is `git log` and the PRs.
- **Names.** Lowercase kebab-case; one doc per subject; ADRs as
  `adr/NNN-title.md`.

### 4.2 What is deleted

- **All finished pass plans and journals, and every superseded or stale
  doc:** 251 files, 43,032 lines (the per-file list is in the inventory).
  The newest journal is kept.
- **Restorable with one command.** Before the deletion commit, the tag
  **`docs-history-2026-10-12`** marks the last tree that holds them all.
  `git show docs-history-2026-10-12:docs/plans/<name>.md` returns any of
  them.
- **Quotes move first.** Every operator quote in a deleted doc moves into
  `requirements.md` *before* the doc is deleted.

### 4.3 The one library (`design/library.md` — the law the code builds to)

From the operator's words: R-A29 *"embed, import, or symlink"*, R-B5
*"effortlessly symlink them and then cache"*, R-B6 *"browse and link
outside of the app to shares/and drives"*, R-A14 *"do both … symlinked …
option to ingest"*, and R-C39 *"WiredFront … raw … to then import into my
slides via the workbench editor"*:

- **One library, one index, one list.**
  - Every item has a **provenance**:
    - **Embedded** — shipped with the app, read-only;
    - **Imported** — copied in, the library owns the bytes;
    - **Linked** — left where it lives (a drive, a share); the library owns
      only the link; it can be offline, and it is relinkable.
  - WiredFRONT and Workbench read the **same** index and show the same item
    the same way.
- **One way to find a file.** One resolver turns a library reference into
  a path for every caller. No probing of folders near the exe; no current
  directory.
- **The index covers the whole library**, not only the top of `inbox`. A
  rescan never hides material that is still on disk.
- **Shipped material is embedded material.** It lives in its own folder
  beside the app (**DQ7, ruled 2026-10-12: separate it from the working
  `assets\` now**), is indexed as Embedded, and shows in both houses. It is
  not a silo.
- **Locations: existing folders join the library in place.** The operator,
  2026-10-12: *"there has to be a way in windows where we can add existing
  folders as locations and if the portable app starts up on another system
  and its not available it offers to be deleted but that should be treated
  like a symlink for lack of my knowledge"*.
  - **Adding a location.** A folder (a drive, a share, a pack folder) is
    added as a **location**. The library records it, with its path and the
    volume's identity, and indexes what is inside **in place**. Nothing is
    copied. Every item in it is **Linked**.
  - **When it is missing.** If the app starts and a location is not there
    (another machine, an unplugged drive, a share that is down), the
    location shows **offline**. It never vanishes. The operator is offered
    **Locate…** (point at where it is now; relink) or **Remove** (deletes
    the location record, never the operator's files).
  - **Prior art.** This is Lightroom's missing folder and Resolve's offline
    media.
  - **It replaces two lists.** Pinned packs (`content/elements/sources.json`,
    which already shows unplugged sources as offline) and Browse libraries
    (`.userdata/libraries.json`) become this one Locations feature.
- **Per-file links are real symlinks.** Developer Mode was turned on
  2026-10-12 and proven: `cmd mklink` creates file and folder symlinks
  unprivileged, and `dpl-index`'s
  `a_folder_symlink_is_unlinked_never_recursively_deleted` now **runs**
  instead of skipping. A dangling symlink (its target missing on another
  machine) is offline with the same Locate/Remove offer.
- **Linking never silently copies.** If a link cannot be made, the operator
  is told, and nothing is duplicated behind their back.
- **Cache.** Thumbnails and posters for linked material are cached in
  `.userdata\cache` (regenerable), so a slow share is browsed fast, per
  R-B5 *"symlink them and then cache"*.

### 4.4 Order of work

| Pass | What | Code? |
|---|---|---|
| **P1** | `requirements.md` (every quote, verified against its source line) · `status.md` (replaces STATE + CURRENT + SESSION) · `docs/README.md` · tag, then delete the 251 · update CLAUDE.md, AGENTS.md, skills, hooks, `.cursor`, and code comments | docs + hooks only |
| **P2** | Merge each subject into its one design doc: design-system, wiredfront, workbench, chat-ai, coach, capture, release, eguipilotai, signals. ADRs into `adr/` with unique numbers. Each merge keeps every law verbatim and drops only what is history. | docs only |
| **P3** | `design/library.md` written from §4.3, for operator approval | docs only |
| **L1…** | The code program that makes the library one, each pass planned and approved: the index covers the whole tree + provenance; one resolver (absorbs D0.1c roots); links that never silently copy; delete the legacy and parallel copies listed in §3 | code |

D0 continues after P1–P3, because D0.1c is now L-program work.

## 5. Wiring

| What | Change |
|---|---|
| `CLAUDE.md` | Read order: `docs/status.md` → `docs/requirements.md` → `docs/README.md`. Session end: update `status.md`. The harness-flag block is untouched. |
| `AGENTS.md` | Doc pointers updated. The broken old-repo `editor-spec` reference is removed. |
| `.claude/ledger/session-start.sh` | Prints `status.md` (short) instead of 2,689 lines of STATE, plus SESSION and CURRENT. |
| `.cursor/hooks/*`, `.cursor/rules/*` | Same files. SESSION's frontmatter role moves into `status.md` (or stays in a tiny `ops/session.yaml` if a hook needs machine-readable fields). |
| Skills | milestone-loop, session-os, coldlight-system, eguipilotai, port-capture, egui-build, true-verify, slide-catalog-pass: each path updated to the canonical doc. |
| `crates/xtask` `docs_status` | Either keep `docs/eguipilotai-playwright-review.md` in place, or move it under `design/eguipilotai.md` and change `main.rs:390` in the same commit. |
| Code comments in `crates/` | Each of the 43 doc references points at the canonical doc. The 3 dangling ones are fixed. |
| `chat-native-program.md` §4.1 | Its line-range citations become R-ids in `requirements.md`. |

## 6. Proof

- **A link check.** Every `docs/…md` path named anywhere in the repo
  (docs, CLAUDE.md, AGENTS.md, skills, hooks, `.cursor`, `crates/`, `.github`)
  resolves to a file. The result is printed as a count, with zero dangling.
- **Every quote is accounted for.** Each quote in the 2026-10-12 harvest maps to
  an R-id, and spot-checks against the source line at the history tag show
  it is verbatim.
- **The gate passes:** `cargo green`, including `docs status` and the
  structure harness census.
- **Both lanes pass** through `gh workflow run green.yml --ref <branch>`.
  Merge only on double-green.
- **A fresh session.** A new session's start hook prints `status.md` and
  the newest journal, and nothing names a deleted file.

## 7. Risks

- **Deleting history.** The tag keeps every file one command away, and
  quotes move before docs go.
- **Merging law docs can drop a law.** P2 merges keep laws verbatim, and
  each merge lists the laws it carried.
- **A parallel session (Cursor Session OS) edits the same docs.** Check
  `gh pr list` and the working tree before each pass, and land P1 in one PR.
- **A huge diff.** P1 is mechanical (moves, deletes, path rewrites) and
  reviewable by its file list. P2 merges are one subject per commit.

## 8. Evidence

### P1 (2026-10-12, branch `docs/one-truth`)

- **The tag.** `docs-history-2026-10-12` → `aa81b7d`, holding all 325 docs.
  It was pushed through the gate (`green ✔`).
- **Quotes, verified before anything was deleted.**
  - Harvest 1 found 171 quotes (library, linking, the houses, process,
    portability). Harvest 2 found 100 (chat/AI, coach, capture, slides,
    signals and the rest, plus R1–R28's sources). Each was cut from its
    source by anchors, never typed.
  - `verify_quotes.py` compares each quote with its source at the tag after
    stripping markdown and line wraps. Result: **271 / 271 verbatim**.
  - The checker was proven able to fail. A one-word change ("symlink" →
    "hardlink" in A29) gave `FAIL A29 … matches 195/230 chars`.
  - Four quotes from the operator's session messages had no repo doc. They
    are recorded with their source stated: R28 (2026-10-10) and three from
    2026-10-12.
  - **`requirements.md` holds 275 requirements** under 275 distinct R-ids.
    R1–R41 keep the chat program's numbers; R7, R16 and R19 have two quotes
    each (a/b). New ids run R42–R274.
  - A cross-check confirms every quote appears verbatim in the file.
  - Six quotes of inferred attribution (H13, J4, K13, K23, K29, K35) are
    marked, and so are five ratified agent wordings.
- **Triage.** Of the 152 quote-like passages the first harvest missed, 49
  were the operator's and 103 were not. The agent prose, UI text and error
  strings are listed in the triage file.
- **One kept doc was on the delete list:**
  `design/wiredfront-capture-share-spec.md`. It is the operator's own spec,
  filed verbatim, so it is kept.
- **Deleted: 247 docs.** 82 markdown docs remain, and the journal folder
  holds only the newest handoff.
- **References.**
  - 167 citation lines in 82 files were repointed to
    `docs-history-2026-10-12:<path>` (markdown links become GitHub URLs at
    the tag).
  - Old-app paths are now marked `old-app:<path>`.
  - The coach daemon's `/health` field `adr` now names the governing
    `coach-seam-adr.md`.
  - CLAUDE.md, AGENTS.md, README, the skills, both grounder agents, the
    Cursor rule and the Claude and Cursor session hooks all read
    `status.md` → `requirements.md` → `README.md`.
  - `check_doc_links.py` (every `docs/…md` path named anywhere resolves,
    markdown links included) reports **0 dangling doc references**.
- **Developer Mode.** The operator turned it on (`AllowDevelopmentWithoutDevLicense
  = 1`). `cmd mklink` creates file and folder symlinks unprivileged, and
  `dpl-index` `a_folder_symlink_is_unlinked_never_recursively_deleted`
  **runs and passes** instead of skipping. Windows PowerShell 5.1's
  `New-Item -ItemType SymbolicLink` still says "Administrator privilege
  required", because it does not request the unprivileged flag. Rust's
  `symlink_file` does.
