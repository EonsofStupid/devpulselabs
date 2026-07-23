# X1 — dpl-index: the rebuildable artifact index

Parent: docs/plans/roadmap.md Program X (census #2). Ledger #107.
Worktree: `C:\localdev\dpl-main` on main. Engine-only pass — the Gallery
and inbox-review UI are X2.

## 1. Context

Captures land in `content_root()/inbox` (pix stills, vix videos, guide
folders) and vanish from sight — the census's #2 missing capability and
the operator's "no way of seeing any of my previous test" pain, now in
the rewrite. X1 ports the old repo's index engine: **files + sidecars
are the TRUTH; the DB is a queryable index, rebuildable any time.**
This also unblocks X2 (Gallery), W5 (guide reviewer), and the Steps
pipeline (T4/G1).

## 2. Current state (ground-truthed this session)

- **Old repo `commands/db.rs`** (read in full): `artifacts` table —
  rel_path UNIQUE, location, kind, name, show_id?, episode_folder?,
  tags JSON, source, target?, seconds?, captured_at, ai_labels JSON,
  cleared_at?, + migrated poster_rel_path?, size_kb?, modified_ms?.
  Laws: upsert ON CONFLICT updates + `cleared_at=NULL` (reactivate);
  rekey (keep → content location); dump = SOFT clear (row survives for
  history); additive best-effort migrations; corrupt/missing DB
  recreated, never data loss; DB file at the data root; the old engine
  was turso — and the OPERATOR RATIFIED TURSO for the rewrite too
  (2026-07-22, overriding the roadmap's rusqlite note): the pure-Rust
  engine, same as the proven old implementation; its async API wraps in
  a small current-thread tokio runtime inside dpl-index.
- **Old wire shape** (`ArtifactRecordOut`, 15 fields): rel_path,
  location, kind, name, show_id?, episode_folder?, tags[], source,
  target?, seconds?, captured_at, ai_labels[], poster_rel_path?,
  size_kb?, modified_ms?.
- **Census content-raw laws** #34–41 + #53: sidecar law (`.meta.json`
  companions travel with media), reindex from disk+sidecars with
  soft-clear of vanished rows and an EMPTY-SCAN GUARD, db_update
  rewrites inbox sidecars, ai_labels reserved (no producer — never
  synthesize).
- **New repo**: `dpl_pixvix` saves `pix-*.png` / `vix-*.mp4` /
  `guide-*/` folders into `cap::content_root().join("inbox")` with
  never-clobber names (`unique_path`); `dpl_engine_steps` owns
  `CaptureMeta` + the forced-metadata law (`normalize_meta_auto`) and
  writes `guide.json`; its header note says "Sidecar + artifact index
  arrive with the PV1c pass" — this pass. NO sidecars, NO db today.
  `/content` is gitignored operator data (never committed).

## 3. Target design

**New crate `crates/index` (`dpl-index`)** — pure engine, no egui:

- `Record` — the 15-field shape above, `serde` (sidecars are JSON).
- `Index::open(path) -> Result<Index>` — turso 0.7 (pure Rust, local
  file; async API wrapped behind a sync surface via a current-thread
  tokio runtime + fresh connection per op, the old repo's exact
  pattern), creates
  the table + runs additive best-effort migrations; a corrupt file is
  renamed `.corrupt` and recreated (files remain the truth).
  `Index::open_default()` = `content_root()/artifacts.db`. (The wider
  data-root law is Program P; noted, not solved here.)
- Ops, each mirroring the old law: `upsert(&Record)` (reactivates),
  `rekey(old_rel, new_rel, show_id, episode_folder)`,
  `soft_clear(rel)`, `set_tags(rel, tags)` (+ rewrite the inbox
  sidecar), `set_ai_labels(rel, labels)` (reserved entry — no producer
  wired, NEVER synthesized), `recent(limit, location?)`,
  `list(kind?, location?)` — active rows only (cleared_at IS NULL).
- **Sidecars** (`sidecar` module): `write(path, &Record)` /
  `read(path)` — `<file>.meta.json` beside media; a guide FOLDER's
  sidecar is `<folder>/artifact.meta.json` (guide.json stays the steps
  manifest, untouched).
- **`reindex(root: &Path)`**: walk `inbox/` (files + guide folders;
  skip sidecars/transients), merge each find with its sidecar (sidecar
  wins for authored fields; fs supplies size_kb/modified_ms/kind),
  upsert all; rows whose file vanished → soft_clear. **Empty-scan
  guard**: a scan that finds ZERO artifacts while the DB has active
  rows aborts with an error instead of mass-clearing (an unmounted
  root must never erase history). Returns `ReindexReport { indexed,
  cleared }`.
- **CLI** `src/bin/dpl-index-cli.rs`: `reindex`, `list [kind]`,
  `recent [n]` — the headless drill the pilot law asks for at the
  engine tier.

**Write-through**: `dpl_pixvix` gains a `dpl-index` dep; every save
path (pix still, vix video, guide auto-file) builds a `Record` +
sidecar and upserts — errors degrade to the status line (capture NEVER
fails because indexing did).

Scale story: a new capture vehicle = build a Record at its save site;
the index/Gallery/reviewer all read the same 15 fields. Reindex is the
repair path for anything (hand-copied files included).

## 4. Changes, in execution order

1. `crates/index/` — Cargo.toml (turso 0.7, tokio rt, serde, serde_json),
   `src/lib.rs` (Record, Index, sidecar, reindex), `src/bin/
   dpl-index-cli.rs`. Tests (tempdir-seeded): open+migrate twice
   (idempotent), upsert/recent round-trip, reactivate-on-upsert,
   rekey, soft_clear hides from list but survives in the file,
   set_tags rewrites the sidecar, reindex indexes a seeded inbox
   (pix + guide folder), reindex soft-clears a vanished file,
   the EMPTY-SCAN GUARD refuses, corrupt-db recreates.
2. Root Cargo.toml: member + workspace dep. deny.toml: the turso tree —
   extend the allowlist ONLY if the gate names a gap.
3. `crates/pixvix`: on each successful save, build Record
   (kind pix→"image", vix→"video", guide→"guide"; name from the file
   stem / meta; source "pixvix"; captured_at now-UTC; size/modified
   from fs) + `sidecar::write` + `Index::open_default().upsert` —
   best-effort, error → status line.
4. `cargo green`; CLI drill on a seeded temp root (reindex → list
   shows the seeds); commit; push (backgrounded).

## 5. Boundaries

🚫 No Gallery/review UI (X2), no keep/dump filing (X2), no
associate/import (X3), no AI-label producer (never synthesize).
🚫 guide.json format untouched. 🚫 Old repo read-only; Codex checkout
untouched. ⚠️ timestamps: captured_at from std::time (SystemTime→ISO
via chrono? NO new dep — format manually from SystemTime epoch).

## 6. Verification

`cargo green` exits 0 with the new suite. CLI drill (real bin, seeded
tempdir): `dpl-index-cli reindex` reports the seeded counts;
`recent` lists them; deleting a file + reindex soft-clears exactly it;
an empty root + populated DB refuses. Exe smoke: a `--pixvix-snap`
capture upserts a row (assert via CLI `recent` naming the new pix).

## 7. Commit plan

`feat(index): X1 — dpl-index: the rebuildable artifact index + sidecars + CLI`
on main; push backgrounded.
