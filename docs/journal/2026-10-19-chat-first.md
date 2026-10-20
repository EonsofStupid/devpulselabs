# 2026-10-19 — chat-first: the #222 port lands, Codex is next

Milestones `orphan-consumers` (landing) and `chat-cw1-codex` (next).
Checkout: **`D:\projects\devpulselabs`**.

## Done so far

- **#276 merged** (`b72926f`, double-green; the Windows lane took 1 h 6 min).
  - The editor's panels are salted, with a regression test.
  - The "debug DPI" finding was two displays, not two builds.
  - The release exe was rebuilt 15:03 and carries the stamp
    `b72926f · 2026-10-19 19:02z`.
- **The #222 port** is on `feat/orphan-consumers` (this PR):
  - the job board, preset looks and workflow dry run (R282);
  - the status bar's lanes stop overlapping at 1100×700;
  - the status menus are exclusive;
  - the devtools build compiles again (#248 had left `BUILD_CARD_AUTHORED`
    in the coach-signal catalog);
  - #222's R0 doc corrections re-checked and applied where still true;
  - `color-palette.png` renamed `WORKBENCH-editor-2.png`;
  - R282 and R283 added;
  - the chat program reordered (§4.4).
  - Proof: four shell tests, each red before its fix; the dry-run test; and
    the `jobs-board` EguiPilotAI scenario at maximized and 1100×700, which
    passes.
- **Chat, measured on the real app** (R283's note):
  - a Codex turn fails with HTTP 400 "'gpt-6-astra' requires a newer version
    of Codex": depot 0.145.0-alpha.18, gate `>=0.145,<0.146`, the operator
    on 0.163.0;
  - Claude has no runtime;
  - there is no shared state, no saved conversation, and no tab close or
    clear.
- **Ruled (operator, 2026-10-19):**
  - "Approve, Codex first": CW1 Codex, CW2 Claude, CW3 shared room;
  - land the #222 port in the background;
  - #222's R8 goes into P2's coach doc as a proposal.

## Exact next step

1. Watch this PR's two lanes. Merge on double-green, rebuild the exe from the
   merge, and close #222 with a comment that links this PR and says where
   each part went.
2. **The CW1 plan** (a new pass plan, `chat-cw1-codex`), grounded in-session, for
   operator approval before any code:
   - an in-place, SHA-verified depot update (`stage-tool-deps` refuses an
     existing root today);
   - Codex from the official release at the operator's version;
   - the app-server schema re-vendored;
   - the gate re-pinned;
   - server requests answered;
   - JSON-RPC errors surfaced;
   - auth health from `account/read`, not the text of `codex login status`;
   - plain-language failures with actions.
   Done means a real Codex turn in the real app.

## Landmines

- `platform_devpulse`'s reaper kills every `xtask.exe`: a silent exit 1
  means re-run.
- A `--run-workflow` fixture breaks EguiPilotAI (stdout is the transport):
  tap Run in a scenario instead.
- EguiPilotAI cannot screenshot the chat peer window (a deferred viewport).
  Capture it from the screen by its process window handle instead.
- Never read vendor credential files; checking that they exist is the limit.
- `.vscode/` and `out/` are untracked and not ours.

## Verify commands

```sh
cd /d/projects/devpulselabs
git fetch -q && git log --oneline -3 origin/main
gh pr list --state open
target/release/eguipilotai.exe test devpulse-windows tests/eguipilotai/jobs-board.toml jobs-board-small
```
