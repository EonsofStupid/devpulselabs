# Move and organize: components on one pattern, data in `.userdata` and `.ai`

**R311 (2026-11-24/25).** **Status:** M1 is built on
`feat/ls4m-components-structure`; M2–M8 are written out to execute and wait
for the operator's OK on the two decisions at the end.

This is the one work queue for the move. It does not restate design that
already has an owner: component layout is
[`components-ls4.md`](components-ls4.md) and
[`design-system.md`](../design/design-system.md) §4.7; storage layout is
[`portable-storage.md`](portable-storage.md) §4.1.

## 1. Context

R311 asks for one fully written plan, the move finished, boring and
predictable folder patterns, colocated code, errors surfaced by the move and
then fixed, and no compatibility or legacy code. It also expects `.ai` and
`.userdata` folders inside the library.

## 2. Goal and non-goals

**Goal.**
- Every slide component lives in one folder named by its id, with the same
  file names, and is painted by the one renderer on real slides.
- Every byte the app stores lives inside the library, in `.userdata` (app
  state) or `.ai` (what AI made or learned), grouped per project.
- The old paths are deleted in the same PR that replaces them.

**Not in this plan:** new components, wider parameters and media slots, your
own templates (LS4d), loading contracts at runtime, accounts beyond "me".
Each is its own pass after this one.

## 3. Ground truth (read or run 2026-11-25)

| Fact | How it was checked |
|---|---|
| 28 components; 6 are painted from their contract (title, section, quote, stat, callout, media); 22 hand-paint in Rust and only in the gallery | counted from every `component.json` |
| A real slide's list or data content is not painted at all | `render/stack.rs`: a `Block::Data` is skipped; status known-broken #12 |
| Folder names did not match contract ids (`big_number` was `stat`), layouts and charts sat in two folders, and `lib.rs` and `elements/mod.rs` re-exported other modules' items | read; fixed in M1 |
| The contracts gate compared by file path, so a moved folder silently skipped the only-grows check | `crates/xtask/src/main.rs`; fixed in M1 |
| THE checkout still uses the old layout: `content\`, `assets\` and `.userdata\` at the repo root; there is no `library\` and no `roots.toml` | `ls` at `D:\projects\devpulselabs` |
| No `.ai`, `.usr` or `.aidata` folder exists anywhere | `ls`; the code reserves `<library>\.aidata` and creates it empty |
| AI data is written into `.userdata` today: `chat\accounts.db`, `chat\provider-profiles.db`, `chat\library\`, `profiles\provider-auth-*`, `analytics\`, `cache\imagine\` | `grep userdata_root()` across `crates/` |
| A project folder keeps its chat archive at `<project>\.devpulse\chat` | `crates/chat/projects/src/lib.rs` |
| Three names are on record for the AI folder: `.aidata` (storage plan), `.usr/.ai` (ruling 2026-11-14), `.ai` (R311) | the three documents |

**Assumed, not verified:** that nothing outside this repo reads
`.userdata\chat` or `.userdata\profiles`; the Linux lane for M1 (only the
hosted run proves it).

## 4. Design

### 4.1 Code: one component, one folder (M1, built)

```text
crates/slides/src/
  components/<id>/component.json   always: the contract
  components/<id>/mod.rs           always: the code
  components/<id>/paint.rs         only when it paints a body (lists, charts)
  components/<id>/state.rs         only when it is interactive
  chart/        what charts share
  render/       the one renderer, one job per file
```

A test fails the gate when a folder breaks the pattern. No file re-exports
another's items.

### 4.2 Code: a body painter per component (M2–M5)

The renderer keeps painting the text column from the contract's layout. A
contract gains `layout.body: { "slot": "<list or data slot>" }`. The column
anchors to the top, and the rest of the stage goes to that component's
`paint.rs`:

```rust
pub struct Body<'a> {
    pub painter: &'a egui::Painter,
    pub rect: egui::Rect,              // the stage under the column
    pub fluid: dpl_coldlight::Fluid,
    pub accent: egui::Color32,         // the card's colour
    pub font: egui::FontFamily,        // the card look's font
    pub values: &'a serde_json::Map<String, serde_json::Value>, // list/data slots
    pub props: &'a serde_json::Map<String, serde_json::Value>,
    pub variant: &'a std::collections::BTreeMap<String, String>,
    pub now: f64, pub t0: f64, pub step: Option<usize>, pub thumbnail: bool,
}
/// Returns one rect per item ("items/0", "items/1", …) for the editor.
pub fn paint(body: &Body) -> Vec<(String, egui::Rect)>;
```

Each component's existing paint code moves into its `paint.rs` and reads
`Body` instead of its own struct. The paint is not rewritten, so the gallery
look is kept (LS4a's first attempt failed that bar). The registry maps a
component id to its painter. The component's struct, its `ui()` and its
`SlideTemplate` impl are deleted; the gallery card is the contract's first
example painted by the renderer.

**The same seven steps for every component:**
1. `component.json`: add `layout` (column slots, `body`), raise the minor
   version.
2. Create `paint.rs`; move the body of `ui()` into `paint(&Body)`.
3. Register the painter (one line in `registry.rs`).
4. Delete the struct, `demo()`, `ui()` and the `SlideTemplate` impl.
5. Tests: its example as a real slide paints one rect per item, inside the
   stage, at full size and as a thumbnail.
6. Shots of the gallery card and a real slide at 1440×900 and 1100×700,
   beside the before shot.
7. `cargo green`.

### 4.3 Data: the library's final layout (M6–M8)

```text
<library>\
  library.json
  common\cache\  common\temp\          regenerable
  accounts\me\
    content\   assets\
    .userdata\                         app state only: index.db, estate\,
                                       policies, present-screens.json,
                                       composer state
    .ai\
      providers\accounts.db            was .userdata\chat\accounts.db
      providers\profiles.db            was .userdata\chat\provider-profiles.db
      providers\homes\<provider>\      was .userdata\profiles\provider-auth-*
      library\                         was .userdata\chat\library (roles, rooms)
      analytics\                       was .userdata\analytics
      projects\<project-id>\
        chats\  logs\  context\  media\
```

- `cache\imagine` stays under `common\cache` because it is regenerable.
- A project folder keeps only `<project>\.devpulse\chat\project.json`, a link
  to its `projects\<project-id>\` folder in the library.
- `dpl-roots` owns every path. `Layout.aidata` becomes `Layout.ai`, and
  `aidata_root()` becomes `ai_root()`. No other crate joins a path under
  `.userdata` for AI data.

## 5. Wiring: the queue

| # | Milestone (one PR each) | What moves | Deleted in the same PR |
|---|---|---|---|
| **M1** | Component folders (built) | 28 folders to `components/<id>/`; `infographics/{stage,tone_cycle}` to `chart/`; `document.rs` split into `render/`; callers in editor, deck, workbench and home name real modules; contracts gate follows `id`; the pattern test | `templates/`, `infographics/`, `document.rs`, every `pub use` in `dpl-slides` |
| **M2** | Lists | `Body`, `layout.body`, the registry's painter map; agenda, features, stats, timeline, code through the seven steps; list items selectable in the editor | those five structs and `ui()`s |
| **M3** | Columns | split, compare | their structs and `ui()`s |
| **M4** | Charts | the eight charts; `chart/` stays shared | their structs and `ui()`s |
| **M5** | Interactive | checklist, reveal, choice, quiz, cost-meter, predict, drag-match; interaction state moves to `state.rs`, live on the deck and at rest in thumbnails and the editor | their structs and `ui()`s; the `SlideTemplate` trait once nothing implements it |
| **M6** | `.ai` in the code | `dpl-roots`: `.aidata` to `.ai`, the sub-paths in 4.3 as functions; accounts, profiles, chat library, sign-in homes, analytics and the ledger write under `.ai` | `aidata`, every AI path under `userdata_root()` |
| **M7** | Chats per project | chats, logs, context and media under `.ai\projects\<id>\`; the project folder keeps only the link | the archive body under `<project>\.devpulse\chat` |
| **M8** | The operator's move | "Bring in my existing library" and `cargo xtask move-library` also split `.userdata` into `.userdata` and `.ai`; dry run first, then the operator taps **Move it** | reading the repo-root `content\`, `assets\`, `.userdata\` outside the drill seam |

Order: M1, then M2 to M5, then M6 to M8. M6 does not depend on M2 to M5 and
can go first if the operator wants the data folders before the components.

**Existing data in M6 and M7.** Pre-release, so there is no fallback read of
the old place. Each PR carries a one-time mover that runs on the operator's
tap (never at launch), copies, verifies byte for byte and leaves the old
files labelled, as `bring_in` already does.

## 6. Proof (every milestone)

- Unit tests named in its row, plus the folder-pattern test.
- `cargo green` locally before merge; `main`'s hosted run read after.
- M2 to M5: shots at 1440×900 and 1100×700 on a scratch library, before and
  after, and the LK and LS pilot scenarios re-run.
- M6 to M8: a scratch library before and after, listed file by file; the
  dry run's output on a copy of the operator's layout; a restart that finds
  every chat and sign-in.

## 7. Risks

- **A component looks worse after it moves.** The paint code is moved, not
  rewritten, and step 6 compares shots before merge.
- **The editor loses tap targets.** `paint` returns a rect per item; a test
  per component counts them.
- **Operator data.** M6 to M8 never delete; they run only on a tap; the
  drill seam keeps tests off the operator's library.
- **Sign-in homes move.** A provider that stores absolute paths in its home
  may need a fresh sign-in; M6 checks each one and says so.
- **Platform-only files.** `dpl-roots` has Windows and Linux branches; the
  hosted Linux lane is read after every merge.

## 8. Evidence

**M1** (2026-11-25, `feat/ls4m-components-structure`):
- `cargo check --workspace --all-targets`: no errors, no warnings.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p dpl-slides`: 43 passed (42 before, plus the pattern test).
  The renderer's own tests moved with it unchanged, including
  `every_seed_renders_through_the_one_path`.
- Full `cargo green` passes on Windows. Its contracts phase reads "28 found,
  28 compared with origin/main — every one only grew", so all 28 moved
  contracts were still compared.

## Decisions for the operator

1. **The AI folder's name.** This plan uses `.ai` beside `.userdata`, as
   R311 says, replacing `.aidata` and the 2026-11-14 `.usr/.ai`. Confirm, or
   say `.usr/.ai`.
2. **Order.** Components first (M2 to M5), or data folders first (M6 to M8)?
