# W8 — the faithfulness pass: quiet spawns · proactive buffers · the signal doctrine

Operator (2026-07-26, verbatim): "please stop rushing this … terminal
windows end up staying ontop of the actual application. The browser
library window is still taking over 20~ seconds to load. There has to
be a way to establish clean patterns that allow us to have that
proactively buffered. As well as ensure this is emitting signals like
my original … i guarantee you are not doing consistent patterns for my
signalkind."

STATUS: EXECUTION-READY, awaiting operator GO. Hardened via
/deep-plan: two Opus evidence sweeps (spawn-site inventory ·
mutation/emit-site inventory), three claims spot-checked by Fable
against the real files (grok hide_child_console :512-519; the main.rs
boot-ordering bug :506-516 vs :520-521; clip.rs probe stdio :58-64) —
all verified. The doctrine itself is LOCKED separately in
docs/design/signal-doctrine.md (operator-directed, incl. the
pre-release reframe: original's DNA + our index vocabulary + the
boundary ENVELOPE + the jobs domain contract). Three legs, each a
HOUSE PATTERN, not a spot fix.

## 1. Context

Three faithfulness failures, all pattern-level. (a) Subprocess spawns
pop console windows over the GUI app. (b) Browse windows freeze ~20s
on open. (c) The original's signal doctrine — every mutation ends in
"index + SIGNAL" on one typed channel — was never ported; our surfaces
poll and go stale.

## 2. Current state (ground-truthed 2026-07-26)

- **Console windows**: ~16 crates call `Command::new`; only 4 call
  sites anywhere set `creation_flags`. The hot paths — dpl-video's
  preview pipe (every clip preview), engine-media (posters, renders,
  gif, replay), engine-capture (records, probes), engine-ae
  (aerender) — spawn consoles that z-fight the GUI exe
  (`windows_subsystem = "windows"`). The original's Tauri sidecar
  suppressed consoles automatically.
- **Browse latency** (`crates/composer/src/browse.rs`, read IN FULL):
  `window_ui` scans the folder SYNCHRONOUSLY on the UI thread
  (`b.entries = Some(scan(&b.cwd))`) — 70k-file packs on the D:
  spinning disk = the 20s freeze; AND `roots()` probes drive letters
  A:–Z: with `is_dir()` EVERY FRAME — sleeping/ghost drives block
  seconds per probe. The deep-search worker (`start_search`) already
  has the right off-thread shape; the ordinary path never got it.
- **Signals** (the original at `D:/localdev/devpulse labs`, read:
  `src-tauri/src/commands/events.rs`, `events.native.ts`, + the five
  emitting files): ONE channel `dpl:capture`, ONE tagged enum
  `CaptureEvent { Started{mode,target} · Saved{file,kind,name,
  destination,source} · Failed{error} · InboxChanged ·
  PaletteVisibility{visible} }` (serde tag="type", camelCase); ONE
  helper (`index_saved` — "ONE place so every PixVix path indexes +
  signals identically"); every mutation site ends "index + signal"
  (region/freeform capture, flatten, steps save, session save, keep,
  dump, staging pull); ONE schema-validated UI subscription (arktype;
  malformed dropped with a warn, never a throw); documented
  integration point (the AI labeler subscribes to Saved).
  OUR port: `dpl_fabric::ShellEvent` covers shell/navigation ONLY.
  ZERO capture signals exist — grep proves no emission on import,
  render write-through, guide export, keep, dump, ingest; every view
  manually invalidates (`self.palette = None`, refresh chips, rescan
  taps) and goes stale between surfaces. "SignalKind" appears in no
  port doc — the doctrine was silently dropped at porting time.
- NOTE: the original repo MOVED to `D:/localdev/devpulse labs`
  (C:\localdev copy is gutted droppings). D: is slow — greps take
  minutes; read files directly by path.

## 3. Target design — three house patterns

**A. The QUIET-SPAWN law.** One helper (new tiny crate `dpl-proc` or a
shared module in engine-capture — decided at execution by dependency
direction: engine-capture is already the lowest engine): `quiet(cmd:
&mut Command)` setting `CREATE_NO_WINDOW` (0x08000000) on Windows
(no-op elsewhere) + stdio policy. EVERY subprocess site in
engine-capture, engine-media, dpl-video, engine-ae, engine-browser,
chat providers routes through it. ENFORCEMENT: a workspace test that
greps crates/ for `Command::new` files lacking the helper import —
the Nth spawn cannot regress silently.

**B. The PROACTIVE-BUFFER law.** No UI surface ever runs filesystem
scans/probes on the paint thread. For Browse: (1) `roots()` computed
ONCE per window (cached in `Browser`; drives change rarely — a
refresh chip re-probes off-thread); (2) the folder scan moves to the
existing Search worker shape (`entries: None` → spawn → honest
"opening this folder…" label → fill); (3) a WARM CACHE seeded at
Composer boot on a background thread: profile roots + pinned packs
pre-scanned into a shared `Arc<Mutex<HashMap<PathBuf, Vec<Entry>>>>`
so the FIRST window opens instantly against the buffer (staleness
honest: cache hit renders immediately, a fresh scan revalidates
behind it). The pattern is documented in doctrine.md §perf as the
standing law for every future surface.

**C. The SIGNAL doctrine.** AUTHORITATIVE TEXT:
`docs/design/signal-doctrine.md` (LOCKED — the six laws: per-domain
channels · original's DNA with OUR index vocabulary · emit-with-
mutation · serde-free fabric with the boundary ENVELOPE
`{"domain","ms","type",…}` · lossy-broadcast-drain-per-frame · the
jobs domain contract established now, migrated incrementally). This
plan implements it; where this section and the doctrine differ, the
doctrine wins.

## 4. Changes, in execution order (evidence-anchored)

**Commit 1 — the quiet-spawn law.**
1. CREATE `crates/proc` (`dpl-proc`, ZERO deps): `pub fn quiet(cmd:
   &mut Command) -> &mut Command` — cfg(windows)
   creation_flags(0x0800_0000), no-op elsewhere; unit test.
2. Apply at the NEEDS-QUIET sites (agent-inventoried, spot-checked):
   `video/src/lib.rs:157` (loop pipe) · `video/src/clip.rs:218`
   (clip pipe — respawns EVERY seek: the worst flasher) ·
   `video/src/clip.rs:58` (duration probe — ALSO runs blocking on
   the UI thread; quiet now, async probe is a named follow-up) ·
   `engine-capture/src/lib.rs:74` `ffmpeg_cmd()` factory (one line
   covers its 5 spawn sites incl. the UI-thread dshow device enum) ·
   `engine-media/src/lib.rs:281` `run_ffmpeg` (ALSO null its
   currently-INHERITED stdio) · grok `profiled_command`
   (lib.rs:501— collapses its local hide_child_console duplicate) ·
   codex `profiled_command` (activation.rs:511 — same collapse; fixes
   the 2 unquieted probe_version/probe_login_status flashes) ·
   engine-browser lib.rs:295 + lottie lib.rs:70,138 (cheap
   insurance; not GUI-reachable today).
   EXEMPT (recorded, untouched): xtask, build.rs, capture_cli,
   pilot-cli/runner (harness debugging value), chat_native_proof
   (self re-exec must inherit), guide test.
3. Enforcement in `cargo green` (xtask): workspace scan — a file
   containing `Command::new` must import `dpl_proc::quiet` or carry
   `// spawn-audit: exempt(<reason>)`; failures name the file.

**Commit 2 — the capture + jobs domains (signal-doctrine.md).**
4. `crates/fabric/src/lib.rs` — `CaptureEvent` (Clone, NOT Copy —
   String fields; precedent fine) + `JobEvent`/`JobId` + two channels
   + publish/subscribe pairs, following :91-126's exact shape; tests.
5. FIX the boot-ordering bug FIRST: `crates/app/src/main.rs` — move
   `wire_fabric` calls (:520-521) ABOVE the harness drills
   (:506-516); fabric exists from :486, the move is legal. Drills
   must signal like real actions.
6. Emit sites, per the sweep table: viziwiz needs ZERO plumbing
   (`self.fabric` :101) — snip×4 (lib.rs:486/:520/:543 +
   editor.rs:371/:400), steps save (:432 `save_pending` →
   Saved{kind:"guide"}+InboxChanged), video done-drain (:818-832),
   session done-drain (:838-857, partials Saved+Failed both — the
   code files partials deliberately), Started at each vehicle start,
   Failed at each Err arm; the SEVEN `self.open` writes funnel into
   one `set_open(bool)` publishing PaletteVisibility exactly once
   per change. Composer: add `dpl-fabric` dep + `fabric:
   Option<Fabric>` field (hand-written Default :198!) +
   `Composer::wire_fabric` called from `WiredfrontHub::wire_fabric`
   (:922); `handle_import` (:1267) emits for the import seam
   (import_file itself gains NO fabric — emission at the routed
   caller, keeping tests hermetic); render worker closure (:889)
   clones the fabric, `index_render` gains `Option<&Fabric>`;
   `run_export` (:3083) same; `packs::ingest_copy` (:204) gains the
   param beside its AtomicUsize; keep_render/:1316,
   keep_export/:1070 emit Saved{location change}+InboxChanged.
   Wiredfront hub (fabric already :122): Dump (:355) →
   InboxChanged, KeepTo (:373) → Saved+InboxChanged, rescan (:276)
   → InboxChanged. Jobs domain: Queued/Done/Failed emitted beside
   the three existing polled states (render :873/:2367, ingest
   :1100, export :1027) — additive, states untouched.
   INDEX TIER STAYS SIGNAL-FREE (agent finding ratified: emitting
   inside dpl-index would double-fire from the CLI binary and N-fire
   from reindex's internal upserts).
7. Consumers: wiredfront views (Gallery/Library/Sessions) + composer
   caches hold a capture Receiver, try_recv per frame → invalidate;
   session `EventLog` gains a capture drain (second rx in SessionRec,
   envelope lines in events.jsonl); seam `/signals` forwards capture
   + jobs in the ENVELOPE (`{"domain","ms","type",…}`) — wire-shape
   tests IN SEAM per kind; status-bar counter counts all domains.
8. NAMED GAP (found by the sweep, fixed here): `apply_review`
   (:1003) mutates the guide with NO index write — add the upsert +
   Saved. NAMED GAP (recorded, own pass): three composer upserts
   skip sidecars (import.rs:99, lib.rs:3051, :3134) — sidecar parity
   pass queued, not this diff.

**Commit 3 — the proactive-buffer law.**
9. `browse.rs`: `roots()` computed once per window (cached field;
   refresh chip re-probes off-thread); folder scan → the Search
   worker shape (honest "opening this folder…" state); Composer owns
   the warm cache (Arc<Mutex<HashMap<PathBuf, Vec<Entry>>>>), seeded
   on a background thread at first `ensure_project` (profile roots +
   pinned packs), handed to every `Browser::new`; hit paints
   instantly + revalidates behind.
10. doctrine.md gains §perf (the buffer law) + the signal-doctrine
    reference; friction rows 16/17/18; STATE at PR time.

## 5. Boundaries

🚫 The ShellEvent channel's existing five variants — untouched (the
signal channel is a SIBLING, as in the original: capture signals ≠
navigation). 🚫 No behavior change to what gets indexed — only that
every index write now signals. 🚫 The original stays READ-ONLY at its
NEW D: home. ⚠️ Ask-first: folding the AI-labeler subscription in now
(the original documents it; our chat/agent lane may want it — its own
pass unless asked).

## 6. Verification

1. Wire test: serialized CaptureSignal::Saved == the original's JSON
   shape ({"type":"saved","file":...,camelCase}). Enforcement test
   fails on an unflagged Command::new file.
2. REAL-EXE: launch the app, run a snip + an import + a render →
   ZERO console windows appear (operator-eyes + a GetConsoleWindow
   probe if automatable); Gallery/Library/FOOTAGE update WITHOUT
   refresh taps (the signal proof — screenshot before/after);
   `/signals` SSE captures saved/inboxChanged during an import drill.
3. Browse drill: open a Browse window on the D: pack root — the
   window paints within one frame with "opening…" then fills;
   the roots bar renders without per-frame drive probes (profile a
   frame or assert via a probe counter in debug).
4. Proof shots + `cargo green` + sequential exe runs.

## 7. Commit plan

Branch `feat/w8-faithfulness`, three commits (quiet-spawn law · browse
buffers · signal doctrine), PR → merge → exe rebuild → STATE.

## 8. Evidence (2026-07-26 execution; provenance: two Opus sweeps
Fable-spot-checked, all decisions + code Fable's)

1. `cargo green ✔` INCLUDING the new spawn audit ("every Command::new
   file is quiet or exempt"). Tests: fabric 8/8 (stamp order across
   domains, JobId in the sequence space, the waker fires per publish),
   seam 12/12 (envelope pinned per kind incl. camelCase + domain on
   shell), viziwiz 7/7, composer 38/38, dpl-proc 1/1.
2. QUIET SPAWNS: dpl-proc applied at video (loop pipe, clip pipe —
   the every-seek flasher — and the UI-thread duration probe),
   engine-capture's factory (5 sites), engine-media run_ffmpeg (stdio
   also nulled — was fully INHERITED), grok + codex factories (their
   duplicate helpers collapsed; the unquieted probe flashes fixed),
   engine-browser + lottie. Exempt markers at the 7 console-native
   files. Operator-eyes acceptance: no console windows over the app.
3. SIGNALS LIVE, twice over: (a) SSE capture during a REAL agent
   render: `{"type":"queued","job":0,"label":"render inbox/w7-demo",
   "domain":"jobs","seq":1,"ms":6092}` then `failed` seq 2 ms 6096 —
   stamped, ordered, and the failure VISIBLE in-stream instantly (the
   doctrine's honesty working; the failure itself is the parallel
   session's embedded-deps staging gap, see 6). (b) The ACTIVITY FEED
   screenshot docs/proof/w8-activity-feed.png: the import drill's
   "00:00 saved image · feed-proof" line + footer "signals · 2" — the
   drill emitted like a real action because BOTH boot-ordering bugs
   are fixed (wire-before-drills AND ears-at-fabric-birth; the second
   was caught BY the feed's empty state — the debug surface debugged
   its own boot).
4. BROWSE BUFFERS: scans off-thread through the Search-worker shape,
   the warm cache seeded at first editor visit (roots probed INSIDE
   the worker — never the paint thread, not even once), cache hits
   paint instantly with revalidate-behind, honest "opening this
   folder…" cold state. The 20s freeze's two causes (sync scan on
   paint + per-frame A:–Z: drive probes) are both structurally gone.
5. MISSED-SITE CATCH: the seam's agent render emitted nothing — found
   BY the silent SSE proof, wired (jobs + Saved with index
   write-through), the stream then spoke. "Every mutation signals"
   now includes agent-driven ones.
6. FOUND (parallel-lane state, not this pass): main now carries the
   embedded-deps verified-ffmpeg system + three-ring root; staging
   requires codex+grok+ffmpeg sources and `sources.local.toml` is
   unauthored on this machine, so RENDERS FAIL app-wide until the
   operator stages (an accurate ffmpeg-only sources.local.toml was
   drafted, gitignored; the stager refuses partial sets). The W6
   PATH-based resolver was superseded by that system.
7. DEVIATIONS: browse roots refresh chip skipped (roots re-probe per
   new window — rarely-changing; noted); comp.json saves stay
   signal-free (documents, not artifacts); command channel is
   contract-only per ratification.
