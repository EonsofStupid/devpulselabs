# Pass — one content universe: embedded, imported, linked

## 1. Context

**Operator ruling, 2026-08-05:**

> "we are in pre-release and need to get this actually aligned to best
> practices and get this properly authored so we can actually have content be
> a seamless mixture of content we embed, import, or symlink once we run it on
> a system"

Three provenances, one library. Today the system has the *mechanisms* (a
single import seam, symlink references, a three-ring root) but no **notion of
provenance**, and its dev root lands somewhere disposable. The consequence was
observed, not theorised: the operator asked what 13 files in their content root
were, and neither of us could tell from the filesystem which were real imports
and which were drill litter — because nothing records where a thing came from.

This pass makes provenance a first-class fact and puts the root somewhere a
`cargo clean` cannot reach.

## 2. Current state (read in-session 2026-08-05)

### 2.1 Root resolution — `crates/engine-capture/src/lib.rs:114-215`

```rust
pub fn content_root() -> PathBuf            // :114, OnceLock, once per process
fn resolve_content_root() -> PathBuf        // :119  explicit → first ambient that EXISTS → ./content
fn explicit_root() -> Option<PathBuf>       // :136  --content-root / --content-root= / CONTENT_ROOT
fn ambient_candidates() -> Vec<PathBuf>     // :151  exe dir, then ≤4 parents, then ./content
pub fn checked_roots() -> Vec<String>       // :173  the honesty surface
pub fn assets_root() / userdata_root()      // :191/:201, siblings of content_root
fn resolve_ring(env, name)                  // :207  ASSETS_ROOT / USERDATA_ROOT, else sibling
```

**The ordering is `.find(|p| p.exists())` — first EXISTING candidate wins.**
From `target\release\devpulselabs.exe` the candidates are, in order:

| # | candidate | today |
|---|---|---|
| 1 | `target/release/content` | **wins** — created by drill runs |
| 2 | `target/content` | — |
| 3 | `<repo>/content` | **already a candidate**, has never existed |
| 4 | `C:/localdev/content` | — |
| 5 | `C:/content` | — |
| 6 | `./content` (cwd) | — |

So the dev root sits inside the build-output directory **only because drills
created it there first**. The repo-root location the operator would expect is
already in the search path and simply loses the race. Most of the dev half of
this problem is therefore DATA, not code.

The three rings (ADR-014) are `content/` (ring 2, the library universe),
`assets/` (ring 1, working/auto-filed) and `.userdata/` (ring 0, state:
index, cache, temp, profiles). Rings 1 and 0 derive as SIBLINGS of ring 2, so
moving the content root moves all three.

### 2.2 The import seam — `crates/composer/src/import.rs`

The ONE door (verified this session): `import_file` copies video/image into
`content/inbox/`; `import_reference` links WITHOUT copying, so a source on an
external drive or share is never moved, renamed or written to. `media_kind`
classifies by extension — `aep|mogrt|cube|ffx|prfpset` are `element` (filed,
not parsed).

`crates/index/src/lib.rs` `jail_inbox` returns a symlink **as the link**
(`symlink_metadata`, does not follow), so removing a reference deletes the
link and never the target. That fix landed earlier this session.

### 2.3 What is missing

- **No provenance field anywhere.** A `Record` cannot say whether it was
  embedded, imported or linked, so the UI cannot show it, the index cannot
  filter it, and deletion cannot be governed by it.
- **No embedded ring.** `docs/design/release-contract.md` rules that fonts,
  icons and tokens are embedded and the depot ships WITH the release, but
  there is no read-only content ring for shipped starter material.
- **Drills and the operator's library share one root.** Every proof run this
  session deposited comps into the same `inbox/` a real import would use.
- **Nothing records that `<repo>/content` is the intended dev root** — it is
  an accident of resolution order.

**MUST GROUND BEFORE CODING** (not verified this session, low context):
`Record`'s exact fields and its persisted schema version in
`crates/index/src/lib.rs`; whether any migration path exists for the index;
`release-contract.md`'s exact current wording on the depot.

## 3. Target design

### 3.1 Provenance is a fact the system records, not a guess

```rust
/// WHERE a library item came from — the thing that decides what may be done
/// to it. Recorded at intake, never inferred from the filesystem afterwards.
pub enum Provenance {
    /// Shipped with the app. Read-only: it may be COPIED into the library
    /// but never edited or deleted, and it survives a reinstall.
    Embedded,
    /// Copied into `content/` by the one import seam. The library owns the
    /// bytes; deleting it deletes them.
    Imported,
    /// Referenced in place — an external drive, a share, another folder.
    /// The library owns only the LINK. Deleting it deletes the link and
    /// NEVER the target, and it can be OFFLINE.
    Linked { target: PathBuf },
}
```

Seamless to browse, distinct in behaviour. One library view, one search, one
drag-onto-timeline; provenance shows as a quiet badge and governs three
things only: **deletability**, **offline state**, and **portability**.

### 3.2 The root, made non-disposable

Resolution keeps its shape (explicit → ambient → cwd) — it is already correct
for a shipped app, where the exe sits beside its rings. Two changes:

1. **`target/**` is REFUSED as an ambient candidate.** A content root inside
   the build output is never intentional; it is the drill accident above.
   `ambient_candidates()` skips any candidate with a `target` component, so
   resolution falls through to `<repo>/content` on a dev run. This is a
   two-line filter and it is what stops `cargo clean` from ever being able to
   eat the library.
2. **The chosen root and WHY is logged at boot**, extending `checked_roots()`
   — which already exists as the honesty surface — to report the winner and
   the reason it won.

Shipped behaviour is unchanged: beside a real exe there is no `target`
component, so ambient resolution picks the exe-adjacent ring exactly as ADR-014
rules.

### 3.3 Drills stop writing into the library

Drill and proof runs get `CONTENT_ROOT` pointed at a scratch root under the
session temp dir. The harness already speaks `CONTENT_ROOT` (`explicit_root`),
so this is a launch-argument change, not an engine change. **A drill that
writes into the operator's library is a bug**, and the pass adds a test that
the drill flags never resolve to the ambient root.

### 3.4 The embedded ring

A fourth, READ-ONLY location: starter content shipped with the release
(sample comps, a demo clip, the element starter set). It is not a fourth ring
on disk — it is a provenance, resolved from an app-relative `embedded/`
directory beside the exe and marked `Provenance::Embedded` at index time.
Using one copies it into `content/` first, so the shipped set is never mutated
— the same "sources are never mutated" law the import seam already follows.

### 3.5 What "seamless" means concretely

| | Embedded | Imported | Linked |
|---|---|---|---|
| shows in the library | ✅ | ✅ | ✅ |
| searchable / taggable | ✅ | ✅ | ✅ |
| drag to timeline | ✅ | ✅ | ✅ |
| can be deleted | ❌ read-only | ✅ deletes bytes | ✅ deletes the LINK only |
| can be offline | ❌ | ❌ | ✅ shows offline, relinkable |
| survives reinstall | ✅ | ✅ | ✅ if the target still exists |

## 4. Changes, in execution order

1. **Delete the 13 drill files** in `target/release/content/` — verified this
   session to be drill output (all timestamped inside today's proof window,
   all four comps referencing the same `inbox/drill-clip.mp4`).
2. **`crates/engine-capture`** — `ambient_candidates()` skips any path with a
   `target` component; `checked_roots()` reports the winner + reason.
   Tests: a fake exe under `…/target/release/` resolves to the repo-root
   candidate; a fake exe NOT under `target` still resolves exe-adjacent.
3. **`crates/index`** — `Provenance` on `Record`, defaulting to `Imported` for
   existing rows. **Bump the persisted schema version and provide the
   migration** (ground the current version first).
4. **`crates/composer/src/import.rs`** — `import_file` stamps `Imported`,
   `import_reference` stamps `Linked { target }`. No new door.
5. **Deletion governed by provenance** — `Embedded` refuses; `Linked` removes
   the link only (the `jail_inbox` behaviour already landed, now made
   explicit by type rather than by path inspection).
6. **Offline state for `Linked`** — a row whose target is missing shows
   offline and offers relink, rather than vanishing or erroring.
7. **The embedded ring** — resolve `embedded/` beside the exe, index as
   `Embedded`, copy-on-use.
8. **Drill scratch** — every drill/proof flag launches with `CONTENT_ROOT`
   under the session temp dir; test that no drill resolves to the ambient root.
9. **Docs** — ADR-014 amended with the `target` refusal and the embedded
   provenance; `release-contract.md` states what ships in `embedded/`;
   `docs-history-2026-10-12:docs/STATE.md`; journal.

## 5. Boundaries

**🚫 Never touch**
- The operator's real content, wherever it resolves. Step 1 deletes ONLY the
  13 files proven to be drill output; anything else is theirs.
- Source files behind a `Linked` record — never move, rename, write or delete
  a target. Referencing promised not to copy; removal must not delete.
- The ONE import seam. Provenance is stamped INSIDE it; a second door is not
  created for embedded content (it copies through the same seam).
- Generated `*_gen.rs`; the vendored Lucide pool.

**⚠️ Ask first**
- Any change to `content_root()`'s ORDER beyond the `target` refusal — the
  shipped-app contract depends on it.
- A default embedded starter set — what ships is a product decision.
- Moving the operator's existing library anywhere.

**Out of scope**
- ColdLight presets (that pass is done and merged into this branch).
- RTFX `.ffx`/`.aep` parsing — still filed, not parsed; a separate pass.
- Thumbnail caching and the folder-add browse UX.
- The estate/Fjall store.

## 6. Verification

- `cargo test -p dpl-engine-capture` — the two resolution tests above.
- `cargo test -p dpl-index` — provenance round-trips through the persisted
  schema; an old row reads back as `Imported`.
- **Break-it:** point `CONTENT_ROOT` at a path inside `target/` explicitly →
  it is HONOURED (explicit always wins; only AMBIENT refuses `target`), and
  boot says so. This asserts the refusal did not become a prohibition.
- `rg 'content_root\(\)' crates` — no caller bypasses the seam.
- **End-to-end on the real exe:**
  1. `cargo clean` in a scratch clone → the library still resolves and is
     intact. *(This is the whole point of the pass.)*
  2. Import a file → shows `Imported`, deletable.
  3. Link a file from another drive → shows `Linked`; delete it; **assert the
     source file still exists on disk**.
  4. Rename the link target → the row shows OFFLINE and offers relink.
  5. Use an embedded item → it copies into `content/`, and the embedded
     original is unchanged.
  6. Run every drill flag → `target/release/content` is NOT recreated.
- Proof shots of the library showing all three provenances at maximized and
  `--window 1100x700`.

## 7. Commit plan

ONE branch, ONE PR, merged only on **double-green**, branch deleted after.
`gh pr list` first. Ordered commits: drill litter removed → ambient refusal →
provenance on Record + migration → seam stamps → deletion/offline rules →
embedded ring → drill scratch → docs.

**Land the current branch first.** It is 31 commits ahead of `origin/main`
and unmerged; stacking a structural pass on top of an unreviewed one is how
the double-green law gets violated by accident.

## 8. Evidence

*(filled during execution — real command output, deviations named explicitly)*
