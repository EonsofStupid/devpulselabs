# Resume pack — CURRENT

**Updated:** 2026-07-30
**Lane:** coach-ledger
**Dirty:** false

## Goal

Land C2b (`dpl-coach-overlay` self-test) via PR; optional Pass 2 stdio loop.

## Done this session

- C2a ADR ratified (`docs/plans/coach-overlay-adr.md`); `AGENTS.md` tipped
- C2b pass-1 self-test **PASS** — journal
  `docs/journal/2026-07-30-c2b-coach-overlay.md`
  - unit 4/4 · clippy clean · structure hygienic
  - `--self-test --name 7` → exit 0 Triggered (`stream_hit="7"`)
  - `--self-test --name bogon` → exit 2 refuse
- Ops tipped: STATE · SESSION · remaining-work · signals · program map

## Next

1. Merge PR for `feat/c2b-coach-overlay` (after double-green)
2. Optional: Pass 2 stdio `CoachCommand` / `CoachEvent` under ADR launch gates
3. Strict pinning → DuckDB ADR → class column

## Blockers

- Windows / Wayland overlay unproven
- Production `app` wiring still forbidden

## Key files

- `crates/coach-overlay/`
- `docs/plans/coach-overlay-adr.md`
- `docs/plans/coach-c2b-overlay-daemon.md`
- `docs/journal/2026-07-30-c2b-coach-overlay.md`
- `docs/journal/2026-07-30-c2s-triggered-live.md`
