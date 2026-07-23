# D3 — the ink layer: stream-time annotation on the deck

Parent: roadmap Program D (census #1). Ledger #106. After D2.
Worktree: `C:\localdev\dpl-main` on main.

## 1. Context

Presenting on the 75", the operator draws ON the slide — circling a
number, underlining a word — live, on camera. The old repo's
`annotation-layer.tsx` (read in full this session, 112 lines) is the
port source: pointer strokes with pressure width, the annotate ink
token color, a Clear chip, strokes ephemeral (cleared when the SLIDE
changes, surviving step changes), pointer events captured only while
enabled.

## 2. Current state (ground-truthed)

- Old layer: width `3 + pressure*4`, color `--ink-annotate` (fallback
  `oklch(0.96 0.02 80)` — warm near-white), round caps, per-slide key
  clears, Clear button bottom-right, `pointer-events-none` when off.
  Old DeckHost: `A` toggles; EdgeNav hidden while annotating (ink owns
  the pointer); toolbar ToggleButton.
- New `dpl-deck` (D2, this session): keyboard map reserves A ("D3");
  edge zones registered before toolbar (later interact wins); toolbar
  `tool_button`s; per-beat clock. `DeckHost { pos, chrome_hidden,
  navigator_open, ... }`.
- egui input: touch events carry `force: Option<f32>`; mouse has no
  pressure — the old layer's `e.pressure || 0.5` default ports as
  force-else-0.5. Strokes must be RETAINED data (egui repaints every
  frame; there is no canvas to accumulate on).
- `cl::ink` has STRONG/BASE/MUTED/SUBTLE — no annotate token yet.

## 3. Target design

- **`cl::ink::ANNOTATE`** — the authored annotate stroke color (the old
  fallback's warm near-white, one const, icon-law style).
- **`dpl_deck::ink` module** — pure retained data + paint:
  `Stroke { points: Vec<(Pos2 normalized, f32 width)> }`;
  `InkLayer { strokes, current }` with `begin/extend(pos_norm, force)/
  finish/clear`; width = `3 + force.unwrap_or(0.5) * 4` (verbatim);
  `paint(ui, rect)` maps normalized points into the live rect (ink
  survives resize — an improvement the retained model gives for free)
  and draws per-segment `line_segment` with the segment's width, round
  caps via endpoint circles.
- **DeckHost integration:** `annotate_on: bool` + `ink: InkLayer`;
  `A` toggles (the reserved key); ink CLEARS when `pos.index` changes
  (never on step changes — the old slide-key semantics); while ON: a
  full-rect drag interaction registered AFTER the edge zones (it wins —
  edge taps can't fire mid-stroke, the old "EdgeNav hidden while
  annotating"), chevrons not painted, an "Ink" toolbar toggle (plain
  word, UI LAW — no glyph guessing) and a "Clear ink" chip bottom-right
  while on. PilotIds `deck.ink`, `deck.ink.clear`. No PilotClock —
  strokes are static (nothing animates).

Per-slide opt-out (`allowAnnotate`) is the same authored show-kind
doctrine as D2's deferred `chrome:"hidden"` — one TODO(doctrine) pass
covers both; NOT invented here.

## 4. Changes, in execution order

1. `crates/coldlight/src/lib.rs` — `ink::ANNOTATE` const.
2. `crates/deck/src/ink.rs` (new) — the module above + unit tests:
   width formula (force/no-force), begin/extend/finish round-trip,
   clear, normalized round-trip through two different rects.
3. `crates/deck/src/lib.rs` — field + A key + slide-change clear +
   drag capture + paint order (ink above slide, below rail/toolbar) +
   toolbar "Ink" toggle + Clear chip + chevron suppression + PilotIds.
   Tests: A toggles; slide advance clears strokes, step advance keeps
   them (the old slide-key law, machine-checked).
4. `cargo green`; exe proof `--present how-ai-works` (toolbar shows
   Ink); commit; push backgrounded.

## 5. Boundaries

🚫 No persistence of ink (ephemeral BY DESIGN — v1 scope, old parity).
🚫 No per-slide opt-out yet (authored-doctrine pass). 🚫 No color/width
pickers (C4 ViziWiz owns rich ink editing; THIS is stream markup).
🚫 Editor/walls untouched — ink is a DECK feature.

## 6. Verification

Green incl. new ink units + the slide-vs-step clear test. Exe proof:
the deck toolbar carries the Ink toggle. (A real drawn stroke needs the
pilot gesture extension — same named deferral as D2's tap flow; the
data layer is fully unit-proven.)

## 7. Commit plan

`feat(deck): D3 — the ink layer: stream-time annotation` on main; push
backgrounded.
