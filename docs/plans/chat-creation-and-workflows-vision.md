# Chat creation & WiredFRONT workflows — program vision

**Status:** OPERATOR VISION CAPTURED 2026-07-27 — not yet ratified into passes
**Source:** operator statement, 2026-07-27 session (quoted requirements below)
**Sits under:** [`chat-window-product-vision.md`](chat-window-product-vision.md),
[`chat-architecture.md`](chat-architecture.md), R1 WiredFRONT editor program
**Bound by:** provider runtime ADR (`chat-provider-runtime-adr.md`), ADR-009
(ffmpeg subprocess), the two-libraries law (Workbench ColdLight ingredients
vs WiredFRONT raw media), `/content` is never deleted by automation.

## 1. Route-aware chat as an operator surface

> "…this chat client needs to be properly route aware, so when I am working
> under WiredFRONT editor you can literally have chat then manage and
> control, add from the library of elements whether I directly import them
> or symlink an external."

Chat already carries route context (`chat.context.route`, route-aware
header) and can ask the shell to navigate. The vision extends chat from
*knowing* the route to *acting on* the route surface:

- With WiredFRONT editor active, chat can add elements from the library
  into the working comp;
- both import modes are first-class: direct import (copy) and
  reference-in-place (symlink an external) — the exact pair already landed
  as `import_reference` + `⧉ copy` ⇄ `⌁ in place` in WF external assets
  (#60). Chat drives the same seam; it does not grow a second import path.
- Chat actions on a route are **typed intents through the fabric/seam**
  (the W10a `place_slide`/`BuildEvent` shape), never free-form control.
  A2UI/coach (C-series) is the adjacent program; chat and coach must not
  fork two command vocabularies for the same surfaces.

## 2. In-chat generation with a draft cache (Grok first)

> "if I am working with grok … we have a mode button to trigger create
> video or create image; if I want to save that into the library as a
> draft we want to be able to click on it, see it preview, and then ignore
> it, where the cache will get flushed every 30 days if nothing is saved.
> We flag the user to say hey, today we are dumping, are you okay — yes /
> never show again / yes remind me next time too."

- **Mode button** in the composer (image / video) — a typed provider
  capability, only enabled when the selected provider genuinely qualifies
  it (`Capability::Images` / `Capability::Video` exist as declared targets
  today; nothing is qualified yet).
- **Draft lifecycle:** generated artifacts land in a *draft cache*, not the
  library. Click → preview. Save → promoted into the **WiredFRONT raw-media
  library** (two-libraries law: generated media is raw media, never a
  ColdLight ingredient). Ignore → stays in cache.
- **30-day flush** applies to the *cache only*. Saved drafts live in the
  library under `content/` and are permanently out of flush scope —
  automation never deletes from `/content`. The cache therefore lives in
  app-owned storage (`.userdata`), not in `content/`.
- **Consent prompt** on flush day: "today we are dumping — are you okay?"
  with three answers: *yes*, *yes, never show again*, *yes, remind me next
  time*. This is a governed interaction (Interaction Library pattern), and
  the never-show-again preference is persisted app state.
- **Open constraint to ground before ratifying the pass:** whether Grok
  Build's documented client surface exposes image/video generation at all.
  If generation requires direct xAI API calls, that collides with the
  subscription-only / no-API-key law and needs an explicit ADR decision
  first. Do not assume; read the provider surface in-session.

## 3. WiredFRONT Workflows

> "…under WiredFRONT: Workflows — tailored workflows that will allow us to
> generate properly and methodically say a custom component(s) that
> represents our storyboarding. Then the evolution to the next to finish
> the full show/episode."

Named, methodical, repeatable pipelines — the editor as the AI-driven
After Effects:

1. **Storyboarding workflow** — generate the custom component(s) that
   represent a storyboard, then evolve storyboard → full show/episode.
2. **How-to / explainer workflow.**
3. **LinkedIn PDF carousel workflow** — slides → PDF carousel for posting.
4. **Episode workflow** — stitching, ad insertion, post-processing;
   rendering stays ADR-009 subprocess work (ffmpeg / helpers), driven
   through dpl-comp / dpl-composer.

Workflow outputs feed the libraries like everything else ("presets are
never one-offs"): components land as reusable ingredients, media lands in
the raw-media library.

## 4. Sequencing (operator's stated order)

1. **First:** Grok mode button + draft cache + preview + save-to-library +
   30-day consent flush (after the generation-surface constraint in §2 is
   grounded).
2. **Then:** WiredFRONT Workflows, one workflow per pass, storyboarding
   first since it feeds the episode workflow.
3. Route-aware chat *actions* (library add / manage-and-control) can ride
   alongside either, reusing #60's import seam and the W10a typed-intent
   shape.

Each pass gets its own 7-section plan + §8 evidence before code, per house
law. This document is the program map, not a pass plan.
