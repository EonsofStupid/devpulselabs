# 2026-08-11 — Coach seam S2: the vocabulary comes home (protocol v2.7)

**Lane:** coach-seam · **Trigger:** operator `gfo` on the S2 plan
(`docs/plans/coach-seam-s2-protocol-rehoming.md`, drafted 2026-08-08,
law-reading ratified 2026-08-10). **Branch:**
`feat/coach-seam-s2-protocol`, built in an ephemeral worktree
(`../devpulselabs-s2`) off `main`.

## Why a worktree

At execution time the operator's live `feat/coldlight-53-hues` lane held
uncommitted edits to `crates/{app,fabric,ledger,seam}` in the one
checkout. SESSION lane law: do not steal. The worktree isolates S2
completely — and it forced one honest design call (below).

## What landed

1. **`egui_pilot_ai_protocol::seam`** (protocol v2.6 → v2.7, additive):
   - Identity spine MOVED from `dpl-coach-pack` — `AppId`, `CellKey`,
     `PackId`, `IdError`, `UNRESOLVED_PREFIX`, `slugify`; `id.rs`
     deleted, callers import from the protocol (no re-export shims —
     the 2026-08-10 law reading: Law 7 protects content, moves go to
     the respective boundary properly). The ADR §2.1 refinement held:
     these are validated newtypes, wire `Deserialize` goes THROUGH
     `parse`, so the wire cannot smuggle an invalid id.
   - `SeamVerb` — the ratified catalog as a typed enum, 21 rows:
     the 2026-08-01 twenty **plus `geom`** (shipped since 2026-08-01,
     uncataloged until now; `window_geom` lives on as a parse alias,
     never emitted). `Extension { SeamExtensionId, params }` carries
     the `x.<namespace>.<verb>` incubator arm.
   - `SeamVerbDescriptor` + `seam_verb_catalog()` (replaces
     `coach-runtime/src/verbs.rs` — deleted after a census test proved
     catalog ⊇ a literal copy of the old rows, name/status/target
     identical: no silent drops, in code).
   - `TransportRung` (`r0`..`r3`), `SeamTarget`, `EffectContract`
     (typed before/after observables + open `extra` map),
     `SeamEvent` (typed `/events` facts), `WindowGeom`.
   - Capabilities `seam.verb` / `seam.events` / `seam.pack-attach`
     registered honest-`Unsupported` on the pilot pipe with evidence
     `coach-seam-s2` — the testing-vs-coaching separation the operator
     demanded is now a visible, queryable fact in the capability
     catalog: the pilot pipe drives EGUI targets; foreign-app verbs
     live behind the daemon.
2. **`dpl-coach-pack` consumes the protocol**: its `Rung` retired for
   the protocol `TransportRung` (TOML bytes `"r0"`/`"r1"` unchanged —
   an S3a-shaped fixture parses + rewrites byte-identically, tested);
   `PromotedRung`/`effective_rung` stay pack-domain (promote-only law
   is not wire vocabulary).
3. **Daemon demoted to translator** (~the 800-line HTTP wrapper is now
   honest): `SeamVerb::parse` at the HTTP edge (unknown verbs 409 BY
   NAME), every `/verb` answer an `EffectContract`, `/verbs` derived
   from `seam_verb_catalog()`, `/events` emitting typed `SeamEvent`s.
   Effect structs in the runtime map into contract observables +
   `extra` per the plan's §4.5 disposition table.

## The one deviation, named

Plan §4.1 wanted the protocol to own the ONE `TransportRung` with
`dpl-fabric` taking the protocol dep (the pack.rs l.16 promise).
`crates/fabric/src/lib.rs` was dirty in the operator's live lane, so
the plan's own fallback shipped instead: fabric keeps its private copy,
the daemon carries a `fabric_rung` mapper + a test pinning both enums
to one wire vocabulary (`r0`..`r3`). Recorded in the plan §4.1 landing
note + architecture.md. **A micro-slice is owed**: retire the fabric
copy and the pinning test once coldlight lands.

## Proof

- Tests: protocol 29 unit + 5 integration (v1-compat now asserts v2.7)
  · coach-pack 22 · daemon 2. Full `cargo green`.
- **The battery is a committed drill** — the 2026-08-01 manual curl
  matrix became `coach-daemon/examples/verb_battery.rs` (`ureq`
  dev-dep, examples-only). Live run this seat (DISPLAY=:1,
  gnome-calculator): **25 PASS / 1 SOFT / 0 FAIL — GREEN**, including
  SSE observe with a concurrent mover and TWO typed refusals (`drag`
  planned, `teleport` unknown). Report:
  `docs/proof/coach-verb-battery/REPORT-2026-08-11-s2.md` (2026-08-01
  report untouched — Law 7).
- **S1 lens over S2 spans**: `seam_reliability` read the battery's 44
  seam rows unchanged (rung label continuity held) — the first battery
  in this repo's history that yields per-verb stats (`open` 100%
  effect @ 433ms avg; refusals counted). S1's origin story closed.
- One battery defect found live and fixed in the DRILL, not the
  daemon: `/inventory` wraps as `{"apps": [...]}`; first run graded it
  FAIL, second run GREEN.

## Ops

PR to `main`, double-green law applies (`gh pr checks --watch` before
merge). Operator-lane files untouched; their dirty `signals.jsonl`
lines are theirs to land. Next: S3b (Foundry v0 + hub UI, first
clickable demo, fabric-inspector overlay + purge/remake ops) → S4
Blender pack → S5 Optimize cards.
