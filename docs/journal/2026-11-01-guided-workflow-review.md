# Guided workflow review — 2026-11-01

Operator R297 requests a proper EguiPilot/ViziWiz/PixVix review, optimized hosted
guides, opt-in human demonstrations compiled into reusable procedures, live AI
demonstrations and guided learning, cheap execution/feedback, and novelty/MCP/
A2UI research. Scope completed as review; no code implementation or publication.

Baseline: `711fc72`, isolated `.worktrees/guided-workflow-review`, branch
`review/guided-workflow-standard`. Operator checkout untouched. Plan:
`docs/plans/guided-workflow-standard-review.md`. Deliverable:
`docs/research/guided-workflow-standard-review.md`. Requirement appended verbatim
as R297 and doc map linked. No commit/push/PR for this review-only cut.

Independent audits: pilot_review (native semantics/Coach), guide_review
(capture/export), workflow_prior_art (primary research). Both code auditors
reviewed the draft; added live demonstration, PixVix identity, export/capture
fidelity and format conflicts from their feedback. All findings are static;
no fresh app tests, screenshots, recordings, model-cost or performance evidence.

Broad novelty is not supportable: DocWizards 2005, AgentRR 2025, Tango and
WalkMe overlap directly. Proposed contribution is a versioned/conformant shared
procedure with watch/follow/execute modes, bounded optional AI feedback and
portable publishing. Current MCP 2026-07-28, Skills/Tasks extensions and A2UI
0.9.1 (1.0 candidate) provide building blocks. October 27 A2UI Express update
already targets compact inference; do not sell compact syntax as first-ever.

Twelve findings are in the report. Highest priorities: guide edit unvalidated
old-path deletion; capture queue can run after Stop/cross sessions; failed Save
loses pending state; source edits delete content; normal CoachHub can launch a
devtools-only overlay; hidden/mis-scoped Coach targets can satisfy/mount wrongly.
Do not broaden runtime boundaries before reconciling them. No defect fixed here.

Next recommended cut: plan focused correctness/preservation fixes and fault
tests, then one native demonstration→reviewed procedure→recorded/live watch,
human follow and execute slice. Preserve existing workflow registry/doctrine
and native Rust UI. Separate public website artifact from native app rendering.
Do not begin universal recorder, training upload, host publication or external
standards outreach from this research handoff.

Documentation validation passed: existing prebuilt xtask docs-status checks
21 canonical capability rows; report JSON parses, six relative links exist,
git diff --check clean. No application test suite run for these docs-only edits.
