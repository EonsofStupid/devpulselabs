# 2026-08-01 — Workbench `/experiments`: experiments as domains, one analyzer

Plan: `docs/plans/workbench-experiments.md`.
Adjacent, not duplicated: `docs/plans/coach-lab-experiment-program.md` (PR
#181) owns the Lab PROCESS. This pass built the SURFACE and the capture
dimension that process needs.

## What the operator asked for

> "workbench and then under workbench we have editor and all of that right
> but then /experiments where they become like their own domain but still
> benefit from my lab analyzer"

and, on the same breath about crates:

> "why are you not properly have workbench inside their existing crate i
> mean you are creating 81 fucking crates"

**No crate was added.** `EXPERIMENTS: SubId`, one `SubRoute`, one `else if`,
and `experiments.rs` inside `crates/hubs/workbench`. The workspace is still
at 69.

## Done

**The finding that shaped the pass.** `manifest.json` carried
`protocol_version · session · profile · fixture · executable ·
adapter_policy · window` — **no experiment, no variant**. The scenario id
survived only incidentally, buried inside step ids. So a bundle could not
say what it was evidence FOR, which makes grouping and comparison
impossible rather than merely unbuilt.

"Define the minimal requirements so it automatically starts getting
captured" therefore resolved to exactly one thing: a run must declare its
experiment and variant in its manifest. Two optional fields.

**Declared on the SCENARIO, not looked up from the experiment document at
run time.** A bundle is evidence, and evidence has to be interpretable
standing alone — without the authoring tree beside it. `Flow` gained
`experiment: Option<ExperimentRef>`; the CLI already loads the flow before
launch, so it reaches the manifest with no second write and no post-hoc
amendment.

**One scan, two presentations.** `scan_runs` was extracted from `TestLab`
into a free function, so `/experiments` and `test-lab` read the SAME
summaries from the SAME walk. A second scanner is what would make
experiments stop benefiting from the lab. `RunSummary` gained
`experiment`/`variant`, read from each bundle's own manifest — a malformed
or absent manifest yields `None` rather than an error, because summarising
whatever evidence exists is the analyzer's job.

**`requires` is a contract.** An experiment states its minimal artifacts;
a bundle that misses one is reported (`3 run(s) · 2 complete`), never
silently counted. An unrecognised requirement can never be satisfied and
says so in red — the alternative is a requirement that looks met because
nobody implemented it.

## Verified

- `cargo test -p dpl-hub-workbench` — **55 pass** (49 + 6 new), plus
  protocol/runtime/lab suites green.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean, **after
  `cargo clean -p` on all six touched crates** (see below).
- Live run stamped the dimension for real:
  `experiment = chat-history-rail`, `variant = rows`.
- `workbench-experiments` drill — **4/4 on the real exe**, asserting the
  domain node, its arm count, and the arm's own node. Not a screenshot a
  human has to read.
- Proof `docs/proof/workbench-experiments-native.png` at 1300×800: the
  experiment, its hypothesis, `requires: manifest, trace`, and
  `rows · chat-local-histories · 1 run(s) · all complete` naming the actual
  bundle `run-33324-…` a drill produced minutes earlier — with no
  per-experiment capture wiring anywhere.

## Three things running it caught that writing it did not

1. **The domain node never minted.** It was tracked on a
   `egui::Frame::group(...)` response, which is not a live node — only the
   arm (a real `ui.label`) appeared in the tree. Moved onto the title
   widget. Without this the drill could only have asserted half the
   grouping.
2. **Hub pilot ids carry a `root/` prefix**
   (`root/workbench.experiments/...`), unlike Chat's viewport-scoped ids.
   The first drill failed on the un-prefixed id.
3. **The card was transparent over the backdrop art** and genuinely hard to
   read in the first proof shot. Filled with `surface::RAISED` + hairline.
   Every hub surface has that backdrop behind it; a bare `Frame::group` is
   not enough over it.

## The cold-clippy lesson, paid again

`cargo clean -p` before clippy caught **four** compile errors a warm run
would have hidden: a `Flow` literal in the protocol's own tests and three
more in `egui-pilot-ai-runtime`, plus a `field assignment outside of
initializer` lint in the new tests. Adding a field to a widely-constructed
struct is exactly the change whose fallout lives in test targets that a
warm gate does not rebuild. This is the second time this session; it is now
habit and it is in `docs/plans/workbench-experiments.md` §6.2.

## Landmines

- **The manifest contract is implicit.** The runner writes an anonymous
  inline `struct Manifest<'a>`; the lab reads it back as untyped JSON. It
  works and it is honest about failure, but writer and reader do not share
  a type. A shared protocol manifest type would be the proper fix — a
  deliberate pass, not a drive-by here.
- `RUNS_ROOT` is the literal default. `EguiPilotConfig.artifact_root` is
  the real authority; a surface that reads the config would be correct for
  a project that moved it.
- `experiments/` is a new top-level authored directory, beside `seeds/`.

## Exact next step

The first real A/B: a second arm on `chat-history-rail` so two variants sit
side by side and the difference is visible rather than computed. That is
the shape the operator described for tuning, and everything it needs now
exists.

## Verify commands

```
cargo clean -p dpl-hub-workbench -p egui-pilot-ai-protocol -p egui-pilot-ai-runtime
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p dpl-hub-workbench                       # 55 pass
target\release\eguipilotai.exe test devpulse-windows tests\eguipilotai\workbench-experiments.toml
target\debug\devpulselabs.exe --route workbench/experiments --mood 0 --no-seam --window 1300x800 --screenshot docs\proof\workbench-experiments-native.png --shot-frame 45
```

Discard the first pilot run after any rebuild (warm-up handshake race).
