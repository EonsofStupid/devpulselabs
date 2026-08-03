# W3 — presets: the 6th token source + the presets wall

Parent: docs/plans/authoring-architecture.md (ratified). Ledger #104.
Worktree: main (the shared checkout is on the Codex seat's branch).

## 1. Context

A preset is a NAMED BUNDLE OF REFS into the generated registries — "the
presets pulled from all of that well organized auto generated styling"
(operator). Data over layer 1, zero new code per preset. W4's trays offer
presets as one-tap looks; the editor's "Save as preset" later appends to
the authored file. W3 delivers the source, the generated registry, and the
gallery-gated wall.

## 2. Current state (ground-truthed this session)

- 5 token sources generate today (hues/fluid/motion/moods/fonts) —
  `crates/coldlight/build.rs`, one `generate_*` each, lint-clean output.
- Registries runtime-enumerable: FONTS (13), MOODS (10), motion effects
  (18 incl. blend-drift + enter-rise/fade/bloom), FLUID_TOKEN_NAMES.
- Library hub walls: pool/templates/moods/fonts (`crates/hubs/library`).
- `cl::mood_blend(painter, rect, a, b, mix, pan)` paints a static blend
  when given fixed mix/pan.

## 3. Target design

- **tokens/presets.json** — two families:
  `look` {id,label,blurb,font,moods[2],motion,ambience?} and
  `motion` {id,label,blurb,enter}. Every value is a registry id.
- **generate_presets()** — cross-validates AT BUILD against the other
  authored sources (unknown font/mood/effect/ambience = build error, the
  same manifest⇄truth law fonts use) and emits `THEME_PRESETS` /
  `MOTION_PRESETS` statics + counts.
- **Runtime**: `ThemePreset`/`MotionPreset` structs + `theme_preset(id)` /
  `motion_preset(id)` lookups; tests pin id-uniqueness and that every ref
  resolves through the live registries.
- **Presets wall** (Library → Presets): each look preset = a card painted
  with its own static mood-blend, label typeset in its own font, chips for
  ambience/motion; motion presets as a labeled row. Gallery gate: a preset
  that exists renders here.
- Pilot note (honest): the wall adds no new interactive surface, so no new
  PilotIds; its renderer-keyed baseline joins the capture set when the
  Lavapipe lane runs walls — screenshot proof is the interim fallback per
  the roadmap.

## 4. Changes, in order

1. create `crates/coldlight/tokens/presets.json` — 5 look presets
   (neon-intro, glacier-stat, arcade-flash, warm-editorial,
   fuchsia-signal) + 3 motion presets (calm-rise, soft-fade, bloom-in).
2. edit `crates/coldlight/build.rs` — `generate_presets()` + rerun-if.
3. edit `crates/coldlight/src/lib.rs` — structs, include, lookups, tests.
4. edit `crates/hubs/library/src/lib.rs` — PRESETS subroute + wall.
5. `cargo green`; exe proof `--route library/presets`; commit; push.

## 5. Boundaries

🚫 No editor application of presets (W4). 🚫 No "save as preset" writer
(W4+). 🚫 Codex branch/worktree untouched.

## 6. Verification

green exits 0 (build-time cross-validation + runtime resolution tests);
`--route library/presets --screenshot docs/proof/w3-presets.png` shows all
5 look cards each in its own font over its own blend + the 3 motion rows.

## 7. Commit plan

`feat(coldlight): W3 — presets, the 6th token source + the presets wall`
on main, pushed.
