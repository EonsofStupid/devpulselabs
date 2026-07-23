# L1 — the IA doctrine: rail, houses, Shows (operator, 2026-07-23)

Status: VISION CAPTURE, extended by the operator mid-review — awaiting
confirmation of the remaining shapes before code. Pre-release law: the
wrong implementations are REBUILT, never shimmed or built upon.
Priority: ABOVE the roadmap.

## 1. RATIFIED this session (operator's answers)

1. **The Library hub DISSOLVES into Workbench → Library.** One
   ingredient home: seeds, templates, blocks, elements, and the
   vocabulary walls (presets · moods · fonts · color pool) become
   sections of Workbench's Library. The top-level hub leaves the rail.
2. **Replays play IN PLACE in the wiredFRONT Gallery** — tapping a
   session card opens the player right there; Sessions stays as the
   dedicated list+player. Everything captured is reviewed in
   wiredFRONT.
3. **Clear seed folders = BY TEMPLATE CLASS**: `seeds/slides/static/`,
   `split/`, `animation/`, `interactive/` — the registry's own
   classes; the library wall's sections ARE the folders.
4. **Home stays the backdrop.** It is part of the STREAMING PROP tool
   — a calm mood floor, deliberately unbusy. Do not turn it into a
   dashboard.

## 2. The EXTENDED vision (operator's words, compressed)

- **The rail is wrong — it behaves like a megamenu.** Wanted: "a
  unique overlay where you click on wiredFRONT and then its subroutes
  appear and EXTEND DOWN, pushing the others down" — an ACCORDION
  rail: tap a hub, its subroutes unfold inline beneath it, the hubs
  below slide down.
- **wiredFRONT redefined — the PREP working area**: "getting
  everything ready for easy insert; copy animations or interaction
  effects; or i create a new custom slide for that content — whether
  it's images or a video or just pure code visually enhanced."
  Content-first work: take a capture/generation, prep it, and mint a
  custom slide from it. (The R1 editor lives here — this confirms its
  home.)
- **NEW rail entry: SHOWS.** On the left rail:
  ```
  Shows
    EffingAI
    DevPulse News
    Eons of Stupidity
  ```
  Each show's world lives under it — its episodes, its GALLERY, and
  the content used for social-media posts, "all of that properly
  organized under here with their gallery and organization." The
  content tree (`content/<show>/`, X3) becomes VISIBLE and navigable
  per show.
- **Workbench extends (later)**: generate PDF carousels; handle
  social-media posts — "hit execute and have it impersonate and post
  once we agree on the content, where it literally types it in and
  adds the media." (The posting automation composes with Program
  DEMO's browser-driver ADR — one driver, two uses: demonstrations
  and posting.)

## 3. The corrected map (as now understood)

```
RAIL (accordion — tap a house, its subroutes unfold, others push down)
├─ Home        the backdrop — the streaming prop floor. Calm. Done.
├─ Shows       the OPERATOR'S CONTENT, by stream category
│   ├─ EffingAI          episodes · show gallery · social content
│   ├─ DevPulse News     episodes · show gallery · social content
│   └─ Eons of Stupidity episodes · show gallery · social content
├─ Present     the show floor — tap a show, full-bleed deck runtime
├─ Workbench   the BUILD house — Library (ALL raw ingredients:
│              seed slides by class · templates · blocks · elements ·
│              presets/moods/fonts/colors) → Editor (tailor) →
│              Episodes (compose) → Studio; later: PDF carousels,
│              social post execution
├─ wiredFRONT  the PREP house — PixVix captures · Gallery (review
│              EVERYTHING captured, play in place) · the R1 editor
│              (stitch/effects/custom slides from content) · Sessions
└─ Chat        the agent console (route-aware; F3)
```

The pipeline across houses:

```
CAPTURE/GENERATE (wiredFRONT) ──prep/effects/custom slide──▶
INGREDIENTS (Workbench/Library) ──edit──▶ TAILORED PIECES ──insert──▶
TEMPLATES ──enhance (elements+traits)──▶ EPISODES ──keep/organize──▶
SHOWS (the Shows rail) ──▶ PRESENT (play) · export · social post
```

## 4. Wrong-state audit (what gets rebuilt)

- The rail expands as a menu, not the wanted accordion overlay.
- TWO libraries (top-level hub + empty Workbench subroute) + a
  separate SEEDS subroute + a TOKENS placeholder — four fragments of
  the one ingredient library.
- `seeds/slides/*` is flat — no class folders.
- No Shows presence on the rail at all — the operator's actual
  content (X3's tree) has no navigable home; Present's picker is the
  only window into it.
- The Gallery lists sessions it cannot play (fixed by ratification
  #2's design).

## 5. RULINGS (operator, 2026-07-23 — all shapes closed)

1. **A show's page = Episodes · Gallery · Social.** Three sections:
   the show's episodes (tap → edit or present), the show's own
   gallery (everything in `content/<show>`, thumbnailed), and its
   social-media content. The whole show world in one place.
2. **The accordion is ONE-OPEN-AT-A-TIME.** Tapping a house unfolds
   its subroutes downward AND folds whatever was open; the houses
   below push down; tapping the open house folds it. Compact always.
3. **wiredFRONT mints → Library ingredient.** A custom slide created
   in the prep house files as an INGREDIENT — it appears in
   Workbench → Library like any seed, gets tailored by the Editor,
   inserted into episodes. One pipeline, no dead ends.
4. **Social lane**: disk home `content/<show>/social/`. Platforms for
   the execute-and-post automation: YouTube (Shorts + community),
   X/Twitter, TikTok, Instagram/Facebook, **potentially Twitch, and
   eventually the operator's OWN community platform** — the adapter
   registry must leave room for a first-party target, not just the
   big four. (Automation = type the post + attach media on the
   operator's own accounts AFTER content approval; shares the
   browser-driver ADR with Program DEMO.)

## 6. The rebuild passes (once §5 closes)

- **L1a — the accordion rail** (shell): the wanted overlay behavior.
- **L1b — seeds into class folders** + doc-layer walk + doctor.
- **L1c — Workbench/Library rebuilt** as the sectioned ingredient
  library (walls absorbed; SEEDS/TOKENS subroutes dissolve; Library
  hub crate dissolves; --focus repointed; rail loses the hub).
- **L1d — the Shows rail entry**: per-show pages over `content/<show>`
  (episodes · gallery · social sections per §5's answers).
- **L1e — Gallery plays sessions in place.**
Each: plan → green → real-exe proof → commit, as always.
