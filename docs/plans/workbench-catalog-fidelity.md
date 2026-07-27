# Cycle 2 — Workbench catalog fidelity (PLAN — awaiting operator approve)

Status: **DRAFT — do not implement until explicitly approved.**

## 1. Context

Operator / Claude session ask: Workbench Library should show real
categories, templates, and overlays — not hollow placeholders. STATE gates
this on a missing `experience-inventory.md`. This cycle authors the
inventory first, then one fidelity pass against it, with EguiPilot proof
before merge (major-feature cadence).

## 2. Current state (to re-read in-session before code)

- Workbench Library: `crates/hubs/workbench`, `docs/plans/l1c-workbench-library.md`.
- ColdLight ingredients / slide-catalog skills: `.claude/skills/coldlight-system`,
  `slide-catalog-pass`.
- Experience inventory: **absent** (`docs/plans/experience-inventory.md`).
- EguiPilot fixture `library` already boots `--route library/templates`
  (verify route still valid after Program-L).

## 3. Target design (proposed)

1. Author `docs/plans/experience-inventory.md` — layman-facing surface census
   for Workbench Library (sections, expected templates/overlays, what "done"
   looks like vs the old app). Ground-truth from READ-ONLY prior app + live
   hub code.
2. One fidelity pass: close the highest-gap inventory rows that are
   ingredient/template/overlay shaped (ColdLight-only; no one-off paint).
3. EguiPilot scenario under `tests/eguipilotai/` + proof PNG; `cargo green`;
   PR → squash-merge → tip STATE.

## 4. Boundaries

- Out this cycle: W10 slide association; full ColdLight remaining program;
  estate selection/undo; AEP Phase 2.
- No inventing templates that are not generated ingredients.

## 5. Verification target

1. Correctness — inventory authored; touched tests green; clippy clean.
2. Reality — EguiPilot `devpulse-cloud` Library scenario + proof PNG.
3. Adversary — empty/missing seeds degrade honestly; no invented catalog rows.

## 6. Halt

**Operator: approve this Cycle 2 plan? (reply `approve`, or adjust scope.)**
