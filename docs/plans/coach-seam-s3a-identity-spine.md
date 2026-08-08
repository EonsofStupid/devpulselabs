# Pass — Coach Seam S3a: the identity spine + pack registry

**Status:** LANDED 2026-08-08 · executes the first slice of the S3
re-prioritization (design ratified in
`docs/journal/2026-08-08-coach-seam-s1-spine-s3-design.md`; parent ADR
`coach-seam-adr.md` §3 + §7.4/§7.7). Operator ruling: "no more
hardcoding — identity resolved, never assumed."

## 1. Context

Today the Coach hub keys every app by `.desktop` basename (`DesktopId`),
the minted fabric dies with the process, and seed logic hardcodes
"cursor"/"calculator" strings. **No pack, profile, or identity
persistence exists in crates** — only in the ADR. Before the Foundry
(S3b) can build anything durable, there must be a durable identity to
build it FOR: a source-of-truth `AppId`, an evidence-based resolver that
aliases whatever is actually scanned onto that truth, and a pack
registry on disk that survives the process.

## 2. Goal / non-goals

**Goal:** a new egui-free crate `dpl-coach-pack` where:

- `AppId` is the minted-once source of truth (never derived at runtime);
- every scan's observations form a `FingerprintBundle`, and an
  evidence-weighted resolver scores that bundle against every installed
  pack's `[identity]` → the pack's `AppId`, or an honest miss;
- a miss synthesizes a generic profile under `x.unresolved:<slug>`
  (ADR §7.2 extension namespace), promotable later — the seam never
  refuses to look;
- packs are TOML files in a directory registry
  (`packs/<app-id>/pack.toml`), `schema = 1` + `min_seam`, unknown keys
  warn-never-fail AND are preserved on rewrite (ADR §7.4);
- the write path is content-addressed (`pack_hash` over the canonical
  body, ADR §7.7) and follows the document law (tmp → bak step-aside →
  rename);
- the promote-only law lives in types: a pack can only promote a verb
  ABOVE the default ladder; it cannot demote below it, cannot grant R3
  (vision never enters the lesson hot path), and the schema has no
  field that could bypass presence or effect gates.

All of it unit-testable without X11.

**Non-goals (later slices, by ruling):** Foundry v0 + hub UI (S3b) ·
Blender pack content (S4) · protocol rehoming of `AppId`/`PackId` into
`egui-pilot-ai-protocol` (S2 — S3a's newtypes are crate-local until the
rehoming pass) · pack signatures beyond the content hash (the `[signing]`
table carries the hash now; real signatures ride the remote-registry
pass) · anchors/`[[anchor]]` fingerprint bundles for UI nodes (Foundry
output, S3b/S4).

## 3. Ground truth (read in full, 2026-08-08)

- `crates/coach-inventory/src/desktop.rs` — `DesktopId` (basename
  newtype), `ColdEntry { id, display_name, exec_hint, startup_wm_class,
  path }`; `crates/coach-inventory/src/lib.rs` — `InventoryItem` adds
  `live: Option<LiveHint { atspi_name, pid, running }>`; the
  cold↔live join is heuristic string matching (`cold_matches_live`) —
  exactly the ad-hoc aliasing this pass replaces with scored evidence.
- `crates/coach-fabric/src/lib.rs` — `request_from_inventory_name`
  hardcodes cursor/calculator seed strings; mint output is in-memory
  only.
- `crates/ledger/Cargo.toml` + `src/lib.rs` — the root-`PathBuf`
  pattern (`LedgerHandle::open(root)`), egui-free, document-law writes.
- Workspace `Cargo.toml` — coach crates are members referenced by
  relative path from consumers (no `workspace.dependencies` entries);
  `toml = "1.1"`, `serde`, `sha2 = "0.11"` already workspace deps.
- `crates/xtask/src/main.rs` structure audit — `EGUI_FREE` census table;
  new crate must be added there + `architecture.md` §3 in the same
  commit.
- ADR §3 (pack TOML sections), §7.2 (`x.` namespace), §7.4 (schema
  version + tolerance + first-class cell key), §7.7 (content-addressed
  registry layout from day one).

## 4. Design — `dpl-coach-pack`

**Identity:**

```rust
pub struct AppId(String);   // parse-validated; two forms only:
                            //   resolved:  reverse-DNS ≥2 segments ("org.blender.Blender")
                            //   extension: "x.unresolved:<slug>" (§7.2 namespace)
pub struct CellKey { pub app_version: String, pub os: String, pub locale: String }
pub struct PackId(String);  // "<app-id>@<version>/<os>/<locale>" — the cell is
                            // first-class in the id from day one (§7.4)
```

`AppId::parse` rejects everything else — no bare strings for identity.
`AppId::unresolved(name)` sanitizes a scan-observed name into the slug.

**Evidence:**

```rust
pub struct FingerprintBundle {  // what a scan observes; all optional
    desktop_id, startup_wm_class, wm_class, atspi_name,
    exe_basename, exe_path, version: Option<String>,
}
pub struct SignalMatch { pub signal: &'static str, pub value: String, pub weight: f32 }
pub enum Resolution {
    Resolved { app_id: AppId, score: f32, matched: Vec<SignalMatch> },
    Unresolved { synthesized: Pack },  // app_id = x.unresolved:<slug>
}
```

Resolver weights (v1, recorded in code beside a census test):
desktop id 4.0 · StartupWMClass / WM_CLASS 3.0 · exe basename 2.0 ·
AT-SPI name 2.0 (name is ONE weighted signal, never the anchor — locale
law) · alias 1.0. Accept at score ≥ 3.0 — i.e. one strong signal or two
weak corroborating ones. Ties break toward the higher score; equal
scores toward the pack with more matched signals; still equal =
`Unresolved` (ambiguity is honest, never a coin flip).

**Pack (schema 1):**

```toml
schema = 1
min_seam = 1

[identity]
app_id = "org.blender.Blender"
display_name = "Blender"
desktop_ids = ["blender.desktop", "org.blender.Blender.desktop"]
wm_classes = ["Blender"]
atspi_names = ["Blender"]
exe_basenames = ["blender"]
aliases = []

[cell]
app_version = "4.2"
os = "linux"
locale = "en"

[ladder]              # per-verb rung promotions, promote-only
# open = "r0"

[[verb]]              # capability matrix rows (Foundry fills; S3a types them)
# verb = "open"; rung = "r1"; reliability = 0.97

[teaching]
# landmarks = []

[signing]
pack_hash = "sha256:…"   # computed by the writer over the canonical body
```

Unknown keys anywhere: collected as warnings, preserved verbatim on
rewrite (serde-flattened extras tables). `schema` newer than supported:
refused loudly (document law). `min_seam` newer than the current seam:
loads with `compatible = false` + warning — degrade, never crash.

**Promote-only in types:** the default ladder for every verb is
R1 → R2. A `[ladder]`/`[[verb]]` entry may name `r0` or `r1`;
`effective_rung(default, grant)` returns the grant only when it is
STRICTLY better than the default; `r3` is unrepresentable as a grant
(the loader refuses it by name). Presence/effect gates have no schema
surface at all — unrepresentable, not just checked.

**Registry:**

```rust
pub struct PackRegistry { root: PathBuf }   // root-agnostic, like the ledger
// open(root) · list() · load(&AppId) · write(&Pack) · resolve(&FingerprintBundle)
```

Layout `packs/<app-id>/pack.toml`; per-stage Foundry artifacts land
BESIDE the pack later (S3b) — the directory is the checkpoint unit.
`write` serializes the body without `[signing]`, hashes it (sha2),
inserts the hash, then tmp → bak step-aside → rename, LF. `load`
recomputes and warns on mismatch (hand-edits are visible, not fatal).
`FingerprintBundle::from_inventory(&InventoryItem)` is the one
constructor the hub will call in S3b (crate dep: `dpl-coach-inventory`,
consistent with fabric/runtime's dep direction).

## 5. Wiring

- Workspace member `crates/coach-pack` (grouped with the coach family in
  `Cargo.toml` members); consumers reference by relative path like every
  coach crate. Deps: `serde`, `toml`, `sha2`, `dpl-coach-inventory`.
- `crates/xtask/src/main.rs`: `dpl-coach-pack` joins the `EGUI_FREE`
  census.
- `docs/design/architecture.md` §3: new entry for `crates/coach-pack`
  (egui-free; may depend on inventory; consumed by `dpl-hub-coach` and
  the Foundry in S3b; never a dep of `app` directly) — same commit.

## 6. Proof

1. Unit (no X11): `AppId` parse laws (accept/reject table incl. the
   `x.` namespace) · resolver scores a Blender-shaped bundle to the
   Blender pack and a stranger bundle to `Unresolved` with a
   well-formed synthesized generic pack · ambiguous tie → `Unresolved` ·
   pack TOML round-trip preserves unknown keys and warns on them ·
   newer `schema` refused, newer `min_seam` degrades with warning ·
   promote-only: `r0` grant wins, `r2`-over-`r1` demotion refused, `r3`
   grant refused by the loader · write→load round-trip verifies
   `pack_hash`; a tampered body warns.
2. Live (this seat): a drill example scans the REAL inventory
   (`dpl_coach_inventory::inventory(true)`), builds a bundle per row,
   resolves each against a registry seeded with a real Blender pack
   written by this crate — Blender resolves to `org.blender.Blender`,
   gnome-calculator honestly synthesizes `x.unresolved:…` (no
   calculator pack exists yet), and the pack file on disk is inspected.
3. `cargo green` (full gate) before push; PR merges on double-green
   only.

## 7. Risks

- **Resolver weights are v1 guesses** — recorded as constants with a
  census test; the Foundry's battery (S3b/S4) is what will tune them
  with data. Honest now, data-driven later.
- **Crate-local newtypes vs S2 rehoming** — `AppId`/`PackId` move (not
  copy) into `egui-pilot-ai-protocol` at S2; no re-export shims per
  house law, so S2 carries the mechanical rename.
- **TOML extras preservation** relies on serde-flatten into
  `toml::Table` — comment/format fidelity is NOT preserved (TOML
  re-serialization drops comments). Recorded honestly: packs are
  machine-authored artifacts; hand annotations belong in the pack dir's
  notes file, not the TOML.

## 8. Evidence (2026-08-08)

- **Unit:** `dpl-coach-pack` 25 green on the first build — the `AppId`
  parse law table (incl. the reserved `x.` namespace and slug charset) ·
  bundle-from-inventory honesty (`live-only:` synthetics are NOT desktop
  ids; `wm_class` never invented) · signed round-trip with
  `hash_verified: Some(true)` and struct equality · tampered body warns
  and still loads · unknown keys at top level, inside `[identity]`, and
  as whole sections all warn AND survive a rewrite (reparse of the
  rewrite re-verifies its own hash) · `schema = 2` refused,
  `min_seam = 99` degrades with warning · promote-only: `r2`/`r3`
  ladder grants and an `r3` verb row refused by name (3 refusals
  counted), `effective_rung` never demotes · resolver: Blender-shaped
  bundle → `org.blender.Blender` at 9.0; one weak signal (2.0) below
  `ACCEPT_SCORE` stays unresolved; stranger synthesizes a well-formed
  promotable `x.unresolved:` pack; exact tie → `Unresolved` (never a
  coin flip); same-score-more-evidence wins · registry: write/load/list
  round-trip with `.bak` step-aside, `UnresolvedWrite` refusal, and a
  corrupt pack becoming a warning beside the good one — never blinding
  the resolver · the weights census pins the exact v1 signal list.
- **Live (this seat, DISPLAY=:1):** `resolve_drill` against
  `/tmp/seam-s3a-proof` — seeded Blender + Calculator packs through the
  crate's own write path (real signed `pack.toml` files on disk,
  inspected); scanned **65 real inventory rows**: the RUNNING
  gnome-calculator resolved to `org.gnome.Calculator` at score 8
  (`desktop_id` +4, `atspi_name` +2 — live AT-SPI evidence — and
  `exe_basename` +2), all 64 strangers synthesized honest
  `x.unresolved:` profiles (`x.unresolved:libreoffice-writer`,
  `x.unresolved:org-wezfurlong-wezterm`, …), and a Blender-shaped
  bundle resolved to `org.blender.Blender` at score 9. Blender itself
  is NOT installed on this seat — recorded honestly; installing it is
  S4 seat prep.
- **Gate:** `cargo green` full pass (fmt + clippy + tests + structure
  audit with the new `EGUI_FREE` entry + release exe). One clippy
  finding (`large_enum_variant` on `Resolution`) fixed by boxing the
  synthesized pack — no `#[allow]`, per law.
