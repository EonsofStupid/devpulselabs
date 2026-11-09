# PC1b native-session qualification — 2026-10-29

Implementation plan: [chat-project-native-execution](../plans/chat-project-native-execution.md).
PC1a delivered in #305. This report does **not** declare PC1b or standalone support complete.
Production native qualification remains false; candidate link validity cannot promote it.

## Exact Windows artifact

Both files came from the approved central depot's `windows-x86_64/codex/0.163.0`
directory, then were copied into the isolated worktree's disposable
`target/pc1b-qualification-20261029/bin`. Original and copied SHA-256 matched.
Windows reported Valid Authenticode signatures from `OpenAI OpCo, LLC` on both.

| File | SHA-256 |
|---|---|
| codex.exe | `2692C8C7BB2CF8B0AEFE638741FC90236F4B824C59578CE84994F9DE58200993` |
| codex-code-mode-host.exe | `07F3869F0849CD54B31CEC1F86EC796F004501BD3A215135C1FACC883A2B9988` |

`--version` returned `codex-cli 0.163.0`. Help and JSON-schema generation passed.
The exact generated list/resume/turn-page response schemas are retained in the
Codex adapter. Compatibility now checks their structural contracts, not only
method names. Generated schemas and raw scratch reports remain under the fixture.

## Disposable native probes

Each process used a fresh explicit CODEX_HOME, cleared environment, scratch
HOME/USERPROFILE/APPDATA/LOCALAPPDATA/temp paths, and provider file-auth mode.
No existing provider profile, auth file or credential was read or copied.
No model prompt or authenticated turn was sent.

| Probe | Result |
|---|---|
| initialize, thread/start, thread/list without auth | Passed. Empty thread has no persisted rollout; resume correctly refused it. |
| Schema-declared thread/inject_items with synthetic user/assistant text | Passed; created a native rollout without a model request. |
| Resume, archive, archived listing, unarchive of that synthetic thread | Passed. |
| Copy only synthetic sessions into a fresh local profile; resume at a new workspace path | Passed, including archive/unarchive. Prior profile and SQLite files were retained. |
| Full turns/items pagination for the injected synthetic history | Both returned empty pages. Injected model-visible items are not proof of a genuine persisted conversational turn. |
| Windows verbatim workspace spelling | Requested `\\?\D:\...`; both response cwd fields used `D:\...`. Read-only sandbox and network=false were acknowledged. |

The profile created `state_5.sqlite`, `logs_2.sqlite`, `goals_1.sqlite`,
`memories_1.sqlite` and `queue_1.sqlite`. Injected history additionally created
`thread_history_1.sqlite` and a `sessions/.../rollout-....jsonl` file. All databases
had associated WAL/SHM files. Only filenames/sizes were inventoried; database
contents were not used to infer a credential/conversation separation.

These results show that synthetic rollout recovery can rebuild local stores.
They do not establish that all real turns, attachments, goals, queued work,
memories or standalone history survive a fresh machine. A sessions-only link is
therefore still a candidate, not a portable-support claim.

## Implemented foundation and review

- Owned profile generations with fixed candidate session/archive links and
  shared lifetime leases; exclusive setup/relocation preserves old generations.
- One explicit profile home for provider version/status/login/runtime commands;
  revalidate marker and links before launch, retain lease through child cleanup.
- Bounded list/resume parsing and paging; atomic native-history delivery; no
  live-token replay for historical messages. Unsupported items get explicit
  read-only diagnostics; provider-native files retain the originals.
- Explicit gated resume/refresh controls, session/draft identity preservation,
  project-generation checks and bounded list merge.
- Review corrections: reject ordinary-start bypass for candidate profiles;
  retire timed-out/failed load receivers before retry; refuse empty native
  history replacing an already saved transcript.

Focused verification before final integrated gate: Codex60, runtime20,
profile7 (real Windows symlinks), feature host18 and window54 tests passed.
All-target clippy passed for the owners tested before the final review changes.
The final local `cargo green` passed: **1,296 tests, zero failed, six existing
ignored**, plus formatting, docs/structure/contracts, clippy, optimized release
build and renderer smoke (shape16,616 pixels, text10,634, export60 frames).
Log: `target/pc1b-green-final.log`. This is not completed real-client qualification.
The confirmed Windows drive-prefix difference now has a narrow comparison
normalization and regression test. It preserves directory case, refuses `..`,
does no filesystem lookup, and does not generalize to unproven UNC variants.

## Remaining gates

1. Operator-completed sign-in in an isolated fixture and real successful turns:
   preserve exact history after A/B/A, restart, fresh profile and moved project.
2. Inventory all conversation-bearing stores using those real operations;
   qualify archive/unarchive, queued work and attachments instead of guessing.
3. Qualify the named standalone entry and shared profile lease with real
   interactive work, concurrent projects, absent/replaced storage and
   parent/process failure drills. Implementation is being added on this draft;
   no standalone capability is enabled by the new executable.
4. Exact approved Linux artifact and matching real qualification. The local
   central depot did not contain the referenced Linux Codex artifact.
5. Native visual proof for the enabled controls, full gate and double-green PR
   delivery. Do not enable or merge a support claim from the synthetic probes.

The implementation is retained as a draft while these gates remain open;
no production qualification switch or standalone-support claim is enabled.

Active and archived native threads now have separate bounded listing phases.
One-use, generation-scoped continuation tokens prevent stale refreshes and
cross-phase provider cursor collisions. Listing never unarchives a thread;
real archived resume remains under qualification. Native item
IDs validate incoming uniqueness, while the project events retain native turn
IDs; inserting earlier items can remint later local event IDs. This remains
visible qualification work before claiming the complete PC1b result.

## Implementation continuation

The named `devpulse-codex-project` console entry accepts only an existing
registry, project identity, immutable generation and a fixed permission mode.
It cannot create/adopt profiles, change the active project, choose an arbitrary
executable or pass through commands. Qualification rejects before filesystem
discovery or provider probes. The same gate applies to the public adapter entry.
An existing interactive terminal is required; GUI terminal/shortcut integration
is still pending. Exact 0.163.0 help supplies `--no-daemon`, preventing reuse of
the provider's shared background daemon. Commands use the explicit candidate
home and bound workspace; read-only denies escalation, while ask/workspace
retain on-request approval. No actual standalone provider turn has been run.

Review caught Linux terminal ownership: a newly isolated process group cannot
read the inherited foreground terminal without handoff. Process-group fixture
cleanup alone is not an interactive-launch qualification. Windows console
Ctrl+C/Break and parent-death behavior also require real terminal evidence.

Exact process-wrap10.0.1 source review found two concrete supervisor blockers:
std JobObject does not request kill-on-job-close, and `wait_on_job` ignores
completion message identity rather than checking all processes exited.
The implementation therefore signals the owned Windows job, boundedly reaps
the direct child and reports tree cleanup as unconfirmed. The caller retains
the profile lease for its process lifetime. Unix interactive spawn refuses.
Tests prove direct-child status, repeated cleanup refusal and closure of the
test descendant's inherited pipe after drop; they cannot promote this backend
to qualified support. A corrected safe supervisor remains required.

Continuation verification (2026-10-30): full `cargo green` passed **1,307
tests, zero failures, six existing ignored**, including clippy, structure,
contracts, optimized release and renderer smoke (16,616 shape pixels, 10,634
text pixels, 60 exported frames). Log:
`target/pc1b-standalone-green-verified.log`. The real release console entry
returned0 for `--help` and1 with the qualification explanation for a bounded
project launch request; its nonexistent registry path remained absent.
Evidence: `target/pc1b-standalone-exe-proof.json`. The isolated main executable
was rebuilt at2026-10-30T04:14:41Z. No new GUI controls were enabled, and this
pass makes no new visual or live-provider support claim.
