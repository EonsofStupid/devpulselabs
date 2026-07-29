# Pass — p4 rail accordion degrade

Status: CLOSED 2026-07-29 — PASS (scroll degrade + reveal). Closes coach
backlog item 4 in `docs/journal/2026-07-28-remaining-work.md`. Measured
failure was `docs/plans/operator-coach-a2ui.md` §12 (600×400 inverted-rect
clip).

## 1. Intent

At cramped window heights the accordion still **exposes** the open house
and its subroutes (clickable, non-inverted rects). Pick one degrade:
**scroll the rail body** — not fold-away, not an overflow menu.

## 2. In / out / blast radius

**In:**
- `crates/shell/src/lib.rs` — `rail()` ScrollArea + open-house reveal
- `docs/shell.md` — cramped-rail exception to “chrome never scrolls”
- Plan evidence + ops NEXT → C2s; STATE tip; remaining-work
- Real-exe screenshots at documented sizes

**Out:** AT-SPI / C2s, coach-overlay daemon, fluid token changes, Windows
re-baseline of coach lessons, changing accordion one-open doctrine.

**Blast radius:** every hub’s rail paint path; zone kittest still holds
(rail *panel* geometry unchanged — only inner body scrolls).

## 3. Design

1. Wrap the house/unfold loop in `ScrollArea::vertical()`
   (`id_salt("dpl_rail_accordion")`, `auto_shrink([false, false])`).
2. On open-house change (`set_open_house` / `toggle_house` open path,
   including `--open-rail`), set `rail_reveal_pending`.
3. While pending, `scroll_to_rect` the open house tile ∪ unfold until it
   intersects the scroll clip with positive area, then clear.
4. Track `last_open_rail_body: Option<Rect>` (clip-intersected) for the
   kittest contract.
5. Amend `docs/shell.md`: content well still owns hub scroll; **rail body
   may scroll when the accordion exceeds the rail zone height**.

## 4. Verification

1. `cargo test -p dpl-shell` — new cramped-rail test + existing zone tests
2. Real exe (sequential):
   - `--window 640x480 --open-rail workbench --route workbench/library --mood 0 --screenshot docs/proof/p4-rail-cramped-640x480.png`
   - `--window 1280x800 --open-rail workbench --route workbench/library --mood 0 --screenshot docs/proof/p4-rail-ok-1280x800.png`
3. Cramped shot must show Workbench subrows (not empty clip under last house)

## 5. Risks

- Auto-reveal fighting operator scroll — mitigated by pending-only reveal
  (fires on open change, not every frame forever).
- ScrollArea changing pilot clip flags — subrows must leave `hidden+clipped`
  inverted state; prove with kittest rects + screenshot.
- 600×400 is below `min_inner_size` 640×480 — prove at **640×480** (contract).

## 6. Done when

Cramped shot + kittest show open house reachable; remaining-work marks p4
CLOSED; ops NEXT → C2s.

## 7. Evidence

Live 2026-07-29 on this box (`DISPLAY=:1`, NVIDIA GB10):

| Check | Result |
|---|---|
| `cargo test -p dpl-shell` (incl. `cramped_rail_keeps_the_open_house_reachable`) | PASS |
| `docs/proof/p4-rail-cramped-640x480.png` — Workbench unfold visible (Library…Test Lab); houses above scrolled | PASS |
| `docs/proof/p4-rail-ok-1280x800.png` — tall control, same open rail | PASS |

Ops NEXT after land → **C2s** (foreign-target spike).
