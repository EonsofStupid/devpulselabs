# ColdLight Design OS

**Status:** operator charter — pre-release  
**Ratified:** 2026-08-03  
**Rank:** **OS** (above all subsystem laws)

ColdLight Design OS is the full design operating system for DevPULSE Labs:
design language, tokens, matrix theme system, composition metal, component and
pattern libraries, and the generator that compiles authored design data into
typed runtime assets.

---

## 1. One-sentence charter

> **ColdLight Design OS** — a matrix-driven design operating system:
> **RenderForge** compiles authored tokens and **ancestral seeds** into a
> hyper-modular **component and pattern library** with default advanced
> **Themes** resolved as legal **matrix coordinates**, packaged as editable
> **presets** the operator applies platform-wide or to a card.

---

## 2. The three product layers (operator vision)

| Layer | What it is |
|---|---|
| **Ancestral seeds** | Baseline DNA inspired by the most noticeable visual identities of design lineages (Bauhaus · Art Deco · Swiss · Aero · Brutalist). Mixable heritage defaults — not a flat skin list. |
| **Raw visual entities** | The modular atoms of appearance (geometry, edge, material, atmosphere, type, color, motion, state, FramePatches, tokens). Mix and match → limitless visuals without hand-authoring every combination. |
| **Presets** | Named, easy to edit, apply at will — **across the platform** or **to a specific card**. What non-technical users touch: click, apply, live preview, tweak, save. |

Complexity stays in the background. The face is **Theme / Themes / ThemePreset**.
**Looks is not a product word.**

---

## 3. Layer stack

```
ColdLight Design OS
  design language · doctrine · census · paint laws
RenderForge (macroparser)
  authored tokens → generated registries, legality, presets
Matrix theme system
  facets · coordinates · ancestral seeds · ThemePresets
Composition metal
  FramePatch · FrameStyle · compose() · WidgetState
Component + pattern library
  Pieces · Combos · Layouts · Starters · Recipes · Patterns
Surfaces
  Workbench · Chat · Shell · … (consume OS; do not invent paint)
```

---

## 4. Glossary

| Term | Definition |
|---|---|
| **ColdLight Design OS** | Full design operating system |
| **RenderForge** | Design compiler / macroparser: authored data → typed generated assets |
| **Ancestral seed / lineage** | Heritage default for advanced baseline styling |
| **Raw visual entity** | Mixable atom of appearance (token, patch axis, facet value, …) |
| **Matrix / coordinate** | Legal faceted combination → a Theme resolution |
| **Theme** | Product face: dressed appearance of surface, card, or document |
| **ThemePreset / preset** | Named, editable, apply-at-will theme bundle |
| **FramePatch** | Engineering: stackable partial `egui::Frame` |
| **Component / Pattern** | Registered library units for authoring |

---

## 5. Subsystem laws

| Rank | Document | Role |
|---|---|---|
| **OS** | **This file** | Charter |
| **S0** | [`theme-vocabulary-law.md`](theme-vocabulary-law.md) | Theme face · FramePatch metal · Looks forbidden |
| **S1** | `docs/plans/coldlight-naming-and-the-pattern-library.md` | Engineering rename + pattern→catalog law |
| **S2** | [`doctrine.md`](doctrine.md) | Composition / paint |
| **S3** | [`design-system.md`](design-system.md) | Component/pattern architecture |
| **S4** | [`coldlight-matrix.md`](coldlight-matrix.md) | Matrix facets and lineages |

**Merge rule:** OS + S0 override every plan. Pre-release: rewrite in place, no
deprecation shims, no dual product vocabulary (no Looks trail).
