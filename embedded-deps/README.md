# DevPULSE private embedded dependencies

This directory contains packaging authority, not runtime credentials and not
cross-platform binary payloads.

Copy `sources.example.toml` to the ignored `sources.local.toml`, select the
source executable for the current build target, then run:

```text
cargo xtask stage-embedded-deps <new-release-root>
```

The command copies those private tools under
`<new-release-root>/embedded-deps/`, hashes the copied bytes, writes
`manifest.json`, and reopens the generated bundle through the production
resolver. DevPULSE reads only that staged output. It never reads
`sources.local.toml`.

Windows and Linux releases are staged separately on their respective build
hosts. Authentication remains in each provider client's ordinary user
profile; never place auth files in this directory.
