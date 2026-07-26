# ADR — the project ESTATE + the agent seam (Fjall · Moka · axum)

Status: ESTATE LANDED on `main` (2026-07-26 re-author of parked
`feat/estate-fjall-moka` — fresh Cargo.lock; Fjall 2.x + Moka 0.12). Operator
directive 2026-07-24: "extend Claude's infra ability, and persistence… add
Fjall, Moka as a local per-project estate… axum signals getting properly to
http endpoints". The axum seam (§2) already exists as `dpl-seam` (R1v); next
is wiring estate into seam/editors for durable agent + editor state.

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

## §2 — The agent seam (axum) — NEXT

A localhost **axum** HTTP server, started by the app, bridges the typed
`Fabric` (tokio broadcast) ↔ HTTP so chat providers generate into and drive
the editors:

- `GET /state` — the current editor state (active comp/scene, selection,
  library manifest) read from the estate.
- `POST /command` — a typed, VALIDATED command (add clip, place annotation,
  add container, enqueue generation). Validation reuses the doc doctors — an
  agent can never write an invalid comp/slide.
- `GET /signals` (SSE) — fabric `ShellEvent`/`DeckEvent` streamed out, so an
  agent sees where the operator is and what changed.
- Generation jobs are an engine queue (mpsc, not broadcast — the fabric law);
  outputs land as REAL files in the inbox and index write-through.

The seam is how "chat can generate and work with this" (operator) becomes
real: the same commands the editor UI issues, an agent issues over HTTP,
against the same estate + doctors.

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
2. axum agent seam over the fabric + estate (§2).
3. Editors read/write the estate (WiredFRONT tab/comp/selection persistence;
   Workbench doc state) + surface the generated manifest.
4. WiredFRONT editor fidelity to the reference (bottom toolbar, left tool
   rail, canvas, inspector) — every control a generated ingredient.
5. Rail accordion: WiredFRONT unfolds Editor · Library · … (indent, own bg,
   hover) — mostly a subroute-list + polish pass over the existing accordion.
