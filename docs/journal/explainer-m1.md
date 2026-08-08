# EX1 · M1 journal — the documents layer (2026-07-29)

**Objective (POAM):** chat-guided intake → production+script bible →
tailored storyboard → real 16:9 video, short↔long as one system.

**Done this milestone (all in `crates/workflows` + `schemas/`):**
- `BibleDoc`/`BibleBeat` — production + script bible (narration required
  per beat: "the script bible has no silent beats"), `validate_bible`,
  crash-safe `save_bible`/`load_bible` with newer-refusal.
- `IntakeSpec`/`IntakeField`/`IntakeFieldKind{Text·Choice·FormatPick·
  BeatsDraft}` — the declarative guided Q&A, carried by the blueprint as
  an ADDITIVE-optional `WorkflowDoc.intake` (stays v1; doctrine law 3
  gained the additive-amendment carve-out, same commit).
- `FormatProfile` census — `short` 4×11s · `standard` 7×17s · `long`
  12×30s, all 16:9/30fps (ratified axis); `format_profiles()`/
  `format_profile(id)`.
- `tailor(&BibleDoc) -> Result<WorkflowDoc, Vec<Issue>>` — pure,
  deterministic, schema-valid output, refuses with named issues;
  narration deliberately stays in the bible (upstream node), never
  leaks into slide text.
- `save_workflow`/`load_workflow` (`workflow.json`) — the missing
  persistence half; ONE shared `save_json` writer (save_board folded in).
- `explainer()` registered (census 1→2) with its 8-field intake and
  7-beat default arc; `validate()` gained the intake arm.
- `schemas/bible.v1.schema.json` (new) + `workflow.v1.schema.json`
  additively amended with the intake vocabulary.

**Proof:** 24/24 `dpl-workflows` tests (8 new: bible pin + drift
refusal, intake-amendment drift refusal, tailor determinism/axis/
refusal, doc round-trip + newer-refusal, format census), fmt + clippy
`-D warnings` clean in-lane.

**Remaining (POAM):** M2 run-becomes-real (raster leg, run_workflow_doc,
sync render, ffprobed short+long) · M3 guided chat intake + panel form ·
M4 close-out (double-green merge, exe, STATE).

**Decisions logged:** per-beat span reaches `comp_for_board` in M2 (the
tailored doc carries geometry now; span rides the profile the run
reads). No AskUserQuestion dialogs from here — decisions stated inline
(operator ruling, 2026-07-29).
