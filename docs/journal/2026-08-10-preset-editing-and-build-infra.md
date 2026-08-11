# 2026-08-10 — Preset editing, the offline-AE thread, and the build infra it needs

A long session with two intertwined arcs: making the native Lottie path a
real **offline After Effects alternative** for the operator's restricted work
environment, and hardening the build/gate infrastructure that surfaced along
the way.

## Done (all merged to main)

**Preset editing (the WiredFRONT thread the operator cares about most)**
- **#221** — the Lottie preset ENGINE: `dpl_lottie::controls::{discover,apply}`
  finds a preset's editable text/colours and patches by JSON pointer. Plus the
  two fixes that made it possible: Lottie `export` had NEVER emitted a frame
  (frame-0 `INSUFFICIENT_CONDITION` treated as fatal), and Lottie TEXT rendered
  BLANK at exit 0 because no font was ever loaded (`tvg_font_load`).
- **#229** — the preset PANEL in the composer inspector: retext and recolour a
  Lottie with the SAME ColdLight tone swatches as tint. Preview and export
  resolve through ONE function (`resolved_lottie`) so they cannot disagree;
  values live on `Layer.controls` as plain JSON (dpl-comp's ceiling forbids
  naming a renderer type). Also **live motion blur in the viewer**
  (`rgba_blurred` averages sub-renders across the shutter — the same integral
  `tmix` computes) with `--mblur-drill` to prove it in the app.

**Build/gate hardening**
- **#227** — the `smoke` gate: runs the REAL renderers and asserts on
  opaque-pixel/frame counts. The three preset fixes above ALL shipped at exit
  0; this closes the ADR-009 subprocess coverage hole. Watched fail before
  trusted.
- **#237** — the `lock` gate: `cargo metadata --locked`. I committed a manifest
  without its lock THREE times (#216, #219, egui 0.36). #236 merged with
  Cargo.toml on 0.36 and Cargo.lock on 0.35 and both lanes went green.
- **#236** — egui 0.35 → 0.36, all five version-locked crates in one PR (wgpu
  29→30). Three breakages: `RawInput::modifiers` removed, `DroppedFile::path`
  is a method, epaint PANICS on a dropped `TexturesDelta` (32 sites).
- **sccache tier 1** — LIVE on laptop (`D:\sccache`, 50G) and server1
  (`/root/.cache/sccache`, 20G), global-default (operator choice A).
  `docs/runbook/sccache-shared-build-cache.md` has the HONEST numbers.

## Exact next step

**The Bodymovin export test — the operator is doing this manually.** Open a
real `.aep` (candidate: `D:\WebWork\design\unsorted\cyberpunk-futuristic-intro`)
in AE, run Window → Extensions → Bodymovin, export the main comp to JSON, and
bring the JSON here. That single measurement decides whether the native path
can carry the operator's ~5,000-element library or not. Everything downstream
(image placeholder control, library ingest, "chain them like RTFX") waits on
it.

Then, in priority order:
1. **`ControlKind::Image`** — a Lottie asset-ref becomes a swappable
   placeholder (`DevPULSE_PLACEHOLDER` → drop a transparent PNG, alpha rides
   for free). This is the "premade with a logo slot" the operator described.
2. **Library ingest** — reference-import (symlink, no copy) `D:\WebWork\design`
   into the WiredFRONT library, tagged by pack, browsable + reachable over the
   seam. The reference-in-place plumbing already exists.
3. **sccache tier 2** — MinIO on a headroom disk (D: or the new drive) so
   server1's warm cache spares the laptop its cold build. THE prize for the
   laptop-slow pain.

## Landmines

- **Build on server1, NOT the laptop.** 3m31s vs 23m28s measured; the laptop
  OOMs (`os error 1455`) and crashes rustc (`STATUS_STACK_BUFFER_OVERRUN`).
  Over SSH you MUST `source $HOME/.cargo/env` or cargo is not on PATH. See
  memory `build-on-server1-not-the-laptop`.
- **sccache is NOT a free speedup.** First clean build is SLOWER (13m53s vs
  5m47s baseline); the win is repeat clean builds (99% hit). `CARGO_INCREMENTAL=0`
  is now global — a real cost to tight iterative dev.
- **egui bumps are ONE PR** — the five crates are version-locked; dependabot's
  per-crate PRs are always red. Memory `egui-bumps-are-one-pr`.
- **A manifest change MUST commit its lock** — the new `lock` gate now catches
  it, but the habit is mine to break.
- **The pre-push hook runs the FULL gate even on docs-only pushes** — a
  markdown change triggers a 20-min laptop `cargo green`. A `paths` check in
  `.githooks/pre-push` would skip it. Not fixed.
- **AE ExtendScript is encoding-picky** — `-noui` drops env vars, and a script
  file needs a UTF-8 BOM + CRLF or AE runs nothing SILENTLY. The working probe
  is `scratchpad/probe5.jsx` via `scratchpad/mkprobe.py`.
- **The licence line holds and is not the operator's ruling.** I will not
  disable aerender/AE licence validation; that is an assistant refusal, not an
  operator decision (corrected in memory `rulings-2026-08-06-media-program`).
  The legitimate offline path is **FRL Isolated** through the DoD JELA (the
  operator is govt, demoing at Indian Head bldg 2015/2172) plus the migration
  ledger as the negotiating lever with Adobe.

## Verify commands

```
# On server1 (fast, sccache-warm) — the honest way to gate:
ssh ssh.clyffy.ai
source $HOME/.cargo/env && source $HOME/.sccache_env
cd /root/dev/devpulselabs && cargo green          # green ✔, smoke + lock included

# Prove the preset path renders (needs the depot staged, junction removed after):
cargo run -p dpl-lottie --example preset_controls -- <preset.json> crates/coldlight/assets/fonts <out>

# Prove motion blur in the APP:
target/release/devpulselabs.exe --route wiredfront/editor --mblur-drill --window 1300x800 --screenshot <png> --shot-frame 420 --no-seam

# sccache health:
sccache --show-stats            # Cache location, Max cache size, hit rate
```
