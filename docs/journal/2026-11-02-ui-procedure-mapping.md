# GF0 execution handoff

Operator: execute following R296. Plan:
`docs/plans/ui-procedure-mapping-spike.md`. Worktree:
`.worktrees/guided-workflow-review`, branch `review/guided-workflow-standard`,
baseline main `711fc72`. Parent and operator checkouts are preserved.

Implemented the isolated `dpl-workflow-interop` workspace crate, with no app
dependency or runtime. It projects/imports existing registered media WorkflowDoc
types through a bounded Open Workflow 1.0.3 profile. Unknown behavior, versions,
ignored native fields, expression ambiguity, invalid wire IDs and duplicate
JSON keys refuse. It assesses three source-grounded UI procedure slices without
pretending they lower to media stages. Descriptive setup annotations survive and
remain unverified. Pinned upstream schemas/licenses are vendored for offline
tests; Arazzo 1.1 describes hypothetical local adapter operations only.

25 tests passed; all-target clippy with warnings denied passed. Actual CLI
positive and negative cases and output: `docs/proof/ui-procedure-mapping/`.
Independent review closed identity bounds and annotation preservation defects.
No application screenshot or live UI qualification is claimed.

Research conclusion remains: do not propose an entirely new workflow language
on this evidence. Investigate the narrow UI identity/outcome/human-follow
contract using existing Rust APIs. Required durable artifact and correlated
export outcomes are absent from current Flow; the coach bridge loses JobId.
The twelve original correctness findings are not fixed by this mapping cut.
Next: correctness/preservation fixes, then a real shared outcome controller and
watch/follow/execute slice; external adoption still needs independent consumers
and reusable distribution terms. No license change or outside contact made.

No commit, push or merge in this cut. Preserve the earlier research documents
and journals. Ignore the tool-created PowerShell ModuleAnalysisCache under
untracked `Microsoft/`; it is not product content.
