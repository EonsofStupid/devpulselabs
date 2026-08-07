# Program — the loss recovery: every verified loss, restored or ruled on

Status: **RATIFIED 2026-08-07** — operator ruling, verbatim: "not fastest
but right plan this out and execute defaults." Every §5 question is RULED
to its PROPOSED default. The ledger (§3) is measured fact; the arcs (§4)
are now law and execute in the §7 order.

## 1. Context

Operator ask, 2026-08-06, verbatim:

> "lets see if there is clear evidence of things to recover"

> "create a full list to methodically get them all restored methodically"

Method: a four-lane claim-verification probe over the corpus (docs/STATE.md,
48 journals, 188 plans, plus a git-side deletion hunt), run 2026-08-06
against `main` @ `bf860c7`. Every claim was tested the same way: does the
named artifact exist → external call-site count (wired vs orphaned) → if
absent, `git log -S` names the removing commit.

**Numbers: 153 claims verified. 136 WIRED. 17 failures.** Distribution:
journals 63/64 (nearly perfect), STATE.md 31/38, plans 42/51. The loss
lives in the plan layer, not the journal layer.

## 2. Rulings that bind this program (nothing here re-opens them)

- **30e35e7 (2026-07-22)** — home-scoping the backdrop was a misread; the
  backdrop is shell-owned and paints app-wide. Live in code today
  (`crates/backdrop/`, no backdrop dep in `crates/hubs/home`).
- **Two-libraries law** — Workbench library = ColdLight ingredients;
  WiredFRONT library = raw media. The W1.2 deletion of
  `composer/src/library.rs` enforced this; it is not a loss to restore.
- **Prove it with the number** — every restoration in §4 closes with a
  measured gate: external call-site count > 0 for a wire, or 0 remaining
  doc-claims for a retirement. No arc is "done" on an agent's say-so.
- **One target, one branch, one PR** — each arc is one branch, tested, one
  merge, branch deleted.
- **Doc and audit table move together** — every code arc updates its
  source-of-truth docs in the same PR.
- **Already superseded, do not duplicate**: "the library caches" part B
  (`poster_for`/`ThumbJob`/consent) is absorbed by
  `docs/plans/media-experience-program.md` Arc A; Gallery virtualization is
  that program's S2. This program references, never re-plans, both.

## 3. The ledger — every verified loss

Verdicts: **LOST** (existed, removed, claim still live) · **ORPHANED**
(exists, zero production call sites) · **PARTIAL** (some parts landed,
claim says all) · **NEVER-BUILT** (claimed/ratified, no trace in any
branch) · **DOC-HAZARD** (docs contradict a ruling or cite dead artifacts).

| # | Finding | Verdict | Evidence anchor |
|---|---------|---------|-----------------|
| L1 | ADR-011 rendering stack: resvg, egui_tiles+vello(+vello_svg), egui_dnd (W4b drag-reorder), egui_flex, egui_virtual_list/infinite_scroll — all absent from Cargo.lock and every branch; plan still says "ratified" | NEVER-BUILT | `docs/design/adr-018-rendering-stack.md` (at measurement: docs/plans/rendering-stack.md:18-79, refiled by R0); `git log --all -S` empty per dep |
| L2 | ADR-011 number collision: slot reassigned to xAI Imagine (`docs/design/adr-011-imagine-generation.md`); rendering ADR never filed in docs/design/; downstream plans cite two docs under one number; ADR-012 is a gap | DOC-HAZARD | `docs/plans/wiredfront-experience.md:132` vs `docs/plans/r1-editor-adr-proposal.md:7` |
| L3 | `poam-visual-experience.md` V1 "animation is a Home privilege" (not started, program first-mover) restates the misread reversed by 30e35e7; `l1-library-doctrine.md:21` reinforces | DOC-HAZARD | `docs/plans/poam-visual-experience.md:42-46,152` |
| L4 | `composer::import::promote_asset` (RING-1, ADR-014): transactional promotion reachable only from a test drill; the deferred "assets shelf, promote chips" pass never landed | ORPHANED | `crates/composer/src/import.rs:272`; callers: `tests/promote_drill.rs` only |
| L5 | `dpl-estate` partitions `Tree::Docs` + `Tree::Manifest`: zero production writers/readers (State + Agents are wired) | PARTIAL | `crates/estate/src/lib.rs:35`; external hits none |
| L6 | `engine-media::ink` (`flatten_ink`, `InkStroke`): kept at the ViziWiz-ink deletion "for a future presenter annotate-and-save". Precision (2026-08-07): deck ink D3 itself LANDED (`8772e30`, Program D complete per roadmap) — what never landed is the presenter **annotate-and-save** consumer the engine was kept for | ORPHANED | `54ba46d` body; zero non-test consumers |
| L7 | `fabric::jobs_running`: defined, self-tested, never read. **Re-measured 2026-08-07**: the shell jobs cell DOES consume the board — `shell/src/lib.rs:952-957` filters `fabric.jobs()` by `JobRow::running()` inline (it needs row labels, not a bare count). The capability is live; the accessor is a duplicate nothing needs | ORPHANED (accessor, not capability) | `crates/fabric/src/lib.rs:954`; external hits 0 |
| L8 | `workflows::DryRunner`: the "honest dry run" has no caller outside its own tests | ORPHANED | `crates/workflows/src/lib.rs:722` |
| L9 | `coldlight::treated_coord`: pub fn, zero callers anywhere including tests | ORPHANED | `crates/coldlight/src/lib.rs:651` |
| L10 | `dpl-engine-ae`: whole crate, zero dependents; its intended consumer replicated `Caps::probe` inline (`composer/src/lib.rs:6294`). Honestly recorded in STATE/journal — the loss is the unscheduled wiring, not the record | ORPHANED | `crates/engine-ae/`; STATE.md:50-57,117 |
| L11 | `RemoveProviderWorkspace` intent: ratified in the chat-provider contract, never built, no analogue symbol at all; `RetryProvider`/`SignOutProvider` shape-changed into `RecoveryActionKind::{Retry, SignIn}` without the contract being reconciled | NEVER-BUILT / PARTIAL | `docs/plans/chat-provider-workspaces.md:78-86`; `crates/chat/core/src/lib.rs:569` |
| L12 | Coach C1 §4 "LANDED" bullet claims the Test Lab "coach" tab and CLI form `coach mount <node> --once`; the tab was never on any branch (deliberately deferred to Slice 8, self-contradicted at :438) and the CLI grammar shipped differently | PARTIAL | `docs/plans/operator-coach-a2ui.md:249-256,438`; lab `enum Tab` has 5 tabs |
| L13 | `pilot_interact`: phantom API — introduced as prose, never as Rust, propagated into 6 docs; real API is `pilot_track` (139 call sites) | NEVER-BUILT (doc-phantom) | `docs/plans/eguipilotai.md:87` + 5 more |
| L14 | Portable packaging census row marks fetch-ffmpeg ✅ — the checkmark points at the OLD Tauri app's `scripts/fetch-ffmpeg.mjs`; the rewrite has only a resolver (`tools-deps`), no fetcher, no package-portable, and the data_root law row is ❌ | NEVER-BUILT | `docs/plans/census.md:66`; `crates/tools-deps/Cargo.toml` |
| L15 | R1u "ingredient LIBRARY (`library_ui`)" reads as live in STATE.md; deleted by W1.2 (`9a95fc7`) under the two-libraries law, and **W1.2 is recorded nowhere in STATE.md** | LOST (deletion ruled; record missing) | `docs/STATE.md:2067-2071` |
| L16 | `brutal_frame`/`clay_frame`: symbols gone (superseded by `cl::treated`), survive only as doc-comment epitaphs; the STATE entry names them without noting the retirement | LOST (superseded; record stale) | `docs/STATE.md:2120-2121` |
| L17 | Doc-rot cluster: (a) 12 dangling citations to reference PNGs renamed in `30fb73b` — incl. two evidence gates in `wiredfront-library-and-player.md:182,232` and `editor-2.png` misfiled as `color-palette.png`; (b) 4 plans cite deleted `crates/hubs/library` present-tense; (c) `c4-viziwiz-ink.md:50,75` + `roadmap.md:167` cite the deleted viziwiz.rs surface and `--ink-drill`; (d) PixVix names live in `census.md:87` + `eguipilotai-v1-delivery.md` (crate renamed viziwiz `54ba46d`); (e) `wiredfront-library-and-player.md` header carries no SUPERSEDED marker for part B | DOC-HAZARD | verifier reports, 2026-08-06 |

## 4. The restoration arcs — in order

Each arc = one branch, one PR. Verification gate stated per arc.

### Arc R0 — the docs tell the truth again (no code; executes existing rulings; FIRST)

The cheapest arc removes the only *active* hazard and stops every stale
claim from misleading the next session. All of these execute rulings that
already exist — no new decisions required.

1. **L3**: rewrite `poam-visual-experience.md` V1 to honor 30e35e7 — the
   perf goal (no idle repaints on static routes) survives; the "Home
   privilege" ownership shape does not. Fix `l1-library-doctrine.md:21`.
   Cite 30e35e7 in both.
2. **L2**: file the rendering stack as a real ADR in `docs/design/` under a
   fresh number (ruled Q3: ADR-018) with honest status
   (**proposed, unexecuted** — see Arc R3); the former
   plans/rendering-stack.md moves there; repoint the downstream
   citations so ADR-011 means only Imagine.
3. **L13**: `pilot_interact` → `pilot_track` across all 6 docs.
4. **L12**: correct the Coach C1 §4 LANDED bullet to match :438 and the
   shipped CLI grammar.
5. **L15/L16**: STATE.md — record W1.2 (library_ui deleted at `9a95fc7`,
   two-libraries law) beside the R1u entry; annotate the
   `brutal_frame`/`clay_frame` entry with the `cl::treated` succession.
6. **L14**: census.md packaging row — ✅ → ❌ with a pointer to Arc R6.
7. **L17a**: repoint the 12 PNG citations; rename
   `color-palette.png` → `WORKBENCH-editor-2.png` (PROPOSED — it is an
   editor concept mock, not a palette; byte-identical `git mv`).
8. **L17b-e**: fix the stale `crates/hubs/library`, viziwiz/`--ink-drill`,
   and PixVix citations; add the part-B SUPERSEDED header to
   `wiredfront-library-and-player.md` pointing at
   `media-experience-program.md` Arc A.
9. **L11 (doc half)**: reconcile `chat-provider-workspaces.md` §intents to
   the shipped `RecoveryActionKind` shape; mark `RemoveProviderWorkspace`
   as an OPEN GAP pointing at Arc R4.

Gate: grep for every dead name in this list returns 0 doc hits (or only
historical-context hits explicitly marked as such); `poam` V1 no longer
contradicts 30e35e7.

### Arc R1 — small wires and retirements (code, small, low risk)

- **L7 `jobs_running`**: RETIRE, not wire — re-measured 2026-08-07, the
  R1 bullet as first written sat on a stale premise. The shell's jobs cell
  already consumes the board (`shell/src/lib.rs:952-957`, `fabric.jobs()`
  filtered by `JobRow::running()` — it needs the row labels a bare count
  cannot give). Ceremonial wiring would double-lock the mutex to no end.
  Delete the accessor; keep the running-count truth covered by tests
  through `jobs()`. Gate: symbol gone, grep 0; fabric tests still assert
  the running-count behavior.
- **L8 `DryRunner`**: wire a `--dry-run` mode on the composer's workflow
  seam (`WfRunner` path) so the honest runner has a caller. Gate: ≥1
  external call site + one harness scenario exercising it.
- **L9 `treated_coord`**: PROPOSED **delete** (zero callers including
  tests; `Dress::Coord` + `matrix::resolve` are the live path). Gate:
  symbol gone, grep 0.

### Arc R2 — RING-1 completes: the assets shelf (code, medium)

**L4**: build the deferred "assets shelf, promote chips" surface so
`promote_asset` has a production caller and the three-ring promotion
(ADR-014) is operator-reachable. Scope from STATE.md:1713's deferral note.
Gate: `promote_asset` external call sites ≥1; a drill promotes a real
asset end-to-end and the original leaves `assets/` transactionally.

### Arc R3 — the rendering-stack ruling executes (rulings first, then code)

**L1** is four separate decisions, not one restoration (§5 table). What is
already clear: W4's own code comment (`crates/editor/src/lib.rs:11`)
promises "reorder / add / delete blocks are W4b" — the editor still cannot
reorder blocks, which is user-visible. PROPOSED:

- **R3a — W4b block reorder**: restore per the W4 rider. Implementation
  free to be hand-rolled egui drag or `egui_dnd` — the *capability* is the
  restoration, the dep pick needs its ruling. Gate: drag-reorder proven in
  a harness scenario.
- **R3b — list virtualization**: DEFER to `media-experience-program.md` S2
  (already planned there). This program only tracks closure.
- **R3c — resvg / vello / egui_flex rows**: PROPOSED **retire** (SVG is
  solved by ADR-015 `dpl-svg`; the compositor shipped on ffmpeg; flex
  never had a consumer). Retirement = the ADR-018 filing in R0 marks them
  superseded-with-reasons, not silence.

### Arc R4 — the chat contract closes (code, medium)

**L11 (code half)**: implement workspace removal — the one intent with no
analogue at all. The shape (a `RemoveProviderWorkspace` intent vs a
recovery-action-style affordance) follows the reconciled contract from R0.
Gate: an operator can remove a provider workspace from the UI; harness
scenario proves it; contract doc and code name the same verbs.

### Arc R5 — the estate fills its shelves or shrinks them (ruling, then code)

**L5**: PROPOSED — wire `Tree::Docs` (the in-app docs/guide surfaces are
the natural writer per the estate doctrine) and `Tree::Manifest` (run
manifests already exist in `dpl-protocol`; persisting them is the natural
reader). If the operator rules either partition speculative, shrink the
enum instead — a hollow partition is drift either way. Gate: each kept
partition has ≥1 production writer AND reader; each cut one is gone.

### Arc R6 — portable packaging becomes true in the rewrite (code, large)

**L14**: build the rewrite's own packaging story — `package-portable`
(exe + `bin/` + fonts + seeds + content layout per the ratified data-root
law) and the fetch/provision step ADR-009-style (tools land beside the
exe; `tools-deps` stays the resolver). This is the census row the ✅
falsely covered. Gate: a packaged folder runs on a machine with no dev
tree; census row flips to a truthful ✅.

### Arc R7 — the ink promise resolves (ruling, then code or deletion)

**L6 + L10**, the two "kept for a future that never came" orphans, both
WiredFRONT-adjacent:

- **`engine-media::ink`**: rule the presenter **annotate-and-save** in or
  out (precision per L6: deck ink D3 landed at `8772e30`; the
  annotate-and-save consumer the engine was kept for is what never came).
  In → schedule it with ink as its engine. Out → delete the module with a
  recovery ref (`54ba46d` carries the full 598-line surface for
  archaeology). Ruled Q8: decided at the next WiredFRONT program pass, no
  later than 2026-09-07.
- **`dpl-engine-ae`**: wiring belongs to WiredFRONT AE orchestration
  (aerender ratified 2026-08-06; `3538ab3`'s body already names WiredFRONT
  as "exactly where AE orchestration belongs"). Schedule the Phase-2
  wiring pass there; until then the xtask ceiling entry stays as the
  honest tombstone. Gate (when run): `dpl-engine-ae` gains ≥1 crate
  dependent and the inline `Caps::probe` replica in composer is retired.

## 5. The rulings table — RULED 2026-08-07, all defaults stand

Operator ruling 2026-08-07 ("execute defaults") ratifies every line:

| # | Question | RULED (2026-08-07) |
|---|----------|--------------------|
| Q1 | `treated_coord` — delete or keep? | delete (R1) |
| Q2 | `color-palette.png` → `WORKBENCH-editor-2.png` rename? | yes (R0) |
| Q3 | Rendering ADR refile number ADR-018? | yes (R0) |
| Q4 | W4b reorder: hand-rolled drag or adopt `egui_dnd`? | hand-rolled unless it fights egui 0.35 (R3a) |
| Q5 | resvg/vello/egui_flex rows: retire as superseded? | retire (R3c) |
| Q6 | Workspace removal shape: intent enum or recovery-affordance? | follow the reconciled contract from R0 §9 (R4) |
| Q7 | Estate `Docs`/`Manifest`: wire both, or shrink? | wire both (R5) |
| Q8 | D3 presenter ink: in or out? | defer to WiredFRONT review — but dated: rule at the next WiredFRONT program pass, no later than 2026-09-07 (R7) |
| Q9 | Arc order after R0/R1: R2→R4→R5→R6, with R3a slotted by appetite? | yes |

## 6. Not in this program (verified, closed, or owned elsewhere)

- "The matrix has zero call sites" — **refuted 2026-08-06**: 7 external
  runtime call sites since #211. No action.
- Library-caches part B — owned by `media-experience-program.md` Arc A.
- Gallery virtualization — owned by `media-experience-program.md` S2.
- Test Lab coach tab — deliberately deferred to Slice 8
  (`eguipilotai-v1-delivery.md:390`); R0 §4 only fixes the stale LANDED
  bullet.
- The chat rail hub, `composer/library.rs`, `hubs/library`, block→element
  moves, seed reshuffles, mock deletions — all verified deliberate-clean;
  records exist (except W1.2's, fixed in R0 §5).
- `MotionPolicy::Natural`/`FastForward` selector — honestly recorded open
  harness limitation; belongs to the eguipilotai lane's own backlog.
- PixVix → ViziWiz in **fixture IDs and scenario scopes**
  (`pixvix-two-window`, `devpulse.pixvix.*`) — public harness inputs, so a
  versioned-alias migration owned by the eguipilotai lane, not a doc edit;
  R0 fixed prose only (surfaced 2026-08-07).

## 7. Sequencing

```
R0 (docs truth, no code)          ← first, removes the active hazard
R1 (small wires + retirements)    ← same week, low risk
── rulings pass on §5 ──
R2 (assets shelf)                 → then R4 (chat contract)
R5 (estate)                       → parallel-safe with R2/R4
R6 (packaging)                    ← large, schedule as its own milestone
R3a (W4b reorder)                 ← slot by appetite after ruling Q4
R7 (ink / engine-ae)              ← rides the next WiredFRONT review
```

Every arc closes by updating this file's §3 row (verdict → RESTORED-at-PR#
or RETIRED-at-PR#) — the ledger is the audit table and they move together.
