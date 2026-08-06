# Program — the browser never blocks, GIMP joins the bench, Adobe gets an exit

## 1. Context

Operator ask, 2026-08-06, verbatim (three asks in one message):

> "I specifically know that browse on these apps always when first being
> developed freeze up and if you take your time to research it shouldnt and
> we can plan it out smart. batches, streamed loading, browser on the
> wiredfront editor is not at all a userfriendly experience for non
> technical it also is not at all how after effects or any common gimp even
> editor works."

> "The editor needs to have gimp properly added as a sub-process to
> accompany wiredfront and have its new photogimp."

> "We also need to research while i have the active subscription for
> creative cloud workflows that will allow us to methodically migrate and
> export to opensource my purchased templates, presets for my wiredfront
> video editor and supports images as well"

Three arcs: **A** the media browsing experience stops freezing and becomes
layman-friendly; **B** GIMP as an ADR-009 subprocess companion (with
PhotoGIMP); **C** the Adobe-exit migration (operator-run playbook:
`docs/runbook/adobe-exit.md`).

Rulings that bind this program (nothing here re-opens them):

- **Consent-gated thumbnailing** — "ask to cache" (operator, 2026-08-05,
  captured in `docs/plans/wiredfront-library-and-player.md` §1/§5). Bulk
  thumbnail generation is offered, never imposed.
- **`browse.rs`'s own doctrine** (`crates/composer/src/browse.rs:14-25`):
  *"No UI surface ever walks a directory on the paint thread"* and *"Roots
  are probed INSIDE the worker."* Arc A is enforcement of an
  already-ratified law, not a new invention.
- **ADR-009** — ffmpeg/aerender/GIMP are subprocesses beside the exe, never
  crate deps. No `gimp` crates in any Cargo.toml, ever.
- **Two-libraries law** — WiredFRONT library is RAW MEDIA; Workbench library
  is ColdLight ingredients. Arc C's outputs ingest into the WiredFRONT side.
- **Sources are never mutated** — thumbnails and GIMP edit sessions work on
  copies/cache paths, never beside or on the operator's media.
- **The product is the EXPERIENCE** — Arc A's gate is feel (first paint
  instant, placeholders then thumbs, tooltips), not just profiler numbers.

Status of the predecessor plan `wiredfront-library-and-player.md`
(measured 2026-08-06): part **A** (editor dress) landed via the matrix arc
(#211); part **C** (preview popup) landed — `crates/composer/src/player.rs`
exists (319 lines, #211); part **B** ("the library caches" — `poster_for`,
`ThumbJob`, the consent prompt) **never landed**: zero grep hits for either
symbol. Arc A below absorbs and supersedes that part B.

## 2. Current state (grounded in-session 2026-08-06, HEAD `fe0e162`)

Full ranked audit lives in §8. The load-bearing findings:

**Four browsing surfaces**: Browse windows (`crates/composer/src/browse.rs`,
raw fs), PROJECT panel (`crates/composer/src/lib.rs:3422`, index + fs),
Library/Gallery shelves (`crates/hubs/wiredfront/src/lib.rs:629`/`:227`,
index). Workbench library is pure in-memory registry — not implicated.

**The freeze, ranked (S1 is the killer):**

- **S1** — `Index::open_default()` is called **four times on the editor's
  first paint** (`crates/composer/src/lib.rs:1763,1776,1835,1842`) and once
  per Library/Gallery/Sessions visit. Each call builds a fresh tokio
  runtime, opens turso, runs `CREATE TABLE IF NOT EXISTS`, connects twice.
  A prior incident is already documented in-repo: calling this per frame
  "hung the UI thread hard enough that a proof run never reached its
  screenshot" (`crates/hubs/workbench/src/author.rs:1055-1061`).
- **S2** — zero virtualization anywhere (`show_rows` has no call sites in
  the workspace). Library paints up to 500 cards per frame; frame 1 fires
  `Image::paint_at` per card, and egui_extras' stock loaders spawn **two OS
  threads per image URI**, decode at full resolution, ignore `SizeHint`,
  and never evict. 500 cards ⇒ up to 1000 threads + gigabytes of decoded
  pixels, process-lifetime.
- **S3** — first paint runs `packs::list_packs` → recursive `count_media`
  walks over PINNED EXTERNAL/USB sources (`lib.rs:1818` →
  `packs.rs:100,65`). A sleeping drive blocks the app for seconds.
- **S4** — Library route runs `count_files(elements)` — recursive,
  uncapped, uncached — **every frame** (`hubs/wiredfront/src/lib.rs:715`).
- **S5** — opening Browse probes **26 drive letters** + reads
  `libraries.json` on the UI thread (`browse.rs:114`, `:246-252`) —
  violating the module's own header law.
- **S6-S13** — blocking `ffmpeg -i` duration probes reachable from the
  paint path (`crates/video/src/clip.rs:154,101`); `read_libraries()` disk
  read + JSON parse every frame (`browse.rs:454`); per-card `is_file()`
  stat every frame (`thumb_uri`); Explorer-drop imports run copy + 2 ffmpeg
  spawns per file synchronously (`lib.rs:2162→import.rs:236`); `pack_size`
  uncapped walk on first Ingest tap; Lottie render blocking per frame/lane
  (self-documented at `lib.rs:6308`).

**What already exists to build on (do not reinvent):**

- `dpl_fabric::Fabric` job bus (`crates/fabric/src/lib.rs:749`): `mint_job`,
  `JobRow.progress: Arc<AtomicUsize>`, `attach_waker` already wired to
  `ctx.request_repaint()` (`crates/app/src/main.rs:1602`). Any background
  publish already repaints the UI.
- Two working off-thread reference shapes: `Composer::start_ingest`
  (`lib.rs:2010-2064`, job + progress + batched invalidation) and the
  Browse scan slot (`browse.rs:58,376-406`, `Arc<Mutex<Option<T>>>` +
  `request_repaint_after(60ms)`).
- The house frame budget precedent: `THUMBS_PER_FRAME = 8` (`browse.rs:47`).
- `PreviewCaps::probe()` (`lib.rs:6296`) for sizing worker pools.
- Poster machinery: `dpl_engine_media::write_poster` (ffmpeg subprocess,
  `engine-media/src/lib.rs:276`), today invoked only at import/ingest —
  which is why **videos have no thumbnails in Browse at all** (by design,
  `browse.rs:738-739`).
- **No disk thumbnail cache exists.** `.userdata/cache/` is created and
  declared regenerable (`engine-capture/src/lib.rs:256,199`), used only by
  Imagine. Greenfield.

## 3. Target design

### Arc A — the browser never blocks

The doctrine, applied everywhere: **first paint needs zero I/O**. Every
browsing surface paints instantly from an in-memory snapshot (worst case:
skeleton cells + names), and background workers stream results in via the
Fabric-waker channel pattern. This is how Bridge/Premiere/Resolve all work
(Premiere even ships the placeholder state as "Media Pending").

- **A1 — kill the paint-path blockers** (pure enforcement, no new UX):
  one shared `Index` handle held on a background index thread (open once;
  UI sends queries over a channel, drains results non-blocking — the
  `author.rs` incident comment becomes law); `count_files`/`list_packs`/
  `scan_inbox` results computed off-thread once and cached in the existing
  §2a panel caches; `roots()` probed inside the worker as its own header
  demands; `read_libraries()` read once + on change signal, not per frame;
  per-card stat removed (existence resolved once per snapshot, not per
  frame); `clip_duration` never called from the paint path (record carries
  duration; probe happens off-thread at import/index time, which
  `import.rs`/`packs.rs` already do); Explorer-drop imports routed through
  the existing `start_ingest` job shape instead of the synchronous loop.
- **A2 — streamed loading + virtualized grids**: all media grids move to
  `ScrollArea::show_rows` (uniform cells, computed cols — the price every
  mature browser pays); directory listings stream in batches (flush every
  N entries or 50 ms); cells are `NotRequested | Queued | Ready(tex) |
  Failed` and paint skeleton-then-swap; **only painted cells request
  thumbnails** (visible-first priority falls out of immediate mode for
  free); a bounded worker pool (cheap lane ≈ cores−2 for decode/stat,
  expensive lane 2-4 for ffmpeg children) with generation-stamped
  cancellation (navigate away ⇒ stale generation ⇒ jobs dropped); texture
  uploads budgeted ≤8/frame (the existing house budget) with an LRU cap on
  `TextureHandle`s so 10k-item folders survive; the egui_extras
  thread-per-URI loaders stop being the media path (kept for UI assets
  only).
- **A3 — the disk thumbnail cache + video thumbs land** (absorbs
  `wiredfront-library-and-player.md` part B): `poster_for(src, cache) ->
  PathBuf` keyed by `(path-hash, mtime, len)` under `.userdata/thumbs/`
  (the predecessor plan's ratified location), JPEG/WebP tiers 256 +
  512, atomic temp-file+rename writes, and a **fail cache** so a broken
  file is never re-probed every open (freedesktop-spec semantics). Video
  thumbs via ffmpeg **seek-before-input** single-frame grab
  (`-ss <10% dur> -i … -frames:v 1 -vf scale=256:-2`, ~30-150 ms/clip,
  ~20x faster than decode-through) — Browse finally shows video
  thumbnails. ffprobe-style duration lands on cards as a badge before the
  thumb arrives. **The consent prompt from the predecessor plan ships
  here unchanged**: adding a folder asks to cache, shows the count,
  progress line, decline ⇒ badges + re-offer.
- **A4 — the layman experience layer**: skeleton cells (never
  spinner-storms), one calm progress pill ("Making previews… 132/580"),
  folder tree + breadcrumbs (children lazy-loaded on expand), filter chips
  `[All][Video][Images][Audio]` + instant in-memory search, duration badge
  on every video cell, hover-scrub via a 10-frame ffmpeg tile strip (zero
  decode on hover; Resolve's gesture), drag-to-timeline with ghost thumb +
  drop-zone highlight, tooltips on every control per the tooltip law.
  Failure cells stay calm: type glyph + name, never an error storm.

### Arc B — GIMP joins the bench (ADR-009 subprocess)

- **Discovery**: bundled portable tree → `C:\Program Files\GIMP 3\bin\`
  glob `gimp-console-3.*.exe` (the exe name moves per minor version) →
  registry. MS Store build refused (ACL'd paths).
- **Profile sandbox**: `GIMP3_DIRECTORY` pointed at an app-managed profile
  dir, seeded with PhotoGIMP's config payload. The user's own
  `%APPDATA%\GIMP` is never touched; PhotoGIMP (GPLv3 config files) may be
  redistributed with attribution.
- **Headless jobs** (`dpl-gimp` façade, shape copied from the ffmpeg/
  aerender seams): render a job `.scm` from a template into temp, spawn
  `gimp-console -i -d -c --batch-interpreter=plug-in-script-fu-eval -b
  "(load …)" -b "(gimp-quit 0)"`, success = exit 0 AND a `DPL-GIMP-OK`
  sentinel on stdout AND the output file exists non-empty — the house
  drill-verdict-line pattern. Script-Fu in GIMP 3 is PDB v3 (drawable
  vectors, resource objects). Cold start 3-10 s (use `-d`, `-f` where no
  text) — batch material, never per-frame. Optional resident mode later:
  the Script-Fu server survives in GIMP 3 (TCP 127.0.0.1, 3-byte-header
  protocol, unauthenticated ⇒ loopback only), amortizing startup for burst
  workloads.
- **Interactive "Edit in GIMP"**: copy the asset to a job path (sources
  never mutated), spawn the GUI exe WITHOUT `-n` (single-instance via
  WM_COPYDATA — the child exits immediately, so never wait on it), watch
  the job file (mtime changed AND size stable across 2 polls ≈ 500 ms ⇒
  saved), reimport + toast, keep watching; explicit "Done" affordance. UX
  copy must teach the one trap: **Ctrl+S in GIMP saves .xcf; File →
  Overwrite/Export writes the image back** (PhotoGIMP's Adobe shortcuts
  soften this).
- Images GIMP produces/edits flow through the one import seam into the
  WiredFRONT library like any other media — and thereafter through Arc A's
  browser like any other card. Operator-parity law holds: GIMP is an
  operator tool first; any AI-driven use comes later and only mirrors what
  the operator can do.

### Arc C — the Adobe exit (operator-run; tooling assists)

Full playbook with commands: `docs/runbook/adobe-exit.md`. Program-level
facts only: `.aep`/`.ffx`/`.mogrt`-inner projects are **dead formats** the
day the subscription ends — they must be rendered/exported now; PSD/AI
merely degrade (GIMP/Inkscape keep reading them). The bulk path
(.mogrt unzip ×2 → ExtendScript queue-all-comps → `aerender` chews the
queue → ProRes 4444 + alpha, H.264 proxy, PNG sequences for overlays) is
**machine-hours and fully automatable** — matches the standing
render-don't-parse ruling. Lottie via Bodymovin is the selective path
(GUI-only, no headless; expressions must be **baked** — ThorVG's
expression support is weak) reserved for templates worth re-animating
inside WiredFRONT. Two P0 items sit on a hard clock: cloud files
(30-day evacuation window at cancellation) and the **CC fonts audit**
(fonts deactivate ⇒ text layouts silently break).

## 4. Changes, in execution order

Each numbered block is one pass (one branch, one PR, per the one-target
law). A1 is execution-ready; later passes get their own ground-truthing
against HEAD at pass time.

**Pass A1 — enforce the doctrine (no new UX, measured delta)**

1. `crates/index/src/lib.rs` — an `IndexHandle`: one background thread owns
   the open `Index` (runtime + connection built once); UI-side handle sends
   query requests over a channel, results land in the existing panel-cache
   `Option<T>`s via non-blocking drain. All 8 `open_default` call sites
   (`composer/lib.rs:1763,1776,1835,1842`;
   `hubs/wiredfront/lib.rs:87,167,607,1125`) convert.
2. `crates/composer/src/lib.rs` `ensure_project()` — the fs sweeps
   (`scan_inbox_for` ×2, `count_files`, `list_content_episodes`,
   `list_packs`) move onto one background "project census" job (Fabric job
   + progress atomic, `start_ingest` shape); first paint renders from
   empty/last-known caches.
3. `crates/hubs/wiredfront/src/lib.rs:715` — the per-frame `count_files`
   dies; the census result is cached in `LibraryView` and invalidated by
   the existing `CaptureEvent::InboxChanged` seam.
4. `crates/composer/src/browse.rs:114` — `Browser::new` stops calling
   `roots()`; roots resolve inside the existing warm/scan worker and
   stream in (the module's own header law). `read_libraries()` at `:454`
   becomes a cached snapshot invalidated on library add/remove.
5. `thumb_uri` per-frame stats (`hubs/wiredfront/lib.rs:476,791,1176`) —
   existence resolved once when the view snapshot builds, stored on the row.
6. Explorer-drop / `import_dir` (`composer/lib.rs:2162,2573`) — route
   through `start_ingest`'s job thread; the UI shows the existing progress
   affordance.
7. `crates/video/src/clip.rs` — `ClipPlayer::open*` takes duration from the
   `Record` when present instead of probing; probe stays only on the
   off-thread import/index paths. Paint-path callers
   (`lib.rs:817,1262`, `wiredfront/lib.rs:965`, `player.rs:87`) pass it.
8. Proof: measured first-paint numbers (§6) + screenshots at real sizes.

**Pass A2 — virtualized, streamed grids** (Library, Gallery, Browse,
FOOTAGE list; worker pool + generation cancel + texture budget/LRU).

**Pass A3 — thumbnail cache + video thumbs + consent prompt** (absorbs
predecessor part B; `poster_for` keyed cache, fail cache, seek-grab).

**Pass A4 — experience layer** (tree/breadcrumbs/chips/badges/hover-scrub/
drag affordances; gated by the experience inventory).

**Pass B1 — dpl-gimp façade + headless jobs** (discovery, sandbox profile,
job runner + sentinel verdicts; architecture.md + xtask audit table amended
in the same commit for the new crate).

**Pass B2 — "Edit in GIMP" interactive loop** (copy-out, watch-for-save,
reimport toast, Done affordance, UX copy).

**Arc C** runs as operator sessions against the runbook, not as app passes;
the only app-side work is ingesting its outputs (already covered by the
import seam) and, later, optional comp-enumeration tooling.

## 5. Boundaries

**🚫 Never touch**
- `/content` — operator data. Thumbnails go under `.userdata/thumbs/`,
  never beside sources. Sources are never mutated (GIMP edits work on
  copies).
- The one import seam — Arc A/B additions route through it, no second door.
- `write_poster`'s ffmpeg args — working poster path.
- Consent: bulk thumbnailing stays ask-first. Making it automatic is an
  operator decision, not an optimization.
- No `#[allow]`; no gimp/image-magick crate deps (ADR-009); no re-export
  shims.

**⚠️ Ask first (PROPOSED, not ratified — per the clay-is-a-button law)**
- The §7 decisions below.

**Out of scope**
- Audio playback (still open from the predecessor plan §7.1).
- Lottie per-frame render blocking (S11) — real, but it is the L4/preview
  arc's problem; parked in STATE, not this program.
- AI-driven GIMP operations (operator-parity: editor first).

## 6. Verification

Operator judges by the number (prove-it-with-the-number law):

- **A1 delta**: time from route-enter to first presented frame on
  `wiredfront/editor` and `wiredfront/library`, cold exe, with (a) a
  pinned-but-unplugged USB source and (b) a 1000+-file elements tree —
  before vs after, measured via the existing frame-instrument or a
  `--ledger` run. Target: first paint < 100 ms in both; today it is
  seconds-to-hang.
- **A2**: Library with 500+ records scrolls at full frame rate; frame 1
  spawns bounded workers (thread count observed, not 1000); navigating
  away cancels (job count drops).
- **A3**: add a folder of ~50 videos → consent prompt states the count →
  accept → browsing stays responsive while thumbs fill → **second open is
  instant** → touch one file's mtime → only that one regenerates → decline
  ⇒ badges, nothing written, offer returns. Break-it: pull the drive
  mid-fill → honest message, no hang.
- **B1**: headless job on a real PSD → PNG lands, sentinel verdict line
  captured; kill-orphans sweep proven (same law as the EguiPilotAI CLI).
- **B2**: edit-in-GIMP round trip on a copy; original untouched
  (hash-compared); toast on save-detect.
- Every pass: `cargo green` exits 0 (cold clippy on touched crates — the
  warm-gate law), screenshots at real window sizes, double-green before
  merge.

## 7. Open — operator decisions needed (all PROPOSED)

1. **GIMP pairing**: bundle **GIMP 3.0.8 + PhotoGIMP** (known-good pairing;
   PhotoGIMP is BROKEN on 3.2.x as of 2026-08 — wrong config dir and
   partial even when copied, upstream issues #194/#200/#216) vs **3.2.4
   vanilla** (better PSD fidelity, no PhotoGIMP yet). Recommendation:
   **3.0.8 + PhotoGIMP now** — the operator named PhotoGIMP explicitly;
   revisit when upstream ships 3.2 support.
2. **Acquire strategy**: detect an existing install / download the official
   installer at first use (no GPL distribution duties) vs bundling the
   ~1 GB portable tree beside the exe (adds GPLv3 source-offer duty for
   GIMP itself — satisfiable, but a real obligation). Recommendation:
   **detect-or-download first**, bundling later if the experience demands
   it.
3. **New crate `dpl-gimp`** (façade beside engine-media) — needs the
   architecture.md new-crate checklist + xtask audit table amendment in
   the same commit. Recommendation: yes, mirroring the lottie-render
   helper's seam.
4. **Thumb cache location**: predecessor plan ruled `.userdata/thumbs/`;
   `.userdata/cache/thumbs/` would sit under the declared-regenerable
   cache root. Recommendation: **`.userdata/cache/thumbs/`** — thumbnails
   are regenerable by definition — but the predecessor text wins unless
   re-ruled.
5. **Arc C start**: the aerender bulk render is machine-hours that can
   begin immediately, independent of app passes. Needs the operator to
   run the one-time AE template setup (two output modules) per the
   runbook, then the loop grinds unattended.

## 8. Evidence

- **Grounding audit** (in-session, 2026-08-06, HEAD `fe0e162`, byte-matched
  against the lane): the S1-S13 ranked suspects with file:line for every
  claim — §2 above carries the top of it; the full audit came from a
  read-only sweep of `crates/composer/src/{lib,browse,packs,import}.rs`,
  `crates/hubs/wiredfront/src/lib.rs`, `crates/index/src/lib.rs`,
  `crates/video/src/{lib,clip}.rs`, `crates/fabric/src/lib.rs`,
  egui/egui_extras 0.35 registry sources. Prior-incident corroboration:
  `crates/hubs/workbench/src/author.rs:1055-1061`.
- **Pattern research** (web, 2026-08-06): freedesktop thumbnail spec
  (specifications.freedesktop.org/thumbnail — keying, mtime invalidation,
  fail dir, atomic rename); ffmpeg seek-before-input ~20x benchmark
  (sebi.io 2024-12-21); egui thread-safe `request_repaint`
  (docs.rs/egui Context; discussions #1748, #797; loaders PR #5394);
  `show_rows` virtualization (egui discussion #228); turbojpeg DCT-scaled
  decode; Adobe Bridge/Premiere media-cache docs (helpx.adobe.com);
  Resolve hover-scrub (Resolve 18 manual part474); Nautilus visible-first
  priority queue (and its no-repriority flaw — ours re-prioritizes for
  free via immediate mode).
- **GIMP research** (web, 2026-08-06): gimp-console batch + Script-Fu v3
  (developer.gimp.org script-fu-changes-v3, docs.gimp.org 3.0
  gimp-fire-up); Script-Fu server alive in GIMP 3 (docs.gimp.org
  gimp-filters-script-fu); `GIMP3_DIRECTORY` (developer.gimp.org
  libgimp func.directory); single-instance WM_COPYDATA (gimp man page);
  PhotoGIMP README + issues #150/#194/#200/#216 (github.com/Diolinux);
  GPL FAQ #MereAggregation/#GPLInProprietarySystem (gnu.org) — subprocess
  = zero GPL exposure for the app.
- **Adobe-exit research** (web, 2026-08-06): sources inline in
  `docs/runbook/adobe-exit.md`.
