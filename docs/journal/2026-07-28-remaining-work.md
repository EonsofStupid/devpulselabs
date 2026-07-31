# Remaining work — coach / ledger / lesson lane

**Resume point for the next session on this lane.** Read this before
coding. Every code pass still authors its own `docs/plans/<pass>.md`
first (law 1). Parallel lanes (authoring, chat assist, L4, workflows)
are listed at the bottom so they are not accidentally stolen.

Updated 2026-07-30 — **product spine alignment** after C2b #158:
physics closed; NEXT is operator experience (C2c), not Pass-2 stdio.
Canonical product contract:
`docs/plans/coach-operator-experience.md`.

---

## The vision (one sentence)

DevPULSE Labs, while running, teaches the operator through a guided
Coach surface: pick an installed app, mint a semantic fabric from
accessibility streams (not screenshots), let AI mount touchpoints that
paint and wait for real presses, advance lessons on real postconditions,
record outcomes in the ledger, and escalate privileged modes only by
operator-keystroke provenance — with Advanced / EguiPilot as a separate
devtools surface.

That vision is now physics-proven, product-incomplete:

| Stack | Status |
|---|---|
| **Record** — stamped coach + intent facts → Parquet ledger | **Landed** (Phase 1.5 → gate → Phase 2) |
| **Authorize** — `OperatorIntent` + determinism modes | **Landed** (switch yes; pinning no) |
| **Teach inside DevPULSE** — instrumented surfaces + postcondition steps | **Partial** (C1 touchpoints; full lesson UX still open) |
| **Paint foreign / monitor** — overlay process | **C2b PASS** (#158 `dpl-coach-overlay` self-test) |
| **Perceive foreign apps** — AT-SPI / UIA Tier A | **PASS** (C2s live Triggered + C2b) |
| **Operator Coach UX** — picker, mint ETA, seeds, refuse tips | **NEXT (C2c→C2f)** — missing |
| **Analyze** — DuckDB lens over ledger files | **Phase 3**, after real volume |

---

## Ordered program (this lane — do in order unless the operator redirects)

Each item is one PR-sized pass. Status is the gate for starting the next.

### 1–5. Foundation — **CLOSED**

p6 Library harness (#135) · `/strict` refusal · p2b passthrough · p4 rail
degrade · C2s foreign-target (live Triggered 2026-07-30).

### 6. C2a ADR → C2b overlay daemon — **CLOSED** (PR #158)

**ADR:** `docs/plans/coach-overlay-adr.md` — RATIFIED.  
**Plan:** `docs/plans/coach-c2b-overlay-daemon.md` — pass-1 PASS.  
**Journal:** `docs/journal/2026-07-30-c2b-coach-overlay.md`.

`--self-test` Triggered on calculator; bogon refuses. Never a dep of
`app`. **Pass-2 stdio is PARKED** (Advanced plumbing — not product NEXT).

### 7. C2c — operator experience spine + UI mock — **LOOK-APPROVED**

**Plan:** `docs/plans/coach-operator-experience.md` +
`docs/plans/coach-c2c-operator-mock.md`.  
**Mock:** `docs/ops/references/coach-operator-mock-c2c.html`.  
Home: shell Coach hub. Operator 2026-07-30: “looks good.”

**Done when:** met for look-approve. NEXT = C2d inventory.

### 8. C2d — lightweight target inventory (Phase 0+1) — **PASS**

**Plan:** `docs/plans/coach-c2d-lightweight-inventory.md`  
**Journal:** `docs/journal/2026-07-30-c2d-inventory.md`  
Crate `dpl-coach-inventory` + `coach-inventory-list`. Cold XDG + shallow
AT-SPI desktop-root census. Calculator seed RUNNING. No deep walk.

**Done when:** met. NEXT = C2e fabric mint.

### 9. C2e — fabric mint for selected Calculator

**PASS** 2026-07-31 — `dpl-coach-fabric` + `coach-fabric-mint`. Live
Calculator Ready · 82 nodes · seed `7` · ~255 ms. Journal:
`docs/journal/2026-07-31-c2e-fabric-mint.md`.

**Done when:** met. NEXT = C2f.

### 10. C2f — Operator Import Process → guided lesson + paint

**Contract:** [`docs/plans/coach-lab-experiment-program.md`](../plans/coach-lab-experiment-program.md)
+ mock rev Import Process. Honesty prompt → essential checks (a11y) →
Standard Import → Mount/paint/Triggered. Advanced Import / Harvest =
Lab spikes (CX-harvest), not a silent fake Standard mint.

**Done when:** operator picks Calculator → confirms import → checks pass
→ sees paint → press Triggered without CLI probes; ledger stamps import facts.

### 11. Strict-mode pinning (replay) — after product path or operator ask

Fixed clock + pinned RNG under `Strict` / `force_evidence`; byte-compare
proof shots.

### 12. Phase 3 — DuckDB C-dep ADR

After real Parquet volume + a real unanswered question. See
`docs/plans/phase3-analytics-program.md`.

### 13. Classification Layer-1 → `class` column

Obvious classes into `class`; redaction outranks reproducibility.

---

## Explicitly parked (do not start without operator ask)

- **C2b Pass-2** stdio `CoachCommand`/`CoachEvent` loop on overlay
  (Advanced; ADR gates still apply).
- Games / multiplayer training tips (training-only ruling; anti-cheat
  hazard).
- Tier B foreign plug-ins (Blender/`bpy`, SolidWorks COM) — after Tier A
  product path.
- Wayland GNOME overlay as first target — X11 first on this box.
- Shell/deck stamping — unstamped by design.
- In-app DuckDB dependency — Phase 3 ADR decides.
- Tier C vision as primary eyes — refuse path only.

---

## Parallel lanes (not this resume — operator redirects)

| Lane | Notes |
|---|---|
| Authoring | Own POA&M — do not steal |
| Chat assist | Own bake-off / provider tabs |
| Fable / WiredFRONT / L4 / workflows | Own branches |

---

## How to start the next session

1. `git checkout main && git pull`
2. Session OS: `docs/STATE.md` → `docs/ops/SESSION.md` →
   `docs/ops/resume/CURRENT.md`
3. Read `docs/plans/coach-operator-experience.md`
4. Author/execute **C2c mock** (or next open C2d–C2f) — plan first
5. One pass → PR → merge → tip this file + SESSION → journal

One pass at a time. Evidence before assertion. Accuracy over speed —
second/third doc passes are welcome when contradictions remain.
