# 2026-10-24 — components-ls4: LS4a, the component engine

Milestone `components-ls4`, pass LS4a. Plan: `docs/plans/components-ls4.md`,
APPROVED 2026-10-24 (untracked; deleted when LS4 completes). Checkout:
**`D:\projects\devpulselabs`**, branch `feat/ls4a-component-engine` from
`8cd3d0e` (the LS3b2 journal tip, stacked on draft PR #290).

The operator's rulings (plan §1, §4b):

- **Option A** — the ratified render tree, phased: LS4a (engine + the six
  text components) → LS4b (lists/data) → LS4c (interactive, touch
  sub-events) → LS4d (your templates).
- **Add props.** **Approve visual changes — shots shown before merge.**
- **Three styling levels**: layout only by default; "save as my template"
  = LS4d; a one-off variant on a slide or on one lesson's use = LS4a.
- **Order**: LS4a → run a lesson → portable storage (plan now, build after
  LS4a) → LS4b → LS4c → LS4d.
- R287 (the operator's Q2 answer, verbatim in plan §1) is still to be
  recorded in `docs/requirements.md`.

## Done so far

- **WIP commits** on the branch: `2155da4` (layout as contract data, slot
  defaults in the cascade, props, a use's variant), `a087bc0` (the renderer
  reads the layout; six text components get layouts), `65586bd` (the six
  templates paint through the one renderer; `slide_from_parts`), `8ae0a78`
  (the editor's Layout row, the Lessons page's "Layout" picker, tests).
- **Uncommitted on top of `8ae0a78`**: `SlotLayout.font` + `tracked`
  (schema, validation, `when` merge), the font cascade (own → line look →
  card look → slot → theme), the tracked paint in `document.rs`, and the
  micro-label slots in the six `component.json`s set to
  `"font": "jetbrainsmono", "tracked": true`. dpl-doc 101 and dpl-slides 41
  pass.
- **Feel-bar shots** (before = release exe at main `b69944a`; after = the
  debug devtools exe) in the session scratchpad `ls4a\`. Title: close to the
  old look once the tracked mono eyebrow is back. **Section divider**: the
  after is a centred stack; the old had a giant number top-left, a hairline
  rule, and the kicker and title pinned to the bottom. **Pull quote**: the
  after lost the oversized mark and the hairline rule, and paints the
  theme's warm wash instead of the dark card with the violet tone.

## Later the same day

- **The feel bar failed, then passed.** The first after-shots lost the
  callout's card, the quote's mark and rule, the section's composition and
  the stat's centring. Plan step **4b** (amendment) makes the decorations
  data: `container`, `anchor`, `mark`, slot `place: top`, slot `rule`, and
  the card's colour painted at last. After 4b the six match the old anatomy
  (`docs/proof/ls4a-*.png`, before/after). The plan's §8 lists the
  trade-offs.
- **EguiPilotAI:** `layout-make`, `lessons-layout` and their `-1100`
  variants pass. The card's `slide.json` says `align: center`, and the
  lesson's use says `align: left`.
- **#286 (LK3a) merged** (`74f7f1d`). LK3b was rebased (tree identical),
  pushed as `152c563`, retargeted to main and marked ready; its poll is
  running. The exe was rebuilt from main at 15:07 (stamp
  `74f7f1d · 2026-10-24 19:06z`).
- **The portable storage plan** is drafted: `docs/plans/portable-storage.md`
  (untracked, Q1–Q6 open).
- **LS4a is in draft PR #291** (stacked on #290). Its proof is complete:
  23/23 LK and LS scenarios re-run on the first try. Shots for the operator
  are in `docs/proof/ls4a-*.png`.
- **R288** (the operator, 2026-10-24) sets the destination: an external
  drive that links videos into slides, and an **exported player**. Ruled
  the same day: "A folder: player + lesson", "Copied into the export",
  "After the gestures". New stub `docs/plans/export-player.md`. The run-a-
  lesson, storage and LS4c plans now name R288.
- **Run a lesson** is planned: `docs/plans/run-a-lesson.md` (untracked,
  Q1–Q7 open).
- **CI fixed** (the operator: "fix whatever is causing it to not be
  available"):
  - **#292:** `actions/checkout`'s `git clean -ffdx` deleted `target/`
    before every run, so the cache now lives outside the checkout.
  - **#293:** the gate's test phase overrode the linker with a full path.
    cargo fingerprints the linker, so clippy and tests rebuilt each other's
    build scripts and proc-macros (`ConfigSettingsChanged`). #293 also adds
    the data-disk path (`DPL_CI_CACHE_ROOT`, `-DataDisk`, "Scaling a
    runner").
  - **The result:** Windows goes from 45–58 min to **6m45s** warm (main
    after #293: clippy 1, test 14, release 13 compiled). The proof is on
    #293.
  - **Still the operator's:** the 150 GB disk on the Proxmox host. This
    machine can't SSH there (the host key isn't trusted).
- **Landed:** #286, #287, #288, #289, #292, #293. #290 (LS3b2) was rebased
  (`f747229`), retargeted to main and marked ready.
- The LK3 plans and the CI plan are deleted (landed).

## Exact next step

1. When #290 is double-green: merge, `git fetch`, then
   `git rebase --onto origin/main 8cd3d0e feat/ls4a-component-engine`. The
   `git diff --stat <old tip> HEAD` may show only main's CI files
   (`green.yml`, `crates/xtask/src/main.rs`, the two runbook files).
2. Push with `--force-with-lease=feat/ls4a-component-engine:791a9831e73c11246ecca12fad0c45e91404923d`,
   then `gh pr edit 291 --base main`. Keep it a **draft** until the
   operator has seen the shots (their ruling Q4).
3. Rebuild the exe from main after every push.
4. Delete `docs/plans/lessons-ls3.md` once #290 lands.

## Landmines

- **The pre-push gate rebuilds the release exe from the branch.** After
  every push: check out main, `cargo build --release -p devpulselabs -p
  dpl-lottie-render`, verify the stamp, return to the branch.
- **The gate occupies the working tree** — no tracked edits or branch
  switches while it runs. Untracked plan files are safe.
- **Landing drops** (`git rebase --onto origin/main <old> <branch>`, then
  `git diff --stat <old tip> HEAD` must be empty): LS3a drops `39792a9`,
  LS3b1 `ff1f263`, LS3b2 `2fccd3c`, LS4a `8cd3d0e`.
- **The lease is the REMOTE tip, as a FULL sha.** `--force-with-lease=<b>:<sha>`
  must name exactly what `git ls-remote` shows. A wrong or guessed lease is
  refused as "stale info" only after the whole gate has run (this cost one
  gate on LK3b). LS4a's is `791a9831e73c11246ecca12fad0c45e91404923d`.
- **Never pipe a push through `head`.** It kills the gate mid-run.
- **A parallel `platform_devpulse` run-dev kills every `xtask.exe`** on the
  machine at startup. A silent exit-1 gate means checking the process list
  (`Get-CimInstance Win32_Process`, by CreationDate) and pushing again.
- **EguiPilotAI** overrides the frame clock when it injects actions, so
  animated walls can stay blank — prove those with `--screenshot`. It
  drops clicks now and then (re-run from a cleared scratch root), cannot
  scroll or open combo boxes, and each run has a 30 s limit.
- **Twelve fixtures in `.eguipilotai.toml` have no `CONTENT_ROOT`** and
  would run against real content. Do not run them.
- Python through a bash heredoc corrupts `\n` / `\\` escapes. Write
  scripts with the Write tool and run `python -X utf8`.

## Verify commands

```sh
cd /d/projects/devpulselabs
git status --short && git log --oneline -6
gh pr checks 286 && gh pr list --state open
cargo test -p dpl-doc -p dpl-slides -p editor -p workbench
```
