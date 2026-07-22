# Shell layout v2 — fluid zone grid, maximized boot, status grid, icon rail

Task #103. Repo `C:\localdev\devpulselabs`, branch `master`, clean at `f120da9`.

## 1. Context

The operator stopped the build with (2026-07-22, verbatim):

> "why is the statusbar not always in the viewport it has to be maxed out to
> properly be seen above the taskbar there has to be a way for you to actually
> build fluid clamp properly where shell is always on the top of viewport
> statusbar bottom leftside rightside overlay the content scrolls inside you
> have to get these zones clearly defined so we can easily edit this should be
> well established grids so i can evolve and have cssgrid for all of my status
> indicators, stop rushing"

> "you need to have leftsidebar be properly in icon mode mouse over produces a
> random generated mood tooltip on reload so we have variety of color. we
> should be maintaining the equivalent of modern UI best practices for egui
> and rust."

> "GET EVERY DEVTOOL, AND WORKSPACE FUCKING SETUP FIRST WHAT THE FUCK YOU
> HARDCODED"

Root causes, confirmed this session:

- **Statusbar off-viewport**: `crates/app/src/main.rs:65` forces
  `with_inner_size([1440.0, 900.0])`. At dpr 1.25 that is 1800×1125 physical —
  taller than a 1080p display, so the footer sits under the taskbar. Fix: boot
  **maximized**.
- **Hardcoded chrome**: `crates/shell/src/lib.rs:20-25` declares
  `RAIL_WIDTH=232 / INSPECTOR_WIDTH=252 / TOP_HEIGHT=44 / FOOT_HEIGHT=40`
  while the generated fluid `shell` token group (topbar-height,
  bottombar-height, rail-collapsed, rail-expanded, panel-width, dock-lip,
  tap-min — `tokens/fluid.json` "Ported byte-identical from the foundry
  shell.fluid.ts... The shell components read these directly") has existed
  since the foundation pass and was never consumed. This violates the
  structure law in docs/workspace.md ("chrome dimensions come from the fluid
  `shell` group").
- **No zone contract**: nothing names the zones or exposes their rects, so
  nothing can test "footer is inside the viewport" — which is exactly the
  regression that shipped.

## 2. Current state (all read in full this session)

- `crates/shell/src/lib.rs` (781 lines) — the five consts above; panels built
  with `egui::Panel::top("dpl_top").exact_size(TOP_HEIGHT)` etc.; footer is a
  single `horizontal_centered` flex row plus a painter-centered viewport
  readout ("painted so the flex sides can't shove it" — the tell that the
  footer needed a real grid); rail rows are icon+label 34px items with the
  active hub's subroutes inlined beneath (`rail_item`/`sub_item`); inspector
  right panel 252px; 9 unit tests on routing/fabric.
- `crates/coldlight/src/lib.rs:439-460` — `Fluid { w_rem, h_rem }`,
  `Fluid::of(rect)`, `eval(min,max,slope,slope_y,intercept)`; generated
  methods land via `include!(OUT_DIR/fluid_gen.rs)`.
- Generated `fluid_gen.rs` (verified in target/): `topbar_height()` 48→60px,
  `bottombar_height()` 65→76px, `rail_collapsed()` 56→68px, `rail_expanded()`
  136→165px, `panel_width()` 320→448px, `dock_lip()`, `tap_min()` — all
  `Fluid` methods, XY-clamped.
- `crates/app/src/main.rs` — fixed inner size 1440×900, min 960×600;
  harness flags; eframe `App::ui(&mut egui::Ui)`.
- `crates/backdrop/src/lib.rs` — `Backdrop` owns a time-seeded xorshift64*;
  `MOODS`/`MOOD_COUNT`, `Mood::{accent_color, scrim_color, name}`,
  `cl::mood_gradient`. RNG is private to `Backdrop`.
- `crates/hubs/library/src/lib.rs:171,254,303` — hubs already own their
  vertical `ScrollArea`s; the shell must NOT wrap the well in another one.
- Workspace pins `egui = "0.35"`; docs/workspace.md lists `egui_kittest` as
  "adopt with the shell-layout pass".
- `docs/shell.md` does not exist yet.

## 3. Target design

**Five named zones, every dimension a generated token, every rect exposed.**

```
┌─ TOP ────────────────────────────── topbar_height() ─┐
├─ RAIL ─┬─ CONTENT (the well) ────────┬─ PANEL ───────┤
│ rail_  │  hubs render here; a hub    │ panel_width() │
│ collap-│  that overflows scrolls     │ (inspector)   │
│ sed()  │  ITSELF — chrome never moves│               │
├─ STATUS (the indicator grid) ── bottombar_height() ──┤
└──────────────────────────────────────────────────────┘
```

- **Zones**: a `Zones { top, status, rail, panel, content }` struct of rects,
  captured from the real panel responses every frame, exposed via
  `Shell::zones()` — the layout is now observable and therefore testable.
- **Status grid**: a `status` module — `StatusCell { id, lane, render }`
  registered in a static `CELLS` slice, three lanes (Left/Center/Right) laid
  out as columns of the bar. Adding an indicator = adding one cell to the
  slice (registry over enumeration, the operator's "cssgrid for all of my
  status indicators"). Cells can answer with a `StatusAction` (Snip/Agents)
  that the shell executes — the paint/mutate split the pixvix palette proved.
- **Icon rail**: `rail_collapsed()` wide, icon-only tiles (`tap_min()` tall),
  active hub carries the tone accent bar + wash. Hover shows a **mood
  tooltip**: label tinted by ONE mood rolled at boot ("on reload"), over a
  mood-gradient strip — colocated in the backdrop domain via a new
  `dpl_backdrop::random_mood_index()`. Subroutes move to a **flyout**: hover
  a hub icon → an anchored card to the right lists its subroutes (the
  existing `sub_item` rows); stays open while icon or card is hovered.
- **Maximized boot** kills the off-screen footer for good; min size stays.
- **egui_kittest adopted** (per docs/workspace.md) with layout tests that pin
  the contract: footer bottom == viewport bottom, chrome dims == token
  values, zones tile the viewport without overlap.

Scale story: a new status indicator = 1 `StatusCell`; a new hub appears in
the icon rail + flyout automatically from its `HubSpec`; a viewport of ANY
size gets correct chrome because every dimension interpolates through the
generated clamps. Control-level geometry (chip/button sizes) migrating to the
`ctl-*` tokens is a follow-up, not this pass.

## 4. Changes, in execution order

1. **edit `crates/backdrop/src/lib.rs`** — add a free function (colocated with
   the mood-random domain) + test:
   ```rust
   /// One random mood index, time-seeded — "on reload so we have variety of
   /// color": the shell rolls its tooltip mood here at boot.
   pub fn random_mood_index() -> usize { ... xorshift64* one-shot ... }
   ```
2. **create `crates/shell/src/status.rs`** — `Lane`, `StatusAction`,
   `StatusCtx`, `StatusCell`, static `CELLS` (brand, snip, viewport, mood,
   agents, signals, build), `grid(ui, ctx) -> Vec<StatusAction>` laying three
   lane columns. Tests: unique ids; every lane populated.
3. **rewrite the chrome parts of `crates/shell/src/lib.rs`**:
   - delete the four consts; compute `let fl = cl::Fluid::of(ui.ctx().content_rect())`
     once per frame and pass into every panel builder;
   - add `pub struct Zones` (+`Shell::zones()`), fill from panel responses;
   - `top_bar` → `fl.topbar_height()`; `status_bar` → `fl.bottombar_height()`
     and delegates to `status::grid`; footer flex code and `FooterClicks` die;
   - `rail` → `fl.rail_collapsed()` icon-only tiles + boot `tooltip_mood`
     (from `dpl_backdrop::random_mood_index()`) + flyout state
     `flyout: Option<HubId>` rendered as an `egui::Area` beside the rail;
   - `inspector` → `fl.panel_width()`;
   - keep: fabric publishing, full-bleed, backdrop paint/tiles, breadcrumb.
4. **edit `crates/shell/Cargo.toml`** — `[dev-dependencies] egui_kittest = "0.35"`;
   new kittest tests in lib.rs: zones pinned to viewport at 1280×800 AND
   1920×1080 (status.max.y == viewport height — THE regression, chrome dims
   == fl token values, zones tile without overlap).
5. **edit `crates/app/src/main.rs`** — viewport builder gains
   `.with_maximized(true)` (inner_size stays as the restored-size fallback).
6. **create `docs/shell.md`** — the zone contract law doc (zones, token
   mapping, scrolling law, status registry, rail/flyout, full-bleed, zones()
   testability).
7. **edit `docs/workspace.md`** — move egui_kittest from "adopt deliberately"
   to adopted.
8. Verify (below), update proofs, commit.

## 5. Boundaries

- 🚫 tokens/fluid.json values — they are the foundry port, not tunables.
- 🚫 backdrop paint chain, fabric events, hub trait/spec, coldlight
  generators — out of scope.
- 🚫 no shell-level ScrollArea around the well (hubs own scrolling; Library
  already does).
- ⚠️ per-hub workspace/registry scaffolding ("wiredfront as well") — its own
  census-driven pass, not crammed in here.
- Out of scope: ctl-token migration of chip/button geometry; rail_expanded()
  pinned-open mode (icon mode is the ratified default; expansion is a later
  affordance); dock_lip consumer.

## 6. Verification

1. `cargo green` exits 0 (fmt, clippy -D warnings, ALL tests incl. the new
   kittest layout tests, release build + mtime assert).
2. Kittest pins (run inside the gate): at both test sizes,
   `zones.status.max.y == viewport.height`, `zones.top.height() ==
   fl.topbar_height()`, `zones.rail.width() == fl.rail_collapsed()`,
   `zones.panel.width() == fl.panel_width()`, content intersects no chrome.
3. Real exe: `target\release\devpulselabs.exe --mood 2 --screenshot
   docs\proof\shell-v2-home.png --route home/main` → shot shows icon rail,
   footer grid with build stamp, all inside the frame (maximized).
4. Real exe interactive proof shot with cursor scripted? No — hover states
   can't be screenshot-scripted yet; the flyout/tooltip correctness is
   covered by kittest pointer simulation if the API allows, else by the
   operator's hands-on pass (stated honestly in the report).
5. The end-to-end proof of the operator's requirement: a maximized boot where
   the status bar is fully visible above the taskbar, chrome sized by the
   generated clamps at the real display size.

## Execution notes (deviations, recorded same-turn)

1. `Zones` gained a `root: Rect` field beyond the plan: the kittest harness
   insets its ui-mode root by a test-only 8px outer margin (eframe's real
   root has none — verified in both sources), so the contract test pins
   chrome against the recorded root, plus a ≤20px root-vs-viewport sanity
   bound. The first test run caught exactly this — the test found a real
   discrepancy class on its first execution.
2. Found and fixed in-pass: `crates/app/build.rs` emits `DPL_BUILD` and the
   shell had `set_build`, but **main.rs never called it** — the provenance
   stamp (the stale-exe killer from #102) had never actually rendered. Wired
   `shell.set_build(env!("DPL_BUILD"))`; the proof shot now shows
   `f120da9 · 2026-07-22 13:53z` in the status grid's build cell.
3. A leftover harness instance locked the exe during the second green run —
   the gate's release step failed EXACTLY as designed instead of shipping a
   stale exe; killed the process, re-ran green, recaptured every proof from
   the fresh binary.

## 7. Commit plan

One commit:

```
feat(shell): layout v2 — fluid zone grid, icon rail + mood tooltip + flyout,
status cell registry, maximized boot
```

`git add`: crates/shell/**, crates/backdrop/src/lib.rs, crates/app/src/main.rs,
docs/shell.md, docs/workspace.md, docs/plans/shell-layout-v2.md,
docs/proof/shell-v2-*.png, Cargo.lock. No push.
