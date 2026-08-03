# 2026-08-01 — The run-bundle contract gets a type (paying #185's stated debt)

The operator: *"if any of this is not how a real senior AI would do it we are
in pre-release and stop and get it done now."*

Fair. #185 shipped with two things written into its own journal under
"Landmines" — which is a polite way of saying *I knew this was wrong and
shipped it anyway*. Pre-release is exactly when that gets paid.

## What was wrong

1. **The manifest contract was implicit.** The runner wrote an anonymous
   inline `struct Manifest<'a>`; the Test Lab read it back as untyped
   `serde_json::Value` with string key lookups. Two halves of one contract
   with no shared type is how a field gets renamed on one side only, and the
   reader silently returns `None` forever.
2. **The runs root was a literal in three crates** — the runner's config
   default, the Test Lab's refresh button, and the Workbench Experiments
   surface. Three copies of a path that must agree.

## What it is now

`egui-pilot-ai-protocol` — the vocabulary crate every side already depends
on — owns the run-bundle contract:

- **`RunManifest`**, serialized by the runner and deserialized by the lab.
  One definition, both directions.
- **`DEFAULT_ARTIFACT_ROOT`**, named once. The runner's config default, the
  lab's button, and the Workbench surface all read it.
- **`WindowSize` moved here from the runner**, because it is part of the
  record the runner writes and the lab reads. Its constructors now yield a
  plain `String`; mapping that to `RunnerError` or `CliError` is the caller's
  concern, which is where policy belongs. **No re-export shim** — the CLI's
  import was repointed at the protocol, per the house law on moves.

## Verified

- `cargo test -p egui-pilot-ai-protocol` — 18 pass, including four new
  contract pins:
  - `a_manifest_round_trips_through_json` — the actual guarantee.
  - `a_manifest_without_the_experiment_dimension_still_reads` — every bundle
    recorded before the dimension existed still reads. **Evidence is never
    rewritten to suit a newer schema.**
  - `a_window_outside_the_band_is_refused_by_axis`.
  - `the_artifact_root_is_named_once`.
- 55 workbench, 14 runner, 11 runtime, 3 lab — all green.
- Cold clippy (`cargo clean -p` on all six touched crates first) — clean.
- End-to-end on the real exe: a live drill wrote `experiment =
  chat-history-rail`, `variant = rows` through the typed writer, and
  `workbench-experiments` still passes reading it through the typed reader.

## Landmines

- `RunManifest.window` reads `None` on drills that do not pass `--window`;
  that is correct and unchanged, but it means the reproducibility note only
  binds for sized runs.
- The Workbench surface still uses the DEFAULT root rather than loading a
  project's `EguiPilotConfig.artifact_root`. One definition now, but a
  project that MOVES its root still needs a config seam into the hub. Named
  here rather than left implicit.

## Exact next step

The second arm on `chat-history-rail`, so the A/B has real data and the
dashboard mock (`docs/design/mocks/experiments-dashboard-mock.html`) has
something true to draw.
