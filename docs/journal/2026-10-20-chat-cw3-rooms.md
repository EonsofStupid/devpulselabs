# 2026-10-20 — chat-cw3-rooms: RM1, roles and rooms as data

Milestone `chat-cw3-rooms`, pass RM1 (plan `docs/plans/chat-rooms-rm1-library.md`,
approved 2026-10-20: Q1 "Approve", Q2 first runnable room "Council"; never
committed, deleted when it lands). Checkout: **`D:\projects\devpulselabs`**,
branch `feat/chat-rooms-rm1` from `1c472a2`.

## Done so far

- **CW1 merged** as #278 (`1c472a2`, double-green). The exe was rebuilt
  14:15 with the stamp `1c472a2 · 2026-10-20 18:13z`.
- **The research** is `docs/research/chat-rooms-and-roles.md`. Part 1 has 47
  numbered sources; part 2 has lettered keys (A, C, G, M, X, P, R, L, S, T).
  It is untracked until the RM1 commit, and the chat-core tests
  `include_str!` it, so it must ship in the same PR.
- **Local commit `cc993f7`** (wip):
  - schemas `schemas/{role,room}.v1.schema.json`;
  - `crates/chat/core/src/rooms.rs`: the types, `validate_role`,
    `validate_room`, `Library::{builtin, with_user}`, and 6 tests;
  - the built-in library `crates/chat/core/library/{roles,rooms}/*.json`
    (11 roles, 5 rooms).
- **The RM1 commit** (on top of `cc993f7`):
  - **The window:**
    - "♜ Rooms & roles" replaces the dead "Councils · not qualified" label;
    - the library view in the AI pane, with room-type and stance filters
      (pilot ids `chat.library/filter/{room,stance}/<schema name>`);
    - collapsible details for every room and role;
    - "Open library folder".
  - **The host:** `load_room_library` (the operator's files, unreadable ones
    named), `reveal_folder`, and the `--chat-proof library` preset.
  - **Tests:**
    - kittest: the button opens the library, every built-in is listed, a
      stance filter narrows it;
    - host: the operator's file replaces a built-in only at a higher
      version; a broken file is named;
    - chat-core: the schema names are pinned to serde.
  - **The real-app proof:** the EguiPilotAI scenario
    `tests/eguipilotai/chat-rooms-library.toml` (fixture `chat-library`,
    scratch content root) passed 10/10 on the devtools exe. Its first run
    caught a duplicate pilot id (the filter chip and the room header were
    both `room/brainstorm`); the chips moved under `filter/`.
  - **Docs:**
    - R284;
    - the program's §4.3.7 amendment and the RM1–RM5 order in §4.4;
    - the README map row;
    - `status.md`;
    - the council passage in `product-vision.md`, which had named a
      `ChatDomain::Council` that no longer exists.

## Exact next step

1. `git push -u origin feat/chat-rooms-rm1` (the gate runs), then
   `gh pr create`, then `gh pr checks <n> --watch`. Merge only on
   double-green.
2. Rebuild the exe (`cargo build --release -p devpulselabs -p
   dpl-lottie-render`) and check its mtime and stamp. Delete the untracked
   plan `docs/plans/chat-rooms-rm1-library.md`.
3. Then CW2 (Claude Code), then RM2–RM5.

## Landmines

- Never drive the operator's live desktop (no SendKeys, clicks or screen
  grabs) without consent. Prefer headless kittest or EguiPilotAI.
- `.vscode/` and `out/` are untracked operator files from 2026-10-02. Never
  add them.
- A collapsing header's accessible label is its pilot name ("Council room",
  "Skeptic role"), not the header text. Tests query those names.
- `LibraryIssue`'s `at` already names the file and field, so print it with
  `Display`.
- `platform_devpulse`'s run-dev reaper kills `xtask.exe`. A silent exit-1
  gate means run it again.

## Verify commands

- `git -C /d/projects/devpulselabs status --short` and `git log --oneline -3`
  (expect `cc993f7` on top of `1c472a2`).
- `cargo test -p dpl-chat-core` (6 rooms tests) and `cargo test -p
  dpl-chat-window` (47).
- `gh pr list`: check for parallel work on rooms before pushing.
