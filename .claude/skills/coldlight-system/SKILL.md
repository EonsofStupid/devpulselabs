---
name: coldlight-system
description: The DevPULSE Labs visual-language gate AND the composable-ingredients foundation. Invoke before ANY pass that adds/edits UI or builds a slide, element, block, or template. Ensures the stackable ColdLight ingredients (traits, elements, blocks, templates) already exist as generated egui code ready to colocate, and holds every surface to the trait doctrine — neubrutalism/claymorphism base + cyber/retro stack, tooltip law, generated-tokens-only, any-shape proofs. Replaces the old dpl-style gate.
---

# ColdLight — the composable styling system + the gate

Read `docs/design/doctrine.md` first (the authored treatment doctrine, ratified
in `docs/plans/pass0-coldlight-treatments.md`). This skill is BOTH the foundation
guarantee and the per-surface gate. It supersedes `dpl-style`.

## The architecture (the concept, ported to egui — do not fight it)

The original's web pipeline (author JSON → Style Dictionary → CSS vars → cascade)
lands in egui as **author JSON → `build.rs` generates Rust consts → a pure
compositor resolves a `Style` struct → immediate-mode paint**. Same concept,
better fit: the compositor is one pure, testable, cacheable function, not a
browser resolving var-chains.

```
tokens/*.json (AUTHORED — the only thing you touch)
  hues · fluid · motion · moods · fonts · presets · TRAITS
        │  build.rs (the design build; unknown ref fails the build)
        ▼
generated *_gen.rs  (NEVER hand-edit — carries "GENERATED … DO NOT EDIT")
        │
compose(base, &[trait_id], overrides) -> ResolvedStyle   (pure fn, per frame, cacheable)
        │
paint fns take ResolvedStyle → egui::Frame + painter ops
        │
manifest (generated) ← the Workbench editor reads THIS ("only show what exists")
```

**Two orthogonal axes — never conflate them:**
- **Styling ingredients** = `traits · elements · blocks · templates` (what dresses a surface).
- **Slide categories** = `static · animation · interactive · split · infographics`
  (what KIND of slide) — a `category` field + registry key, plus open stackable
  `tags` (`topic:*`, `episode:*`, …). Independent of styling.

## The treatments (traits — authored in `tokens/traits.json`, composed per-axis)

Two BASE treatments + two STACKABLE traits (operator-ratified). Never hand-roll
any of them; if a variant is missing, author the trait data and regenerate.

- **NEUBRUTALIST** (base, actionable) — the egui-native strength move. Flat
  saturated fill, crisp 2px hard edge, HARD offset shadow `[3,3]` zero blur
  (a shape, not a glow), tight radius. The "lit-from-behind" pop is a CHEAP
  painted back-glow / tone-wash — NEVER a real blur. Loud, honest, fast.
- **CLAYMORPHIC** (base, passive) — soft extruded surface (the original's
  neumorphic specialty): DIRECTIONAL double-shadow (light top-left, dark
  bottom-right), generous radius, hairline edge, optional two-color diffuse
  tint. Content sits IN it; chrome sits ON it.
- **CYBER** (stack) — cyan glow + thin bright edge + hue-drift. Rides on top.
- **RETRO** (stack) — scanline texture + chunky amber edge + sharp corners + flicker.

**Composition law (per-axis, from the resolved-struct compositor):**
- **Additive axes** — `shadows`, `tint/gradient layers`, `back-glow`, `animation`
  — every trait's contribution CONCATENATES (retro scanline AND cyber glow both paint).
- **Override axes** — `fill`, `edge`, `radius`, `ink`, `font` — last trait in the
  stack wins. The additive/override split is a static table beside the axis enum.

## Foundation gate — run BEFORE any slide/UI is built

The pass is not ready to build until, for every ingredient it needs:
1. The **trait** exists in `tokens/traits.json` and regenerates cleanly.
2. The **element/block/template** exists in `crates/slides` and paints ONLY via
   `compose(...)` + coldlight paint fns — never ad-hoc `Color32::from_rgb`, never
   a bespoke `*_frame` helper (the old `brutal_frame`/`clay_frame` are legacy;
   migrate them into the trait system, do not extend them).
3. The ingredient appears in the generated **manifest** (so the Workbench editor
   can offer it). An ingredient no surface/manifest exposes is a gallery-gate
   violation.
If an ingredient is missing → build IT first (same discipline), then proceed.

## Per-surface laws (every touched surface)

1. **Tooltip law** — every interactive control ships `cl::tip(resp, "…")` in plain
   layman words. Grep the diff for `.clicked()`; each near a tip-less control fails.
2. **Generated tokens only** — color from `ramp(id)` / `Tone`; size from
   `Fluid::of(rect)`; radius/space/motion from generated consts. No raw hex, no
   magic px, no ad-hoc paddings.
3. **Paint vocabulary** — crisp edges, solid fill + `gradient_tint`, painted
   back-glow/sheen/seam, hard offset shadows. **Never blur, never feathered borders.**
4. **Any-shape proof** — screenshot the surface at maximized AND `--window 1100x700`
   minimum (wide/tall extremes when layout changed). The fit guard owns the window.
5. **Feel bar** — look at the shot and ask "would the operator say this matches the
   first app's polish?" If not, it is not done — fix before PR.

## Record

Doctrine deviations go in the plan's §8. Update `docs/STATE.md` for surfaces still
un-migrated. Every UI/slide pass MUST invoke this skill.
