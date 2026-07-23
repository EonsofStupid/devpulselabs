# C1 — Stills parity (polygon editor, delay, re-snip, clipboard, nudge)

Parent: roadmap Program C (#108). After D4 (4e9e48e). Worktree:
`C:\localdev\dpl-main` on main.

## 1. Context

Roadmap C1: *"freeform polygon (overlay + engine), window-mode stills +
picker, capture delay, re-snip, clipboard auto-copy, arrow-key nudge
ceremony."* Window-mode stills + the picker ALREADY exist (Pass B4);
the engine already carries the polygon masks. C1 is the OVERLAY tier:
the polygon editor, the nudge-then-confirm region ceremony, delay,
re-snip memory, and clipboard copy.

## 2. Current state (ground-truthed this session)

- `crates/engine-capture/src/lib.rs`: `still_freeform_to` /
  `still_freeform_from_png(src, dest, points: &[[f64;2]])` (fractional
  vertices) EXIST; `still_region_from_png(src, dest, FracRect)`;
  `freeze_primary_to` (the frozen frame). Nothing to add engine-side.
- `crates/pixvix/src/lib.rs` (read in full): Img tab chips Region ·
  Full · Free — Free posts `free_deferred` → an honest status line
  ("freeform lasso lands with the polygon editor"). The region
  viewport: fullscreen frozen frame, drag = live rect, veil, loupe,
  **drag_stopped FIRES the capture immediately** (no adjust step),
  Esc cancels. `RegionSelect{tex, frozen, drag}`; `PaletteActions`
  routes closure→owner; `unique_path` never-clobber; `index_capture`
  write-through. One grandfathered `#[allow(clippy::too_many_arguments)]`
  on `palette()`.
- egui 0.35: `Context::copy_image(ColorImage)` exists (verified in the
  registry source) — clipboard needs NO new dependency.
- The old app's ceremony (Pass ① "Pro snip"): release ARMS an
  adjustable selection; arrows nudge by 1px (Shift = 10px), Enter (or
  double-click) captures, Esc cancels.

## 3. Target design

- **Region flow v2 — the ceremony**: `RegionSelect.drag` becomes
  `sel: Option<Rect>` + `dragging: Option<Pos2>`. Drag paints live;
  release ARMS (rect stays, tinted handles); ←↑→↓ nudge the whole rect
  1px (Shift 10px); Enter/double-click CAPTURES; Esc cancels; a new
  drag replaces the armed rect. The hint line states the ceremony.
- **Freeform polygon editor** (same frozen-frame viewport, a MODE of
  RegionSelect): every click adds a vertex; the path paints live with
  vertex dots; clicking within 8px of the first vertex OR Enter closes
  ≥3 vertices and captures via `still_freeform_from_png`; Esc cancels;
  Backspace removes the last vertex. Veil = full-screen dim with the
  polygon fill un-dimmed (paint the polygon as a mesh of the frozen
  texture at full brightness over the veil).
- **Capture delay**: an Img-tab delay selector chip cycling 0s → 3s →
  10s (one chip, label shows the value). Full/Region/Free honor it:
  the action arms a `delay_until: Option<Instant>` and the palette
  shows a countdown in the status line; the capture (or freeze) fires
  when it elapses. 0s = today's behavior.
- **Re-snip**: the last capture SHAPE is remembered
  (`LastSnip::Full | Region(FracRect) | Free(Vec<[f64;2]>) |
  Window(id)`); a "Re-snip" chip re-fires it against a FRESH frame
  (fresh freeze for region/free) without re-dragging. Honest status
  when nothing has been snapped yet.
- **Clipboard auto-copy**: every successful still ALSO lands on the OS
  clipboard via `ctx.copy_image` (decode the written PNG →
  ColorImage); status gains "· copied". Failure degrades to a status
  suffix, never blocks the file.
- Pilot ids: `pixvix.delay`, `pixvix.resnip`, `pixvix.free` mode etc.;
  the region viewport's ceremony states ride NodeMetadata values
  (armed/dragging/polygon-n-vertices) so scenarios can assert them.

## 4. Changes, in execution order

1. **edit `crates/pixvix/src/lib.rs`** —
   a. `RegionSelect` → `{tex, frozen, mode: SnipMode, sel:
      Option<Rect>, dragging: Option<Pos2>, poly: Vec<Pos2>}` with
      `SnipMode::{Rect, Free}`; `begin_region(ctx, mode)`.
   b. The region viewport: ceremony (arm → nudge → Enter/double-click)
      for Rect; the vertex editor for Free; shared veil/loupe; finish
      routes to `still_region_from_png` / `still_freeform_from_png`.
   c. `Pixvix` fields: `delay: u8 (0|3|10)`, `delay_armed:
      Option<(Instant, PendingSnip)>`, `last: Option<LastSnip>`;
      the countdown check in `ui()`; Full/Window record `last` too.
   d. Clipboard: `copy_png_to_clipboard(ctx, path)` helper; called on
      every still success; "· copied" suffix.
   e. Palette Img tab: the delay chip + Re-snip chip; Free chip now
      opens the polygon editor (the deferral line dies).
2. `cargo green`; proofs (§6); commit; push backgrounded.

## 5. Boundaries

🚫 Engine untouched (masks exist). 🚫 Video/Session/Steps tabs
untouched (C2/C3). 🚫 No new dependencies (clipboard = egui's own).
⚠️ The grandfathered `#[allow]` on `palette()` must not grow — new
palette inputs ride `PaletteActions`/state structs.

## 6. Verification

- `cargo green` exits 0.
- `--pixvix --mood 0 --screenshot docs/proof/c1-palette.png
  --shot-frame 30` — the Img tab with the delay + Re-snip chips.
- `--pixvix-snap` still exits green (Full path + clipboard suffix in
  the printed status).
- The region/polygon ceremony is INTERACTIVE (frozen-frame viewport) —
  verified by hand this pass; the pilot scenario waits on the Codex
  multi-window targets (same deferral as D4, declared).

## 7. Commit plan

`feat(pixvix): C1 — stills parity (polygon editor, ceremony, delay,
re-snip, clipboard)`. Push backgrounded.

## 8. Executed — evidence

Executed with three honest deviations: (1) the polygon interior is NOT
un-dimmed (egui meshes convex polygons only; the freeform veil is a
light whole-frame dim with the bright path/vertices/close-ring over
it — revisit if the operator wants true cutout preview); (2) the
palette-chips screenshot is IMPOSSIBLE from the harness — the palette
is its own always-on-top OS viewport and the screenshot captures the
main window only (the D4 constraint, same deferral: the multi-window
pilot targets are the Codex lane); docs/proof/c1-palette.png records
the wiredfront/pixvix route instead; (3) the retired grandfathered
`#[allow(clippy::too_many_arguments)]` — `palette()` now takes a
`PaletteView` bundle, zero allows in the workspace again.

Proven: `cargo green` exits 0; the `--pixvix-snap` drill exits 0 with
"saved ./content/inbox/pix-2.png **· copied**" — the Full path AND the
egui clipboard land end-to-end on the real exe. The ceremony (arm →
nudge → Enter), the polygon editor, delay countdown, and Re-snip are
wired through the one action path (`PaletteActions` →
`arm_or_fire` → `fire`) and await the operator's hands + the deferred
pilot scenario for interactive verification.
