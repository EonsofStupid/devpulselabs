# Recorder Stop, Save, and recovery — GF1b

Status: implemented and locally verified, 2026-11-02. Continues the approved guided-workflow review, findings F2 and F7, after GF1a. No protocol or production process boundary changes. [Evidence and limits](../proof/recorder-stop-save/README.md).

## Grounding

The current recorder uses one unbounded queue of untagged triggers. Stop changes a flag without waiting for capture; a worker may still write and announce recording afterward. Start deletes an unresolved source before metadata validation. Save takes the session before fallible file operations and renames its source. These transitions must be repaired together. ViziWiz currently calls Stop synchronously on the paint thread and already retains its landing draft after Save errors.

## Contract

- Serialize lifecycle transitions. A typed generation identifies queued capture work. Use a bounded per-recorder queue and nonblocking hook submission; manual submission reports queue pressure. Reject stale generations before capture, between capture phases, and before committing a step.
- Stop invalidates the generation, waits for the in-flight capture gate, and acknowledges only when capture cannot commit again. ViziWiz waits in a background worker and displays Stopping until acknowledgement. Repeated toggles cannot start a new session while Stop is pending.
- If Stop arrives after checkpoint publication has begun, that already-committing step finishes before acknowledgement. Its durable checkpoint and in-memory count must agree. Stop during capture/settling cancels the uncommitted attempt. Automatic stop after a persistence error must also open the Save/Archive path, including zero-step sources.
- Event sinks are queue-only and must not call lifecycle methods. Status reentry is supported. Emit accepted recording-state events within the capture barrier, after durable checkpoint publication; emit Stop state before acknowledging. Never hold the session mutex during a callback.
- Start refuses unresolved pending work. Source directories are exclusively reserved, hidden, recorder-owned folders. Every capture attempt receives a fresh path, including cancelled/unchanged captures. No source assets are removed or overwritten.
- Publish an immutable versioned checkpoint before advancing the committed count. Recovery uses only completed checkpoints, validates flat relative assets and recorder ownership, and refuses malformed complete checkpoints. Interrupted staging remains available for diagnosis. Recover into a stopped session.
- Reservation/checkpoint failures pause recording, invalidate queued work, and report the storage error with the last durable count. Transient capture-backend errors may skip a capture and continue.
- A per-source OS file lease excludes ordinary concurrent app instances/recorders. Session clones retain the lease; closing/crashing the owner releases it. Discovery reads existing lock files without creating source content, skips active owners, and reports per-source issues alongside healthy candidates. Recovery reacquires the lease and revalidates before returning a handle.
- Save requires a stopped session. Copy validated assets to an exclusively reserved destination, gate it with the existing guide pending marker, sync its manifest, and publish only when complete. Keep the source and retry handle on every failure.
- Guide Save runs off the paint thread because copying large captures can be slow. Retain and lock the draft until acknowledgement; block conflicting capture/archive actions. The existing app drill waits for both Stop and Save. Indexing after publication must use the saved target and current library identity, not accidentally write into a newly selected library.
- Persist a source receipt after destination preparation and before final publication. Retry/recovery resolves the same prepared destination; a completed receipt prevents a second published copy after restart. Refuse changed save metadata while a prepared transaction needs completion. No source deletion on Save or Discard; Discard archives the checkpoint and detaches only after archival succeeds.
- ViziWiz provides a refresh/recover affordance for owned interrupted recordings in the current writable inbox. Recovery never starts OS recording automatically. Root writable guards must rebase removable-drive paths before filesystem mutation.
- A prepared receipt supplies the original Save metadata on restart and after an in-session failure. The card retains these fields and retries the same transaction without adding current policy tags.
- Discovery classifies already-completed receipts from their ownership/publication metadata without rereading historical frame bytes. Pending publication still receives full asset verification. Include valid zero-step checkpoints for archival, but ignore aborted Start folders with no complete checkpoint.

## Delivery boundaries

Native Rust only. No live OS-hook use in automated tests; inject a capture backend/barriers for deterministic tests. No automatic replay, standards conformance claim, F9 capture-target fix, universal filesystem power-loss guarantee, or defense against hostile concurrent path replacement. Existing GF1a pending-guide reader/index gates are reused. Archived source retention is intentional and may require future user-directed cleanup.

## Verification

Exercise stale queued work across sessions, full queues, Stop while capture is blocked and during settling, event ordering, pending Start refusal, failed checkpoint, Save copy/manifest/publication failure and retry, destination collisions, recovery and completed-receipt deduplication, malformed/unsafe checkpoint refusal, and preservation of source bytes. Test ViziWiz acknowledgement/draft retention where practical. Run affected Rust tests, clippy/fmt, structure/docs gates, and a real disk recovery/publication example. Any native screenshot or live recording evidence is reported separately; fake-backend and disk tests do not prove OS hooks or pixels.
