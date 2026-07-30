# ADR-016 — The icon language: SVG sources → generated stroke geometry

**Status:** FILED 2026-07-30, PARTIALLY SUPERSEDED AT FILING. Authored in
a parallel session as "ADR-015" while #163 merged the same territory as
`adr-015-svg-icon-engine.md`; renumbered and filed verbatim below during
the 2026-07-30 alignment pass. The ENGINE decision below (§2–3:
roxmltree/svgtypes/kurbo build-deps, polyline storage,
`assets/icons/lucide/`, pinned tag in icons.json) is SUPERSEDED by the
landed zero-dep `dpl-svg` engine (ADR-015 + its 2026-07-30 amendment:
stroked meshes baked at build, pool at `assets/lucide/`, tag in
`VERSION`). Still LIVE here: §4's styled paint vocabulary (brutal at
icon scale; clay per the 2026-07-30 "clay IS a button" doctrine ruling)
and §5's operator-SVG-same-pipe future — both gated on the vision mock
(`docs/design/mocks/visual-language-v3-mock.html`).

*(original text, verbatim, below)*

**Status:** PROPOSED 2026-07-29 (rides the V3a pass PR; merging the PR
ratifies it). Decides the SVG→egui path that POAM-visual-experience V3
gated everything on.

## Context

- Operator redirect (2026-07-29, verbatim): *"Lucide for now that we will
  style in neubrutalist and claymorphism variants"* — the personal SVG
  library becomes a LATER intake through the same pipeline, not the gate.
- Icons today are Phosphor FONT glyphs (`cl::icon`, semantic names carried
  over from the old shell's *lucide* inventory — so Lucide is a return to
  the original family, port-fidelity). Font glyphs style by color only:
  no stroke-weight control, no geometry-level treatment.
- There is no SVG tooling anywhere in the workspace; `HubSpec.icon` has
  carried the note "until the sprite atlas lands" since P1.

## Decision

1. **Sources are vendored, pinned, licensed.** Lucide SVGs (ISC) land at
   `crates/coldlight/assets/icons/lucide/` with the license file beside
   them — the exact Phosphor.ttf precedent. The pinned upstream tag is
   recorded in `tokens/icons.json`. Sources are never mutated (house law).
2. **Build-time geometry, not runtime SVG, not a raster atlas.**
   `tokens/icons.json` (one entry per MEANING — census law) drives
   `build.rs`: each SVG's stroke centerlines are flattened ONCE into
   24-box polylines and emitted as const data (`icons_gen.rs`). Paint
   scales + strokes at runtime — any size, any tone, zero new runtime
   deps, zero textures.
   - *vs runtime resvg:* a heavy runtime dependency, texture uploads,
     recolor-to-tint. Rejected.
   - *vs raster/sprite atlas:* fixed sizes and baked colors — styled
     variants explode combinatorially. Rejected.
   - *vs the Lucide TTF (Phosphor pattern):* cheapest, but geometry is
     baked into the font — no weight control, no geometry-level variants,
     and no seam for the operator's own SVGs later. Rejected as the
     language; Phosphor glyphs REMAIN the fallback vocabulary.
3. **Build-deps only** (the `palette` precedent — the runtime graph is
   unchanged): `roxmltree` (XML), `svgtypes` (`SimplifyingPathParser`
   normalizes path data to Move/Line/Curve/Quadratic/Close with arcs
   pre-converted to curves — verified against 0.16.1 docs), `kurbo`
   (adaptive flattening). Accepted element vocabulary: `path`, `circle`,
   `ellipse`, `rect`, `line`, `polyline`, `polygon` — anything else fails
   the build with the icon named (honest failure, the traits precedent).
4. **Style is applied at paint, never baked into assets.** The icon is
   geometry; the treatment is the trait system's job: `icons::paint`
   (plain, tintable), `icons::paint_brutal` (the neubrutalist hard-offset
   shadow axis at icon scale), clay = the icon sitting IN `cl::clay()`
   (doctrine: content sits in it; clay is never a button). Trait data in
   `tokens/traits.json` stays the single styling truth.
5. **The operator-SVG future is the same pipe.** Their library lands as a
   sibling source dir + `icons.json` entries through the SAME generator;
   the registry API does not change. Curating/normalizing that set is its
   own later pass (POAM V3 note).

## Consequences

- Adding icon N+1 = one vendored SVG + one `icons.json` line.
- Unknown id at runtime degrades (`Option` — caller falls back to a
  Phosphor glyph or bullet); a missing file, bad geometry, or unsupported
  element FAILS THE BUILD.
- coldlight's build script grows ~24 tiny SVG parses — negligible.
- No architecture-audit change: dpl-coldlight is a SUPPORT crate with no
  §2 ceiling; build-deps sit outside ceilings (precedent: `palette`).
