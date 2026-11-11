# GF0 — three-procedure mapping experiment

2026-11-02. Implements the first engineering step of the
[adoption assessment](ui-procedure-standard-adoption.md), authorized by
**execute**. Plan: [GF0](../plans/ui-procedure-mapping-spike.md).
Local module: `crates/workflows/src/interop` (first built as its own crate, then folded into `dpl-workflows`). No product UI, recorder or runtime change.

## Mapping result

| Procedure | Existing app evidence | Open Workflow representation | Arazzo representation | Current native contract and gap |
|---|---|---|---|---|
| Name and save a reusable Look | `tests/eguipilotai/own-looks.toml`; editor `save_as_new_look`; `dpl_doc::look::load_look` | Named custom fill/click/observe calls, explicit node/window and durable-observation obligation | Proposed adapter operations with UI response criteria; durable persistence remains explicitly unmapped | Flow carries Fill/Click/Text, but Text is substring UI evidence. The naming field does not publish its value. Durable completion requires independent storage observation. |
| Select Claude in the independent Chat window | `tests/eguipilotai/chat-peer-window.toml`; provider node publishes Selected; SelectProvider intent | Explicit chat.primary targets and selected-state observation | Proposed selection operation and response criteria | Flow can scope the click; Coach can check selected state in that same viewport. There is no direct Selected expectation or shared human-follow controller. No credentials or provider turn is involved. |
| Start guide export and observe completion | Composer start_export mints JobId and emits Done/Failed; `root/composer.guide/export` | Proposed start and correlated-terminal observation calls | Proposed start/status operations with bounded polling | Flow cannot capture the export JobId or await its terminal result. Generic jobs.done loses identity. A Clear button can appear for failure as well as success. Event/polling equivalence is not established. |

All three require UI-specific capability/evidence adapters. The existing
production WorkflowDoc has only media stages and cannot represent these UI
steps. The checker reports that mismatch rather than coercing them into Render
or another unrelated stage.

The UI fixtures are mapping slices, not launchable scenarios: required setup is
retained as descriptive, unverified preconditions. In particular the existing
Chat fixture lacks a scratch content root and must not be launched against
operator content as part of this experiment. No UI scenario was launched here.

## Positive control and boundary

The experiment projects actual registered `storyboard` and `explainer` documents
to bounded Open Workflow custom calls and reconstructs the same typed native
documents, including intake, stage order and parameters. It reuses StageKind's
serialization and labels; no alternative stage registry or executor is added.

The projection establishes data interchange for this subset. It does not prove
an independent workflow runtime knows how to execute the custom calls. Likewise,
passing an upstream JSON Schema does not make metadata an enforced postcondition.

## Concrete gaps exposed

1. A durable artifact outcome needs an authoritative read after persistence;
   a visible label is a weaker observation and must remain labeled as such.
2. A user and an automated action need a shared outcome controller. Existing
   Coach signals alone do not provide resumable human/automation equivalence.
3. Async completion must preserve producer/run identity. An event or poll result
   from another job must never advance this procedure.
4. Required semantics cannot live solely in optional metadata ignored by a
   general consumer. The local profile checker refuses missing/unknown
   obligations, but an independent consumer still needs this explicit contract.
5. A native ID can be valid internally but fail Open Workflow's name constraints;
   reject that case, never silently rename identities.
6. Native literal text containing an expression marker can be reinterpreted by
   an outside engine. This subset refuses it until escaping is specified.

These support a small UI-domain profile/adapter experiment. They do not justify
replacing existing workflow control-flow semantics or adding a new event bus.

## Verification

See the [run record](../proof/ui-procedure-mapping/README.md) for actual checks and CLI output. No application
execution, persisted user data change, host recording or performance benchmark
belongs to this mapping experiment. It makes no claim of product readiness or
independent implementation.

2026-11-02: all 25 crate tests passed (2 strict JSON, 10 native round-trip/refusal,
9 UI semantic assessment, 4 upstream schema checks). Formatting and all-target
clippy with warnings denied passed. The actual `mapping.exe` demo, export and
import passed; unsupported export execution, duplicate JSON, malformed JSON,
unknown behavior and an unknown workflow all returned the intended exit 2.
Independent code review found and resolved name-boundary and annotation-loss
issues before the final verification. No general workflow runtime was executed.
