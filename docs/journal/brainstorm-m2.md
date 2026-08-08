# BS1 · M2 journal — the surface (2026-08-07)

**Objective (POAM):** the operator types "brainstorming" → a phased
editable board populates inline → every gesture is a typed op with undo
→ Converge promotes into the explainer intake pre-fill.

**Done this milestone:**
- **Trigger** (`parse_brainstorm_trigger`, window): whole-buffer
  `brainstorming`/`brainstorm`/`/brainstorm` at Ai submit — the
  `parse_operator_slash` provenance law verbatim (pattern-in-string
  never escalates); rides the ONE submit path beside slash, enabled
  even without a provider session.
- **Intents**: `ChatWindowIntent::{StartBrainstorm, Brainstorm(op),
  BrainstormUndo, BrainstormRedo, PromoteBrainstorm}` → pure `apply()`
  pass-throughs → `HostIntent` counterparts.
- **The board paints inline** in `ai_conversation` from a host-pushed
  `BrainstormView` (the H1 ContentView projection shape): phase header
  + honest status line + Undo/Redo; seed row; per-phase bodies —
  Diverge = tree-indented editable item rows (commit-on-focus-loss
  drafts, breakdown ↳, cascade ✕, add row), Cluster = unsorted rows
  with sort-into buttons + cluster lanes + add-group (label→kebab id),
  Converge = the ordered read-through + "Promote to intake". Assist
  items carry a cyan `assist` disclosure until an operator edit claims
  them. Every control tip'd + pilot-tracked
  (`chat.brainstorm.*` semantic nodes).
- **⊞ Component lives**: the pre-declared disabled button is now
  enabled in the Ai domain and opens the board — the Interaction
  Library's first connected component ("not connected in this slice"
  is finally false).
- **Host session** (`app/chat.rs`): `BrainstormSession { dir, doc,
  undo, redo, status }`, checkpoint pattern (dedupe, cap 100, redo
  cleared); ONE write path checkpoint→`apply`→`save_brainstorm`→
  project; refusals pop the checkpoint and surface their named issues;
  boards mint no-clobber in `<content>/inbox/brainstorm[-N]`;
  promote distills into `intake-prefill.json` (`IntakePrefillDoc v1`,
  the missing persistence half added to dpl-workflows).
- **Drill** `--brainstorm-drill <scaffold>` (pre-window, exit by
  verdict): mints, works every phase through `apply`, proves
  round-trip + validity + distill + prefill save. Live verdicts:
  run 1 `brainstormed breakdown → …inbox\brainstorm (3 ideas, 2
  groups, 3 prefills)`; rerun minted `brainstorm-2` (no-clobber);
  `ghost` refused naming the census.
- **Proof state** `ChatProofMode::Brainstorm` (+ parse + arrangement +
  the native-proof duplicated list + usage): a deterministic in-memory
  worked board at Cluster. Captured via `chat-native-proof --state
  brainstorm` → `docs/proof/bs1-brainstorm-board.png` (1300x800, the
  peer window's own geometry), READ per true-verify: phase visible,
  assist disclosure on the unsorted row (caught missing on the first
  capture, fixed, re-shot), sort buttons, editable lane row, disabled
  Add-group until text, Undo enabled/Redo disabled, ⊞ Component
  enabled. Committed as the baseline.
- **Censuses**: xtask harness-flag census green (46 flags — CLAUDE.md
  amended same commit); `dpl-chat-window → dpl-workflows` edge noted
  in architecture.md §1 (surface→document, the editor→dpl-doc
  direction; no ceiling row exists for surfaces).

**Proof:** 44/44 window (3 new: trigger provenance, intent
pass-throughs, projection) · 34/34 workflows · 37/37 app · fresh
`cargo clean -p` clippy zero-warning · structure audit green · drill
verdicts above · PNG baseline read.

**Deviations:** peer-window proof captures the window's OWN geometry
(one shot), not the two route sizes — the route-size law govern route
screenshots; compact mode is its own proof state. Logged in plan §9.1.

**Remaining (POAM):** BS-M3 — assist populate job (assist's first real
behavior), promote announce interaction, pilot scenario, close-out.
