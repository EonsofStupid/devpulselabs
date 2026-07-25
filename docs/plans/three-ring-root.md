# RING-1 — the three-ring data root (`content` · `assets` · `.userdata`)

Operator ratification (2026-07-25): generated content was landing
outside the app ("ALL generated content is to be organized and stored
inside the application… frictionless like my original"); structure
ratified as the two visible boundaries + app state — `assets/` (working)
promoting into `content/` (confirmed), with `.userdata/` (Turso index,
DuckDB analytics later, cache, temp, browser profiles). Senior-review
conditions 1–6 accepted as this pass's acceptance criteria. ADR:
`docs/design/adr-014-three-ring-root.md`.

## §1 Context

The app is portable; the original's frictionless flow files every
artifact inside the app automatically. Today only Ring 2 exists
(`content/`), so tools without a seam (this session's browser captures)
invent locations. This pass creates Ring 0 + Ring 1, moves the index
into Ring 0 additively, gives captures an auto-filing seam, and makes
promotion (assets→content) a transactional one-call gesture.

Editor mapping (operator, verbatim intent): WiredFRONT = PREP house,
operates on `assets/`; Workbench = BUILD house, reads `content/`.

## §2 Current state (ground-truthed this session)

- `crates/engine-capture/src/lib.rs:97-164` — `content_root()`:
  `--content-root` arg → `CONTENT_ROOT` env → ambient candidates
  (exe-adjacent → ≤4 parents → cwd) → self-create beside exe; OnceLock
  once-per-process; `checked_roots()` honesty surface. THE root owner —
  `dpl-index` and `dpl-composer` both resolve through it.
- `crates/index/src/lib.rs:80-114` — `Index::open(path)` (corrupt
  quarantine + rebuild), `open_default()` = `content_root()/
  artifacts.db`, Turso (`turso::Builder::new_local`).
- `crates/composer/src/import.rs:55-101` — `import_file(src)`: copy
  into `content/inbox/` (elements → `content/elements/imported/`),
  no-clobber, poster+seconds, `Index::open_default().upsert` — the ONE
  seam. Sources never mutated.
- `crates/engine-browser` (this branch): ephemeral profiles in
  `std::env::temp_dir()`; `browser_cli shot` writes wherever told —
  both now ring-law violations to fix.
- `crates/engine-capture/Cargo.toml` — deps xcap/image/webp (light
  enough to be depended on for root resolution; `dpl-index` already
  does exactly that).
- `.gitignore` — has `/content`, plus this session's `demo-captures/`
  (to be retired by this pass).

## §3 Target design

Siblings beside the exe (dev: beside `content/` at the repo root),
derived from `content_root()`'s parent:

```
content\    Ring 2 — CONFIRMED: the library universe (unchanged layout)
assets\     Ring 1 — WORKING:  assets\captures\<kind>\<job>\…  (auto-filed)
.userdata\  Ring 0 — STATE:    index.db · cache\ · temp\ · profiles\
```

- Ring 1/0 resolve in `dpl-engine-capture` (the root owner):
  `assets_root()` / `userdata_root()`, env seams `ASSETS_ROOT` /
  `USERDATA_ROOT` (mirroring the `CONTENT_ROOT` drill contract),
  else derived: `content_root().parent()/assets|.userdata` (a rootless
  content root nests them inside itself — degenerate case only).
- `ensure_rings()` creates `assets\`, `.userdata\{cache,temp,profiles}`
  + a `README.txt` in `.userdata` stating the deletability contract
  (condition 2: cache/temp = always regenerable; index/profiles =
  precious). App boot calls it and logs all three rings (the
  `checked_roots` honesty law).
- **Index migration (condition 6, additive):** `open_default()` →
  `.userdata\index.db`; when absent and `content\artifacts.db` exists:
  copy → verify-open → rename old to `artifacts.db.migrated-to-userdata`
  (labeled retirement, never deletion). Old-path callers: none besides
  `open_default` (grep in §8).
- **Promotion (condition 1, transactional):** `composer::import::
  promote_asset(src)` = `import_file(src)` (copy + index committed)
  then remove the ORIGINAL only when it lives under `assets_root()` —
  external sources can never be deleted by construction. Worst-case
  failure direction = duplicate, never loss.
- **Captures auto-file (the frictionless law):**
  `assets_capture_dir(kind, job)` → created `assets\captures\<kind>\
  <job>\`. `browser_cli shot <url> --job <job> [--name <stem>]` files
  there no-clobber; explicit `<out.png>` stays for harness use.
  Ephemeral browser profiles → `.userdata\temp\browser\`; `login`/
  `--profile` accept a bare NAME → `.userdata\profiles\<name>`.
- Index stays the catalog of Ring 2 only; Ring-1 discovery is
  filesystem truth (Browse windows already open folders). Cataloging
  assets in the index (`location='assets'`) is a flagged follow-up.
- Paths in the index remain root-relative (condition 3) — unchanged.
- DuckDB analytics (`.userdata\analytics.duckdb`) is pass RING-2 with
  its own C-dep ADR (the ADR-012 pattern); condition 5's stale-assets
  report ships there. Never smuggled into this diff.
- Flagged cleanup (not this pass): extracting root resolution into a
  `dpl-root` crate is a no-shim move-pass touching every consumer.

## §4 Changes, in execution order

1. `crates/engine-capture/src/lib.rs`: add `assets_root()`,
   `userdata_root()` (OnceLock + env seams + pure derivation fns),
   `assets_capture_dir(kind, job)`, `ensure_rings()`. Unit tests on the
   PURE derivations (sibling-of-content; degenerate rootless case;
   capture-dir nesting) — the cached statics stay untested in-process,
   same as `content_root` today.
2. `crates/index/src/lib.rs`: `open_default()` per §3; migration logic
   as a pure `fn adopt_legacy(old: &Path, new: &Path)` unit-tested with
   temp files (old-only → migrated+labeled; both → new wins; neither →
   fresh).
3. `crates/composer/src/import.rs`: `promote_asset(src)` + pure guard
   `fn removable_after_promote(src, assets_root)` with unit tests;
   ignored FS drill `composer/tests/promote_drill.rs` (fixture asset →
   promote → in inbox + indexed + gone from assets).
4. `crates/engine-browser`: dep `dpl-engine-capture`;
   `fresh_profile_dir()` → `.userdata\temp\browser\`; CLI: `--job`/
   `--name` auto-filing, bare-name profiles, local no-clobber helper.
5. `crates/app/src/main.rs`: boot `ensure_rings()` + one tracing line
   naming all three rings (beside the `init_ffmpeg()` call site).
6. `.gitignore`: add `/assets/` + `/.userdata/`; drop `demo-captures/`.
7. Move `demo-captures\demo-001-round1\*` → `assets\captures\web\
   demo-001\round-1\`; delete the empty `demo-captures\`.
8. Docs: ADR-014; `docs/plans/demo-guide-pipeline.md` capture-plan
   paths; `docs/STATE.md` tip.

## §5 Boundaries

- 🚫 `/content` layout and the import seam's destinations: unchanged.
- 🚫 Never delete `content\artifacts.db` (labeled rename only).
- 🚫 No DuckDB dep in this diff (RING-2 + ADR).
- 🚫 `crates/lottie` + parked Cargo lines: untouched, as before.
- ⚠️ UI surfacing (assets shelf in WiredFRONT, promote chips) is its
  own pass under the experience gate — this pass is the data spine +
  CLI/harness seams.

## §6 Verification

1. `cargo test -p dpl-engine-capture -p dpl-index -p dpl-composer
   -p dpl-engine-browser` — new units green.
2. `cargo test -p dpl-composer --test promote_drill -- --ignored` —
   live promote proof.
3. `cargo green` exit 0.
4. Real exe boot: rings created beside `content\`, boot log names all
   three; `content\artifacts.db` becomes `.migrated-to-userdata` with
   `.userdata\index.db` present; Gallery still lists prior artifacts
   (index survived the move).
5. `browser_cli shot https://example.com --job smoke` files under
   `assets\captures\web\smoke\` with no path given.
6. Demo-001 stills present under `assets\captures\web\demo-001\
   round-1\`; `demo-captures\` gone.

## §7 Commit plan

Branch `feat/three-ring-root` stacked on `feat/demo-browser-driver`
(uses its CLI); one commit, message `RING-1 — the three-ring data
root: assets/.userdata + additive index migration (ADR-014)`; PR after
#41 merges (rebase to main). Push runs the gate hook.

## §8 Evidence

- `crates/engine-capture/src/lib.rs:97-164,1381-1398` (resolution +
  candidate tests) — read this session.
- `crates/index/src/lib.rs:76-120` (open/quarantine/open_default).
- `crates/composer/src/import.rs:1-101` (the seam, read in full).
- `crates/engine-capture/Cargo.toml` (dep weight check).
- Grep `artifacts.db` across crates at execution time confirms
  `open_default` is the only path constructor (verified in step 2).
