# 2026-07-28 — Authoring: the FULL vision (pre-mockup capture)

**Purpose:** a loss-proof capture of the operator's authoring vision, dictated
this session after a course-correction. The prior build (Passes A/B/C) *forked* —
it shipped code ahead of the full intent and, worst of all, embedded the OLD raw
line-by-line `dpl-editor` where the streamlined guided flow belongs. We are
resetting: **capture everything here → build the full clickable mockup as the
single source of truth → implement by CAPTURING the mockup, losing no complexity
or capability.** Read this + memory before touching anything.

---

## The reset (operator, verbatim intent)

- Stop forking. Stop coding ahead of the intent. **Mock-first**: the full mockup
  is authored FIRST and becomes the spec; implementation then *captures* it.
- Nothing is lost. The complex editors/capabilities we've already built are NOT
  thrown away — they become the **opt-in deeper iterations** behind a simple
  default, never the default path.
- The old raw `dpl-editor` (line-by-line, rough) is **fully replaced** by the
  guided editor — it does not survive as-is.

## Workbench → Editor: TWO LANES (first choice)

The Editor is the Workbench's default surface (defaults to the last-used lane).
On entry you make a **first choice** of two lanes — click or hover, a **tooltip
explains** each. Design-system terminology is **industry-aligned internally**;
the UI keeps simple words.

### Lane 1 · PLATFORM — author the design system itself
Guided workflows + **skills** + **schemas** keep the composition chain clean,
optimized, and well-authored: the UI writes organized, best-practice code and
the person literally cannot break the patterns. This lane REPLACES the old raw
editor.

**The composition chain (raw → composed), which the schemas enforce ("adhere to
the chain"):**
- **Traits** (UI) = *treatments* (internal) — the raw visual treatments; we
  generate the raw visuals here.
- **Presets** — authored bundles (of traits/style).
- **Elements** — the atoms.
- **Blocks & Templates** — **use** presets + elements; never hand-rolled.
- **Recipes** — behaviors/interactions, applied on top.

So: author raw **traits/presets** → **elements** are the atoms → **blocks &
templates** compose from presets + elements → **recipes** add behavior. A
template can't hand-roll a look; it must reference the established chain. Exact
internal names follow industry-standard design-system naming (tokens / variants
/ components / etc.) — finalize when building, but the CHAIN above is the law.

### Lane 2 · CONTENT — author actual slides & content
The streamlined production editor: build real **slides → episodes/shows** for
**streaming, news, and the episodic TV show**. Feeds the EXISTING episodes
machinery (W10 / `dpl_doc` episodes / Studio place), not a separate silo. This is
where the overlay/popup guided workflow lives — the streamlined experience the
forked Pass C got wrong.

## User-facing vocabulary (KEEP SIMPLE): Blocks · Elements · Effects · Templates
Four words on the UI. Internal code uses the proper industry-aligned names
(treatments/presets/recipes/…). "Effects" is the simple UI face of the
recipes/treatments world. Simple front, rigorous engine — no jargon on a tile.

## THE CORE UX LAW — the 3-click overlay (this is the whole point)

The default authoring interaction is an **overlay system** that:
- **reacts seamlessly to the viewport** — scrolls/reflows as you move, no modal
  friction, options stay in view;
- makes options **easy to SEE** — you glance, you recognize, you pick;
- **click → drills down → inserts** — a click inserts; the next click drills into
  the next level; **the destination (an actual insert) is reached within 3
  CLICKS.** Three clicks, max, from "I want something" to "it's on the card."

This click→drill-down→insert flow is the **DEFAULT**. It **evolves into the more
complex iterations we've built before** (the After-Effects-style editor, the full
composer, deep per-property control) as **opt-in depth** — the power is one level
down, never in the default path. Optimize relentlessly for: fewest clicks,
most-visible options, instant insert.

## What this supersedes

- The forked authoring code — Pass B (PR #128) and Pass C (PR #130) — is
  superseded by the mock-first redo. PRs stay UNMERGED. Pass C's Content tab (the
  embedded raw `dpl-editor`) is exactly the mistake being replaced.
- Pass A (merged, #124: the `BuildEvent::CardAuthored` signal spine + `author`
  subroute + seam envelope) and DS2 (#114: the element census) are sound
  foundations and stay — the mockup builds ON them.

## Next (fresh, after /clear)

1. **Build the full clickable MOCKUP** (HTML, self-contained) capturing: the
   two-lane Editor (Platform / Content), the composition chain, the four-bucket
   simple vocabulary, and above all the **3-click seamless-overlay** default with
   the deeper iterations behind it. Reference the prior mock
   (`docs/design/mocks/authoring-surface-mock.html`) but this is a FULLER spec.
2. Operator ratifies the mockup.
3. Implement by **capturing** the mockup — signal-first, ColdLight, Census law,
   layer law — losing no complexity. Plan → work → test → verify per pass.

Memory: [[authoring-charter-vision]], [[design-system-build-order]],
[[git-workspace-and-ci-state]].
