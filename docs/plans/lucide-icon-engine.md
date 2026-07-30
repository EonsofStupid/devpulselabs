# Pass — the Lucide icon engine (POAM V3, ADR-015)

## 1. Context

The operator (2026-07-29, verbatim): "lets take the time to pull lucide
and then properly own it as part of coldlight, we will have the SVG
engine that is based off the airbnb engine airvx and establish baselines
for our diverse and default polished stylings."

The original app's icons were Lucide; the port substituted Phosphor font
glyphs (`cl::icon` self-describes as "Phosphor glyphs under the OLD
lucide inventory's semantic names"). This pass restores the real icon
language, owned: vendored Lucide 1.27.0 + a zero-dependency SVG engine
(`dpl-svg`) whose philosophy is visx's — geometry unopinionated, styling
composed by ColdLight. Decision record: `docs/design/adr-015-svg-icon-engine.md`.

## 2. Current state (all read in-session 2026-07-29)

- `crates/coldlight/src/lib.rs:687` — `pub mod icon`: ~20 Phosphor
  string consts (HOME, WORKBENCH, … FREE), one per meaning. Doc-comment
  names the Lucide debt.
- `crates/coldlight/build.rs` — seven generators (`generate_ramps`,
  `_fluid`, `_motion`, `_aurora`, `_moods`, `_fonts`, `_presets`,
  `_traits`), each: read `tokens/*.json`, validate refs (unknown → the
  build fails), emit `*_gen.rs` to OUT_DIR. `generate_fonts` is the
  vendored-asset exemplar (entry whose TTF is missing fails the build).
- `crates/coldlight/Cargo.toml` — deps: egui only; build-deps:
  serde_json, palette.
- `crates/coldlight/assets/` — Phosphor.ttf + `Phosphor-LICENSE-MIT.txt`,
  `fonts/*.ttf` + `fonts/licenses/*` — the vendoring precedent.
- `crates/vector/src/lib.rs` — `dpl-vector`, "pure geometry as epaint
  meshes… callers bring a `Color32`" — untouched by this pass (its
  concern is arcs/rings; icon meshes are generated, not built here).
- `crates/hub/src/lib.rs:47` — `HubSpec { icon: &'static str }` with
  "Text glyph for the rail until the sprite atlas lands (P1)".
- Consumers of `cl::icon::*` — 40 hits, 11 files: `crates/viziwiz/src/
  palette.rs` (21, as `RichText::new(glyph)`), `crates/deck` (5),
  `crates/shell/src/lib.rs` (4) + `status.rs` (1), hubs home/workbench/
  wiredfront/shows/present (6), `crates/app/src/main.rs` (1).
- `crates/hubs/workbench/src/library.rs:54` — `LibSection::ALL`, 9
  sections (Seeds…Moods); slugs feed `--lib-section`.
- `docs/design/architecture.md` — §1 layer stack, §2 exact-set ceilings,
  §5 new-crate checklist; audit lives in `crates/xtask/src/main.rs`.

## 3. Target design

- `crates/coldlight/assets/lucide/{icons/*.svg, LICENSE, VERSION}` —
  Lucide 1.27.0 vendored (the pool on disk; only registry-referenced
  icons enter the binary — the fonts law).
- `crates/svg` → `dpl-svg` (ENGINE, ceiling ∅): SVG icon subset →
  stroked triangle meshes, pure `(f32, f32)`/`u32` buffers. Modules:
  `xml` (element reader), `path` (d-grammar → `Vec<Cmd>`, arc→cubic),
  `flatten` (adaptive cubic flattening), `stroke` (round caps/joins →
  `Tess { vertices, indices }`), `icon` (svg text → `Tess`, entry fn
  `tessellate_icon(svg: &str, tolerance: f32) -> Result<Tess, SvgError>`).
  Unsupported constructs are ERRORS, never silent skips.
- `tokens/icons.json` (8th authored source): `{ "icons": [ { "id":
  "home", "lucide": "house", "label": "Home", "blurb": "…" }, … ] }` —
  every id currently in `pub mod icon`, semantic names preserved.
- build.rs `generate_icons` → `icons_gen.rs`: `pub struct IconDef { id,
  label, lucide, blurb, vertices: &'static [(f32, f32)], indices:
  &'static [u32] }`, `pub static ICONS: &[IconDef]`, `pub fn
  icon_def(id: &str) -> Option<&'static IconDef>`, `ICON_COUNT`.
  Missing SVG / parse error / unknown field → build fails.
- coldlight runtime: `IconDef::mesh(&self, rect: Rect, color: Color32)
  -> egui::Mesh` (map 24-box → rect, tint); paint helpers
  `cl::draw_icon(painter, id, rect, color)` and widget-level
  `cl::icon_label(ui, id, size, color) -> Response`; the brutal variant
  = shadow-black mesh at `[3, 3]` offset under the tinted pass (a
  painted shape, never blur).
- `LibSection::Icons` (10th section, slug `icons`): grid of every
  registry icon — default, tone-accent, and brutal renditions + id
  labels; tooltips per the tooltip law.
- End state (M2): `pub mod icon` (Phosphor consts) DELETED; all 40 call
  sites painted; `HubSpec.icon` documents "generated icon registry id".

## 4. Changes, in execution order

**M1 — the engine + the pool + the registry (PR 1):**
1. Vendor: download `lucide-icons/lucide` tag `1.27.0` source archive →
   extract `icons/*.svg` to `crates/coldlight/assets/lucide/icons/`;
   write `LICENSE` (upstream text verbatim) and `VERSION`
   (`lucide 1.27.0 — https://github.com/lucide-icons/lucide/releases/tag/1.27.0`).
2. Create `crates/svg` (`dpl-svg`) per §3 module map + inline tests:
   d-grammar cases (relative/absolute, S/T reflection, arc sweep flags),
   flatten tolerance monotonicity, stroke cap/join vertex sanity, and a
   corpus test parsing EVERY vendored SVG (the pool stays honest).
3. Workspace `Cargo.toml`: member in ENGINE order; architecture.md §1 +
   §2 (`dpl-svg | ∅ | the icon-subset SVG law is dependency-free`) +
   xtask audit table — same commit.
4. `tokens/icons.json`: the ~20 semantic ids from `pub mod icon`, each
   mapped to its Lucide name (house, hammer, squares-four, film-slate,
   robot, monitor-play, camera, caret-up, arrows-out, arrows-in, x,
   caret-left, video-camera, footprints, record, square, scan,
   app-window, monitor, lasso — ground-truth each against the pool).
5. build.rs: rerun-if-changed for icons.json + assets/lucide;
   `generate_icons` (build-dep `dpl-svg`, path = "../svg").
6. coldlight lib.rs: include `icons_gen.rs`; `IconDef::mesh`,
   `draw_icon`, `icon_label` (+ brutal rendition helper).
7. `LibSection::Icons` + its grid surface in workbench library.
8. Proof: `cargo green`; exe rebuild; screenshots `--lib-section icons`
   at maximized AND `--window 1100x700`.

**M2 — the migration (PR 2):**
9. shell lib.rs `icon_button` + call sites → painted icons; status.rs.
10. `HubSpec.icon` → registry id; hubs home/workbench/wiredfront/shows/
    present; app main.rs registration.
11. viziwiz palette.rs (21 sites) + deck (5) → `icon_label`/`draw_icon`.
12. DELETE `pub mod icon`; grep proves zero `cl::icon::` remains;
    Phosphor.ttf retirement deferred (separate cleanup once a census
    proves no other glyph consumers).
13. Proof: rail/topbar/palette screenshots both window sizes; kittest
    suites green; feel-bar check vs the original.

## 5. Boundaries

- 🚫 `/content` (operator data), the Lucide SVGs once vendored (sources
  are never mutated — derived geometry is generated, never written back),
  `dpl-vector` (untouched), generated `*_gen.rs` (never hand-edited).
- ⚠️ Ask first: retiring Phosphor.ttf/font registration; vendoring less
  than the full pool; any new fluid token for icon sizing.
- Out of scope: arbitrary operator-SVG intake (follow-up ADR per
  ADR-015), animated icons, stroke-weight variant baking.

## 6. Verification

- `cargo test -p dpl-svg` — grammar/corpus tests pass (corpus = all
  vendored SVGs parse + tessellate).
- `cargo green` exits 0 (fmt, clippy, tests, structure audit — proves
  the §2 ceiling + member ordering + no-orphan for the new crate).
- Break-it checks: an icons.json entry naming a missing Lucide file
  fails the build with the file named; an SVG with an unsupported
  construct fails `dpl-svg` tests loudly.
- `target\release\devpulselabs.exe --lib-section icons --screenshot
  docs\proof\lucide-icons-gallery.png --window 1100x700` (and maximized)
  — every registry icon renders in three renditions.
- M2: `rg "cl::icon::"` → zero hits; existing shell/deck/viziwiz kittest
  contracts green; rail + capture-palette proof shots at both sizes.

## 7. Commit plan

Two PRs, each branch → `gh pr create` → double-green → merge:
1. `feat(coldlight): the owned Lucide icon engine (ADR-015, POAM V3)` —
   vendored pool, dpl-svg, icons.json, generator, gallery section,
   architecture/audit amendments, this plan + the ADR.
2. `feat(coldlight): the icons wear Lucide — Phosphor glyph consts retired`
   — the 11-file migration, HubSpec doc, proofs.
STATE.md updated after each merge; exe rebuilt HERE both times.

## 8. Evidence

(filled during execution)
