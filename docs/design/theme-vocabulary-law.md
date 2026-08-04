# Theme vocabulary law (S0)

**Status:** operator law — pre-release  
**Ratified:** 2026-08-03  
**Parent:** [`coldlight-design-os.md`](coldlight-design-os.md)  
**Scope:** product language for how a card or surface is dressed.

This file is **S0** for the **Theme face** of ColdLight Design OS. Every plan,
mock, chip label, and agent skill matches this file or is wrong. Pre-release:
**rewrite in place** — no deprecation path, no alias, no “formerly Looks.”

---

## 1. The decision

**The product noun is Theme.**

- Operator chips and shelves: **Themes**  
- Documents: **`Theme` / `theme.json`**  
- Named apply-at-will bundles: **`ThemePreset`** (presets)  
- Engineering partial of `egui::Frame`: **`FramePatch`** — metal, never the chip  

---

## 2. Forbidden and required

| Word | Verdict |
|---|---|
| **Theme / Themes / ThemePreset** | **LAW — product face** |
| **`dpl_doc::Theme` / `theme.json`** | **LAW — document cascade** |
| **FramePatch / FRAME_PATCHES / `frame-patches.json`** | **LAW — engineering only** |
| **Looks / Look / LookPreset / `looks` slug** | **FORBIDDEN** — remove, do not deprecate |
| **trait / TraitDef / traits.json** (ColdLight product sense) | **FORBIDDEN** — rename to FramePatch (S1) |

English “looks wrong” and Rust `pub trait ElementDemo` are not this law.

---

## 3. Face vs metal

| Face (drawn) | Metal (code / tokens) |
|---|---|
| Themes | `frame-patches.json`, `FramePatch`, `FRAME_PATCHES` |
| Theme / ThemePreset | document + preset registries |
| Combos / Pieces | `blocks/` / `elements/` |

---

## 4. Identifier map (target)

| Kind | Correct | Forbidden |
|---|---|---|
| Library chip | `LibSection::Themes`, slug `themes`, label Themes | Looks, Traits |
| Author shelf | `Shelf::Themes`, label Themes | Looks |
| Shelf metal slug | `frame-patches` (after N1) | `looks`, `traits` |
| Preset type | `ThemePreset`, `THEME_PRESETS`, `theme_preset` | LookPreset, LOOK_PRESETS, look_preset |
| Apply helpers | `apply_theme_preset`, `with_theme`, `theme_enter`, `theme_preset_tray` | apply_look, with_look, look_enter, look_tray |

---

## 5. Rank

| Rank | Path |
|---|---|
| OS | `coldlight-design-os.md` |
| S0 | **This file** |
| S1 | `docs/plans/coldlight-naming-and-the-pattern-library.md` |
| S2–S4 | doctrine · design-system · coldlight-matrix |
| S5+ | `crates/coldlight/`, doc, Workbench face, STATE, resume |

OS + S0 win. Code that still says Looks is fixed to match this law.
