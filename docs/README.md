# The doc map

Every doc in `docs/`, and what it is the truth for. **One subject has one
governing doc.** Where a subject still has several, the governing one is
named first and the others are marked **P2**: they merge into it during
`docs-one-truth` P2.

- **Read first:** [`status.md`](status.md) (where we are) →
  [`requirements.md`](requirements.md) (the operator's words, verbatim; nothing
  else is built) → this map.
- **Deleted docs** (finished plans, old journals, superseded and stale docs)
  are readable at the tag `docs-history-2026-10-12`:
  `git show docs-history-2026-10-12:<path>`. A citation written as
  `docs-history-2026-10-12:<path>` points there.
- **The prior app** is cited as `old-app:<path>`: a file under
  `D:\localdev\review-archive\devpulse labs`. It is read-only, never built
  on.
- **The rules** (CLAUDE.md, "One truth"):
  - an operator quote lives only in `requirements.md`;
  - a replaced doc is deleted, never marked superseded;
  - a pass plan is deleted when it lands.

## Where we are and what is wanted

| Doc | Truth for |
|---|---|
| [`status.md`](status.md) | Now · known broken · next. Rewritten each pass. |
| [`requirements.md`](requirements.md) | Every operator requirement, verbatim, dated, with an R-id. |
| [`plans/product-vision.md`](plans/product-vision.md) | The product: the houses and how they compose. |

## Law (gate-enforced or ratified)

| Doc | Truth for |
|---|---|
| [`design/architecture.md`](design/architecture.md) | Layer law, dependency ceilings, named exceptions. Enforced by `cargo green` (structure). |
| [`design/signal-doctrine.md`](design/signal-doctrine.md) | Signals: one fabric, typed facts. |
| [`design/workflow-doctrine.md`](design/workflow-doctrine.md) | Workflows are documents, not code paths. |
| [`shell.md`](shell.md) | The shell zone contract. |
| [`ops/definition-of-done.md`](ops/definition-of-done.md) | What "done" means. |

## The library (content, rings, embedded / imported / linked)

| Doc | Truth for |
|---|---|
| [`design/adr-014-three-ring-root.md`](design/adr-014-three-ring-root.md) | The three rings (`content`, `assets`, `.userdata`). Governs until P3 writes `design/library.md`. |
| [`plans/content-provenance.md`](plans/content-provenance.md) | Embedded / imported / linked, one universe (never executed). **P3** source. |
| [`plans/l1-library-doctrine.md`](plans/l1-library-doctrine.md) | The library's information architecture. **P3** source. |
| [`plans/guided-landing-charter.md`](plans/guided-landing-charter.md) | Where captures land, guided. **P3** source. |
| [`design/estate-and-agent-seam.md`](design/estate-and-agent-seam.md) | The estate store and the agent seam. |

## Design system (ColdLight)

| Doc | Truth for |
|---|---|
| [`design/design-system.md`](design/design-system.md) | **The** design system: tokens, roles, naming law. |
| [`design/coldlight-matrix.md`](design/coldlight-matrix.md) | The look layer (matrix). **P2** → design-system. |
| [`design/doctrine.md`](design/doctrine.md) | Paint laws (tooltip law, no blur); its trait parts are history. **P2** → design-system. |
| [`design/visual-language.md`](design/visual-language.md) | Only its RULED/OPEN conflict register is live. **P2** → design-system. |
| [`design/adr-015-svg-icon-engine.md`](design/adr-015-svg-icon-engine.md) | The icon engine. |
| [`design/adr-016-icon-language.md`](design/adr-016-icon-language.md) | §4 is live. **P2** → adr-015. |
| [`design/matrix-proof-deck.md`](design/matrix-proof-deck.md) | The matrix proof deck (not yet built). **P2**. |
| [`plans/effects-motion-axes.md`](plans/effects-motion-axes.md) | Effects × motion axes (not yet built). |
| [`plans/keyframe-engine.md`](plans/keyframe-engine.md) | The keyframe engine (partly built). |
| [`plans/poam-preset-system.md`](plans/poam-preset-system.md) | The preset system plan (partial). **P2**. |

## WiredFRONT (the raw editor)

| Doc | Truth for |
|---|---|
| [`plans/wiredfront-experience.md`](plans/wiredfront-experience.md) | **The** editor experience contract (UX gate), with `design/mocks/wiredfront-editor-mock.html`. |
| [`plans/wiredfront-workflows-program.md`](plans/wiredfront-workflows-program.md) | The WiredFRONT workflows map. |
| [`plans/wiredfront-editor-dress.md`](plans/wiredfront-editor-dress.md) | Editor wears the mock (not yet built). **P2**. |
| [`plans/wiredfront-editor-residency.md`](plans/wiredfront-editor-residency.md) | Icons, residency, linking (not yet built). **P2**. |
| [`plans/wiredfront-library-and-player.md`](plans/wiredfront-library-and-player.md) | The library popup player (part B moved to media-experience). **P2**. |
| [`plans/r1-editor-adr-proposal.md`](plans/r1-editor-adr-proposal.md) | The editor ADR (partly ratified). **P2**. |
| [`plans/media-experience-program.md`](plans/media-experience-program.md) | Browse, players, GIMP (not yet built). |
| [`runbook/adobe-exit.md`](runbook/adobe-exit.md) | Leaving Adobe. |

## Workbench (the build house)

| Doc | Truth for |
|---|---|
| [`plans/workbench-editor.md`](plans/workbench-editor.md) | **The** Workbench editor (in flight). |
| [`plans/authoring-charter.md`](plans/authoring-charter.md) | The authoring thesis. **P2** → workbench. |
| [`plans/experience-inventory.md`](plans/experience-inventory.md) | The UI-pass gate. |
| [`plans/control-shelf-button.md`](plans/control-shelf-button.md) | Control shelf (not yet built). |
| [`plans/entity-inspector-scaffold.md`](plans/entity-inspector-scaffold.md) | Entity inspector (not yet built). |
| [`design/slide-type-checklist.md`](design/slide-type-checklist.md) | Slide-type checklist (draft). **P2**. |

## Chat and AI

| Doc | Truth for |
|---|---|
| [`plans/chat-native-program.md`](plans/chat-native-program.md) | **The** chat + AI program (paused behind D0). |
| [`plans/chat-provider-runtime-adr.md`](plans/chat-provider-runtime-adr.md) | The provider runtime exception to the no-IPC law. |
| [`design/adr-011-imagine-generation.md`](design/adr-011-imagine-generation.md) | Imagine generation. |
| [`design/adr-017-provider-capability-registry.md`](design/adr-017-provider-capability-registry.md) | The provider capability registry (§2 superseded). |
| [`plans/explainer-guided-workflow.md`](plans/explainer-guided-workflow.md) | The guided explainer (M2/M3 open). |
| [`research/chat-provider-capabilities.md`](research/chat-provider-capabilities.md) | Provider capability research (cited by 4 crates). |
| [`research/chat-rooms-and-roles.md`](research/chat-rooms-and-roles.md) | Rooms and roles research (R284): what makes a group of AIs better, the role and room schemas, how each AI joins (cited by the built-in library and its tests). |

## Coach

| Doc | Truth for |
|---|---|
| [`plans/coach-seam-adr.md`](plans/coach-seam-adr.md) | **The** coach seam ADR (ratified 2026-08-08). |
| [`plans/coach-overlay-adr.md`](plans/coach-overlay-adr.md) | The overlay process exception. |
| [`plans/coach-operator-verb-catalog-adr.md`](plans/coach-operator-verb-catalog-adr.md) | The verb catalog. |
| [`plans/operator-coach-a2ui.md`](plans/operator-coach-a2ui.md) | The C-series map (out-of-app half moved to the seam ADR). **P2**. |
| [`research/coach/README.md`](research/coach/README.md) · [`seam-real-world-challenge.md`](research/coach/seam-real-world-challenge.md) | Coach research. |
| [`proof/coach-verb-battery/`](proof/coach-verb-battery/) | Verb-battery reports (written by `coach-daemon/examples/verb_battery.rs`). |

## Capture (ViziWiz, sessions, guides)

| Doc | Truth for |
|---|---|
| [`design/viziwiz-architecture-spec.md`](design/viziwiz-architecture-spec.md) | **The** ViziWiz spec: three products. |
| [`design/wiredfront-capture-share-spec.md`](design/wiredfront-capture-share-spec.md) | The operator's own capture-and-share spec, filed verbatim. Its platform parts are marked "do not implement" (external reach row 6). |
| [`plans/s-adr-sessions.md`](plans/s-adr-sessions.md) | Session capture ADR. |
| [`plans/port-viziwiz.md`](plans/port-viziwiz.md) | ViziWiz port program (draft). **P2**. |
| [`plans/demo-guide-pipeline.md`](plans/demo-guide-pipeline.md) · [`plans/demos/demo-001-twilio-notification-keys.md`](plans/demos/demo-001-twilio-notification-keys.md) | The demo-guide pipeline (dormant). |
| [`design/adr-013-browser-driver.md`](design/adr-013-browser-driver.md) | The headless browser driver. |

## Release, depot, CI

| Doc | Truth for |
|---|---|
| [`plans/d0-laptop-deps.md`](plans/d0-laptop-deps.md) | **D0**: dependencies proven on this laptop (in flight). |
| [`design/release-contract.md`](design/release-contract.md) | The portable release contract. **P2** → release. |
| [`plans/private-tool-depot.md`](plans/private-tool-depot.md) | The tool depot. **P2** → release. |
| [`plans/external-reach-inventory.md`](plans/external-reach-inventory.md) | Everything outside this repo, for operator confirmation. |
| [`plans/mac-parity.md`](plans/mac-parity.md) | macOS parity (dormant; named by `green.yml`). |
| [`runbook/ci-runners.md`](runbook/ci-runners.md) | The estate CI runners and their security rule. |
| [`runbook/sccache-shared-build-cache.md`](runbook/sccache-shared-build-cache.md) | The shared build cache. |
| [`plans/crate-consolidation-review.md`](plans/crate-consolidation-review.md) | The crate census review. |
| [`plans/rendering-stack.md`](plans/rendering-stack.md) | The rendering and widget stack (its "ADR-011" number collides; **P2** renumbers). |

## EguiPilotAI (the harness)

| Doc | Truth for |
|---|---|
| [`plans/eguipilotai.md`](plans/eguipilotai.md) | **The** harness architecture. |
| [`plans/eguipilotai-v1-delivery.md`](plans/eguipilotai-v1-delivery.md) | The v1 delivery order. |
| [`eguipilotai-devpulse-authoring.md`](eguipilotai-devpulse-authoring.md) | Authoring scenarios for DevPULSE. |
| [`eguipilotai-playwright-review.md`](eguipilotai-playwright-review.md) | The capability matrix. **`cargo green` reads this exact path** (`xtask docs_status`). |
| [`research/eguipilotai/`](research/eguipilotai/README.md) | The harness research and contracts. |

## Ops

| Path | What |
|---|---|
| [`journal/`](journal/) | The newest milestone handoff only (the session hooks read it). |
| [`ops/references/signals-schema.md`](ops/references/signals-schema.md) | The ops signal vocabulary. |
| [`plans/docs-one-truth.md`](plans/docs-one-truth.md) | This reorganization (in flight). |
