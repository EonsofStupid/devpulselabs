# GF0 local checker evidence

2026-11-02, Windows. Baseline main `711fc72`, branch
`review/guided-workflow-standard`, with the uncommitted GF0 changes.

This qualifies the mapping/checker artifact only. No application window,
provider, recorder, workflow executor or HTTP endpoint was launched.

## Checks

From the review worktree root, reusing the existing adjacent build cache:

```powershell
cargo test -p dpl-workflow-interop --offline --target-dir ../public-repository/target
cargo clippy -p dpl-workflow-interop --all-targets --offline --target-dir ../public-repository/target -- -D warnings
cargo fmt -p dpl-workflow-interop --check
cargo run -p dpl-workflow-interop --offline --target-dir ../public-repository/target --example mapping -- demo
```

25 tests passed, zero ignored/failed. Clippy passed with warnings denied.
The `demo` output is [demo.json](demo.json): two registered native workflows
preserved, three UI fixtures schema-valid with explicit execution gaps, Arazzo
comparison schema-valid with hypothetical operations and polling limitations.
`mapping-checks-passed` is not a runtime-conformance claim.

The actual built example was also invoked directly:

| Case | Observed exit | Result |
|---|---|---|
| demo | 0 | Mapping checks passed |
| export storyboard | 0 | Schema-valid bounded projection |
| import exported storyboard | 0 | Native identity and document reconstructed |
| assess export-observe | 2 | Unsupported execution with correlated-job gap |
| duplicate object key input | 2 | Refused before field loss |
| malformed JSON | 2 | Refused |
| injected unknown behavior | 2 | Refused |
| export nonexistent workflow | 2 | Refused |

The last six outputs are retained in [cli-cases.json](cli-cases.json). Exported
and adversarial inputs were written only to ignored `target/gf0-cli-check/`.
No source/user document was rewritten by the CLI. The schema source hashes and
licenses are recorded in the [schema provenance](../../../crates/workflows/interop/schemas/README.md).

The checked Windows debug example SHA-256 was
`5eae4439f30820c53b2d3f996c96c07f7d3384b3b193da6f7f15262dbca7ffed`.
Debug binaries are not reproducibility promises or distributable releases.
