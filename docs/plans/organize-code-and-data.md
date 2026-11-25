# Move and organize: components on one pattern, data in `.userdata` and `.usr/.ai`

**R311 (2026-11-24/25).** **Status:** M1 landed (#333). M6 is built on
`feat/m6-usr-ai`. M2–M5, M7 and M8 are written out to execute.

This is the one work queue for the move. It does not restate design that
already has an owner: component layout is
[`components-ls4.md`](components-ls4.md) and
[`design-system.md`](../design/design-system.md) §4.7; storage layout is
[`portable-storage.md`](portable-storage.md) §4.1.

## 1. Context

R311 asks for one fully written plan, the move finished, boring and
predictable folder patterns, colocated code, errors surfaced by the move and
then fixed, and no compatibility or legacy code. It also expects the AI and
`.userdata` folders inside the library, and names the 2026-11-14 ruling
(`<library>/.usr/.ai/`, grouped per project) as the one to carry out.

## 2. Goal and non-goals

**Goal.**
- Every slide component lives in one folder named by its id, with the same
  file names, and is painted by the one renderer on real slides.
- Every byte the app stores lives inside the library, in `.userdata` (app
  state) or `.usr/.ai` (what AI made or learned), grouped per project.
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
| The AI folder is `.usr/.ai` (ruling 2026-11-14, confirmed 2026-11-25); `.aidata` was the storage plan's earlier name | requirements R303, R311 |

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
    .userdata\                         app state: index.db, estate\,
                                       policies and settings, profiles\
                                       (browser and sign-in sessions),
                                       analytics\, composer state
    .usr\.ai\
      chat\accounts.db                 M6: was .userdata\chat\accounts.db
      chat\provider-profiles.db        M6: was .userdata\chat\provider-profiles.db
      chat\library\roles\ rooms\      M6: was .userdata\chat\library
      projects\<project-id>\           M7
        chats\  logs\  context\  media\
```

- `dpl-roots` owns the path: `Layout.ai` and `dpl_roots::ai_root()`. No other
  crate joins a path under `.userdata` for AI data.
- **M6 keeps relative paths the same** (`chat/…`), so a caller changes only
  which root it asks for.
- **Existing data is carried once, when the library opens** (`ai_carry.rs`):
  a carried folder that is in `.userdata` and not yet in `.usr/.ai` is copied
  and checked byte for byte. The source is only read and stays where it was.
  A failed copy removes what it made and the open reports it. Once carried,
  `.usr/.ai` is the truth and is never copied over.
- **Stays in `.userdata` for now, each with its reason:**
  - `profiles\provider-auth-*`: browser sign-in sessions, shared with the
    browser engine's profiles; moving one risks a forced sign-in.
  - `analytics\`: the whole app's signal ledger, and the coach daemon finds
    it on its own through `USERDATA_ROOT`. It moves with M7's `logs\` if the
    operator wants the AI part split out.
  - `cache\imagine`: regenerable.
- A project folder keeps only `<project>\.devpulse\chat\project.json`, a link
  to its `projects\<project-id>\` folder in the library (M7).

## 5. Wiring: the queue

| # | Milestone (one PR each) | What moves | Deleted in the same PR |
|---|---|---|---|
| **M1** | Component folders (built) | 28 folders to `components/<id>/`; `infographics/{stage,tone_cycle}` to `chart/`; `document.rs` split into `render/`; callers in editor, deck, workbench and home name real modules; contracts gate follows `id`; the pattern test | `templates/`, `infographics/`, `document.rs`, every `pub use` in `dpl-slides` |
| **M2** | Lists | `Body`, `layout.body`, the registry's painter map; agenda, features, stats, timeline, code through the seven steps; list items selectable in the editor | those five structs and `ui()`s |
| **M3** | Columns | split, compare | their structs and `ui()`s |
| **M4** | Charts | the eight charts; `chart/` stays shared | their structs and `ui()`s |
| **M5** | Interactive | checklist, reveal, choice, quiz, cost-meter, predict, drag-match; interaction state moves to `state.rs`, live on the deck and at rest in thumbnails and the editor | their structs and `ui()`s; the `SlideTemplate` trait once nothing implements it |
| **M6** | `.usr/.ai` in the code (built) | `dpl-roots`: `Layout.ai` at `.usr/.ai`, `ai_root()`, the one-time carry; provider accounts, provider profiles and the role and room library write under it | `.aidata`, `aidata_root()`, the three AI paths under `userdata_root()` |
| **M7** | Chats per project | chats, logs, context and media under `.usr\.ai\projects\<id>\`; the project folder keeps only the link | the archive body under `<project>\.devpulse\chat` |
| **M8** | The operator's move | "Bring in my existing library" and `cargo xtask move-library` end in this layout (the carry does the split); dry run first, then the operator taps **Move it** | reading the repo-root `content\`, `assets\`, `.userdata\` outside the drill seam |

Order: M1, then M2 to M5, then M6 to M8. M6 does not depend on M2 to M5 and
can go first if the operator wants the data folders before the components.

**Existing data.** Pre-release, so there is no fallback read of the old
place. M6's carry is the pattern: copy, verify, leave the source untouched.

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

**M6** (2026-11-25, `feat/m6-usr-ai`):
- `cargo test -p dpl-roots`: 51 passed, including
  `ai_data_is_carried_once_into_usr_ai_and_the_source_is_untouched` and
  `a_library_without_old_ai_data_just_gets_the_folder`.
- Full `cargo green` passes on Windows.
- Real exe on a scratch library holding a copy of `.userdataat`
  (`target/drills/m6-usr-ai`): after one launch `.usr.aiat` holds
  `accounts.db`, `provider-profiles.db` and their `-wal` files, and the
  source files hash the same as before the launch.

## Decisions for the operator

1. **Next:** M7 (chats, logs, context and media per project under
   `.usr\.ai\projects\`), or the components (M2 to M5)?
2. **Analytics and sign-in sessions** stay in `.userdata` (reasons in 4.3).
   Say if either should move under `.usr\.ai` too.
