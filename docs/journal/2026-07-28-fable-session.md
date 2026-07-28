# Journal — the Fable session, 2026-07-24 → 2026-07-28

One session, seven landed passes, two locked doctrines, one operator
course-correction absorbed mid-flight. Written as the clear-and-resume
handoff; the NEXT SERIES section at the bottom is the resume point.

## What landed (chronological, all merged to main)

- **W5 — the real .guide export** (#34): `dpl-guide` crate — manifest
  round-tripping the Steps recorder's guide.json, hand-rolled base64,
  ONE self-contained `guide.html` (embedded stills, click markers,
  do/done toggles, replay video with tappable event chapters, zero
  external refs), `guide-video.mp4`, the Guide-tab reviewer (prune/
  reorder/caption over `engine-steps::edit`), `--export-guide` seam.
  RATIFIED: html + mp4 both; replay embedded by default w/ sidecar
  toggle. W5.3 added `replay.gif` (palettegen social clip; LinkedIn-
  native = the light mp4) + the AI-driven `--steps-drill` (real
  recorder, self-capture opt-in, real Save-card path) — the full
  capture→review→bundle→export chain was driven end-to-end by the
  agent, all artifacts real. Operator verdict: "not exactly what i
  wanted but enough to move forward" — the guide UX awaits redlines
  (memory: w5-shape-not-final).
- **P1 — the portable user-data root** (#34, same branch): KNOWN #3
  killed. Operator-ratified ONE folder; resolution: `--content-root`
  arg → env → exe-adjacent → exe-parents walk-up → cwd; double-click
  proven to find the repo root; the 9 scattered captures migrated
  home. (The parallel lane later evolved this into the three-ring
  root: content/assets/.userdata.)
- **W6 — frictionless media-in** (#39): the Library became the real
  thumbnailed shelf (double-click → editor + navigate; one outside
  seam `Composer::add_from_library`) + managed ffmpeg at boot
  (superseded by the depot system — see consolidation).
- **W7 — image layers** (#46): stills are first-class timeline
  citizens — no schema change (`in_pt=0, out_pt=5.0`, trimmable),
  `StitchClip.still` loop/hold legs + `format=yuv420p` on every leg,
  every add path appends, the preview shows the held frame, the agent
  seam maps stills. Proof: seam render 8.000s exact (5s still + 3s
  clip).
- **W8 — the faithfulness pass** (#48; the "stop rushing"
  correction): THREE house laws. (1) QUIET-SPAWN — `dpl-proc::quiet`
  at every GUI-reachable subprocess + a cargo-green audit that fails
  unflagged spawns. (2) PROACTIVE-BUFFER (doctrine.md §perf) — Browse
  scans off-thread, warm cache at boot, drive-root probes never on
  the paint thread (both causes of the 20s freeze gone). (3) **THE
  SIGNAL DOCTRINE** (`docs/design/signal-doctrine.md`, LOCKED) —
  capture + jobs fabric domains, stamp-at-publish with ONE global
  seq, the UI waker, emit-with-mutation everywhere (incl. the agent
  seam — a missed site the live SSE proof caught), views invalidate
  on signal, ONE envelope `{domain,seq,ms,type}` at every boundary
  (session events.jsonl + `/signals` SSE, pinned by test), the
  ACTIVITY FEED live in the inspector. Planned via /deep-plan (two
  Opus evidence sweeps, Fable-verified). Ratified: stamp-at-publish;
  feed in W8; commands CONTRACT-ONLY until R2c.
- **Depot consolidation** (#101): operator directive — ONE supply.
  `C:/localdev/dptools-deps/artifacts/windows-x86_64/` holds all four
  tools (codex 0.145.0-alpha.18 — recovered from OpenAI's hashed
  install behind the `.local/bin` shim — ffmpeg 8.1.2, chromium
  extracted, grok); `dptools-deps/sources.local.toml` points every
  entry at the depot; `stage-tool-deps` → verified bundle; a real
  seam render ran through the verified ffmpeg (23.000s). The
  `embedded-deps` sprawl (husk dir + strays) is gone, grep-clean.
- **PR hygiene**: eight stale stacked-branch husk PRs (#20–#27)
  closed as superseded, content verified on main.

## Process record (what the next session should know)

- The operator runs PARALLEL sessions in THIS one checkout. Expect
  main to move underneath you (this session absorbed #35–#48 →
  #95–#107 mid-flight, plus worktrees appearing). Re-ground before
  every pass; never trust in-context file knowledge across a gap.
- "STOP RUSHING" correction (2026-07-26, memory:
  stop-rushing-faithfulness): findings + plan for operator eyes
  BEFORE code when quality drift is flagged; grep the ORIGINAL
  (`D:/localdev/devpulse labs`, READ-ONLY, slow disk) for its
  vocabulary before claiming a subsystem ported.
- Python-scripted multi-edits MUST assert every anchor — two silent
  no-op patches cost a gate cycle in W8.
- The 16GB laptop OOMs the full gate under load; server1
  (ssh.clyffy.ai, memory: server1-build-box) is the build box; the
  docs-only pushes here survived the local hook, but heavy pushes
  should consider server1.
- Proof-shot exe runs are SEQUENTIAL; kill orphaned drill exes
  (`tasklist | grep devpulselabs`) — one held the exe file-lock and
  blocked a release build for an hour.
- GitHub's Actions broker flaked all day 2026-07-25; the local
  pre-push gate is the enforcement (CLAUDE.md law) — merging on its
  authority when CI queues forever is established precedent.

## NEXT SERIES (the resume point, in rough priority)

1. **Guide UX redlines (operator-gated).** W5's shape is tolerated,
   not blessed. Ask the operator for redlines against the real flow
   (reviewer feel, html player look, export defaults) and land them
   as friction-audit rows — one row per pass, proof per row.
2. **Jobs-domain migration.** Collapse the three bespoke polled
   states (RenderState / IngestState / ExportState in composer) onto
   the jobs domain properly (one JobBoard consumed by status UI);
   doctrine law 7 established the contract, W8 emits additively —
   the migration pass finishes it. Include a status-bar "working…"
   truth fed by jobs signals.
3. **R2c — chat edits the open comp.** The guaranteed COMMAND channel
   (mpsc + ack, contract reserved in the doctrine) built against the
   real consumer: chat proposal → doctor verdict → Apply (one undo
   step). The seam (R1v/w) + signal envelope are ready for it.
4. **R2b — effects registry v1** (860 `.cube` LUTs via ffmpeg lut3d;
   friction row 7) — sequenced after the AE orchestrator lane per
   STATE; check that lane's phase before starting.
5. **Named small gaps** (one tidy pass, or riders): sidecar parity
   (three composer upserts skip sidecars — import.rs:99 + the two
   export/render write-throughs); async duration probe (clip.rs
   blocks the UI thread on add — quieted in W8, still synchronous);
   Browse roots refresh chip (skipped deviation); the AI-labeler
   subscription to `saved` (original's documented integration point —
   ask-first).
6. **Ops debt**: the Windows runner still isn't a service (one
   elevated `schtasks` command, CLAUDE.md); codex version drift note
   (depot 0.145.0-alpha.18 vs example 0.145.0 — harmless, but the
   example should learn the real version string on next touch).
7. **The convergence thread** (background, no pass yet): the signal
   ENVELOPE is the seam the "one platform" merge rides; the depot
   README's analytics stack (`dpt-telemetry`, signals schema) is the
   natural consumer of the stamped stream when the operator wants
   analytics — do not build unasked.

Operator acceptance still open from this arc: double-click a fresh
`guide.html` on the phone (the W5 acceptance test); eyeball that no
console windows appear in daily use (W8 leg 1).
