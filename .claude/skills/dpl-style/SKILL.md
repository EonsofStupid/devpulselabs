---
name: dpl-style
description: The DevPULSE Labs visual-language gate. Invoke before ANY pass that adds or edits UI (hubs, shell, palette, editors) so surfaces land doctrine-clean — brutal/clay treatments, tooltip law, token-only values, any-shape proofs. Use when the operator asks for UI work, polish, styling, or when a pass touches anything the user sees.
---

# DevPULSE styling gate

Read `docs/design/doctrine.md` first. Then hold every touched surface to it:

1. **Classify each element**: actionable → BRUTAL (`cl::brutal_frame` /
   the shared chip painters); passive → CLAY (`cl::clay_frame`). Never
   hand-roll either treatment; extend ColdLight if a variant is missing.
2. **Tooltip law**: every interactive control gets `cl::tip(resp, "…")`
   in plain layman language. Grep your diff for `.clicked()` — each one
   near a control without a tip is a violation.
3. **Token-only**: no raw hex, no magic paddings. `cl::surface/ink/Tone/
   RADIUS*/space` only.
4. **Any-shape proof**: screenshot the touched route at BOTH maximized
   and `--window 1100x700` minimum. Wide/tall extremes when layout code
   changed.
5. **Feel bar**: before calling it done, look at the screenshot and ask
   "would the operator say this matches the first app's polish?" If not,
   it is not done — fix it before the PR.
6. Record doctrine deviations explicitly in the plan's §8; update
   `docs/STATE.md` rollout list for surfaces still un-swept.
