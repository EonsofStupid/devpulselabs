# Local UI-procedure mapping fixtures

These documents are an offline mapping experiment, not runnable UI automations,
an installed adapter, a network endpoint, or a full Open Workflow/Arazzo
conformance claim. The proposed `dpl.*` calls are descriptive contracts. Only
the separate `devpulse.*` media profile projects actual registered native
workflow stages.

Three distinct results must remain visible:

1. **Upstream schema validity:** document shape satisfies a pinned schema.
2. **Profile assessment:** required behavior is recognized and its gaps reported.
3. **Native compilation:** existing `WorkflowDoc` can represent the behavior.

All three UI cases lack a complete native media-workflow mapping. A successful
schema check does not erase those gaps or authorize execution.

| Fixture | Existing source/evidence | Required missing guarantee |
| --- | --- | --- |
| `open-workflow/edit-save.json` | `tests/eguipilotai/own-looks.toml`: fill `root/editor/look-name`, click `root/editor/look-save`, inspect `root/editor/look-chip` | The existing Text check matches a substring. This selected fragment does not prove durable save or human/automation equivalence. Full fixture setup is a precondition. |
| `open-workflow/window-setting.json` | `crates/chat/window/src/lib.rs`: Claude provider selection in independent `chat.primary` | Viewport-scoped targeting, selected-flag observation, and manual completion parity are required. No provider turn, credentials, permission change or persistence is represented. The window is already open. |
| `open-workflow/export-observe.json` | Composer export control and `crates/app/src/coach_signals.rs` bridge | The proposed completion observer needs the JobId from this export. Current Flow cannot obtain that correlation; generic `jobs.done` loses JobId. |

Metadata `sources`, `preconditions`, and `limitations` are descriptive strings.
The `required` list is normative for the local profile; unsupported or unknown
requirements must not be dropped. `mappingOnly: true` is not a security boundary
for a generic standards runner: do not execute these fixtures in one.

`arazzo/ui-procedures.json` compares those cases with API-shaped procedures.
Its relative source points to `arazzo/fixture-api.openapi.json`, a proposed
adapter description with no implemented server. HTTP acknowledgement is input
delivery evidence only. Boolean observation responses are proposed adapter
contracts, not results collected from the app. The export comparison uses
bounded polling rather than event consumption; this difference is explicitly
retained in `losses`. Arazzo extensions alone do not make these obligations
mandatory for other tools.

The standards tests validate the three Open Workflow documents and Arazzo
document using vendored schemas, check local operation references, and export
the actual `dpl_workflows::workflows()` registry as positive shape controls.
They do not implement/evaluate Arazzo expressions or validate the entire
OpenAPI specification. Source documents and semantics remain independently
reviewable. See `../schemas/README.md` for versions, derivation and licensing.
