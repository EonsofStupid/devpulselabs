# R301 native chat — platform context

The normal host supplies chat's workspace and storage. There is no separate
chat-project setup. Development builds use the app's existing build-project
resolution; packaged builds use the configured platform library. Chats save
under that library's userdata/chat/conversations. Existing workspace archives
are copied into this archive without changing the originals.

## Run the development version

From this checkout:

```powershell
cargo xtask dev -- --chat-open
```

This builds the real native peer with gated devtools and stages the named tool
depot. New chat offers provider availability, the existing role library, and
an editable starter. Create chat prepares a draft; Send starts the provider
turn. The platform project, workspace and current view accompany that turn.

## Verification record

- `cargo green`: passed before live refinements (1,401 passing tests, six
  ignored, release executable rebuilt, real renderer smoke passed).
- Native `chat-new` scenario after the permission-ID fix: nine steps passed,
  run `target/eguipilotai/runs/run-97764-18e9f9ddfbe568a4`.
- Chat-window suite after the long-draft fix: 61 passed, zero failed. Includes
  an actual Send click at 1040×640 using the complete Skeptic role starter.
- Platform host tests cover automatic boot, draft restoration, retry after
  storage returns, and refusing old selection intents against app context.
  Archive tests cover distinct supplied contexts and preserving legacy data.
- Live startup observed Codex's qualified connection and account automatically,
  then New chat created a real provider session and placed the Skeptic starter
  in its draft. Final live send and final gate evidence are recorded below.

Live testing caught a duplicate permission-control ID and a long multiline
draft pushing Send below the window. Both are corrected and covered. It also
found an unrelated Grok failure leaking into the Codex popup; failures are now
scoped to the selected/pending provider. The popup footer stays visible while
its choices scroll.

## Screenshots and scope

`new-chat*.png` are preserved rejected drafts containing the old project flow.
`platform-chat-*.png` are the first corrected context-only popup screenshots,
inspected at 1440×900 and 1100×700; they precede the live layout refinements.
Use the final screenshots listed below for current acceptance.

Authored screenshot data is a layout fixture, not evidence of a live AI reply.
Claude is detected on this machine but has no qualified chat adapter. Grok
rejected cached subscription authentication. Meta Muse is not integrated.
Saved provider sessions remain read-only until native resume is qualified.
No claim of all-provider support or completed native resume is made.

## Final acceptance — 2026-11-21

- `target/chat-awareness-green-composer.log`: **green**, 1,403 tests passed,
  six ignored across 169 suite reports. Final release renderer smoke: 16,616
  opaque shape pixels, 10,634 text pixels and 60 exported frames.
- `target/chat-awareness-window-final.log`: 61 chat-window tests passed,
  including long-role Send and unrelated-provider-failure regressions.
- Final EguiPilot scenario: **nine steps passed**, run
  `target/eguipilotai/runs/run-89456-18ea1a8e3701f74c`. Both permission-control
  IDs are unique; neither old project-picker control exists in the normal UI.
- Debug executable rebuilt at 01:12:07 local time (101,855,232 bytes); release
  rebuilt at 01:16:57 (74,965,504 bytes). Both are from this corrected source.
- [1440×900 popup](final-popup-1440x900.png) and
  [1100×700 popup](final-popup-1100x700.png): opened and visually inspected.
  These are authored layout fixtures with no live-provider claim.
- [Actual Codex reply](live-codex-platform-reply.jpg): native UI at 1040×640.
  New chat → Skeptic → typed task → Create chat → Send produced a real answer
  identifying `devpulselabs` and `wiredfront/viziwiz`. The task explicitly asked
  for no file reads or tools. No project picker or permission change was used.
  The long role starter was sent intact and the conversation saved. The Chat
  peer then closed normally, leaving its native parent open.

The test used isolated platform storage in `target/chat-platform-live/library`.
Logs are in `target/chat-platform-live/`; earlier failed-launch logs were copied
to separate files before subsequent runs. Code and proof are local, uncommitted;
no PR merge or all-provider readiness is claimed.
