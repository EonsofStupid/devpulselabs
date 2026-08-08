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
| L7 | `fabric::jobs_running`: defined, self-tested, never read — the half-finished coaching-plane surface (the jobs-board plan deferred "a jobs inspector panel") | **RESTORED-at-R1 (2026-08-07)** — THE JOB BOARD inspector built; `jobs_running` is its header truth, `ack_job` its ack chips, every control pilot-tracked for the coach plane | `crates/fabric/src/lib.rs:954`; consumer `shell::jobs_popup` |
| L8 | `workflows::DryRunner`: the "honest dry run" had no caller outside its own tests | **RESTORED-at-R1 (2026-08-07)** — `dry_run_workflow` seam + Dry-run chip + `--dry-run-workflow` flag | `crates/workflows/src/lib.rs:722`; consumers in composer/hub/app |
| L9 | `coldlight::treated_coord`: pub fn, had zero callers anywhere including tests | **RESTORED-at-R1 (2026-08-07)** — the preset picker's rows wear their presets through it | `crates/coldlight/src/lib.rs:651`; consumer `shell::preset_row` |
| L10 | `dpl-engine-ae`: whole crate, zero dependents; its intended consumer replicated `Caps::probe` inline (`composer/src/lib.rs:6294`). Honestly recorded in STATE/journal — the loss is the unscheduled wiring, not the record | ORPHANED | `crates/engine-ae/`; STATE.md:50-57,117 |
| L11 | `RemoveProviderWorkspace` intent: ratified in the chat-provider contract, never built, no analogue symbol at all; `RetryProvider`/`SignOutProvider` shape-changed into `RecoveryActionKind::{Retry, SignIn}` without the contract being reconciled | NEVER-BUILT / PARTIAL | `docs/plans/chat-provider-workspaces.md:78-86`; `crates/chat/core/src/lib.rs:569` |
| L12 | Coach C1 §4 "LANDED" bullet claims the Test Lab "coach" tab and CLI form `coach mount <node> --once`; the tab was never on any branch (deliberately deferred to Slice 8, self-contradicted at :438) and the CLI grammar shipped differently | PARTIAL | `docs/plans/operator-coach-a2ui.md:249-256,438`; lab `enum Tab` has 5 tabs |
| L13 | `pilot_interact`: phantom API — introduced as prose, never as Rust, propagated into 6 docs; real API is `pilot_track` (139 call sites) | NEVER-BUILT (doc-phantom) | `docs/plans/eguipilotai.md:87` + 5 more |
| L14 | Portable packaging census row marks fetch-ffmpeg ✅ — the checkmark points at the OLD Tauri app's `scripts/fetch-ffmpeg.mjs`; the rewrite has only a resolver (`tools-deps`), no fetcher, no package-portable, and the data_root law row is ❌ | NEVER-BUILT | `docs/plans/census.md:66`; `crates/tools-deps/Cargo.toml` |
| L15 | R1u "ingredient LIBRARY (`library_ui`)" reads as live in STATE.md; deleted by W1.2 (`9a95fc7`) under the two-libraries law, and **W1.2 is recorded nowhere in STATE.md** | LOST (deletion ruled; record missing) | `docs/STATE.md:2067-2071` |
| L16 | `brutal_frame`/`clay_frame`: symbols gone (superseded by `cl::treated`), survive only as doc-comment epitaphs; the STATE entry names them without noting the retirement | LOST (superseded; record stale) | `docs/STATE.md:2120-2121` |
| L18 | **The coach/eguiPilot split (operator question 2026-08-07: "where did the live-overlay code go? was Coach wrongfully created separate?").** Censused: NOTHING was lost — desktop-spike + coach-overlay + coach-inventory/fabric/runtime + the axum daemon (127.0.0.1:7843) are all on main, proven live (25 PASS/1 SOFT verb battery on Calculator; foreign spotlight painted — `docs/proof/c2s-foreign-spotlight.png`; Cursor Ctrl+K located). Stranded on two axes: (a) 100% `cfg(linux)` — on the Windows seat every verb Refuses, the overlay stubs exit 2, inventory is empty (Windows/UIA lane = deferred C7); (b) the unification seams were all cut — Pass-2 stdio shared-protocol loop parked day one, so two verb vocabularies (protocol `Touchpoint`/`CoachStyle` vs the 101-verb catalog), two transports (pilot stdio vs axum HTTP), two event streams (fabric COACH domain vs unstamped SSE), hand-copied palette, zero crate references between stacks. Also: `CoachStyle::{Arrow,Outline,DimAround}` protocol-only (refused "not painted yet"), no ScreenRect/fabric-id target arm, A2UI Surface commands (C3) not started | PARTIAL / STRANDED — ruled 2026-08-07: one system (debug mode + coaching mode), unify by EXPANDING eguiPilot → Arc R8 | census 2026-08-07, three reports; `crates/coach-*`, `crates/desktop-spike`, `crates/egui-pilot-ai/src/coach.rs` |
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

### Arc R1 — the orphans get their missing halves (code) — EXECUTING

Re-ruled 2026-08-07 (§5 amendment): no deletions. Each orphan's original
ask is excavated, then its missing consumer built.

- **L7 `jobs_running`**: BUILT 2026-08-07 — THE JOB BOARD inspector
  (`shell::jobs_popup`): clickable pilot-tracked jobs cell, board rows
  with live progress, ack chips → `ack_job`, `jobs_running()` as the
  header truth, "jobs · N done" when outcomes wait unacked. The board's
  ack contract has its consumer; the coach plane can mount it and register
  clicks. Gate: `jobs_running`/`job`/`ack_job` external call sites ≥1
  each (met); build + tests green; proof shot of the open board.
- **L8 `DryRunner`**: BUILT 2026-08-07 — `Composer::dry_run_workflow`
  (refusal contract mirrors Run; no disk, no run folder, no job facts, no
  spend), the Dry-run chip beside Run in the Workflows tab
  (pilot-tracked, `composer.workflows.dry`), and the `--dry-run-workflow
  <id>` harness flag through hub + app + the CLAUDE.md census. Gate:
  external call sites ≥3 (chip dispatch, hub seam, app flag — met);
  `dry_run_workflow_previews_without_touching_disk` green.
- **L9 `treated_coord`**: BUILT 2026-08-07 (Q1 reversed) — the preset
  picker's rows now WEAR their presets through `treated_coord` (full
  treatment, hover/worn states, resolved inks) instead of painting flat
  color chips. Gate: external call site ≥1 (met); build + tests green;
  proof shot of the picker.

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

**L5**: wire `Tree::Docs` (the in-app docs/guide surfaces are the natural
writer per the estate doctrine) and `Tree::Manifest` (run manifests
already exist in `dpl-protocol`; persisting them is the natural reader).
Per the §5 amendment there is no shrink option — hollow partitions are
half-finished implementations and get finished. Gate: each partition has
≥1 production writer AND reader.

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

### Arc R8 — coach comes home: eguiPilot's coaching mode, whole (ruled 2026-08-07)

Operator ruling, verbatim essence: "this and eguiPilot are the same — we
have its debug mode and coaching mode… we need to just properly have it
properly expanding eguiPilot and then get the mock to fully capture this
cutting edge concept: the app sees the installed applications, an axum
fabric creates the seam for that application to be manipulated/opened/
reviewed, and reliably paints live circles, arrows, floating commands
above terminal etc."

Sequenced (mock gate law — "mocks built first for a clear vision"):

1. **R8a — THE MOCK**, first and fully: the end-to-end concept in the
   house mock style (docs/design/mocks/) — installed-app census → per-app
   fabric seam → the live overlay vocabulary (ring, circle, ARROW,
   floating command panel) painted over a terminal — so nothing downstream
   is guessed. Fed by the existing research (docs/research/coach/*, the
   ratified ADRs, the C-series proofs) — the research is largely done;
   the mock is what was never built.
   **FILED 2026-08-07: `docs/design/mocks/coach-one-system-mock.html`** —
   interactive (click the coached target → Triggered → effect contract →
   Satisfied → grounded-progress tick; census drift → re-mint; mode
   toggle), every element badged SHIPPED/SPEC'D/NEW with provenance,
   ColdLight ladder engine + treatments verbatim, the operator's ask and
   the PBS gamify principle quoted verbatim. Render-verified headless at
   1400px. Awaiting operator redlines; R8b-R8e execute against it.
2. **R8b — one vocabulary**: reconcile protocol `Touchpoint`/`CoachStyle`/
   `TriggerKind`/`CoachEvent` with the 101-verb catalog; add the missing
   target arms (ScreenRect, foreign fabric id); paint the refused styles
   (Arrow, Outline, DimAround) in the ONE painter both coaches share.
3. **R8c — one transport + one event stream**: the overlay speaks the
   pilot protocol (the parked Pass-2 stdio loop, un-parked); daemon events
   enter dpl-fabric's stamped COACH domain; the axum door remains the
   agent HTTP seam but carries the same typed vocabulary.
4. **R8d — the Windows lane (deferred C7, now the seat that matters)**:
   UIA in place of AT-SPI, layered/transparent window in place of X11
   override-redirect, behind the already-platform-neutral
   `compose_screen_rect` algebra; developed and proven on the estate's
   Windows VM (devpulse-runner-win-01 on server1).
5. **R8e — the Teacher client**: the daemon's 19 shipped verbs get their
   first real caller (deferred three journals running).

Boundaries: the separate overlay PROCESS stands (egui-wgpu transparent
backbuffer constraint, ratified ADR); the two-surface rule (Operator
Coach vs Advanced/EguiPilot) stands — one system, two modes, two doors.

## 5. The rulings table — RULED 2026-08-07, then AMENDED the same day

**Operator ruling, 2026-08-07 (supersedes Q1 and every delete-shaped
default): an orphan is a half-finished implementation, never dead code.
"If it's not being called it's because the implementation was half-assed —
get it implemented, don't delete it. Find out why it was even implemented."
The recovery move for EVERY orphan is: git-archaeology the original ask,
then build the missing consumer.** Executed same-session:

- `treated_coord` (Q1 REVERSED — was "delete"): archaeology found its
  purpose in its own doc comment — "the preset path", the constructor for a
  surface to wear a matrix coordinate. Its missing consumer was the preset
  picker, whose rows painted flat color chips instead of the real look.
  BUILT: `preset_row` now wears each preset through `treated_coord` (full
  treatment — back-glow, shadows, edge; hover previews Hover, worn sits
  Active), inks from the preset's own resolution.
- `jobs_running` (retire-ruling REVERSED): archaeology found the deferred
  consumer named in the jobs-board plan's own out-of-scope list — "a jobs
  inspector panel" — and the board's ack contract waiting for it. BUILT:
  THE JOB BOARD popup (`shell::jobs_popup`) — the status jobs cell is now
  clickable and pilot-tracked, `jobs_running()` is the header truth,
  resolved rows carry pilot-tracked ack chips driving `ack_job`, unacked
  outcomes surface as "jobs · N done" instead of silence. Coach-plane
  ready: every control registers on the pilot tree.

Original table (Q1's default no longer stands; the rest hold):

| # | Question | RULED (2026-08-07) |
|---|----------|--------------------|
| Q1 | `treated_coord` — delete or keep? | ~~delete~~ REVERSED same day: complete it (see amendment above) |
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
