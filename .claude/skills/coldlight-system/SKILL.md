---
name: coldlight-system
description: The DevPULSE Labs visual-language gate AND the composable-ingredients foundation. Invoke before ANY pass that adds/edits UI or builds a slide, element, block, or template. Ensures the stackable ColdLight ingredients (FramePatches, elements, blocks, templates) already exist as generated egui code ready to colocate, and holds every surface to the doctrine — Theme face, FramePatch metal, neubrutalism/claymorphism base + cyber/retro overlays, tooltip law, generated-tokens-only, any-shape proofs. Replaces the old dpl-style gate.
---

# ColdLight Design OS — gate + composition metal

Read first, in order:

1. `docs/design/coldlight-design-os.md` — OS charter (seeds · entities · presets · Themes)
2. `docs/design/theme-vocabulary-law.md` — **Theme face**; Looks forbidden; FramePatch is metal
3. `docs/design/doctrine.md` — composition / paint laws

This skill is BOTH the foundation guarantee and the per-surface gate. It
supersedes `dpl-style`.

**Product face:** Themes / Theme / ThemePreset.  
**Engineering metal:** FramePatch / FrameStyle / compose.  
**Forbidden:** Looks, LookPreset, ColdLight product `trait` / TraitDef.

## The architecture (egui — do not fight it)

Author JSON → RenderForge / `build.rs` generates Rust → pure `compose` →
`FrameStyle` → immediate-mode paint.

```
tokens/*.json (AUTHORED — the only thing you touch)
  hues · fluid · motion · moods · fonts · presets · frame-patches · matrix
        │  build.rs (unknown ref / illegal coordinate fails the build)
        ▼
generated *_gen.rs  (NEVER hand-edit — "GENERATED … DO NOT EDIT")
        │
compose(tone, base, &[overlays], state) -> FrameStyle
        │
paint fns take FrameStyle → egui::Frame + painter ops
        │
manifest (generated) ← Workbench editor ("only show what exists")
```

**Do not conflate:**

| Concern | Words |
|---|---|
| Operator dress | **Theme / Themes / ThemePreset** |
| Composition metal | **FramePatch · FrameStyle · compose** |
| Library ladder | Pieces · Combos · Layouts · Starters (elements/blocks/templates) |
| Slide categories | static · animation · interactive · split · infographics |

## The treatments (FramePatches — `tokens/frame-patches.json`)

Two BASE + two OVERLAY patches (operator-ratified). Never hand-roll; author
patch data and regenerate.

- **NEUBRUTALIST** (base, actionable) — flat saturated fill, crisp 2px hard edge,
  HARD offset shadow `[3,3]` zero blur, tight radius, painted back-glow —
  NEVER real blur.
- **CLAYMORPHIC** (base) — directional double-shadow, generous radius, hairline
  edge, optional diffuse tint. Clay may be actionable (doctrine ruling).
- **CYBER** (overlay) — cyan glow + thin bright edge + hue-drift.
- **RETRO** (overlay) — scanline + chunky amber edge + sharp corners + flicker.

**Composition law:**

- **Additive** — `shadows`, `tints`, `back_glow`, `anim` — concatenate.
- **Override** — `fill`, `stroke`, `corner_radius`, `override_text_color`,
  `override_font_id` — last overlay wins.

## Foundation gate — run BEFORE any slide/UI is built

The pass is not ready to build until, for every ingredient it needs:
1. The **patch** exists in `tokens/frame-patches.json` and regenerates cleanly.
2. The **element/block/template** exists in `crates/slides` and paints ONLY via
   `compose(...)` + coldlight paint fns — never ad-hoc `Color32::from_rgb`, never
   a bespoke `*_frame` helper (the old `brutal_frame`/`clay_frame` are legacy;
   migrate them into the patch system, do not extend them).
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
