# 2026-10-20 — chat-cw1-codex lands; rooms and roles researched next

Milestones `chat-cw1-codex` (in PR) and `chat-cw3-rooms` (research done
2026-10-20, plan next). Checkout: **`D:\projects\devpulselabs`**.

## Done so far

- **#277 merged** (`51e1600`, double-green). #222 was closed with a pointer
  comment. The exe was rebuilt 11:01 with the stamp
  `51e1600 · 2026-10-20 15:00z`.
- **CW1, branch `feat/chat-cw1-codex`** (plan `docs/plans/chat-cw1-codex.md`,
  approved 2026-10-19; never committed, deleted when it lands):
  - **Adoption** (`crates/chat/providers/codex/src/adoption.rs`): the
    operator's installed Codex is proven by Windows checking OpenAI's
    Authenticode signature on every file, by its version, and by its own
    protocol schema. `dpl-tools-deps` gains `place_tool`, `write_manifest`,
    `install_tool` and `reload_current`. xtask gains `adopt-tool`,
    `update-tool-deps` and `vendor-schema`.
  - **Codex 0.163.0 adopted** into the central depot (row 61 of the reach
    inventory) and into `target/debug` and `target/release`.
  - **The runtime:** every server request is answered (approvals become
    cards, the rest are refused by name); refused requests fail their own
    turn; a 120 s deadline; `account/read`.
  - **Permission modes** (Ask first / Work in the workspace / Read only),
    saved per provider and chosen in the chat header.
  - **The host:** Ready means the account ("Ready · ChatGPT Pro"); failures
    read in plain words; Update Codex.
  - **Real app (debug, scratch workspace) proved:**
    - "Reply with the single word OK." gets "OK";
    - the account line and the selector show;
    - an Ask-first request shows the card and waits;
    - Deny refuses.
- **Rooms and roles:** two research passes ran on 2026-10-20 (the science
  and frameworks of roles and rooms; participant plumbing for Codex, Claude,
  Gemini, Grok and local endpoints). Their findings feed the CW3 rooms plan.

## Exact next step

1. Watch CW1's two lanes. Merge on double-green, rebuild the exe from the
   merge, and verify the stamp.
2. The operator owes four real-app checks, by their own clicks or with
   consent to drive the desktop:
   - Workspace mode writes a file with no card;
   - Read only declines with a note;
   - the mode survives a restart;
   - Update Codex from the button.
3. The rooms and roles plan (CW3), from the research, for approval.

## Landmines

- **Never drive the operator's live desktop** (SendKeys, clicks, screen
  grabs) without consent (memory: desktop-automation-while-operator-present).
- EguiPilotAI cannot capture the chat peer viewport.
- `platform_devpulse`'s reaper kills every `xtask.exe`: a silent exit 1
  means re-run.
- The Windows CI lane takes 40 to 70 minutes.

## Verify commands

```sh
cd /d/projects/devpulselabs && git status -sb && gh pr list --state open
/c/Users/jessa/AppData/Local/Programs/OpenAI/Codex/bin/codex --version
cargo test -p dpl-chat-codex -p dpl-tools-deps -p dpl-chat-accounts
```
