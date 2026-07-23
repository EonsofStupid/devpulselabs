# D5 — Scripts (MDX-lite, byte-compatible)

Parent: roadmap Program D (#106). After D2 (fe0902f) + D3 (8772e30);
D4 presenter+sync follows (its pilot lane just unblocked via the Codex
seat's viewport-lifecycle slice). Worktree: `C:\localdev\dpl-main` on
main after 4b72a87.

## 1. Context

Roadmap: *"D5 Scripts. MDX-lite format kept byte-compatible (YAML
frontmatter + inert body), parsed in dpl-doc; beats-parity validation;
teleprompter consumes."* The teleprompter is D4's surface; D5 delivers
the FORMAT: parsing, the folder-contract home, parity validation, and
a visible script presence on the episodes wall. Scripts are DATA —
nothing in them ever executes.

## 2. Current state (ground-truthed this session)

- The OLD contract (read this session,
  `devpulse labs/src/domains/content/schema/script.schema.ts` + real
  corpus `content/shows/eos/episodes/*/source/script/*.mdx`): YAML
  frontmatter between `---` lines — `slideId` (kebab), `title`,
  `beats:` list of `{id: kebab, cue: string≥1, note?: string}` — over
  a markdown-ish body rendered as text, never executed. *"Beat ids
  follow the step semantics: `["setup", ...slide step ids]`, so
  beats.length === stepCount (steps + 1). Parity is a doctor
  warning."*
- New tree: `Slide.reveals: Vec<String>`, `step_count() = reveals+1`;
  `SLIDE_RESERVED = ["slide.json","ink.json","notes.md","assets"]`
  (W3b folder contract — a new concern claims a reserved name);
  `validate_episode` walks slide folders; `Issue{path,message}`.
- NO yaml dependency in the workspace; dpl-doc is serde_json only.
- Old scripts lived in `<episode>/source/script/<slide-id>.mdx`; the
  new tree has no `source/` tier — colocation is its law (slide.json ·
  ink.json · notes.md side by side).

## 3. Target design

- **Home: `<slide-folder>/script.mdx`** — colocated like ink/notes
  (the new tree's law; byte-compat binds the FILE FORMAT, not the old
  path). `SLIDE_RESERVED` gains "script.mdx".
- **`crates/doc/src/script.rs`** (new module): `ScriptBeat{id,cue,
  note:Option}`, `Script{slide_id,title,beats,body}`;
  `parse_script(&str)` — a STRICT subset parser for exactly the
  authored grammar (`---` fences; `key: value` scalars, optionally
  double-quoted with `\"`/`\\` unescape; `beats:` as `- id:` blocks
  with `cue:`/`note:` continuation lines; anything outside the subset
  is a NAMED error, never a guess). No yaml dep — the format is ours,
  and the parser is pinned against the real corpus verbatim in tests.
  `load_script(slide_dir) -> Result<Option<Script>, String>` (absent
  file = Ok(None) — scripts are optional);
  `parity_issues(slide, script)` — expected ids `["setup",
  ...reveals]`; count/id mismatches named (the doctor-warning law).
- **`validate_episode`** grows the script arm: for each slide folder
  holding script.mdx — parse errors named; `slideId` must equal the
  folder name; parity issues appended.
- **Episodes wall (workbench)**: each slide row gains a script chip —
  "script · N beats" when present/clean (a named issue line otherwise)
  — the visible proof surface until D4's teleprompter consumes.
- **Seed exemplar**: `seeds/episodes/how-ai-works/title/script.mdx`
  authored against the title slide's real reveals — the template ships
  showing the format.

## 4. Changes, in execution order

1. **create `crates/doc/src/script.rs`** — types + parser + loader +
   parity; tests incl. the REAL cold-open.mdx text verbatim (the
   byte-compat pin), quoting/unescape, optional note, refusal cases,
   parity ok/mismatch.
2. **edit `crates/doc/src/lib.rs`** — `pub mod script;`;
   SLIDE_RESERVED += "script.mdx"; the script arm in
   `validate_episode` (+ a test: a scripted seed-shaped scratch
   episode validates clean; a wrong-beat script is named).
3. **create `seeds/episodes/how-ai-works/title/script.mdx`** — beats
   matching the title slide's reveals exactly.
4. **edit `crates/hubs/workbench/src/lib.rs`** — the episodes wall's
   per-slide script chip.
5. `cargo green`; proof (§6); commit; push backgrounded.

## 5. Boundaries

🚫 No teleprompter/presenter surface (D4). 🚫 No yaml dependency —
the subset parser refuses instead of guessing. 🚫 Old-repo files are
READ ONLY (the corpus pins tests; nothing copies wholesale). ⚠️ The
seed episode must keep validating clean (seed tests).

## 6. Verification

- `cargo green` exits 0 (parser corpus pin + parity + episode tests).
- Real-exe proof: `--route workbench/episodes --mood 0 --screenshot
  docs/proof/d5-episode-scripts.png --shot-frame 60` — the seed
  episode's wall showing the title slide's "script · 2 beats" chip
  (or however many its reveals demand) with zero doctor findings.

## 7. Commit plan

`feat(doc): D5 — MDX-lite scripts (parse, parity doctor, colocated
script.mdx)` — script.rs + lib.rs + seed script + workbench chip +
proof. Push backgrounded.

## 8. Executed — evidence

Execution matched the plan, two additions surfaced by verification:
(1) the body slice initially dropped the first body line when no blank
line followed the fence — fixed to "skip the fence, drop at most ONE
leading blank line", caught by the unit suite; (2) the proof needed a
wall seam, so `open_episode` landed with the picker's badge vocabulary
(`<show>/<id>`, `template/<id>`, bare id) + `--open-episode` — and the
first shot proved the union law the hard way (bare "how-ai-works" hit
the CONTENT clone, which predates the script). Green: all 7 script
tests incl. the verbatim old-corpus pin
(`the_old_corpus_parses_byte_compatible`) and
`episode_scripts_validate_and_parity_breaks_are_named`. Proof:
docs/proof/d5-episode-scripts.png — the seed episode wall with
"2 headline · script · 2 beats" and zero doctor findings. D4
(presenter+sync) now owns "teleprompter consumes".
