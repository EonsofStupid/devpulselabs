# BND1 · M3a journal — Clyffy stands (2026-08-08)

**Done this slice:**
- **The house session is HOST-OWNED** (the brainstorm/content
  projection law — a design correction over the plan's runtime-slot
  sketch): `ClyffySession { id, backing, events }` in `Shared`, minted
  ONCE on first presence sync, never re-identified. The
  `SessionProviderMismatch` guards are never even in play.
- **The identity law, pinned**: `rebind()` swaps the backing and
  returns the UNCHANGED id; events keep their per-event provider tags
  in order (`swapping_clyffys_backing_preserves_the_transcript`).
  `is_active()` = the CURRENT binding is among the activated providers
  — presence without a binding is not activation, and a signed-out
  binding waits honestly (`clyffy_is_inert_without_a_signed_in_backing`).
- **Presence-gated arrival**: `sync_clyffy` runs beside the roster
  republish every frame — the moment one provider is activated, Clyffy
  stands; the binding defaults to the first activated provider and
  survives sign-outs (shown inactive, never erased).
- **The pinned row**: Clyffy renders FIRST under the sidebar's PINNED
  heading (the orphaned `SessionGroup::Pinned` heading finally has an
  occupant) — glyph C, backing shown (`via codex` / `no backing yet`),
  state honest (`ready` / `waiting for a signed-in provider`), pilot
  node `chat.clyffy.session` with the backing as its value.
- **`/model <provider>` is REAL**: census-validated against the roster,
  then `rebind` — "Clyffy is now backed by claude." on the honesty
  line; the transcript untouched by construction.
- Selecting the row states its truth plainly ("sends arrive with the
  next slice · /model swaps the backing").

**Proof:** 39 app tests (2 new Clyffy pins) + 17 core + 45 window.

**Deviations (M3b, named — never silently done):** the
send-through-binding path (Clyffy's composer sends routed to the
backing provider's runtime via a pending-bind on SessionReady) and the
house transcript JSONL + catalog decoder are the NEXT slice; the row
says so itself. The profiles FutureAdapter catalog row stays until M3b
makes "qualified" true.

**Remaining (POAM):** M3b send + persistence · M4 close-out.
