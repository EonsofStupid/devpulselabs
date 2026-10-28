# PC1a project chat storage — 2026-10-28

Plan: [chat-project-storage](../../plans/chat-project-storage.md). These are native
Windows captures of the final feature-enabled debug executable, SHA-256
`D917DD8F844A927FB1AAD28A2BED4B07E030977C5890902A4D80AD8D6AED86AA`.
All filesystem interactions used private scratch roots under
`target/pc1a-native-20261028`, with synthetic conversations and no live provider.

## Logic and artifact gate

- Full `cargo green`: **1,265 passed, 0 failed, 6 existing ignored** across
  165 test summaries. Formatting, lockfile, documentation, architecture,
  contracts, clippy, release build and real renderer smoke all passed.
- Feature-enabled window suite: **53 passed**; host project suite: **11 passed**.
- Focused project owner **19**, runtime **19**, subprocess **5**, Codex **49**,
  Grok **30**, and machine presence **12** tests passed before the final gate.
- Release smoke: shape 16,616 opaque pixels; text 10,634; export 60 frames.
- Local logs: `target/pc1a-green.log`, `target/pc1a-window-test.log`,
  `target/pc1a-host-test.log`, `target/pc1a-dev-build.log` (scratch, not committed).

## Native evidence

| Capture | Observed result |
|---|---|
| [projects-1440x900.png](projects-1440x900.png) | Project B setup/activation completes while the main window is hidden. |
| [restored-a-1440x900.png](restored-a-1440x900.png) | B → A restores A's transcript and saved draft; blocked provider does not hide the archive. |
| [projects-1100x700.png](projects-1100x700.png) | Compact restart restores active A; project controls and setup fit. |
| [restored-a-1100x700.png](restored-a-1100x700.png) | B's separate draft survives restart, then switching back restores A's draft and read-only transcript. |

A's draft was `PC1A native proof: project A draft retained.` B's was
`PC1A native proof: B has its own draft.` Both were entered through native UI,
saved by the project worker, and recovered separately after switching and restart.
The Send control remains disabled for the restored read-only native session.
Chat and main windows closed gracefully; no task-owned app process remains.

The harness used `--chat-open --chat-authored --chat-proof projects` (1440×900)
or `projects-compact` (1100×700), plus `--no-seam --mood 0`, with explicit
`CONTENT_ROOT`, `ASSETS_ROOT`, `USERDATA_ROOT` and `DPL_CHAT_STATE_HOME` scratch
paths. Project proof modes enable real project persistence only with the explicit
machine-state override; authored providers remain synthetic.

These four captures establish the new project-control baselines. Comparison with
the prior compact Chat baseline preserves the native transcript/composer layout;
the visible project bar and read-only banner are intentional additions. Each final
PNG was inspected. No missing glyphs, blank paint regions or overflowing controls
were observed; transcript scrolling clips offscreen history normally. Project
controls render in the independent peer window. No new gallery component or slide
template was registered; the existing structure/contracts gate remains green.

## Review corrections and limits

Review tightened immutable checkpoints/deltas, stale-writer rejection, exclusive
Save a copy recovery, input locking, offline close/send guards and bounded probes.
Native verification found and fixed project workers stalling when the main window
was hidden and saved conversations being hidden by unavailable-provider activation.
Regression tests cover both; final captures and the full gate include the fixes.

This proves PC1a project archives and drafts. It does not qualify live provider
sends, provider-native resume, standalone launch/profile links, Linux native UI,
or cross-machine portable session recovery. Those remain PC1b/CI qualification.
Windows file sync is implemented; power-loss directory durability is not claimed.
Bounded probes reap the exact spawned child, not escaped process descendants.
Prior archive revisions are retained. No operator library, provider profile or
credential was modified.
