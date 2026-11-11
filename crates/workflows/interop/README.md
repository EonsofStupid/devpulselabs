# Workflow interop (module `interop` of dpl-workflows) — GF0 local mapping experiment

Pure mapping and diagnostics, with no execution engine or application dependency.
This crate tests whether existing standards can carry DevPULSE's workflow data
and identifies missing semantics in three source-grounded UI procedures.

## Run

From the workspace root:

```powershell
cargo test -p dpl-workflows --offline
cargo run -p dpl-workflows --offline --example workflow_mapping -- demo
cargo run -p dpl-workflows --offline --example workflow_mapping -- export storyboard
cargo run -p dpl-workflows --offline --example workflow_mapping -- assess crates/workflows/interop/fixtures/open-workflow/export-observe.json
```

`demo` succeeds when the mapping experiment checks pass, including expected UI
capability gaps. It does not run a workflow or establish execution conformance.
`assess` prints those gaps and exits 2 because verified UI execution is not
implemented. `import <json-file>` returns a native WorkflowDoc only for the
strict supported media profile. `export <registered-id>` prints that profile.
Output goes to stdout; source files are not changed. Invalid input exits 2.

## Three separate questions

1. Does the document validate against its pinned upstream schema?
2. Does it fit this explicitly bounded local mapping profile?
3. Can the current native types carry all its required semantics?

A yes to the first is not a yes to the others. In particular the UI cases remain
unsupported by the production media workflow engine. EguiPilot has useful
devtools primitives, but durable artifact outcomes, correlated export completion
and a shared human/automation controller are not established by this experiment.

## What is mapped

The registered media workflows export to Open Workflow custom calls derived
from StageKind::label(). Stage arguments are explicit `with` fields; document
metadata carries the native version, label, blurb, surface and optional intake.
The importer reconstructs the existing public types and uses their validator.
There is no duplicated stage census or interpreter. Custom calls would require
an external adapter before any outside runtime could execute them.

Unknown fields, unimplemented behavior, unsupported versions, invalid ordering
and duplicate stage identities refuse. Canonical comparison prevents serde's permissive native
decoding from silently losing fields. Expression-looking arguments refuse because
literal escaping/interpolation has not been mapped. Open Workflow document-name
constraints can be narrower than native IDs; unsupported IDs refuse unchanged.
The raw JSON reader rejects duplicate keys, trailing data, excessive nesting and
documents over 1 MiB. Callers using the Value-based API must preserve that strict
decoding boundary themselves.

UI fixtures are small descriptive slices of existing app behavior, with explicit
preconditions and source references. Their custom calls are proposed adapter
names, not new callable endpoints. The Arazzo fixture describes hypothetical
OpenAPI operations; its polling representation is not an event-preserving
translation of export completion. No API server is created.

See [mapping plan](../../../docs/plans/ui-procedure-mapping-spike.md),
[adoption assessment](../../../docs/research/ui-procedure-standard-adoption.md),
the fixtures' provenance and the schemas' source/license records. This local
prototype retains the repository license; no independently reusable SDK or
official standards conformance claim is made.
