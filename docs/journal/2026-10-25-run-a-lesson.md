# 2026-10-25 — run-a-lesson: RL, one pass

Milestone `run-a-lesson`. Plan: `docs/plans/run-a-lesson.md`, APPROVED
2026-10-25 (untracked; deleted when RL lands). The operator's answers,
asked in-chat:

- Q1 "Both" — run a lesson or an episode;
- Q2 "Yes, locked Run";
- Q3 "Slides on 75", script on laptop";
- Q4 "Tap anywhere + presenter drives";
- Q5 "Beats now, card taps in LS4c";
- Q6 "One per card, shared";
- Q7 "Both in one pass".

Checkout **`D:\projects\devpulselabs`**, branch `feat/rl-run-a-lesson` from
`b4b4f9a` (main, #291 LS4a).

## Before this pass, the same day

- **The stack landed:** #286–#291, the CI fixes #292–#293, docs #294–#295.
  The Windows lane went from 45–58 min to about 7 (main after #293: 6m45s).
  The CI cache can move to its own data disk; attaching it is the
  operator's step on the Proxmox host.
- **R288** (the external drive, the exported player) and **R289** (what
  portable means) are recorded. The run-a-lesson and portable-storage
  plans are approved; export-player is a stub.

## RL — what landed (see the plan's §8)

- **dpl-doc:** `Showing { Episode, Lesson }` with one shared walk
  (`lesson_shown`). Scripts gain `render_script`, `save_script` and
  `follow_reveals`; an empty cue parses and the doctor names it.
- **Fabric:** `DeckCommand::Run { target, start }` on its own channel.
- **Deck:** opens a `Showing` at a start card, with `next`, `back` and
  `jump`. Touch: tap anywhere / left 15% back / swipe / long press.
- **Present hub:** drains Run; full-bleed plus OS full screen; the
  presenter opens at once (pilot viewport `present.presenter`) with Back /
  Next / jump / Swap screens; places per machine in `.userdata/present-
  screens.json`; "Run ›" in the picker.
- **Workbench:** Run this lesson; ▶ Run beside each card's Job; Run on the
  episode wall; "N taps · script" on each card; the On tap panel in the
  card sheet (the tab made real) and the Lessons editor; the tab rail
  wraps when its tabs don't fit.
- **`--run-lesson <id>`** (CLAUDE.md harness list).

## Exact next step

1. Push `feat/rl-run-a-lesson`, open the PR (#296 is named in
   `docs/status.md` — check it), double-green, merge, rebuild the exe.
2. Then **portable storage** (ST), approved: ST1 → ST2 → ST5 → ST3 → ST4.
   Write its §4c work queue against the real files first.

## Landmines

- **Proof-shot exe runs are sequential.** Scenario runs use the devtools exe
  `target/debug/devpulselabs.exe`; a `cargo test` mid-run can replace it.
- **The presenter's OS window can't be captured** (eframe's immediate
  viewport). Prove its content with `--presenter-drill N` (panel mode).
- **The deck's entrance is frozen under EguiPilotAI's clock.** Prove beats
  with `--run-lesson <id> --screenshot --shot-frame N`.
- Scenario `rl-make` fills the On tap fields. If the panel's layout grows,
  `cue-1` falls below the fold at 1100×700 (the harness can't scroll).
- The lease is the REMOTE tip as a full sha. Never pipe a push through
  `head`. A `platform_devpulse` run-dev kills every `xtask.exe` at its
  startup.

## Verify commands

```sh
cd /d/projects/devpulselabs
git status --short && git log --oneline -6
gh pr list --state open
cargo test -p dpl-doc -p dpl-deck -p dpl-hub-present -p dpl-editor -p dpl-hub-workbench
```
