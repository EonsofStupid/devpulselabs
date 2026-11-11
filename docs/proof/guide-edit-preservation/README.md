# GF1a — guide revision evidence

2026-11-02, Windows. `review/guided-workflow-standard`, baseline `711fc72` plus
uncommitted GF0/GF1a changes. [Plan](../../plans/guide-edit-preservation.md).

## Behavior proved

Apply writes a separate source-guide revision. Source bytes, skipped media,
unknown attachments and raw root/step/session metadata are preserved. Old and
submitted references are validated; traversal, device/ADS names, ambiguous JSON,
bad identities and malformed session events refuse before reserving output.
Fault injection covers partial copying, manifest publication and final commit:
the source survives and retry uses a fresh folder. Pending revisions refuse
through inbox scans, recursive linked-Location scans, the shared guide loader
and optional-session export. The Composer integration test verifies the returned
relative identity, typed loader compatibility and new index identity.

Gallery uses the first retained frame cached at indexing, after sidecar merging.
Legacy records lacking a poster are filled once when the view loads. It does not
parse guide manifests every paint. Existing sidecar bytes remain unchanged.

## Verification

The complete affected library suites passed: Composer 64, engine-steps 14,
guide 10, index 24, WiredFRONT 1: **113 passed, zero failed**. One pre-existing
Composer PDF rasterization test remains ignored because it needs bundled
pdftoppm. The final legacy-poster backfill regression was rerun separately.

The test runner needs isolated roots, as `xtask::run_workspace_tests` establishes:

```powershell
$gf1TestRoot = Join-Path (Get-Location) ('target/gf1-tests-' + [Guid]::NewGuid())
$env:CONTENT_ROOT = Join-Path $gf1TestRoot 'content'
$env:ASSETS_ROOT = Join-Path $gf1TestRoot 'assets'
$env:USERDATA_ROOT = Join-Path $gf1TestRoot '.userdata'
cargo test -p dpl-engine-steps -p dpl-index -p dpl-guide -p dpl-composer -p dpl-hub-wiredfront --lib --offline --target-dir ../public-repository/target
cargo clippy -p dpl-engine-steps -p dpl-index -p dpl-guide -p dpl-composer -p dpl-hub-wiredfront --all-targets --offline --target-dir ../public-repository/target -- -D warnings
cargo fmt -p dpl-engine-steps -p dpl-index -p dpl-guide -p dpl-composer -p dpl-hub-wiredfront --check
```

Clippy with warnings denied and format checks passed for the affected crates.
The structure audit and docs-status gate also passed; the latter reported 21
evidence-backed capability rows.
The first plain Cargo invocation omitted scratch roots, causing existing
Composer tests to fail startup; supplying the documented roots resolved those
failures. No operator library/configuration was used to make tests pass.

The actual Rust executable ran successfully:

```powershell
cargo run -p dpl-engine-steps --example guide_revision --profile test --offline --target-dir ../public-repository/target
```

Its [JSON result](drill.json) records the retained synthetic source/revision
folders and successful original/media/metadata preservation checks. The example
checks actual disk artifacts and JSON; typed guide loading is covered separately
by the Composer test. No recording, model call, ffmpeg job or application UI was
started by this example.

## Scope and limits

- Flat, unexported source-guide folders only. Nested attachments and exported
  guides refuse explicitly; edit the original source and export again. No
  automatic migration, deletion or regeneration of existing exports.
- Failed revisions remain marked pending for diagnosis. They are not user-facing
  published artifacts; no automatic cleanup removes original content.
- Static link/reparse and ancestry checks are not protection against a hostile
  concurrent process swapping paths. Tests cover injected IO/publication failure,
  not power-loss durability of every filesystem.
- Windows directory-symlink refusal ran without an unavailable-privilege
  diagnostic in the nocapture run. Unix link cases require Linux CI; not run here.
- No full application screenshot, recorder Stop/Save qualification, performance
  benchmark or standards conformance claim. Metadata preservation applies to
  Apply; the older export serializer remains a separate limitation.
