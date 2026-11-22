# Provider golden pattern — minimum captured contract

Status: planned, 2026-11-21. Requirement: R302. This pass changes documentation
only. Existing uncommitted application changes remain present for review; this
document does not declare their project association correct.

## 1. Outcome and authority

Every provider follows one lifecycle and supplies the same minimum structured
record to the existing chat runtime. The adapter translates vendor differences;
the host owns conversation state and publishes capability facts to the platform.
The native UI renders that record. Adding a provider must not introduce another
project picker, session store, host event loop or definition of Ready.

The platform owns project identity, selection and workspace/storage association.
Chat receives a versioned snapshot of that association. Provider-native threads,
chat archives, platform projects and library locations are different identities.
They must be explicitly related, never equated because paths or names match.

This extends [chat-native-program](chat-native-program.md). The production
boundary remains [chat-provider-runtime-adr](chat-provider-runtime-adr.md);
the single capability writer remains
[ADR-017](../design/adr-017-provider-capability-registry.md). No new generic
process runner, credential store, IPC boundary or external integration is
authorized by this plan.

## 2. Grounding: what the code actually establishes

| Existing owner | Verified shape / gap | Planned treatment |
|---|---|---|
| `crates/chat/core/src/lib.rs` | Typed provider/session/turn IDs, manifests, activation, capability sets, permissions, account summaries, optional token counts and guided-value provenance already exist. | Extend these types; do not create a parallel provider schema. |
| `crates/chat/runtime/src/seam.rs` | `ProviderRuntime`, commands and events exist. `RuntimeCapabilities` has only three booleans; connection currently implies start/stream/interrupt. Live updates lack a common typed model/configuration/complete-usage record. | Enrich this seam and make qualification explicit. |
| `crates/chat/runtime/src/lib.rs` | Registry owns qualified capabilities, sessions, events and per-session drafts. | Keep this owner and its archive path. |
| `crates/app/src/chat.rs` | Both `grok_runtime` and the neutral `runtimes` map exist; `send_grok_turn` and `send_seam_turn` duplicate host dispatch. | Converge common lifecycle/attribution through the existing seam, preserving Grok-specific protocol behavior inside its adapter. |
| `crates/chat/providers/grok/src/seam.rs` | `GrokRuntimeSeam` already maps the Grok transport. | Complete this integration rather than inventing another adapter framework. |
| `crates/chat/accounts` / `profiles` | Non-secret activation evidence, permission choices and authored local/custom configuration have distinct stores. | Keep the distinction between observed runtime identity and operator configuration. |
| `crates/chat/context` | Route snapshots have IDs and explicit attachment state. | Extend structured context here; do not make a formatted prompt string the authoritative project record. |
| `crates/fabric/src/lib.rs` | Provider capability projections and AI call/usage/outcome signals exist. | Publish from the same host record; keep existing consumers. |
| `crates/chat/testkit` | Existing deterministic fixtures. | Add reusable adapter conformance cases here. |

### Project defect requiring correction first

R292 explicitly says the selected working project determines storage
**independently of executable and media-library location**. The recent
`PlatformContext::current()` in `crates/app/src/chat/project.rs` instead chooses
the build folder or configured library and derives its label from the filename.
`Project::for_platform()` then uses a single userdata/chat/conversations archive.
Those choices prove workspace awareness, not the required project association.

The recent diff changes the app/chat binding and chat archive code. It does not
change `crates/roots`, hub implementations or `crates/estate`. That narrow file
finding does not prove that project-management behavior is unaffected: changing
the consumer's association can still disconnect it from the correct project.

Searches found separate identities in chat archives, Social projects and
EguiPilot test projects. None has been established as the platform's active
working-project authority. A directory name is not a substitute. P0 below must
trace the real selection and persistence path, including earlier R292 work.
If that owner is absent in this checkout, record that implementation gap and
propose its location explicitly; do not mint a chat-owned replacement.

Prior evidence is limited: the live Codex reply proved transport, role delivery
and receipt of the build-folder/view metadata. The 1,403-test gate and screenshots
did not prove platform project switching or project-bound archive correctness.

## 3. Minimum record: every adapter must account for every field

“Required” means the field and its truth state are required, not that a vendor
must expose every value. Host-owned facts are filled by the host; an adapter
must never invent them. Diagnostic views name the source and observation time.

| Scope | Minimum captured fields | Authority / requirement |
|---|---|---|
| Identity | `provider_id`, `profile_id`, runtime instance/generation ID, adapter/build revision | Typed IDs; always known. Distinguish two configured connections to the same vendor and a replaced process for the same connection. Reconcile existing static `ProviderProfileId` and stored `ProviderProfileKey` rather than confusing either with provider identity. |
| Client | client kind, OS, architecture, discovered installation identity, selected executable identity, client version | Host resolver plus bounded named probe. Detected installation and executable actually launched are separate facts. |
| Compatibility | protocol kind/version, schema or mapping revision, qualification result, evidence reference | Adapter handshake and pinned fixtures; version string alone never proves compatibility. |
| Observation | source, observed UTC time, expiry/freshness, check in progress, last typed failure | Required for discovery/auth/capability observations; last-known evidence cannot silently become live readiness. |
| Account | authentication state/method, non-secret account label, plan/tier; opaque account identity when exposed | Provider-owned status/handshake. Account ID and plan may be NotReported. Never capture passwords, cookies, auth-file contents, tokens or device codes in this record. |
| Availability | installed state, authentication state, transport state, qualified capability set, limitation/recovery reason | Separate dimensions, derived centrally; one string such as “Ready” is insufficient storage. |
| Project binding | platform project ID, project revision, display label, authorized workspace reference, archive/storage association reference | Supplied by the platform owner. Explicit Unbound/Unavailable states; never derive project identity from cwd, executable, library, social project or provider thread. |
| Context | snapshot ID/revision, capture time, route, permitted entity/selection references, attachment references | Host snapshots at dispatch. Metadata awareness does not attach file bodies. Existing explicit attachments keep their own scope. |
| Session | DevPULSE session ID, provider-native session ID, provider/profile identity, bound project revision, created/updated times, lifecycle state | Host ID always; provider ID acknowledged before sending. Keep archive ID distinct. State says whether history is read-only, resumable or attached live. |
| Model | requested selection (specific or ProviderDefault), effective model ID, source of effective model | Operator request versus provider report. “Provider default” is a selection policy, not an effective model name. NotReported is allowed and visible. |
| Options | requested/effective permission mode and sandbox/workspace scope; requested/effective reasoning/mode, temperature, top-p, output-token limit, output format when exposed | Each option states supported/unsupported/not reported and whether it may change in-session. No silent dropping, invented defaults or silent permission widening. These are captured metadata, not compulsory setup fields. |
| Role | role ID, role-definition revision/hash, applied instruction snapshot reference | Existing role library and operator selection; explicit NoRole. Preserve exactly what was sent even if the role library later changes. |
| Turn | host call/turn/request IDs, native correlation IDs when reported, session/provider/profile/project/context/role references, event sequence | Mandatory attribution before enqueue. Late events from a retired runtime generation cannot update a newer connection or another project. |
| Timing/outcome | queued, accepted/started, first-output and finished timestamps; duration; pending/succeeded/cancelled/failed outcome; stop reason | Host clock with provider facts distinguished; monotonic clock for duration. Queue acceptance does not mean provider acceptance or success. |
| Conversation | original operator message and ordered response/event references, delivery state, retry linkage | Existing conversation archive; structured context and role instructions remain distinguishable from the operator's words. Do not duplicate full transcripts into telemetry. |
| Usage | fresh input, cache read/write, output, total, context used/size; per-turn or cumulative scope | Existing `TokenUsage`; reported values only. Unknown remains unknown. Context occupancy is a level, not a sum. Record scope so repeated cumulative reports do not double count. |
| Limits/cost | rate-limit state/reset/retry-after and quota when reported; provider-reported charge or separately labelled estimate with price-source revision | Optional values with explicit absence state. Subscription allowance is not API token cost; no fabricated zero charge. Never needed merely to start a conversation. |
| Tools/approvals | stable tool/request ID, tool kind, summary, scope, started/completed outcome; approval request ID, offered choices and chosen decision | Required when corresponding capability is qualified. Requests stay correlated to provider/session/turn/project. Never auto-accept an unknown choice. |
| Failure | category, stage, affected IDs, retryable flag, safe operator message, recovery action, diagnostic reference | Typed failure path, including startup/auth/protocol/timeout/disconnect/cancel/persistence. Unrelated provider failures cannot change the selected conversation. |

### Truth states and storage rules

Use an explicit typed observation state: Known(value), NotReported,
Unsupported(reason), Unknown(reason) or NotApplicable. Staleness accompanies the
last observation; it is not a replacement guessed value. Empty strings and zero
must not encode absence. Reuse `GuidedValue`'s provenance/sensitivity concepts
where they fit; it currently lacks observation time and absence states, so P1
must reconcile these rather than introducing two conflicting value systems.

Required launch facts (approved executable, compatibility, workspace authority
for a file-capable runtime, valid auth and enforced permissions) cannot be
Unknown. A provider-default model with unknown effective name can still chat,
but the UI must say so and model-dependent cost must remain unknown.

Persist non-secret profile/evidence records in their existing stores and
conversation records in the archive associated by the platform. Live handles,
PIDs and in-flight authorization challenges remain transient. Normal devtools
show IDs, state transitions, timing, bounds and safe diagnostics; they do not
silently copy raw protocol frames, prompts, credentials or private file paths
into shareable logs. Conversation content follows its existing private archive
policy. This contract adds no credential-handling path.

## 4. The lifecycle every provider follows

```text
platform project/context snapshot + operator's provider/model/role choice
  -> named resolver and compatibility check
  -> provider-owned authentication observation
  -> adapter handshake -> normalized capability/identity/configuration record
  -> host creates session, freezes its project binding, waits for native ack
  -> host snapshots each turn -> adapter maps request -> ordered typed events
  -> archive + fabric + native UI consume the same attributed result
```

1. Discovery is passive and bounded: one check per connection at a time,
   off the paint thread, cancellation/timeout supported. Installed, signed in,
   transport connected and turn-capable are independent observations.
2. The production resolver follows the existing ADR. The current presence
   helper's per-user/PATH fallback must be reconciled with that boundary before
   it becomes the template. Inventory may report a candidate; it must not bypass
   executable qualification or turn into arbitrary launch permission.
3. Qualified capabilities are the intersection of implemented adapter behavior,
   vendor/session support, successful qualification and current policy. Mere
   connection must not automatically grant interrupt/resume/tools/model choice.
4. A new session consumes the platform's current binding. Changing the platform
   project changes the visible project scope, not an existing session's identity
   or a running process's workspace. A turn already started completes into its
   original project. Returning A → B → A restores A's own chats and drafts.
5. An explicitly unbound chat may exist if the platform supports one. It is
   labelled unbound; it must not be saved as an invented project. A missing
   active-project binding produces an actionable context state, never a second
   chat-project setup form or silent fallback to the media library.
6. Role instructions, original user text, contextual metadata and attachments
   form one structured request. Adapters render that request into their
   supported wire format; prompt concatenation is a transport detail. The
   sent context revision is inspectable after the view changes.
7. Model/account/permission/workspace changes are acknowledged or refused.
   Existing sessions never silently switch provider, account or project.
8. Each dispatched turn reaches exactly one terminal outcome. Cancel, process
   loss, late/duplicate events and retries cannot strand the spinner or credit
   usage twice. History replay is labelled separately from live generation.
9. The host owns cleanup. Peer close, root close, crash and timeout retain the
   existing bounded process-tree cleanup and draft preservation guarantees.

## 5. Provider mapping and admission sheet

These are findings from this checkout and previous local proof, not fresh
verification of current vendor APIs. Before implementing a mapping, record the
exact artifact/version and official protocol source used for that adapter.

| Provider | Present in this checkout | Work to meet the common contract |
|---|---|---|
| Codex | App-server runtime, activation, decoder, native-history/candidate-profile code. One live new-session/role/response test succeeded. | First complete reference adapter: capture requested/effective config, normalize complete usage and correlation, prove project binding, cancellation and reconnect. Existing live success is not blanket conformance. |
| Grok | ACP runtime, activation/retry logic and `GrokRuntimeSeam`; host still has special paths. Prior live run lacked cached subscription authentication. | Move host lifecycle onto its existing seam without discarding authentication/retry/terminal behavior; run identical conformance cases and qualify a real authenticated turn. |
| Claude | Descriptor, named auth observation and history/usage decoder; no live `ProviderRuntime` implementation. | Add the named live adapter only after its versioned protocol mapping and launch gates; detection alone cannot grant StartSession/StreamTurns. |
| Gemini | Descriptor and named observation; no qualified live runtime in this checkout. | Same contract and admission checks; implement its actual documented transport without copying host lifecycle code. |
| Cursor | Descriptor and presence specification; source itself marks the live probe unverified on this machine. | Qualify observation first, then a live adapter only where the supported client exposes the required interface. |
| Meta Muse | No established provider identity or adapter found. | Obtain exact product/client identity before choosing a protocol or claiming installation/support. Do not alias it to another Meta product. |
| Local/custom and Clyffy | Existing profile/selection concepts are separate from these native subscription clients. | Preserve authored configurations; mark unsupported fields explicitly. Clyffy records its logical identity and actual backing provider/model, rather than pretending to be another independently installed subscription client. |

Each admission sheet must contain: provider/profile IDs; OS/client version;
resolver identity; official protocol reference and mapping revision; a row for
every minimum field (native source / host source / unsupported reason); exact
capabilities offered; limits/timeouts/cleanup policy; fixture evidence; real
artifact evidence; known gaps. No provider-specific shortcut to Ready.

## 6. Implementation order and file ownership

| Cut | Changes in existing owners | Exit evidence |
|---|---|---|
| P0 — establish project authority | Read platform selection/persistence and R292 history; trace UI selection → stable ID → workspace → archive. Review the recent changes to `app/src/chat/project.rs`, `app/src/main.rs`, `chat/projects` and all existing archives before altering bindings. Record the actual owner/API and a non-destructive correction map. | Explicit mapping for two distinct working projects sharing one installation/library; identify every consumer. No inferred ID or directory fallback. If owner is missing, report the gap before coding a replacement. |
| P1 — minimum typed contract | Extend `chat/core`, `chat/runtime/seam.rs`, `chat/context` and the existing `fabric` projections. Add shared cases in `chat/testkit`. Reconcile account/profile identity and provenance types; migrate provider-only connection keys where necessary to distinguish profiles and runtime generations. | Compile-checked required fields and truth states; fake adapters can exercise the entire lifecycle, including unsupported capabilities and two profiles of the same provider. No new provider framework or project store. |
| P2 — correct host association and make Codex the reference | Use P0's platform owner to supply immutable project/context bindings. Correct the recent library/build-folder shortcut; preserve older archives and newly created test/archive copies. Replace lossy prompt/usage/config projection using P1. | Real A/B/A switching, restart and in-flight switch isolation; reference Codex session, role, streamed reply, cancellation and preserved drafts; no chat-owned project picker. |
| P3 — bring Grok through the same host path | Wire `GrokRuntimeSeam`; common host maps and lifecycle replace duplicated dispatch only after parity checks. Keep vendor authentication, retries and terminal behavior in the Grok crate. | Same conformance results as Codex, plus existing Grok-specific tests. A genuine auth failure remains visible rather than converted to Ready. |
| P4 — admit other providers individually | Claude, then other named adapters according to their actual availability and protocol evidence. Keep detection/history useful before live admission. Meta Muse waits for exact identity. | Completed per-provider admission sheet and real turn evidence. No manifest-only capability promotion. |
| P5 — operator/dev presentation | `chat/window` shows provider/account/model, inherited project, role and permissions before Send; existing devtools show the detailed capture record and evidence ages. | Native screenshots and interaction checks at default/small sizes, long roles, reconnect and missing-data cases. Same record drives UI, workflows and diagnostics. |

P0 is the immediate next step. No provider expansion or archive migration
should be interleaved with guessing project ownership. P1 may be specified in
detail while that trace is performed; P2 requires its verified result.

## 7. Acceptance matrix

The same reusable battery runs for every live adapter; unsupported operations
must produce a truthful disabled state, never a synthetic success.

| Case | Required proof |
|---|---|
| Missing executable / wrong version / bad handshake | Precise state and recovery; no capability promotion or unauthorized fallback. |
| Installed but signed out / stale evidence | Last checked time and reason shown; no send enabled from an old observation. |
| Authenticated but adapter absent | Detection/history remains usable; live chat stays unavailable. |
| Unknown account plan / model / quota / token split | Explicit NotReported; no guessed plan, model, price or zero usage. |
| New session + role + long draft | Native ack precedes draft placement; original draft survives; Create/Send stay reachable; selected role revision is recorded. |
| Project A/B/A, same library/install | Separate stable binding and archives; no chats/drafts mixed and no extra chat setup. |
| Project/view changes during turn | Original turn retains its immutable project/context; new turns receive the right snapshot. |
| Unsupported option or permission mode | Rejected before send or clearly unavailable; requested/effective values cannot silently disagree. |
| Interleaved provider sessions | Account/model/project/event attribution remains correct; one provider's failure cannot overwrite another's UI. |
| Streaming, cancellation, timeout, crash and reconnect | One terminal outcome; bounded cleanup; stale-generation events discarded; replay does not double-count usage. |
| Repeated/cumulative usage and history replay | Exactly-once accounting with declared units/scope; context occupancy is not summed. |
| Offline/replaced storage and restart | Failed saves retain drafts and existing history; retry cannot silently retarget or overwrite a different project's archive. |
| Diagnostics and credentials | Safe synthetic sentinel tests prove secrets never enter persisted diagnostics; full private transcript is not copied into telemetry. |
| Real Windows artifact | Exact build/provider versions, native screenshots and real turn transcript evidence. Linux/macOS remain unqualified until run there. |

Unit/contract checks prove logic. Native tests prove the real UI. Per-provider
live evidence proves only the tested operation/version/account/workspace.
Follow the repository's full gate and release-executable requirements for an
implementation cut; a planning-only change does not warrant another full build.

## 8. Open facts and review boundary

No user answer is needed to establish this minimum provider contract. Two facts
remain unresolved for implementation: the platform's authoritative active
working-project owner/API (first investigate this repo/history), and the exact
product meant by Meta Muse. Ask only for the specific missing fact after that
investigation; do not make the operator reconstruct the architecture for us.

This plan preserves the existing native Rust architecture and existing work.
It does not claim the platform's project management is repaired. Completion is
the P0 ownership map followed by the applicable conformance evidence, not a
popup screenshot or an AI repeating the repository directory name.
