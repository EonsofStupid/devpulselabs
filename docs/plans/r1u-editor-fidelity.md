# R1u — Editor fidelity: bottom toolbar · left tool rail · rail accordion Editor/Library · the ingredient Library surface

## 1. Context

The WiredFRONT editor is the operator's After Effects — stitch, post-process,
effects, fonts, mint elements for Workbench. The operator's explicit words
(2026-07-24, verbatim):

> "I want the video editor properly built. That means the toolbar being built
> properly along the bottom and then the left side for tool selection."
> "when I click on WiredFRONT the leftside bar extends down with editor,
> library and so on properly slightly indented with a different background and
> mouse over effects."
> "we are building this so presets are not one offs but building a true
> library off the ingredients cleanly generated"

The reference images (`docs/design/wiredfront-editor-inventory.md`) show the
editor with a LEFT vertical tool strip and a BOTTOM bar (steps/narration/
transport). The current `dpl-composer` surface has the toolbar on TOP and
annotate tools in a TOP row — that is the defect this pass removes. The rail
accordion exists but the unfolded subroutes have no distinct background and no
indent, and WiredFRONT's subroute is still named "Composer" with no Library.

## 2. Current state (all read in-session, branch `feat/r1t-preview-annotate`, clean)

- `crates/shell/src/lib.rs`
  - `rail()` (~421): accordion works; the unfold section paints only a 3px
    mood strip then rows — no group plate, rows full-width.
  - `sub_item()` (859–898): allocates `width×26`, selected = tone stroke +
    dot at x+10, hovered = `surface::OVERLAY` fill, text at x+22 in
    `ink::MUTED`.
- `crates/hubs/wiredfront/src/lib.rs`
  - `pub const COMPOSER: SubId = SubId("composer")` (22); SPEC subroute
    "Composer" (50–54); dispatch `route.sub == COMPOSER → self.composer.ui`
    (1031). Grep: no consumer outside this file.
- `crates/composer/src/lib.rs`
  - `Composer { comp, dir, selected, preview, palette, render, tab, status }`
    (32–47) — no tool state.
  - `edit_tab()` (459–490): toolbar TOP (`toolbar_h=38` at rect.min), then
    preview+inspector, timeline bottom. `toolbar()` (582–644): New/Save/
    Render + pill + right stats.
  - `annotate_tab()` (542–580): tool row TOP (`toolrow_h=36`, horizontal
    chips Select/Arrow/Callout/Box/Spotlight/Text, inert), canvas below.
  - Helpers `chip`/`small_btn`/`plate`/`tab_stub`/`fit_aspect`/
    `painter_label`; `Act` enum; 6 tests.
- `crates/coldlight/src/lib.rs` — generated registries this pass reads:
  `TRAITS: &[TraitDef{id,role,label,blurb,…}]`, `LOOK_PRESETS: &[LookPreset]`,
  `MOTION_PRESETS: &[MotionPreset]`, `FONTS: &[FontDef{id,label,class,blurb}]`,
  `MOODS: &[Mood{name,stops,scrim,accent}]` (+ `Mood::at`), `compose(tone,
  base, stack, state) -> ResolvedStyle` (stack is NON-static — usable for
  runtime swatches), `ResolvedStyle::behind_shapes`, `font_family(id)`,
  `treated(...)/.margin()/.show()`, `TraitState::{Idle,Hover,Active}`.

## 3. Target design

1. **Rail accordion** — the unfolded subroute group sits on its own recessed
   plate (`surface::BASE`, darker than the rail's `RAISED`) with the mood
   strip on top; rows are indented 10px onto the plate; hover = `OVERLAY`
   fill + ink brightens to `STRONG`. Tap WiredFRONT → Editor · Library ·
   Overview · ViziWiz · Gallery · Sessions unfold.
2. **WiredFRONT subroutes** — `COMPOSER`→`EDITOR` (`SubId("editor")`,
   "Editor"); new `LIBRARY` (`SubId("library")`, "Library"). Editor first in
   the list (the editor IS the product), then Library, then the capture
   floors.
3. **Editor layout (Edit mode)** — LEFT vertical tool strip (Select/Arrow/
   Callout/Box/Spotlight/Text — a real, persisted selection on `Composer`),
   CENTER preview, RIGHT inspector, TIMELINE above a full-width BOTTOM
   toolbar (New/Save/Render + render pill + comp stats — the transport bar).
   Annotate mode shares the SAME left tool strip (no top row).
4. **Library surface** (`dpl_composer::library_ui`) — renders ONLY the
   generated registries: traits (live `compose()` swatches), look presets,
   motion presets, fonts (labels rendered in their own face), moods (gradient
   strips via `Mood::at`). Zero hand-authored entries — adding the Nth
   ingredient is authoring JSON + regenerating; this surface picks it up with
   no code change. Both editors will read this same surface (Workbench wiring
   is a later pass).

## 4. Changes, in execution order

1. `crates/shell/src/lib.rs` — edit `rail()`: inside `if f > 0.001`, before
   the mood strip, paint the group plate
   (`child.painter().rect_filled(plate_rect, RADIUS_SM, cl::surface::BASE)`).
   Edit `sub_item()`: indent the row rect 10px from the left of the slot;
   hover paints `OVERLAY` fill AND text ink becomes `STRONG`; selected
   unchanged except drawn on the indented rect.
2. `crates/hubs/wiredfront/src/lib.rs` — rename const `COMPOSER`→`EDITOR`
   with id "editor"; add `LIBRARY`; SPEC order: Editor, Library, Overview,
   ViziWiz, Gallery, Sessions; dispatch `EDITOR → self.composer.ui(ui)`,
   `LIBRARY → dpl_composer::library_ui(ui)`.
3. `crates/composer/src/lib.rs` — add `mod library; pub use library::library_ui;`;
   add `Tool` enum (Select/Arrow/Callout/Box/Spotlight/Text, ALL/label/blurb)
   + `tool: Tool` field (+ Default) + `Act::Tool(Tool)` + apply arm; new
   `tool_rail()` (vertical strip, active state via `TraitState::Active`,
   tipped, pilot-tracked); rework `edit_tab()` rects: `toolbar` full-width at
   BOTTOM (h 44), tool strip LEFT (w 64, top→timeline), timeline above the
   toolbar (right of the strip), preview/inspector fill the rest; rework
   `annotate_tab()`: left tool strip + canvas, top row deleted.
4. `crates/composer/src/library.rs` — NEW: `library_ui` with the five
   generated sections as described in §3.4.

## 5. Boundaries

- 🚫 `/content`, generated `*_gen.rs`, `tokens/*.json` (no new ingredients
  needed this pass), the read-only reference app.
- 🚫 No `#[allow]`, no re-export shim for the removed `COMPOSER` const.
- ⚠️ Ask first: any change to `dpl-comp`'s document schema.
- Out of scope: the axum agent seam (next pass, r1v), Workbench↔Library
  wiring, annotation tool BEHAVIOUR (placement/styling — next editor pass),
  scenes, narration.

## 6. Verification

1. `cargo fmt` then `cargo test -p dpl-composer -p dpl-shell -p dpl-hub-wiredfront`
   → exit 0.
2. `cargo build --release -p devpulselabs` → exit 0.
3. Screenshots (SEQUENTIAL), real sizes:
   - `--route wiredfront/editor --window 1280x800` → toolbar visibly at the
     BOTTOM, tool strip on the LEFT, timeline above the toolbar.
   - `--route wiredfront/editor --window 1100x700` (fit-guard minimum).
   - `--route wiredfront/library --window 1280x800` → all five generated
     sections render.
   - `--open-rail --window 1280x800` → WiredFRONT unfolded: Editor + Library
     rows on the recessed plate, indented.
4. Eye pass on every shot against the feel bar + reference images.

## 7. Commit plan

Branch `feat/r1u-editor-fidelity` off `feat/r1t-preview-annotate` (stacked).
One commit: `feat(wiredfront): R1u — bottom toolbar + left tool rail + rail
accordion Editor/Library + the generated ingredient Library`. Adds
`crates/composer/src/library.rs`, `docs/plans/r1u-editor-fidelity.md`; edits
the three files above. Push + PR onto the stack.

## 8. Evidence

- Gate: fmt clean · clippy clean (workspace) · ALL workspace tests passed
  (the background `cargo green` run was killed by the harness during the
  final release-build step both times; the release build was completed in
  the foreground — `Finished release ... in 1m 28s`).
- `cargo test -p dpl-composer -p dpl-shell -p dpl-hub-wiredfront` → all
  green (5 + 20 + 0).
- Proof shots (SEQUENTIAL via `Start-Process -Wait`, all exit 0):
  - `docs/proof/r1u-editor-1280.png` — Edit mode: workflow tabs top, TOOLS
    rail LEFT (Select pressed-in Active), preview center, inspector right,
    TIMELINE above the full-width BOTTOM New/Save/Render transport bar.
  - `docs/proof/r1u-editor-1100.png` — fit-guard minimum; inspector yields
    (adaptive width), Callout fits one line, preview stays readable.
  - `docs/proof/r1u-library.png` — the recessed shelf: live trait swatches
    (all four compose through `compose()`), look-preset cards with `›`
    separators, scroll to motion/fonts/moods.
  - `docs/proof/r1u-rail-open.png` — wiredFRONT unfolded: Editor · Library
    · Overview · ViziWiz · Gallery · Sessions on the darker plate, rows
    indented, mood strip on top, Editor selected.
- DEVIATIONS from the plan:
  1. First shot round accidentally ran the four exe instances in PARALLEL
     (PowerShell `&` does not wait on a GUI-subsystem exe). All routes
     involved are read-only against the index; the round was discarded and
     re-run strictly sequentially. Lesson recorded: proof shots use
     `Start-Process -Wait`.
  2. Two mid-execution fixes beyond §4, all inside this pass's scope,
     driven by the eye pass: adaptive inspector width (170–240), tool rail
     76px (Callout wrap), Library recessed plate + vertical preset cards +
     `›` instead of the tofu `→`.
