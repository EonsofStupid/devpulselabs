# BS1 · M1 journal — the documents layer (2026-08-07)

**Objective (POAM):** type "brainstorming" in chat → phased editable
board populates inline → assist proposes visible peer items → Converge
promotes into the explainer intake pre-fill.

**Done this milestone (all in `crates/workflows` + `schemas/`):**
- `BrainstormDoc`/`BrainstormItem`/`BrainstormCluster`/`BrainstormPhase`
  (diverge→cluster→converge) /`BrainstormOrigin` (operator|assist) —
  the board document; empty seed is VALID at Diverge (the board opens
  before the idea lands), `validate_brainstorm` names every fault
  including breakdown-chain loops.
- `BrainstormOp` census (8 verbs: Seed · AddItem · EditItem ·
  RemoveItem · AddCluster · RenameCluster · AssignCluster ·
  AdvancePhase) through ONE pure all-or-nothing `apply` — a refused op
  mutates nothing and names its issues. Phase gates: leaving Diverge
  needs seed + ≥2 ideas; leaving Cluster needs ≥1 cluster and every
  root idea sorted (unsorted ids named); past Converge refuses
  ("promote it, don't advance it"). EditItem CLAIMS assist work
  (origin flips to Operator — touched work is the operator's work);
  RemoveItem cascades the subtree so nothing dangles.
- `BrainstormScaffold` census (v1: `breakdown`) + `mint_brainstorm`.
- `distill(&BrainstormDoc) -> Result<Vec<IntakePrefill>, Vec<Issue>>`
  — pure/deterministic, Converge-gated; subject ← seed, key-points ←
  cluster-ordered item texts, beats ← cluster labels; every emitted
  field id proven against the explainer intake's census.
- `schemas/brainstorm.v1.schema.json` (strict, additionalProperties
  false, v const 1) + `load_brainstorm`/`save_brainstorm` riding the
  shared crash-safe writer (now "all four run documents").

**Proof:** 34/34 `dpl-workflows` tests (10 new: scaffold census,
determinism + all-or-nothing, refusal ladder, assist-claim, cascade,
round-trip + newer-refusal, distill fill/refuse, wire pins with SIX
negative drift cases incl. unknown phase/origin), fmt + clippy clean
in-lane on a fresh compile.

**Decisions logged:** V2's "unknown op tag" negative moved from the
wire schema to the apply unit tests — ops are wire-adjacent (the
assist seam will speak them) but not persisted in v1, so the doc
schema stays the only wire contract (deviation noted in plan §9.1).

**Remaining (POAM):** BS-M2 surface (trigger, view, inline painters,
host session + undo, --brainstorm-drill, ChatProofMode::Brainstorm) ·
BS-M3 assist populate + promote wiring + pilot scenario + close-out.
