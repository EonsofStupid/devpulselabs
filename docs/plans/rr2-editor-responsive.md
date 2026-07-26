# RR2 — uncramp the wiredFRONT editor below 1280px

Program: Responsive Rendering (RR). RR0 = the `fit()` no-panic law (#54);
RR1 = the sweep + cramp map (#55). RR2 acts on RR1's map: the AE editor is
crash-free at all sizes but CRAMPED below ~1280px wide. Ground-truthed
2026-07-26 against `crates/composer/src/lib.rs::ae_screen`.

## 1. Context

RR1's map: `wiredfront/editor` at 1024×640 shows center-column text overlap
(the viewer's "Add footage…" spills into the INFO/EFFECTS panels) and the top
tool row overflows under the inspector; it is clean at ≥1280 wide. RR2 makes
it usable down to the small sizes the sweep exercises. Gate: the
wiredfront-experience bar — every surface renders honestly at real sizes.

## 2. Current state (ground-truthed 2026-07-26)

`ae_screen` (`crates/composer/src/lib.rs:1743`) is an absolute-rect layout:
- FIXED panel widths: `project_w = 250.0` (`:1750`), `right_w = 232.0`
  (`:1751`). `viewer_rect` is the leftover between them (`:1770-1773`), so at
  a narrow `body` the viewer is starved → the text overlap.
- `tool_strip` (`:~1815`) is a plain `ui.horizontal` of all tools + 5
  placeholder buttons — no wrap/scroll, so it overflows the fixed-height
  `tools_rect` at narrow widths.
- `timeline_h` already uses `fit()` (RR0) — no panic; RR2 is purely the
  cramp, not a crash.

## 3. Target design

1. **Fit-to-available panel widths.** `project_w`/`right_w` become
   `fit(<preferred>, <min>, body.width() * <fraction>)` so they shrink on
   narrow bodies but never below a usable min — the viewer keeps real width
   (roughly doubles at 1024×640). Uses the existing `fit` helper (RR0), one
   source of truth.
2. **Scrollable tool strip.** Wrap `tool_strip`'s row in
   `egui::ScrollArea::horizontal()` with `VisibleWhenNeeded` scrollbar — it
   stays one 40px line and scrolls when it overflows, hiding nothing,
   changing no layout math.
3. **Measure, then maybe breakpoint.** Rebuild + real proof-shot at 1024×640.
   If fit-to-available + scroll makes it clean, RR2 ships there. If the
   smallest sizes still overlap, add a narrow breakpoint that collapses the
   right (INFO/EFFECTS) panel to a toggle — RR2.1, only if measured-needed.

## 4. Changes, in execution order

1. `ae_screen`: `project_w`/`right_w` → `fit(...)` of `body.width()`.
2. `tool_strip`: wrap the `horizontal` in `ScrollArea::horizontal`.
3. Rebuild release; proof-shot the editor at 1024×640 / 1280×720 / 1920×1080
   on the real display; compare against RR1's cramped baseline.
4. If clean → done. If not → the breakpoint (documented as executed or as
   RR2.1).

## 5. Boundaries

- Only `ae_screen` + `tool_strip` — no other route (RR1 showed them clean at
  sampled sizes; if the full PNG set flags another, it is its own RR pass).
- No behavior change — pure layout. No new state unless the breakpoint is
  measured-necessary.
- No `#[allow]`; `fit` reused, not reinvented.

## 6. Verification

- `cargo green`.
- Real-display proof-shots (this box + HDMI 1920×1080): the editor at
  1024×640 is UNCRAMPED (no text overlap, tool row reachable) — before/after
  in §8.
- eguiPilot: `inspect devpulse-cloud` on the editor fixture — assert no two
  interactive node `rect`s overlap at the small size (the semantic-rect check
  RR1 unlocked). (If the editor fixture needs a size arg, note it.)
- Re-run `cargo xtask sweep` — editor cells stay no-panic.

## 7. Commit plan

Branch `feat/rr2-editor-responsive`, one PR (plan + `ae_screen`/`tool_strip`).
PR → `green-linux` → squash-merge → STATE.md + §8.

## 8. Evidence (2026-07-26)

Four changes in `ae_screen`/panes (`crates/composer/src/lib.rs`), all proven
on the real HDMI 1920×1080 display at the size RR1 flagged (1024×640):

1. **Fit-to-available panel widths** — `project_w`/`right_w` → `fit(pref,
   min, body.width()*frac)`; the center viewer keeps real width instead of
   being starved (preferred widths still hold ≥~1280).
2. **Scrollable tool strip** — `tool_strip` wrapped in
   `ScrollArea::horizontal` (`auto_shrink [false,true]`); the row stays one
   line and scrolls instead of overflowing under the inspector.
3. **Scrollable effects pane** — `fx_pane` wrapped in `ScrollArea::vertical`
   (`auto_shrink [false,false]`); the narrow-column copy clips-and-scrolls
   inside the panel instead of spilling into the timeline.
4. **Truncating NEXT hint** — status line 2 restructured: comp facts
   right-align FIRST, then the NEXT hint fills the remainder with
   `Label::truncate()` — the two never overrun each other.

BEFORE (RR1 baseline @1024×640): center-column text overlap (viewer copy vs
INFO/EFFECTS), tool row overflowing under the inspector, EFFECTS copy
spilling into the timeline, NEXT-hint colliding with the comp facts.
AFTER (@1024×640): all four resolved — tool row contained/scrollable,
EFFECTS clipped, viewer roomy, bottom line clean ("NEXT: … or…" truncated +
facts side-by-side). Wide sizes (≥1280) are unchanged — every change is a
no-op when there is room. `cargo green` green; `green-linux` gated.

DEVIATION: the eguiPilot semantic-rect no-overlap assertion (§6) is deferred
— there is no `editor` fixture and `inspect` can't pass `--window`, so a
small-size semantic check needs a harness addition (fixture + size arg),
noted for RR1.1/a later pass. The real-display proof-shots are the
verification of record here.
