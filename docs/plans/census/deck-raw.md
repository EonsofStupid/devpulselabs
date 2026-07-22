# Census raw — DECK / PRESENT / SLIDES RUNTIME (old repo) — 63 capabilities

## Episode loading & manifests (overlaps content census 1–14)
1–12. Manifest schema, per-slide contract, structural integrity, doctrine rules, fs scan, load-one, native seam, content-status diagnostics, asset resolver seam, manifest→deck adapter, broken-slide escape hatch, chapter markers derivation — all solid.

## Registry (overlaps workbench census 35–40)
13. Self-registering registry ~18-19 components
14. Component-class taxonomy static/split/animation/interactive (budget counts interactive)
15. Schema props + formHints — partial

## Sequencing / reveal steps
16. Beat-grammar step math: advance/retreat over {index, step}; stepCount=reveals+1; step0=setup; roll to next slide step0; retreat lands prior slide LAST step; holds at ends
17. Clamp helpers + canAdvance/canRetreat for button states
18. URL as source of truth /deck/$id/$slide/$step; loader caches episode (30s); clamps + redirect-replaces to canonical
19. Legacy 2-segment URL redirect to step0 (bookmarks survive)
20. Controlled reveal consumption: ctx.stepIndex/stepCount; RevealList deck-driven vs self-paced
21. Slide-key remount discipline: key on slide.id only (steps re-render, never remount; ink persists)

## Audience runtime host (DeckHost)
22. Full-viewport host: slide + progress rail + edge nav + toolbar + navigator + annotation; data-brand wrapper
23. Keyboard nav: ←/→/PgUp/PgDn/Space beat; Space bails on focused controls; ↑/↓ whole-slide; g navigator, a annotate, h chrome; bails in inputs
24. Swipe nav: 60px horizontal, rejects vertical-dominant, [data-no-swipe] opt-out
25. Edge tap zones: 12vw full-height invisible prev/next + chevrons; disabled at ends; hidden when chrome hidden/annotating
26. Runtime chrome store: chromeHidden/annotateOn/navigatorOpen
27. Per-slide chrome override chrome:"hidden"
28. Progress rail: top fill bar + chapter tick marks; fades w/ chrome
29. Runtime toolbar: Home, navigator, presenter opener, annotate, chrome toggle, title + NN/NN·step
30. Slide navigator overlay: modal grid of slide cards (text cards, not thumbnails — partial)

## Touchscreen escape hatches
31. Home dot bottom-left when chrome hidden
32. Bottom-edge 7% reveal strip taps toolbar back
33. Per-slide allowAnnotate:false opt-out

## Ink annotation
34. Pointer ink canvas: DPR-aware, pressure width 3+p*4, --ink-annotate token color, captures only while enabled
35. Per-slide auto-clear + manual Clear (ephemeral BY DESIGN; no persistence/colors — v1 scope)

## Presenter + sync
36. PresenterHost teleprompter: scaled live preview + timer + transport + beat list + script
37. Beat jump-list ((setup) + each reveal) jumps to step
38. Script teleprompter w/ active-beat highlight, cue+note+body
39. Inline emphasis renderer **bold**/*i*/`code` (never HTML exec)
40. "Up next" panel
41. Elapsed timer mm:ss
42. Presenter-window reuse (named window, refocus not reload — preserves timer)
43. BroadcastChannel sync per deck (deck:<id>) {slideIndex, stepIndex}; loop-safe; last-writer-wins; replace-navigate
44. /present/* loader byte-identical to deck + scripts

## MDX-lite scripts
45. Parser: YAML fence + inert body; null-with-warning degrade
46. Frontmatter schema: slideId/title/beats; beats==stepCount parity doctor warning
47. Loading both platforms keyed by slideId

## Fluid primitives
48. ScaledStage: 1920×1080 logical stage transform-scaled (ResizeObserver); vw/svh caveat
49. FitScale: scale DOWN to fit, never up; jitter-guarded
50. HoloScroll: holographic arrows, rAF continuous scroll, page nudge, overflow-only, hidden scrollbar
51. FluidFrame: container-type:inline-size + clamp(100cqi); publishes --frame-w/h
52. FluidGrid/FluidStack container-driven
53. Fluid XY math core (FLUID_X vw / FLUID_Y svh) + runtime hooks same math as build
54. SlideFrame: safe-area padding, align modes, bleed option

## Fullbleed (shell)
55. \ toggles fullBleed, Esc exits, FullbleedExit corner affordance + aria-keyshortcuts
56. Shell shortcuts: [/] rails, Ctrl+Shift+2 nav dock, Esc cascade; skip editable targets

## Slide fx application
57. fx wrapper: real box applying fx-* classes + fx-canvas painter; unknown fx harmless no-op
58. data-fx-tone re-tones subtree; tone via ctx.look
59. fx vocabulary: 18 tones; fx-card/halo/glass-strong/sheen/scanlines/noise/bg-hero/aurora/duotone/spot/conic
60. fx membership doctor warning (shape-valid at load)
61. density comfy/compact via ctx.look.density

## Input affordances
62. usePinchZoom: pinch 1–6×, pan when zoomed, double-tap reset; still-zoomable template
63. useReducedMotion + useDragToken support hooks

Gaps flagged: ink ephemeral by design; navigator text cards not thumbnails; sync last-writer-wins only; presenter preview proportional not pixel-identical.
