# Authoring shell — the signal-first build plan (charter pass 1)

**Status: PLAN — awaiting operator go.** The first build pass of the
[authoring charter](authoring-charter.md): the real egui authoring surface, wired
to the signal fabric from the first commit, delivered as an **end-to-end walkable
shell** so the operator can red-pen the whole flow on the real exe. Signal
adherence (`../design/signal-doctrine.md`, LOCKED) is the spine, not a finish.
Reference UX: [`../design/mocks/authoring-surface-mock.html`](../design/mocks/authoring-surface-mock.html).

Gate: `cargo green` per sub-pass; every mutation publishes per the signal
doctrine; surfaces paint ColdLight only; `dpl-doc`/`dpl-editor` stay egui-free of
the fabric where the layer law requires.

---

## 1. Context

The charter is ratified in shape; the operator wants the *real* UI to red-pen
end to end, and named the non-negotiable: **it must ride the signal system**,
because the fabric is what lets AI create interaction overlays effortlessly. This
plan builds the surface signal-first and reuses the two overlay mechanisms that
already exist, rather than inventing a modal system.

## 2. Current state (ground-truthed this session)

- **Fabric** — `dpl-fabric`, one file `crates/fabric/src/lib.rs`. Per-domain
  enums, `publish_X`/`subscribe_X` on a `#[derive(Clone)]` `Fabric` (one
  instance, many holders, built at `crates/app/src/main.rs:545`). Stamp-at-publish
  wraps every fact as `Stamped<T>{seq,ms,ev}` (`:415`, `stamp()` `:496`), one
  global `AtomicU64` seq; the **waker** fires `request_repaint` on every stamped
  publish (`:104-109`). **`BuildEvent`** already exists (`:330-341`:
  `SlidePlaced{show,episode,chapter,slide}`, `EpisodeChanged{show,episode}`) —
  the domain a build/authoring surface uses; Studio already publishes it
  (`crates/hubs/workbench/src/studio.rs:227`).
- **Surfaces get the fabric** via `wire_fabric(&fabric)` storing
  `Option<Fabric>` — the workbench pattern (`crates/hubs/workbench/src/lib.rs:118`,
  field `:77`, called `main.rs:575`). The new surface copies this.
- **Coach precedent (interaction→signal→overlay→fact).** `coach_signals.rs`
  subscribes to shell/deck/capture/jobs/providers/**build** — *not its own coach
  domain* (anti-loop). Touchpoints trigger on `FabricEvent{signal:build.*}` /
  `Press`/`Hover` (`crates/egui-pilot-ai/src/coach.rs:64`), paint on
  `LayerId::new(Order::Foreground,…)` **without intercepting input**
  (`egui-pilot-ai/src/lib.rs:1144`, `coach.rs:8-11`), and tap lifecycle facts
  back via `set_coach_event_tap → publish_coach` (`main.rs:906`,
  `coach_fabric.rs:17`). This IS "AI creates overlays for interactions."
- **Overlays today** — no shared modal in the shell. Three mechanisms:
  (1) foreground-layer painter (coach spotlight); (2) `egui::Area` at
  `Order::Foreground` (chat overlays, `crates/chat/window/src/lib.rs:2209`);
  (3) **a governed declarative overlay STACK** — `OverlayCoordinator` +
  `InteractionRegistry` + `InteractionContent{Notice/Approval/RouteResult/
  ArtifactPreview}` in `crates/chat/interactions/src/lib.rs` (priority queue,
  focus governance, `open/ordered/dismiss`; **providers supply declarative
  content, never egui callbacks/geometry/colors** — `:1-4`). This is the rail we
  reuse for AI-authored overlays.
- **Editor** — `dpl-editor` is a view over ONE `dpl_doc::Slide`
  (`crates/editor/src/lib.rs`), edits via `mutate()`→`dpl_doc::save`
  immediately, ColdLight-pure, and **does NOT depend on `dpl-fabric`**
  (`Cargo.toml:8-11`). The workbench hub owns the fabric and embeds the editor
  (`editor: Option<dpl_editor::Editor>` `:93`).
- **Route model** — `Route{hub,sub}`; shell dispatches on `route.hub`, hub
  switches on `route.sub`. Workbench subroutes: `library · editor · episodes ·
  studio · test-lab` (`SPEC` `:29-61`).

## 3. Design — the signal-first authoring surface

### 3a. Where it lives
A **new Workbench subroute `author`** (one `SubRoute` in `SPEC`, one branch in
`WorkbenchHub::ui`). The hub already holds the `Fabric` (`wire_fabric`), so the
surface publishes without touching the editor's dependency ceiling.

### 3b. The surface (charter §§1–2, mock v4)
- **Intent bar** (top) — captured first, threads into the Ask-AI brief, stays
  editable. Facts only; no fabric traffic until a mutation.
- **Four-bucket infinite gallery** — Blocks · Elements · Effects · Templates,
  rendered from the **real registries** (`dpl_slides::blocks::elements()` census
  from DS2 + the template `registry::demos()`), so tiles are live and the
  census gate guarantees completeness. Virtualized scroll (egui_virtual_list,
  RS3-ratified) for the "endless" feel on the 75″ panel.
- **Touch filmstrip comp** (bottom) — the deck you're building; horizontal,
  tap-to-edit, ≥48px targets.

### 3c. The overlay editor — REUSE the governed stack, don't invent
New/Edit card opens a **popup overlay** driven by an `OverlayCoordinator`-modeled
stack (lift the abstraction from `crates/chat/interactions`, or depend on it),
with tabs Content · Quote · Media · Interactions · Ask AI. Content is
**declarative** (the interactions crate's law) so an AI-authored overlay can only
supply data, never paint. Guided fields are constrained (tone from the pool,
maxlen, bounded steppers). Rendered as an `Area`/foreground layer over the hub's
central content. Output is a valid `dpl_doc::Slide` at `SCHEMA_V` via the
editor's `mutate()`→`save` (edits stay egui-free in `dpl-doc`).

### 3d. Signals (the spine)
- **Additive `BuildEvent` variants** (never a flat enum, serde-free): extend
  `BuildEvent` with authoring facts — e.g. `CardAuthored{show,episode,slide}`,
  `EntityRegistered{kind,id}` (recipe/preset/element registered by AI). Additive
  variants only; envelope hand-mapped + pinned by a `crates/seam` wire-shape test.
- **Emit-with-mutation** — the hub publishes AFTER `dpl_doc::save` succeeds (as
  Studio does today), through one seam; `Failed` where it fails. Never before.
- **Anti-loop** — the authoring surface publishes `BuildEvent` and **does not
  subscribe to `BuildEvent`**; the coachable-signal catalog already forwards
  `build.*` to touchpoints, so AI/coach overlays react without the surface
  self-triggering.
- **Batching** — a bulk action (e.g. explode a PDF to cards) publishes ONE
  completion fact, not one-per-card.

### 3e. AI creates interaction overlays (the payoff, mechanism reused)
An interaction is a **registered recipe** (design-system step ④) whose triggers
ride the fabric exactly like a coach touchpoint: `signal:build.*` / `Press` /
`Hover` / `ValueChanged`. The AI authoring an interaction = declaring the recipe
entity (Census law) + a declarative overlay spec in the interactions registry.
Because the overlay content is declarative and the trigger is a fabric signal,
the AI **cannot** hand-paint or hard-wire — it can only register data the
existing renderer draws. That is what makes it "effortless" *and* safe.

### 3f. Census
Any authoring entity list (recipes, interaction ids) gets ONE `declare()`-style
registry fn + a census test, modeled on `coach_signals::declare`
(`crates/app/src/coach_signals.rs:268`) and its drift tests.

## 4. Scope

**In (this program):** the `author` subroute; intent bar; four-bucket infinite
gallery over real registries; touch filmstrip; the governed overlay editor with
five tabs → valid `dpl_doc::Slide`; additive `BuildEvent` authoring facts +
envelope test; the interaction-overlay-via-signal mechanism reused from coach +
interactions; the recipe/interaction census.

**Out (own passes / deferred):** real AI *generation* (the entity-first
generator runs on the C-series agent + chat seam — Ask-AI ships **Copy-prompt
first** per charter §8.2, Generate wires when the seam lands); deep per-template
authoring fidelity beyond the census set; the big-bang catalog migration
(design-system step ⑥).

**Blast radius:** `dpl-fabric` (additive `BuildEvent` variants + one seam test),
`dpl-hub-workbench` (new subroute + surface), a new surface module (or a small
`dpl-author` surface crate if the workbench file grows too big — layer-law
decision at Pass A), possibly a dependency on `dpl-chat-interactions` for the
overlay stack. `dpl-doc`/`dpl-editor` unchanged in dependency ceiling.

## 5. Work plan (ordered sub-passes — each its own PR + `cargo green`)

- **A · Signals + skeleton.** Additive `BuildEvent` authoring variants + seam
  wire-shape test; the `author` subroute + empty ColdLight surface wired to the
  hub's fabric; a stub publish on a placeholder mutation proven live on
  `/signals` + the activity feed. *Smallest, proves the spine.*
- **B · Gallery.** Four buckets over `elements()` + template registry; real
  thumbnails; virtualized infinite scroll; intent bar; touch filmstrip.
- **C · Overlay editor.** The governed overlay stack + five tabs + guided
  constrained capture → `dpl_doc::Slide` save with emit-with-mutation publish.
- **D · Interactions.** The recipe registry + census; the signal-driven
  interaction overlay (coach-pattern, foreground, non-intercepting); an existing
  recipe (`reveal`) applied end to end.
- **E · Ask-AI (Copy).** The click-raw→prompt composer + Copy-to-clipboard
  (charter §8.2 ratified path); the entity-first plan preview.
- **F · Generate (deferred).** Wire the entity-first generator to the chat/agent
  seam when it's ready; library-grows fact on registration.

Pass A + B + C together are the **red-pen-ready walkable shell**. The operator
red-pens after C (or earlier per pass); D–F build on the marked-up surface.

## 6. Risks

- **Editor ↔ fabric dependency.** Keep `dpl-editor` fabric-free; the hub
  publishes (existing separation). Decided: hub-publishes.
- **Overlay stack: reuse vs lift.** Prefer depending on
  `dpl-chat-interactions`'s coordinator; if its chat coupling is too tight, lift
  the `OverlayCoordinator` into a shared surface crate. Decide at Pass C by
  reading the crate's deps.
- **Infinite-scroll perf on the 75″ touch panel** — virtualize from day one
  (RS3); never a flat 200-tile grid.
- **Signal storms** — authoring bulk actions obey the batching law (one
  completion fact).
- **Surface file bloat** — if the `author` surface outgrows `workbench/src`, mint
  a `dpl-author` surface crate (structure-audit ceiling + egui-free census entry
  in the same commit, new-crate checklist).

## 7. Verification / acceptance

- `cargo green` per sub-pass (fmt · spawn+structure audits · clippy -D warnings ·
  tests · release build).
- **Signal proof** (the essential one): `crates/seam` wire-shape test pins the
  authoring envelope; a live round — author a card → `BuildEvent` appears on the
  `/signals` SSE **and** the shell activity feed, stamped, same frame (waker).
- Census tests for any new registry (recipes/interactions), drift-guarded.
- **Red-pen surface (real exe):** screenshots at REAL window sizes of the
  authoring surface, the infinite gallery, and an open overlay editor — the
  end-to-end flow the operator marks up.
- Layer law held: `cargo xtask structure` green (no new lateral deps, ceilings
  intact, egui-free census intact).

## 8. Evidence

*(filled at proof time — per-pass `cargo green` tails, the seam envelope test
output, the `/signals` live round, and the real-size authoring-surface
screenshots.)*
