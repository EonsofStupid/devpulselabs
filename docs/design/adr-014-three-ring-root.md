# ADR-014 — The three-ring data root

| Field | Value |
|-------|-------|
| **Status** | ACCEPTED (operator ratification 2026-07-25; senior-review conditions 1–6 folded in) |
| **Owner** | `dpl-engine-capture` (the root owner) · pass plan `docs/plans/three-ring-root.md` |

## Decision

One portable application folder, three rings with hard boundaries:

| Ring | Folder | Contract |
|------|--------|----------|
| 0 — STATE | `.userdata\` | App state: `index.db` (Turso), `analytics.duckdb` (RING-2), `cache\`, `temp\`, `profiles\`. `cache\`+`temp\` are 100 % regenerable (deleting them can never lose work — the deletability contract is written into `.userdata\README.txt`); the DBs and profiles are precious. Never creative material. |
| 1 — WORKING | `assets\` | Raw + in-process, **auto-filed, zero dialogs**: `assets\captures\<kind>\<job>\…`. WiredFRONT (the PREP house) operates here. Shallow nesting by context — taxonomies calcify. |
| 2 — CONFIRMED | `content\` | The library universe, entered ONLY by an explicit operator gesture (KEEP / Save / Export). Indexed, portable, shareable. Workbench (the BUILD house) builds from here. Layout unchanged. |

Laws:

1. **Generation is automatic into Ring 1; promotion to Ring 2 is always
   explicit** — and transactional: copy + index write-through commit
   first, the assets original is removed only after success, and only
   when it lives under `assets\` (external sources are undeletable by
   construction). Failure direction = duplicate, never loss.
2. **Root-relative paths only** in the index, for every ring.
3. **Retention is a report, not a reaper** (RING-2/DuckDB): nothing
   auto-deletes operator material, ever.
4. **Additive migrations**: the index moved `content\artifacts.db` →
   `.userdata\index.db` by copy → verify → labeled rename
   (`.migrated-to-userdata`) — never deletion.
5. Rings resolve as siblings derived from `content_root()` (which keeps
   its arg → env → ambient → self-create chain); drill seams
   `ASSETS_ROOT` / `USERDATA_ROOT` mirror `CONTENT_ROOT`. Boot creates
   the rings and NAMES them in the log (the honesty law).

Precedent: Unreal `Saved`/`Intermediate`/`Content`, Resolve
cache/media-pool/gallery, Lightroom ingest/catalog/library. Embedded
Turso (metadata) + embedded DuckDB (analytics) is the right-sized
standalone stack — ClickHouse/Postgres rejected as servers in a
portable app.

## Rejected

- Everything-in-`content\` (working material silently pollutes the
  library; today's status quo failure).
- Tool-chosen locations (scratchpads, repo-root folders) — the drift
  this ADR exists to kill.
- Auto-promotion on capture — the library stays trustworthy only if a
  human confirms entry.
