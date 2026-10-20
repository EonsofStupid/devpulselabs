# 2026-10-20 — chat-cw1-codex: Codex on the operator's subscription

Milestone `chat-cw1-codex`. The plan is `docs/plans/chat-cw1-codex.md`,
APPROVED 2026-10-19 (Q1 "Approve"; Q2 "configurable permissions").
Checkout: **`D:\projects\devpulselabs`**, branch `feat/chat-cw1-codex` from
`b72926f`. Uncommitted until the CW1 PR; #277 owns the committed journal
until then.

## Done so far

- **#276 merged** (`b72926f`). The exe was rebuilt 15:03 with the stamp
  `b72926f · 2026-10-19 19:02z`.
- **#277** (the #222 port, plus chat-first docs R283) is open. CI was running
  at handoff (`gh pr checks 277`). On double-green: merge, rebuild the exe,
  and close #222 with a pointer comment.
- **CW1 grounding, verified in-session:**
  - the operator's Codex `C:\Users\jessa\AppData\Local\Programs\OpenAI\Codex\bin\codex.exe`
    0.163.0 and `codex-code-mode-host.exe` have valid Authenticode signatures
    ("OpenAI OpCo") and SHA-256s equal to GitHub's published `rust-v0.163.0`
    digests (`2692c8c7…`, `07f3869f…`);
  - the 0.163 schema is generated at
    `D:\.usr\Temp\claude\d--localdev-devpulselabs\d0ec6155-b7fb-480f-b3ea-5b7ca10f26c7\scratchpad\cw1\schema-0.163`;
  - every method and notification the adapter uses still exists in 0.163;
  - 0.163 has 10 server requests (5 approval kinds, tool call/input, MCP
    elicitation, attestation, `account/chatgptAuthTokens/refresh`), and
    `account/read` returns `{requiresOpenaiAuth, account: apiKey | chatgpt{email, planType} | amazonBedrock}`.

## Design decisions made while grounding (record them in the plan's §4 amendments)

- **No in-process WinVerifyTrust.** The workspace denies `unsafe_code` and
  `#[allow]` is banned, so the signature check runs Windows' own
  `Get-AuthenticodeSignature` through a quiet subprocess and parses the
  status and signer.
- **Adoption is split.**
  - `dpl-tools-deps` stays spawn-free (its ceiling is serde, serde_json,
    sha2). It gains only the pure-IO install: copy, hash, an atomic manifest
    rewrite, removal of the old version, and a reload of `resolve_current`
    (today a `OnceLock`).
  - The signature, version and protocol checks live in a new
    `crates/chat/providers/codex/src/adoption.rs`. CW2 moves the shared part
    to its final home in the AI boundary when Claude needs it.

## CW1a DONE (local commits `5f1956a`, `d47c083` on `feat/chat-cw1-codex`)

- The schema is re-vendored at 0.163.0, the version range is gone (the
  minimum is the pinned build), and `adoption.rs` exists (with 7 tests).
  `dpl-tools-deps` has `place_tool`, `write_manifest`, `install_tool` and
  `reload_current`; xtask has `adopt-tool` and `update-tool-deps`.
- The operator's Codex 0.163.0 was **adopted for real** (the Windows
  signature check passed). The central depot now has
  `artifacts/windows-x86_64/codex/0.163.0` (the old 0.145 artifact is left,
  as it is the operator's). `sources.local.toml` was rewritten (a backup is
  in the scratchpad at `cw1/sources.local.toml.bak`). `target/debug` and
  `target/release` were updated in place.
- **A real Codex turn works:** "Reply with the single word OK." gave "OK",
  usage 30040 of 258400.

## CW1b (local commit `fd94a7a`)

- **Runtime:** server requests answered (approvals become cards, the rest
  refused by name), refused requests fail their own turn, a 120 s deadline
  (paused while a card waits), and `account/read`.
- **Seam:** `SetPermissionMode` and `Account`.
- **Settings:** `PermissionMode` is saved in the accounts store and chosen
  in the provider header.
- **Host:** Ready means the account, failures read in plain words, and
  "Update Codex" adopts the operator's build in the background.
- **Real app (debug, scratch workspace) proved:**
  - "Ready · ChatGPT Pro" and the "Ask first" selector;
  - an Ask-first request shows the card and waits (nothing ran);
  - Deny refuses (no file).
  The first card was approved by a click from either the harness or the
  operator (unclear).
- **Stopped:** the operator is using the machine. A capture caught their
  Discord window (deleted unread), and SendKeys could type into their apps
  (memory: desktop-automation-while-operator-present). Still to prove,
  with the operator's consent or by the operator's own clicks: Workspace
  mode creates the file without a card; Read only declines with a note;
  the mode survives a restart; Update Codex from the button.
- The devtools build fails on this branch only because it predates #277's
  coach-signal fix; a rebase on #277 resolves it.

## Was: the exact next step (CW1a: a working turn)

1. `cargo xtask vendor-schema codex <codex.exe>`: regenerate `codex/schema/`
   (add `ServerRequest.json`, `VERSION` 0.163.0), plus a census test.
2. Delete the version range (`activation.rs:10-11,43-60,125-128`); the gate
   becomes the protocol check.
3. `dpl-tools-deps`: `install_tool` plus the resolver reload; xtask's
   `stage_tool_dependencies` uses the same copy and hash.
4. `adoption.rs` and `cargo xtask adopt-tool codex <dir>`. Adopt 0.163 into
   `D:\projects\dptools-deps`, re-stage `target/debug` and `target/release`.
5. Prove a real Codex turn on a dev run (scratch content root).

Then CW1b: server requests (`runtime.rs` `classify` misreads requests as
notifications; responses drop errors), the deadline, `account/read` health,
the permission modes (`PermissionMode` in chat-core; the accounts store's
`chat_provider_settings`), and the window's Update, Sign-in and Permissions
controls.

## Landmines

- Never read vendor credential files. Never run `codex login` or `logout`
  for the operator.
- Capture the chat window from the screen by its process window handle
  (`grab-window` pattern); EguiPilotAI cannot shoot that peer viewport.
- `platform_devpulse`'s reaper kills every `xtask.exe`: a silent exit 1
  means re-run.
- The Windows CI lane takes 40 to 70 minutes.

## Verify commands

```sh
cd /d/projects/devpulselabs && git status -sb && gh pr checks 277
/c/Users/jessa/AppData/Local/Programs/OpenAI/Codex/bin/codex --version
```
