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

## Exact next step

1. **LS2b is draft PR #282**, stacked on #281, which is stacked on #280.
   The exe was rebuilt from `main` at 18:01 (stamp
   `4ee638b · 2026-10-21 21:59z`).
2. **The proposal is waiting on the operator:**
   `docs/plans/lesson-schema-preset-sets.md` (untracked) lists 14 text, 6
   quote and 5 picture presets, made only from existing fonts, sizes, inks
   and tones. After approval: add them to `presets.json` with `for` set to
   text, quote or picture; lift the "block presets are refused" rule in
   `dpl_doc::validate`; the editor's text tray offers them.
3. **When the runners return**, merge in order #280 → #281 → #282. Retarget
   each to `main` after its parent merges, wait for double-green, and
   rebuild the exe after each merge. The watcher task (bz2mu1rfp) polls
   #280.
4. Then LS3: lessons and episodes by reference.

## Landmines

- **The pre-push gate rebuilds `target/release/devpulselabs.exe` from the
  BRANCH.** LS2a's exe upgrades content at startup, so after every push,
  rebuild from `main`: `git checkout main && cargo build --release -p
  devpulselabs -p dpl-lottie-render`, check the stamp, then check the branch
  out again. Done 2026-10-21 15:52 (stamp `4ee638b · 2026-10-21 19:51z`).

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

## Verify commands

- `git -C /d/projects/devpulselabs status --short` and `git log --oneline -2`
  (expect the branch on `4ee638b`).
- `cargo test -p dpl-slides` once the contracts land.
