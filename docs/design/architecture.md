# ARCHITECTURE — the layer law

**Status:** LAW — enforced by `cargo xtask structure` (a `cargo green`
phase). Authored from the real dependency graph 2026-07-28 (main
`2e7bcea`); every claim here traces to a manifest, never to intent.
**Change protocol:** amending a layer, ceiling, or exception = amend this
file AND the audit table in `crates/xtask/src/main.rs` in the SAME
commit, with the why. The gate fails until both agree.

## 1. The layer stack (dependencies point DOWN, never up or sideways)

```
authored data      tokens/*.json · dptools-deps/sources.*.toml
   ↓ build.rs / xtask (generated consts, staged depots — never hand-edited)
DOCUMENT crates    dpl-doc · dpl-comp · dpl-workflows        (pure serde data;
                   schema_v + newer-refusal + crash-safe tmp/bak writes)
ENGINE crates      dpl-proc · dpl-engine-{capture,media,steps,ae,browser} ·
                   dpl-index · dpl-estate · dpl-landing · dpl-tools-deps ·
                   dpl-lottie · dpl-guide · dpl-intent        (IO + subprocess
                   leaves per ADR-009; egui-free; dpl-proc and
                   dpl-intent are pure LAW crates, ∅ deps)
SUPPORT crates     dpl-coldlight (tokens+paint) · dpl-vector · dpl-video ·
                   dpl-backdrop                               (egui-typed helpers)
SURFACE crates     dpl-slides · dpl-editor · dpl-deck · dpl-composer ·
                   dpl-viziwiz · dpl-chat-window/-windows/-renderers
                   (egui; ALL paint via ColdLight — no raw hex, no ad-hoc frames)
HUB crates         crates/hubs/* — one line each in the app registry;
                   a hub NEVER depends on another hub
SHELL → APP        dpl-shell → devpulselabs (the only crate that wires all)
side rails         dpl-fabric (signals) · dpl-seam (agent HTTP) ·
                   egui-pilot-ai-* (semantic harness) · xtask (automation)
```

`chat/*` mirrors the same shape internally: `dpl-chat-core` is the pure
document/domain layer; providers and accounts are engines; window/windows/
renderers are surfaces.

## 2. Ceilings (exact sets, gate-enforced)

| crate | direct deps are EXACTLY | why the ceiling exists |
|---|---|---|
| dpl-proc | ∅ | the quiet-spawn law is dependency-free |
| dpl-comp | serde, serde_json | AI-editable comp truth stays pure data |
| dpl-workflows | serde, serde_json, dpl-comp, dpl-doc | workflows are documents, not code paths (D3); wire contract `schemas/*.v1.schema.json`, pinned by test (dev-dep jsonschema — dev-deps sit outside this ceiling; see workflow-doctrine.md) |
| dpl-doc | serde, serde_json, dpl-coldlight | slide truth + the ONE styling edge (below) |
| dpl-tools-deps | serde, serde_json, sha2 | the depot resolver trusts nothing else |
| dpl-landing | serde, serde_json | the landing policy is shared pure data |
| dpl-estate | fjall, moka, serde, serde_json | Ring-0 persistence, no app types |
| dpl-intent | ∅ | the operator-intent proof must never learn to serialize, clone, or arrive over a wire (signal-ledger plan §9.3) |

Adding an edge to these crates is a DECISION: amend the table here + in
the audit, same commit, with the reason.

## 3. Standing exceptions (recorded, not hidden — each removal is its own pass)

- **dpl-doc → dpl-coldlight**: `Look`/`resolve` speak the token
  vocabulary (fonts, inks, moods). The document REFERENCES tokens; it
  never paints. (Pulling the token IDS into a paint-free crate would
  remove this edge — a ratified later pass, not a drive-by.)
- **dpl-seam → dpl-coldlight**: the envelope serializes tone/token ids.
- **dpl-fabric → dpl-hub + dpl-chat-core + dpl-intent**: facts carry Route,
  provider types, and the intent gate's mode/cause/channel. The fabric is a
  side rail, not a layer violation; reusing the gate's types avoids a
  mirrored vocabulary.
- **dpl-video / dpl-vector / dpl-backdrop are egui-linked**: they are
  SUPPORT crates (players, paint math), deliberately above the engine
  line.
- **dpl-lottie-render lint override**: the ONE FFI leaf carries a
  per-crate `[lints.rust] unsafe_code = "allow"` (the C API demands
  `unsafe`; every other crate inherits the workspace deny) — allowlisted
  in the audit's hygiene check.
- **crates/desktop-spike/**: an orphan spike folder outside the
  workspace; allowlisted in the audit. Deleting it is the operator's
  call.

## 4. The laws every new crate inherits

- **One concern.** The lib.rs doc-header names it in one sentence; if the
  sentence needs "and", it is two crates.
- **Naming**: `dpl-` prefix; kebab ids; grouped folders (`chat/*`,
  `hubs/*`) when a family exceeds ~3 crates.
- **Census law**: anywhere a LIST of things exists (templates, workflows,
  stages, coach ids, harness flags), ONE registry function enumerates it
  and a census test (or the structure audit) fails when reality and the
  list drift. Nothing else enumerates.
- **Document law**: persisted JSON carries `v` (schema version); `load`
  refuses newer; `save` is pretty-LF via tmp → bak-step-aside → rename.
- **ADR-009**: external capability = subprocess/service behind a seam
  (ffmpeg, pdftoppm, aerender, ThorVG, ComfyUI). `-sys` FFI links are
  allowed in exactly one leaf: `dpl-lottie-render`.
- **Surfaces paint ColdLight only**: color from tones/ramps, size from
  Fluid, no blur, hard shadows (doctrine.md owns the mechanics).
- **Signals**: mutations publish on the ONE fabric, stamp-at-publish,
  batching laws (signal doctrine owns the mechanics).

## 5. New-crate checklist (copy into the PR description)

1. Layer named (§1) and the member entry placed in that order in the
   workspace `Cargo.toml`.
2. `version.workspace` / `edition.workspace`; effective `publish = false`;
   `[lints] workspace = true`.
3. lib.rs doc-header: the one concern + its dep ceiling in a sentence.
4. Inline `#[cfg(test)] mod tests` (no separate test crate unless it
   drills a real exe).
5. If the crate is a document/engine crate: add its ceiling or egui-free
   census entry to the structure audit + §2/§3 here, same commit.
6. If it registers things: the registry fn + census test land WITH the
   first registered thing.

## 6. The audit (what fails the gate)

`cargo xtask structure` (runs inside `cargo green` after the spawn
audit): ceiling exact-sets (§2) · egui-free census · no lateral hub deps ·
`-sys` only in dpl-lottie-render · manifest hygiene (lints + publish) ·
no orphan crate folders (allowlist §3) · harness-flag census (flags
parsed in `crates/app/src/main.rs` + `--content-root` ⇔ CLAUDE.md's
list, both directions).

Sibling laws: `doctrine.md` (treatments/paint) · `design-system.md`
(composable UI model) · `signal-doctrine.md` (facts) · ADR-009
(subprocesses) · the two-libraries law (`wiredfront-experience.md` §7).
