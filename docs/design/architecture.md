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
                   dpl-lottie · dpl-guide · dpl-intent · dpl-ledger · dpl-svg
                   (IO + subprocess leaves per ADR-009; egui-free; dpl-proc,
                   dpl-intent, and dpl-svg are pure LAW crates, ∅ deps;
                   dpl-ledger writes Parquet via pure-Rust `parquet`+`snap`;
                   dpl-svg rides as coldlight's BUILD-dep — icon geometry
                   is generated, never parsed at runtime, ADR-015)
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

`dpl-chat-window → dpl-workflows` is an ordinary surface→document edge
(the `dpl-editor → dpl-doc` direction): the chat surface renders the
brainstorm board document and emits its typed ops; the host owns all IO.

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
| dpl-svg | ∅ | the icon-subset SVG law is dependency-free — parsing, flattening, and stroking are owned (ADR-015) |

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
- **crates/desktop-spike/**: orphan spike probes (never a dep of `app`);
  workspace member; Linux-gated AT-SPI/x11rb. Operator may delete later.
- **crates/coach-overlay/** (`dpl-coach-overlay`): ratified **devtools-only**
  overlay process (ADR `coach-overlay-adr.md`); never a dep of `app`;
  Linux X11 backend first. Production builds must not wire it.
- **crates/coach-inventory/** (`dpl-coach-inventory`): C2d lightweight
  target inventory (XDG cold catalog + AT-SPI desktop-root census);
  egui-free; never deep-walks; consumed by `dpl-hub-coach` (never by
  `app` directly).
- **crates/coach-fabric/** (`dpl-coach-fabric`): C2e budgeted Tier A
  fabric mint for one selected target; egui-free; may depend on
  inventory; consumed by `dpl-hub-coach` (never by `app` directly).
- **crates/hubs/coach/** (`dpl-hub-coach`): Operator Coach Import UI
  (C2f); depends on inventory + fabric + runtime; spawns
  `dpl-coach-overlay` binary only (never a crate dep on overlay).
- **crates/coach-pack/** (`dpl-coach-pack`): S3a pack registry (seam
  ADR §3/§7.4/§7.7): fingerprint bundles, evidence-weighted resolver,
  schema-1 TOML packs under a root-agnostic `packs/<app-id>/pack.toml`
  registry, promote-only law in types; egui-free; may depend on
  inventory + `egui-pilot-ai-protocol` (S2 rehomed the identity spine —
  `AppId`/`CellKey`/`PackId` — and the transport rung into
  `egui_pilot_ai_protocol::seam`; this crate imports them, no re-export
  shims); consumed by `dpl-hub-coach` + the Foundry (S3b); never a dep
  of `app` directly.
- **crates/coach-runtime/** (`dpl-coach-runtime`): FabricGraph bridge
  (session resolve / invoke / extents + reality presence); egui-free;
  may depend on fabric + inventory; never a dep of `app` directly. The
  verb CATALOG is not here — S2 rehomed it to
  `egui_pilot_ai_protocol::seam::seam_verb_catalog()`; this crate is
  the engine behind the verbs, not the vocabulary.
- **crates/coach-daemon/** (`dpl-coach-daemon`): localhost Axum
  TRANSLATOR onto the protocol seam (ADR `coach-control-plane-adr.md` +
  S2 `coach-seam-s2-protocol-rehoming.md`); never a dep of `app`; agent
  hop = HTTP; app hop = runtime AT-SPI; overlay remains painter-only
  under overlay ADR. May depend on `egui-pilot-ai-protocol` (verbs
  parse to `seam::SeamVerb` at the HTTP edge; answers are
  `seam::EffectContract`; `/events` speaks `seam::SeamEvent`) +
  `dpl-fabric` + `dpl-ledger` (seam ADR S1): every verb
  execution/refusal publishes a seam span on a daemon-local signal
  fabric and the ledger writes it to the same Parquet layout the app
  uses — the files are the meeting point between the two processes.
  (`dpl-fabric`'s private `TransportRung` copy is held to the protocol's
  wire labels by a daemon test until its own retirement micro-slice —
  S2 plan §4.1 deviation.)
- **Teacher-dev T0** (program `docs/plans/coach-teacher-dev.md`):
  `egui_pilot_ai_protocol::seam` grows `TeacherMode` / `GoalId` /
  `TeacherGoal` / `TeacherEvent` (sibling to observation `SeamEvent`,
  not merged with it). `dpl-fabric` mirrors a serde-free `TeacherFact`
  as `SeamEvent::Teacher` — **no** new protocol edge on fabric (same
  law as `TransportRung` / `SeamAgency`). `dpl-ledger` projects those
  as `teacher.*` row types on the existing `seam` domain;
  `seam_reliability` skips the prefix so a goal/step cannot become a
  fake verb. Capability `seam.teacher-dev` is honest-Unsupported on
  the pilot pipe. No new crate. Clyffy/hub/daemon untouched this cut.

## 4. The laws every new crate inherits

- **One concern.** The lib.rs doc-header names it in one sentence; if the
  sentence needs "and", it is two crates.
- **Naming**: `dpl-` prefix; kebab ids; grouped folders (`chat/*`,
  `hubs/*`) when a family exceeds ~3 crates. Stated exactly, because this
  law was prose-only until 2026-07-30 and drifted where nobody looked: a
  crate is `dpl-` + its folder; a BINARY is kebab-case; a `src/bin/*.rs`
  file is snake_case with its binary name DECLARED (`[[bin]] name` +
  `path`). The declaration is not ceremony — cargo auto-discovers an
  undeclared file and the FILENAME becomes the shipped binary name, which
  is how `browser_cli` shipped underscored while every sibling was kebab.
  RATIFIED exceptions, listed beside the audit that enforces this:
  `devpulselabs` (the exe), `xtask` (cargo's own convention), and the
  `egui-pilot-ai*` family, which stays unprefixed as its own product
  surface (operator ruling 2026-07-30 — "not everything has to be dpl-").
  A new exception amends this bullet and the audit table in the SAME
  commit.
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
list, both directions) · the naming law (crate = `dpl-` + folder · kebab
binaries · snake `src/bin` sources, with §4's ratified exceptions).

Sibling laws: `doctrine.md` (treatments/paint) · `design-system.md`
(composable UI model) · `signal-doctrine.md` (facts) · ADR-009
(subprocesses) · the two-libraries law (`wiredfront-experience.md` §7).
