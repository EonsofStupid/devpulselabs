# Provider golden pattern — minimum captured contract

Status: planned contract, reconciled 2026-11-22 (R309). This is a contract
appendix to [chat-native-program](chat-native-program.md) §4.8, not another
implementation queue. The approved order there remains authoritative. No claim
that Claude or another detected client can chat follows from this document.

## 1. Authority

The host captures common facts; named adapters translate their protocol.
The UI consumes those facts. Project identity, provider thread identity and
installation/library paths are distinct. Never infer a project ID from a
directory name or copy credentials into a project. Installed, signed in,
connected and qualified to perform an operation are separate states.

## 2. Current owners, verified against merged main 67fe63f

| Owner | Responsibility |
|---|---|
| `crates/ai/core/src/{provider,registration,activation,capability,adapter,usage}.rs` | Provider vocabulary, capabilities, runtime seam, usage; extends these types. |
| `crates/ai/core/src/catalog.rs` and `catalog/` | Data-driven catalog, not a new provider enumeration. |
| `crates/ai/host/src/{presence,machine,usage}.rs` | Vendor sign-in observations, install/process scan and usage census. |
| `crates/ai/providers/*` | Named vendor adapters; Claude currently has detection/history but no live transport. Grok still has a distinct host path; A7 owns convergence. |
| `crates/ai/{accounts,profiles}` | Non-secret activation evidence and authored configuration. |
| `crates/chat/{runtime,context,projects,window,testkit}` | Chat sessions, drafts, explicit context, existing archive binding, native UI and fixtures. |
| `crates/app/src/chat/{project,trust}.rs` | Existing project binding and machine-local permission ceilings. |
| `crates/fabric` | Shared capability/call/usage projections. |

The stale draft's `PlatformContext::current()` and `Project::for_platform()`
equated the build/library folder with the user's project. They are excluded
from the reconciliation; the exact draft survives at recovery commit 2a7d0a5.
Current project storage and trust remain intact. The stronger R308 requirement
for platform-inherited binding still needs an explicit owner/API trace; this
merge does not claim it solved. ST4 must connect the approved library storage
layout and project association without inventing another registry.

Earlier live Codex evidence proved only transport and supplied metadata on
the stale draft. It did not prove correct project identity or A/B switching.

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
| Grok | ACP runtime and activation/retry logic; host still has special paths. Prior live run lacked cached subscription authentication. | A7 supplies the shared adapter implementation without discarding authentication/retry/terminal behavior; run identical conformance cases and qualify a real authenticated turn. |
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


## 6. Apply within the approved program

Do not implement this appendix as a second P0–P5 roadmap. R310 first lands
the current reconciliation. Thereafter use chat-native-program §4.8.4:
R305 custom AI, ST4 storage/context association, A2b/CW2 Claude and generic ACP,
DV1 diagnostics, and the approved room/Clyffy passes. A7 owns Grok convergence.
Extend AI types in `crates/ai/core`; never recreate `chat/runtime/seam.rs`,
`chat/providers/*` or `app/presence.rs`, which merged main already superseded.

For each provider capture a row for every minimum field: native source,
host source, or explicit unsupported/not-reported state. Required launch
facts must be verified before launch. Optional usage/model details must not
be guessed or made a compulsory setup form.

## 7. Common acceptance battery

- Missing executable, incompatible version, failed handshake and signed-out
  states never promote capabilities.
- Native acknowledgement precedes role-draft placement; original drafts and
  unrelated providers survive failures. Long drafts keep Create/Send reachable.
- A/B/A project switching and a view change during a turn retain the correct
  project/context references. Missing ownership is a gap, never a path fallback.
- Permission changes respect the project's trust ceiling; unsupported options
  are explicitly refused. Installing a desktop client does not qualify a CLI.
- Streaming, cancel, timeout, crash and reconnect have one terminal outcome,
  bounded cleanup and generation-safe attribution. Replayed/cumulative usage
  is not double-counted.
- Storage failure retains drafts/history; diagnostics do not copy secrets.
- Real turn evidence records exact provider/build versions and tested OS.
  Windows proof does not qualify Linux/macOS or another provider.

The exact product meant by Meta Muse and its supported transport remain open.
No new launch boundary or credential-handling path is authorized here.
