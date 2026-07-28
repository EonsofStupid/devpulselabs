# Jobs-board migration — the three polled states collapse onto ONE JobBoard

Journal 2026-07-28 NEXT SERIES item 2. Doctrine: `docs/design/signal-doctrine.md`
jobs law ("collapsing those states onto the jobs domain is its own later pass" —
this is that pass) + law 7 (PROGRESS stays polled; signals fire at transitions).

## 1 · Context

W8 established the jobs domain as a contract: every long-running operation
(stitch render, pack ingest, guide export, workflow stages, chat imagine,
seam renders) publishes `JobEvent::Queued/Done/Failed` on the fabric —
ADDITIVELY, beside three bespoke polled states that remained the UI's truth:

- `RenderState` (composer) — the status row's render pill,
- `IngestState` + `ingest_done: Arc<AtomicUsize>` (composer) — the ingest pill,
- `ExportState` (composer) — the Guide tab's export pill,
- plus a fourth copy: the seam's `RenderStatus` map backing `GET /render`.

Four hand-rolled copies of the same idiom (Arc<Mutex<enum>> + poll +
repaint_after), each visible only on its own surface. A render started from
the editor is invisible from the Gallery; the status bar has no honest
"working…" truth. The journal's resume note names the fix: **one JobBoard
consumed by status UI, status-bar "working…" truth fed by jobs signals.**

## 2 · Current state (all read in-session at commit `eac44d2`)

- `crates/fabric/src/lib.rs` (817 lines):
  - `JobEvent { Queued{job,label} · Done{job,out} · Failed{job,error} }` (~line 157),
    `JobId(pub u64)`, `JobReceiver`.
  - `Fabric` struct (~432): per-domain broadcast senders (now incl. `intent`),
    `seq: Arc<AtomicU64>`, `born`, `waker`. `Fabric: Clone` shares channels.
  - `mint_job` (~506), `publish_job` (~525: `send(self.stamp(event)); self.wake()`),
    `subscribe_jobs`.
- `crates/composer/src/lib.rs` (5854 lines):
  - Fields (138–214): `render: Arc<Mutex<RenderState>>`, `ingest: Arc<Mutex<IngestState>>`,
    `ingest_done: Arc<AtomicUsize>`, `export: Arc<Mutex<ExportState>>`,
    `fabric: Option<dpl_fabric::Fabric>`, `capture_rx: Option<CaptureReceiver>`.
  - Defaults (303–336): the three states start Idle, fabric/capture_rx None.
  - Enum defs: `ExportState` (254), `RenderState` (563), `IngestState` (571).
  - `start_render` (1401–1463): validates via `render_parts()` (L3), empty →
    `RenderState::Failed("no playable clips to render yet")`; else Running,
    `mint_job` + `Queued{label:"render"}` behind `if let (Some(f), Some(j))`,
    thread runs `render_comp_resolving`, Ok → `index_render(&out, fabric.as_ref())`
    + `Done{out}` + `RenderState::Done(out)`; Err → `Failed` + `RenderState::Failed`.
  - `start_export` (1612–1676): same shape; guard `ExportState::Running`; label
    `"guide export"`; thread runs `run_export(..., fabric.as_ref())`;
    `ExportState::Done(e.dir)` / `Failed(e)`.
  - `keep_export` (1680–1700): reads `ExportState::Done(d)`, on keep Ok resets Idle.
  - `start_ingest` (1715–1776): guard `IngestState::Running`; resets `ingest_done`;
    label `format!("ingest {name}")`; thread: `Index::open_default()` +
    `packs::ingest_copy(&root, &pack, &name, &ix, &done)` (done: `&AtomicUsize`,
    bumped per file); Ok → `Done{out: format!("{copied} file(s)")}` + ONE
    `InboxChanged`; `IngestState::Done{pack,copied}` / `Failed(e)`.
  - `keep_render` (2299–2319): reads `RenderState::Done(p)`, on keep Ok resets Idle.
  - `ui()` drain (2356–2393): ingest pill match (Running → status + repaint 300ms;
    Done → status + Idle + cache reset; Failed → status + Idle), then render
    Running → repaint 200ms.
  - `status_row` (3945–3967): snapshots `(rendering, done, pill)` from RenderState.
  - `guide_tab` (4099–4115): snapshots `(exporting, pill, done_dir)` from ExportState.
  - `wire_fabric` (2166–2169), `signal` (2173–2177, if-let), `signal_saved`
    (2181–2191, takes `&Option<Fabric>`), `drain_signals` (2196–2214, if-let rx).
  - Other fabric users: workflow `run_workflow` (2103: `let fabric = self.fabric.clone()`
    + `let Some(f) = &fabric else { return }` inside the stage callback), fx-drill
    `index_render(&out, self.fabric.as_ref())` (1937), `signal_saved` call (1595, 2247).
  - `index_render(out, fabric: Option<&Fabric>)` (4712+, private) and
    `run_export(..., fabric: Option<&Fabric>)` (4768+, pub — called with `None`
    from `crates/app/src/main.rs:313` headless drill). Signatures UNCHANGED.
- `crates/seam/src/lib.rs` (1358+ lines at c48d3dc, grew with intent):
  - `RenderStatus { Running · Done · Failed(String) }` (319–323),
    `SeamState.renders: Mutex<HashMap<String, RenderStatus>>` (336, init ~403).
  - render command (~506–567): guard `Some(RenderStatus::Running)` → refuse;
    insert Running; thread publishes Done/Failed + Saved write-through, then
    inserts final RenderStatus.
  - `get_render` (718–737): maps None/Running/Done/Failed →
    `{"state":"idle"|"running"|"done"(+out rel)|"failed"(+error)}`.
- `crates/shell/src/lib.rs`: `activity_job_line` (95–102); Shell holds `fabric`
  + five ears incl. `jobs_rx` (drained ~400 into activity + signal_count);
  `status_bar` (~540 pre-intent) builds `status::StatusCtx { tone, viewport, ppp,
  navs, signals, build, mood }` and calls `status::grid`.
- `crates/shell/src/status.rs` (239 lines): `StatusCtx` (44–52), `CELLS` registry
  (62–93: brand, snip | viewport, mood | signals, build), cells are
  `fn(&mut Ui, &StatusCtx) -> Option<StatusAction>`; tests: unique ids, lanes
  non-empty.
- Laws re-read: CLAUDE.md, signal-doctrine.md (LOCKED; its jobs law explicitly
  reserves this migration), architecture doctrine (no new crates, no new deps
  → no audit-table change; fabric gains only std atomics).

## 3 · Target design

**THE JOBBOARD lives inside the Fabric** — a fold of the jobs stream plus the
PROGRESS side-channel, one per process, shared by every Fabric clone:

- `publish_job` first FOLDS the event into the board, then broadcasts + wakes
  (a consumer woken by the signal reads a board already true).
- `JobRow { job, label, state: JobState{Running·Done{out}·Failed{error}},
  progress: Arc<AtomicUsize> }` — `progress` is the latest-wins counter
  (doctrine law 7's PROGRESS class, centralized; workers bump it lock-free,
  UIs format it). No `JobEvent` wire change — NO Progress broadcast variant.
- Accessors on `Fabric`: `job(JobId) -> Option<JobRow>` · `jobs() -> Vec<JobRow>`
  · `jobs_running() -> usize` · `ack_job(JobId)` (removes the row — the
  consumer's "I've shown this outcome").
- Retention: Running rows never evicted; resolved rows persist until acked,
  capped at `RESOLVED_CAP = 64` (oldest resolved evicted first) so unacked
  publishers (workflow stages, chat, seam) never grow the board unboundedly.
- A `Done`/`Failed` with no Queued row inserts a resolved row labeled
  `(untracked)` — the board never panics, never lies by omission.

**Composer** drops the three enums + two state Arcs + `ingest_done`, and holds
three HANDLES instead: `render_job: Option<JobId>`,
`ingest_job: Option<(JobId, String /*pack*/)>`, `export_job: Option<JobId>`.
Every pill reads its handle's board row. `fabric` becomes NON-Option (a private
`Fabric::new()` by default — drills/tests publish into silence exactly as
before, but the board always works; `wire_fabric` swaps in the app's).
`capture_rx` becomes non-Option (subscribed to the private fabric at default).

**Seam** drops `RenderStatus`; `renders` becomes `HashMap<String, JobId>` and
`GET /render` reads the board (wire shape unchanged). The render thread no
longer writes a trailing status — the Done/Failed publish already updated the
board.

**Shell** gets the honest "working…" cell: `StatusCtx.working: Option<String>`
computed each frame from `fabric.jobs()` (0 running → None; 1 → its label
[+ progress count when > 0]; n → "n jobs"), painted by a new `jobs` cell in the
Right lane which requests a 300 ms repaint while Some — route-independent truth.

Scale story: the Nth job producer calls `mint_job` + `publish_job` exactly as
today and appears on every consumer (feed, SSE, status bar, GET /render) for
free; the Nth job CONSUMER reads `fabric.jobs()` — no new channels, no new
polled states, ever.

## 4 · Changes, in execution order

### Step 1 — `crates/fabric/src/lib.rs` (edit)

1a. Below the `JobReceiver` type alias, add:

```rust
/// One job's live row on THE JOBBOARD (doctrine law 7: PROGRESS is shared
/// polled state — this row is that state, centralized; signals stay
/// transition-only). `progress` is a latest-wins unit counter the worker
/// bumps lock-free (files copied, frames done); 0 until reported.
#[derive(Debug, Clone)]
pub struct JobRow {
    pub job: JobId,
    pub label: String,
    pub state: JobState,
    pub progress: Arc<AtomicUsize>,
}

impl JobRow {
    pub fn running(&self) -> bool {
        matches!(self.state, JobState::Running)
    }
}

/// A job's lifecycle position, folded from the jobs stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Running,
    Done { out: String },
    Failed { error: String },
}

/// Resolved rows kept for unacking consumers before eviction (oldest first).
const RESOLVED_CAP: usize = 64;
```

(`use std::sync::atomic::AtomicUsize;` joins the existing atomic imports.)

1b. `Fabric` struct: add field `board: Arc<std::sync::Mutex<Vec<JobRow>>>`,
init `board: Arc::new(std::sync::Mutex::new(Vec::new()))` in `new()`.

1c. `publish_job` folds BEFORE broadcasting:

```rust
    /// Publish a job transition — folded into THE JOBBOARD first, then
    /// stamped and broadcast, waking the UI (consumers woken by the signal
    /// read a board that is already true).
    pub fn publish_job(&self, event: JobEvent) {
        self.fold_job(&event);
        let _ = self.jobs.send(self.stamp(event));
        self.wake();
    }

    fn fold_job(&self, event: &JobEvent) {
        let mut board = self
            .board
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match event {
            JobEvent::Queued { job, label } => {
                board.retain(|r| r.job != *job);
                board.push(JobRow {
                    job: *job,
                    label: label.clone(),
                    state: JobState::Running,
                    progress: Arc::new(AtomicUsize::new(0)),
                });
            }
            JobEvent::Done { job, out } => {
                Self::resolve(&mut board, *job, JobState::Done { out: out.clone() });
            }
            JobEvent::Failed { job, error } => {
                Self::resolve(&mut board, *job, JobState::Failed { error: error.clone() });
            }
        }
        // Unacked resolved rows are bounded; Running rows never evict.
        let mut resolved = board.iter().filter(|r| !r.running()).count();
        while resolved > RESOLVED_CAP {
            if let Some(i) = board.iter().position(|r| !r.running()) {
                board.remove(i);
                resolved -= 1;
            } else {
                break;
            }
        }
    }

    fn resolve(board: &mut Vec<JobRow>, job: JobId, state: JobState) {
        match board.iter_mut().find(|r| r.job == job) {
            Some(row) => row.state = state,
            // A transition without a Queued — record it honestly anyway.
            None => board.push(JobRow {
                job,
                label: "(untracked)".into(),
                state,
                progress: Arc::new(AtomicUsize::new(0)),
            }),
        }
    }
```

1d. Accessors beside `subscribe_jobs`:

```rust
    /// One board row, cloned (`progress` shares the live counter).
    pub fn job(&self, job: JobId) -> Option<JobRow> { ... find + cloned }
    /// The whole board, in arrival order.
    pub fn jobs(&self) -> Vec<JobRow> { ... clone }
    /// How many jobs are running right now — the "working…" truth.
    pub fn jobs_running(&self) -> usize { ... count running }
    /// Remove a row — the consumer that showed the outcome acknowledges it.
    pub fn ack_job(&self, job: JobId) { ... retain != }
```

1e. Tests (module `tests`): `the_board_folds_the_job_stream` (Queued → row
running + jobs_running 1; Done → resolved with out, running 0; ack removes),
`board_progress_is_shared_with_the_worker` (bump via cloned row's Arc, visible
in a fresh `job()` read), `resolved_rows_evict_at_the_cap_running_never`
(publish 70 queued+done pairs + 1 queued-only → board holds 64 resolved + the
running row).

### Step 2 — `crates/composer/src/lib.rs` (edit)

2a. Fields: replace the four state fields (+ their doc comments) with

```rust
    /// The in-flight/last stitch render — a handle into the fabric's
    /// JobBoard (None = never rendered, or the outcome was acknowledged).
    render_job: Option<dpl_fabric::JobId>,
    /// The in-flight/last pack ingest + the pack's display name.
    ingest_job: Option<(dpl_fabric::JobId, String)>,
    /// The in-flight/last guide export.
    export_job: Option<dpl_fabric::JobId>,
```

`fabric: Option<dpl_fabric::Fabric>` → `fabric: dpl_fabric::Fabric` (doc: a
private fabric by default so drills/tests keep a working board — publishes go
to no subscribers; `wire_fabric` swaps in the app's ONE fabric).
`capture_rx: Option<dpl_fabric::CaptureReceiver>` → non-Option.

2b. `Default::default()`: build `let fabric = dpl_fabric::Fabric::new();
let capture_rx = fabric.subscribe_capture();` before the struct literal; init
the three handles to `None`, `fabric`, `capture_rx` accordingly; delete the
removed inits (`render:`, `ingest:`, `ingest_done:`, `export:`).

2c. Delete enums `RenderState`, `IngestState`, `ExportState` (their doc
comments too). Delete `AtomicUsize` from composer's imports if now unused
(`Ordering` stays — progress loads use it).

2d. `start_render`: empty-parts branch mints + publishes Queued then Failed
(the refusal becomes an honest fact; pill parity via the board):

```rust
        if parts.base.is_empty() {
            let job = self.fabric.mint_job();
            if let Some(old) = self.render_job.replace(job) {
                self.fabric.ack_job(old);
            }
            self.fabric.publish_job(dpl_fabric::JobEvent::Queued { job, label: "render".into() });
            self.fabric.publish_job(dpl_fabric::JobEvent::Failed {
                job,
                error: "no playable clips to render yet".into(),
            });
            return;
        }
```

Main path: drop the `state` Arc + `RenderState::Running` write; mint
unconditionally, ack-replace the old handle, publish Queued; thread clones the
non-Option fabric, calls `index_render(&out, Some(&fabric))` and publishes
Done/Failed exactly as today (no more `if let (Some(f), Some(j))` — direct).

2e. `start_export`: guard becomes
`if self.export_job.and_then(|j| self.fabric.job(j)).is_some_and(|r| r.running()) { return; }`;
same mint/ack-replace/publish/thread rewrite (`run_export(..., Some(&fabric))`).

2f. `start_ingest`: guard via `self.ingest_job`'s row running (status message
unchanged); after publishing Queued, take the row's counter —
`let done = self.fabric.job(job).map(|r| r.progress).unwrap_or_default();` —
thread passes `&done` to `packs::ingest_copy` (signature untouched); delete
`ingest_done` reset; set `self.ingest_job = Some((job, name.clone()))`
(ack-replace any old handle).

2g. `ui()` drain: rewrite the ingest pill block against the board row
(Running → status "Ingesting {pack} — {n} file(s) processed…" from
`row.progress` + repaint 300 ms; Done{out} → status
`"Ingested {pack} — {out} now local in your library."` + ack + clear handle +
cache reset (out is already "N file(s)"); Failed{error} → status + ack + clear;
row missing → clear handle). Render keep-alive:
`if self.render_job.and_then(|j| self.fabric.job(j)).is_some_and(|r| r.running())`.

2h. `keep_render` / `keep_export`: read `Done { out }` from the handle's row
(`PathBuf::from(out)` — out was minted by `.display()` on this same path);
on keep Ok, `ack_job` + clear the handle (replaces the `Idle` resets).

2i. `status_row` snapshot from the board:

```rust
        let row = self.render_job.and_then(|j| self.fabric.job(j));
        let (rendering, done, pill) = match row.as_ref().map(|r| &r.state) {
            None => (false, false, None),
            Some(dpl_fabric::JobState::Running) =>
                (true, false, Some(("Rendering...".to_string(), cl::ink::BASE))),
            Some(dpl_fabric::JobState::Done { out }) => (false, true,
                Some((format!("Rendered: {}", Self::short(&PathBuf::from(out))), cl::ink::STRONG))),
            Some(dpl_fabric::JobState::Failed { error }) =>
                (false, false, Some((format!("Render failed: {error}"), cl::ink::MUTED))),
        };
```

2j. `guide_tab` snapshot likewise from `export_job`'s row (Done pill text
`"Exported: {}/guide.html"` via `Self::short(&PathBuf::from(out))`; `done_dir =
Some(PathBuf::from(out))`).

2k. Fabric plumbing simplifications: `wire_fabric` sets both non-Option
fields; `signal` publishes directly; `signal_saved` takes `&dpl_fabric::Fabric`
(call sites textually unchanged); `drain_signals` drops the `if let Some(rx)`;
`run_workflow`'s stage callback drops `let Some(f) = &fabric else`; fx-drill's
`index_render(&out, self.fabric.as_ref())` → `Some(&self.fabric)`; W8 gap-fix
site 1595 / import site 2247 unchanged text.

### Step 3 — `crates/seam/src/lib.rs` (edit)

3a. Delete `RenderStatus`; `renders: Mutex<HashMap<String, JobId>>` (doc: rel →
the job on the fabric's board; the board is the status truth).

3b. Render command: guard = existing entry whose board row is running; then
`renders.insert(rel.to_string(), job)` right after `mint_job` (before spawn);
delete the thread's trailing `renders...insert(key, status)` block and the
`let status = match result {...}` (the publishes already fold the board);
`key` binding deleted if unused.

3c. `get_render`: entry → `s.fabric.job(id)` → map `None`(either) → idle,
Running → running, Done → done + `out: format!("{rel}/render.mp4")`
(wire shape unchanged), Failed{error} → failed + error. An entry whose row was
evicted reads idle — pre-release honest enough; documented at `RESOLVED_CAP`.

### Step 4 — `crates/shell/src/status.rs` + `crates/shell/src/lib.rs` (edit)

4a. `StatusCtx` gains `pub working: Option<String>` (doc: the jobs-board
"working…" truth — None = quiet).

4b. New cell registered in `CELLS` Right lane between `signals` and `build`:

```rust
    StatusCell { id: "jobs", lane: Lane::Right, render: jobs },
```

```rust
/// The JobBoard's honest "working…" truth — visible from EVERY route.
fn jobs(ui: &mut Ui, cx: &StatusCtx) -> Option<StatusAction> {
    if let Some(text) = &cx.working {
        cl::tone_dot(ui, cx.tone);
        ui.label(RichText::new(text).small().color(cl::ink::BASE));
        // Progress counters tick without a signal — keep the bar live.
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
    }
    None
}
```

4c. `Shell::status_bar` computes `working` before building `StatusCtx`:

```rust
        let running: Vec<dpl_fabric::JobRow> =
            self.fabric.jobs().into_iter().filter(|r| r.running()).collect();
        let working = match running.as_slice() {
            [] => None,
            [one] => {
                let n = one.progress.load(std::sync::atomic::Ordering::Relaxed);
                Some(if n > 0 {
                    format!("working · {} · {n}", one.label)
                } else {
                    format!("working · {}", one.label)
                })
            }
            many => Some(format!("working · {} jobs", many.len())),
        };
```

and passes `working` in the `StatusCtx` literal.

### Step 5 — docs (edit)

5a. `docs/design/signal-doctrine.md`: the jobs law's tail — replace the "This
pass adds the channel … its own later pass. QoL payoff now: …" sentences with
the migrated record: THE JOBBOARD (jobs-board-migration, 2026-07-28) is the ONE
shared polled state, folded inside the fabric from every `publish_job`;
per-job progress is the row's latest-wins counter; RenderState/IngestState/
ExportState/RenderStatus are gone; the status bar's `working…` cell and the
seam's `GET /render` read the board. Law 7's PROGRESS line: "(the RenderState
idiom)" → "(the JobBoard row's latest-wins counter — formerly the RenderState
idiom)". Consumers map: add the status-bar jobs cell + GET /render.

5b. `docs/STATE.md`: session-end entry (this pass, proof pointers).

## 5 · Boundaries

- 🚫 `/content` (operator data), the original app checkout, `JobEvent`'s wire
  shape (`job_json`, `events.jsonl` — pinned by seam tests, untouched), the
  intent/coach/capture domains, `packs::ingest_copy`'s signature,
  `run_export`/`index_render` public signatures (main.rs headless caller keeps
  passing `None`).
- 🚫 NO `JobEvent::Progress` broadcast variant — law 7 forbids progress on the
  signal stream; the board row IS the progress mechanism.
- ⚠️ Ask first: migrating chat.rs's per-draft job tracking, Browse's scan
  caches (proactive-buffer, not jobs), any mpsc COMMAND channel (reserved for
  R2c).
- Out of scope: jobs history/ledger (Phase 2 dpl-ledger), a jobs inspector
  panel, per-stage workflow progress UI.

## 6 · Verification (exact, in order)

1. `cargo test -p dpl-fabric` — the three new board tests + existing jobs
   tests pass.
2. `cargo test -p dpl-composer -p dpl-seam -p dpl-shell` — exit 0.
3. `cargo green` — fmt, clippy (no `#[allow]`), full tests, structure audit,
   release build all green.
4. Real-exe seam truth (sequential, staged scratch root): launch
   `devpulselabs.exe --seam-port 4599 --window 1280x832`, `POST /command
   {"cmd":"render","dir":"inbox/<staged comp>"}` → 202; `GET /render?dir=…`
   returns `running` then `done` with `out` (the board answering, not the old
   map); `/signals` SSE shows `{"domain":"jobs","type":"queued"…}` then `done`
   with increasing `seq`.
5. Screenshot proof at 1280×832: during a running render/ingest the status bar
   shows `working · render` (the jobs cell) from a NON-composer route; after
   completion the composer status row shows the `Rendered: …` pill (board-
   backed). Saved under `docs/proof/`.
6. Exe rebuilt in THE checkout (`cargo build --release -p devpulselabs -p
   dpl-lottie-render`), mtime moved.

## 7 · Commit plan

Branch `feat/jobs-board-migration` in a worktree lane (parallel-session law —
main moved under this session once already). ONE commit:

    feat(jobs): ONE JobBoard on the fabric — the three polled states collapse

    RenderState/IngestState/ExportState (composer) + RenderStatus (seam) fold
    into a board inside the fabric: publish_job folds first, then broadcasts;
    progress is a latest-wins counter on the row (doctrine law 7); the status
    bar gains the honest "working…" cell; GET /render reads the board.

`git add` the four crates + `docs/plans/jobs-board-migration.md` +
`docs/design/signal-doctrine.md` (+ proof shots + STATE.md). Push branch,
`gh pr create`, merge on green, then rebuild the release exe in
`C:\localdev\devpulselabs` and update `docs/STATE.md`.

## 8 · Evidence

Ground truth read in-session 2026-07-28 at `eac44d2` (post #110 L3 +
#116 intent-gate — main moved mid-grounding; every site re-read after):
fabric lib.rs 100–620 + intent diff; composer lib.rs 100–340, 505–575,
1395–1780, 1580–1607, 1860–1975, 2150–2400, 3940–3990, 4090–4120 + grep maps
of RenderState/IngestState/ExportState/self.fabric/signal_saved/ingest_copy;
seam lib.rs 300–400, 480–600, 700–772 + RenderStatus grep; shell lib.rs
60–280, 380–565 + status.rs in full; signal-doctrine.md in full; STATE.md,
journal + next-phase notes in full.
