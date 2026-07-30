# ADR-015 — the owned SVG icon engine (Lucide, carried the visx way)

**Status:** PROPOSED 2026-07-29 (operator directive, verbatim: "lets take
the time to pull lucide and then properly own it as part of coldlight, we
will have the SVG engine that is based off the airbnb engine airvx and
establish baselines for our diverse and default polished stylings").
**Deciders:** operator + this pass. **Gated by:** the visual-experience
POAM V3 (`docs/plans/poam-visual-experience.md`); executes as
`docs/plans/lucide-icon-engine.md`.

## Context

The original app's icon language WAS Lucide — the port's own code admits
it: `cl::icon` is documented as "Phosphor glyphs under the OLD lucide
inventory's semantic names", and `HubSpec.icon` carries "Text glyph for
the rail until the sprite atlas lands (P1)". The substitution was a
stopgap; this ADR retires it.

"The airbnb engine airvx" = **visx** (Airbnb's low-level visualization
primitives): unopinionated geometry building blocks, styling brought by
the caller, compose only what you need. That philosophy already lives in
this workspace — `dpl-vector`'s header is literally "callers bring a
`Color32`, geometry comes back". The icon engine extends the same law to
SVG: **geometry is data; ColdLight brings the styling.**

Lucide (ISC; Feather-derived icons MIT) is a strict discipline, not just
a set: every icon is 24×24, stroke-based, stroke-width 2, round caps and
joins. That strictness is what makes OWNING the engine tractable — the
icon subset of SVG is small enough to implement completely, correctly,
with zero dependencies.

## Decision

1. **Vendor Lucide 1.27.0 wholesale** (released 2026-07-25) into
   `crates/coldlight/assets/lucide/`: the `icons/*.svg` pool, the LICENSE
   text, and a `VERSION` file recording the tag + source URL. Precedent:
   Phosphor.ttf + its MIT text, the fonts pool + OFL texts. The pool on
   disk is the authored source; only REGISTRY-REFERENCED icons enter the
   binary (the fonts law, again).

2. **A new engine crate `dpl-svg`** — ENGINE layer, egui-free, dep
   ceiling ∅ (a law crate like dpl-proc). One concern: *the icon subset
   of SVG resolved to stroked triangle-mesh geometry.* It owns:
   - a minimal XML element reader (no external parser) for the Lucide
     shape vocabulary: `path`, `circle`, `rect`, `line`, `polyline`,
     `polygon`, `ellipse`;
   - the full `d`-attribute grammar (M/L/H/V/C/S/Q/T/A/Z, absolute and
     relative), elliptical-arc → cubic conversion;
   - adaptive cubic flattening (tolerance-driven) → polylines;
   - polyline stroking with round caps + round joins → pure vertex/index
     buffers (`(f32, f32)` + `u32` — no egui types).
   Unsupported SVG (gradients, transforms, text, fills beyond
   `currentColor` conventions) FAILS loudly — the engine's honesty is the
   point; Lucide's guidelines guarantee the subset.

3. **Geometry is generated at BUILD time.** `dpl-svg` rides as a
   build-dependency of `dpl-coldlight` (beside serde_json + palette —
   build-deps sit outside runtime ceilings). A new authored source
   `tokens/icons.json` (the 8th) maps semantic ids → Lucide names
   (one entry per MEANING, exactly like today's `icon` mod);
   `generate_icons` in build.rs parses the referenced SVGs, tessellates
   at tolerance 0.02 (sub-pixel to ~500 px), and emits `icons_gen.rs`:
   `IconDef { id, label, lucide, blurb, vertices, indices }` + registry
   fn + counts. A registry entry whose SVG is missing FAILS THE BUILD —
   the manifest⇄truth law fonts already enforce. Zero runtime parsing,
   zero runtime deps, zero per-frame tessellation.

4. **Styling composes at paint time, never in the geometry** (the visx
   law). The mesh is single-channel: painted by mapping the 24-box to a
   target rect and tinting vertices. Baselines established by this ADR:
   - **default polished**: `ink::BASE`/tone ink, geometry as authored;
   - **tone accent**: any ladder/tone color — one tint, same mesh;
   - **brutal**: the icon's own mesh re-drawn offset `[3, 3]` in shadow
     black UNDER the tinted pass — a hard offset shadow as a SHAPE, never
     a blur (doctrine law);
   - **duotone/glow**: painted wash or back-glow layers behind the mesh,
     from the trait system's additive axes.
   Stroke-WEIGHT variants (e.g. a heavier brutal cut) would be baked as
   additional generated weights per icon — deferred until a surface asks.

5. **The gallery gate**: icons join the Workbench library as
   `LibSection::Icons` (the 10th section), enumerating the generated
   registry — provable via the existing `--lib-section icons` harness
   path. An icon no surface can browse is a gallery-gate violation.

## Rejected

- **usvg/resvg or lyon as dependencies** — capable, but "properly own
  it" was the directive; the Lucide subset doesn't need a full SVG
  engine, and the dep discipline (§2 ceilings, ADR-009) treats external
  capability as a decision, not a convenience. Revisit ONLY for
  arbitrary-SVG intake (below).
- **Rasterize to textures** — resolution-dependent, atlas management,
  and it forfeits per-vertex tinting (the styling baselines above come
  free on meshes).
- **egui-phosphor / icon fonts, continued** — glyph fonts can't carry
  duotone, offset-shadow shapes, or per-vertex treatment; and the
  fidelity debt to the original's Lucide language stands.
- **Runtime SVG parsing** — a runtime dep + cache complexity for icons
  that never change between builds; the generated-consts pattern is the
  house pattern (7 sources already work this way).

## Consequences

- `cl::icon::*` Phosphor string consts and every `RichText`-glyph call
  site (40 across 11 files) migrate to painted icons; `HubSpec.icon`
  becomes a registry id and its "until the sprite atlas lands" note is
  resolved. Phosphor.ttf itself is retired in a follow-up once a grep
  census proves zero remaining glyph consumers.
- `docs/design/architecture.md` §1 (ENGINE list) + §2 (ceiling `dpl-svg
  = ∅`) and the xtask audit table are amended in the same commit that
  adds the crate (structure law).
- **Arbitrary operator SVGs (POAM V3's "years of svgs") stay OPEN**: the
  Lucide subset is the baseline engine, not a general SVG renderer. When
  the operator points at the collection, a follow-up ADR decides full-SVG
  handling (extend `dpl-svg` vs an ADR-009 helper) against real files.
