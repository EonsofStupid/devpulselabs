# C2f — Operator Import UI (Coach hub)

**Date:** 2026-07-31  
**Branch:** `feat/c2f-coach-import-ui`  
**Plan:** `docs/plans/coach-c2f-operator-import-ui.md`

## Shipped

- New hub crate `dpl-hub-coach` (`HubId("coach")`), one registry line in
  `crates/app/src/main.rs`.
- Operator flow: inventory list (Cursor first, Calculator second) →
  honesty Import modal → essential checks → Standard mint progress →
  lesson-ready → **Start guided lesson** spawns `dpl-coach-overlay`
  spotlight (quiet spawn; never a crate dep on overlay).
- Thin a11y / cold / Deep → Advanced Import tip (no fake Standard; no
  harvest in C2f).
- Structure: inventory + fabric consumed by hub; architecture + orphan
  tip amended same commit.

## Unit

`cargo test -p dpl-hub-coach --lib` — Cursor → Standard + seed
`Search Ctrl+K`; Calculator seed `7`; Deep/cold → AdvancedDivert;
conquest rank order.

## Reality (this seat)

- `cargo green` ✔
- Proof shot: `docs/proof/c2f-coach-hub.png` — Coach rail, Cursor
  selected RUNNING, Calculator next, Import / analyze… ready.
- Live import→paint on Cursor: operator drill with release exe +
  `dpl-coach-overlay` beside it.

## Out / next

Advanced harvest (CX-harvest) · Teacher · Pass-2 · Notepad (Windows).
