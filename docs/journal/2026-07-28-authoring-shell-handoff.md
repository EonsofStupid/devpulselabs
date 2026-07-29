# 2026-07-28 — Authoring shell handoff (resume at Pass A)

**Purpose:** a clean-context handoff. A fresh session reads this + the memory
files and resumes building the authoring UI against the mock via the discipline
the operator asked for: **plan → work → test → verify**, one sub-pass at a time,
until the surface matches the mock and the operator's red pen.

---

## One-line state

DS2 element census DONE (PR #114). The authoring **charter** (vision) + the
signal-first **build plan** are written and on PR #119. Mock v4 is the ratified
UX picture. **Resume by executing Pass A** of `docs/plans/authoring-shell.md`.

## The loop to run (operator's instruction)

For each sub-pass A→F: **plan** (it's already scoped in `authoring-shell.md` §5 —
re-ground the real files in-session first, never from memory) → **work** (branch
off `main`, code) → **test** (`cargo green` + the pass's specific proofs) →
**verify** (against the mock + the signal/census/ColdLight laws; real-exe
screenshot where it's a surface) → land via PR → next pass. Build it out until
the surface matches `docs/design/mocks/authoring-surface-mock.html`.

## Where to resume: PASS A (smallest, proves the signal spine)

1. **Additive `BuildEvent` authoring facts** in `crates/fabric/src/lib.rs`
   (`BuildEvent` is at `:330-341`, already has `SlidePlaced`/`EpisodeChanged`).
   Add variants like `CardAuthored{…}` / `EntityRegistered{kind,id}`. Additive
   ONLY, serde-free. Add/extend the `crates/seam` wire-shape envelope test that
   pins the JSON.
2. **New `author` subroute** on the Workbench hub: one `SubRoute` in `SPEC`
   (`crates/hubs/workbench/src/lib.rs:29-61`) + one branch in `WorkbenchHub::ui`
   (`:327-348`). Empty ColdLight surface for now.
3. **Publish after a mutation** from the hub (it already owns the fabric via
   `wire_fabric` `:118`, field `fabric: Option<Fabric>` `:77`). Prove it live:
   the fact appears stamped on the `/signals` SSE **and** the shell activity feed
   the same frame (waker law).
4. `cargo green` + the `/signals` live round = Pass A done. Then Pass B (gallery).

## Ground truth — do NOT re-derive (from the fabric map + signal doctrine)

- **Fabric API** (`crates/fabric/src/lib.rs`, pkg `dpl-fabric`, one file):
  per-domain enums; `publish_build`/`subscribe_build` (`:545`/`:550`), same shape
  for capture/coach/job/etc. `Fabric` is `#[derive(Clone)]`, one instance built
  `crates/app/src/main.rs:545`, shared by clone. **Stamp-at-publish**:
  `Stamped<T>{seq,ms,ev}` (`:415`), `stamp()` (`:496`), ONE global AtomicU64 seq.
  **Waker**: every stamped publish calls `request_repaint` (installed
  `main.rs:1206`). Surfaces hold `Option<Fabric>` via a `wire_fabric` setter.
- **The interaction→signal→overlay loop (coach precedent — reuse, don't invent):**
  `coach_signals.rs` subscribes to build/etc. **not its own coach domain**
  (anti-loop); touchpoints trigger on `FabricEvent{signal:build.*}`/`Press`/
  `Hover` (`egui-pilot-ai/src/coach.rs:64`); overlays paint on
  `LayerId::new(Order::Foreground,…)` **without intercepting input**
  (`egui-pilot-ai/src/lib.rs:1144`); lifecycle facts tap back via
  `set_coach_event_tap → publish_coach` (`main.rs:906`, `coach_fabric.rs:17`).
- **Governed declarative overlay stack (reuse for AI-authored overlays):**
  `crates/chat/interactions/src/lib.rs` — `OverlayCoordinator` (priority queue,
  `open/ordered/dismiss`, focus governance) + `InteractionContent{Notice/
  Approval/RouteResult/ArtifactPreview}`. **Providers supply declarative content,
  never egui callbacks/geometry/colors** — the exact safety rail for "AI creates
  overlays."
- **Editor** (`crates/editor/src/lib.rs`, `dpl-editor`): view over ONE
  `dpl_doc::Slide`, `mutate()`→`dpl_doc::save` immediately, ColdLight-pure, and
  **does NOT depend on `dpl-fabric`** — keep it that way; the HUB publishes.
- **Overlays**: no shared modal in the shell today; `egui::Area` at
  `Order::Foreground` (chat, `crates/chat/window/src/lib.rs:2209`) or the
  coordinator above. A shell-wide modal would hook at the tail of `Shell::ui`
  (`crates/shell/src/lib.rs`, after `central()` `:1042`).

## Laws that gate every pass (signal-doctrine.md LOCKED + architecture.md LAW)

- Per-domain enums, **no flat/global enum**; a new domain = enum + channel + two
  methods; new signal = one variant.
- **Emit-with-mutation**: publish AFTER the write succeeds, one seam; `Failed`
  where it fails. A mutation that doesn't signal is a bug.
- Fabric **serde-free**; one envelope at boundaries, pinned by a `crates/seam`
  test. Never stamp by hand.
- **Anti-loop**: a producer surface must not subscribe to the domain it
  publishes.
- **Batching**: bulk actions → ONE completion fact, never per-item.
- **Census law**: any list (recipes/interaction ids) = one `declare()` fn + a
  census/drift test (model on `coach_signals::declare` `:268`).
- `dpl-doc`/`dpl-editor` stay **egui-free / fabric-free** per the layer law;
  `cargo xtask structure` enforces ceilings. **Surfaces paint ColdLight only**
  (tones/`Fluid::of`/generated consts; no raw hex, no ad-hoc frames).

## Open decisions still to ratify (charter §8)

1. Blocks-vs-Elements exact line (proposal: Elements=atoms, Blocks=assembled
   chunks below a slide). 2. ✅ prompt path = both, phased (Copy first). 3.
   starter recipe set. 4. how "skills" surface in the aware chat. Confirm with
   operator as the passes reach them.

## Working discipline (the non-negotiables that bit us this session)

- **Real clone = `~/Projects/devpulselabs`** (no underscore). `devpulse_labs`
  (underscore) is a DIVERGENT non-git snapshot — never work there, never copy
  across. Check `git rev-parse` first. See memory [[git-workspace-and-ci-state]].
- Work in a **worktree off `main`** so the operator's active checkout is
  undisturbed. **PRs, never direct pushes**; the pre-push hook runs `cargo green`
  (don't bypass). `main` moves fast — expect to rebase; STATE.md is the usual
  conflict (resolve newest-on-top).
- **The product is the EXPERIENCE, not the feature list.** A green-but-invisible
  pass is a regression. Mock-first: build against
  `docs/design/mocks/authoring-surface-mock.html`. Don't hand back plumbing as
  the deliverable.
- Merge/close-out of UI PRs (incl. DS2 #114) and charter ratification are the
  operator's calls — don't self-merge.

## Artifacts

- Mock (live, clickable): the DevPULSE Author artifact (v4 — intent-first, real
  template thumbnails, infinite scroll, touch filmstrip, overlay editor + AI
  entity-first loop). Saved as `docs/design/mocks/authoring-surface-mock.html`.
- Charter: `docs/plans/authoring-charter.md`. Build plan:
  `docs/plans/authoring-shell.md`. Both on **PR #119**.
- DS2 census: `docs/plans/ds2-element-library.md`, PR #114 (merge + Windows shot
  pending).
- Memory: [[authoring-charter-vision]], [[design-system-build-order]],
  [[git-workspace-and-ci-state]], [[pr-squash-merge-on-completion]].

## What waits on the operator

Merge DS2 #114 (+ Windows element-wall screenshot) · ratify charter §§1–5 + the
3 open §8 decisions on #119 · give "go" to start Pass A (or it starts on next
engagement per this journal).
