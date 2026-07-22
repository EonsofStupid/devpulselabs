# Protocol and input execution contract

Status: specified, not implemented. This is a breaking correction to the
current protocol and must land before scheduler, trace viewer, recorder, or
multi-viewport expansion.

## Why the current shape must change

The current `ClientEnvelope.sequence -> ServerEnvelope.sequence` design puts
command results, step progress, captures, errors, readiness, and stop notices
in one `ServerEvent` enum. `ProcessSession::receive_for` describes strict
one-message lockstep, while `run` deliberately receives many `Step` messages
for the same sequence. That contradiction makes unsolicited diagnostics,
viewport lifecycle events, cancellation, concurrent status, and reliable
tracing impossible to add without special cases.

WebDriver BiDi supplies the useful mature contract: commands are correlated by
opaque IDs, may be long-running or finish out of order, and return one success
or error response; events are independent messages. EguiPilotAI should use the
same separation without copying browser modules.

The input side has a related issue. High-level actions currently construct
ad-hoc vectors of egui events. A robust driver needs stateful input sources,
ordered ticks, explicit tracing, and guaranteed release on every exit path.
W3C Actions is the reference behavior.

## Wire model

Protocol v2 has three top-level message kinds:

```text
ClientCommand(request_id, session_id, method, params)
ServerResponse(request_id, session_id, success | error)
RuntimeEvent(event_id, session_id, scope, event)
```

All identifiers are typed newtypes. JSON/TOML strings are their serialized
form; Rust APIs do not accept interchangeable bare strings.

| Identifier | Meaning |
|---|---|
| `SessionId` | One authenticated target process connection |
| `RequestId` | One client command; unique within a session |
| `EventId` | Monotonic server event sequence within a session |
| `RunId` | One flow execution, including all attempts inside the target |
| `StepId` | Stable scenario step identity, not merely an array offset |
| `ViewportId` | One native egui viewport lifetime |
| `AttachmentId` | One content-addressed artifact descriptor |

Rules:

1. Every accepted command receives exactly one terminal response: success or
   structured error.
2. Events never satisfy a request. Responses never masquerade as events.
3. Responses may arrive out of request order. The runner routes by
   `RequestId`; it never assumes the next frame is its response.
4. Events are ordered by `EventId`. A detected gap is trace corruption unless
   the event class is explicitly declared coalescible.
5. A long `run` command emits scoped progress events and returns one compact
   `RunResult` after completion. Status and cancellation commands may be
   processed while it is active.
6. Unknown optional event variants can be ignored only when the negotiated
   protocol capability says so. Unknown commands, response variants, or
   required events are compatibility errors.

## Handshake and capabilities

The first command remains an authenticated handshake. Its response includes:

- exact protocol major/minor;
- target build and enabled feature set;
- maximum command, event, and attachment descriptor sizes;
- supported action, assertion, snapshot, motion, renderer, and adapter
  capability IDs;
- supported viewport and accessibility behavior;
- queue capacities and coalescing policy;
- renderer identity when already known, otherwise a later required renderer
  event before capture.

A major mismatch fails. A minor mismatch is accepted only when every required
capability for the selected scenario/project is present. The runner performs a
preflight capability check before the first scenario action.

## Runtime events and pressure

Required, lossless events:

- session/viewport lifecycle;
- run and step start/finish;
- actionability decision and action plan summary;
- assertion terminal result;
- diagnostic raised/cleared;
- busy/motion state changes;
- attachment committed;
- panic, disconnect, renderer failure, and input cleanup.

High-rate frame, repaint, pointer-move, and intermediate assertion-sample
events are opt-in and coalescible. Full semantic frames and image bytes are
never placed inline in the event queue; events refer to revisions or jailed,
size-checked attachments.

Backpressure has one rule per class:

- command responses and required events are lossless; a full queue pauses the
  producer at a safe frame boundary;
- coalescible telemetry keeps the newest value and reports the number dropped;
- attachment bodies stream or persist outside the control queue;
- queue saturation itself produces a diagnostic and trace record.

The default MCP surface does not expose subscriptions. The runner selects a
named trace level (`off`, `failure`, `retain-on-failure`, `on`) from trusted
project configuration and subscribes internally.

## Run and cancellation state machine

```text
created -> preflight -> running -> passed
                         |  |
                         |  +-> failed
                         +----> cancelling -> cancelled
created/preflight/running ------> infrastructure-failed
```

Cancellation is idempotent. A terminal run result is emitted once. Stop first
cancels active work, performs input cleanup, flushes required trace events and
attachments, and then acknowledges shutdown. Process death is still an
infrastructure failure even if the runner can recover partial artifacts.

## Stateful input model

Scenarios retain the compact, human-facing actions (`click`, `fill`,
`drag-to`, `press`, and so on). The runtime compiles each resolved action into
an internal `InputSequence`:

```text
InputSequence
  sources: keyboard, pointer-0, wheel-0
  ticks:
    - simultaneous source actions for one egui frame
    - optional virtual duration
```

The session stores:

- pointer position and pressed buttons per pointer source;
- pressed keys and current modifiers per keyboard source;
- active touches when that capability is added;
- current viewport for every source;
- a LIFO cancellation list for release actions.

Each tick is injected into one `RawInput` pass, rendered, and recorded. A
duration advances `PilotClock`; it does not sleep wall-clock time unless the
selected natural-time project explicitly requests it.

### Required action plans

| High-level action | Compiled input behavior |
|---|---|
| `hover` | move pointer through a deterministic path; final frame has no button pressed |
| `click` | hover, press, frame, release, frame; response is checked after release |
| `double-click` | two complete click sequences using controlled timestamps |
| `press` | parse chord, key-down modifiers then key, key-up in reverse order |
| `type` | focus target, emit text/key events according to the declared input mode |
| `fill` | focus, select-all through platform project mapping, replace text, verify value |
| `clear` | focus, select-all, delete/backspace, verify empty value |
| `drag-to` | hover source, press, minimum-distance move, configured path ticks, final target check, release |
| `scroll` | pointer over declared region, wheel ticks, settle |
| `scroll-into-view` | registered scroll adapter or bounded real-wheel search; never a fake success |
| `set-value` | accessibility `SetValue`/increment/decrement path or a named typed adapter; never relabeled as pointer input |

`press` grammar is parsed at scenario load time. Unknown keys, duplicate
modifiers, or impossible platform mappings are authoring errors, not runtime
timeouts.

### Cleanup invariant

After success, assertion failure, timeout, cancellation, protocol error, stop,
or panic recovery, the runtime must attempt `release_all`:

1. release pressed pointer buttons;
2. release non-modifier keys;
3. release modifiers;
4. cancel active touches;
5. send pointer-gone where appropriate;
6. render one cleanup frame;
7. emit an `input.cleaned` event with the previous and final state.

Failure to clean input is an infrastructure error and the process is not
reused.

## Locator resolution and actionability

The high-level locator is resolved against the newest committed semantic
revision immediately before planning an action. The plan pins the target ID,
viewport, geometry revision, and intended hit point. Any semantic or geometry
change before the first press causes a fresh resolution. A drag may pin source
and destination for its duration; the destination is rechecked before release
and fails with evidence if it no longer receives the hit.

| Check | click | hover | fill/type/clear | drag source | drag target | set value |
|---|:---:|:---:|:---:|:---:|:---:|:---:|
| exactly one current target | yes | yes | yes | yes | yes | yes |
| visible through clip chain | yes | yes | yes | yes | yes | yes |
| stable for two frames | yes | yes | yes | yes | before release | yes |
| enabled | yes | no | yes | yes | yes | yes |
| editable | no | no | yes | no | no | capability-specific |
| receives intended hit | yes | yes | focus hit | yes | yes | accessibility target |
| correct viewport | yes | yes | yes | yes | yes | yes |

`force` may bypass nonessential visibility/stability/hit checks only for the
few actions that explicitly support it. It never bypasses uniqueness, current
revision, viewport, protocol capability, or a disabled control. Trial mode
runs resolution, auto-scroll, and actionability without dispatching the input
sequence.

An actionability timeout returns the last target revision, every failed check,
clip ancestry, occluding node when known, nearby actionable controls, and the
last two geometry samples.

## Scrolling contract

There are two honest paths:

1. A rendered target inside a standard scroll region uses bounded wheel input
   until actionability succeeds or progress stops.
2. A virtualized/off-screen target requires an owner-registered
   `ScrollAdapterId`. The adapter receives a stable semantic key and requests
   the owning egui `ScrollArea` to reveal it on a later pass.

Adapters are typed app operations in traces. Their request/result data is
bounded and serializable. No generic runner code reaches into application
state or claims an off-screen node exists without owner-supplied semantics.

## Conformance gates

- out-of-order responses route to the correct pending request;
- events interleave with every command without breaking responses;
- a long run accepts status and cancel commands;
- queue saturation preserves required events and reports telemetry drops;
- every action yields the documented tick sequence;
- modifiers/buttons release on every injected failure boundary;
- drag paths work at more than one DPI and viewport;
- stale geometry forces re-resolution;
- unsupported scroll/set-value behavior fails before input;
- in-memory and stdio adapters run the same protocol suite;
- corrupt, oversized, stale-session, and duplicate-response messages fail
  closed with bounded diagnostics.

## Deliberate non-goals

- arbitrary application function calls;
- raw egui event injection from MCP;
- user-provided executable paths, environment, or artifact paths;
- CSS/XPath/DOM evaluation;
- pretending OS-global input was delivered when only the app received an egui
  event.
