# Remaining work — coach / ledger / lesson lane

**Resume point for the next session on this lane.** Read this before
coding. Every code pass still authors its own `docs/plans/<pass>.md`
first (law 1). Parallel lanes (authoring, chat assist, L4, workflows)
are listed at the bottom so they are not accidentally stolen.

Updated 2026-07-28 after PR #117 (`dpl-ledger`) landed on main.

---

## The vision (one sentence)

DevPULSE Labs, while running, can teach — inside its own UI today, and
eventually over the whole monitor and into foreign apps — with lessons
whose steps advance on real postconditions, whose outcomes land in a
queryable ledger, and whose privileged modes only escalate by the
provenance of the operator's keystroke.

That vision is now half-foundation, half-product:

| Stack | Status |
|---|---|
| **Record** — stamped coach + intent facts → Parquet ledger | **Landed** (Phase 1.5 → gate → Phase 2) |
| **Authorize** — `OperatorIntent` + determinism modes | **Landed** (switch yes; pinning no) |
| **Teach inside DevPULSE** — instrumented surfaces + postcondition steps | **Partial** (Workbench started; Library section selection still blocks a full lesson) |
| **Paint the monitor** — click-through overlay process | **Spike proven** (transparency + passthrough on X11); daemon not done |
| **Perceive foreign apps** — AT-SPI / UIA tier ladder | **C2b self-test PASS** (`dpl-coach-overlay`); C2a ratified |
| **Analyze** — DuckDB lens over ledger files | **Phase 3 ADR**, after real volume |

---

## Ordered program (this lane — do in order unless the operator redirects)

Each item is one PR-sized pass. Status is the gate for starting the next.

### 1. Library section harness (p6) — **CLOSED** (PR #135)

`--lib-section <slug>` on the real exe; Library walls provable. Lesson
E2E scripts may still deepen coverage, but the harness blocker is gone.

### 2. Live synthetic `/strict` refusal — **CLOSED** (2026-07-29)

Mint-site policy extracted as `resolve_operator_slash` in
`crates/app/src/chat.rs`; unit matrix proves agent-injected `/strict`
→ `IntentRefused · synthetic-input` and mode stays behavioral. Plan:
`docs/plans/strict-refusal-witness.md`.

### 3. Overlay passthrough proof (p2b) — **CLOSED** (2026-07-29)

**Why:** transparency is proven on X11+wgpu; click-through was not.
C2b cannot start without knowing whether `mouse_passthrough` actually
delivers clicks to a window underneath.

**Shape:** `passthrough-probe` in `crates/desktop-spike` — witness +
overlay + XTEST; control pair ON/OFF. Plan:
`docs/plans/p2b-overlay-passthrough.md`. Evidence: spike §8.1 — **PASS**.

### 4. Rail accordion degrade (p4) — **CLOSED** (2026-07-29)

**Why:** at small heights the accordion clipped the unfold to nothing —
lessons that name a rail hub failed when chrome collapsed.

**Shape:** rail body `ScrollArea` + pending reveal of the open house.
Plan: `docs/plans/p4-rail-accordion-degrade.md`. Proof:
`docs/proof/p4-rail-cramped-640x480.png` + shell kittest.

### 5. C2s foreign-target spike (p5) — **CLOSED** (2026-07-29 / live 2026-07-30)

**Plan:** `docs/plans/coach-c2s-foreign-target-spike.md` — evidence §9 + §10.
Locate + compose + ColdLight paint + `Action.doAction` + **Triggered
observe** (Sync-grab + ReplayPointer; digit-matched display watch;
`GetAccessibleAtPoint` identity gate) on `gnome-calculator` button `7`.
**Live operator proof 2026-07-30:** ButtonPress inside hotspot → Triggered.
Journal: `docs/journal/2026-07-30-c2s-triggered-live.md`.

### 6. C2a coach-overlay ADR → C2b daemon — **C2b SELF-TEST PASS** (2026-07-30)

**ADR:** `docs/plans/coach-overlay-adr.md` — RATIFIED.  
**C2b plan:** `docs/plans/coach-c2b-overlay-daemon.md` — pass-1 PASS.  
**Journal:** `docs/journal/2026-07-30-c2b-coach-overlay.md`.

Crate `crates/coach-overlay` (`dpl-coach-overlay`): locate → identity →
WINDOW paint → Sync-grab → Triggered on calculator; bogon refuses.
Never a dep of `app`. Pass 2 stdio coach loop deferred.

**Done when (pass 1):** met — self-test paints hotspot + Triggered /
non-intercept observe. Remaining: land PR; optional Pass 2 stdio.

### 7. Strict-mode pinning (replay pass)

**Why not earlier:** the gate exists; pinning is expensive and belongs
with replay / baseline comparison. Evidence already *forces* the mode
label; this pass makes strict mean byte-identical.

**Done when:** fixed clock + pinned RNG path under `Strict` /
`force_evidence`, proven by a byte-compare of two proof shots.

### 8. Phase 3 — DuckDB C-dep ADR

**Why last in this spine:** needs real Parquet volume and a real
question the app cannot answer today. Three candidates stay live
(external CLI only / subprocess / `bundled` crate). No implementation
in the ADR pass.

**Done when:** ADR merged with a measured recommendation.

### 9. Classification Layer-1 → `class` column

**Why after ledger exists:** schema already reserves `class`. Base
CUI/Presidio vocabulary, store the class never the value (§9.5).
Per-app packs and HMAC pseudonymization are later.

**Done when:** obvious classes (password / PII-shaped / CUI-tagged)
write into `class` on matching rows; redaction still outranks
reproducibility.

---

## Explicitly parked (do not start without operator ask)

- Games / multiplayer training tips (training-only ruling; anti-cheat
  hazard).
- Tier B foreign plug-ins (Blender/`bpy`, SolidWorks COM) — ratified
  narrowly as marshal-only; after Tier A works.
- Wayland GNOME overlay as first target — X11 first on this box.
- Shell/deck stamping — unstamped by design; ledger does not invent
  stamps for them.
- In-app DuckDB dependency — Phase 3 ADR decides.

---

## Parallel lanes (not this resume — operator redirects)

Do not pick these up as "the next thing" from this journal unless the
operator says so. They have their own nexts:

| Lane | Next (from their own journals / STATE) |
|---|---|
| Authoring | Pass B — four-bucket infinite gallery + filmstrip |
| Chat assist | First bake-off interaction + estate preference record |
| Fable / WiredFRONT | Guide UX redlines; R2c chat→comp; R2b LUT registry |
| L4 / workflows | Their feature branches (`feat/l4-…`, `feat/workflow-…`) |

---

## How to start the next session

1. `git checkout main && git pull`
2. **Session OS boot:** `docs/STATE.md` → `docs/ops/SESSION.md` →
   `docs/ops/resume/CURRENT.md` (skill `session-os`).
3. Author `docs/plans/<pass>.md` for **item 1** (Library section harness)
   unless the operator redirects — non-trivial: grounder → gap-closer →
   approve → executor.
4. Execute one pass → PR → merge → update this file + ops SESSION →
   journal near context budget → stop or take the next item.

One pass at a time. Evidence before assertion. Main moves — re-ground
before every pass.
