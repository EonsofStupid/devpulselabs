# ST1 verification

Local checkout: `D:/localdev/devpulselabs/.worktrees/portable-storage-st1`.
Branch: `feat/portable-storage-st1`, base `530dd17` (RL #296).
The operator approved the storage plan and authorized this isolated checkout.

## Implemented behavior

- `dpl-roots` owns configuration, library identity and root layout. Capture,
  video, app, index, editor and other consumers use that owner.
- No configuration means a native choice before any library stores or
  upgrades open. An unavailable or invalid configured library gets an
  error with Retry/Quit, never an empty fallback library.
- Imported libraries live in `<program>/library`; linked libraries are
  checked against their `library.json` identity. Exact alternate-volume
  candidates can recover a different drive letter; duplicate identities
  are refused during that search.
- New libraries use `common/{cache,temp}` and
  `accounts/me/{content,assets,.userdata,.aidata}`. Existing unmarked
  populated directories are not adopted automatically.
- New config and identity files are created exclusively. Existing config,
  a wrong marker, unsupported schema, path conflicts or invalid account
  layout are errors, not overwrite opportunities.
- Explicit CLI/env scratch roots still work and never persist configuration.
  Parent-folder and CWD discovery are removed. Composer/ViziWiz/browser
  scratch uses the library's common temp/cache. Composer state writes to
  `.userdata`, with read-only fallback to an existing old state file.

## Configuration example

Setup writes this structure beside the executable (the actual id is minted
once and must match `library.json`):

```toml
v = 1
mode = "linked"
id = "library-example"
label = "My library"
account = "me"
last_path = "E:/DevPULSE/library"
```

Imported mode omits `last_path`. The marker carries `v`, `id`, `label` and
`created` (Unix seconds); it contains no machine-specific path or credentials.

To stage an existing marked library without changing it:

```text
cargo xtask stage-roots <program-dir> <existing-library>
```

## Local verification

- Windows root-engine tests: **20 passed**, including layout/config/marker
  round trips, changed drive, duplicate identity, missing/wrong/corrupt
  markers, newer schema, malformed CLI, no-clobber, unmarked legacy data,
  config-path collision, files occupying ring paths, and shared asset lookup.
- `cargo clippy -p dpl-roots --all-targets --offline --target-dir
  target/roots-check -j 2 -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- Locked offline Cargo metadata: passed; `dpl-roots` has exactly serde,
  serde_json and toml, with no new third-party package added to the lock.
- Structure audit: passed (10 ceilings, 40 egui-free crates, 75 workspace
  members, 50 documented flags). Documentation capability audit: passed.
- Integration tests for roots, composer, video, index, capture, browser,
  ViziWiz, app and xtask: passed (offline, dev debug info disabled).
- Final release executable build: passed. Captured artifact SHA-256:
  `90BBF6B361DD07AC84FF4F7C0ED7DEAF33D6CC061F7547D5F8CB2FC44B7FABFF`.
- Native screenshots: six release captures plus six initial debug captures,
  all opened and inspected.
  First launch left its program folder containing only the executable;
  unavailable startup left only the executable and config, and did not
  recreate the missing library.
- Final `cargo green`: passed, including workspace Clippy with warnings
  denied, **1,187 tests passed / 0 failed / 6 existing live-tool tests ignored**,
  workspace release build and renderer smoke (16,616 shape pixels,
  10,634 text pixels, all 60 export frames).
- Native interaction: passed. Windows accessibility invoked the actual
  imported-library button; config and marker ids matched, all six ring
  folders existed, and the main app opened in the same process. Only that
  test process was closed afterward.

The final gate includes the review fix that avoids inspecting a bare Windows
drive prefix (`D:`), which would consult its current directory. Gate log:
`target/st1-green-final.log`. Environment: `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=8`, `CARGO_NET_OFFLINE=true`.

## Native evidence

Requested logical windows were 1440x900 and 1100x700. At this machine's
1.5 display scale, their PNGs are 2160x1350 and 1650x1050 respectively.
These are new storage baselines, not replacements for earlier hub proofs.

| State | 1440x900 | 1100x700 | Observed |
|---|---|---|---|
| First launch | [Capture](release/first-launch-1440x900.png) | [Capture](release/first-launch-1100x700.png) | Name, both library choices and Quit visible; no clipped startup controls or missing glyphs |
| Marked library | [Capture](release/configured-library-1440x900.png) | [Capture](release/configured-library-1100x700.png) | Real app opens after reading the staged config and marker |
| Missing library | [Capture](release/unavailable-library-1440x900.png) | [Capture](release/unavailable-library-1100x700.png) | Retry/Quit and the failed location visible; no fallback library |

The configured control opens the existing WiredFRONT hub. Its compact
capture has the Sessions card extending behind the footer; this cut changes
its root calls, not its layout, so the control does not certify that hub's
layout. The older `shell-v2-wiredfront.png` and `wiredfront-pixvix.png`
baselines use different versions, sizes and fixtures and cannot establish a
pixel-equivalent comparison. No new slides, templates, pool entries or
registered components were added; the gallery/composition gate has no new
items in this cut.

Release fixtures and logs are retained under
`target/st1-native-20261026-021335-0102`; initial debug evidence remains under
`target/st1-native-20261026-015432-1524` and the PNGs beside this report.
The release wrapper explicitly waits for GUI processes; an initial wrapper
attempt that did not wait failed before capturing and was corrected.
Only synthetic data was staged.
The screenshots cover launch states; they do not claim a physical USB
unplug/replug test or ST2's automatic reconnect behavior.

## Boundaries

ST2 owns periodic reconnect, live unplug detection, reopening databases and
preserving/saving in-memory edits. ST1 does not make existing open IO handles
unplug-safe. ST5 owns Locations; ST3 relocation; ST4 account switching and
moving existing data, including chat/analytics into `.aidata`.

No operator library was moved or changed. The source checkout was read for
the approved plan and instructions only. This report records local evidence;
it does not claim CI, a merge, or a rebuild in the operator's source checkout.
