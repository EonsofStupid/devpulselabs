# DevPULSE tool-depot staging

This directory contains packaging authority, not runtime credentials and not
cross-platform binary payloads.

Copy `sources.example.toml` to the ignored `sources.local.toml`, select the
source executable for the current build target, then run:

```text
cargo xtask stage-tool-deps <new-release-root>
```

The command copies the target-specific subset under
`<new-release-root>/dptools-deps/`, nests each tool by family, hashes the
copied bytes, writes
`manifest.json`, and reopens the generated bundle through the production
resolver. DevPULSE reads only that staged output. It never reads
`sources.local.toml`.

The developer-owned central depot may live at `C:\localdev\dptools-deps`, but
the shipped application never reaches back into that checkout. Runtime
resolution remains executable-relative and portable.

Windows and Linux releases are staged separately on their respective build
hosts. Authentication remains in each provider client's ordinary user
profile; never place auth files in this directory.
