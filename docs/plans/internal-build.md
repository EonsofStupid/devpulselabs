# Internal build — two builds from one zip pattern, with traces and dev tools

**R312 (2026-11-25).** **Status:** draft for the operator's OK. No code yet. Built after
[the backbone](backend-postgres.md).
Brings forward DV1 (the dev panel) from the 2026-11-14 order, and gives ST4's
`logs`, `context` and `media` folders their first writers.

## 1. Goal

The operator unzips DevPULSE Labs and runs it as one of two builds:

- **Internal build**: shows what the app is doing. Every AI call is traced
  (the prompt as sent, each step, the timings, the tokens, the process's CPU
  and memory), a dev panel shows it live, and dev tools file existing
  subscription sessions, images and research under the right project.
- **Customer build**: the same app with none of the dev code compiled in.

Both come from one source tree by one packaging command.

## 2. What exists today (read in the code, 2026-11-25)

| Subject | Today | Where |
|---|---|---|
| Zip | One zip per OS, made by hand-written steps inside the nightly workflow. It holds the app and the Lottie helper. One build only. | `.github/workflows/nightly.yml` |
| Build kinds | One feature, `eguipilotai-devtools`. It is the test driver, not a dev panel. | `crates/app/Cargo.toml` |
| Logs | Text logs to the console, level set by `RUST_LOG`. Nothing is saved to the library. | `crates/app/src/main.rs:78` |
| Signals | The signal ledger writes Parquet under `.userdata/analytics`. It is off unless `--ledger` is passed. | `crates/ledger/src/lib.rs` |
| AI call timings | Measured inside the Codex runtime for its watchdog. Shown nowhere. The prompt as sent is not recorded. | `crates/ai/providers/codex/src/runtime.rs` |
| Per-project folders | `chats` and `providers/codex` are written (M7). `logs`, `context` and `media` have no writer. | `crates/chat/projects/src/lib.rs` |
| Existing sessions | The Codex crate has adoption and native-history code. No screen lists a vendor's existing sessions and files them by project. | `crates/ai/providers/codex/src/{adoption,runtime/native_history}.rs` |
| Processes | The app starts its helpers as subprocesses (ADR-009). No screen or doc lists them in one place. | `docs/design/architecture.md` |
| Database | Turso, an embedded SQLite-compatible engine, in three crates: the library index, AI accounts and AI profiles. It has tables with relationships, for the library only. | `crates/index`, `crates/ai/accounts`, `crates/ai/profiles` |
| Chats | Journal files on disk per project. No database row links a chat to its project, images or research. | `crates/chat/projects` |
| Parquet | Used only by the signal ledger for statistics. Append-only files with no relationships. | `crates/ledger` |
| Vector search | None. `crates/vector` draws shapes; it is not a vector database. Nothing stores embeddings. | `crates/vector/src/lib.rs` |
| Postgres | Not used. A sentence in ADR-014 rejected it; the operator ruled on 2026-11-25 that this was never their decision. | `docs/design/adr-014-three-ring-root.md` |

## 3. Design

### 3.1 Two builds, one packaging command

- The app crate gains one feature, `internal`. It only pulls in the
  devtools family (3.7) and opens its windows; no dev code lives in the app
  crate. The customer build compiles none of it, and the structure audit
  fails if any non-`internal` path reaches a devtools crate.
- `cargo xtask package <internal|customer>` builds the app and its helpers,
  stages the folder, and writes the zip. The nightly workflow calls this
  command, so there is one packaging truth.
- Zip names: `devpulselabs-internal-<os>-x64.zip` and
  `devpulselabs-<os>-x64.zip`.
- The internal build says so in the window title and the status bar.

### 3.2 Traces

One trace is one AI call from the operator pressing send to the last byte.

- Each trace is a list of timed steps: prompt built, process started, sent,
  first token, each tool call, done or failed.
- It records the prompt exactly as sent, the model and options, tokens in
  and out, limits reported by the vendor, and the CPU and memory of the
  vendor's process.
- It is written as one JSON Lines file per chat at
  `<library>/.usr/.ai/projects/<project id>/logs/<chat id>.jsonl`.
- Providers emit steps through the existing provider runtime trait. Chat
  code does not time anything itself.
- A trace never holds a credential. The writer refuses a step whose text
  matches the vendor token patterns already used by the trust code.

### 3.3 Dev panel (internal build only)

The `trace` tool (3.7), opened from the status bar or from the chat it traces.

- **Timeline**: the steps of the current call as bars on one time axis.
- **Prompt**: the text as sent, with the parts labelled (instructions,
  persona, project context, the operator's message).
- **Stats**: time to first token, total time, tokens, limits, CPU, memory.
- **History**: past calls of this chat, read from the trace file.
- Built from ColdLight components. Every control has a tooltip.

### 3.4 Sessions and files organizer (internal build first)

The `organizer` tool (3.7). Plain code, no AI. It runs on request and shows
what it will do before it does anything.

- **Sessions**: reads each signed-in vendor's own session list through the
  vendor's CLI, matches each session to a project by its working folder,
  and lists matched, unmatched and already filed.
- **Filing**: a matched session is linked into that project's
  `providers/<vendor>/` folder. The vendor's own files are only read.
- **Images and research**: a file attached to or produced in a chat is saved
  to that project's `media/` or `context/` folder with a small record of
  which chat made it.

### 3.5 The engine and its processes

- **The engine** is the app. **Its processes** are the helpers it starts
  and stops: the backend, each AI's CLI, the Lottie renderer, the coach
  overlay and ffmpeg.
- The `processes` tool (3.7) lists each one: name, purpose, state,
  CPU, memory, how long it has run.
- `docs/design/architecture.md` gains one table with the same names.

### 3.6 The backend

Postgres, planned in full in [the backbone](backend-postgres.md). It is built
first; this plan's milestones follow BE6.

### 3.7 The devtools family: one pattern, repeated for every tool

**Why.** Today dev surfaces are scattered across `coach-overlay`
("devtools-only"), `chat/testkit`, `desktop-spike`, `xtask` and an app
feature flag. Families that architecture.md §4 says should be grouped
(more than about 3 crates) sit flat: `egui-pilot-ai-*` (7), `coach-*` (7),
`engine-*` (5). Every tool this plan adds (traces, organizer, processes,
code context) would grow that pile. Instead there is one family, and every
tool in it has the same shape.

```text
crates/devtools/
  core/            dpl-devtools-core   the tool contract: the dpl.devtool/1 manifest
                                       type, the registry (census law), the
                                       read-only file and process access every
                                       tool uses, and the deny-list check. No egui.
  ui/              dpl-devtools-ui     the window, tab strip, filter chips, row,
                                       tag chip, path actions (copy, open, show)
                                       and preview: ColdLight only, used by every
                                       tool, owned by none.
  tools/<id>/      dpl-devtool-<id>    one tool. The SAME files in every tool:
    Cargo.toml
    tool.json        dpl.devtool/1: id, label, summary, tabs[], what it reads,
                     what it writes (most write nothing), which build it ships in
    data/            the tool's own data files, only if it has any
    src/lib.rs       doc header (one concern) and registration; nothing else
    src/model.rs     the tool's types (serde); no egui
    src/engine.rs    the work (scan, parse, compute); no egui; unit-tested.
                     It becomes engine/<job>.rs, one job per file, when it
                     outgrows one file.
    src/view/mod.rs  the tab switch
    src/view/<tab>.rs   one file per tab named in tool.json
```

**Laws (gate-enforced, amended into architecture.md and the structure audit
in the same commit):**
- **Orthogonal:**
  - a tool depends on `devtools-core`, `devtools-ui` and the platform crates
    whose data it reads; never on another tool;
  - `engine` and `model` never import egui;
  - only `view/` paints.
- **Census:**
  - `tool.json`'s tabs equal the `src/view/<tab>.rs` files equal the
    registry;
  - a tool folder missing a fixed file, or holding an extra one, fails the
    gate (as components do since R311 M1).
- **Platform knowledge stays in the platform.** A tool reads platform data;
  it does not own it. Code context's file conventions are AI knowledge, so
  they live in the AI catalog (`dpl-ai-core`), where chat can use them too.
- **Headless as well as on screen.** Every engine's output is serde, so
  `cargo xtask devtool <id> --json` runs the same engine without a window,
  for scripts and AI agents. No second implementation.
- **Same tools, every build kind.**
  - The internal build opens them from the status bar.
  - The dev run (`cargo xtask dev`) is an internal build.
  - The customer build has none.

**The tools this plan adds, all on this pattern:**
- `code-context` (3.8, R313);
- `trace` (3.2–3.3, the dev panel);
- `organizer` (3.4);
- `processes` (3.5).

The trace *type and writer* stay a platform crate (`crates/chat/trace`),
because chat writes traces in every build. The *viewer* is the devtool.

**Not in this plan, named so it is not lost:** grouping today's flat families
(`egui-pilot-ai/*`, `coach/*`, `engine/*`) and deciding whether
`coach-overlay` and `desktop-spike` join `devtools/`. Each is a pure move,
its own PR, after the operator's OK, on the R311 rule (move, let errors
surface, fix imports, no shims).

```text
crates/xtask/src/package.rs      (3.1)
crates/xtask/src/devtool.rs      (headless runner)
```

### 3.8 Code context (R313, internal build only)

A dev window that reads a project and the operator's AI folders and shows,
in tabs, every rule, to-do, doc, README and agent instruction file. Each one
is tagged with **which AI it belongs to** and **where it comes from**, with
its path one click from the clipboard. It is plain code with no AI. It only
reads, and it never changes or sends a file.

**What it reads (roots).**
- The platform's current project folder (R308: chat and tools inherit it).
- This repo, in a dev run.
- The operator's own AI folders in the home directory: `~/.claude`,
  `~/.codex`, `~/.gemini`, `~/.grok`, `~/.cursor`, `~/.copilot` and the
  rest that the conventions name.
- On this computer, 2026-11-27, the first six exist.

**Conventions are data, not code.**
- `crates/ai/core/catalog/context-files.json` lists every known file name,
  glob and folder. Each entry has:
  - the AI ids that read it (checked against the AI catalog, so a typo fails
    the gate);
  - its scope: repo, repo-local (uncommitted), nested, user, or managed;
  - its kind: instructions, rules, memory, skill, agent, command, hook,
    settings, MCP or doc;
  - how to parse it (markdown, `.mdc` frontmatter, TOML, JSON);
  - the vendor doc URL, and `verified`.
- Shared files carry every AI that reads them. `AGENTS.md`, for example, is
  read by Codex, Cursor and others.
- A new AI or a renamed file is a data edit. Each entry also carries the
  precedence the vendor documents (for example Codex's `AGENTS.override.md`
  before `AGENTS.md`, Claude Code's fallback to `AGENTS.md` only when no
  `CLAUDE.md` exists) and `status: current | legacy`. Windsurf is now Devin
  Desktop, Kimi CLI is now Kimi Code, and Gemini CLI is being replaced by
  Antigravity CLI. The data comes from
  [AI context files](../research/ai-context-files.md): 13 tools, checked
  against each vendor's docs on 2026-11-27.
- Files that are not specific to any AI (README, `docs/**`, CHANGELOG,
  CONTRIBUTING, LICENSE) are tagged **project**.

**Never opened (the deny list, also data, enforced in code).**
- **Credential files** are listed with a lock tag so the operator knows they
  are there, and their contents are never read. The list is the research's
  verified deny-list, for example `~/.claude/.credentials.json`,
  `~/.codex/auth.json`, `~/.grok/auth.json`, `~/.copilot/providers.json`,
  `~/.kimi-code/credentials/**`, and `.env*` anywhere. On this computer
  `~/.claude/.credentials.json`, `~/.codex/secrets` and
  `~/.codex/.sandbox-secrets` exist.
- **Settings and MCP files** can hold keys inline (`~/.claude.json`,
  `~/.codex/config.toml`, `~/.grok/config.toml`, `.mcp.json`). They are
  listed by path and tags only; their contents are not shown in this
  milestone.
- **Session transcripts** (`~/.claude/projects/**/*.jsonl`,
  `history.jsonl`, `~/.copilot/session-store.db`, …) belong to the history
  catalog, so this tool counts them and does not open them.
- A test proves that no deny-listed file is ever opened. A file whose name is
  unknown and whose path lies under a vendor folder is listed, not parsed.

**Tags on every row.**
- **AI:** Claude, Codex, Cursor, Gemini, Grok, Copilot, …, read from the
  catalog's look so the colours match the chat.
- **Scope:** repo, repo-local, nested (with its folder), user, or managed.
- **Git:** tracked, untracked or ignored, from one `git ls-files` run (a
  subprocess; no new dependency). A folder outside git says "not in git".
- **Convention:** verified, or unverified (from the vendor's docs, not yet
  seen in use).

**Tabs.**
| Tab | Shows |
|---|---|
| Overview | Counts per tab and per AI, and which AIs have files here. **What each AI reads here**: pick an AI and a folder, and see the files it would load, in its documented order. That is how to see that Claude Code ignores `AGENTS.md` here because `CLAUDE.md` exists |
| Instructions | `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, `.github/copilot-instructions.md` and the rest, grouped by AI. Each shows its outline (headings) and its imports (`@path` in CLAUDE.md), resolved and clickable |
| Rules | Rule files (`.cursor/rules/*.mdc`, `.claude/rules/`, `~/.codex/rules`, …) with their frontmatter (`description`, `globs`, `alwaysApply`); each file's own bullets listed under it |
| Skills & agents | Skills (`SKILL.md`: name, description), sub-agents, slash commands and hooks, per AI and scope |
| Settings & MCP | Settings and MCP config files: path and tags only. Values are shown only for keys that are not on the deny list |
| Memory | Memory files (for example `~/.claude/projects/<project>/memory/*.md`) with their frontmatter description |
| Docs | READMEs, `docs/**`, plans and ADRs, with title and first heading |
| To-dos | `TODO`, `FIXME`, `HACK`, `XXX` in code comments, and unchecked `- [ ]` boxes in markdown, each with `path:line` |
| Paths | Every row above in one sortable, filterable list |

**On every row:**
- **Copy path**, **Copy relative path** and **Copy as link**
  (`[name](path#Lline)`);
- **Open**, in the system's default app;
- **Show in folder**;
- a read-only preview.

There is a search box across all tabs, and chips filter by AI, scope, git
state and kind. Every control has a tooltip, it is built from ColdLight
components, and it works at 1100×700.

**When it runs.** It scans on open and on Refresh, off the frame loop, on
one worker, never on a timer. It skips `target/`, `node_modules/` and
`.git/`, and any file over 1 MB is listed but not parsed.

**Where the code lives.**
- `crates/ai/core/catalog/context-files.json`, with its types in
  `dpl-ai-core`: the conventions, beside the AI catalog they cite.
- `crates/devtools/tools/code-context/`, on the 3.7 pattern:
  - **tabs** `overview`, `instructions`, `rules`, `skills`, `settings`,
    `memory`, `docs`, `todos`, `paths`;
  - **engine jobs** `roots`, `walk`, `classify` (conventions → tags),
    `parse` (headings, frontmatter, imports, to-dos) and `reads` (what each
    AI loads, in order).

## 4. Milestones (one PR each)

| | Milestone | Proves it |
|---|---|---|
| **IB1** | The `internal` feature, `cargo xtask package`, the nightly on that command, the build named in the window | Both zips unpack and start on a scratch library; the customer exe holds no dev panel text |
| **IB1c** | The devtools family (3.7: `devtools/core`, `devtools/ui`, `dpl.devtool/1`, its audit, `cargo xtask devtool`) and its first tool, code context (3.8) | On this repo and a scratch home: every convention row found and tagged; no deny-listed file opened (test); copy, open and show-in-folder work; captures at 1100×700 and maximized |
| **IB2** | The trace crate; Codex emits steps; traces saved per chat | A real Codex call writes a trace with the prompt and all timings |
| **IB3** | The dev panel: timeline, prompt, stats, history | Captures at 1440×900 and 1100×700 of a real call |
| **IB4** | The organizer: sessions, then images and research | Existing Codex sessions listed and filed on a scratch library; sources unchanged byte for byte |
| **IB5** | The Processes tab and the architecture table | Every running helper appears and stops showing when it exits |

## 5. Rules kept

- Dev code is absent from the customer build, never hidden by a switch.
- No credential is read or stored. Sign-in state comes from each vendor's
  own status command.
- Vendor files and the operator's library content are never changed.
- Proof runs use a scratch library and run one at a time.
- No aliases, no re-exports, no code kept for an older layout.

## 6. Risks

- **Prompt text in a file.** Traces hold what the operator typed. They stay
  in the library's `.usr/.ai` and are never sent anywhere.
- **Only Codex can chat today.** IB2 and IB3 are proven on Codex. Each later
  AI must emit the same steps to appear in the panel.
- **Vendor session lists differ.** IB4 starts with Codex. An AI whose CLI
  cannot list sessions is shown as "cannot be listed", not guessed.
- **This computer has no project registry yet**, so the real-window proof
  needs the operator to set up one project first.

## 7. Decisions for the operator

1. **Approve this plan.** It is not approved yet. It starts after the
   backbone lands.
2. **Code context (3.8, R313): where it goes in the order.** It needs only
   IB1's internal build, with no backend and no AI call, so it can land right
   after IB1 while the backbone's BE0 waits on the operator.

## 8. Evidence

None yet. Each milestone records its proof in its PR and in `status.md`.
