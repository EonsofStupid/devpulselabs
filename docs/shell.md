# The shell — the zone contract

Layout v2 (task #103, 2026-07-22). The operator's directive, verbatim:
*"shell is always on the top of viewport statusbar bottom leftside rightside
overlay the content scrolls inside you have to get these zones clearly
defined so we can easily edit... well established grids so i can evolve and
have cssgrid for all of my status indicators."*

## The five zones

```
┌─ TOP ────────────────────────────────── topbar_height() ─┐
├─ RAIL ─┬─ CONTENT (the well) ─────────────┬─ PANEL ──────┤
│ rail_  │ the active hub renders here over │ panel_width()│
│ collap-│ the app backdrop; a hub that     │ (inspector)  │
│ sed()  │ overflows scrolls ITSELF —       │              │
│        │ chrome never moves               │              │
├─ STATUS (the indicator grid) ──── bottombar_height() ────┤
└──────────────────────────────────────────────────────────┘
```

## Laws

1. **Every chrome dimension is a generated token.** `Shell::ui` computes
   `cl::Fluid::of(ctx.content_rect())` once per frame; the panels take
   `topbar_height()`, `bottombar_height()`, `rail_collapsed()`,
   `panel_width()`. The rail widens to `rail_expanded()` while a house is
   open (law 5); rail tiles are `tap_min()` tall. Editing a zone's size
   means editing `crates/coldlight/tokens/fluid.json` — never a constant
   in paint code.
2. **The zones are observable.** Each frame captures the REAL panel rects
   into `Zones { root, top, status, rail, panel, content }` —
   `Shell::zones()`. The egui_kittest test
   `zones_pin_chrome_to_the_viewport_at_any_size` pins the contract at two
   viewport sizes: status bottom == root bottom (the off-screen-footer
   regression), chrome dims == token values, the well tiles inside the
   chrome without overlap.
3. **Content scrolls inside.** The well is fixed; a hub whose surface
   overflows owns its own `ScrollArea` (Library already does). The shell
   never wraps hubs in one — that would double-scroll.
4. **The STATUS zone is a registry, not a flex row.** `shell::status` —
   three lanes (Left/Center/Right thirds), each cell one `StatusCell { id,
   lane, render }` in the static `CELLS` slice. Adding an indicator = adding
   ONE cell. Cells paint and *answer* (`StatusAction`); the shell mutates —
   never the other way.
5. **The RAIL is an ACCORDION** (Program L, doctrine §5.2 — the operator's
   "click on wiredFRONT and its subroutes appear and extend down pushing
   the others down"). Folded it is the icon rail at `rail_collapsed()`.
   Tapping a house unfolds its subroutes INLINE beneath its row — the
   houses below push down — folds whatever was open (ONE open at a time),
   and navigates to the house's default route; tapping the open house
   folds it without navigating; tapping a subroute navigates and leaves
   the house open. While a house is open the rail widens to
   `rail_expanded()` (animated, 0.18s) and hub labels fade in beside their
   glyphs. The unfold wears a 3px strip tinted by the boot-rolled mood
   (`dpl_backdrop::random_mood_index()` — fresh every launch, the
   operator's "random generated mood on reload").
6. **Boot is maximized** (`with_maximized(true)` in crates/app). A fixed
   logical size overflows small displays at dpr > 1 — that was the original
   footer-under-the-taskbar bug.
7. **Full-bleed** hides all chrome (Esc or the exit chip leaves); only the
   CONTENT zone survives in `zones()` while it's on.

## Evolving it

- New status indicator → one `StatusCell` in `crates/shell/src/status.rs`.
- New hub → registered in `crates/app/src/main.rs`; the rail accordion,
  breadcrumb, and dispatcher pick it up from `HubSpec` automatically.
- New zone geometry → a token in the fluid `shell` group + a consumer here;
  the kittest contract test must learn the new zone in the same pass.
