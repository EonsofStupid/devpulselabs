# ADR-011 — the rendering & widget stack (ratified picks + adoption passes)

Status: ratified 2026-07-22 (operator: "Get them planned out for
implementation"). Research grounded in live July-2026 ecosystem state.

## The bet, in one line

**Linebender is egui's own direction** — egui 0.35 replaced its text backend
with `skrifa` + `vello_cpu` (verified in our Cargo.lock: epaint already
ships vello's CPU renderer). We align every vector/graphics choice with
that stack, and every widget choice with the crates that demonstrably track
egui releases (Rerun's and lucasmerlin's).

## Ratified picks

| Need | Pick | Why | Where |
|---|---|---|---|
| SVG, correctness tier | **resvg** (linebender-stewarded) via egui_extras `svg` feature | THE Rust SVG renderer; pure Rust; June-2026 active | RS1 (now) |
| SVG/vector, GPU interactive tier | **vello + vello_svg** | Linebender GPU renderer; the editor compositor path; version-unified with epaint's vello by deny.toml | RS4 / R1 |
| Docking / panel layout | **egui_tiles** (rerun-io) | Rerun-maintained → never lags an egui release; drag-drop tiling + grids | RS4 (wiredFRONT editor shell) |
| Drag & drop | **egui_dnd** (hello_egui) | Active; tracks egui; touch-friendly | RS2 (W4 slide strip) |
| Flex layout | **egui_flex** (hello_egui) | Active; built on egui's new intrinsic sizing | RS2 (editor trays) |
| Huge lists/grids | **egui_virtual_list / egui_infinite_scroll** (hello_egui); evaluate rerun's egui_table for tabular views | Gallery must scroll thousands of artifacts on the touchscreen | RS3 (X2) |
| Node graphs | **egui-snarl** | Active (0.9, 2026); customizable wires; serde built in | RS5 (deferred until a node UI exists) |
| Dev charts | **egui_plot**, version-pinned | Official but "looking for maintainer" — INTERNAL surfaces only | RS6 (Test Lab/analytics) |
| Show-slide graphics | **ours** (epaint + dpl-vector + ColdLight + motion) | No library gives mood-toned, entrance-animated, token-colored show charts | standing law |

## Adoption laws (every RS pass, no exceptions)

1. **Single-version rule**: the crate must resolve against OUR egui minor;
   deny.toml `multiple-versions = "deny"` is the machine check. A crate that
   pins a different epaint is rejected, not vendored around.
2. **ColdLight adapter**: third-party widgets are wrapped once in a
   `cl`-styled adapter (colors from the pool, geometry from fluid tokens) —
   call sites never see the foreign theming API. One adapter module per
   crate, colocated with the consuming feature domain.
3. **PilotClock**: any animation the crate performs registers a motion
   adapter the day it lands (pilot law 3), or its surface is excluded from
   captures explicitly.
4. **Gallery gate**: visually novel adoptions render on a wall/demo surface
   (the existing law) so regressions are visible.
5. **Show surfaces stay ours**: nothing third-party paints inside a slide
   document render. Adopted widgets live in chrome, editors, and internal
   tools only.
6. **Exit honesty**: each adoption notes its blast radius (what we rewrite
   if the crate dies). Wrapped-adapter discipline keeps that radius to one
   module.

## Adoption passes

### RS1 — resvg SVG support (NOW; small, standalone)
- Cargo: egui_extras feature `"svg"` (pulls resvg through the maintained
  loader path).
- Doc schema: `Media::Image`/future doc media blocks accept `.svg` — the
  file loader handles it once the feature is on.
- Gallery gate: one SVG asset rendered on the Library templates/media demo
  (an authored ColdLight-styled SVG in assets/) + a coldlight/slides test
  asserting the loader resolves it.
- Verify: green + wall proof. Blast radius: feature flag.

### RS2 — W4 riders: egui_dnd + egui_flex (with ledger #105)
- Adopt inside the editor pass, not before: dnd for the slide-strip
  reorder + tray drag-to-apply; flex for tray layouts.
- Adapters: `crates/editor/src/ui/dnd.rs`, `.../flex.rs` (names final at
  W4 plan time). Touch: tap-min targets preserved; dnd animations →
  PilotClock adapter; pilot gesture extension exercises a drag in the W4
  scenario.

### RS3 — X2 riders: egui_virtual_list (+ egui_table evaluation)
- Gallery grid virtualizes from day one (the old app virtualization was a
  planned perf pass — we adopt at build time instead of retrofitting).
- Evaluation gate for egui_table: only if the Gallery's list view is
  genuinely tabular; otherwise virtual list alone.
- Pilot: scroll-through scenario over a seeded 500-artifact fixture root.

### RS4 — R1: the editor rendering ADR (vello + egui_tiles)
- The wiredFRONT editor ADR (already roadmapped as R1) now has its stack
  named: egui_tiles for panel/docking chrome; egui_wgpu paint callbacks +
  vello for the layered compositor (video frames as textures, vector
  overlays via vello_svg, offscreen render targets for export).
- ADR is written and RATIFIED WITH THE OPERATOR before code (two-editors
  law: none of this touches Workbench).
- Blast radius note: vello pre-1.0 — the compositor seam isolates it
  behind our own `Layer`/`Compositor` traits so a vello major bump (or
  swap) is one crate.

### RS5 — egui-snarl (DEFERRED)
- Trigger: the first real node UI (recipe editor / pipeline view). No
  pre-adoption. When triggered: adapter + serde persistence into the
  document layer's refs-never-values law.

### RS6 — egui_plot, internal-only (with Test Lab/analytics work)
- Pinned minor; adapter feeds colors from the pool; INTERNAL-ONLY law
  recorded next to the dependency (a slide using it should fail review).
- Re-evaluate the maintainer situation at each egui minor bump; our
  authored infographics are the escape hatch (blast radius: swap for
  hand-paint, already proven in 5 templates).

## Explicit non-picks (recorded so we stop re-deciding)

- **egui_dock** — alive, but egui_tiles wins on maintainer alignment
  (Rerun) and grid layouts. Revisit only if tiles loses a needed feature.
- **plotters / egui-plotter** — retained-style bridge, wrong fit for
  immediate-mode show graphics; nothing it gives that ours + egui_plot
  don't.
- **egui_graphs (petgraph viz)** — analysis-graph aesthetics; snarl covers
  the editor-node use case we'd actually build.
- **Raw resvg dependency** — we consume it through egui_extras' loader;
  a direct dep only appears if we need custom rasterization (media
  pipeline SVG→raster export would be that trigger).

## Sequencing

RS1 immediately (one small pass). RS2/RS3 ride W4/X2 — the roadmap
sequencing already covers them. RS4 = R1 ADR next time the operator wants
the editor conversation. RS5/RS6 on their triggers.
