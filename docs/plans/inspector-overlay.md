# Pass — the inspector becomes the OVERLAY it was specified as

## 1. Context

The shell's PANEL zone (the inspector) was ported as a **docked**
`egui::Panel::right` that permanently reserves `panel_width()` — a 20–28 rem
token, i.e. **320–448 px** — on every route, every frame. The operator's
layout directive, recorded verbatim in `docs/shell.md` since task #103:

> "shell is always on the top of viewport statusbar bottom leftside
> **rightside overlay** the content scrolls inside"

The right side was specified as an **overlay** from day one. The dock is a
port-fidelity failure: at a 1280-wide window the inspector eats ~29% of the
content well — space the operator needs for editor surfaces. Operator,
2026-07-29: "The right sidebar of shell is an optional overlay — the content
gets that area."

This pass makes the inspector an **optional overlay**: closed by default
(content owns the full width to the root's right edge); toggled from the
topbar; when open it **floats over** the well's right edge without shrinking
the content rect. The original Svelte reference repo is gutted (only
`src-tauri\target` remains — known open question), so fidelity is anchored on
the recorded directive plus the operator's restatement today.

## 2. Current state

- `crates/shell/src/lib.rs` (package `dpl-shell`) — everything lives here:
  - Header doc lines 10–18 describe the five zones; PANEL described as chrome
    that frames the content.
  - `Zones` struct (lines 31–41) captures `panel: Rect` per frame.
  - `Shell::ui` (lines 436–534): layout order is top_bar → status_bar → rail
    → `let panel = self.inspector(ui, &fl);` → `central(ui)` →
    `cl::frame_overlay(ui.painter(), content, tone)`.
  - `fn inspector` (lines 847–933): `egui::Panel::right("dpl_inspector")
    .exact_size(fl.panel_width())` docked panel; body = INSPECTOR header,
    route card, SIGNALS card (activity feed), `determinism_card`, TOOLS
    stub. All paint already goes through ColdLight vocabulary
    (`surface::RAISED`, `sheen`, `card_frame`, `tip`).
  - `fn top_bar` (lines 569–606) returns `(Rect, bool)` — the bool is the
    full-bleed click, produced by `icon_button` in a right-to-left cluster.
  - Tests: `zones_pin_chrome_to_the_viewport_at_any_size` (lines 1756–1792)
    asserts `z.panel.width() == fl.panel_width()` and
    `z.content.max.x <= z.panel.min.x + 0.5` — the DOCK contract, which this
    pass inverts.
- `docs/shell.md` — the zone contract doc: diagram shows PANEL as a docked
  column; Law 1 lists `panel_width()` among docked chrome; Law 2 describes
  the kittest pin of the docked geometry.
- `crates/app/src/main.rs` — flag-parse block (`--open-rail` at 285–292),
  `Boot` struct (open_rail at 490), destructure (580), apply site (869–873:
  `shell.set_open_house(...)`).
- `CLAUDE.md` — harness-flags list (ground-truthed 2026-07-27) must gain the
  new flag.
- Only other `panel_width()` consumers: `crates/hubs/workbench/src/author.rs`
  lines 540/986 use it as a *column-measure token*, not the panel — NOT
  touched.
- `zones()` consumers outside shell tests: none (grepped workspace).

## 3. Target design

- `Shell.inspector_open: bool`, **default false**. Closed: the PANEL zone is
  `Rect::ZERO`, CONTENT spans to the root's right edge.
- Open: the inspector paints as a floating plate over the well's right edge —
  `x ∈ [content.max.x − panel_width(), content.max.x]`, full content height.
  The content rect itself does not change; nothing reflows.
- Toggle affordances (touch-first, tooltip law): a topbar `icon_button`
  (`cl::icon::WINDOW`) beside the full-bleed button, and a `cl::icon::CLOSE`
  button in the overlay's header. Both tooltipped in layman words.
- Overlay visuals: existing chrome vocabulary only — plate fill
  `surface::RAISED` + `sheen`, and a hard-edged black-diffusion band down the
  plate's exposed left edge (the same `from_black_alpha` diffusion pattern
  `frame_overlay`/`launcher_chip` already establish; no blur, no new tokens).
- Harness: `--inspector` boot flag pins the overlay open for proof shots
  (mirror of `--open-rail`); `set_inspector_open`/`inspector_open` public for
  kittest.
- Scale story: zone geometry still comes only from the fluid `shell` token
  group; no new tokens, no new ColdLight ingredients (gate satisfied — the
  pass repositions an existing plate).

## 4. Changes, in execution order

1. **edit `crates/shell/src/lib.rs`** — header doc: describe PANEL as the
   optional overlay ("PANEL is an optional overlay — closed by default, the
   content well owns the full width; open, it floats over the well's right
   edge").
2. **edit `crates/shell/src/lib.rs`** — `Zones.panel` doc: "`Rect::ZERO`
   while the inspector overlay is closed."
3. **edit `crates/shell/src/lib.rs`** — `Shell` struct: add
   `inspector_open: bool` after `full_bleed`; init `false` in `with_ears`.
4. **edit `crates/shell/src/lib.rs`** — accessors beside `full_bleed()`:
   `pub fn inspector_open(&self) -> bool` and
   `pub fn set_inspector_open(&mut self, on: bool)`.
5. **edit `crates/shell/src/lib.rs`** — `top_bar` returns
   `(Rect, bool, bool)` (adds `toggle_inspector`); a second `icon_button`
   with `cl::icon::WINDOW` and tip "inspector overlay (floats over the
   content)" added after the full-bleed button in the right-to-left cluster.
6. **edit `crates/shell/src/lib.rs`** — `Shell::ui`: consume the new tuple,
   `if toggle_inspector { self.inspector_open = !self.inspector_open; }`;
   reorder the tail to `central` → `frame_overlay` → conditional
   `self.inspector(ui, &fl, content)` (else `Rect::ZERO`) → capture zones.
7. **edit `crates/shell/src/lib.rs`** — rewrite `fn inspector(&mut self, ui,
   fl, content: Rect) -> Rect`: compute the overlay rect; swallow-clicks
   background interact (`dpl_inspector_bg`); paint diffusion band + plate +
   sheen; build a `new_child` ui inside `rect` (12px margins, clipped);
   header row = INSPECTOR label + CLOSE `icon_button` (tip "close — the
   content keeps this space"); body cards unchanged (route, SIGNALS,
   determinism, TOOLS), called directly on the child (no closure).
8. **edit `crates/shell/src/lib.rs` tests** —
   `zones_pin_chrome_to_the_viewport_at_any_size`: replace the two dock
   assertions with `z.panel == Rect::ZERO` and
   `eq(z.content.max.x, z.root.max.x)`. Add test
   `the_inspector_overlays_the_content_instead_of_docking`: open via
   `set_inspector_open(true)`; assert `eq(z.panel.width(),
   fl.panel_width())`, `eq(z.content.max.x, z.root.max.x)`, and
   `z.content.contains_rect(z.panel)`.
9. **edit `crates/app/src/main.rs`** — parse
   `let inspector = args.iter().any(|a| a == "--inspector");` beside
   `--open-rail`; add `inspector: bool` to `Boot`, its construction, and the
   destructure; apply after the `open_rail` apply site:
   `if inspector { shell.set_inspector_open(true); }`.
10. **edit `docs/shell.md`** — diagram: PANEL drawn as an overlay riding the
    CONTENT edge; Law 1 keeps `panel_width()` as the overlay's width token;
    Law 2 documents `panel = Rect::ZERO` when closed; new Law 8: "The PANEL
    is an OPTIONAL OVERLAY (the operator's 'rightside overlay the content'):
    closed by default, it reserves no width; open it floats over the well and
    the content rect does not shrink."
11. **edit `CLAUDE.md`** — harness flags line: add `--inspector` after
    `--open-rail <hub>`.

## 5. Boundaries

- 🚫 `/content` — never.
- 🚫 `crates/hubs/workbench/src/author.rs` — its `panel_width()` uses are a
  measure token, unrelated.
- 🚫 `crates/coldlight/tokens/*` — no token changes; `panel-width` stays and
  keeps sizing the overlay.
- 🚫 No `#[allow]`, no new crates, no generated-file hand-edits.
- ⚠️ `docs/design/architecture.md` / xtask structure audit — untouched (no
  crate or dependency changes); if the structure phase objects, STOP and
  re-ground.
- Out of scope: per-hub tool docks (P1), pilot-tracking the topbar toggle
  (the full-bleed button sets the precedent — neither is tracked), inspector
  content changes.

## 6. Verification

1. `cargo test -p dpl-shell` → exit 0, including the new overlay test.
2. `cargo clippy -p dpl-shell -p devpulselabs --all-targets` → exit 0, no
   warnings.
3. `cargo build --release -p devpulselabs -p dpl-lottie-render` → exit 0;
   `target\release\devpulselabs.exe` mtime moves.
4. Proof shots (SEQUENTIAL, real sizes — ColdLight any-shape gate):
   - `devpulselabs.exe --screenshot shot-closed-1100.png --shot-frame 5 --window 1100x700 --mood 0`
   - `... --screenshot shot-open-1100.png --inspector --window 1100x700 --mood 0`
   - `... --screenshot shot-closed-1280.png --window 1280x800 --mood 0`
   - `... --screenshot shot-open-1280.png --inspector --window 1280x800 --mood 0`
   Pass: closed shots show content reaching the right edge (no docked
   column); open shots show the plate floating over content, content
   unmoved beneath. Feel bar: matches the shell's existing chrome polish.
5. End-to-end proof = the operator's requirement directly: the closed-state
   screenshot at 1100x700 shows the editor/content area wider by
   `panel_width()` than the pre-pass baseline.

## 7. Commit plan

- Branch `shell-inspector-overlay`; commit all edits above plus this plan.
- Message: `Shell — the inspector becomes the optional overlay it was
  specified as (docs/shell.md "rightside overlay")`.
- `gh pr create` → wait for BOTH lanes (`gh pr checks <n> --watch`) →
  merge ONLY on double-green → rebuild release exe on main → STATE.md.

## 8. Evidence

- `cargo test -p dpl-shell` — 22 passed, 0 failed (includes the new
  `the_inspector_overlays_the_content_instead_of_docking` and the rewritten
  `zones_pin_chrome_to_the_viewport_at_any_size`).
- `cargo clippy -p dpl-shell -p devpulselabs --all-targets` — exit 0.
- `cargo build --release -p devpulselabs -p dpl-lottie-render` — exit 0;
  exe mtime 2026-07-29 10:03.
- Proof shots (sequential, real exe, `--mood 0 --no-seam`, session
  scratchpad `shot-{closed,open}-{1100,1280}.png`):
  - closed @1100x700 and @1280x800: the content grid reaches the window's
    right edge — a full extra gallery column visible where the dock sat.
  - open @1100x700 and @1280x800: the inspector plate floats over the
    well's right edge; the content columns beneath are position-identical
    to the closed shot (no reflow). Left-edge diffusion band reads as
    painted depth; topbar shows the new toggle beside full-bleed.
- Deviations from the plan: none. ColdLight gate: no new ingredients; the
  card-width overflow under the plate's right margin is pre-existing docked
  behavior, clipped identically (parity kept, noted for a future polish
  pass).
