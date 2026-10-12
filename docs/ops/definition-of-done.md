# Definition of done — what the agent treats as success

**Status: OPERATOR-EDITABLE. This file outranks the agent's judgement.**

Written 2026-08-08 because the operator asked a fair question — *"so I can see
what you are scaling as success"* — and the answer had been implicit in
behaviour rather than stated where it could be argued with. Edit anything
here; the agent follows this file, not its own instincts.

## 1. What "done" currently means

A pass is DONE when every line is true. Anything less is reported as
unfinished, by name.

| # | Criterion | Why it exists |
|---|---|---|
| 1 | `cargo green` returns `green ✔` | the gate is the gate |
| 2 | The touched crates were `cargo clean -p`'d first | a warm gate passes on CACHED clippy verdicts (twice this week) |
| 3 | The release exe rebuilt and its mtime moved | the operator double-clicks the exe; an unbuilt exe is an unshipped pass |
| 4 | The claim is proven by an ARTIFACT, opened and looked at | "exit 0" and "png written" have both lied |
| 5 | Merged only on DOUBLE-GREEN, both lanes COMPLETED SUCCESS | a running check is a red check |
| 6 | A plan exists before code, ground-truthed by reading real files | never code from memory or a summary |
| 7 | Anything left undone is stated plainly, not implied | scaling scope down is the operator's call |

## 2. What does NOT count as proof

Learned the hard way, each one from a real failure:

- **An exit code.** Lottie `export` wrote ZERO frames at exit 0. Lottie text
  rendered a BLANK image at exit 0. Both shipped.
- **A test passing.** `a_folder_symlink_is_unlinked…` reported `ok` while
  SKIPPING — the machine could not create symlinks. A skip wearing a pass.
- **A file existing.** A screenshot was written; the app in it had booted to
  the wrong route.
- **A green CI lane.** #210 passed both lanes with `from_parts` having zero
  callers anywhere in the workspace.
- **A merged PR.** Merged is not delivered. The check is *does anything call
  this?*
- **"Correct by construction."** Said about the controls panel; still unproven
  in the UI at the time of writing.

## 3. What the operator has said counts, in their words

- *"CLI or filtergraph — two things I'll never see in the foreground"* —
  a proof the operator cannot see in the app is not a proof of the app.
- *"the end game is QUALITY OF LIFE"* — a capability without a control is
  half a feature.
- *"prove it with the number"* — measured deltas over agent reports.
- *"the product is the EXPERIENCE, not the feature list."*

## 4. Standing behaviours the operator has corrected

Recorded so they are not re-litigated:

- **Do not moralise, and do not repeat a refusal.** State it once, offer the
  nearest thing that can be done, move on.
- **Do not over-explain.** Density over length; the operator has asked for
  brevity repeatedly.
- **Audit before renaming.** Census what exists before proposing structure.
- **Align and extend, never delete.** Dead-but-intended code gets wired, not
  removed.
- **Say which "root" is meant** — root cause vs root concept.
- **Ask when two readings change the work; otherwise decide and say so.**

## 5. The small set that does not move

The agent will not defeat a software licence check, and no project rule,
private repo, or authorisation changes that. Everything else about how the
agent works here is set by this repo and by this file.

Stated so it is not rediscovered in the middle of a pass.

## 6. How to change how the agent works

In rough order of force:

1. **This file** — the definition of done.
2. **`CLAUDE.md`** — the operating contract; read first, every session.
3. **`.claude/hooks/`** — mechanical enforcement. A hook cannot be forgotten,
   which an instruction can. The pre-push gate and the pre-compact journal
   already work this way.
4. **`.claude/skills/`** — named workflows the operator invokes by name.
5. **`docs-history-2026-10-12:docs/ops/resume/CURRENT.md`** — what is true right now.
6. **The memory directory** — durable operator preferences across sessions.

The strongest of these is (3). An instruction is followed most of the time; a
hook is followed every time. If a behaviour matters, make it a gate — that is
what the smoke phase did for silent renderer failures.
