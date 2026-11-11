# Recorder Stop/Save — GF1b evidence

Verified locally on Windows, 2026-11-02. This record covers the implementation in
`review/guided-workflow-standard`, following
[the plan](../../plans/recorder-stop-save.md). It does not describe a released build.

## Implemented contract

Generation-tagged bounded work prevents old triggers entering a new session.
Stop waits for in-flight capture/checkpoint work; a checkpoint already committing
finishes before acknowledgement so memory and recovery agree. Checkpoints precede
count updates. Pending sessions cannot be replaced by Start.

Save copies into a reserved unpublished destination, retains the source and retry
handle on failure, and records its prepared intent before publication. Recovery
finishes that transaction with the original metadata. OS leases exclude a second
writer. Discovery reports damaged sources alongside healthy recoveries, includes
empty checkpoints for archival, and avoids rereading historical completed frames.
Archive detaches a source without deleting it.

ViziWiz waits for Stop and Save off the paint thread, keeps the draft through
failure, locks prepared metadata, and offers interrupted-recording recovery.
Asynchronous indexing checks the captured library generation and authoritative
saved path; a library change defers indexing without undoing successful publication.

## Verification record

The six affected library suites passed **155 tests**, with one pre-existing
Composer PDF test ignored because it requires the bundled external PDF tool:

| Suite | Passed |
|---|---:|
| engine-steps | 45 |
| ViziWiz | 11 |
| guide | 10 |
| index | 24 |
| Composer | 64 |
| WiredFRONT | 1 |

The recorder tests include 11 lifecycle/barrier cases and 21 storage cases.
They cover queue pressure, cancelled/stale work, checkpoint and publication
failures, collision refusal, source leases, partial recovery scans, prepared
metadata, and actual publication through the native guide loader and repeated
preserving edits. Four new ViziWiz tests cover acknowledgements, draft retention,
repeated toggles and successful publication with deferred indexing.

Tests used fresh isolated CONTENT_ROOT, ASSETS_ROOT and USERDATA_ROOT values.
Capture backends are injected and tests never install OS hooks.

```text
cargo test -p dpl-engine-steps -p dpl-viziwiz -p dpl-guide -p dpl-index -p dpl-composer -p dpl-hub-wiredfront --lib --offline --target-dir ../public-repository/target
cargo clippy -p dpl-engine-steps -p dpl-viziwiz -p dpl-guide -p dpl-index -p dpl-composer -p dpl-hub-wiredfront -p devpulselabs --all-targets --offline --target-dir ../public-repository/target -- -D warnings
cargo fmt -p dpl-engine-steps -p dpl-viziwiz -p devpulselabs -- --check
```

All passed. Clippy also type-checked the native app targets; this is not an app
launch or pixel test. The structure audit passed (11 ceilings, 41 egui-free
packages, 77 members, 51 flags, 77 crates and 17 binaries), and `git diff --check`
passed.

The actual Rust executable `recorder_recovery` also passed, using
`cargo run -p dpl-engine-steps --example recorder_recovery --profile test --offline --target-dir ../public-repository/target`.
[Captured output](drill.json) records source preservation, matching copied bytes,
stopped recovery, successful native guide loading, completed publication and no
duplicate recovery after restart. Its fixture uses explicitly synthetic frame
bytes and is retained under the worktree's ignored `target/` for inspection.

## Limits

- No live recording, native-window screenshots, Linux execution or performance
  qualification is claimed by this record.
- Stop cannot preempt an OS capture call. It waits for a truthful acknowledgement;
  the native UI does not wait on that call.
- Reservation, root/path-guard and checkpoint failures pause recording. The
  existing native capture API returns untyped errors for capture/encoding failures;
  those report a skipped attempt and may continue recording.
- Source retention includes skipped/cancelled attempts. Full immutable checkpoint
  snapshots currently make retained checkpoint metadata grow quadratically with
  step count. An append-only step journal and user-directed retention remain
  follow-up work; recorder storage is not yet size-optimized.
- Event sinks must remain queue-only and must not reenter lifecycle methods.
  Arbitrary rebinding of the one process-wide hotkey sink is unqualified.
- Filesystem checks do not defend against hostile concurrent path replacement,
  nor establish a universal power-loss guarantee across filesystems.
- This cut does not fix the separate do/done target-selection issue (F9), provide
  shared watch/follow/execute outcomes, or establish protocol conformance.
