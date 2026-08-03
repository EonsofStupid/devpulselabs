# 2026-08-01 — The last named landmine, closed

`docs/journal/2026-08-01-run-manifest-contract.md` closed with a landmine
rather than a fix:

> The Workbench surface still uses the DEFAULT root rather than loading a
> project's `EguiPilotConfig.artifact_root`. One definition now, but a
> project that MOVES its root still needs a config seam into the hub.

The operator's standing instruction is that pre-release is exactly when a
known-wrong thing gets fixed instead of named. So it is fixed.

## What it was

`configured_artifact_root(workspace)` did not exist. The Experiments surface
read the DEFAULT root, so a project that moved its artifact root would have
seen an empty list — and the surface would have reported "no runs", which is
a lie in the same family as the count label.

## Where it landed, and why not the obvious place

First attempt put it in `egui-pilot-ai-protocol` beside `CONFIG_FILE` and
`DEFAULT_ARTIFACT_ROOT`. That was wrong twice over: `toml` is only a
DEV-dependency there, and — the real reason — **protocol carries names and
types, not filesystem access**. Adding IO to the vocabulary crate to save an
import is how a layer stops meaning anything.

It lives in `egui-pilot-ai-lab`, which already reads the filesystem
(`scan_runs` walks the runs root). Protocol keeps `CONFIG_FILE` — a NAME,
which is exactly what a vocabulary crate should own, and which the runner
now imports from there instead of declaring its own.

It reads only the ONE field it needs, so it never mirrors the runner's full
config vocabulary.

## Verified

- Four new pins in `egui-pilot-ai-lab`, the load-bearing one being
  `a_moved_artifact_root_is_honoured` — without it this change is a
  refactor, not a fix.
  - `a_config_without_the_key_falls_back_to_the_default`
  - `a_missing_config_means_nobody_moved_it` — absence is a fallback, never
    an error
  - `an_unreadable_config_falls_back_rather_than_failing` — a malformed
    config must not take a surface down with it
- 55 workbench · 18 protocol · 14 runner — green
- Cold clippy after `cargo clean -p` on all five touched crates — clean
- `workbench-experiments` still passes on the real exe, now reading its root
  through the config

## Landmines

None outstanding for this surface. `CONFIG_FILE` moved out of the runner
with no re-export shim; the CLI's import was repointed.

## Exact next step

The second arm on `chat-history-rail` — the first real A/B, so the dashboard
mock has something true to draw.
