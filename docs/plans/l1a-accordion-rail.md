# L1a — the accordion rail

Parent: docs/plans/l1-library-doctrine.md (§5 ruling 2, §6). Ledger #118.
Worktree: `C:\localdev\dpl-main` on main, after a35d569.

## 1. Context

The operator, verbatim: *"right now you have it as a megamenu, we want this
to be a unique overlay where you click on wiredfront and then its subroutes
appear and extend down pushing the others down."* Ratified ruling (doctrine
§5.2): **one open at a time** — tapping a house unfolds its subroutes
downward AND folds whatever was open; the houses below push down; tapping
the open house folds it; "Compact always."

Today the rail is icon-mode with a HOVER-anchored floating flyout card —
a megamenu. That dissolves (pre-release, no shims) and is replaced by the
click-driven accordion living INSIDE the rail panel.

## 2. Current state (ground-truthed this session)

- `crates/shell/src/lib.rs` (read in full): `Shell.flyout: Option<(HubId,
  f32)>`; `rail()` renders one `icon_tile` per hub at
  `fl.rail_collapsed()` width, hover records an anchor, click →
  `default_route()`; `flyout_ui()` paints an `egui::Area` card at
  `fl.rail_expanded()` width with the boot-rolled `tooltip_mood` strip +
  `sub_item` rows. Pilot ids: `shell.rail/<hub>` (value
  active/inactive), `shell.flyout/<hub>/<sub>`.
- The flyout machinery has NO consumers outside the shell crate (grepped).
- Tokens (crates/coldlight/tokens/fluid.json, shell group):
  `rail-collapsed` 3.5–4.25 rem, `rail-expanded` 8.5–10.3125 rem — both
  already generated; no token work needed.
- `docs/shell.md` law 5 documents the hover flyout; law 1 says the flyout
  card is `rail_expanded()` wide. Both must be rewritten.
- `crates/app/src/main.rs`: flag parsing pattern (`--route`, `--focus`…)
  → `Boot` struct → `App::new` applies pre-registration; hub ids reach the
  CLI as leaked strings.
- Shell tests: unit tests on navigate/activate semantics + two kittest
  harness tests (`zones_pin_chrome_to_the_viewport_at_any_size` asserts
  `rail.width == fl.rail_collapsed()`).

## 3. Target design

- `Shell.open_house: Option<HubId>` replaces `flyout`. Boot = `None`
  (compact). `toggle_house(id)`: open house tapped → fold, NO navigation;
  otherwise → becomes the open house (folding any other implicitly) AND
  navigates to the hub's default route (touch-first: one tap still goes
  somewhere). Tapping a subroute row navigates and leaves the house open.
- The rail panel WIDTH answers the accordion: per-house unfold factor
  `f = ctx.animate_bool_with_time(("dpl_rail_unfold", hub), open, 0.18)`;
  panel width = `rail_collapsed + (rail_expanded − rail_collapsed) ·
  max(f)`. Folded rail is byte-for-byte the icon rail; an open house
  widens it to the expanded token.
- A house row (`house_tile`): the icon stays centered in the COLLAPSED
  column; the hub label fades in to its right (`gamma_multiply(max_f)`)
  as the rail widens. Active accent bar/bevel/wash unchanged from
  `icon_tile` (which it replaces — same painting, plus label).
- The unfold section under a house: full height = 3px mood strip
  (the boot-rolled `tooltip_mood` gradient survives here — the operator's
  "random mood on reload") + one `sub_item` row (26px + 2 gap) per
  subroute; the section allocates `full_height · f` and clip-reveals from
  the top — subroutes literally extend down and push the houses below.
- Pilot ids: houses keep `shell.rail/<hub>` (value now
  `active+open | active | open | inactive`); subroute rows become
  `shell.rail.sub/<hub>/<sub>` (MenuItem, selected/available) — the
  `shell.flyout` namespace dies with the flyout (no shims).
- `flyout_ui`, the hover-anchor logic, and the `flyout` field are DELETED.
  `tooltip_mood` stays (the accordion's mood strip).
- Harness: `--open-rail <hub>` sets the open house pre-run (proof shots).
  Unknown hub degrades to a folded rail, never a panic.

## 4. Changes, in execution order

1. **edit `crates/shell/src/lib.rs`** —
   a. Module doc: rail paragraph → accordion description.
   b. `flyout: Option<(HubId, f32)>` field → `open_house: Option<HubId>`
      (+ doc comment; `flyout: None` in `new()` → `open_house: None`).
   c. Add after `activate_hub`:
      `pub fn open_house(&self) -> Option<HubId>`,
      `pub fn set_open_house(&mut self, id: Option<HubId>)`,
      `pub fn toggle_house(&mut self, id: HubId)` (fold-if-open else
      open+`activate_hub`).
   d. Rewrite `rail()`: compute per-house factors + `max_f` before the
      panel; `Panel::left` at the interpolated width; per hub render
      `house_tile` (click → collect toggled id) then, when `f > 0.001`,
      the clip-revealed unfold section (mood strip + `sub_item` rows,
      click → collect Route, pilot `shell.rail.sub`); after the panel
      apply toggle/navigation. Delete the hover/flyout tail.
   e. Delete `flyout_ui` entirely.
   f. `icon_tile` → `house_tile(ui, icon, label, tone, active, expand,
      height, collapsed_w)`: same painting + the fading label at
      `collapsed_w`-anchored x. Update the call site.
   g. Tests: keep all existing; add
      `tapping_a_closed_house_opens_it_and_navigates`,
      `tapping_the_open_house_folds_without_navigating`,
      `opening_another_house_folds_the_first`, and kittest
      `an_open_house_widens_the_rail_to_the_expanded_token`
      (set_open_house → run ≥60 steps → `zones().rail.width ≈
      fl.rail_expanded()`).
2. **edit `docs/shell.md`** — law 1: flyout-card sentence → "the rail
   widens to `rail_expanded()` while a house is open"; law 5 rewritten as
   the accordion law (click toggles, one open, mood strip, subroutes
   inline); "Evolving it" bullet: flyout → accordion.
3. **edit `crates/app/src/main.rs`** — parse `--open-rail <hub>`
   (the `--route` pattern); `Boot.open_rail: Option<String>`; in
   `App::new` after the boot-route navigation:
   `shell.set_open_house(Some(HubId(Box::leak(...))))`.
4. `cargo green`; proofs (§6); commit; push backgrounded.

## 5. Boundaries

🚫 No token edits (both widths already exist). 🚫 No Shows entry, no
Library dissolution, no seed folders — those are L1b–L1d. 🚫 The topbar,
status grid, inspector, central well, full-bleed paths untouched. ⚠️ The
zones kittest contract must keep passing at boot (folded = collapsed
width) — the new expanded-width assertion is a NEW test, not a rewrite.

## 6. Verification

- `cargo green` exits 0 (new shell tests included).
- Folded proof: `--route wiredfront/gallery --mood 0 --screenshot
  docs/proof/l1a-rail-folded.png --shot-frame 60` — icon rail at
  collapsed width, no flyout anywhere.
- Open proof: `--open-rail wiredfront --route wiredfront/gallery --mood 0
  --screenshot docs/proof/l1a-rail-open.png --shot-frame 90` — the rail
  widened, wiredFRONT's labeled row with its subroutes unfolded beneath
  it, the houses below pushed down, mood strip visible.

## 7. Commit plan

`feat(shell): L1a — the accordion rail (click-to-unfold, one open house)`
— lib.rs + shell.md + main.rs + the two proofs. Push backgrounded.

## 8. Executed — evidence

Execution matched the plan, one addition: `house_tile`'s eight inputs are
bundled in a `HouseTile` struct (clippy's argument budget under the
no-`#[allow]` law — the wiredFRONT `CardState` pattern). Green: all four
new tests pass (`tapping_a_closed_house_opens_it_and_navigates`,
`tapping_the_open_house_folds_without_navigating`,
`opening_another_house_folds_the_first`,
`an_open_house_widens_the_rail_to_the_expanded_token`). Proofs:
docs/proof/l1a-rail-folded.png (icon-compact, no flyout);
docs/proof/l1a-rail-open.png (wiredFRONT unfolded — labeled houses, mood
strip, five subroutes with Gallery selected, Chat pushed down, rail at
the expanded token). Incident during proofs: two overlapping exe
instances corrupted content/artifacts.db (recovery filed
artifacts.db.corrupt); `dpl-index-cli reindex` rebuilt it from disk
(2 artifacts) — the rebuildable-index law's first live save. Lesson:
proof shots run SEQUENTIALLY, never two instances on one index.
