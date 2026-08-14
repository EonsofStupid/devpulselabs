# Pass — Coach Seam S3b: Foundry v0 + the first clickable demo

**Status:** EXECUTED 2026-08-14 — operator `go` (2026-08-14); built in
the `../devpulselabs-s3b` worktree off main @ ea57cdc. Deviations from
the draft are marked **[AMENDED]** in place; evidence in §8.
Parent ADR `coach-seam-adr.md` §4 (Foundry) + §8.3; identity spine
landed in S3a (`coach-seam-s3a-identity-spine.md`); vocabulary landed
in S2 (`coach-seam-s2-protocol-rehoming.md`, protocol v2.7). Operator
asks carried in: **debug overlays** + **purge/remake** ("multiple
rounds to make this viable") and the **optimized channel** (AI reads
compact packs + folds; execution is local and token-free).

## 1. Context

Everything below the UI now exists and is merged: identity resolves by
evidence (S3a), the verb vocabulary is typed protocol law (S2), every
verb execution emits a seam span to Parquet (S1). But NOTHING is
clickable: the Coach hub still keys apps by heuristic string matching
(`seed_control_for` hardcodes "cursor"/"calculator" —
`crates/hubs/coach/src/lib.rs` l.51), the minted fabric dies with the
process, no pack has ever been BUILT by a pipeline (S3a's Blender pack
was written by a drill), and the operator has never pressed a button
and watched a fabric land in the catalog. S3b is the slice where the
mock's stages 1–3 (Pick → Foundry → Profile card) become the real exe.

## 2. Goal / non-goals

**Goal — the first clickable demo:**

1. A new egui-free crate **`dpl-coach-foundry`**: the staged,
   checkpointed pipeline (census → resolve → fabric → probe → induct →
   battery → publish) that builds a pack cell on disk, resumable after
   a kill, with **purge** and **remake** as first-class operations.
2. Every probe/battery execution emits a **seam span** (S1 shape) to
   the same Parquet layout — reliability per verb is measured, never
   asserted, and the pack's `[[verb]]` rows carry it.
3. The Coach hub gains the Foundry: Pick shows **resolved identity**
   (evidence score + pack tier, replacing heuristic guesses), a
   **"Build fabric (Foundry)"** action, a **job console** (stage rows,
   progress, purge/remake), and a **Profile card** (pack id, cell,
   hash, verb reliability, warnings; raw TOML under a collapsing
   Advanced section — ADR §6.1).
4. The hardcoded cursor/calculator seed strings **die**: the lesson
   seed comes from the pack's `[teaching] seed_control` (or first
   actionable anchor when absent), never from name sniffing.
5. A **fabric inspector** debug paint: the overlay binary gains an
   `inspect` mode that outlines resolved anchor rects over the live
   app — the "am I anchored where I think I am" loop the operator
   asked for.

**Non-goals (later slices, named):** Blender R0 bridge + Blender pack
(S4 — the seat is prepped, the bridge shape is proven by the spike) ·
overlay R0/X11 presence arm (S4, Blender needs it; calculator is
AT-SPI-visible) · active exploration (opening menus/dialogs in a
sandbox — S4+, v0 probes are NON-DESTRUCTIVE only) · Optimize cards
(S5) · DataFusion lens (blocked on an arrow-59 release, ADR §10) ·
remote registry · fabric-rung retirement micro-slice (**UNBLOCKED
2026-08-14** — the coldlight lane landed in #248 and
`crates/fabric/src/lib.rs` is free; still its own micro-slice, not
smuggled into S3b).

## 3. Ground truth (read in full, 2026-08-11, worktree off main @ 0a17312)

- `crates/hubs/coach/src/lib.rs` (869 l) — phase machine Pick →
  Confirm → Checks → Minting → Ready → LessonLaunched/Refused;
  `seed_control_for`/`mint_seed_name` hardcode cursor/calculator;
  mint runs on a thread over `mpsc` (the Foundry console reuses this
  exact pattern); overlay spawned as binary with `spotlight` args.
- `crates/coach-fabric/src/lib.rs` (227 l) — `mint(request, cancel,
  on_progress)` budgeted walk; `Fabric { nodes: Vec<FabricNode {
  path, role, name, actions }>, seed_path, minted_ms }`; in-memory
  only; `request_from_inventory_name` repeats the seed hardcoding.
- `crates/coach-pack` (S2 shape) — `PackRegistry::open(root)` /
  `list/load/write/resolve`; `FingerprintBundle::from_inventory`;
  `Pack` with `[teaching]` extras table, `[[verb]]` rows
  (verb/rung/reliability), promote-only `PromotedRung`; identity
  types now imported from `egui_pilot_ai_protocol::seam`.
- `crates/coach-runtime/src/lib.rs` — the full verb engine is public
  and daemon-free: `probe_presence`, `window_geom/move/resize/focus`,
  `app_open/close/wait_until/assert_state/type_text/press_key/
  select/scroll`, `resolve/invoke/extents/mount_spec`. The Foundry
  calls the engine directly; the daemon stays a devtools translator.
- `crates/coach-overlay` — IS a workspace member (gate-covered);
  binary has `spotlight` + self-test; lib has `IRect`,
  `compose_screen_rect`, frame-extent math the `inspect` mode reuses.
- S1 pattern (daemon): local `SignalFabric` + `LedgerTap` drain to an
  analytics root — the Foundry replicates this (hubs cannot reach the
  app's fabric; the Parquet files are the meeting point, per
  architecture.md).
- Operator lane: `feat/coldlight-53-hues` holds uncommitted
  `crates/{app,fabric,ledger,seam}` — S3b touches NONE of those
  files (deps on the crates are fine; edits are not).

## 4. Design

### 4.1 `dpl-coach-foundry` (new egui-free crate)

One concern: build a pack cell from live observation, checkpointed,
resumable, purgeable. Deps: `coach-inventory`, `coach-fabric`,
`coach-pack`, `coach-runtime`, `dpl-fabric` + `dpl-ledger` (spans),
`egui-pilot-ai-protocol` (verb names, rungs), `serde`/`toml`.

**Stages (v0, ADR §4 pipeline, non-destructive slice of it):**

| # | stage | does | artifact (checkpoint) |
|---|---|---|---|
| 1 | `census` | presence gate + fingerprint bundle from the inventory row | `foundry/census.toml` |
| 2 | `resolve` | bundle vs registry → `AppId` (or synthesize `x.unresolved:` and STOP for promotion — building for a guess is waste) | `foundry/resolve.toml` |
| 3 | `fabric` | budgeted mint (existing `dpl_coach_fabric::mint`); nodes persisted as **anchor rows** (path + role + actions + name-as-one-signal) | `foundry/anchors.toml` |
| 4 | `probe` | each SAFE verb executed K=3 times against the live app with its oracle: `presence`, `geom`, `focus`, `move`+restore, `resize`+restore, `wait_until`, `assert_state` — every run emits a seam span | `foundry/probes.toml` |
| 5 | `induct` | probe stats → `[[verb]]` rows (verb, rung, reliability = effect_ok/executed); seed anchor elected (first actionable) → `[teaching] seed_control` | staged pack body |
| 6 | `battery` | one verification pass per inducted verb — ALL green or no publish (ADR §4 ship gate) | `foundry/battery.toml` |
| 7 | `publish` | `PackRegistry::write` (content hash, tmp → bak → rename) | `pack.toml` |

**Checkpoint law [AMENDED at landing]:** the pack directory is the
checkpoint unit (`packs/<app-id>/foundry/` + `job.toml` cursor).
Census and Resolve are LIVE TRUTH — they re-observe on every run
(presence and evidence can change between runs; caching them would be
a lie). Fabric/Probe/Induct/Battery skip when their artifact survives;
Publish is idempotent. A killed job continues where it stopped, never
restarts from zero. Move/resize probes record the original geometry
FIRST and restore it even on failure. The kill is simulated by a
`halt_after` request field (drill arg `halt-after=<stage>`), not the
draft's `DPL_FOUNDRY_HALT_AFTER` env — a typed field over ambient
state.

**Purge/remake (operator ask, copy-never-clobber honored):**
`purge(app_id)` MOVES the whole `packs/<app-id>/` dir to
`packs/.trash/<app-id>-<unix-ms>/` — nothing is ever deleted, the
trash is inspectable, and re-runs start clean. `remake` = purge + run.

**Spans [AMENDED]:** the Foundry owns a local `SignalFabric` +
`LedgerTap` (daemon's S1 pattern). Probe and battery spans carry
`attrs: {"driver": "foundry", "foundry_stage": "probe"|"battery",
"atspi_name": <name>}` — the open attrs map doing exactly what §7.5
built it for (the draft's `foundry_job` id waits for multi-job
concurrency, which v0 does not have). A ledger that fails to open is
a LOUD console warning, never silent non-recording. The S1
`seam_reliability` fold answers "how reliable is `move` on this app"
from REAL rows.

**Roots [AMENDED]:** `FoundryRequest` takes explicit
`registry_root`/`analytics_root` (root-agnostic law — drills hand temp
dirs). The HUB resolves them: `DPL_PACK_ROOT` → the app's userdata
ring (`dpl_engine_capture::userdata_root()`), and `DPL_ANALYTICS_ROOT`
→ ring `/analytics` — the same ladder the daemon speaks, so Foundry
spans and daemon spans meet in the same Parquet files.

### 4.2 Coach hub (crates/hubs/coach)

- **Pick**: on selection, `FingerprintBundle::from_inventory` +
  `registry.resolve` → the right panel shows `AppId` + score +
  matched signals (or "stranger — generic profile") and pack state
  (none / built <date> / cell mismatch). The conquest sort stays.
- **New phase `Foundry`**: stage rows with live progress over `mpsc`
  (the mint pattern), per-stage ✓/→/refused, job id + checkpoint
  note, **Purge** and **Remake** buttons (confirm modal — destructive
  to the cell, not to history), honest ETA copy.
- **New phase `ProfileCard`**: pack id, cell, hash tail, `[[verb]]`
  reliability table, warnings; collapsing "Advanced" with the raw
  TOML; **"Inspect anchors"** debug button (spawns overlay `inspect`).
- **Seed law:** `seed_control_for`/`mint_seed_name` are DELETED. The
  lesson path reads the pack's `[teaching] seed_control`; when the
  pack has none (or the app is unresolved), the mint runs seedless
  and the session seeds on the first actionable node. The
  cursor/calculator unit tests become pack-driven tests.
- Import/lesson flow otherwise unchanged — Foundry is a peer action
  to Import, not a replacement (Import = this-session fabric; Foundry
  = durable pack).

### 4.3 Overlay `inspect` mode (crates/coach-overlay)

**[AMENDED]** New subcommand: `inspect --app <atspi> --name <anchor>`
(repeatable) `--seconds <n>`. The overlay LOCATES each named anchor
itself at spawn time (AT-SPI window-space extents — the same
`locate_target` the spotlight uses), which kills the draft's
extents-drift risk at the root: rects are live truth, never stored
coordinates. Misses are printed per name and counted on the caption
plate (`N anchors · M misses`). Paints click-through outline rects +
labels + a caption; no grabs, no XTEST — a debug lens, not a lesson.
Presence gate unchanged (AT-SPI; the R0/X11 arm is S4). The hub feeds
it the first 6 named actionable anchors from `anchors.toml`.

### 4.4 What this buys the token problem

The channel the operator asked for is now closed end-to-end: the AI
plans against a pack (KB-sized TOML: identity, ladder, verb
reliability, seed) + reliability folds (one Parquet query), and the
execution loop (probe, battery, lesson beats) is local Rust emitting
spans — zero tokens in the hot path. S3b makes the pack REAL so that
contract stops being theory.

## 5. Wiring

- Workspace member `crates/coach-foundry`; xtask `EGUI_FREE` census +
  `architecture.md` §3 entry (deps named: inventory, fabric, pack,
  runtime, dpl-fabric, dpl-ledger, protocol; consumed by
  `dpl-hub-coach`; never a dep of `app`) — same commit.
- `dpl-hub-coach` gains the `dpl-coach-foundry` dep (hub already
  depends on inventory/fabric/runtime — consistent direction).
- NO app/fabric/ledger/seam file edits (operator lane). No new
  harness flags in `app/main.rs` this slice; the drill hooks ride
  env vars (precedent: `CONTENT_ROOT`, `DPL_LEDGER`), flagged for
  red-pen: `DPL_COACH_VIEW=<phase>` lets the screenshot harness open
  the hub on the Foundry console / Profile card without a click.

## 6. Proof

**Unit (no X11):** job state machine walks stages in order · resume
skips completed stages (simulated kill between 3 and 4) · purge moves
to `.trash` (never deletes) and remake starts clean · induction math
(reliability = effect_ok/executed; refused counts score zero) ·
battery red → publish refused · seed elected from first actionable
anchor · hub classify tests become pack-driven (hardcoded-string
tests retired WITH the strings).

**Live (this seat, DISPLAY=:1, sequential):**
1. `foundry_drill` example: full pipeline against RUNNING
   gnome-calculator → `packs/org.gnome.Calculator/pack.toml` on disk
   with measured `[[verb]]` reliability; inspect the file.
2. Kill/resume **[AMENDED]**: run with `halt-after=fabric`, rerun —
   census/resolve re-observe (live truth), fabric SKIPS from its
   checkpoint, probe→publish complete. Checkpoint law proven.
3. Purge/remake drill: cell lands in `.trash`, second build succeeds.
4. `seam_reliability` over the drill's analytics root folds the probe
   + battery spans.
5. Overlay `inspect` on calculator: outline rects visible (manual
   verify + screenshot).
6. Exe screenshots at real sizes (`--window`): Coach hub Pick with
   resolved identity; Foundry console; Profile card (via
   `DPL_COACH_VIEW`) — committed under `docs/proof/`.

**Gate:** full `cargo green`; PR; merge on double-green only; STATE +
journal + resume pack ride the PR.

## 7. Risks

- **Slice size.** This is the largest coach slice yet (new crate +
  hub UI + overlay mode). Mitigation: the Foundry engine and the hub
  console are separable — if review wants two PRs, the cut line is
  "engine + drill green" / "hub UI + overlay".
- **Probe safety on a live app.** v0 probes only presence/geometry/
  focus/wait/assert — no text, no clicks, no lifecycle (open/close
  excluded: closing the operator's real app is not a probe). Geometry
  restored even on failure. Destructive probing waits for the S4
  sandbox (scratch profile law).
- **`DPL_COACH_VIEW` is a test hook in product code** — env-gated,
  documented, and inert unless set; flagged for operator red-pen
  (alternative: an `--open-coach <phase>` harness flag in app/main.rs
  AFTER the coldlight lane lands).
- **Calculator is the only live target** on this seat that is both
  AT-SPI-visible and safe; Blender validates the R0 arm in S4. The
  pipeline is app-agnostic by construction (identity via resolver,
  verbs via engine) — but S3b only PROVES it on one cell, honestly.
- **Anchor extents drift** between mint and inspect (app moved):
  inspect resolves extents at spawn time via the live handle, not
  from the stored anchor file.

## 8. Evidence (2026-08-14)

**Code:** new crate `dpl-coach-foundry` (stage/artifacts/probe/job +
`foundry_drill` example) · hub gains identity block, Foundry console,
Profile card, `DPL_COACH_VIEW` hook · overlay gains `inspect` ·
`coach-pack` gains typed `teaching_seed_control` · seed sniffing
DELETED from `coach-fabric::request_from_inventory_name` (now takes
the caller's seed), the hub (`seed_control_for`/`mint_seed_name`
gone), the bridge probe, and the daemon (`/import` seed comes from
the body only); `coach-runtime::resolve_seed` gained the
first-actionable fallback so a seedless fabric mounts something real.
The verb battery imports with an explicit `"seed": "7"` (it IS the
calculator script).

**Unit tests:** coach-foundry 13 (stage law, artifact round-trips,
minted identity, seed election, catalog-parse of every probe verb,
induct fold, checkpoint detection, job state, purge-to-trash) ·
coach-pack 22 (seed round-trips ride the existing signed-TOML tests) ·
coach-fabric 8 · hub-coach 4 (hardcoded-seed tests retired; replaced
by `identity_and_seed_come_from_the_registry_never_from_names` over a
temp registry).

**Live drills (gnome-calculator, DISPLAY=:1):** full pipeline → 84
anchors in ~430 ms, 7/7 verbs effect-green ×3 runs, battery 7/7,
signed pack published (`sha256:a91cd2af…`, reliability 1.0 rows,
`[teaching] seed_control = "Calculator"`) · halt-after=fabric then
rerun → `[ fabric] Skipped — checkpoint holds 84 anchors`, published ·
purge → `packs/.trash/org.gnome.Calculator-<ms>` (twice, both
inspectable) · remake → fresh cell published · with the pack
installed, resolve reports `score=8 via
desktop_id+atspi_name+exe_basename` (the identity loop closes) ·
`seam_reliability /tmp/s3b-ana` → 28 seam rows, 7 verbs, 100% effect
rate, avg_ms 12 (geom) to 472 (resize).

**Proof shots (committed):**
`docs/proof/coach-foundry-pick-1280x800.png` (Cursor honestly
UNRESOLVED with Build fabric offered),
`docs/proof/coach-foundry-console-1280x800.png` (7 stage rows, 100%,
checkpoint notes, Remake),
`docs/proof/coach-foundry-profile-1280x800.png` (pack cell, hash,
lesson seed, 7 measured verbs, Inspect anchors, Advanced raw TOML),
`docs/proof/coach-foundry-inspect-live.png` (live calculator with
cyan outline rects on 7/8/4/= · "4 anchors · 0 misses").

**Elected seed honesty note:** first-actionable elected `"Calculator"`
(the frame node carries an action on this GTK build) — lawful v0;
better election heuristics (role-weighted: button > frame) are S4
fuel, recorded here so nobody mistakes it for a bug later.
