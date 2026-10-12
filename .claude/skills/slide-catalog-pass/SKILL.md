---
name: slide-catalog-pass
description: Execute ONE slide-catalog pass — faithfully port a single element, block, infographic, or template from the original app to egui using only ColdLight ingredients, author its seed document, categorize + tag it, and surface it organized under the Workbench Library. Use when building or porting slides, populating the Library, or closing slide-catalog gaps. Runs looped, one slide per pass.
---

# Slide-catalog pass — one slide, ported faithfully

**Argument:** the catalog item to port (e.g. `templates/reveal-list`,
`infographics/donut-share`, `elements/callout-badge`). One item per pass.
Precondition: `coldlight-system` foundation gate is green (its ingredients exist).

## The source of truth

The original catalog lives at `D:\localdev\review-archive\devpulse labs\src\domains\slide-catalog`
(READ-ONLY): `elements/` · `frame/` · `infographics/` · `templates/`. Read the
original `.tsx` for THIS item in-session — never port from memory. The port lives
in `crates/slides/src/{blocks,infographics,templates}` + seed docs in
`seeds/slides/<category>/<name>/slide.json`.

## The two deliverables of every pass

A ported catalog item is not done until BOTH exist:
1. **The renderer** — egui code in `crates/slides`, painting ONLY via
   `compose(...)` + coldlight paint fns (no ad-hoc hex, no bespoke frame helper).
   Faithful to the original's structure, layout, and feel.
2. **The organized seed** — an authored `slide.json` under the right **category**
   folder (`static | animation | interactive | split | infographics`), carrying
   `kind`, `category`, stackable `tags` (`topic:*`, `episode:*`, …), a `recipe`
   (trait stack), and authored content blocks — REGISTERED so it appears in the
   Workbench Library, filterable by category × tags. A renderer with no organized
   seed is only half-ported (this is the gap that made the Library feel empty).

## Protocol

1. **Read the original.** The item's `.tsx` (+ its element/infographic deps).
   Note every capability: layout, states, reveal steps, data shape, interaction.
   Nothing dropped silently — if a capability can't port, record it in §8.
2. **Ground the port target.** Read the current `crates/slides` code for this item
   (many exist but paint ad-hoc). Read the seed contract in `crates/doc`.
3. **Plan** (`docs/plans/`, 7 sections + §8) if the item is non-trivial or new.
4. **Build** via `coldlight-system` laws — renderer through `compose(...)`; author
   the seed doc; wire the category + tags; register it in the manifest/Library.
5. **Green** — `taskkill //F //IM devpulselabs.exe`, then `cargo green`.
6. **Verify vs original** (`true-verify`) — screenshot the ported slide at real
   window sizes and set it beside the original's look. Feel-bar: matches the first
   app's polish, or it's not done.
7. **Commit** one item per PR (or a tight category batch). Rewrite
   `docs/status.md` (catalog progress under Now). Rebuild the operator exe +
   move its mtime.

## Catalog checklist (tick as ported — the "capture ALL slides" contract)

- **elements** (3): callout-badge · eyebrow · media-figure
- **frame** (1): slide-frame
- **infographics** (7): bar-compare · cost-meter-face · donut-share · engine/chart-frame · line-trend · probability-bars · stack-layers
- **templates** (16): big-number · bullets-list · callout-card · choice-poll · code-block · comparator-panel · cost-meter · drag-match · media-hotspots · predict-rank · quiz-panel · reveal-list · stats-grid · still-zoomable · title-card (+ split-panel, port-net-new)

Missing renderers to net-new during the loop: **bullets-list, comparator-panel,
media-hotspots, still-zoomable**, and verify **engine/chart-frame** (`stage.rs`).
Every ported item gets an organized seed — the renderer existing is not enough.
