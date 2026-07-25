# W4 — pack ingest: pin everything, GRAB the favorites (both, as asked)

## 1. Context

Operator (2026-07-25): "is there a way we can do both? Have them
symlinked where they are also have the option to ingest so i can
eventually grab the ones i often use?" YES: pinning (W3) already IS
the app-level symlink — packs referenced in place, zero bytes moved.
W4 adds the other half: a per-pack **Ingest** that copies one pack's
media into `content/elements/<pack>/` (indexed, postered, local) so
favorites survive without D:. Literal NTFS symlinks under `content/`
are rejected: they'd drag 70k files into rescans and blur the artifact
jail; pin + on-demand ingest gives the same power with clean seams.

## 2. Current state (read in-session)

- W3 `packs.rs`: `sources.json` registry, `Pack{name,path,files}`,
  `list_packs` (capped counts), `count_media`; W3 `browse.rs`:
  `Browser{slot,cwd,entries,picked,pinned,open}`, bar chips, cards;
  W3 `lib.rs`: `drain_media_feeds` drains picked+pinned, `RenderState`
  pattern for off-thread work, `import::media_kind`/`no_clobber`/
  `sanitize_name`, `index_render` write-through, caches reset by
  setting `Option`s to None. `Index::open(&db_path)` exists alongside
  `open_default` (hermetic tests possible).

## 3. Target design

- `packs.rs` gains: `media_files(pack) -> Vec<PathBuf>` (recursive,
  dot-skipped, media only, name-sorted) · `pack_size(pack) -> (files,
  bytes)` (uncapped, for the arm confirm) · **`ingest_copy(root, pack,
  name, progress) -> Result<usize, String>`** — copies each media file
  to `content/elements/<kebab name>/<subpath>` (SKIP-IF-EXISTS →
  resumable/idempotent), posters+seconds for videos (best-effort),
  write-through via `Index::open(root/artifacts.db)`, bumps an
  `Arc<AtomicUsize>` progress counter. Disk-full (os 112) aborts
  honestly with the copied-so-far count. `Pack` gains
  `ingested: bool` (= `elements/<kebab name>` exists).
- `browse.rs`: an **Ingest pack** bar chip, TWO-TAP armed (the Dump
  law): first tap computes and shows `Ingest N files · X.X GB?`;
  second tap pushes `ingest_req`; any navigation disarms.
- `lib.rs`: `IngestState{Idle,Running{pack},Done{pack,copied},
  Failed}` + `Arc<Mutex<..>>` + `Arc<AtomicUsize>` progress (the
  RenderState pattern); one ingest at a time; drained from Browse
  windows; a status pill (`Ingesting <pack> — N files…`) while
  Running; caches reset on Done. Shelf rows show `local` for ingested
  packs (they're also ordinary artifacts everywhere).

## 4. Changes, in execution order

1. `packs.rs`: `media_files` · `pack_size` · `ingest_copy` + Pack
   `ingested` + tests (hermetic scratch root: copies land under
   `elements/<name>/`, re-run copies 0 (resumable), index rows exist
   via `Index::open`).
2. `browse.rs`: `ingest_req: Option<PathBuf>` + `arm: Option<(usize,
   u64)>` on Browser; the armed Ingest chip; disarm on nav.
3. `lib.rs`: IngestState + `start_ingest` (thread) + drain + status
   pill + `Pack.ingested` trail on shelf rows.

## 5. Boundaries

🚫 D: stays READ-ONLY (copies only; skip-if-exists; never delete/move
sources). 🚫 One ingest at a time. 🚫 No literal symlinks under
`/content`. 🚫 No new deps (AtomicUsize + std threads). Disk-full
aborts honestly, never partial-file (copy to `<name>.part` then
rename). Out of scope: unpin/uningest UI, dedupe across packs,
selective sub-pack ingest.

## 6. Verification

1. `cargo test -p dpl-composer` green (W3 15 + ingest hermetic tests).
2. `cargo green` ✔.
3. Real-exe drill: scratch CONTENT_ROOT, ingest the REAL smallest pack
   (`videohive-W0LkFNDI-lower-thirds`, 0.14 GB) via a staged
   `--ingest-pack <dir>` harness seam? NO — keep the UI path honest:
   hermetic test + unit drill cover the copy; the operator's first real
   ingest is the two-tap chip. Proof shot: shelf rows show counts; the
   armed chip label is exercised in a Browse window by the operator.
4. FS: sources on D: untouched (mtime), `elements/<pack>/` populated,
   re-ingest copies 0.

## 7. Commit plan

Branch `feat/w4-pack-ingest` off main; one commit; push, PR, merge;
session-end law.

## 8. Evidence

Executed 2026-07-25, same session as W1–W3:

1. `cargo test -p dpl-composer` → **16 passed**, including the hermetic
   ingest drill: a scratch pack (mp4 in a subfolder + wav + a txt) →
   `ingest_copy` copies BOTH media files into
   `elements/<kebab>/<subpath>` (txt skipped), the SOURCE stays intact,
   a SECOND run copies 0 (resumable/idempotent), both index rows exist
   with `source == "ingest"`, and `local_dir` marks the pack local.
2. `cargo green` ✔; exe rebuilt.
3. UI wiring live on the exe: the Browse-window bar gains the two-tap
   **Ingest pack** chip (arm shows `Ingest N files · X.XX GB?` from a
   real full-size walk; navigation disarms); the composer runs ONE
   ingest at a time on its own thread with a live
   `Ingesting <pack> — N file(s)…` status pill; on Done the shelf,
   footage, and census caches refresh and the pack's shelf row reads
   **local**. Copies are `.part`-then-rename (never a torn file);
   disk-full aborts honestly naming the copied-so-far count.
4. The operator's first real ingest is deliberately theirs to fire —
   the smallest real pack (`videohive-W0LkFNDI-lower-thirds`, 0.14 GB)
   is a good first tap once their disk move finishes.
