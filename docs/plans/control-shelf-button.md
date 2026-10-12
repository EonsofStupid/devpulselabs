# Pass — the Control shelf gets its first element, and the editor's interior is rebuilt on it

**Spec:** `docs/design/design-system.md` — §1 Elements, §6.1 (the word
"element", ratified 2026-07-27), §7 build order step 2.
**Sibling laws:** `docs/design/doctrine.md` (treatments) ·
`docs/design/architecture.md` §1 (layer stack) + §6 (the audit).

## 1. Context — why this pass exists

The operator opens the card editor and reports it looks the same as it did a
week ago. It does. `dpl_editor::Editor::ui()` last changed **2026-07-22**
(`17b8963`). Everything since — #141, #149, #151, #155, #159, #160 — built the
overlay modal, the tab rail, the tabs and the live pane *around* it. #176 gave
the authoring surface real controls, but landed them in
`crates/hubs/workbench/src/controls.rs`, which the editor cannot see:

```
git grep -c 'controls::' main -- crates/editor/src/lib.rs   →  0
```

So the editor still hand-paints its own. That is not a style opinion; it is a
written violation. `architecture.md` §1 puts `dpl-editor` in SURFACE crates,
whose one-line contract is *"ALL paint via ColdLight — no raw hex, no ad-hoc
frames."* The editor imports ColdLight 50 times and calls its compositor zero
times:

```
cl::ink:: 13 · cl::surface:: 10 · cl::space:: 12 · cl::RADIUS 10 · cl::Tone 5
cl::compose 0 · TraitState 0 · Fluid 1
```

It uses the token pool and hand-assembles the look. The law existed; nothing
enforced it, so it drifted for nine days without a red gate.

This is also not one file's problem. Workspace-wide, **four** files call
`compose()` — `author.rs`, `controls.rs`, `library.rs`, `viziwiz/palette.rs`.
Out of 68 crates. Every other surface reaches past the system into raw tokens
and invents its own hover, which is why a census of pressable things found
**six** competing implementations (`shell::icon_button`, `editor::chip`,
`editor::big_button`, `library::chip_row`, `composer::chip`,
`author::tab_chip`) before `controls.rs` became a seventh.

## 2. Current state (ground-truthed in-session, `460936d`)

**The shelf already exists and is deliberately empty.**
`crates/slides/src/elements/census.rs:36` declares
`ElementCategory::{Type, Mark, Meter, Divider, Media, Container, Control}`,
documented: *"Populated shelves paint; empty shelves stay honestly empty
(`Container`/`Control` arrive by promotion / with recipes, later build-order
steps)."* The element library holds `badge · bar · divider · eyebrow · media ·
stat` — six on four shelves. **Control has nothing on it.**

**The element contract is written** (design-system §1): standalone (reads only
its arguments, never global state) · `element(ui/painter, Fluid, props,
treatment, fidelity)` · **"never hand-rolls a treatment — it *receives* a
`ResolvedStyle`"** · categorized · registered in the census, which the Library
wall renders and a headless render gate exercises.

**`controls.rs` breaks that contract in one specific way.** It is otherwise
close to right — measures its own galley, honours `Fluid::tap_min()`, states
through `TraitState`, names itself via `widget_info` — but line 69 is
`cl::compose(tone, "neubrutalist", &[], state)`. The treatment is hard-coded
inside the element. Per §1 the caller supplies it.

**The editor's hand-painted surface, exactly:**

| site | what | magic |
|---|---|---|
| `lib.rs:816` `chip()` | 8 call sites (411, 459, 526, 538, 550, 555 ×2) | font 13.0, pad 22.0, height 30.0 |
| `lib.rs:844` `big_button()` | 1 call site (336, "Undo") | 84.0 × 30.0, font 13.5 |
| `lib.rs:383` look tile | inline in `look_tray` | 150.0 × 84.0 |

**Two real gaps, both found by reading rather than assumed:**

1. **`TraitState` is `Idle | Hover | Active` — there is no `Disabled`.**
   `big_button(ui, "Undo", can_undo)` dims and swallows taps when disabled.
   Nothing in the trait system expresses that today.
2. **`dpl-slides` has no `[dev-dependencies]`.** The census render gate cannot
   drive a *control* (which needs click/hover simulation) without
   `egui_kittest`, which `hub-workbench` already carries.

**Dependency direction is already legal.** `dpl-editor` depends on
`dpl-slides` (its `Cargo.toml` lists it), and `hub-workbench` uses
`dpl_slides::motion::Clock`. Both can consume an element from the library with
no new edge and no new crate — the crate count stays 68.

## 3. Design — the shelf's first element, and the treatment comes from outside

**The button is an element, not a helper.** It lands as
`crates/slides/src/elements/control.rs` on the `Control` shelf, registered in
the census like every other element, so it appears on the Library wall and the
render gate paints it. An element that isn't registered can't exist — that law
is what stops a seventh implementation appearing next week.

**The treatment is a parameter.** Signature follows §1's contract:

```rust
pub fn button(ui: &mut Ui, fluid: Fluid, props: ButtonProps<'_>, style: &ResolvedStyle) -> Response
```

The caller resolves `compose(...)` and hands the result down. Doctrine still
says actionable = neubrutalist, but that is now the *caller's* sentence, said
once at the surface, not a constant buried in the element. This is the single
change from `controls.rs` as merged.

**Disabled becomes a real state, minimally.** `TraitState` gains `Disabled`,
and the trait data resolves it the way the other states resolve — not a special
case in paint code, and not an alpha multiplier applied at the call site. A
disabled control also stops sensing clicks, which is behaviour, not paint.

**The editor's three hand-painted things go, they are not ported.** The 8
`chip` sites become the button element in its selected form; the one
`big_button` site becomes the button element with `Disabled`; the 150×84 look
tile keeps its identity as a *preview* (it paints a mood blend, which no button
does) but takes its size from `Fluid` instead of two literals.

**`controls.rs` is deleted, not left as a shim.** Its two call sites in
`author.rs` move to the element. No re-export shims on moves is a standing law.

**The tap target is the risk, and it is faced not dodged.** The editor's chips
are 30px tall in dense trays; `tap_min()` is ≥44. Trays will get taller. That
is the correct direction (touch-first is a law) but it is a layout change to
prove at real window sizes, not to discover later.

## 4. Scope

**In:** the `button` element on the `Control` shelf + census entry + render
gate · `TraitState::Disabled` and its trait-data resolution · `controls.rs`
deleted and `author.rs` moved onto the element · the editor's `chip`,
`big_button` and the look tile's literals removed · `egui_kittest` as a
`dpl-slides` dev-dep · tests · proof at two window sizes.

**Out, and named:**
- **The other four hand-painted controls** (`shell::icon_button`,
  `library::chip_row`, `composer::chip`, `author::tab_chip`). They are the same
  defect and they are a migration, which design-system §6.3 ratified as
  **BIG-BANG** — one concentrated pass, not opportunistic drive-bys. This pass
  proves the element on two crates; that pass moves the catalog.
- **`toggle` and `stepper`.** §1 names them on the Control shelf. Promotion
  rule: extract when it recurs. Neither recurs yet.
- **The treatment layer** (`tokens/treatments.json`, build-order step 3).
  Callers pass a composite treatment by id today, exactly as `author.rs`
  already does.
- **Recipes and fidelity** (steps 4–5). `cost_tier` is `Baseline`, like every
  element currently registered.
- **The editor's information architecture.** This pass changes what its
  controls *are*, not what tabs or trays exist.

## 5. Work plan (atomic, in order — commit at each step that compiles)

1. `TraitState::Disabled` + its resolution in the trait data; unit tests that a
   disabled style differs from idle on the axes the doctrine names.
2. `elements/control.rs` — `button` taking `&ResolvedStyle`; census entry;
   `egui_kittest` dev-dep; render-gate + tap-target + `widget_info` tests.
3. `author.rs` onto the element; **delete** `controls.rs`; workbench tests green.
4. The editor's 8 `chip` sites → the element; **delete** `chip`.
5. The editor's `big_button` site → the element with `Disabled`; **delete**
   `big_button`. Assert a disabled control does not report a click.
6. `look_tray`'s 150×84 → `Fluid`-derived.
7. `cargo green`; proof shots at 1280×800 and 1100×700; §8; `docs-history-2026-10-12:docs/STATE.md`.

## 6. Risks

- **The tap target reflows the editor's trays.** Mitigation: shots at both
  sizes are acceptance, not decoration; if a tray overflows, the tray scrolls
  — the control does not shrink below the token.
- **`TraitState::Disabled` is a token-layer change with workspace blast
  radius.** Mitigation: it is additive, and every existing `match` on
  `TraitState` is in the four `compose()` callers named in §1 — a small, known
  set the compiler will point at.
- **The look tile is not a button and could be forced into one.** Mitigation:
  it stays its own paint; only its literals change. Elements are promoted when
  they recur, and a mood-blend preview recurs nowhere yet.
- **This pass could sprawl into the big-bang migration.** Mitigation: §4 names
  the four out-of-scope implementations explicitly; touching them fails review.
- **A "moved" control could silently lose behaviour** (the exact failure that
  produced this pass — #130's editor was hand-salvaged and its interior was
  dropped). Mitigation: step 3–5 each delete the old function in the same
  commit that rewires its callers, so a missed call site is a compile error
  rather than a leftover.

## 7. Verification / acceptance

- `cargo green` clean on both lanes.
- `git grep -n 'fn chip\|fn big_button' crates/editor/` → **no hits**.
- `git grep -rn 'controls::' crates/` → **no hits**; the file is gone.
- The `Control` shelf is non-empty in the census, and the census drift test
  passes.
- A test that the button clears `Fluid::tap_min()` at two window sizes.
- A test that a `Disabled` button reports **no** click — the `big_button`
  behaviour must survive the move, by assertion, not by inspection.
- A test that the button is findable by its label through `widget_info`.
- Real-exe proof at 1280×800 **and** 1100×700 of the editor's Words tab with
  its tray, showing filled, edged, tap-sized controls.
- The operator can state, looking at the shot, that the editor's controls read
  as pressable. The feel bar is part of acceptance (coldlight-system skill).

## 8. Evidence

_(filled on completion — gate output, test names, proof paths, findings)_
