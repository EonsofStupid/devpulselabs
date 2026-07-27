# STATE — the living ledger

Updated every pass. This file is the first read of every session.

**2026-07-27 — GROK SYSTEM-BROWSER AUTH LANDED (#67).** Device URL opens in
the OS default browser (not CDP/bundled Chromium); Cloudflare automation
reject fixed. Proof `proof-chat-grok-system-browser-auth.png`. Real-account
retest remains the operator gate. Contract: `docs/plans/chat-grok-activation.md`.
Open product PRs: none.

**2026-07-27 — W10a SLIDE ASSOCIATION LANDED (#72).** `dpl_doc::place_slide`
(copy into existing episode chapter; never-clobber; transactional rollback);
`BuildEvent` fabric domain + seam `build_json`; thin Workbench Studio place
form. EguiPilot `workbench-studio` +
`docs/proof/workbench-studio-place.png`. Plan
`docs/plans/w10-slide-association.md` §8a/§8.

**2026-07-26 — WORKBENCH CATALOG FIDELITY LANDED (#70).** Experience
inventory authored; Library gains Blocks chip (live eyebrow/badge/stat/
ambient); Templates + Seeds sectioned by class with honest empties;
EguiPilot fixture fixed to `workbench/library`. Plan
`docs/plans/workbench-catalog-fidelity.md` §8 +
`docs/plans/experience-inventory.md`.

**2026-07-26 — ESTATE→SEAM WIRE LANDED (#68).** Ring-0 estate at
`userdata_root()/estate`; seam seeds/persists `shell/route`; WiredFRONT
dual-writes `lastComp`; `GET /state` exposes `estate` + `lastComp`; cold boot
restores hydrated route when no `--route`. Plan
`docs/plans/estate-seam-wire.md` §8 — EguiPilot + restart drill PASS.
Preceded by **estate crate (#65)**.

**2026-07-26 — ESTATE CRATE LANDED (#65).** `dpl-estate` — Fjall LSM KV +
Moka read cache; partitions Docs/State/Manifest/Agents. Design ADR
`docs/design/estate-and-agent-seam.md`.

**2026-07-26 — W9c UNIFIED LANDING LANDED (#63).** Every PixVix vehicle
routes through `PendingLanding` + `land()` behind Auto/Ask; video/session
mint organized paths at start; LandingCard + policy strip (mode + bucket);
policy/card tags on index rows. Plan `docs/plans/w9c-unified-landing.md` §8.
Also merged same-day: **Grok signal-driven activation (#62)** (Codex).
**Hygiene:** pruned CLOSED stacked remotes (`feat/r1*`, chat-consolidation,
`design/coldlight-traits`, old estate park).

**2026-07-26 — WF EXTERNAL ASSETS LANDED (#60).** Reference-in-place imports
(`import_reference` single-file symlink + `⧉ copy` ⇄ `⌁ in place` toggle) and
cross-platform Browse roots + operator libraries (`★ Add/Remove library` →
`.userdata/libraries.json`). Plan `docs/plans/wf-external-assets.md` §8.

**2026-07-26 — W9a/W9b GUIDED LANDING (foundation + organized stills).**
**W9a** (#57): shared `dpl-landing` crate — `CaptureKind`, `LandingMode`
(Auto/Ask), pattern + policy JSON under `.userdata/`. **W9b** (#58): stills
land as `content/inbox/<bucket>/<kind>/<name>` (not flat `pix.png`). Plans:
`docs/plans/guided-landing-charter.md`, `w9-capture-landing.md`,
`w10-slide-association.md`. **W9c completes the program (#63).**

**2026-07-26 — RESPONSIVE RENDERING (RR) — RR0–RR2 LANDED.**
**RR0** (#54): the `fit()` law — no layout panic when floor > ceiling.
**RR1** (#55, `docs/plans/rr1-resolution-sweep.md`): `cargo xtask sweep`
— 6 routes × 7 sizes, isolated subprocess, software-GL; all 42 cells
no-panic on a real HDMI 1920×1080 (headless mutter @592×440 made `--window`
a NO-OP — display lesson). **RR2** (#56, `docs/plans/rr2-editor-responsive.md`):
editor uncramped below 1280px (fit panels, scroll tool strip / fx pane,
truncating NEXT hint). Proven @1024×640. DEFERRED: RR1.1 = offscreen render
+ `editor` eguiPilot fixture. **Runner note:** `dpl-linux` needs
`sudo ./svc.sh install` to persist across session teardown.

**2026-07-26 — GROK SYSTEM-BROWSER AUTH CORRECTION LOCALLY QUALIFIED:** the first
real-account attempt proved that visible bundled Chromium was still
automation-controlled through CDP and was rejected by xAI's Cloudflare
boundary. The corrected contract sends only the strict validated device URL
to the computer's default browser; bundled Chromium remains EguiPilotAI and
capture infrastructure. Four browser-boundary tests, 61 dependent
signal/UI tests, the full app check, all fourteen EguiPilotAI ceremony steps,
and native proof `proof-chat-grok-system-browser-auth.png` pass. The
real-account retest remains the operator gate. Canonical contract:
`docs/plans/chat-grok-activation.md`.

**2026-07-26 — GROK SEAMLESS ACTIVATION LOCALLY QUALIFIED
(`codex/grok-seamless-activation`):** the first real provider activation now
uses the exact bundled `grok login --device-auth` ceremony, a strict
`accounts.x.ai` URL/code parser, DevPULSE-owned visible Chromium, ACP
verification before Ready, and a non-secret local Turso evidence ledger.
The portable tool authority is consolidated under `dptools-deps/` with typed
`providers/`, `browsers/`, and `media/` families; Chromium is verified as a
complete tree. The authored eight-step EguiPilotAI flow and two byte-identical
real native-window captures pass. Personal xAI account completion remains the
explicit operator gate. Canonical contract:
`docs/plans/chat-grok-activation.md`.

**2026-07-25 — PRIVATE PORTABLE DEPENDENCIES LOCALLY QUALIFIED
(`codex/private-embedded-deps`):** `dpl-tools-deps` is now the sole
production authority for app-relative, manifest-declared, byte-length and
SHA-256 verified private tools. Codex, Grok, and ffmpeg no longer search
machine installs or `PATH`; Windows and Linux payloads stage through
`cargo xtask stage-tool-deps`. Real Windows and WSL Linux bundles passed
version probes and resolver verification. The real Chat peer passed, the
signed-out Grok surface exposes enabled provider-owned Login while Send stays
disabled, relocated release startup passed with provider-bearing `PATH`
entries removed, and the complete local `cargo green` gate passed. Canonical
contract and evidence: `docs/plans/private-tool-depot.md`.

**Tip:** on `main` (2026-07-27) at Grok system-browser auth (#67). **DG1 LANDED** (#43,
`feat/dg1-mixed-guide`, `docs/plans/dg1-mixed-media-guide.md` — operator:
stills for where-to-click + session-replay clips when appropriate, ORDERED
in one `.guide`): replay-segment steps land — `Step.kind:"replay"` +
`clipIn/clipOut/clip` (recorder guides unchanged); export cuts segments
from the ORIGINAL session via `engine-media::render_clip_cut`
(replay-light law) into `step-00N.mp4` interleaved in place;
`guide.html`'s stage swaps still↔clip in one ordered sequence (embed
default; jailed sidecar mode); `guide_video_args` evolved to ordered
`VideoLeg`s; the reviewer round-trip (engine-steps::edit +
composer::review_edit) carries clip identity so caption edits never
drop media. PROVEN live: real ffmpeg cut + REAL headless Chrome walking
still → clip → still via dpl-engine-browser. **RING-1 LANDED** (#42, `feat/three-ring-root`,
`docs/plans/three-ring-root.md`, ADR-014 — operator-ratified after the
capture-location failure, senior conditions 1–6 as acceptance): the
THREE-RING data root — `content\` (CONFIRMED library, unchanged) ·
`assets\` (WORKING: auto-filed `assets\captures\<kind>\<job>\`) ·
`.userdata\` (STATE: `index.db` Turso, cache/temp/profiles + deletability
README). Index moved ADDITIVELY (`content\artifacts.db` → copy → verify →
labeled `.migrated-to-userdata`, `Index::open_adopting`); promotion =
`composer::import::promote_asset` (transactional: import commits, then
the assets original — and ONLY an assets original — is removed);
browser captures auto-file (`browser_cli shot <url> --job <job>`),
ephemeral profiles in `.userdata\temp\browser\`, named profiles on the
`.userdata\profiles\` shelf; boot `ensure_rings()` logs all three.
WiredFRONT = PREP house over assets; Workbench = BUILD house over
content. DuckDB analytics = RING-2 (own C-dep ADR, never smuggled).
UI surfacing (assets shelf, promote chips) = its own experience-gated
pass. **DEMO-B landed**
(`feat/demo-browser-driver`, `docs/plans/demo-browser-driver.md`,
ADR-013): `dpl-engine-browser` — headless Chromium as a SUBPROCESS over
CDP. Resolve-never-download (injected → `DPL_BROWSER` → bundled →
Chrome installs → Edge, the Windows zero-install fallback → PATH incl.
the Linux names); `--headless=new` + ephemeral profile + port from
`DevToolsActivePort`; navigate/evaluate/click/insert-text/screenshot;
`browser_cli shot <url> <png> [--window WxH] [--profile <dir>]`; live
`#[ignore]` drill PROVEN on real Chrome (1.3 s end-to-end). New
workspace dep ratified: tungstenite 0.27 (no TLS — localhost only).
**DG-B.1 folded in same-day** (operator: self-contained/portable, NO
extension): persistent named demo profiles (never deleted, never the
operator's real profile) + headful mode + `browser_cli login <dir>` —
the one-time visible sign-in ceremony; captures then ride the saved
session headlessly. First capture round banked: 3 public Twilio stills
(headless, 1366×768) + ordered manifest in the session scratchpad;
learning: Twilio login is EMAIL-FIRST (two-beat sign-in step). Paper trail filed: the
demo-pipeline charter (`docs/plans/demo-guide-pipeline.md`, DRAFT —
DG1/DG2 ordering awaits ratification) + the first storyboard
(`docs/plans/demos/demo-001-twilio-notification-keys.md`). Demo
identity: clyffy@angrygaming.org (operator-established; credentials
stay operator-side — the agent never enters passwords; captures ride a
signed-in demo profile). NOTE: `feat/lottie-1-spike` work is
uncommitted; its Cargo lines were parked and restored during this pass.

2026-07-24 (late), THE COURSE CORRECTION SESSION: the
operator ruled the R1 sequencing a failure (plumbing landed while the
editor stayed a chip list, and R1u's editor spine was GUIDE-maker DNA —
Capture/Annotate/Edit/Narrate/Preview/Export + an Export .guide chip).
The fix, ratified via a mock-first loop: `docs/design/mocks/
wiredfront-editor-mock.html` (v2.1) is THE editor spec — **After Effects,
panel for panel** — and `docs/plans/wiredfront-experience.md` is the UX
GATE: every editor pass must move a row of its §5 friction audit.
**W1 landed** (PR #28): the editor rebuilt to the mock. **W2 landed**
(`feat/r1y-media-windows`, `docs/plans/w2-guided-stitch.md`): R1y's
media windows (Browse peer viewports onto local folders + Explorer
drag-drop, both through ONE import seam — copy to inbox/elements,
poster+seconds, index write-through, sources never touched) PLUS the
guided stitch workflow (NEXT-step hint strip · seeded comp names on
Save · renders indexed with posters · KEEP chips file the render into
any show from the editor) and the GUIDE tab as the tailored 1-2-3-4
export flow (seeded name chips, honest player-pass note).
**RATIFIED 2026-07-24 (operator): `.aep` + `.lottie` are first-class
imports** (personal-use tool) — they enter as ELEMENT artifacts
(`content/elements/imported/`, indexed); PARSING into comp layers is
its own spike, never faked. New harness seam: `--import-drill <dir>` +
the ignored FS drill `composer/tests/import_drill.rs`.
**W3 landed** (`feat/w3-pack-shelf`, `docs/plans/w3-pack-shelf.md`):
the PACK SHELF — the operator's themeforest stores pinned IN PLACE
(census: video = 70,018 files/79.6 GB incl. 9,021 .mogrt + 5,392 .aep
+ 860 .cube; audio = 250/2.6 GB; **C: had 14 GB free → the copy-all
ratification is SUPERSEDED by arithmetic; pin-in-place + copy-on-use
is law**). `elements/sources.json` (app-written registry) + ELEMENTS —
your packs in the Project panel (real `videohive-*`/`audiojungle-*`
rows, capped counts) → click opens a Browse window on the pack →
click a file copies JUST IT through the import seam. Kind matrix grew:
audio (wav/mp3/ogg/flac/m4a/aac → inbox) + mogrt/cube/ffx/prfpset
elements. Pin pack chip in Browse windows; `--pin-pack <dir>` harness
seam; D: trees are READ-ONLY (mtime-asserted). ALSO: the 51 GB
`target\debug` tree was deleted mid-pass when C: ran out of disk.
**W4 landed** (`feat/w4-pack-ingest`, `docs/plans/w4-pack-ingest.md`):
BOTH modes, as the operator asked — pins stay the in-place reference
(the app-level symlink) AND a per-pack **Ingest** grabs favorites
local: two-tap armed chip in the pack's Browse window (arm shows the
real `N files · X.XX GB` before anything moves), one background ingest
at a time with a live progress pill, copies land in
`content/elements/<pack>/` (indexed + postered, `source="ingest"`),
`.part`-then-rename (no torn files), SKIP-IF-EXISTS (resumable),
disk-full aborts honestly; ingested packs read **local** on the shelf.
Sources on D: stay read-only, always. Literal NTFS symlinks under
/content were REJECTED (they'd drag 70k files into rescans and blur
the artifact jail).
**W5 landed** (`feat/w5-guide-export`, `docs/plans/w5-guide-export.md`
— the execution plan superseding the proposal): **the real .guide
export**. RATIFIED (operator, this session): deliverable = BOTH the
self-contained `guide.html` AND a single `guide-video.mp4`; replay
EMBEDDED in the html by default with a sidecar toggle. NEW crate
`dpl-guide` (manifest round-tripping the recorder's guide.json ·
hand-rolled base64, zero new deps · `write_html` — stills as data URIs,
swipe/arrows, do/done toggle, recorded-click marker dot, replay with
tappable event chapters, ZERO external refs · `export_guide` — copy +
renumber kept stills, light replay re-encode, forward-contract folder
the native player will zip). engine-media grew `render_replay`
(≤720p/15fps/crf30 — 88→44 KB on the drill) + `render_guide_video`
(stills held 3s then the replay, one mp4). The Guide tab is the REAL
flow now: pick (guides + sessions from the index) → REVIEW & TUNE (the
W5 reviewer `engine-steps::edit` was waiting for: thumbs, Keep/Skip,
reorder, captions, two-tap Apply that prunes source frames) → name →
export (embed + video toggles, off-thread) → keep chips. ONE path
`dpl_composer::run_export` (jail, no-clobber mint, index write-through
kind "guide" source "export") shared by the tab and the new
`--export-guide <guide-rel> [<session-rel>]` headless seam. Proof: §8
of the plan (real-exe drill: 4 embedded stills + embedded mp4 + 0 http
refs, guide-video exactly 15.000s; shots
docs/proof/w5-guide-tab-{1100x700,1600x1000}.png). Proof-only env
`DPL_GUIDE_PICK` opens the reviewer for screenshots.
**W5.3 landed** (same branch, operator mid-session ask "full test by
you ... exported as gif or whatever ... for linkedin"): the SOCIAL GIF
(`replay.gif`, palettegen two-pass, 480px/12fps, Guide-tab toggle;
LinkedIn-native = the bundled light session.mp4) + the AI-DRIVEN FULL
TEST seam `--steps-drill <n>` (REAL recorder start via the palette
handler with a drill-only self-capture opt-in, n real marked window
captures, real Save-card save). FULL TEST RAN, all real: 3 stills
captured by the drill, real 8s session (490 KB -> 44 KB light re-encode),
replay.gif 179 KB/79 frames, guide-video 15.6s, self-contained html.
FINDING FIXED: seeded guide names now clamp to 48 chars (a path-length
window title minted a folder that broke Win32 cwd limits).
**P1 landed** (same branch, `docs/plans/p1-data-root.md`): KNOWN #3
FIXED -- the portable user-data root. Operator-ratified ONE folder
(content/ stays the single tree); resolution law ported from the
original: `--content-root` arg -> CONTENT_ROOT env -> exe-adjacent ->
exe-parents walk-up -> cwd, first existing; nothing -> self-creates
BESIDE THE EXE; `checked_roots()` honesty logged at boot. PROOF: exe
run from target/release resolved the repo content root, no scatter.
Operator-approved migration: the 9 scattered captures moved into the
real inbox (2 renamed `-migrated-N`), stale tree removed. NOTE: one
Gallery `rescan` tap reindexes them.
**R1z landed** (`feat/r1z-editor-safety`,
`docs/plans/r1z-editor-safety.md`): NEVER LOSE WORK — friction rows 1+8
closed. Snapshot UNDO/REDO (comp is pure `Clone+PartialEq` data; cap
100, deduped, ONE step per completed drag; history per-comp) ·
debounced AUTOSAVE (2s after the last edit on a SAVED comp, crash-safe
`dpl_comp::save`, flush on New/Open + `Drop` at exit; facts line shows
`· unsaved edits` honestly; unsaved comps still wait for the guided
Save) · REOPEN-LAST (`composer-state.json` at the content root — the
`sources.json` precedent — written by Save/Open, restored once-lazily
only when the editor would open empty; drills/tests stay hermetic) ·
the KEYBOARD MAP, one pure unit-proven `key_act` path guarded by
Editor-tab + `egui_wants_keyboard_input`: Space play/pause · Del/
Backspace remove-selected · Home/End · ←/→ step 1s · Ctrl+Z /
Ctrl+Shift+Z / Ctrl+Y (operator ratified the arrows/Home/End extras).
BUG FIXED en passant: New was `*self = default()` — it severed the
in-flight render/ingest/export Arcs, closed Browse windows, and dumped
every cache; it is now a comp-only reset (`new_comp`). New harness seam
`--save-comp <name>` (Save-chip path headlessly). PROOF: 27 composer
tests; real-exe SEQUENTIAL drill — run 1 imports 2 clips + saves
`r1z-proof`, cold-boot runs 2+3 REOPEN it ("Reopened inbox/r1z-proof —
your last comp.", both layers on the timeline); shots
`docs/proof/r1z-reopen-{1280x800,1600x1000}.png`.
**R2a.1 landed** (`feat/r2a1-deep-pack-browse`,
`docs/plans/r2a1-deep-pack-browse.md`): deep-pack browse, the
D:-independent half of R2a — ground truth 2026-07-25: the
`D:\webwork\themeforest` store is OFFLINE (drive absent; G: is Google
Drive) and the real root has NO sources.json — the operator never
pinned the real stores. Shipped: ① OFFLINE-PACK HONESTY — an
unreadable pinned source lists as one `offline` shelf row (tooltip:
plug it back in; ingested copies stay local) instead of silently
vanishing (the old `continue` bug, exposed by the missing drive
itself); ② Browse-window SEARCH — type-to-filter the open folder live
+ Enter runs a recursive name search under it OFF-thread (cap 500,
said in the UI), results in the same click-to-import cards, nav/x
clears; ③ census fix — `sources.json` no longer counts as an element
(phantom "1 imported element" since W3). PROOF: 32 composer tests;
pin→unplug→boot drill shots
`docs/proof/r2a1-offline-pack-{1280x800,1600x1000}.png`.
**R2a.2 landed** (`feat/r2a2-mogrt-reader`,
`docs/plans/r2a2-mogrt-reader.md`): the drive RETURNED mid-session and
the REAL stores are PINNED in the real root. SPIKE VERDICTS (real
specimens, 2 unrelated packs): `.mogrt` = plain zip
(definition.json + project.aegraphic + thumb.png; DEFLATE) — the
metadata layer of the ratified preset-editing is READABLE without AE;
`.aep` = RIFX `Egg!` opaque — **the parsing question stays OPEN by
operator ruling ("we will keep it open as I research")**, files remain
elements; `.cube` = trivial text (R2b confirmed cheap); the stores
carry Mac `._*` AppleDouble twins (dot-skip already excludes).
RATIFIED: dep = the FULL `zip` crate ("I will need it eventually") —
workspace `zip = "8"`, deflate-only. SHIPPED: `composer/src/mogrt.rs`
reader (capsuleName/capsuleTags/thumb.png, stem fallback, honest
errors); import + ingest ENRICH mogrt element records (real name, tags,
extracted `-poster.png`); Browse cards paint REAL mogrt previews via a
budgeted lazy cache (≤8 zip reads/frame, `bytes://` egui uris); the
import status says what was actually read. `clientControls` (editable
text/font/size decls) documented, read when the controls UI pass
consumes it. PROOF: 36 tests; real `Title 01.mogrt` through the exe →
sqlite-verified enriched row + byte-identical extracted poster; shots
`docs/proof/r2a2-mogrt-import-{1280x800,1600x1000}.png`.
**W6 landed** (`feat/w6-frictionless-media-in`,
`docs/plans/w6-frictionless-media-in.md`) — operator redlines killed:
(a) the LIBRARY is the real thumbnailed MEDIA SHELF — double-click any
clip/image and the Editor takes it (video → timeline via the one
add_clip path, undo/autosave riding; image → FOOTAGE honestly until
image layers) and the app navigates there; one outside seam
`Composer::add_from_library`. (b) MANAGED FFMPEG at boot (the original
tools.rs law): bin/ beside exe → process PATH → winget Links → the
winget Packages scan (FINDING: Gyan.FFmpeg never gets a Links shim),
injected into all three engine seams; none found warns naming every
location. PROOF: PATH=System32-only export drill runs end to end; the
drill also proves P1's `--content-root` arg live. Friction rows 13+14
DONE. Shots docs/proof/w6-library-{1100x700,1600x1000}.png.
**W7 landed** (`feat/w7-image-layers`,
`docs/plans/w7-image-layers.md`; operator: images are "kind of
essential for all of this to matter"): IMAGES ARE FIRST-CLASS TIMELINE
LAYERS. No schema change (a still is `in_pt=0, out_pt=5.0`, trimmable);
`StitchClip.still` + loop/hold ffmpeg legs (+ `format=yuv420p` on every
leg — mixed concat law); every add path appends (FOOTAGE tap, Library
double-click, Explorer drop, import drill); the preview well shows the
held frame (no ffmpeg pipe for stills); the seam maps stills so agents
get image layers over HTTP free. PROOF: real-exe import of png+mp4 ->
both on the timeline, preview shows the still; seam render =
8.000000s exactly (5s still + 3s video) at 1920x1080. Friction row 15
DONE; the "image-layers pass" copy is GONE repo-wide. Shots
docs/proof/w7-image-timeline-{1100x700,1600x1000}.png.
**W8 landed** (`feat/w8-faithfulness`,
`docs/plans/w8-faithfulness.md` + THE LOCKED
`docs/design/signal-doctrine.md`; operator: "stop rushing … establish
clean patterns … my signalkind"): THREE HOUSE LAWS. (1) QUIET-SPAWN —
`dpl-proc::quiet` at every GUI-reachable subprocess (video pipes,
engine factories, chat providers' collapsed duplicates, browser,
lottie) + a cargo-green AUDIT: an unflagged `Command::new` fails the
gate. (2) PROACTIVE-BUFFER (doctrine.md §perf) — Browse scans
off-thread, warm cache seeded at first visit, roots probed once off
the paint thread; the 20s freeze's both causes structurally gone.
(3) THE SIGNAL DOCTRINE (ratified: stamp-at-publish w/ ONE global seq
across domains, activity feed in W8, commands contract-only until
R2c) — fabric grows CAPTURE + JOBS domains + the WAKER; every
mutation signals through index+signal seams (viziwiz's one seam, the
composer, hubs, THE AGENT SEAM's render — a missed site caught live
by the silent SSE proof); views invalidate on signal (refresh chips
no longer load-bearing); session events.jsonl + seam `/signals` speak
ONE ENVELOPE `{domain,seq,ms,type,…}` pinned by test; the ACTIVITY
FEED lives in the inspector (proof shot: the import drill's "saved
image" line — TWO boot-ordering bugs fixed: wire-before-drills +
ears-at-fabric-birth). Proofs in plan §8 (SSE capture of a live
agent-render's stamped queued/failed; green ✔ incl. the audit).
KNOWN (parallel lane): embedded-deps staging unauthored on this
machine → renders fail app-wide until `sources.local.toml` is staged
(ffmpeg-only draft written, gitignored; stager wants codex+grok too).
**AEP ORCHESTRATOR — new pass, PAUSES R2b** (operator 2026-07-25:
installing AE/Animate; "pull aerender.exe into ours and have wiredfront
editor viziwiz actually be an orchestrator"). This RESOLVES the open
`.aep` thread the house way — never parse RIFX, ORCHESTRATE Adobe's own
headless `aerender.exe` as a subprocess (ADR-009, the ffmpeg pattern).
Planned via the NEW `/deep-plan` skill (Fable architects; one Opus
agent filled the AE-execution gap, spot-checked): full plan
`docs/plans/aep-orchestrator.md`. GROUND TRUTH (verified): aerender
26.3x87 is fully headless (~13s boot, parseable `PROGRESS:`/`INFO:`/
`ERROR:` stdout); comp enumeration needs `AfterFX.exe -r` which is
blocked by AE's Welcome/Home screen (`"Show Welcome Screen" = 01`, one
pref toggle); default OM templates incl. `H.264 - Match Render Settings
- {5,15,40} Mbps` (mp4) + Lossless; ffmpeg 8.1.2 has libwebp_anim/
libaom-av1/svtav1/gif for the export selector. RATIFIED decisions: full
comp picker in v1; app toggles the Welcome pref on the operator's tap;
export selector (mp4/webp/avif/av1/gif); DYNAMIC perf tuning (16GB
laptop is the operator's WEAKEST — 32/64/128GB rigs coming for content/
streaming); dep `sysinfo` for the RAM probe.
**Phase 1 LANDED** (`feat/aep-1-engine`): NEW crate `dpl-engine-ae`
(the ADR-009 sibling of ffmpeg) — `locate` (AE 2026 found), `Caps`
probe + `RenderTuning::for_caps` dynamic policy (MFR OFF/RAM-conservative
@16GB, ON @128GB; Balanced/MaxThroughput/Gentle profiles), RIFX gate,
aerender arg-VECTOR builder (spaced template names survive — the
quoting gotcha), stdout grammar parser, safe Welcome-pref toggle
(AE-closed guard + `.dpl-bak` backup, ending-preserving). 10 tests; a
`--example probe` per-rig diagnostic. NEXT PHASES: 2 enumeration
(first real-AE run, after the pref toggle) → 3 render+timeline ingest →
4 ffmpeg export selector → 5 automation tap + tuning UI.
**THEN R2b** (resumes after the orchestrator): the 860 `.cube` LUTs via
ffmpeg lut3d (gate row 7), then R2c chat-edits-comp. Ratified:
3D/camera/tracker/motion-blur LATER; AI ephemeral in-app + chat (one
seam); WiredFRONT before Workbench.
Memory: `experience-first-gate`, `wiredfront-orchestrates-adobe`.
KNOWN (pre-existing, observed by the W2 cross-process drill): opening
`artifacts.db` with a WAL left by ANOTHER process trips the index's
quarantine (`.corrupt`); in-app (same-process) flows unaffected.
NOTE: `seeds/slides/static/headline-clean/slide.json` carries an
operator edit (fonts→orbitron, moods) left uncommitted on purpose.

**Architecture (ratified):** three products, all Rust/egui —
`docs/design/viziwiz-architecture-spec.md` + the rewritten
`docs/plans/port-viziwiz.md`: WiredFRONT capture / Workbench authoring /
Native Guide Player (winit+wgpu+libmpv; `.guide` ZIP + portable `.exe`).

**Chat is DONE as a peer window** (`docs/plans/chat-consolidation.md`): a
status-bar-triggered, Steam-overlay-like native peer window — NOT a hub route.
It carries the read-only session histories + trait provider chips, and a lite
**route launcher** that publishes `NavigateRequest` so the MAIN window
navigates (chat launches routes, never embeds them). The rail-hub `dpl-hub-chat`
is DELETED. Memory: `chat-is-a-peer-window`.

## Proven working (engine layer)

- Programs L (IA/accordion rail), D (deck/present/presenter/scripts),
  X (index/gallery/keep), C (capture parity: stills C1, steps C2,
  video C3, ViziWiz C4) — each with plan doc §8 evidence + exe drills.
- OS-global hotkeys Ctrl+Shift+S/9/X (rdev); Steps Save card; video
  format/audio matrix; ink flatten (source never mutated).
- EguiPilotAI client seat works: doctor green, live pixvix snapshot.
- **WiredFRONT Composer (R1a)** — the "After Effects, chat-route-aware"
  video editor's FOUNDATION landed (branch `feat/r1a-comp-document`, off
  `refactor/chat-consolidation`): `dpl-comp` (the `comp.json` composition
  document + doctor — tracks→layers→sources, refs-never-values, crash-safe
  save, `validate → Vec<Issue>`, folder contract; mirrors `dpl-doc`; 8
  tests) + `dpl-composer` (four-pane shell — Palette · Timeline clip-list ·
  Inspector · single-source Preview via the S2 ClipPlayer; ColdLight paint,
  cl::tip everywhere; 4 tests) mounted as the `wiredfront/composer`
  subroute. `cargo green ✔`; proof
  docs/proof/r1a-composer-{1100x700,maximized}.png. ADR
  `r1-editor-adr-proposal.md` — four forks RATIFIED 2026-07-24: hybrid
  gen backend (local ComfyUI + hosted media APIs behind one `GenBackend`
  trait, chat mesh orchestrates), local ort+BiRefNet matting, copy-all
  elements into `content/elements/`. Plan `docs/plans/r1a-comp-document.md`.
  R1c (elements ingest) → R1d (ViziWiz mode) → R1e (matting) → R1f (agent
  tools) remain. Memory: [[wiredfront-editor-is-r1]].
- **R1b.1 — the stitch RENDER landed** (branch `feat/r1b-stitch-render`,
  stacked on R1a): `engine-media::stitch_args`/`render_stitch` (decoupled,
  primitive inputs — no dpl-comp dep) build the ffmpeg CONCAT filtergraph
  (trim → fps-normalize → scale+pad → concat, `-an`, faststart, crf 21,
  ADR-009 subprocess) + a **Render** chip in the Composer that stitches the
  clip-list into `inbox/<comp>/render.mp4` OFF the UI thread (polled
  `RenderState`). `cargo green ✔`. END-TO-END PROOF: two mismatched clips
  (640×480@30 + 800×600@24) → one 1280×720@30 file of exactly 5.00s
  (docs/proof/r1b-stitch-render-demo.mp4). Plan
  `docs/plans/r1b-stitch-render.md`. R1b.2 (xfade + composited preview +
  egui_tiles docking) is the next rider; then R1c–R1f.
- **R1s — editor SHELL landed** (branch `feat/r1s-editor-shell`, stacked on
  R1b): the operator's real design reference (two images, 2026-07-24) is now
  captured in `docs/design/wiredfront-editor-inventory.md` — THE fidelity
  bar (the gate KNOWN BROKEN #5 said was never authored). The Composer was
  reshaped from a flat 4-pane skeleton to the **WiredFRONT editor spine**: a
  workflow-tab bar (Capture · Annotate · Edit · Narrate · Preview · Export +
  Export .guide, active glows voltage), a SCENES/ASSETS left rail, and the
  active mode's surface — EDIT hosts the working timeline editor, the other
  five tabs are honest styled stubs naming what each will do. Tofu-box
  glyphs (＋/↑/✕) replaced with font-safe labels. `cargo green ✔`; proof
  docs/proof/r1s-editor-shell.png. Pass order to full fidelity is in the
  inventory doc §"Pass order": R1-scene → R1-annotate → R1-steps →
  R1-narrate → R1-preview → R1-export, interleaved with R1c–R1f.
- **R1t — Preview + Annotate modes** (branch `feat/r1t-preview-annotate`,
  stacked on R1s): the ANNOTATE tab now shows the tool row
  (Select/Arrow/Callout/Box/Spotlight/Text) over a scene canvas; the
  PREVIEW tab shows a 16:9 device frame + Touch/Mouse/Keyboard input chips;
  both render the selected clip's frame via the ClipPlayer. A
  `DPL_COMPOSER_TAB=<mode>` env override opens a specific tab for
  screenshots (proof affordance, inert in normal use). Proof:
  docs/proof/r1t-{annotate,preview}.png. Still stubs: tool behaviour, the
  annotation data model, input-mode selection, sequence preview — the
  annotation/preview passes fill them. PR #24.
- **R1u — editor FIDELITY** (branch `feat/r1u-editor-fidelity`, stacked on
  R1t, PR #25): the operator's explicit layout ask landed — the transport
  toolbar (New/Save/Render + stats) spans the full width along the BOTTOM
  with the timeline directly above it; TOOL SELECTION is a real, persisted
  LEFT rail (Select/Arrow/Callout/Box/Spot/Text, Active state presses in)
  shared by Edit + Annotate; the inspector is adaptive (170–240) so
  1100x700 never starves the preview. The RAIL ACCORDION unfolds
  wiredFRONT → **Editor · Library** · Overview · ViziWiz · Gallery ·
  Sessions on a recessed plate (surface::BASE), rows indented 10px, hover
  wakes the ink (`composer` SubId renamed `editor`; no shim). NEW
  `wiredfront/library`: the generated INGREDIENT LIBRARY
  (`dpl_composer::library_ui`) — renders ONLY ColdLight's generated
  registries: TRAITS composed live via `compose()`, LOOK/MOTION presets,
  FONTS in their own faces, MOODS sampled via `Mood::at`. Never a one-off;
  the Nth ingredient = JSON + regenerate, zero UI code. Gate ✔ (fmt,
  clippy, all tests, release; ran in the pre-push hook). Proof:
  docs/proof/r1u-{editor-1280,editor-1100,library,rail-open}.png. Plan:
  docs/plans/r1u-editor-fidelity.md.
- **R1v — the AGENT SEAM landed** (branch `feat/r1v-agent-seam`, stacked on
  R1u): `dpl-seam` — axum on `127.0.0.1:7341` (own thread + runtime, egui
  never touched; `--seam-port <n>` / `--no-seam`): `GET /state` (last-heard
  route + the full hub/subroute registry built from the SAME HubSpecs the
  shell registers), `POST /command` (navigate / viziwiz-summon — unknown
  routes refused BY NAME, 422 + issues, doctor-style), `GET /signals` (every
  ShellEvent as SSE; wire shapes pinned by test, fabric stays serde-free).
  5 tests. LIVE PROOF: POST navigate → the real exe navigated (GET /state
  flipped to wiredfront/editor) and SSE captured navigate-request +
  route-changed (§8 of docs/plans/r1v-agent-seam.md). This is how chat
  drives the editors. Estate (Fjall+Moka) is pushed on
  `feat/estate-fjall-moka`, ready to join the seam for agent persistence.
- **R1w — CHAT RENDERS VIDEO over the seam** (branch
  `feat/r1w-comp-endpoints`, stacked on R1v): the comp-document surface —
  `GET /comps` (inbox census) · `GET /comp?dir=` (document + live doctor
  report) · `POST /comp` (DOCTOR-GATED write: invalid → 422 with every
  issue named, NOTHING touches disk; absent dir mints `inbox/comp-N`) ·
  `POST /command {"cmd":"render"}` + `GET /render?dir=` (the ffmpeg stitch
  off-thread, polled; sources never mutated, output is always the NEW
  render.mp4). Rel dirs JAILED inside the content root (`..`/absolute/
  drive-letter refused by name). 9 tests. LIVE PROOF (scratchpad
  CONTENT_ROOT): POST a 2-clip comp over HTTP → render → ffprobe
  `duration=5.000000` exactly, mismatched sources (640x480@30 + 800x600@24)
  normalized to 1280x720@30 — an agent authored a composition and got a
  rendered video via HTTP end to end (§8 of docs/plans/
  r1w-comp-endpoints.md). NEXT riders: R1c elements ingest → Library;
  annotation model for the left-rail tools; estate-backed agent state.

## KNOWN BROKEN / regressed (the honest list)

1. **Footer/status bar invisible unless maximized** — FIXED (this PR):
   continuous `fit_window` guard (the old repo's work-area law, ported) —
   any restored window exceeding the monitor is auto-shrunk (decoration
   delta honored) and nudged on-screen, every frame, so DPI changes and
   monitor drags self-correct. Proof: `--window 2000x820` on a smaller
   monitor auto-fits to 1522×778 with the footer visible
   (docs/proof/footer-2000x820-fit.png). `--window <w>x<h>` is now a
   standing proof flag — UI proofs must include non-maximized sizes.
   NOTE: egui exposes monitor size, not the work area — the guard
   reserves 48 logical px for the taskbar; if a taller taskbar clips,
   raise the allowance or port a Win32 work-area query.
2. **The experience layer was never specced.** NOW BEING FIXED:
   `docs/design/doctrine.md` DEFINES the language (BRUTAL = actionable,
   CLAY = passive, tooltip law, token law, any-shape law, the feel bar)
   and the `coldlight-system` skill (formerly `dpl-style`) gates every UI
   pass on it. ColdLight grew the primitives (`brutal_frame`/`clay_frame`/
   `tip`, `RADIUS_LG`).
   SUPERSEDED (2026-07-24): the thin BRUTAL/CLAY doctrine is being
   replaced by the composable TRAIT system faithfully ported from the
   original ColdLight (neubrutalism/claymorphism base + cyber/retro
   stack, per-axis compositor, generated manifest the Workbench editor
   reads). Plan: `docs/plans/pass0-coldlight-treatments.md`. Skills:
   `coldlight-system` (foundation gate) + `slide-catalog-pass` (per-slide
   faithful port). See KNOWN BROKEN #5 below.
   First application landed: the shared `filter_chip` is brutal (hard
   offset shadow, hover press) with layman tips on gallery filters,
   rescan, and ViziWiz Undo/Clear/Flatten.
   Sweep 1 DONE: pixvix palette — every chip brutal via the tip-mandatory
   `pilot_chip` (18 controls with layman tips: snap modes, delay, re-snip,
   video matrix, session, guide Save/Discard, REC pill Mark/Stop) + tab
   tips + refresh buttons. NOTE: the palette is its own OS viewport —
   visual proof awaits the operator's eyes / pilot capture (immediate-
   viewport capture unsupported); compile+green verified, tips visible in
   pilot snapshots.
   Sweep 2 DONE: shell chrome — footer launchers (Snip/Agents) brutal
   with layman tips; rail house tiles tip their hub name; unfolded
   subroute rows tip their blurbs (proof:
   docs/proof/doctrine-footer-launchers.png).
   Sweep 3 DONE: hub placeholder subroute cards → CLAY (passive planes;
   active keeps the tone scatter). FINDING for its own pass: these
   cards LOOK tappable but are inert — hubs have no navigation seam;
   needs the deferred fabric NAVIGATE event, then the cards go brutal
   and tap-to-navigate (frictionless law).
   Sweep 4 DONE: gallery + sessions — layman tips on the session thumb
   ("Play this session right here"), the two-tap Dump (armed state
   explains itself), Keep + every keep-target chip, the session list
   cards, the player's back/play controls, and the seek rail (names the
   orange event ticks).
   Sweep 5 DONE: workbench Library — section chips explain their walls
   in plain words, the solo view's back/replay tip, wall titles say
   "Tap to see this one big". Deck toolbar audited: already compliant
   (tool_button carries tips).
   Sweep 6 DONE: ViziWiz — tone dots name their color, width slider,
   back, and picker cards all tip in plain words.
   NAVIGATE seam DONE (plan docs/plans/navigate-seam.md): hub subroute
   cards are tap-to-navigate — `ShellEvent::NavigateRequest` published
   by chat/workbench/wiredfront (wire_fabric), drained by the shell;
   cards show the tone edge + "Go to <label>" tip on hover; the session
   EventLog records `navigate-request` honestly.
   **Remaining**: the feel-bar review pass across every route against
   the old app (screenshot-by-screenshot). Every UI pass MUST invoke
   `coldlight-system` (the gate, formerly `dpl-style`).

5. **The visual language was invented, not captured (operator, 2026-07-24).**
   The port replaced the original's real ColdLight design system — authored
   trait JSON (neumorphic/cyber/retro) composed per-axis, a manifest the
   editor reads, documented in the original's `src/lib/design/TRUTH.md` —
   with a hand-invented BRUTAL/CLAY doctrine + magic-number `*_frame`
   helpers. The `experience-inventory.md` gate that both CLAUDE.md and
   doctrine.md require WAS NEVER AUTHORED, so Sweeps 1–6 ran against the
   invention with no reference to the original's look. AND the Workbench
   Library holds only 4 authored seed slides (all `static`) of ~27 catalog
   types — the renderers are ~90% ported but the slides aren't organized.
   FIX (ratified 2026-07-24 "go"): `docs/plans/pass0-coldlight-treatments.md`.
   Foundation LANDED on main (`compose` / `cl::treated` / neubrutalist+
   claymorphic). PR #16 (`design/coldlight-traits`) was CLOSED without merge
   after the stack landed via later PRs — remote branch kept for triage of
   any leftover tip commits. REMAINING in this program: tints painting +
   cyber/retro anim (needs motion.json + painter), the generated manifest
   the Workbench editor reads, the experience-inventory doc, then the
   `slide-catalog-pass` loop (~27 slides). Skills: `coldlight-system` +
   `slide-catalog-pass`. Memory: `port-fidelity-standard`.

## CI + process (live as of github-repo-hardening)

- Self-hosted `dpl-win-laptop` (Windows) + `dpl-linux` / `dpl-linux-clyffy-lab`
  (ARM64 mini-PC) run the green gate. Branch protection is Pro-gated — CI
  is advisory; enforcement = local pre-push hook (`core.hooksPath=.githooks`,
  mode 100755) + discipline. NEVER squash-merge a code PR unless
  `green-linux` is green (Windows pending alone is not a fail).
- **Dependency graph + Dependabot:** vulnerability alerts + automated
  security fixes enabled; `.github/dependabot.yml` weekly Cargo + Actions
  version PRs. GraphQL proves Cargo manifests on the graph.
- **Nightly Releases:** `.github/workflows/nightly.yml` → rolling tag
  `nightly` (exe-only zips). Pull: `./scripts/pull-nightly.sh` /
  `.\scripts\pull-nightly.ps1`. Plan: `docs/plans/github-repo-hardening.md`.
- `delete_branch_on_merge=true` on the repo.
- Docs-only PRs skip CI (paths-ignore) and may merge immediately.
- Runner persistence: Windows → schtasks (CLAUDE.md); Linux →
  `sudo ./svc.sh install` (session `./run.sh` dies on logout).
3. **FIXED (P1, 2026-07-25)** — the portable user-data root law ported
   (docs/plans/p1-data-root.md): resolution is arg -> env -> exe-adjacent ->
   walk-up -> cwd; double-click finds the repo content root (proven);
   scattered target/release/content migrated + removed.
4. Stale EguiPilotAI scenarios/fixtures (pre-Program-L flyout/Library refs).

## Next

1. **W10b** — LandingCard Episode polish, live budget strip, activity-feed
   line, full chapters×slots compose wall (`w10-slide-association.md`).
2. Estate deeper wire (selection/undo, Workbench session, Agents tree).
3. Seed empty classes / animation templates (catalog follow-ons).
4. Parallel backlog: RR1.1; AEP Phase 2; ColdLight remainder; mac-parity.
   Ops: `dpl-linux` `svc.sh install`; unset Cursor `CARGO_TARGET_DIR` for
   local green (or fix `locate_asset`). Operator: real xAI account retest
   for Grok system-browser auth (#67).

## Process facts

- Work lands via PR. The operator's exe rebuild + this file's update are
  part of DONE. Pushes can be slow — background them; commit is what counts.
- Old app reference: `C:\localdev\devpulse labs` (READ-ONLY).
