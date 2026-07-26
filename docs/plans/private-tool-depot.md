# Private DevPULSE tool depot

**Status:** IMPLEMENTED AND LOCALLY QUALIFIED
**Date:** 2026-07-25
**Owner:** DevPULSE operator
**Scope:** private, personal DevPULSE releases for Windows and Linux

## Outcome

DevPULSE is a portable application. A shipped build is a relocatable,
target-specific application directory whose private subprocess tools live
under one `dptools-deps/` root:

```text
DevPULSE/
  devpulselabs.exe
  dptools-deps/
    manifest.json
    providers/codex/0.145.0/codex.exe
    providers/grok/0.2.112/grok.exe
    browsers/chromium/<version>/...
    media/ffmpeg/8.1.2/ffmpeg.exe
```

The Linux package uses the same relative layout and extensionless executable
names. Windows and Linux payloads are separate release artifacts; a normal
release never carries binaries for another operating system or architecture.

This is private quality-of-life tooling. Redistribution policy is not a
product gate. The gates are deterministic packaging, integrity, compatibility,
containment, and real local proof.

## Authorities

`dpl-tools-deps` is the only production tool resolver. It:

- anchors resolution at the running DevPULSE executable;
- reads `dptools-deps/manifest.json`;
- accepts only the current compile target;
- rejects absolute paths, parent traversal, duplicate IDs, unsupported schema,
  missing files, non-files, and paths escaping the embedded root;
- verifies the declared byte length and SHA-256 before returning a path;
- returns a typed dependency identity, version, digest, and absolute path;
- never reads `PATH`, a working directory, an install registry, a provider
  home, model output, route context, or application configuration.

Provider adapters receive only paths returned by this resolver. Their existing
version and protocol probes remain a second compatibility gate.

Development and packaging tools may locate or download an operator-owned
source binary. That source discovery is outside the production process and
must end by copying the selected binary into a staged release and generating
the release manifest from the copied bytes.

## Source and generated state

Committed source:

```text
dptools-deps/
  README.md
  sources.example.toml
crates/tools-deps/
```

Private local source selection:

```text
dptools-deps/sources.local.toml
```

`sources.local.toml` is ignored. It contains packaging-machine paths only and
is never read by DevPULSE.

Generated release state:

```text
<release-root>/dptools-deps/manifest.json
<release-root>/dptools-deps/<family>/<id>/<version>/<binary>
```

The generated manifest is ordinary release output and is not edited by hand.
Existing staged files are replaced only inside an explicitly selected release
root. Operator data, content, and credentials are never touched.

Single-file tools are copied as files. Chromium is copied as a complete
versioned directory payload with an explicit entrypoint. Its tree digest
covers every normalized relative path, file length, and file payload;
copying only `chrome.exe` is forbidden because that is not a portable browser.

## Manifest v2

```json
{
  "schema": 2,
  "target": "x86_64-pc-windows-msvc",
  "dependencies": [
    {
      "id": "grok",
      "family": "provider",
      "version": "0.2.112",
      "path": "providers/grok/0.2.112/grok.exe",
      "bytes": 123,
      "sha256": "lowercase hex",
      "tree": null
    }
  ]
}
```

The manifest target must exactly match the compiled DevPULSE target. Digests
are lowercase SHA-256. IDs and versions are single safe path segments.

## Packaging command

`cargo xtask stage-tool-deps <release-root>` reads the private source
selection, copies only the current target's dependencies, verifies that every
source is a file, writes the manifest last, and then reopens the generated
bundle through `dpl-tools-deps`.

The required source entries are Codex, Grok, Chromium, and ffmpeg. Browser
sources are directories and declare a safe relative `entrypoint`; the other
sources are single executable files.

Provider update is whole-bundle replacement. DevPULSE never self-updates one
embedded provider executable while running.

## Authentication

Only executable payloads are portable. Provider credentials remain owned by
the official provider clients in the operator's normal user profile. The
release bundle never contains, reads, copies, inventories, or hashes provider
credential files.

The embedded Codex and Grok executables continue to launch their exact official
browser login commands. Moving the DevPULSE directory does not move or clone
the account.

## Failure behavior

- Missing manifest: every affected private tool is visibly unavailable.
- Wrong target: reject the complete manifest.
- Missing, corrupt, or escaped dependency: block only that dependency and show
  a bounded diagnostic.
- Unsupported provider version: preserve the provider adapter's existing
  version-qualified state.
- No production fallback to an installed binary is permitted.

## Acceptance

- Unit tests cover schema, target, duplicates, traversal, containment, missing
  files, length mismatch, digest mismatch, and successful resolution.
- Codex and Grok unit tests prove their profiles accept an injected absolute
  executable and never search `PATH` or provider install directories.
- The Grok authorization browser accepts only the verified Chromium
  entrypoint returned by `dpl-tools-deps`; installed-browser fallback is not a
  production authorization path.
- App startup obtains provider, browser, and media paths through
  `dpl-tools-deps`.
- A generated Windows bundle resolves a real Codex binary and a real Grok
  binary with `PATH` scrubbed.
- Linux resolver and packaging behavior is covered by target-gated tests and a
  Linux local packaging run before declaring Linux release-qualified.
- The package can be moved to a different directory and still resolve.
- `cargo green` passes before push.

## Implementation evidence

The 2026-07-25 implementation pass established:

- a real Windows `x86_64-pc-windows-msvc` bundle containing Codex `0.145.0`,
  Grok `0.2.112`, and ffmpeg `8.1.2`, with all copied bytes recorded in the
  generated SHA-256 manifest;
- a real Linux `x86_64-unknown-linux-gnu` bundle containing Codex `0.145.0`,
  Grok `0.2.112`, and static ffmpeg `8.1`, with version probes and production
  resolver verification performed under Ubuntu in WSL;
- four resolver tests passing on both Windows and Linux;
- the complete repository `cargo green` gate passing locally, including
  formatting, evidence-backed docs status, warning-free workspace clippy,
  workspace tests and doc tests, and the release build;
- EguiPilotAI's real `chat-peer-window` scenario passing all 24 steps and the
  authored AI/Social scenario passing all 27 steps;
- `chat-grok-signed-out-readiness.toml` proving against the real peer Chat
  window that the bundled signed-out Grok client produces an enabled Login
  control while Send remains disabled;
- a release-directory relocation, including a path containing spaces, followed
  by a successful real-executable screenshot while provider-bearing `PATH`
  entries were removed.

The first full gate also exposed a Windows self-lock when the running release
`xtask.exe` attempted to rebuild itself. The gate now explicitly excludes
`xtask` from its nested workspace release build; `xtask` has already been
compiled by `cargo green`, while every shipped crate and binary remains inside
the final release gate.

The 2026-07-26 Grok activation follow-up removed the duplicate
`embedded-deps` naming and made `dptools-deps` the single committed,
packaging, and release-root authority. It added the typed `browser` family,
staged Chromium `150.0.7871.129` as a complete 309-file tree, verified its
439,360,895 bytes with a deterministic tree digest, and read the Windows PE
file version without launching Chromium.

## Deliberate boundaries

This slice establishes the cross-platform dependency foundation and migrates
the Chat provider launch boundary. It does not claim that a Windows machine
has executed the Linux payload, that provider credentials are portable, or
that future app-owned helpers are already migrated.
