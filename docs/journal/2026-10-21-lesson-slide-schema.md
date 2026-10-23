# 2026-10-21 — lesson-slide-schema: LS1, component contracts

Milestone `lesson-slide-schema`, pass LS1. Plan: `docs/plans/lesson-schema-ls1.md`,
APPROVED 2026-10-21. It is never committed, and is deleted when it lands.
The operator's answers:
- Q1 "Approve";
- Q2 "Lessons build episodes";
- Q3 "Upgrade once, keep backups";
- Q4 "Components first, real and gated".

Research: `docs/research/lesson-slide-schema.md` (untracked; it ships with
LS1). Checkout: **`D:\projects\devpulselabs`**, branch
`feat/ls1-component-contracts` from `4ee638b` (main, #279 RM1).

`chat-cw3-rooms` is **parked** after RM1 (merged #279; exe rebuilt 15:46,
stamp `4ee638b · 2026-10-20 19:45z`). Next there: a Codex-smoothness pass
(offered, not yet approved), then CW2, then RM2–RM5.

## Done so far

- **Research** with four agents. Key ground truth was re-checked by hand: a
  slide has no id, episodes copy slide folders, `apply_look` bakes, the
  renderer never reads `template`, and lessons are not stored.
- **The model** (plan §4.1) has five layers: look, component, entity preset,
  slide, and lesson or episode. Lessons build episodes. The rules: pin
  content and float presentation; variants at the point of use; render
  locks.
- **The LS1 design, decided while grounding:**
  - **`dpl.component/1`** (`schemas/component.v1.schema.json`) holds:
    - `id` (it equals the template id), semver `version`, `name`, `summary`;
    - `class`: today's 4 classes, with infographics tagged `infographic` so
      the interactive budget doesn't change;
    - `slots[{id, label, entity, required, tooltip, limits, item}]`;
    - `props[{id, label, kind, default, tooltip, min, max, options}]`;
    - `variant_axes[{id, label, tooltip, options[{id,label}], default}]`;
    - `forbidden`, `tags`;
    - `examples[{id, label, content, props, variant}]`;
    - `provenance{forked_from?, note?}`.
  - Slot, prop and axis ids are snake_case, matching the template's serde
    field names.
  - **Templates:** each template struct derives `Deserialize` and
    `Serialize`, with `#[serde(skip)]` on runtime state. `demo()` becomes
    `contract::from_example(contract(), 0)`, which merges the example's
    maps, deserialises, then calls `restart()`. Interactive templates' state
    vectors must be sized in `restart()`.
  - **`SlideTemplate::spec()`** returns `&'static ComponentContract`.
    `TemplateSpec` and its duplicated label and blurb go. Workbench
    `library.rs` and `author.rs` read `spec.id`, `name`, `class` and
    `summary`.
  - **`Tone`** gets a serde helper in slides (by hue id). **`Media`** becomes
    `{kind: image|video, src}` or `{kind: anim, ambient}` with owned
    strings.
  - `dpl-slides` has no structure ceiling, so adding serde is free. The new
    `contracts` phase in `xtask green` still needs the `architecture.md`
    amendment and the audit table in the same commit.

## LS1 built (2026-10-21)

- **All 28 contracts** are in. Four helpers converted the templates in
  parallel, each in its own folders, without cargo. A script then checked
  the old `demo()` literals at `4ee638b` against the new examples: all
  strings match, and the only numbers not carried are runtime counters.
- **The contract model moved to `dpl_doc::component`**, so the egui-free
  `xtask` gate shares the one evolution rule. Item fields gained `numbers`
  and nested `list`, and `data` slots declare their object's fields.
  `predict.rounds` became a list with a minimum of 1, which closed an
  empty-list panic.
- **The Workbench solo view** shows the contract beside the stage. It opens
  by default at an area width of 1100 or more; otherwise the "contract"
  toggle opens it. Templates are clipped to their stage.
- **Proof:**
  - doc: 7 component tests; slides: 30 tests; workbench: 55 tests;
  - clippy clean in both feature sets;
  - EguiPilotAI `workbench-component-contract` (fixture `library`) and
    `-1100` (fixture `library-1100`) both pass;
  - shots in `docs/proof/ls1-component-contract-{1440x900,1100x700}.png`.

## LS1 in PR, LS2a built (2026-10-21)

- **LS1 is PR #280** (`7320100`). The local gate passed. **CI is blocked:
  both estate runners are OFFLINE** (`devpulse-runner-win-01`,
  `devpulse-runner-lnx-01`; `gh api repos/EonsofStupid/devpulselabs/actions/runners`).
  The jobs have queued since 15:11Z. The operator must bring Proxmox VM 119
  and LXC 121 back; never merge without double-green.
- **The LS2 plan** (`docs/plans/lesson-schema-ls2.md`, untracked) was
  APPROVED 2026-10-21:
  - Q1 "two PRs";
  - Q2 "bring me a proposed list";
  - Q3 "seeds read-only".
  It was amended to the operator's ratified blocks shape: blocks ARE the
  slots, and every block is an entity.
- **LS2a**, on branch `feat/ls2a-slides-by-id` stacked on LS1:
  - **`dpl.slide/3`:** `id` (ULID, `crates/doc/src/id.rs`), `rev`,
    `component id@major`, `variant`, per-entity preset and tone,
    `Block::{Text, Media, Quote, Data}` with an optional size,
    `objective_ids`, `role` (Gagné), tags, provenance, recipe, ext.
  - **The loader** reads the schema first: an old file is `UpgradeNeeded`,
    a foreign one is `Schema`.
  - **New in `dpl-doc`:**
    - `validate_against` the contract;
    - `slide_from_example`;
    - the library at `content/library/slides/<id>/`;
    - `fork` and `fork_into_library`; `place_slide` and `clone_episode`
      fork their copies.
  - **`crates/doc/src/legacy.rs`:** the one-time upgrade (title, stat and
    quote id maps; every size kept; a `slide.v2.json` backup; inbox drafts
    move into the library). The app runs it at startup (`main.rs`). The 8
    seeds were upgraded with `cargo xtask upgrade-slides seeds`; their
    backups were removed, since git history is theirs.
  - **Writers:**
    - the editor raises `rev` on every change, undo is a new revision, quotes
      are a real block plus name and role slots, and the picture goes in the
      `media` slot;
    - the Workbench mints into the library and starts from the tapped Layout
      or Starter;
    - the seed picker forks;
    - composer make-slide goes into the library;
    - workflows' `slide_for_slot` uses the title and subtitle slots.
  - **`schemas/slide.v3.schema.json`** is checked by `dpl-doc` tests.

## LS2b built (2026-10-21)

- **Operator ruling 2026-10-21:** "Card look keeps a font default", like a
  slide master. It is recorded in the LS2 plan §4.4.
- **On `feat/ls2b-presets-by-reference`, stacked on LS2a:**
  - `presets.json`: the 5 theme looks gain `for: card`, `exposes` (5 knobs)
    and `tone`. `build.rs` checks the entity, 4–7 knobs, colour, legal
    knobs per entity, and the tone against the hues.
  - `dpl_doc::card_preset` plus a three-tier `resolve`: slide → look
    (background, tone, font as the line default) → show. Unknown presets
    are named; block presets are refused until the text, picture and quote
    sets are approved.
  - **Editor:**
    - `apply_look` is deleted, and `use_look` stores the id and clears the
      overrides it governs;
    - the tray marks the chosen look and lists its knobs in plain words;
    - the inherited background and font go through the cascade;
    - look cards carry pilot ids (`editor.look/<id>`) and accessibility
      labels.
  - **Proof:**
    - 1079 workspace tests; clippy clean both ways;
    - EguiPilotAI `editor-look-by-reference` passes: the slide stores
      `preset: neon-intro` at rev 2, with nothing copied;
    - shot: `docs/proof/ls2b-look-by-reference.png`.

## Linked looks (milestone `linked-overrides`, 2026-10-21)

- **The operator (R286):** "link them, but advanced users can break the link
  and have full granular control … latest UI trends, effortless".
- **Research and grounding are done.** The plan is
  `docs/plans/linked-looks-lk1.md` (untracked), APPROVED with:
  - Q1 "Approve";
  - Q2 a built-in look the operator pushes back to becomes their own
    version, replacing it for them, with "Back to the built-in";
  - Q3 the preset sets approved as written (LK3).
- **The model:** a derived state (follows · with changes · own style);
  `resolve_traced` with a source per knob; reset, reset all,
  stop_following and follow_again; the look chip and menu; a dot on every
  value that is yours; try-before-keep; "More controls" gated by `exposes`;
  one revision per edit burst.

## LK1 built (2026-10-22)

- **`dpl_doc::link`** (new): `Source`, `Knob`, `LinkState` (derived, never
  stored), `source`, `changes`, `link_state`, `reset`, `reset_all`,
  `stop_following` (copies the look's values, so nothing changes on screen,
  and sets `provenance.unlinked_from`), `follow_again(keep_my_changes)`.
  The schema gains `unlinked_from`.
- **Departures from the plan's letter (same truth, recorded for the PR):**
  - `resolve_traced` became `link::source(slide, knob)`, one knob at a
    time; `resolve` stays the paint path.
  - The dot opens a small floating sheet ("Yours: … / The look's: …"),
    and the "Use the look's" chip beside it is the one-tap way back. The
    first cut gave the value back on a dot tap, which surprised; the sheet
    is what the plan described.
  - The stage long-press compare is not built; Keep / Cancel covers looks.
- **The editor:** the look chip ("Trying: Neon intro" while a look is on
  trial) and its menu; Keep in the Hero weight; the dot, sheet and give-back
  chip on Background, Font, Size and Ink; "More controls"; typing settles
  into one revision after a 1.5 s pause; tooltips and pilot ids on every
  touched control (`editor/*`, `editor.yours/*`, `editor.compare/*`,
  `editor.font/*`, `workbench.author.swatch/*`).
- **The Workbench:** every tab but Words shows the chip and Undo; the
  Background tab shows the background the slide SHOWS, with the dot.
- **Proof:**
  - tests: doc 56, editor 11, Workbench 55; clippy clean;
  - EguiPilotAI `linked-looks` and `linked-looks-1100` pass (47 steps each),
    and `editor-look-by-reference` passes with its new Keep step;
  - on disk the run ends at rev 6 with `preset: neon-intro`, no background
    and no `unlinked_from` (follow again cleared it);
  - shots: `docs/proof/lk1-trying.png`, `lk1-one-change.png`,
    `lk1-compare-1100.png`, `lk1-own-style-1100.png`.

## LK2a built (2026-10-22, milestone `own-looks`)

- **Plan** `docs/plans/linked-looks-lk2.md` (untracked), APPROVED 2026-10-22:
  "Approve, two PRs"; Open the look edits "In the card editor".
- **Built on `feat/lk2a-own-looks`** (from the LK1 branch): `dpl.look/1`
  and its schema, `dpl_doc::look` (`Look`, `Looks`, `validate_look`,
  `validate_looks`, `save_as_new_look`), `dpl_doc::Styling`; `card_preset`,
  `resolve`, the `link` verbs and the renderer take the looks; the editor
  holds a `Styling` and the content root (`Editor::open(dir, content_root)`),
  `DeckHost::open(dir, looks)`; Save as a new look with its name sheet; your
  looks first in the tray and the Library; the missing-look state.
- **Proof** is in the plan's §8: tests, `own-looks` (+1100) on the real exe,
  the LK1 scenarios still passing, the look and both slides on disk, four
  shots in `docs/proof/lk2a-*`.

## Exact next step

1. **LK1 is draft PR #283** (a6a2378, base `feat/ls2b-presets-by-reference`).
2. **LK2a:** commit on `feat/lk2a-own-looks`, push through the gate, rebuild
   the exe from `main`, open a draft PR based on `feat/lk1-linked-looks`.
3. The merge order once the runners return: #280 → #281 → #282 → #283 →
   LK2a, each retargeted to `main` (`gh pr edit <n> --base main`),
   double-green, the exe rebuilt after each, each plan file deleted as it
   lands.
4. Then LK2b (plan §4.4–4.6): the spread sheet with its count, before and
   after, the two-part undo step, your version of a built-in, Back to the
   built-in, and editing the look in the card editor.

## Landmines

- **The pre-push gate rebuilds `target/release/devpulselabs.exe` from the
  BRANCH.** LS2a's exe upgrades content at startup, so after every push,
  rebuild from `main`: `git checkout main && cargo build --release -p
  devpulselabs -p dpl-lottie-render`, check the stamp, then check the branch
  out again. Last done 2026-10-22 00:18 after the LK1 push (stamp
  `4ee638b · 2026-10-22 04:14z`).

- **Codex findings of 2026-10-20 (not yet in any doc; they go into
  `status.md` with LS1):**
  1. Adoption copied only `bin/`. The Codex package layout v1 also has
     `codex-resources/` (`codex-command-runner.exe`,
     `codex-windows-sandbox-setup.exe`) and `codex-path/` (`rg.exe`). Without
     them, sandboxed shell commands fail and ask to run unsandboxed. This
     was proven A/B with the probe
     `scratchpad/cw1/codex_probe.py`.
  2. Unmapped app-server notifications become coral UNKNOWN cards (about 14
     per short turn).
  3. Codex's working directory is the app's current directory (the build
     folder when double-clicked), and `thread/start` sends no `cwd`.
  4. There is no `thread/resume` or `thread/list`, so every launch starts a
     new thread.
- **Research on shared state (2026-10-20):** don't drive the desktop apps.
  Keep the room log as the truth, injected through Codex
  `thread/inject_items` and Claude's persistent stream-json. Optionally add
  an MCP room server for guests. `--bare` will become the default for
  `claude -p` and ignores the subscription login.
- The "grounder" agent type fails (its model is unavailable). Use
  general-purpose.
- `.vscode/` and `out/` are untracked operator files; never add them.
- **Capture timing:** a capture taken before the title card's rise-in
  entrance ends shows almost no text. Scenarios wait 1.5 s before capturing.
- **The title card's subtitle does not wrap.** At 1100x700 with the contract
  open, it is clipped at the stage edge. Real slot layout comes with LS4.
- **Tapping a look only TRIES it now (LK1).** Any scenario that picks a look
  must click `root/editor/keep` after it.
- **Own-look scenarios find the look by its name** (`role = "button"`,
  `name = "Orchid intro"`), and name locators are strict: clear
  `target/eguipilotai/ls2a/content/library/looks` before each run.
- **The estate runners are offline** (server1 unreachable on port 22 on
  2026-10-22; the operator is bringing it back). #280's run timed out after
  24 h without starting; rerun it (`gh run rerun 39623397906`) once both
  runners are online.

## Verify commands

- `git -C /d/projects/devpulselabs status --short` and `git log --oneline -2`
  (expect the branch on `4ee638b`).
- `cargo test -p dpl-slides` once the contracts land.
