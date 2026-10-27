# ADR-014 — The three-ring data root

| Field | Value |
|-------|-------|
| **Status** | ACCEPTED (operator ratification 2026-07-25; senior-review conditions 1–6 folded in) · **AMENDED 2026-08-03: analytics is Arrow/DataFusion over Parquet, not DuckDB** (operator ruling; see Amendment below) |
| **Owner** | `dpl-roots` (ST1, portable storage); earlier pass `docs-history-2026-10-12:docs/plans/three-ring-root.md` |

## Decision

One portable application folder, three rings with hard boundaries:

| Ring | Folder | Contract |
|------|--------|----------|
| 0 — STATE | `.userdata\` | App state: `index.db` (Turso), the RING-2 analytics **Parquet estate** (queried in-process by DataFusion), `cache\`, `temp\`, `profiles\`. `cache\`+`temp\` are 100 % regenerable (deleting them can never lose work — the deletability contract is written into `.userdata\README.txt`); the DBs and profiles are precious. Never creative material. |
| 1 — WORKING | `assets\` | Raw + in-process, **auto-filed, zero dialogs**: `assets\captures\<kind>\<job>\…`. WiredFRONT (the PREP house) operates here. Shallow nesting by context — taxonomies calcify. |
| 2 — CONFIRMED | `content\` | The library universe, entered ONLY by an explicit operator gesture (KEEP / Save / Export). Indexed, portable, shareable. Workbench (the BUILD house) builds from here. Layout unchanged. |

Laws:

1. **Generation is automatic into Ring 1; promotion to Ring 2 is always
   explicit** — and transactional: copy + index write-through commit
   first, the assets original is removed only after success, and only
   when it lives under `assets\` (external sources are undeletable by
   construction). Failure direction = duplicate, never loss.
2. **Root-relative paths only** in the index, for every ring.
3. **Retention is a report, not a reaper** (RING-2 analytics): nothing
   auto-deletes operator material, ever.
4. **Additive migrations**: the index moved `content\artifacts.db` →
   `.userdata\index.db` by copy → verify → labeled rename
   (`.migrated-to-userdata`) — never deletion.
5. `dpl-roots` resolves the library through the explicit CLI/environment
   drill seam or `roots.toml` beside the executable. No parent walk, CWD
   search or silent empty fallback. Boot creates rings only after resolving
   the selected library, and names them in the log (the honesty law).

## Amendment 2026-10-25 — portable library identity and layout (ST1)

R287–R289 and the approved portable-storage plan put the library's identity,
configuration and layout in the egui-free `dpl-roots` engine. A configured
library is identified by `library.json` (`v`, typed `id`, `label`, `created`),
not by a remembered drive letter. `roots.toml` beside the executable records
imported (`library/` beside the program) or linked mode, identity, label and
active account; linked mode records the last path. Both formats are versioned.
Invalid, missing, mismatched or ambiguous identity never selects another
library or creates a replacement. Looking for a marker does not write files.

New libraries have `common/{cache,temp}` and
`accounts/me/{content,assets,.userdata,.aidata}`. The three ring meanings
above stay intact; regenerable shared files use `common`. Creation requires
an explicit first-launch choice and never overwrites a marker or roots file.
Composer state writes to `.userdata/composer-state.json`; an existing
content-root state file is a read-only fallback until migration.

Existing operator trees are not automatically changed. The explicit
`--content-root` / `CONTENT_ROOT` dev and test seams retain sibling ring
derivation and `ASSETS_ROOT` / `USERDATA_ROOT` overrides, and never persist
configuration. ST4 owns account switching and the shown, verified migration
of existing files, including chat/analytics into `.aidata`. ST2 owns automatic
polling, live disconnection, reopening long-lived stores and retained edits.
ST1's stateful resolver alone is not a guarantee of unplug-safe active IO.

`cargo xtask stage-roots <program-dir> <existing-library>` creates a linked
configuration only after validating the existing marker. It refuses an
existing roots file and never adopts an unmarked legacy content tree.

## Amendment 2026-10-26 — a build's project; today's folders move in (ST1b)

R291 ("stored inside the project properly … you let the person decide"):
- **The program folder of a build is its project.** An executable at
  `<project>/target/<profile>/` whose `<project>/Cargo.toml` is a file keeps
  `roots.toml` and an imported `library/` in `<project>`, out of reach of
  `cargo clean`. Exactly those two fixed levels are checked; nothing walks.
  A copied or released executable is unchanged.
- **Today's rings move in once, on the person's tap.** The first screen's
  "Bring in my existing library…" plans the move read-only — each of
  `content/`, `assets/` and `.userdata/` under a chosen folder, with its
  files and bytes — then copies them to `accounts/me/{content,assets,
  .userdata}` of a new library the person places, compares every file byte
  for byte, and publishes `account.json`, `library.json` and `roots.toml`
  last. Links, a destination that holds files, and a library inside a ring
  (or the reverse) are refused. On any failure the move removes what it made.
  The source is only read: the old folders stay exactly as they were.
- `cargo xtask move-library --dry-run <source> [<library>]` prints the same
  plan. The move itself has no command-line form.

ST4 keeps account switching and the split of `.userdata/` into `.userdata/`
and `.aidata/`.

## Amendment 2026-10-27 — a library that isn't connected (ST2a)

- **`roots.toml` remembers where a linked library reappears.** Found on
  another drive letter, the new `last_path` is written back. The new text
  goes to `roots.toml.new` and is renamed over the old file only once
  complete, so a failure leaves the old file whole. An imported library and
  a seam never write it.
- **While a linked library is away, the app looks for it** every two
  seconds, off the UI thread: the remembered path, then the same path on
  every other drive letter or mount point — never a deep scan. When it is
  found, the app opens with no tap. A probe that read an older `roots.toml`
  than the person's latest choice is never installed.
- **The person can point at it (Locate…) or choose another library.** That
  replaces `roots.toml` only after they confirm; the library on the drive is
  only read.

Precedent: Unreal `Saved`/`Intermediate`/`Content`, Resolve
cache/media-pool/gallery, Lightroom ingest/catalog/library. Embedded
Turso (metadata) + in-process Arrow/DataFusion over Parquet (analytics)
is the right-sized standalone stack — ClickHouse/Postgres rejected as
servers in a portable app.

## Amendment 2026-08-03 — analytics is DataFusion, not DuckDB

**Operator ruling:** *"we will be removing ducksdb and be using arrow data
fusion as well instead."*

The original text named embedded DuckDB as RING-2. Ground truth says the
codebase had already chosen the other half of that stack:

- `dpl-ledger` **already writes Parquet through Arrow** — `ArrowWriter`,
  `RecordBatch`, `arrow-array`/`arrow-schema` 59 (`crates/ledger/Cargo.toml`).
- Nothing in `crates/` ever linked DuckDB. It was a decision on paper with
  a Parquet-emitting ledger underneath it.

So this is alignment, not a pivot: **DataFusion queries the Parquet the
ledger already emits**, in-process and pure Rust, which keeps ADR-009's
spirit (no C++ engine embedded in a portable app) and removes a native
dependency the build never took on.

What does NOT change: the ring boundaries, root-relative paths, the
transactional promotion law, and law 3 — retention stays a report. The
query engine changed; the contract did not.

**Downstream:** `docs-history-2026-10-12:docs/plans/coach-lab-experiment-program.md` names "DuckDB
baselines" for Lab analytics. That plan is owned by another workstream —
flagged here rather than edited, so its owner rules on it.

## Rejected

- Everything-in-`content\` (working material silently pollutes the
  library; today's status quo failure).
- Tool-chosen locations (scratchpads, repo-root folders) — the drift
  this ADR exists to kill.
- Auto-promotion on capture — the library stays trustworthy only if a
  human confirms entry.
