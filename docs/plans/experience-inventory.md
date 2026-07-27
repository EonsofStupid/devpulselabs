# Experience inventory — Workbench Library (layman census)

The operator-facing gate for catalog fidelity. A surface that looks empty
or invents ingredients fails this inventory. Grounded 2026-07-26 against
live code (`crates/hubs/workbench`, `dpl-slides` registry, seed tree).

## What the Library is

Workbench → Library is the **one ingredient home**: starter slides, live
slide designs, reusable blocks, and the ColdLight vocabulary (traits,
colors, presets, fonts, moods). Tap a chip; that wall is below. No top-level
Library hub (dissolved in L1c).

Route: `workbench/library`. Templates solo via `--focus <template-id>`.

## Chip row (expected)

| Chip | Operator meaning | Status (2026-07-26) |
|------|------------------|---------------------|
| seeds | Ready-made starter slides you copy into a show | Real for `static` (4 seeds); other classes empty folders |
| templates | Every slide design, running live | Real — 18 demos; was a flat scroll (class headers = Cycle 2) |
| blocks | Small reusable pieces templates are built from | **Was missing** — Cycle 2 adds wall over existing paint fns |
| traits | Composable looks (neubrutal / clay / cyber / retro) | Real |
| colors | The whole color pool | Real |
| presets | Saved style / motion bundles | Real (looks rich; motion rows thinner) |
| fonts | Every typeface in its own face | Real |
| moods | Backdrop color moods | Real |

Out of chip row (elsewhere): Studio subroute still a placeholder; Elements
chip deferred (no registry yet); “overlays” = trait hover/active states,
not a separate catalog.

## Template classes (must be visible)

| Class | Registry demos | Seed folders |
|-------|----------------|--------------|
| static | title, stat, callout, stats, code + 6 infographics | 4 seeds |
| split | split | empty (`.gitkeep`) |
| animation | **none** | empty |
| interactive | reveal, choice, quiz, cost-meter, predict, drag-match | empty |

Done looks like: every class has a section header on Templates and Seeds;
empty classes say so honestly; no invented demos.

## Blocks (existing paint — must surface)

`dpl_slides::blocks::{eyebrow, badge, stat, ambient_backdrop}` — used inside
templates; Cycle 2 shows them as live swatches on the Blocks wall.

## Done for a fidelity pass

1. Inventory authored (this file).
2. Highest gaps closed without inventing templates: Blocks chip, class
   headers on Templates + Seeds, EguiPilot fixture on `workbench/library`.
3. Real-exe EguiPilot proof + STATE tip.
