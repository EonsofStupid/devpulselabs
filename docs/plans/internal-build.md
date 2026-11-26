# Internal build — two builds from one zip pattern, with traces and dev tools

**R312 (2026-11-25).** **Status:** draft for the operator's OK. No code yet.
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
| Postgres | Not used. ADR-014 chose an embedded database plus Parquet and rejected a Postgres server inside a portable app. | `docs/design/adr-014-three-ring-root.md` |

## 3. Design

### 3.1 Two builds, one packaging command

- The app crate gains one feature, `internal`. All dev code sits behind it,
  in `crates/app/src/internal/`. The customer build compiles none of it.
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

A panel in the chat window, opened from the status bar.

- **Timeline**: the steps of the current call as bars on one time axis.
- **Prompt**: the text as sent, with the parts labelled (instructions,
  persona, project context, the operator's message).
- **Stats**: time to first token, total time, tokens, limits, CPU, memory.
- **History**: past calls of this chat, read from the trace file.
- Built from ColdLight components. Every control has a tooltip.

### 3.4 Sessions and files organizer (internal build first)

Plain code, no AI. It runs on request and shows what it will do before it
does anything.

- **Sessions**: reads each signed-in vendor's own session list through the
  vendor's CLI, matches each session to a project by its working folder,
  and lists matched, unmatched and already filed.
- **Filing**: a matched session is linked into that project's
  `providers/<vendor>/` folder. The vendor's own files are only read.
- **Images and research**: a file attached to or produced in a chat is saved
  to that project's `media/` or `context/` folder with a small record of
  which chat made it.

### 3.5 Processes, named in one place

- A Processes tab in the dev panel lists the app and every subprocess it
  started: name, purpose, state, CPU, memory, how long it has run.
- `docs/design/architecture.md` gains one table with the same names.
- The backend is an open decision (§7).

### 3.6 Folder pattern

```text
crates/app/src/internal/
  mod.rs
  panel/        timeline.rs  prompt.rs  stats.rs  history.rs  processes.rs
  organizer/    sessions.rs  files.rs
crates/chat/trace/            one crate: the trace type, writer and reader
crates/xtask/src/package.rs
```

## 4. Milestones (one PR each)

| | Milestone | Proves it |
|---|---|---|
| **IB1** | The `internal` feature, `cargo xtask package`, the nightly on that command, the build named in the window | Both zips unpack and start on a scratch library; the customer exe holds no dev panel text |
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

1. **Backend and Postgres.** ADR-014 rejected a Postgres server inside the
   portable app. Either keep that and name the embedded database as the
   backend, or add Postgres as a helper process, or treat Postgres as the
   separate platform's backend that this app connects to.
2. **Order.** This plan ahead of the components (M2 to M5), or after.
3. **Approve this plan** so IB1 can start.

## 8. Evidence

None yet. Each milestone records its proof in its PR and in `status.md`.
