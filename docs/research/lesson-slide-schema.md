# Lessons, slides, components and variants: research

**Date:** 2026-10-20. **Asked by the operator (becomes R285):**

> we need to properly research and ensure that we are building the schema
> that we can then build out our lessons and mix and match individual slides
> but allow for variants so we can always refresh visuals and expand our
> ilbrary for components

**Method:** two read-only grounding passes and two web research passes, run
in parallel on 2026-10-20.
- The grounding passes covered this repo and the old app at
  `D:\localdev\review-archive\devpulse labs`.
- The web passes covered modular lesson and slide models, and component
  variant systems.

Key ground-truth claims were re-checked by hand in the same session. Sources
are numbered in §7. **[2nd]** marks a secondary source; **[unconf]** marks a
claim that could not be confirmed.

## 1. The question, made precise

Four abilities, each needing something the schema must hold:

| Ability | What the schema needs |
|---|---|
| **Build lessons** | a container that holds an ordered set of slide *uses*, with learning fields (objectives, segments, checks) |
| **Mix and match slides** | slides with their own stable identity in a library, referenced by lessons, never copied |
| **Variants, to refresh visuals** | a slide's content kept apart from its look, and a look chosen at the point of use |
| **Grow the component library** | components defined by a typed contract (slots, props, variant axes), added as data, versioned so old slides keep rendering |

## 2. What exists today (this repo, 2026-10-20)

**Slides have no identity of their own.**
- `dpl_doc::Slide` (`crates/doc/src/lib.rs:26-47`, `SCHEMA_V = 2`) holds:
  - `v`, `template`, `background?`;
  - `blocks` (`Text` or `Media`);
  - `enter?` and `reveals`.
- It has no id, title, version, category, tags, preset or provenance. A
  slide's identity is its folder name.

**Episodes contain their slides. They do not reference them.**
- `Episode { v, id, title, chapters }` lists slide subfolder names
  (`lib.rs:476-489`).
- Placing a slide copies its folder, renaming it `-2` on a clash
  (`mint_slide_name`, `lib.rs:1156`).
- A slide listed twice in one episode is an error (`lib.rs:610-614`).
- The only by-reference slide pointer anywhere is the storyboard's
  `SlotFill::Slide{rel}` (`crates/workflows/src/lib.rs:218-235`).

**Looks are baked into the slide.**
- `apply_look` (`crates/editor/src/lib.rs:680-692`) copies a preset's moods
  and font onto the slide, and the preset itself is forgotten.
- This contradicts the operator's own ratified ruling of 2026-07-31: a preset
  is a **reference** plus sparse overrides, and every entity has a preset
  exposing 4–7 traits, colour always one of them
  (`docs/plans/workbench-editor.md` §2.2, §2.4, §8.3).
- The ratified `SCHEMA_V` 2→3 bump was never built.

**The template field is inert.**
- The registry holds 28 code templates (20 slides plus 8 infographics,
  `crates/slides/src/registry.rs:68-99`), each with hard-coded demo props
  and no serde.
- The document renderer never reads `Slide.template`
  (`crates/slides/src/document.rs`). Every slide renders as one centred
  stack.
- `design-system.md` §4 (2026-08-09) answers R170 with a v3 `Node` tree
  (slots, matrix coordinates, motion). It is unbuilt, and it differs from the
  ratified 07-31 shape.

**There are no lessons, decks or show documents.**
- Workbench offers `ContainerKind::Lesson`, but says "Lessons aren't saved
  anywhere yet" (`crates/hubs/workbench/src/containers.rs:117-131`).
- A show is just a folder, and three show ids are hard-coded
  (`crates/hubs/shows/src/lib.rs:22-27`).

**There are no variants.**
- Today's "variants" are forked copies. The seed episode's `stat` slide is
  the seed `stat-callout` with one font changed.
- Elements: 6. Blocks: 0 (`blocks/census.rs`). Recipes: 0.

**There is no schema for slide-side documents.**
- `schemas/` covers workflows, storyboards, bibles, brainstorms and chat
  roles and rooms. Nothing covers `slide.json`, `episode.json`, `theme.json`
  or `comp.json`.
- The chat role schema (`dpl.role/1`) is the only item schema with
  `version`, `tags` and `provenance`. It is the model to follow.

**The operator's requirements already point here:**
- R43 and R69: "properly blocks, and templates", "a true diverse library".
- R44, R46, R47: generated styling colocated per slide folder.
- R60: presets are "not one offs".
- R68: a slide associates with an episode or presentation, with
  "flexibility to evolve".
- R96 and R97: "methodically upgrade their visuals… later easily edit".
- R141, R143, R144: 10–15 presets, 4–7 traits, "colour lives per entity",
  "everything is an ENTITY".
- R150: traits plus motion plus interactions are recipes.
- R166: mix and match presets on each entity.
- R170: "a schema that defines the slide clearly… it has a source of truth".
- No requirement yet defines a lesson container.

## 3. What the old app did

The old app was TanStack Start, React 19 and Tauri, with arktype schemas and
Style Dictionary tokens.

**Worth carrying forward:**
- **A slide is data:** `component` (a registry name) plus validated `props`.
  An unknown or invalid slide degrades to an error plate and the deck keeps
  running.
- **Each registry entry carries** `componentClass`, `propsSchema`,
  `description` and `formHints`. Editor forms are generated from the props
  schema.
- **A living style-guide episode** has one demo slide per component, and a
  missing demo shows a loud card.
- **The stacking algebra:**
  - additive axes concatenate, and on override axes the last trait wins;
  - per-state fallback chains;
  - a generated availability manifest, and generated types that fail the
    build on a look that doesn't exist.
- **Asset versions** use `_vN` plus `_SUPERSEDED`, and nothing is deleted.
- **A one-off becomes a catalog component** after two uses.

**What went wrong:**
- **Reuse was copying.** Slides were inline per episode, and nothing
  referenced a slide by id. Masters were copied into episodes. A
  `library: {token}` field was declared but never used.
- **The variants were three global knobs:** `density`, 18 `tone`s, and a
  stack of `fx-*` classes. There were no named visual variants of a
  component, and no way to swap a slide's look while keeping its identity.
- **Two styling systems ran side by side** (the fx matrix and the
  RenderForge presets).
- **Lists were hand-synced in three places**, and the same props shape was
  written twice.
- **Content was baked into code:** diagrams hard-coded their data.
- **`schemaVersion: 1|2` existed, but nothing migrated or branched on it.**

## 4. How mature systems do it

### 4.1 Reuse by reference

- **DITA** [1–3]:
  - The topic is the unit of reuse, and maps assemble topics.
  - Keys add late binding. The topic says `keyref="x"` and the map decides
    what `x` means; a parent map overrides its submaps.
  - Variants are profiling attributes (`@audience`, `@platform`), filtered
    at build time.
- **Open edX** [4, 5]:
  - Course → section → subsection → unit → components, with components in
    separate files "to better enable content reuse".
  - Libraries v2 link an upstream library block to downstream copies through
    `upstream_version`.
  - A sync replaces every field the author did not customise.
  - A declined update is remembered (`upstream_version_declined`) [6, 2nd].
- **cmi5** [7]: the Assignable Unit is "the unit of tracking". Every unit's id
  is a globally unique IRI, and an id is never reused for a different
  activity [8, 2nd].
- **H5P** [9, 10]:
  - Content types are libraries versioned major.minor.patch. Dependencies pin
    major.minor and float the patch, and patches must not change behaviour.
  - A slot names the sub-content types it accepts, each pinned to a version.

### 4.2 Content apart from presentation

- **PowerPoint (OOXML)** [11, 12]:
  - Master → layout → slide. Placeholders inherit by index and type.
  - Direct formatting on a slide overrides the layout, and re-applying the
    layout strips it. This is the "direct formatting rot" to avoid.
- **Google Slides API** [13, 14]: `createSlide` takes a layout plus
  placeholder mappings, and templating is copy plus `replaceAllText`.
- **Keynote** [15]: Change Theme moves your content into the new theme's
  slides. Optionally, your style changes are kept.
- **Slidev** [16, 17]: a layout per slide, with named slots
  (`::right::`). Layouts resolve built-in → theme → local, and the last one
  wins. **Reveal.js** [18]: a theme is a stylesheet swap.
- **AI slide tools** (marketing evidence only):
  - Beautiful.ai swaps layouts "while preserving your exact copy" [19].
  - Gamma restyles by theme without touching text or structure [20].
  - Tome left slides in 2024 [21, unconf].
- **The common pattern:**
  - Content lives in **named slots**, and presentation resolves **by name
    through a chain**.
  - Direct overrides are the escape hatch, and they are where decks rot.

### 4.3 Headless CMS modeling

- **Sanity** [22, 23]:
  - Documents stand alone and objects are embedded.
  - References name the types they may point to.
  - Array items get a persistent `_key`, which gives each use an identity.
  - Renaming a type orphans existing data.
- **Strapi** [24]: a *dynamic zone* is an ordered list drawn from an allowed
  set of components. That is "a slot that accepts one of N block types".
- **Contentful** [25–27]:
  - A link can be restricted to certain types.
  - Variants come from locale fallback chains.
  - Personalization lists alternative entries against a baseline.
- **Builder.io** [28]: a symbol floats (edit once, every use updates), while
  a template is a fork.
- **Plasmic** [29]: variants share one structure and inherit from the base,
  as groups or toggles, plus global variants for theme and screen size.

### 4.4 Variant systems for components

- **Figma** [30–32]:
  - Component properties come in five types: variant, boolean, text,
    instance swap, slot.
  - Use separate axes (Type, Size), never one combined "Style".
  - On/off is a boolean, not a variant.
  - An instance records `mainComponent + componentProperties + overrides`.
  - Overrides survive a swap only when layer names match. Position cannot be
    overridden without detaching.
  - Library updates are reviewed and accepted, so a refresh happens only
    when the author accepts it [33].
- **Storybook CSF** [34]: a story is a named set of `args` for a component.
  Stories are the examples, and the visual baselines.
- **CVA, Tailwind Variants and Panda recipes** [35–38]:
  - A component's variants are declared as `base`, `variants`,
    `defaultVariants` and `compoundVariants`, where an array value means
    "any of these".
  - Slot recipes style each named part.
  - Panda generates only the combinations in use.
- **Radix Themes and shadcn** [39, 40]: the global theme (accent, radius,
  scaling) is separate from each component's `variant` prop. Restyle by
  overriding semantic tokens, not component classes.
- **How they avoid combinatorial explosion:**
  - Variant axes are independent, and each has a default.
  - Compound rules are sparse exceptions, not a full matrix.
  - Booleans, text and swaps are props, not variants.
  - The theme is separate from variants.
  - Only the combinations in use are generated.

### 4.5 Tokens and themes

- **The W3C Design Tokens format 2025.10** [41, 42] is the first stable
  version (2025-10-28):
  - Typed tokens, plus composite types such as shadow, gradient, typography
    and transition.
  - `{group.token}` aliases.
  - `$deprecated`.
  - `$extensions`, which tools must preserve.
- **The Resolver Module 2025.10** [43]: token sets plus *modifiers* with
  contexts (theme: light/dark), merged in order with the last value winning.
- **Terrazzo** [44] implements resolvers, and its rule is orthogonality: no
  two modifiers write the same tokens.
- **Style Dictionary v5** [45] is adopting the 2025.10 value formats; its
  resolver support is unconfirmed.
- **ColdLight already follows the orthogonality rule:** each matrix facet
  owns disjoint axes ("ONE value per facet",
  `crates/coldlight/tokens/matrix.json`). It can be exported to DTCG later
  without changing its model.

### 4.6 Data-bound motion and video templates

- **Remotion** [46, 47]:
  - A composition declares a Zod `schema` plus `defaultProps`, and the studio
    builds controls from them.
  - `calculateMetadata` derives duration and size *from the data*.
- **Lottie slots** [48–50]:
  - The animation declares `slots`, and properties carry a `sid`.
  - Runtimes replace slot values for colour, scalar, vector, gradient, text
    and image.
  - **ThorVG**, the renderer behind this app's Lottie helper, exposes
    `LottieAnimation::gen(slot)`, `apply(id)` and `del(id)` (since 1.0),
    plus markers [50].
- **dotLottie 2.0** [51]:
  - Themes are rules mapped onto slots at runtime: "only properties with a
    slot ID can be targeted".
  - State machines can switch themes.
- **Rive** [52, 53]:
  - View models are typed (number, string, boolean, color, enum, list,
    nested).
  - "Once the property is bound, the hierarchy no longer matters", so
    designers restructure freely.
- **After Effects MOGRT** [54, unconf]: the Essential Graphics properties are
  the template's editable surface.
- **The common thread:**
  - Each tool declares a typed parameter contract, and visuals bind to
    parameter **names**, never to structure.
  - The data and the visual can be swapped independently.

### 4.7 Learning science for the lesson container

- **Mayer's multimedia principles** [55, 56], 15 with evidence:
  - coherence, signaling, redundancy and contiguity cut extraneous load;
  - segmenting, pre-training and modality manage essential load.
- **Gagné's nine events** [57, 58]: attention → objective → recall →
  present → guide → practise → feedback → assess → retention. Write the
  objectives first.
- **Merrill** [59, 2nd]: problem-centred, with activation, demonstration,
  application and integration.
- **Sizing:** across 6.9M edX sessions, median engagement topped out at
  about 6 minutes whatever the video's length. Segment content into chunks
  "ideally less than 6 minutes" [60].
- **Worth encoding as data:**
  - objective ids and statements;
  - prerequisites;
  - estimated minutes;
  - segment boundaries;
  - a role per slide use (its phase);
  - check items tied to objectives;
  - narration kept apart from on-screen text.
- **Better as guidance or lints:**
  - coherence;
  - redundancy (lintable once narration and on-screen text are separate);
  - signaling (a block capability);
  - voice.

### 4.8 Identity and versioning

- **Identity:**
  - Use stable ids, never paths (cmi5 IRIs, OLX `url_name`), with a separate
    identity per use (Sanity `_key`) [7, 23].
  - A content hash proves integrity, not identity: it changes with every
    edit [61].
- **Versions:** a released version is immutable [62].
  - H5P pins major.minor and floats the patch.
  - Open edX pins, syncs explicitly and remembers declined updates.
  - Builder.io symbols float, and its templates fork.
- **Schema evolution:**
  - Use an absolute `$id` and `$defs` [63].
  - `additionalProperties: false` closes an object, and
    `unevaluatedProperties: false` keeps `allOf` extension working [64].
  - Compatibility depends on whether the model is open or closed [65].
- **Rust:** adding a variant to an exhaustive public enum is a breaking
  change [66]. For data, the same rule reads: adding optional props or axis
  options is safe, and renaming or removing them breaks.
- **Visual baselines:** one per story per mode, keyed by **name**, so a
  rename starts a new baseline [67].

## 5. Synthesis: the model this research supports

Five document kinds, each a closed, versioned JSON Schema in the house style
(`schemas/<kind>.v<major>.schema.json`, plus serde types, plus `validate()`,
plus jsonschema tests, as `dpl.role/1` does):

| Layer | Holds | Never holds | Versioning |
|---|---|---|---|
| **Look** (the ColdLight theme presets, matrix coordinates and traits) | tokens only; the default look per entity kind | content | float within a major |
| **Component** (a contract per template, colocated with its code) | id, `class`, named **slots**, typed **props** with defaults, tooltips and limits; a few independent **variant axes** with defaults; sparse compound rules; motion beats; Lottie slot bindings; named **examples** | colours, positions | semver; each version must be a superset of the last (a gate) |
| **Entity preset** (ratified 2026-07-31) | the 4–7 traits an entity exposes, colour always one; `for` names the entity kind | content | semver; float within a major |
| **Slide** (`dpl.slide/3`: the ratified v3, plus identity) | stable id, `rev`, title, a component reference (`id@major`), the axis choices, content **per slot** (text, asset ids, data), narration apart from on-screen text, objective ids, a suggested role, category, tags, provenance (`forked_from`), `ext` | hex values, pixels, baked looks | `rev` increments on each content edit |
| **Container** (a lesson, episode or deck) | id, `rev`, title, objectives, prerequisites, estimated minutes, segments of **uses**, checks, a look | slide content | `rev` |

**A use** (one slide placed in a container) is
`{ key, slide: {id, rev}, variant?, overrides{slot → value}, role }`.
- Overrides are allowed only on traits the entity's preset `exposes`, keyed
  by slot name, never by position.
- The same slide can appear in many containers, or twice in one, each use
  with its own key.

**The rules:**
1. **Pin content, float presentation.**
   - A use pins a slide's `rev`. When the slide changes, the container shows
     "update available"; you accept or decline, and a decline is
     remembered.
   - Looks, presets and components float within their major version.
   - So **refreshing visuals is bumping a look, preset or component, and no
     lesson has to be touched**.
2. **Variants are chosen at the point of use, never copied into content.**
   - A real difference in content is a fork, which records `forked_from`.
   - A difference in look is an axis choice or a preset.
3. **Exports record a render lock:** the resolved component, preset and look
   versions, plus a token hash. An export then re-renders identically, and a
   refresh is a deliberate re-lock.
4. **The library grows by data.**
   - A new component is a contract file plus its paint function.
   - A new variant is an axis option.
   - A new look is a preset.
   - The gate proves every new contract version is a superset of the last,
     so evolution never breaks a slide.
5. **Capture now, because backfilling is painful:**
   - ids;
   - `rev`;
   - the component reference;
   - the slot-keyed content;
   - objective ids and role;
   - the narration / on-screen text split;
   - provenance;
   - per-use keys;
   - `ext`.

**Reconciling the two existing v3 designs:**
- The ratified 07-31 decisions stand: a preset is a reference with sparse
  overrides; presets are per entity, with `for` and `exposes`; `Block::Quote`;
  a reserved `recipe` field; `title`.
- The 08-09 `Node` tree becomes **what a component's slots render into**
  (the resolved, styleable tree the engine paints), not the authored format.
  Authors edit slot-keyed content, and the component expands it into nodes.

## 6. Anti-patterns, each seen in the wild or in this codebase

- Copy-pasted slides. This is today's `place_slide`, and the old app's
  inline slides. Use a reference, and fork only with provenance.
- Styling inside content: today's `apply_look` bake, and OOXML direct
  formatting.
- References without a version, or updates that land in a lesson silently.
- One variant per look (`callout-cyber`): looks belong in the look layer.
- A show/hide variant where a boolean prop belongs.
- Overrides keyed by position or structure instead of slot name.
- Renaming a type, slot, prop or axis option. Ids are permanent, and stored
  values are strings, never enum ordinals.
- Lists hand-synced in several places. One generated manifest is the source.
- Untyped template variables. Editing a Lottie file per use, instead of
  using its slots.
- Generating every combination of examples or baselines. Generate the named
  ones.

## 7. References

1. DITA 1.3 conref — https://docs.oasis-open.org/dita/dita/v1.3/os/part1-base/archSpec/base/conref.html
2. DITA 1.3 keys — https://dita-lang.org/1.3/dita/archspec/base/keys-core-concepts
3. DITA 1.3 conditional processing — https://dita-lang.org/1.3/dita/archspec/base/condproc
4. Open edX OLX course structure — https://docs.openedx.org/en/release-teak/educators/olx/organizing-course/course-xml-file.html
5. Open edX ADR 0020 upstream/downstream — https://github.com/openedx/edx-platform/blob/master/docs/decisions/0020-upstream-downstream.rst
6. Open edX upstream_sync (mirror) — https://git.andalsoftware.com/Andal.LND/edx-platform/src/commit/276df19d48c388721c82b5cb4e4c1abd7f1f2d5d/cms/lib/xblock/upstream_sync.py
7. cmi5 specification (Quartz) — https://github.com/AICC/CMI-5_Spec_Current/blob/quartz/cmi5_spec.md
8. xAPI activity ids — https://xapi.com/deep-dive-activity/
9. H5P library definition — https://h5p.org/library-definition
10. H5P semantics — https://h5p.org/semantics
11. python-pptx, placeholders — https://python-pptx.readthedocs.io/en/latest/user/placeholders-understanding.html
12. Open XML slide masters — https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-slide-masters
13. Google Slides CreateSlideRequest — https://googleapis.dev/ruby/google-apis-slides_v1/v0.19.0/Google/Apis/SlidesV1/CreateSlideRequest.html
14. Google Slides merge guide — https://developers.google.com/workspace/slides/api/guides/merge
15. Keynote, change theme — https://support.apple.com/en-lamr/119885
16. Slidev layouts — https://sli.dev/guide/layout
17. Slidev built-in layouts — https://sli.dev/builtin/layouts
18. Reveal.js themes — https://revealjs.com/themes/
19. Beautiful.ai — https://www.beautiful.ai/
20. Gamma layout guide — https://gamma.app/nl/explore/content/guides/gamma-ai-presentation-tool-flexible-layout-customization-guide
21. Tome (single source) — https://yespress.io/products/tome.md
22. Sanity reference type — https://www.sanity.io/docs/studio/reference-type
23. Sanity array type — https://www.sanity.io/docs/studio/array-type
24. Strapi content-type builder — https://docs.strapi.io/cms/features/content-type-builder
25. Contentful content types — https://contentful.com/developers/docs/references/content-management-api/content-types
26. Contentful localization — https://www.contentful.com/help/localization/field-and-entry-localization/
27. Contentful personalization — https://contentful.com/developers/docs/personalization/architecture
28. Builder.io symbols — https://www.builder.io/c/docs/symbols
29. Plasmic variants — https://docs.plasmic.app/learn/variants/
30. Figma component properties — https://help.figma.com/hc/en-us/articles/5579474826519-Explore-component-properties
31. Figma variants — https://help.figma.com/hc/en-us/articles/360056440594-Create-and-use-variants
32. Figma instance overrides — https://help.figma.com/hc/en-us/articles/360039150733-Apply-overrides-to-instances
33. Figma library updates — https://help.figma.com/hc/en-us/articles/360039234193-Review-and-accept-library-updates
34. Storybook CSF — https://storybook.js.org/docs/api/csf
35. CVA variants — https://cva.style/docs/getting-started/variants
36. Tailwind Variants slots — https://www.tailwind-variants.org/docs/slots
37. Panda CSS recipes — https://panda-css.com/docs/concepts/recipes
38. Panda CSS slot recipes — https://panda-css.com/docs/concepts/slot-recipes
39. Radix Themes — https://www.radix-ui.com/themes/docs/theme/overview
40. shadcn/ui theming — https://ui.shadcn.com/docs/theming
41. DTCG stable version announcement — https://www.w3.org/community/design-tokens/2025/10/28/design-tokens-specification-reaches-first-stable-version/
42. DTCG format 2025.10 — https://www.designtokens.org/tr/2025.10/format/
43. DTCG resolver module 2025.10 — https://www.w3.org/community/reports/design-tokens/CG-FINAL-resolver-20251028/
44. Terrazzo resolvers — https://terrazzo.app/docs/guides/resolvers/
45. Style Dictionary releases — https://github.com/style-dictionary/style-dictionary/releases
46. Remotion schemas — https://www.remotion.dev/docs/schemas
47. Remotion calculateMetadata — https://www.remotion.dev/docs/calculate-metadata
48. Lottie spec, composition (slots) — https://lottie.github.io/lottie-spec/latest/specs/composition/
49. LottieFiles slots — https://docs.lottiefiles.com/en/runtimes/distributions/js/v0.x/core-concepts/slots
50. ThorVG Lottie API — https://github.com/thorvg/thorvg/blob/main/src/loaders/lottie/thorvg_lottie.h
51. dotLottie 2.0 — https://dotlottie.io/spec/2.0/
52. Rive data binding — https://rive.app/docs/editor/data-binding/overview
53. Rive view models — https://rive.app/docs/editor/data-binding/view-models
54. After Effects master properties — https://www.adobe.com/se/learn/after-effects/web/intro-master-properties
55. Mayer 2024, Educational Psychology Review — https://doi.org/10.1007/s10648-023-09842-1
56. Mayer's principles, definitions (2nd) — https://services.dartmouth.edu/TDClient/1806/Portal/KB/PrintArticle?ID=171655
57. Gagné's nine events (USU) — https://www.usu.edu/teach/help-topics/teaching-tips/gagnes-9-events-of-instruction.php
58. Gagné's nine events (NIU) — https://www.niu.edu/citl/resources/guides/instructional-guide/gagnes-nine-events-of-instruction.shtml
59. Merrill's first principles, synthesis (2nd) — https://www.yorku.ca/unsdgs/toolkit/wp-content/uploads/sites/617/2023/04/Merrills-First_Principles_of_Instruction_A_synthesis.pdf
60. Guo, Kim, Rubin 2014, edX video engagement — https://pg.ucsd.edu/publications/edX-MOOC-video-production-and-engagement_LAS-2014.txt
61. Git objects — https://git-scm.com/book/en/v2/Git-Internals-Git-Objects
62. Semantic Versioning 2.0.0 — https://semver.org/
63. JSON Schema, structuring — https://json-schema.org/understanding-json-schema/structuring
64. JSON Schema, object — https://json-schema.org/understanding-json-schema/reference/object
65. Confluent schema evolution — https://docs.confluent.io/platform/current/schema-registry/fundamentals/schema-evolution.html
66. Cargo SemVer compatibility — https://doc.rust-lang.org/cargo/reference/semver.html
67. Chromatic modes — https://www.chromatic.com/docs/modes/
