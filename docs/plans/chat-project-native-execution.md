# PC1b — native project sessions and standalone Codex execution

Status: operator authorized **execute** on 2026-10-29 after PC1a delivery (#305).
Current cut: implemented foundation passes full local gate (1,296 tests);
retain as draft pending live-turn, complete storage and standalone qualification.
This executes the approved `chat-project-storage.md` PC1b scope. Work only in
the existing isolated worktree, branch `feat/chat-native-project-resume`, based
on `bfc8e33`. Existing provider profiles and credentials are outside test scope.

## 1. Ground truth

PC1a owns project archives and restores native sessions read-only. Codex0.163.0
request/notification schemas are vendored; response envelopes are not. The
adapter cannot list/load sessions, ordinary successful responses are discarded,
and the host discards SessionsPage. Native history cannot be replayed through
the live-token path: that path discards user text and allocates fresh turn IDs.
The profile currently inherits ambient CODEX_HOME. No stable profile/link/lease
owner exists. Grounding was independently checked with the session-os roles.

## 2. Qualification first

Use the already approved central depot Windows0.163.0 artifact (reach56/61),
copying its signed executable and companion into a new scratch fixture. Verify
OpenAI Authenticode signatures and record hashes before execution. Generate
schemas/help with an explicit disposable CODEX_HOME, workspace and environment;
do not inspect/copy auth from any existing profile or perform live sends.
Retain exact response schemas before implementing decoders. Inventory native
conversation paths, archive/unarchive and index reconstruction in this release.
The local Linux artifact is currently absent; Linux runtime qualification must
be supplied through an approved exact artifact before advertising support.

## 3. Storage and lifetime

Implement a separate project-profile module in the existing project owner with
fixed unqualified Codex candidate layout (`sessions`, `archived_sessions`), owned
metadata and exclusively created home. Creating a candidate is an explicit
setup/qualification action, never evidence that this layout is sufficient. No
metadata flag or caller-supplied boolean may promote a candidate to supported.
Keep credentials/config local; link only proven conversation paths. Validate
marker, identity, real ancestors and exact link target. Active clients hold OS
shared locks; setup/relocation needs exclusive access. Immutable new profile
generations preserve old links on relocation instead of deleting or retargeting
them. Missing privileges or unsupported native footprints fail closed.

## 4. Adapter and host

One explicit validated profile must reach version/status/login/app-server and
the named standalone entry. Add bounded correlated list/resume operations from
the exact response contracts: message size, total history, pagination count,
cursor loops and deadlines. Native history loads transactionally and reconciles
existing thread/session identities and saved drafts. Failed loads retain the
read-only archive; successful resume must not start a new thread. Completion
must match the active project generation. Standalone entry has fixed provider
arguments, workspace, permission mode and profile; it holds its lease and owns
child lifetime without requiring the Chat window.

## 5. Gates

No capability claims from discovery alone or fixture-only tests. Real Windows
and Linux drills must establish standalone persistence, A/B/A, restart, fresh
local profile/index reconstruction, moved project, archive/unarchive, wrong
marker, offline/replaced target and concurrent client behavior. If a required
store mixes credentials or cannot be safely redirected, retain unsupported
status and report the concrete evidence. Auth-required sends need a separately
available operator-signed-in disposable fixture; never borrow credentials.

## 6. Verification and delivery

Unit/protocol tests cover malformed/late/paged responses, history identity,
limits, stale leases and failed switching. Real native windows at1440x900 and
1100x700 prove exposed controls. Review, full cargo green, PR with both CI lanes
completed SUCCESS, then isolated release rebuild. Preserve unrelated untracked
Microsoft/ and the provisional PC1a image. Never mutate the primary checkout.

## 7. Rollback and scope

Leave existing archives/profiles/links intact. Failed setup retains partial
owned artifacts with explicit recovery; no copy fallback masquerading as a link.
Only qualified Codex surfaces are enabled; other standalone apps follow their
own qualification. Do not repeat approval questions for already approved scope.

## 8. Evidence

PC1a: `docs/proof/chat-project-storage/README.md`. PC1b findings and exact artifact
hashes will be recorded here before enabling native resume or standalone support.

2026-10-29: Windows depot0.163.0 and code-mode companion have Valid OpenAI
Authenticode signatures; copies were hash-verified in
`target/pc1b-qualification-20261029/bin`. Exact version/help/schema generation
passed under a disposable explicit environment. Native initialize/thread-start/
list passed without sign-in, but resume of the empty thread failed with
`no rollout found`. No authenticated turn was sent. Fresh local profile created
state5, logs2, goals1, memories1 and queue1 SQLite stores; conversation footprint
is therefore still under qualification. The operator has been asked to complete
provider-owned sign-in in the isolated fixture for the required live-turn proof.
