# GF1a — preserve guides when applying review edits

2026-11-02. The operator's resume continues the correctness/preservation work
identified after GF0. This cut addresses F1 in the guided-workflow review;
capture barriers/recovery and Coach outcome fixes remain separate cuts.

## Grounding and contract

At the reviewed baseline, `engine-steps/src/edit.rs::update_guide` overwrites guide.json,
reconstructs only five root fields and deletes files using unvalidated old
manifest references. Composer's Apply is its only production caller.
The replacement remains owned by engine-steps; no parallel editor or workflow
engine. Apply creates a fresh sibling revision and never mutates the source.

1. Validate source manifest, old and submitted media references, and regular
   source entries before writing. Reject traversal, Windows alternate streams,
   links/reparse points and nonregular files. Validate directory ancestry too.
   The engine cannot promise protection against a hostile concurrent filesystem
   administrator swapping paths; document that boundary rather than claim it.
2. This API edits captions/order/kept steps only. Identify original steps by
   their unique original order; reject unknown/duplicate identities and media
   substitutions. Merge into original raw JSON so root and step extension
   metadata survive; clear a caption explicitly and renumber the new sequence.
3. Bound this cut to flat guide folders: reject nested attachments explicitly;
   copy every regular file, including pruned frames and unknown sidecars.
   Exclusively reserve a new sibling revision folder. A newly created
   `.guide-revision-pending` marker keeps it out of index scans, including when
   session.mp4/events.jsonl have been copied. Copy without overwriting and
   publish guide.json only after writes succeed. Finally rename the pending
   marker to `.guide-revision-complete` as the publication point. These atomic
   renames move only newly created staging metadata, never source content.
   Index ignores pending revisions; subsequent edits refuse pending sources.
   Completion marker is regenerated, not copied. Failed unpublished folders
   remain for diagnosis; retry reserves another folder. No source deletion.
4. Do not label old rendered derivatives as current after editing. For this
   bounded cut refuse exported inputs carrying guide.html or rendered guide/
   GIF references with a clear instruction to edit the source then export again.
   This preserves existing exported content and defers regeneration rather than
   silently invalidating metadata. Unknown attachments remain preserved.
5. Return a typed revision result containing path/count. Composer indexes and
   selects that result, reports source preservation, and retains the pending
   edits on failure. Update misleading deletion tooltips; keep layout/controls.
   Before writing, use the existing roots writable guard and its rebased source
   and root paths; unplug/replacement refusal preserves pending reviewer edits.
6. Reader audit: the common guide loader, optional session export and recursive
   linked-Location scanning also refuse pending revisions. Gallery resolves its
   thumbnail from the first retained manifest step through the public guide
   loader at indexing time (index gains a direct internal guide dependency).
   Gallery reads the resulting cached poster path, rather than a preserved/pruned
   step-001.webp or parsing manifests each paint. Legacy missing poster paths
   are filled once when Gallery rows load. Unknown sidecars remain byte-preserved;
   metadata preservation here covers Apply, not the existing narrower export serializer.

## Verification

Fault tests use unique, disposable fixtures only: malicious old references and
an outside sentinel; unsafe references on replay/still paths; unknown metadata;
reorder/prune/caption preservation; duplicate/missing source identity; original
bytes unchanged; reserved destination and retry behavior; a copy/publication
failure before manifest publication leaving no guide.json, and failure before
completion leaving a pending marker which readers refuse; static symlink/reparse
refusal where supported.
Run engine tests, affected Composer logic tests/check, format and clippy. A
Rust executable example exercises actual disk revision creation; Composer's
integration test separately checks typed loader compatibility. Neither records
or touches operator content. No pixel claim or full-app qualification from these
logic checks. Review independently before
recording completion; preserve GF0's files and existing unrelated work.
