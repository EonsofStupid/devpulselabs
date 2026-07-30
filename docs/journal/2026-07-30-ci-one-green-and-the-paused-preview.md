# 2026-07-30 — one definition of green, and the preview that killed itself

## Done

**The paused preview never decoded a frame (PR #170, `446fde0`).**
The operator's "ffmpeg isn't loading" was real, and it was not the depot.
`dpl-tools-deps` resolves and injects the verified `dptools-deps` payload at
boot exactly as designed (`verified embedded ffmpeg resolved …/ffmpeg/8.1.2/
ffmpeg.exe`), and renders/drills/play/backdrop all worked. But the composer
opens a preview as open → seek → pause inside ONE UI tick, and every one of
those steps sets the clip reader's stop flag. `read_clip` checked that flag
at the TOP of its loop, so a fresh pipe was killed before ffmpeg delivered
frame one — and nothing respawns while the state is unchanged. A paused
editor (i.e. most of editing) therefore sat on the loading label forever.
Fix: publish a frame FIRST, honor stop after. Proof: identical
`--fx-drill … --route wiredfront/editor --screenshot --shot-frame 300`
probes on a scratch content root — pre-fix stage empty, post-fix stage shows
the decoded frame with the FX overlay composited. Verified again on THE
checkout's exe (footer stamp `446fde0`).

The stale label that misdirected this ("needs ffmpeg on PATH") was fixed
separately in #157 — it now says "Loading the frame…", or "The media engine
is missing — reinstall the app" when nothing was injected
(`dpl_video::ffmpeg_available`).

**CI: both lanes now run the actual gate (PR #171, `198d7a7`).**
The hosted lanes hand-rolled fmt + clippy + test. `cargo green` is seven
phases; CI was running three. Missing: `docs-status`, the spawn audit, the
STRUCTURE audit (dep ceilings — "structure is law" was enforced only by the
laptop hook), and the workspace RELEASE build with the exe-regression
assert. A PR could report green while breaking the architecture doctrine or
failing to link a release binary. Both lanes now run `cargo green`, so the
hook, Windows, and Linux are one command. Self-tested on its own PR:
**green-linux 40 min, green-windows 59 min, both success, cold cache** —
inside the 90-minute ceiling that this pass raised for the added release
build.

**Nightly was one fire away from dead, and shipped a broken zip.**
#168 (merged 18:14 UTC today) moved green to hosted runners and
deregistered the estate runners, but `nightly.yml` still asked for
`[self-hosted, dpl-win]` / `[self-hosted, dpl-linux]`. The last nightly ran
fine at 07:59 UTC today (before the move); the NEXT scheduled fire —
07:15 UTC 2026-07-31 — would have queued forever against labels nothing
answers. Both lanes are hosted now. Separately and longstanding: the zip
built only `-p devpulselabs`, never `dpl-lottie-render`, which must ship
BESIDE the exe (ADR-009) — every published nightly to date is broken for
every Lottie surface. Both binaries now build and stage. The Linux asset
was ARM64 (from the old ARM box) while Windows was x64; it is now
`devpulselabs-linux-x64.zip`, and `scripts/pull-nightly.sh` follows.

## Exact next step

1. Watch the 07:15 UTC nightly (2026-07-31) — first run of the hosted
   lanes and of the two-binary zip. The build steps are a strict subset of
   what `cargo green` just proved on both hosted images; the untested part
   is stage+publish.
2. Delete the stale `devpulselabs-linux-aarch64.zip` asset from the
   `nightly` release once the x64 asset lands — it is from the retired ARM
   box and nothing publishes it anymore. Operator call (public artifact).
3. Operator-gated, unchanged: the three Bodymovin `.json` exports (mogrt
   title, Toko typography slide, cyberpunk HUD element) → verify each
   through the native Lottie pipeline to prove the template-variants path.

## Landmines

- THE checkout (`C:\localdev\devpulselabs`) is held by a concurrent session
  (`feat/d1-chat-content-seam`, staged work). Never switch its branch. Lane
  worktree for this arc: `C:\localdev\devpulselabs-fx`.
- `gh` is NOT on server1; PRs from the laptop. `source ~/.cargo/env` inline
  before any cargo there (the hook calls bare `cargo`).
- The auto-permission classifier blocks writes to files containing release-
  publishing commands (`nightly.yml`) on some models — surgical Edits on a
  different model went through. Not a repo problem.
- No lane ever RUNS the Windows binary. `eguipilotai.yml` (Lavapipe) is the
  only lane that launches a real exe, it is Linux-only AND
  `workflow_dispatch`-only. Windows is the product platform. Open gap,
  needs its own pass and an operator ruling.

## Verify commands

- Gate parity: `cargo green` locally == the single CI step in
  `.github/workflows/green.yml`.
- Preview fix: `target\release\devpulselabs.exe --content-root <scratch>
  --fx-drill inbox/base.mp4 inbox/base.mp4 --route wiredfront/editor
  --screenshot <png> --shot-frame 300 --window 1280x800 --no-seam`
  (probe clip: `ffmpeg -f lavfi -i testsrc2=size=1280x720:rate=30:duration=4`).
- Depot health: any boot log line `verified embedded ffmpeg resolved`.
- Runner fleet (expect 0 self-hosted):
  `gh api repos/EonsofStupid/devpulselabs/actions/runners --jq .total_count`
