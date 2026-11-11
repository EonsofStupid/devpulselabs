# GF1a handoff — immutable source-guide edits

User resumed after GF0. Continued in `.worktrees/guided-workflow-review`, branch
`review/guided-workflow-standard`, main baseline711fc72. Plan:
`docs/plans/guide-edit-preservation.md`. Prior GF0 and unrelated work preserved.

Implemented F1: engine-steps update_guide returns GuideRevision, validates old
and submitted media/identity/schema, preserves raw metadata and all flat source
files, and writes exclusively reserved sibling revisions. Pending -> complete
marker rename is the publication point; failure retains pending output and
original bytes. Exported/nested inputs refuse explicitly. No source cleanup.

Composer uses the roots writable/rebase guard, indexes and opens the returned
revision, keeps failed edits, and explains preservation in its existing controls.
Guide loader/session export, index inbox/Location scans exclude pending output.
Index owns cached first-kept guide posters; Gallery backfills legacy missing
posters at load. Index gains a direct internal guide dependency; no external
dependency or production process boundary added.

113 affected library tests passed, one pre-existing PDF test ignored; format
and clippy passed. Standalone actual-disk Rust example passed. Evidence:
`docs/proof/guide-edit-preservation/`. Initial plain test invocation lacked
scratch roots; corrected to the existing xtask isolation contract. Independent
review caught pending-reader bypasses, root identity/rebase, malformed session
events and legacy thumbnails; fixes were incorporated and exercised.

No application screenshots, Linux run or real recording. No commit/push/merge;
the parent/operator code checkouts remain untouched. Do not stage the unrelated
PowerShell cache under Microsoft/. Next: F2 Stop/session isolation with F7
pending-save recovery, followed by the other F3–F12 findings before claiming
watch/follow/execute reliability. Existing standards mapping remains qualified
only at its documented static/CLI scope.
