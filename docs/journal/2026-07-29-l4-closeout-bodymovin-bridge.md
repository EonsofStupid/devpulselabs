# 2026-07-29 — L4 close-out → Bodymovin bridge

## Done so far
- L4 (blend/matte/LUT) landed: PR #123 squash `867bff3`; STATE entry PR #132
  `67e663f`; merged-main release exes placed in THE checkout's
  `target\release` (rebuilt again today by a concurrent session, 11:52).
- AE exit strategy delivered: PNG-mov straight-alpha bake law for FX packs;
  Bodymovin→Lottie for templates; aerender fallback while subscribed.
- Operator is installing Bodymovin now. Coached the flow: open the .aep (or
  File → Open Project on the Toko .mogrt) in AE FIRST — Bodymovin is only an
  exporter panel over the open project; enable Scripting & Expressions →
  "Allow Scripts to Write Files"; Glyphs ON per comp; one .json (+ images/
  folder if raster) per export.
- ffmpeg "on PATH" question answered: injection is fully depot-based and
  proven live (`verified embedded ffmpeg resolved …dptools-deps\media\
  ffmpeg\8.1.2`). The composer stage label was stale pre-depot text — fix on
  branch `fix/ffmpeg-label-honesty` (dpl_video::ffmpeg_available + honest
  label), gating on server1 now.
- References check: operator dropped `docs/design/references/editor.png` +
  `editor-2.png` (concept mocks) and two old-repo docs
  (`wiredfront-capture-share-spec.md`, `adr-010-workbench-assembly.md` —
  NOTE: the latter's filename says workbench but its content is the OLD
  repo's "ADR-010 — the wiredFRONT Editor"). Current-build shots taken at
  1280x800 (wiredfront/editor) and 1480x840 (workbench/library) for the
  alignment verdict.

## Exact next step
1. When the server1 gate passes: `gh pr create` from the laptop for
   `fix/ffmpeg-label-honesty`, merge on green, sync worktree
   `C:\localdev\devpulselabs-fx`.
2. Deliver the references-alignment verdict to the operator (editor IA
   aligns; mocks are directional concept art, not pixel law; the misnamed
   ADR file needs a rename/ruling).
3. WAIT on operator: three Bodymovin .json exports (mogrt title, Toko
   typography slide, cyberpunk HUD element) → verify each through the
   native pipeline (`--lottie-import`, playback, L4 export).

## Landmines
- THE checkout is held by a concurrent session (branch was
  feat/milestone-loop-ledger; main moved to bd6f4d8 / PR #156 today) —
  never switch its branch; lane worktree is `C:\localdev\devpulselabs-fx`.
- Proof-shot exe runs stay SEQUENTIAL; PS 5.1 needs
  Start-Process -Wait for the GUI exe.
- gh is NOT on server1 — PRs from the laptop; `source ~/.cargo/env` inline
  before any cargo on server1.
- docs/journal + ledger emits: `sh .claude/ledger/emit.sh` from THE
  checkout root.

## Verify commands
- Depot health: run the exe with `--screenshot` and look for
  "verified embedded ffmpeg resolved" in the boot log.
- Label fix: `rg "ffmpeg on PATH" crates/composer` → only test/doc hits.
- Lottie check (when jsons arrive):
  `target\release\devpulselabs.exe --lottie-import <file>` (sequential).
