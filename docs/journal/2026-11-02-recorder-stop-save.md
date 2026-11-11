# GF1b — recorder Stop, Save and recovery

Operator resume continued F2/F7 from the GF1a handoff in
`D:/localdev/devpulselabs/.worktrees/guided-workflow-review`, branch
`review/guided-workflow-standard`, baseline main `711fc72`.

## Delivered

Plan: [recorder-stop-save](../plans/recorder-stop-save.md). Runtime concerns now
live in `engine-steps/src/lifecycle.rs`; persistence in `storage.rs`, each with
separate tests. Work is bounded and tagged by generation. Start/Stop events are
ordered; Stop cancels uncommitted attempts and waits for capture/checkpoint
completion. A checkpoint already committing finishes before acknowledgement.
Reservation/path/checkpoint errors pause without discarding source files.

Exclusive hidden sources, immutable checkpoints/attempt reservations and OS
leases support stopped restart recovery. Save copies, records prepared metadata,
publishes behind the existing guide marker, and only then clears the handle.
Failures retain source and retry state. Discovery skips active writers, reports
bad sources alongside healthy ones, includes empty checkpoints for archival,
and classifies completed receipts without rereading historical frame bytes.
Save and Archive never delete source content.

ViziWiz has background Stop/Save acknowledgements, recovery controls, draft
retention, prepared-field locking and archive handling. Repeated toggles cannot
reset a draft. `save_with_path` returns the actual publication path; queued Save
and later indexing check the library generation. Successful publication remains
successful when indexing must be deferred. The existing app drill waits for
both asynchronous transitions. No production endpoint/process boundary added.

## Evidence

[Run record and actual drill output](../proof/recorder-stop-save/README.md).
155 affected library tests passed; one pre-existing PDF test ignored. Counts:
engine 45, ViziWiz 11, guide 10, index 24, Composer 64, WiredFRONT 1. This includes
real native guide loading and repeated preserving revisions after recorder
publication. The executable disk recovery example passed with its synthetic
fixture retained under `target/`. Clippy with warnings denied passed for all
affected crates and native app targets; fmt, structure and diff checks passed.

Independent review drove fixes for initial event ordering, source leases,
per-capture root guards, canonical manifest compatibility, prepared metadata,
partial discovery, completed-scan cost and asynchronous Save indexing. A consumer
compile caught the recovery button's PilotPath/PilotId mismatch; widening the
private helper to the existing Into<PilotPath> contract fixed it before the
final green test run.

## Limits and NEXT

No live OS recording, app screenshots, Linux run or performance qualification.
Native capture/encode errors still share a string boundary and may skip/continue;
the reservation/path/checkpoint pause behavior is specifically qualified.
Full immutable checkpoints retain quadratic metadata as step count grows;
append-only journaling and user-directed retention are follow-up work. Event
sinks remain queue-only. Do not claim universal filesystem power-loss or hostile
path-swap protection, shared watch/follow/execute outcomes or standards adoption.

NEXT: ground and fix F3 (Coach's production/devtools overlay boundary), followed
by F4–F6 predicate/locator/scope correctness. F8–F12 remain open, including capture
fidelity, success verification, re-export, performance and redaction review.
Plan before code and preserve the current artifact evidence.

No commit, push, merge or release rebuild. GF0, GF1a and prior research remain
uncommitted alongside this cut. Preserve parent/operator checkouts and other
lanes. `Microsoft/Windows/PowerShell/ModuleAnalysisCache` is tool cache, not product
content. The parent ops resume index is updated separately after the final docs
gate; this worktree contains the implementation and evidence.
