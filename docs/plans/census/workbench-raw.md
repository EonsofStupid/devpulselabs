# Census raw — WORKBENCH / STUDIO (old repo) — 69 capabilities

## WorkBench shell & routing
1. Tabbed nav (Dashboard·Studio·Library·Tokens·RROFlow) + rail flyout mirror — solid. _shell.workbench.tsx
2. Dashboard = creator wizard + structure doctor, parallel load — solid. _shell.workbench.index.tsx

## Creator wizard
3. CreatorForm: pick show/number/title → createEpisodeNative writes full standardized folder — solid
4. Auto next-episode-number per show — solid
5. Live target-path preview, slugified folder id — solid
6. On-create: invalidate router, navigate into author stepper — solid

## Structure doctor
7. DoctorReport: per-episode pass/warn/fail chip + every doctrine issue — solid
8. runDoctorNative: schema, required files, registered components, prop validation, asset existence, chapter lock, interaction budget — solid

## Author stepper
9. 6-stage golden path Brief→Storyboard→Script→Slides→Doctor→Rehearse — solid
10. Live doctor verdict chip in header, re-run after every save — solid
11. beforeunload dirty guard — solid
12. Author route resolves episodeId→folder, guards missing — solid
13. MarkdownStage: brief/storyboard/script editor, explicit save, per-stage guidance — solid
14. SlidesStage: JSON-free slide editor, add/remove/move/duplicate — solid
15. Component picker from live registry grouped by class; per-slide chapter/beat/title/props — solid
16. Schema-validate before save; raw-JSON fallback textarea — solid
17. Budget/band readout (N slides · M interactive (max 3) · band 15–24) — solid
18. Rehearse: deep-link into deck at slide 0 — partial

## Component gallery
19. One live card per registered component on ScaledStage w/ fixtures — solid
20. Per-card error boundary — solid
21. Loud degrade states (missing demo/asset/fixture invalid) — solid
22. Registry-driven sort + count header — solid
23. Replay card live from newest capture — solid
24. Desktop asset-protocol resolution — solid
25. gallery-demos canonical fixtures (3 marked missing-asset: video, video-loop, replay) — solid
26. types/readme — solid

## Token explorer
27. Primary ramp + brand aliases row — solid
28. Pool grid every hue × steps — solid
29. Gradients grid per-hue, filterable, click-to-copy — solid
30. FX tone matrix — 18 tones live via data-fx-tone — solid
31. FX ingredients — 24 composable classes copyable + tone preview — solid
32. CopyChip clipboard flash — solid
33. Tokens route — solid

## RROFlow
34. Route + coming-soon boundary panel — STUB

## Registry service
35. register/get/list/componentClassOf; dup warning; unknown never crash — solid
36. Contracts: SlideComponentRegistration (arktype propsSchema, render, formHints), RegistrySlideContext.assetUrl, SlideLook, PropsFormHint — solid
37. Side-effect enumerator of 19 registrations + custom barrel — solid
38. 19 registered components: title, still, split, stat, video, video-loop, reveal, choice, bullets, callout, code, stats, hotspots, diagram, quiz, drag-match, predict, cost-meter, replay — solid
39. formHints coverage — partial (2 of 19: predict, cost-meter)
40. Custom per-episode component home — STUB (convention documented, empty)

## Authoring Studio P1/P2
41. Three-region shell (Palette · Canvas+Inspector · Content Library), desktop gate — solid
42. Show/episode pickers + Save episode.json + dirty/busy/count status — solid
43. Studio route: content root + artifacts + listings; live-reload on capture events — solid
44. useEpisodeDraft: load/validate episode.json both directions — solid
45. Draft mutations: addSlide (fixture-seeded, unique id), mutate/move/duplicate/remove (refuses empty) — solid
46. PalettePanel: registry cards preview themselves w/ fixtures, class chip — solid
47. Placement: pointer-drag (useDragToken screen-space hit-test) OR tap-append (touch/keyboard) — solid
48. ComposeCanvas: live preview IS drop target, drop highlight, N/M header — solid
49. Slide strip + move/duplicate/remove; unregistered-component message — solid
50. SlideInspector: title/component/chapter/beat/tone + PropsForm; reseed props on component switch — solid
51. PropsForm: arktype-driven; string/textarea/number/boolean/enum/string-list/object-list/json; JSON fallback — solid
52. describeProps: toJsonSchema → FieldDesc, formHints honored, degrades to JSON — solid
53. SlidePreview: validates w/ component schema, 1920×1080 ScaledStage, boundary — solid
54. studioContext: fully-revealed steps, per-platform assetUrl — solid
55. studio-slide service: humanize/kebabify/uniqueKebabId/seedProps/newSlide/episodeAssetBase — solid
56. GuideReviewer: do-frame + click-highlight ring + →done frame + app/title context — solid
57. GuideReviewer edit: reorder/delete/caption, updateGuideNative, ≥1 guard — solid
58. Frame-to-slide import: inbox capture → episode generated assets → still slide — solid

## Content Library panel
59. Unified library component, grid + sticky detail pane, mobile scroll — solid
60. Rescan (reindexArtifacts) both-direction reconcile — solid
61. FilterBar: kind chips w/ counts, location chips, free-text over name/tags/aiLabels — solid
62. LibraryCard: thumb + per-kind icon fallback + duration badge + tag/AI chips — solid
63. LibraryItem union model, unknown kinds still list, pure filter helpers, LibraryActions — solid

## RenderForge
64. renderForge resolver (kind,preset,state,families)→data-attrs+var chains — solid
65. renderRecipe compositor: base+traits, additive concat, override last-wins — solid
66. duoGradient runtime two-hue gradient over pool — solid
67. cssVarString — solid
68. Public surface/types + RENDERFORGE.md — solid
69. base/keyframes CSS + unit tests — solid

Stubs: RROFlow, custom registry barrel. Partials: Rehearse hand-off, formHints (2/19), 3 gallery fixtures missing assets.
Spine: registration + fixture → appears in Library, palette, inspector automatically.
