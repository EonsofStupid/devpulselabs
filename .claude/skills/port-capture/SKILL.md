---
name: port-capture
description: Senior-dev salvage audit — review the prior repo (C:\localdev\devpulse labs), faithfully capture ONE subsystem's full capability surface, and produce the approved port plan targeting the egui/Rust structure. Nothing dropped silently.
---

# port-capture — the faithful-capture plan builder

> **ASSUMPTION BROKEN (ground-truthed 2026-07-27):** the reference checkout at
> `C:\localdev\devpulse labs` is BUILD OUTPUT ONLY — no source tree, no `.git`.
> The source-capture premise below is dead until a source checkout is restored
> (location = open question, per memory). Salvage that still exists there: old
> exes, `artifacts.db`, `content/shows`. Do not run this skill's source
> inventory against that path; report the gap instead.

**Argument:** the subsystem to capture (e.g. `slides`, `steps-recorder`, `capture-engine`,
`editor`, `chat`, `guides`, `infographics`). One subsystem per invocation — one plan,
one reviewable pass.

## Mission

The prior app (`C:\localdev\devpulse labs`, TypeScript/React/Tauri) is the **salvage
source**. The current app (`C:\localdev\devpulselabs`, pure-Rust egui workspace) is the
**target**. This skill's whole job is to prevent the half-capture failure mode: features
that existed, were paid for in effort, and silently never arrived in the new app
(the "18 tones generated, 3 used" recurrence). You are the senior dev doing the audit —
you read the actual source, you enumerate everything, and every single item gets an
explicit disposition. **The plan IS this skill's deliverable.** No code edits here.

## Ground rules (repo law — see AGENTS.md)

- The old repo is **read-only**. Never build on it, never edit it, never move files out
  of it. Copy, never move. Never delete content.
- ADR-009 (ffmpeg = subprocess, never a crate dep) and ADR-010 (editor spec,
  `devpulse labs/docs/wiredfront/editor-spec.md`) carry as law into every plan.
- Content (`data/`, captures, guides, episodes) is never mutated by any plan.
- Plans follow the plan-code-change discipline: 7 sections, ground-truthed, executable
  by a dev who has never seen either repo.

## Protocol

### 1. Inventory the old repo — read, don't remember

- Locate the subsystem: `Glob`/`Grep` under `C:\localdev\devpulse labs\src\domains\`,
  `src\lib\`, `src-tauri\src\`, `docs\`, `tokens\`, `content\`/`data\`.
- Read every implicated file **in full, this session**. A compaction summary or a memory
  of a file is not the file.
- Produce the **capability inventory**: every template, element, command, token, config
  entry, animation, interaction, and behavior the subsystem ships. Count them. Name them.
  For registries/config (e.g. `register.ts`, `nav-dock.config.ts`, `gallery-demos.ts`),
  enumerate every registration — the registry is the census.
- Note LOC and the file map (path → role) so the plan's reader can verify the sweep.

### 2. Survey the target

- Read the current state of the target crates (`crates/hub`, `coldlight`, `shell`,
  `hubs/*`, plus any crate the port would create) and `docs/foundation-plan.md`'s
  salvage map. What already landed? What does the contract (`Hub`, `HubSpec`, tones
  from the generated pool, `Fluid::of`) require of the ported design?

### 3. Disposition table — the no-silent-drop gate

Every inventory item gets exactly one of:

- **PORT-VERBATIM** — logic/data survives as-is (token JSON, curve tables, pure Rust
  engine code, step schemas). Name the target file.
- **REAUTHOR** — same capability, rebuilt in the egui idiom (React component → epaint
  surface; CSS fx → paint vocabulary; arktype schema → Rust types). Name the target
  design and the egui mechanism.
- **RETIRE** — dropped on purpose, with the ratified reason quoted (e.g. rrweb replay
  retired for video-first — already ratified). **A retire without an operator
  ratification on record becomes an ASK-FIRST item in the plan — never decide it alone.**

The table's row count must equal the inventory's item count. If they differ, the audit
is not done.

### 4. Write the plan

`docs/plans/port-<subsystem>.md` in the NEW repo, all seven sections: Context (quote
the operator's requirement), Current state (both repos, file-by-file, verbatim
excerpts), Target design (crate layout, registry entries, scale story), Changes in
execution order (exact files, full contents for new files), Boundaries (🚫/⚠️/out of
scope), Verification (cargo test names + `--screenshot --route` shots that prove each
capability visibly renders — the gallery-gate rule: if it exists, a surface shows it),
Commit plan.

### 5. Gate, then hand off

Quality gate: could a fresh session execute every step without a question? Does every
"currently says X" trace to a read from this session? Then present the plan for
approval. Execution belongs to `/egui-build <plan-path>`; verification to
`/true-verify`.
