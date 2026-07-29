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
| **Paint the monitor** — click-through overlay process | **Spike proven** (transparency); passthrough + daemon not done |
| **Perceive foreign apps** — AT-SPI / UIA tier ladder | **Plan only** (C2s spike authorized, not executed) |
| **Analyze** — DuckDB lens over ledger files | **Phase 3 ADR**, after real volume |

---

## Ordered program (this lane — do in order unless the operator redirects)

Each item is one PR-sized pass. Status is the gate for starting the next.

### 1. Library section harness (p6) — **CLOSED** (PR #135)

`--lib-section <slug>` on the real exe; Library walls provable. Lesson
E2E scripts may still deepen coverage, but the harness blocker is gone.

### 2. Live synthetic `/strict` refusal (optional witness) — **NEXT**

**Why now:** the provenance law is unit-covered; the optional live
SSE witness was left on the table when #116 landed. Small, closes a
honesty gap before overlay work distracts.

**Done when:** agent fills composer with `/strict` + Send →
`IntentRefused · synthetic-input` on SSE + feed; mode stays
behavioral. Screenshot optional.

### 3. Overlay passthrough proof (p2b)

**Why:** transparency is proven on X11+wgpu; click-through is not.
C2b cannot start without knowing whether `mouse_passthrough` actually
delivers clicks to a window underneath.

**Shape:** synthetic click + witness window (plan already anticipated
`dpl-coach-overlay` as its own process). Probe only — no product
chrome.

**Done when:** click lands on the witness under the overlay; documented
in the overlay spike plan §evidence.

### 4. Rail accordion degrade (p4)

**Why:** at small heights the accordion can clip to nothing — a
lesson that names a rail hub fails when chrome collapses. Product
feel + lesson reliability.

**Done when:** at a documented cramped size the rail still exposes the
active hub (scroll / collapse / overflow — pick one, prove it).

### 5. C2s foreign-target spike (p5)

**Plan:** `docs/plans/coach-c2s-foreign-target-spike.md` (authorized).

**Why before C2a ADR:** findings feed the ADR; the operator ruled
spike-first. Tier A app on this ARM64 X11 box; proof-of-concept only;
AT-SPI path with zero adapter.

**Done when:** resolve a control by identity, paint ColdLight spotlight
at true screen coords, observe operator press, separately invoke by
identity — evidence written into the spike plan, then C2a.

### 6. C2a coach-overlay ADR → C2b daemon

**Why after C2s:** ADR argues from measured coords + AT-SPI reality,
not taste. Separate process `dpl-coach-overlay`, feature-gated /
devtools-only until ratified (no-IPC law; chat providers are the
template).

**Done when:** ADR merged; daemon self-test paints a hotspot and
proves non-interception.

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
