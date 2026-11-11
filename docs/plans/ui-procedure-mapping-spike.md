# GF0 — executable standards mapping experiment

2026-11-02. Authorized by the operator's **execute** following R296's adoption
assessment. Baseline main711fc72 in the existing isolated review worktree.

## Objective and scope

Implement the first concrete engineering gate of the adoption assessment:
three source-grounded UI procedures mapped against pinned Open Workflow1.0.3,
Arazzo1.1.0 and existing Rust contracts, plus a runnable mapping checker.
This is a local devtools/interop experiment, not a new execution engine.

Grounding found that `dpl-workflows::StageKind` is exclusively media production.
EguiPilot Flow separately owns devtools UI actions/expectations. Preserve this
distinction: an unsupported mapping returns explicit gaps, not fabricated native
support. No production endpoint, process adapter, UI change or live recorder.

## Implementation

1. Add one isolated workspace crate, `dpl-workflow-interop`, for pure mapping,
   validation and diagnostics; a local CLI/example reads fixtures and prints
   a JSON assessment. It has no dependency from the application and does not
   invoke actions, commands, scripts, network or model inference.
2. Pin standards/versions and fixture provenance. Ground edit/save in the
   existing own-looks fixture, secondary-window setting in Chat's provider
   selection, and asynchronous export in Composer's real export path. Record
   unsupported persisted-effect, multiwindow, job-correlation and human-follow
   guarantees. Do not perform provider turns or change permission settings.
3. Implement a bounded Open Workflow projection/import of the existing
   `WorkflowDoc` media stages as a positive control. Reuse public Rust types and
   validators, preserve identities and parameters, reject unsupported features
   and versions, and report every unrepresented semantic obligation. No second
   registry of native stage kinds and no arbitrary shell/script/container arm.
4. Keep UI mappings descriptive until their required semantics can be carried
   by existing public APIs. Validate fixture documents against pinned upstream
   schemas when available; distinguish schema validity from runtime support.
   Arazzo is an API mapping comparison, not a claim that every UI is an API.

## Verification

Meaningful tests: actual registered native workflows round-trip without loss;
invalid/version-mismatched documents refuse; injected unknown required behavior
never disappears; fixture assessments expose capability gaps consistently.
Run crate tests, formatting and clippy, then the actual mapping CLI over the
fixtures and a negative case. Record output as evidence. No pixel claim: no UI
is changed by this pass. Independent grounding and implementation review.

## Deliverable

Working local mapping code, fixtures, source/standard references, actual CLI
results and a field-by-field gap matrix. Amend this plan if schema/source
evidence changes the implementation. Keep licensing, repository separation,
external interviews and formal standards submission as explicit later decisions.
Do not commit/push unrelated parent or worktree edits.
