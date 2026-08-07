# Journal — 2026-08-07: coach seam audit, the true mock, the challenge

## Why this session happened

Operator stopped the lane: "get a true mock made … it properly extends
eguiPilot … stop handwaving me." The audit that followed proved the
complaint correct.

## The audit (ground-truthed, three parallel investigations)

1. **The CX daemon is a second agent surface.** `egui-pilot-ai-protocol`
   already owns the control vocabulary (`Action`, `CoachCommand`/`CoachEvent`,
   framed nonce-authenticated stdio, `coach.desktop-overlay` still honestly
   unsupported), and C2s §3.4 mandates foreign perception emit the SAME
   shape "with no new agent surface". The Axum daemon (own verb enum,
   HTTP/SSE) violates that. The ~2,500-line `coach-runtime` engine is
   transport-agnostic and survives; the ~800-line HTTP wrapper gets demoted
   to a devtools adapter and the vocabulary rehomed. Days, not weeks.
2. **Missing foundations the operator explicitly asked for:** no tracing
   (battery proved verbs, captured zero reliability stats), no per-app
   profiles (sessions produced no reusable pack), no mock-first.
3. **Research (2026 SOTA):** UFO² validates the thesis — declarative
   per-app adapters over a generic a11y fabric, native-API-first when a
   pack declares one, beat every screenshot agent. AT-SPI2 stays the
   semantic layer (D-Bus, works on Wayland); input injection must be a
   trait with x11/XTEST now and portal+libei later. OTel GenAI
   `execute_tool` spans → DuckDB is the tracing model. Nobody has
   published a teaching-oriented profile schema — open ground.

## Shipped this session (uncommitted, awaiting red-pen)

**`docs/design/mocks/coach-seam-mock.html`** — the true mock, house
convention (single-file, staged JS flow, honesty lede, red-pen queue).
Six stages: Pick (tier ladder) → Seam build → Profile card (the config
series: Calculator/Cursor/Blender/VS Code/SolidWorks/Claude/CapCut) →
Verb console (the ratified 101 + effect contracts) → Lesson
(Coach/Teacher agency toggle) → Trace ledger (spans → DuckDB).
Verified live in the browser, all stages interactive.

## Rulings (operator, 2026-08-07)

- **Mock first, review before the seam ADR is ratified** (three-pass plan
  approved in mock-only mode).
- **Blender is the first Tier B pack** (socket addon, locale-stable
  `bpy.ops`, cheapest build-once proof).

## The challenge (real-world success gates, delivered on request)

1. Name-anchored fabric breaks on locale/theme/version — anchors must be
   fingerprint bundles + on-attach revalidation with honest degradation.
2. A tree walk is not a fabric — active exploration (menus/dialogs) needs
   a sandbox law + checkpointed, resumable, hours-long builds.
3. A pack is a claim until an automated verification battery proves every
   anchor + verb with oracles — packs ship only on battery green.
4. Distribution is a security product — data-only packs where possible;
   signed adapters (Blender addon, VS Code bridge) where not; games need a
   coach-marks-only zero-injection mode (anti-cheat).
5. The matrix (app × version × OS × locale) is the cost — the Foundry
   pipeline itself is the asset; few cells, verified hard, honest degrade.

## Honest ledger (operator asked directly)

Not a waste: the runtime engine, the live proofs (C2s, overlay, mint,
verb battery 25 PASS), and the ratified verb vocabulary are durable and
independently validated by the research. The waste: the HTTP wrapper
rework, and weeks of reliability stats + pack data that were never
captured because tracing/profiles were skipped. Correction is scoped.

## Next (pending operator red-pen of the mock)

1. Operator red-pens `coach-seam-mock.html` (5 queued questions in the
   mock footer) → rev, including Foundry job console for stage 2.
2. Seam ADR: rehome vocabulary into `egui-pilot-ai-protocol`, AppProfile
   schema, transport ladder per verb, tracing model.
3. Implementation slices: tracing spine → profile registry → Blender pack.
