# Workbench `/experiments` — experiments as domains, one analyzer

Branch: `feat/workbench-experiments` off `main`.
Adjacent, not competing: `docs/plans/coach-lab-experiment-program.md` (PR
#181) owns the Coach Lab PROCESS — honesty prompt, essential checks,
Standard/Advanced import, experiment lifecycle, analytics baselines
(named DuckDB there; **ADR-014 amended 2026-08-03 to Arrow/DataFusion** —
that plan's owner rules on its own wording). **This
plan builds the SURFACE and the capture dimension that process needs.**
Adopt, never duplicate.

## 1. Context

The operator, 2026-07-31, correcting a scope I had drifted:

> "workbench and then under workbench we have editor and all of that right
> but then /experiments where they become like their own domain but still
> benefit from my lab analyzer"

and, on the crate sprawl:

> "why are you not properly have workbench inside their existing crate i
> mean you are creating 81 fucking crates why is there 81 crates"

and earlier, on what the surface is for:

> "workbench needs to be a proper shell. where we load our experiments and
> define the minimal requirements so it automatically starts getting
> captured by our analytics, and visualizors that allow me to create
> infographics, and dashboards that allow me to understand potential
> improvements and other real life visual tools the really really smart
> people build out when doing bleeding edge development"

**Hard constraint, stated by the operator and obeyed by this plan: NO NEW
CRATE.** Everything lands in `crates/hubs/workbench`, which already exists.
The workspace is at 69 crates with a 43-line `chat/providers/gemini` among
them; this pass does not add to that.

## 2. Current state

Ground-truthed in-session against `951fb2a`.

### 2.1 Workbench already has six sub-routes

`crates/hubs/workbench/src/lib.rs:30-35` and the `SPEC.subroutes` table at
`:42`: `library`, `author`, `editor`, `episodes`, `studio`, `test-lab`.
Dispatch is a plain if-chain in `fn ui` (`:391`), falling through to
`dpl_hub::placeholder::surface`. **Adding a sub-route is one `SubId`, one
`SubRoute` entry, one `else if` — this is the cheap, intended extension
point.**

### 2.2 The analyzer exists, and is manual and flat

`egui_pilot_ai_lab::TestLab` is mounted at `test-lab` (`:403`). It is
labelled `PASSIVE PREVIEW · LIVE CONTROL UNSUPPORTED` and has five tabs:
Agent tree, Full tree, Runs, Trace, Visual.

`TestLab::scan_runs(root)` (`egui-pilot-ai-lab/src/lib.rs:58`) walks a
directory of run bundles and yields:

```rust
pub struct RunSummary {
    pub id: String, pub path: PathBuf,
    pub has_manifest: bool, pub has_trace: bool,
    pub capture_count: usize, pub stderr_bytes: u64,
}
```

**Correction to an earlier claim in this session:** `scan_runs` IS called —
from inside `TestLab::ui` at `:233`, behind a manual button reading
"Refresh target/eguipilotai/runs". It is not unwired. What it is: manual,
hard-coded to one path, and producing ONE FLAT LIST with no grouping.

### 2.3 The bundle contract already matches what EguiPilot writes

A real run from this session,
`target/eguipilotai/runs/run-23148-18c74c3f2563e940/`:

```
manifest.json   trace.jsonl   stderr.log   renderer.json   renderer-identity.json
```

`manifest.json` → `has_manifest`, `trace.jsonl` → `has_trace`,
`stderr.log` → `stderr_bytes`. Field for field. The canonical root is
`EguiPilotConfig.artifact_root`, defaulting to `target/eguipilotai/runs`
(`egui-pilot-ai-runner/src/lib.rs:42`) — a CONFIG value, not a constant, so
this pass must read it rather than re-hard-code the literal.

### 2.4 The gap that makes automatic capture impossible today

`manifest.json` carries exactly:

```
protocol_version · session · profile · fixture · executable
adapter_policy · window
```

**There is no experiment id and no variant.** The scenario id survives only
incidentally, embedded in step ids inside the results
(`chat-local-histories:f3a95d76788646c9:1`). So today a bundle cannot say
what it was an experiment IN, or which arm of it — which means grouping,
comparison, and every dashboard above them are impossible, not merely
unbuilt.

**This is the load-bearing finding of the pass.** "Define the minimal
requirements so it automatically starts getting captured" resolves to one
concrete thing: *a run must declare its experiment and variant in its
manifest.* Everything downstream falls out of that and nothing works
without it.

### 2.5 Workflow surface nesting is declared and unenforced

`dpl_workflows::Surface { Wiredfront, Workbench }` exists and its own
doc-comment says *"the hubs mount by this, the engine never branches on
it."* The ONLY consumer in the workspace is `dpl-composer` printing a label
string (`composer/src/lib.rs:2730`). No hub mounts by surface.

Real, and **out of scope here** (§5) — it is the workflow list's problem,
not the experiment surface's. Recorded so it is not lost.

## 3. Target design

### 3.1 `/experiments` is a sibling sub-route, not a new crate

```
workbench/
  library · author · editor · episodes · studio   (unchanged)
  test-lab      ← the ANALYZER, unchanged in this pass
  experiments   ← NEW: the domains
```

One `SubId`, one `SubRoute`, one `else if`. The separation is deliberate
and is the operator's: **`/experiments` is where an experiment lives as its
own domain; `test-lab` is the analyzer they all benefit from.** Keeping
them as siblings is what lets experiments be independent without each one
carrying its own analysis code.

### 3.2 An experiment is a document, colocated like a seed

Authored JSON under `experiments/<id>/experiment.json`, discovered by
directory walk exactly as seeds are:

```jsonc
{
  "v": 1,
  "id": "chat-history-rail",
  "label": "Does the history rail actually surface chats?",
  "hypothesis": "Rows beat a count — the operator finds a past chat without searching.",
  // THE MINIMAL REQUIREMENTS. A bundle missing any of these does not
  // count as a run of this experiment — stated, so capture is a contract
  // and not a hope.
  "requires": ["manifest", "trace"],
  // The arms. One entry = a plain experiment; two or more = an A/B.
  "variants": [
    { "id": "rows",  "scenario": "chat-local-histories" },
    { "id": "count", "scenario": "chat-local-histories-legacy" }
  ]
}
```

Authored, not generated: an experiment is a human intention. It is data so
the surface can list it without code changes, and so a later pass can mint
one from Chat.

### 3.3 The manifest gains two fields — the whole of "automatic"

The runner writes `experiment` and `variant` into `manifest.json` when the
scenario is named by an experiment. Both `Option<String>`; a run that is
not part of an experiment stays exactly as it is today.

That single addition is what makes capture automatic: an experiment does
not call a capture API and cannot forget to. **It emits the bundle it
already emits, now self-describing, and the analyzer groups it.**

### 3.4 The analyzer gains grouping, not a second analyzer

`RunSummary` gains `experiment: Option<String>` and `variant:
Option<String>`, read from the manifest during the existing `scan_runs`
walk. `TestLab` keeps its five tabs and its flat Runs list.

`/experiments` then presents the SAME `RunSummary` data grouped by domain:
per experiment, per variant, newest first, with each variant's run count
and whether its `requires` are satisfied.

One scan, one summary type, two presentations. A second analyzer is what
would make experiments stop benefiting from the lab.

### 3.5 The first dashboard, and what it is honestly for

Per experiment: variants side by side, each with run count, requirement
satisfaction, and capture/stderr volume. Where two variants share a
scenario shape, they sit next to each other so a difference is visible
rather than computed.

**Deliberately NOT in this pass:** trend lines, regression detection,
significance, analytics rollups. `trace.jsonl` is the typed signal stream
those need, and it is the same substrate PR #181 wants for its baselines —
so the depth analytics belong with that program, over a capture dimension
this pass makes exist. Building a chart before the dimension exists is how
the count label happened.

### 3.6 Scale

Bundles accumulate. `scan_runs` is already a bounded single-directory walk
and stays one scan; grouping is in-memory over the summaries it already
produces. No per-experiment scan, no N+1 walk.

## 4. Changes, in execution order

1. `crates/egui-pilot-ai-runner/src/lib.rs` — write `experiment` and
   `variant` into the manifest when the scenario is claimed by an
   experiment.
2. `crates/egui-pilot-ai-lab/src/lib.rs` — `RunSummary` gains the two
   optional fields, read in the existing `scan_runs` walk.
3. `crates/hubs/workbench/src/lib.rs` — `EXPERIMENTS: SubId`, its
   `SubRoute` entry, and the `else if` arm.
4. `crates/hubs/workbench/src/experiments.rs` — **new module in the
   existing crate**: discovery of `experiment.json`, the domain list, and
   the grouped run view.
5. `experiments/chat-history-rail/experiment.json` — the first real
   experiment, over the scenario H1 already proved.
6. Unit pins (§6.3).
7. `tests/eguipilotai/workbench-experiments.toml` — the route renders the
   experiment and its variant grouping.

## 5. Boundaries

🚫 **Never touch**

- **No new crate.** The operator's explicit instruction. `/experiments` is
  a module in `crates/hubs/workbench`; the analyzer stays where it is.
- `test-lab`'s existing five tabs and its flat Runs list — additive only.
  It is a working debugging surface and this pass is not a redesign of it.
- `/content`, and the run bundles themselves: bundles are READ. Nothing
  here rewrites, prunes, or moves evidence.
- PR #181's Lab process documents. This plan cites them and adds the
  surface; it does not restate or fork them.

⚠️ **Ask first**

- Any change to `trace.jsonl`'s shape — #181's baselines depend on it.
- Anything that makes an experiment executable from the surface (running,
  not reading). This pass is read-only over evidence, per the D1/H1
  discipline that a model is proved by use before verbs land on it.

**Out of scope**

- **Hubs mounting workflows by `Surface`** (§2.5). Real and unenforced, but
  it belongs to the workflow list, not the experiment surface.
- Trend/regression/significance analytics and rollups (§3.5) — the engine
  is Arrow/DataFusion over Parquet (ADR-014, amended).
- Minting an experiment from Chat.

## 6. Verification

1. `cargo test -p dpl-hub-workbench -p egui-pilot-ai-lab -p egui-pilot-ai-runner`.
2. `cargo clean -p` the touched crates, THEN
   `cargo clippy --workspace --all-targets -- -D warnings` — this session's
   lesson: a warm clippy pass reports cached verdicts, and both hosted
   lanes went red on one after a local `green ✔`.
3. New unit pins:
   - `an_experiment_document_round_trips` — authored JSON → domain.
   - `a_run_without_an_experiment_is_still_summarised` — the optional
     fields never break existing bundles.
   - `runs_group_by_experiment_then_variant`.
   - `a_bundle_missing_a_required_artifact_is_reported_not_counted` —
     `requires` is a contract, and an unmet one is stated out loud.
   - `the_analyzer_scans_the_configured_root` — `artifact_root` from
     config, never the hard-coded literal.
4. `cargo green` → `green ✔`; merge ONLY on double-green.
5. **The live proof:** `workbench-experiments` drills to
   `workbench/experiments` on the real exe and asserts the experiment and
   both variants render from a REAL bundle on disk — not an authored
   fixture. H1's lesson: authored fixtures prove composition, never that
   the product works.
6. Proof shot of `workbench/experiments` at 1300×800.
7. Regressions: any existing workbench scenario, plus `chat-local-histories`
   (its bundle is the fixture this pass reads).

## 7. Commit plan

Branch `feat/workbench-experiments`. One commit:

```
feat(workbench): /experiments — experiments as domains, one analyzer
```

`gh pr list` first. #181 touches docs only and #183 is docs only, so no
code conflict is expected — but #181 owns the Lab vocabulary, so if it
lands first, adopt its names here rather than introducing synonyms.

## 8. Evidence

Filled during execution. §6.5 is the measure: a real run bundle produced by
a real drill, appearing under its experiment and variant without any
per-experiment capture wiring.
