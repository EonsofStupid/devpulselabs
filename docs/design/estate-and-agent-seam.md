# ADR — the project ESTATE + the agent seam (Fjall · Moka · axum)

Status: ESTATE LANDED (#65). Seam wire (estate-seam-wire): Ring-0 open at
`userdata_root()/estate`, durable `shell/route` + WiredFRONT `lastComp` on
`GET /state`, dual-write `remember_last`. Operator directive 2026-07-24.
Full §2 vision (selection/manifest/agent blobs) remains NEXT after this thin
honest slice.

## Why

The editors (WiredFRONT = After Effects; Workbench = frictionless slide
editor) and the chat providers (Claude/Codex/Grok) all need two things the
app didn't have: **durable local state** and a **way for chat to drive the
editor**. Rather than bolt persistence onto each surface, we add one estate
layer and one HTTP seam, both pure-Rust, no external server.

## §1 — The estate (`dpl-estate`) — LANDED

**Fjall 2.x** (embedded LSM-tree KV) = the persistent estate. **Moka 0.12**
(concurrent cache) = the hot-read layer in front of it. One `Keyspace`, four
partitions (`Tree`): `Docs` (comp/scene/annotation blobs), `State` (editor UI
+ undo cursor), `Manifest` (the generated ingredient manifest), `Agents`
(chat session state). API: `open · put/get · put_json/get_json · remove ·
keys · persist`. Proven: round-trip through cache+disk, tree isolation, and
**state survives reopen** (3 tests green).

**Coexistence, not replacement.** The SQLite/TURSO artifact INDEX (X1) stays
— it answers structured media queries over the content library. The estate is
KV blobs + cache for editor/agent/manifest state. Different jobs; both wrapped
by Moka on the read path where it helps.

## §2 — The agent seam (axum) — PARTIAL

The localhost **axum** seam (`dpl-seam`, R1v/R1w) bridges typed `Fabric` ↔
HTTP. **Estate-backed today** (`docs-history-2026-10-12:docs/plans/estate-seam-wire.md`):

- `GET /state` — `{ version, route, hubs, estate, lastComp }` where `route`
  hydrates from / dual-writes `Tree::State` key `shell/route`, and
  `lastComp` reads `wiredfront/last-comp` (WiredFRONT reopen pointer).
- `POST /command` — navigate / viziwiz-summon / render (validated).
- `GET /signals` (SSE) — fabric events.
- Comp docs remain on disk via `/comp*` (doctors); not mirrored into
  `Tree::Docs` this pass.

**Still NEXT:** selection / undo cursor / library manifest in `/state`;
`Tree::Agents` session blobs; Workbench session; generation job queue
beyond the existing render command.

## §3 — The ingredient-library law (spans everything)

Presets/effects/fonts/annotations/elements/recipes are **NEVER one-offs**.
Each is authored data → `build.rs` generated Rust → the generated MANIFEST
(cached in the estate `Manifest` tree) → surfaced in the Library that BOTH
editors read. WiredFRONT mints elements; Workbench composes containers +
styles + traits (animation/motion/interaction "recipes"). The manifest is the
single source both editors and the agent seam enumerate — "only show what
exists," cleanly generated. This is the ColdLight composable-ingredients
system (traits → elements → blocks → templates) made the spine of the
product, per the coldlight-system gate.

## Build order

1. `dpl-estate` (Fjall + Moka) — DONE.
2. axum agent seam over the fabric + estate (§2 thin slice: route +
   lastComp) — DONE (estate-seam-wire).
3. Editors read/write the estate (WiredFRONT selection/undo;
   Workbench doc state) + surface the generated manifest.
4. WiredFRONT editor fidelity to the reference (bottom toolbar, left tool
   rail, canvas, inspector) — every control a generated ingredient.
5. Rail accordion: WiredFRONT unfolds Editor · Library · … (indent, own bg,
   hover) — mostly a subroute-list + polish pass over the existing accordion.
