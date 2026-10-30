# Project chat — machine awareness, automatic storage and standalone clients

**Status:** implementation authorized by the operator's **resume** in direct
response to review of this concrete design. PC1a delivered in #305 (`bfc8e33`),
with full gate, native proof, double-green CI and isolated release rebuild.
PC1b execution was authorized on 2026-10-29; its implementation is in progress and remains
subject to its real-provider qualification gates. R292 includes supported
standalone apps. Base `e60c140` (ST5a, #303); isolated branch
`feat/chat-project-storage`. This pass does not reorder R290's storage-first
queue. Governing program: `chat-native-program.md` §4.7.

## 1. Problem and evidence

Chat can probe provider status, but a launch directory still determines the
provider workspace. Host histories are read from machine-local folders. There
is no durable selected-project binding, and events/drafts remain in memory.
Codex's live adapter lacks the list/load path despite its protocol supporting
it. A link without that persistence and resume wiring cannot deliver R292.

Grounded owners:

- `crates/app/src/presence.rs`: status probes use generic executable names;
  bounded output exists, but blocking reads/wait have no bounded deadline.
- `crates/app/src/main.rs`: provider working directory is process CWD;
  `local_chat_history_catalog` chooses host histories.
- `crates/chat/runtime/src/lib.rs`: in-memory session events and drafts.
- `crates/chat/providers/codex/src/runtime.rs`: list/load rejected; successful
  ordinary RPC responses are discarded. Its existing request ledger can own
  the missing response correlation. Schema baseline is `codex-cli 0.163.0`.
- `crates/app/src/chat.rs`: Connected starts a new session, SessionsPage is
  discarded, and seam session workspace labels are fixed to DevPULSE.
- `crates/chat/window/src/lib.rs`: session selection currently selects memory.

## 2. Result and boundaries

The default setup offer discovers this machine and connects a chosen working
project to portable chat. Once enabled, conversations and drafts save there
automatically. The active project and durable location stay visible. A → B → A
and restart recover each project's own work. Supported standalone clients use
the same project binding. Installed, signed in, runtime-qualified and
portable-storage-qualified are separate facts.

First complete provider target: Codex's existing in-app adapter plus a
qualified standalone CLI launch entry. Claude Code follows in the existing
provider order. Desktop Codex, Claude Desktop/Cowork, Grok, Gemini/Antigravity
and Cursor are individually qualified; discovery alone never enables a
portable-save badge. This is not a general application launcher or a promise
to redirect every existing shortcut.

No credential migration, arbitrary provider-home rewrites, automatic adoption
of unmarked folders, network synchronization or unrelated chat/AI crate rename.
Existing chats remain where they are until an explicit copy-only import is
designed. Never rename an existing vendor directory out of the way to install
a symlink: that violates the user's copy/never-move law.

## 3. Ownership and proposed layout

Add one egui-free concern, `crates/chat/projects` / `dpl-chat-projects`, with
typed project identity, versioned metadata, bindings, project archive and
validated link operations. Proposed ceiling: serde and serde_json; filesystem
operations use Rust's standard library. Windows link privileges are a visible
qualification result, not a reason to invoke an elevated generic shell.
Register the layer/dependency audit and egui-free census with the new crate.

Proposed portable tree, under the explicitly chosen working project:

```text
<workspace>/.devpulse/chat/
  project.json                    # version, stable id, display name
  state/<revision>.json           # bounded archive delta: events, drafts, selection
  state/<revision>.commit         # synced commit identity/checksum; incomplete tails retained
  providers/codex/sessions/        # provider-native conversation files
  providers/claude/projects/       # future, separately qualified
```

This is separate from the app's media-library layout. Portable metadata uses
project-relative paths, not an old machine's home or drive letter. Event
schema versioning, tail recovery and size limits belong to this owner.
Execution grounding: a committed delta journal saves only changed values,
including streamed text suffixes, instead of rewriting the whole conversation
on each autosave. Incomplete revisions are retained but never treated as saved.
An immutable full checkpoint is written after at most 63 deltas (or earlier
when the 128 MiB replay-byte budget would be exceeded). All history is retained.
Loading validates the newest complete checkpoint and its subsequent chain;
corruption in superseded historical records does not block that current state,
but a complete newer corrupt checkpoint is never silently skipped. Individual
records and reconstructed state are limited to 32 MiB, and directory scans to
200,000 revision entries. Each Project handle remembers the revision it loaded;
clones share that expectation. Under the writer lock, an unexpected committed
revision refuses the save with a reopen/conflict error, preserving caller edits
instead of allowing stale concurrent app instances to overwrite one another.

Machine-local bindings live under an app-owned profile base: proposed
`%LOCALAPPDATA%/DevPULSE/chat` on Windows and
`${XDG_STATE_HOME:-~/.local/state}/devpulse/chat` on Linux. They map stable ids
to current workspace paths and contain provider-owned machine profiles. An
unavailable OS base is an explicit setup error, never a CWD fallback.

`DPL_CHAT_STATE_HOME` is an explicit absolute-path override for private test
and portable deployment bindings. It does not change provider credentials.
The devtools-only `projects` / `projects-compact` authored fixtures use real
project storage only with this override, at 1440×900 and 1100×700; provider
execution and machine transcript discovery remain disabled in those fixtures.

For the Codex qualification candidate, each project gets a distinct new local
`profiles/<project-id>/codex` home. Only its new `sessions` link targets the
project's native-session directory. Config/auth/logs/index remain machine-local;
DevPULSE never reads or copies auth files. Authentication is performed by the
provider in that profile and may require a fresh sign-in. The compatibility
drill must determine whether this is sufficient for that exact version's
resume/index behavior before enabling it. Do not assume current online docs
exactly describe the vendored 0.163.0 schema.

The native footprint must include archive destinations and all other durable
conversation files, not only active `sessions`. Before promoting this layout,
inventory `archived_sessions`, history and SQLite/index behavior in the exact
qualified release. Any additional conversation-only directories get their own
validated project links. Keep reconstructible indexes local only after a fresh
profile successfully resumes project-native files without its old index. If a
required store mixes credentials with conversations, that storage mode remains
unsupported; never copy it wholesale. Although the vendored resume description
mentions rollout-path/history recovery, the actual 0.163.0 request properties
do not admit either field. That recovery route requires separately qualified
schema evidence; it cannot be implemented from descriptive prose alone.

Status probes, login, app-server and the standalone entry must receive the same
explicit project profile. Replace inherited ambient `CODEX_HOME` for these
operations. A sign-in verdict from the person's default profile cannot stand in
for authentication in the new profile. Reuse of provider-managed keychain state
is only reported if the provider itself demonstrates it; no credential links.

Each project profile/link is stable. Changing the visible active project
selects another binding; it does not retarget an existing process's session
directory. This is necessary for independent clients working concurrently.
No link is created over a real directory or an unowned reparse point.
Use a per-profile lease shared by DevPULSE and its standalone launch entry.
Relocation waits for all clients to release the binding; stale leases require
process-identity checks and explicit recovery, never blind deletion by PID.

## 4. Implementation cuts

### PC1a — project storage and honest setup

1. Implement the typed project owner, exclusive setup, marker validation,
   bounded reads, durable writes, recovery and per-project archive. Persist
   accepted events and drafts; surface write failures and retain dirty state.
2. Add the machine capability snapshot using existing provider registries and
   cards. Resolve discovery candidates separately from launch qualification;
   apply a deadline and bounded child cleanup to each status probe. A timed-out
   provider cannot stall all following providers.
   Centralize bounded capture in the existing std-only `dpl-proc` leaf and use
   it for the named presence, Codex version/login-status and Grok version
   probes, including discovery during project preparation. The helper receives
   only caller-authored named-provider Commands: no generic command interface.
   Concurrent capped stdout/stderr readers share a fixed reader-slot ceiling;
   a deadline kills/reaps the exact owned child without joining readers whose
   pipes escaped to descendants. Escaped descendants are not claimed reaped;
   occupied slots prevent unbounded accumulation while their pipes stay open.
3. Add native setup/project controls to the existing chat peer window using
   typed intents. Select the working project explicitly; show disconnected,
   unconfigured and unsupported states honestly. The UI may say configured
   archive, but not standalone saving/resume supported before PC1b proves it.
   Native proof on 2026-10-28 showed an unavailable-provider card hiding an
   already restored project transcript. Saved ReadOnly sessions must render
   their messages and retain editable drafts independently of provider
   activation; keep current provider status visible and sending disabled.
4. Bind new provider sessions to the selected project instead of process CWD.
   Keep open sessions bound to their original project. Save drafts before
   switching; a failed flush blocks the switch without discarding input.
   Native evidence on 2026-10-28 disproved root-only worker servicing: a hidden
   main viewport can skip the root frame despite an explicit repaint request.
   One shared host service must therefore run at both root and independent Chat
   peer frame boundaries. It drains project completions, schedules autosave and
   dispatches verified actions after releasing the shared-state lock. Deferred
   Chat close/save retries must also complete from peer frames alone. Repaint
   requests supplement this servicing; they are not a main-window liveness gate.

### PC1b — Codex save/resume and standalone qualification

5. Qualify the exact Codex executable/schema in disposable project/profile
   fixtures first. Check sessions, indexing, keyring/file auth placement,
   expired auth, retained native history and resume after machine-path changes.
   Include archive/unarchive and a fresh machine-local profile with only the
   portable project files. Exact-CWD filters require explicit rebasing when a
   project moves; never rewrite unrelated sessions to make a picker work.
   No real provider-home mutation or credential inspection.
6. Map list/resume through Codex's existing request ledger, decode successful
   responses and project supported native history into chat. Wire SessionsPage,
   typed load intent, true project labels and reconnect choice. Do not create
   a new thread when the person chose a saved session.
7. Provide a named, project-bound standalone launch entry. It must launch only
   the qualified provider, with the bound CWD and explicit profile; it is not a
   shell-command field. Its operation must not require the chat window to be
   open. Its user action is **Open Codex for this project**, optionally exposed
   as a generated project shortcut. The named-provider launcher remains alive
   for the child lifetime, owns its lease and reaps only that child process tree.
   It uses the same qualified adapter boundary and configured permission mode.
   A generic independently installed client using its old profile is
   explicitly outside this binding until its own integration is qualified.
8. Publish the standalone-supported capability only after real CLI work,
   automatic project persistence, A → B → A and native resume pass on Windows
   and Linux. Keep provider version and supported launch entry visible.

PC1a/PC1b are two reviewable commits/PRs, not two claims that the entire feature
is complete. Later providers extend the same owner and typed capability table.

## 5. Switching, offline behavior and recovery

Create new project/profile artifacts exclusively. Validate marker, link source,
canonical target and account/provider binding before launch. Journal setup;
publish the binding only after validation. Failed setup leaves the previous
selection usable and partial new artifacts explicitly recoverable. Never
repair an unrelated existing link or remove user content as rollback.

The owned activation record changes atomically after saves succeed. Existing
sessions keep their binding; any necessary shutdown waits for that named
provider's supervised process only. An independently started client cannot
have its storage swapped underneath it.

Missing or replaced projects are offline and new activations fail closed.
DevPULSE buffers its own unsaved work and reports the failure. A third-party
client does not inherit `dpl-roots` write guards: unplug behavior, target
replacement and dangling-link handling must be tested per client. Do not
promise that every external process immediately stops disk writes, or mark a
client portable-safe when it silently recreates a fallback session directory.
Cross-machine provider-native resume remains unavailable until the exact
version/path migration drill succeeds; the project archive is still readable.

PC1a recovery includes an explicit **Save a copy** action after a failed save.
It writes the retained chats and drafts into a newly created archive in another
chosen workspace, then activates that copy. The old archive is retained, and
an already configured destination is refused. This is necessary for a stale
writer conflict: simply reopening cannot proceed through the ordinary
save-before-switch path without discarding either instance's edits.

## 6. Verification and delivery

- Logic: unique identities, path traversal/reparse refusal, no-clobber,
  interrupted setup recovery, malformed metadata, truncated event tails,
  pending drafts, write failure, wrong marker, project isolation and bounded
  provider probes. A rejected switch must preserve both projects' content.
- Adapter: list paging, correlated errors, resume replay, unknown native items,
  permissions, disconnect/reconnect, no unwanted new thread, schema admission.
- Real Windows/Linux artifact: setup and project switch at 1440×900 and
  1100×700; independent standalone entry starts a genuine provider session,
  changes persist under the chosen workspace, A → B → A and restart resume;
  concurrent A/B clients, absent drive and changed path. No fixture-only proof
  can enable the standalone capability.
- Fresh-machine proof: start A in the standalone project entry, exit, work in
  B, return to A and resume from both surfaces; then use a fresh local profile
  and changed project path. Include archive/unarchive, lease recovery, wrong
  marker and replacement-drive behavior before publishing support.
- Authentication-required live sends need an operator-owned signed-in fixture;
  absent access is reported as unqualified, not a fake pass. All automated
  filesystem drills use disposable roots and synthetic conversations.
- Full cargo green, self-review, double-green PR merge, then rebuild the
  authorized isolated release. No primary checkout or operator data mutation.

## 7. Decisions and rollback

Confirmed: standalone apps where supported; automatic per-project organization
as the default offer. Do not ask that scope again.

The operator resumed this concrete first design: stable project bindings,
machine-local provider profiles with project-linked conversation storage,
explicit standalone project launch entries, and Codex first in two cuts.
This does not silently redirect the person's existing default AI shortcut.
Actual provider-home setup remains an explicit product setup action. Development
and automated verification use disposable roots, never the operator's profiles.

Rollback disables owned launch entries/bindings while preserving archives,
machine profiles and any existing provider files. Credentials stay owned by
the provider throughout. Restore the previous active binding on transaction
failure; never move/delete old histories to fit the new layout.

## 8. Qualification evidence and sources

PC1b execution detail: [`chat-project-native-execution.md`](chat-project-native-execution.md).
Exact artifact and current real-probe findings:
[`../research/chat-project-native-qualification.md`](../research/chat-project-native-qualification.md).

Delivery integration (2026-10-28): PR #305 is pushed at `0b0468a`. Main advanced
to `855f674` (#304, ST5a′) during the pass. Integrate that change without altering
its location watcher or this cut's peer project service; reconcile the shared
status page to preserve both results, then rerun the full gate and double-green
CI. The only textual merge conflict is the status header. No PC1b scope changes.

Code grounding and gap review use the session-os roles with inherited models;
the named Fable/Opus model roles are unavailable. Current main includes ST5a;
there is no open chat-storage PR. No provider process, host profile, link or
credential was accessed during this read-only qualification.
Gap review tightened the complete native artifact inventory, fresh-profile
resume, consistent authentication/profile selection, binding leases and the
exact named standalone launch entry. These are delivery gates, not waived risks.

Official documentation checked 2026-10-27 (documented interfaces, not proof of
our deployed versions). All support flags remain gated by the real drills:

| Client | Documented basis | Consequence for this plan |
|---|---|---|
| Codex CLI / app-server | `CODEX_HOME` covers config, auth, logs and sessions; the protocol has thread resume. [Environment](https://learn.chatgpt.com/docs/config-file/environment-variables), [authentication](https://learn.chatgpt.com/docs/auth), [app-server](https://learn.chatgpt.com/docs/app-server) | Qualify a new machine-local profile with only conversation storage linked. Do not make the entire home portable. Runtime/native-resume implementation is still missing here. |
| Claude Code CLI | Project transcripts live below its config directory; configurable project names require an alternate config directory and startup environment. [Sessions](https://code.claude.com/docs/en/sessions) | Candidate for a separately qualified profile/entry. This says nothing about every Claude Desktop or Cowork session. |
| Gemini CLI | Automatically saved chats are under a per-project hash below `.gemini/tmp`. [Session management](https://geminicli.com/docs/cli/session-management/) | Project association exists, but safe redirection and the separate Antigravity product still need qualification. |
| Grok Build | Headless sessions have resume flags and live in `.grok/sessions`. [Headless](https://docs.x.ai/build/cli/headless-scripting) | A shared session folder is not sufficient evidence for per-project link switching; qualify modes separately. |

PC1a implementation evidence is recorded in
[`../proof/chat-project-storage/README.md`](../proof/chat-project-storage/README.md):
full local gate, focused tests and native project switching/restart at both sizes.
The initial read-only grounding above describes the pre-implementation baseline.
PC1b provider-native resume, profile links, standalone entries and cross-machine
portability remain unqualified; the UI does not advertise them as available.
