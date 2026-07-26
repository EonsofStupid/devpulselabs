# RR1 — the resolution sweep: a no-panic matrix + cramp map

Program: **Responsive Rendering (RR)** — make the app work at all resolutions.
- **RR0** (landed, #54): the `fit()` law — no layout panic when a floor
  exceeds a proportional/shrinking ceiling. Fixed two live panics.
- **RR1** (this pass): a repeatable sweep across a size × route matrix that
  (a) ASSERTS no panic and (b) captures proof-shots so cramping is visible —
  the objective map that drives RR2's hand-tuning. Ground-truthed 2026-07-26.

## 1. Context

RR0 fixed the crash I hit live at `1280x800` on `wiredfront/editor`. But that
panic survived for a reason: proofs ran maximized/large only — CLAUDE.md's own
words, *"the footer bug hid behind maximized-only screenshots."* Without a
sweep, "works at all resolutions" is a claim, not a fact, and RR2's uncramping
has no checklist and no regression guard. RR1 builds the instrument.

Gate row (wiredfront-experience.md): supports the whole experience bar — every
surface must render honestly at REAL window sizes, not only maximized.

## 2. Current state (ground-truthed 2026-07-26)

- The app already self-captures: `--screenshot <png> --window <w>x<h>
  --route <hub/sub> --shot-frame <n>` boots the real app, paints, captures its
  own window, exits — `crates/app/src/main.rs:4-5,62-85,300-302`. Proven on
  this box (Wayland + lavapipe software Vulkan; the ARM GPU driver is
  incompatible so it falls back to software — fine for proof).
- No sweep exists — screenshots are run one size at a time by hand, which is
  exactly how the `1280x800` panic hid.
- `xtask` is the workspace automation home (`cargo green`, `fix`,
  `stage-tool-deps`): dispatch is a string match in `main()` —
  `crates/xtask/src/main.rs:622-645`; `run_command` spawns and checks status
  (`:26-36`). A `sweep` subcommand fits beside `green`.
- eguiPilot verifies the SEMANTIC tree: `eguipilotai inspect <profile>
  <fixture>` launches the instrumented app (feature `eguipilotai-devtools`,
  `egui_pilot_ai::install`) and dumps the widget-tree snapshot —
  `crates/egui-pilot-ai-cli/src/main.rs:146-153`. Linux profile:
  `devpulse-cloud` (debug) / `devpulse-hardware` (release);
  fixtures in `.eguipilotai.toml` (viziwiz/library/chat/…).

## 3. Target design

**`cargo xtask sweep [outdir]`** — an isolated-subprocess matrix runner:
- MATRIX (sizes): `1024x640, 1280x720, 1280x800, 1366x768, 1600x900,
  1920x1080`, plus a tall/narrow `860x1200` (portrait catches the fixed-width
  panels). ROUTES: `home/shows, wiredfront/editor, wiredfront/library,
  workbench/library, workbench/episodes, workbench/test-lab` (extend as hubs
  land).
- For each (route, size): spawn the RELEASE exe
  `--screenshot <outdir>/<route>__<size>.png --window <size> --route <route>`
  as its OWN process (a panic fails only that cell, never the sweep), under
  software rendering (`LIBGL_ALWAYS_SOFTWARE=1`) so it needs no real GPU.
- Collect per-cell verdict: `ok` (exit 0 + png written) / `panic` (stderr
  carries "panicked" or non-zero exit) / `timeout`. Write
  `<outdir>/sweep.json` (route, size, verdict, png, ms) and print a compact
  table.
- EXIT NON-ZERO if any cell panics — so it is a real gate, not a report.
- The PNGs are the CRAMP MAP: I review them and RR2's plan lists each cramped
  route×size as a checklist row (no silent truncation — the report names every
  cell).

**Isolation over an in-app loop:** one egui viewport per process is simplest
and a panic in one cell must not abort the rest — subprocess-per-cell is the
honest design.

## 4. Changes, in execution order

1. `crates/xtask/src/main.rs`: add `"sweep" => sweep(&arguments)` to the
   dispatch (`:626`); implement `fn sweep(args)` — matrix consts, per-cell
   subprocess spawn with a wall-clock timeout, verdict collection, `sweep.json`
   + table, non-zero on any panic. Reuse `run_command`'s spawn idiom.
2. A thin `#[ignore]`-by-default integration test `tests/resolution_sweep.rs`
   that runs a CORE subset (the editor route × the small sizes that panicked)
   and asserts no-panic — a fast guard runnable in CI without the full matrix.
3. Nothing in production code changes — RR1 is pure instrumentation.

## 5. Boundaries

- RR1 does NOT change any layout — it only MEASURES. Uncramping is RR2.
- Not wired as a blocking green-linux job in THIS pass (a full GUI matrix in CI
  needs the runner's display/software-GL env settled first — noted as an RR1.1
  follow-up); the `#[ignore]` core test + local `cargo xtask sweep` are the
  interim guard. No silent scope — this boundary is explicit.
- Video/render cells will WARN (ffmpeg/dptools-deps unstaged on this box) but
  that is orthogonal to layout — the sweep flags it, doesn't fail on it.

## 6. Verification

- `cargo green` (fmt + clippy + tests + release build) — the sweep code itself.
- Run `cargo xtask sweep target/sweep` on this box → `sweep.json` shows every
  cell `ok` (RR0 already fixed the known panics); the PNGs are attached as the
  cramp map in §8.
- eguiPilot: `eguipilotai inspect devpulse-cloud viziwiz` returns a semantic
  snapshot (proves the devtools verify-path works on this box, for RR2/W9
  assertions) — confirmed live in §8.
- The `#[ignore]` core test passes when un-ignored.

## 7. Commit plan

Branch `feat/rr1-resolution-sweep`, one PR (plan doc + `xtask sweep` + the
core test). PR → `green-linux` → squash-merge → STATE.md + §8 evidence.

## 8. Evidence (2026-07-26)

1. **`cargo xtask sweep`** implemented (`crates/xtask/src/main.rs`): 6 routes ×
   7 sizes = 42 cells, each an ISOLATED subprocess (software-GL), per-cell
   120s timeout, `sweep.md` table + a PNG per cell, exits non-zero on any
   panic/timeout/fail.
2. **eguiPilot verify-path confirmed on this box**: `eguipilotai inspect
   devpulse-cloud viziwiz` returns the semantic snapshot — nodes with `id`,
   `role`, `name`, `actions`, and `rect` bounds (the rects let RR2/W9 assert
   no-overlap / on-screen programmatically). Kill orphan `devpulselabs` procs
   after CLI runs (a second inspect raced to 0 nodes otherwise).
3. **THE DISPLAY FINDING (the real lesson):** the first sweep rendered EVERY
   cell at 640×480 — this box was a headless GNOME/mutter output at 592×440,
   which won't allocate an oversized window surface, so eframe fell to its
   640×480 `min_inner_size` and `--window` was a NO-OP. The earlier "cramp at
   1024×640" and "panic at 1280×800" were both really ~640×480. A real
   HDMI-0 1920×1080 display was connected; `--window` is then honored — dims
   vary exactly (1024×640, 1280×720, 1366×768, 1600×900) and cap to the
   screen (1920×1080→1854×1011, 860×1200→860×995). LESSON: window-size proofs
   are meaningless on a display that can't grow — the proper, display-
   independent path is OFFSCREEN rendering (see §5 boundary; RR1.1).
4. **Verdicts (real sizes):** all 42 cells `ok`, zero panics — RR0's `fit()`
   holds at every real size.
5. **THE CRAMP MAP:** `wiredfront/editor` is CRAMPED below ~1280px wide —
   at 1024×640 the center column text overlaps (INFO/EFFECTS collide with
   "Add footage…"; "NEXT…" overlaps the status line) and the top tool row
   overflows under the inspector; CLEAN at ≥1280 wide. → RR2 target:
   fit-to-available center + a wrapping/scrolling tool row + a breakpoint
   that collapses panels below ~1100–1280px. (Other routes render clean at
   the sampled sizes; the full PNG set is the checklist RR2 walks.)
6. **DEVIATIONS:** report is `sweep.md` (human table) not `sweep.json` —
   better for a cramp map, no serde_json dep. The `#[ignore]` CI test (§4.2)
   is DEFERRED: a GUI test needs a display honoring sizes, which this box
   only has when a monitor is attached — so the display-independent OFFSCREEN
   render harness is the real CI guard, scoped as its own pass (RR1.1). The
   `cargo xtask sweep` binary is the interim local guard.
