# AEP Orchestrator — WiredFRONT drives aerender headless, renders land on the timeline

Status: PLANNED 2026-07-25 via the `/deep-plan` skill (Fable architects;
one Opus agent filled the AE-execution gap, findings spot-checked below).
Operator-directed: this PAUSES the R2b (LUT) sequencing — "pull
aerender.exe into ours and have wiredfront editor viziwiz actually be an
orchestrator." Resolves the OPEN `.aep` thread the house way: never
parse RIFX/`Egg!`, ORCHESTRATE Adobe's own headless renderer as a
subprocess (the ADR-009 pattern ffmpeg already follows).

## 1. Context

The operator installed a LICENSED After Effects 2026 (personal
quality-of-life tool, own project files). Their store holds 5,392 real
`.aep` projects that today enter the app as inert filed elements. This
pass makes them RENDERABLE: drop an `.aep` → pick a comp → AE bakes it
headless → the render becomes an ordinary artifact on the timeline,
and (operator-ratified) is exportable to mp4/webp/avif/av1/gif. Ratified
decisions (AskUserQuestion, 2026-07-25):

1. **Full comp picker in v1** — enumerate the project's comps and let the
   operator pick, not type blind. "Built to perform like a proper piece
   of software" → real memory/CPU tuning, not a naked spawn.
2. **The app toggles AE's Welcome-screen pref on the operator's tap** (a
   one-time config change, with an AE-closed guard + a backup).
3. **A real export selector** — mp4, webp, avif, av1, gif, and more.
4. **DYNAMIC performance tuning** (operator, mid-plan): "this is on
   purpose my weakest machine… i have a 32 and 64 and 128gb system…for
   content/streaming." The tuning MUST be a runtime capability probe, not
   constants — same binary, right behavior from 16 GB to 128 GB.

## 2. Ground truth (verified this session — provenance noted)

**aerender CLI (I ran `aerender -help`, version 26.3x87):** renders a
named comp (`-comp "Name"`, auto-added to the render queue if absent) or
the whole queue; `-project`, `-output`, `-s/-e/-i`, `-RStemplate`,
`-OMtemplate`, `-renderSettings`, `-outputSettings`, `-mem_usage
image_cache_pct max_mem_pct`, `-mfr ON|OFF max_cpu_pct`, `-v
ERRORS_AND_PROGRESS`, `-close DO_NOT_SAVE_CHANGES`, `-sound OFF`,
`-continueOnMissingFootage`, `-reuse`. There is **NO enumerate/list
mode**.

**Opus agent findings (spot-checked by me against the real files):**
- **aerender is fully headless** — ~13 s engine boot, then renders; no
  dialogs; parseable stdout. Ready today.
- **Comp enumeration via `AfterFX.exe -r <script.jsx>` is BLOCKED by AE's
  Welcome/Home screen** (a WebView2 surface that hangs the script launch
  — NOT a license/sign-in wall; the agent's heartbeat test proved the
  block is at startup before any script eval). Pref
  `"Show Welcome Screen"` in `…\Roaming\Adobe\After Effects\26.3\Adobe
  After Effects 26.3 Prefs.txt`. **SPOT-CHECK ✓**: I grepped that file —
  `"Show Welcome Screen" = 01` at line 1417. Flipping to `00` (AE closed)
  unblocks `-r`.
- **Default Output-Module templates** (agent read them from the prefs;
  **SPOT-CHECK ✓** I confirmed the mp4 + Lossless names in
  `…26.3\Adobe After Effects 26.3 Prefs-indep-output.txt`): three
  `H.264 - Match Render Settings - {5,15,40} Mbps` → self-contained
  `.mp4`; `Lossless`/`High Quality` → AVI; image seqs (`Photoshop`,
  `TIFF Sequence with Alpha`, `Multi-Machine Sequence`, `Alpha Only`).
  NOTE the literal DOUBLE space in `- Match Render Settings -  5 Mbps`.
  Render-Settings templates: `Best Settings`, `Draft Settings`,
  `Current Settings`, `Multi-Machine Settings`, `DV Settings`.
- **aerender stdout grammar** (verbatim, for the Rust parser): version
  banner `aerender version 26.3x87`; `PROGRESS: <text>` (first is always
  `Launching After Effects...`); `INFO:After Effects: <text>` (note: NO
  space after the first colon); `aerender ERROR: <text>`; `aerender
  SYNTAX ERROR: <text>`.
- **Quoting gotcha:** a PowerShell `-ArgumentList` array mangled
  `Best Settings` → aerender saw `Settings` → SYNTAX ERROR. Build the
  command as an ARG VECTOR (`std::process::Command` arg-by-arg, which
  quotes correctly on Windows) — never via a shell.
- Binary RIFX comp-name parsing is UNRELIABLE (the specimen is a CS6/v12
  project storing names in legacy `tdsn` chunks, not `Utf8`) —
  confirming ExtendScript enumeration is the real path.
- The agent's `enumerate.jsx` (walks `app.project.items`, dumps
  CompItem JSON, quits) is written but NEVER RAN (Welcome block). It is
  UNPROVEN on this machine — §6 step 1 proves it once the pref is off.

**Machine probe (I ran it):** this laptop = 8 physical / 16 logical
cores, 15.3 GB RAM → ~1 GB/logical-core, the deliberately-weak tier.

**ffmpeg (I ran `ffmpeg -encoders`):** 8.1.2 full build on PATH
(`…WinGet\…\ffmpeg.exe`) with `libx264`, `libwebp_anim`, `libaom-av1`,
`libsvtav1`, hardware `av1_nvenc`/`qsv`/`amf`, `gif` — the full export
menu is achievable here. ffmpeg is already the app's ADR-009 subprocess
(engine-media `render_stitch`/`render_replay`/`render_gif`).

**Repo seams (read in full earlier this session):** `.aep` classifies as
`media_kind` "element"; `drain_media_feeds` already drains
`i.raw.dropped_files`; `handle_import` routes by kind; `RenderState`
(Idle/Running/Done/Failed) + `std::thread` + `Arc<Mutex>` + progress is
the house background-job pattern (stitch, ingest, export all use it);
`index_render` write-through (poster+seconds+upsert); `ClipPlayer`
(open/seek/position/texture/playing/toggle) already decodes any playable
clip to egui textures and the timeline plays it. **Steps 3–4 of the
operator's roadmap (GPU pipeline, playback) are therefore already built
— the orchestrator's job ends at "produce a playable clip."**

## 3. Target design

Three layers, each honoring its house law.

### 3a. NEW crate `dpl-engine-ae` (pure orchestration, no egui)
- `locate() -> Option<AeInstall>` — scan `%ProgramFiles%\Adobe\Adobe
  After Effects <ver>\Support Files` for `aerender.exe` + `AfterFX.exe`,
  newest version wins; version-agnostic (never hardcode 2026). Linux/mac
  return None honestly (cross-support: the feature simply absent, not a
  crash).
- **Capability probe + tuning policy** — `Caps { ram_gb, logical_cpus }`
  probed at runtime; `RenderTuning::for_caps(caps, Profile)` → the
  `-mem_usage`/`-mfr` values. Policy (initial constants, tunable, UNIT
  TESTED at 16/32/64/128 GB × core counts):
  - `reserve_gb = max(4.0, ram_gb * 0.12)` (OS + the egui app + its GPU
    textures).
  - `max_mem_pct = clamp((ram_gb - reserve_gb)/ram_gb*100, 50, 90)`.
  - `image_cache_pct = round(max_mem_pct * 0.6)`.
  - `mfr = ram_gb / logical_cpus >= 2.5` (MFR spawns per-core AE workers,
    each needing GBs — enabling it RAM-starved thrashes). When on,
    `mfr_cpu_pct = clamp(leave ~2 logical cores free, 40, 90)`, lowered
    when RAM/core is tight. Worked example — THIS 15.3 GB / 16-logical
    box: `mfr = 0.96 < 2.5 → OFF`, `max_mem ≈ 74`, `image_cache ≈ 44`
    (RAM-conservative, single-frame — correct for the weak tier). A
    128 GB / 16-logical rig: `mfr = 8 ≥ 2.5 → ON @ ~87% CPU`,
    `max_mem ≈ 90`. `Profile::{Balanced (default), MaxThroughput
    (dedicated rig), Gentle (while streaming)}` shifts the reserve/CPU
    knobs; the operator overrides per-machine.
- `welcome_screen_blocking(&AeInstall) -> bool` — parse the prefs file
  for `"Show Welcome Screen" = 01`.
- `enable_automation(&AeInstall) -> Result<()>` — GUARD: refuse if any
  `AfterFX`/`aerender` process is running (AE rewrites prefs on quit and
  would clobber us); back up `Prefs.txt` → `Prefs.txt.dpl-bak`; flip the
  one line to `00`. Operator-tap-only (ratified). Reversible.
- `enumerate_comps(&AeInstall, aep) -> Result<Vec<Comp>>` — write a
  temp `.jsx` (paths injected as absolute literals) that dumps
  `[{name,width,height,duration,frame_rate,num_layers}]`; run
  `AfterFX.exe -r script.jsx`; read the JSON. Cache keyed on aep path +
  mtime (enumeration boots AE — do it once per project). `Comp` carries
  what the picker shows.
- `render(&AeInstall, RenderJob, impl FnMut(Progress)) -> Result<PathBuf>`
  — build the arg VECTOR (never a shell), spawn with `CREATE_NO_WINDOW`
  (Windows `CommandExt::creation_flags`), apply the tuned
  `-mem_usage`/`-mfr` + `-sound OFF -close DO_NOT_SAVE_CHANGES
  -continueOnMissingFootage -v ERRORS_AND_PROGRESS`, stream stdout, parse
  the line grammar into `Progress`/`Info`/`Error`. Output = a canonical
  playable intermediate (H.264 mp4 via `-OMtemplate "H.264 - Match Render
  Settings - 40 Mbps"`) for the timeline.
- `rifx_is_aep(path) -> bool` — 4-byte `RIFX` magic gate before any spawn.
- Tests (no AE needed for the pure parts): tuning policy across the RAM/
  core matrix; arg-vector quoting (spaces survive); stdout line parser
  (real samples from §2); RIFX check; prefs-blocking parse; comp cache
  key.

### 3b. Export selector — `dpl-engine-media` gains `transcode`
`ExportTarget::{Mp4H264, WebpAnim, Avif, Av1(Webm|Mp4), Gif,
LosslessPassthrough, ImageSeq}` → an ffmpeg arg vector off the baked
intermediate (or aerender-native for Lossless/ImageSeq). Reuses the
existing `run_ffmpeg`. The canonical timeline clip stays mp4 (ClipPlayer
plays it); export is a DELIVERY step producing a NEW artifact, never
mutating the intermediate. Encoders confirmed present (§2).

### 3c. `dpl-composer` UI wiring
- Route: an `.aep` drop that passes `rifx_is_aep` → the AE flow instead
  of filing inert (today's `handle_import` element branch).
- `AeJob` state (the `RenderState` shape: `std::thread` + `Arc<Mutex>` +
  progress) walking enumerate → pick → render → (optional export) →
  `index_render`-style write-through → timeline layer.
- **Comp picker** — cards/list from `enumerate_comps` (cached), honest
  empty/one-comp cases.
- **"Enable AE automation" one-tap** — shown only when
  `welcome_screen_blocking`; explains it edits one line of AE's config
  while AE is closed; fires `enable_automation` on tap.
- **Export selector** — the `ExportTarget` menu (default mp4; ratified
  operator choice per render).
- **Progress pill** — parses aerender stdout (`PROGRESS:` frames) like
  the ingest pill; boot phase honestly labelled ("Starting After
  Effects…", ~13 s).

## 4. Changes, in execution order (phased PRs — each a reviewable diff)

**Phase 1 — the crate foundation (no AE required to test):** create
`crates/engine-ae` (`dpl-engine-ae`): `locate`, `Caps` probe,
`RenderTuning::for_caps` + policy, `rifx_is_aep`, the aerender arg-vector
builder, the stdout parser, `welcome_screen_blocking` + `enable_automation`
+ prefs backup. Full unit tests (tuning matrix, quoting, parser, RIFX,
prefs). Dep decision (§5) resolved here.

**Phase 2 — enumeration (first real-AE verification):** `enumerate_comps`
+ the temp-jsx runner + cache. §6 step 1 proves it end-to-end once the
pref is toggled.

**Phase 3 — render + timeline ingest:** `render`; composer `.aep`
drop-route + `AeJob` + comp picker + progress pill + write-through; a
baked comp lands as a timeline layer. `--aep-render <aep> <comp>`
headless harness seam (mirrors `--export-guide`).

**Phase 4 — export selector:** `dpl-engine-media::transcode` +
`ExportTarget` UI.

**Phase 5 — the automation tap + tuning UI:** the "enable AE automation"
control + the per-machine `Profile`/override UI.

Docs (STATE, this §8) + proof shots each phase.

## 5. Boundaries

🚫 Sources (`.aep` and pinned trees) are READ-ONLY — renders/exports are
NEW artifacts (`/content` law; ADR-009 subprocess, never a crate dep). 🚫
aerender/AfterFX spawned with `CREATE_NO_WINDOW`; `enable_automation`
refuses while AE runs and backs up the pref first. 🚫 Never parse the
RIFX payload for comp data (unreliable + the thing we're avoiding). 🚫 No
bespoke frame→VRAM path — reuse ClipPlayer/timeline. ⚠️ **Ask-first
DEPS:** the RAM probe needs a cross-platform source — RECOMMEND the
`sysinfo` crate (small, Windows+Linux, also cores) vs a per-OS syscall;
operator ratifies before Phase 1. ⚠️ The app writing AE's pref is
operator-RATIFIED but tap-only and reversible. Out of scope: rendering
`project.aegraphic`/mogrt via AE (separate), the editable client-controls
UI, `.aepx` XML projects, Adobe Animate orchestration (same pattern,
later), hardware-encoder auto-selection (software encoders default; hw is
a later tuning pass).

## 6. Verification

1. **Enumeration spike (gates Phase 2):** with operator OK, `enable_
   automation` flips the pref (AE closed), then `enumerate_comps` on a
   REAL `.aep` writes a comps.json with real names — proving the agent's
   unproven `-r` path. If it still hangs, STOP and re-ground (fallback:
   render-first, operator types the comp).
2. `cargo test -p dpl-engine-ae` — the pure suite (tuning matrix asserts
   MFR OFF @16 GB and ON @128 GB; quoting keeps `Best Settings` intact;
   parser handles the real stdout samples).
3. `cargo green` ✔ each phase.
4. **Real-exe bake drill:** drop/`--aep-render` a real comp → an mp4
   appears, indexed, on the timeline, playing via ClipPlayer; the
   progress pill tracked aerender's `PROGRESS:` lines; total wall-clock
   reported (boot + render). Proof shots at real window sizes.
5. **Tuning proof across tiers:** unit-asserted now; when the operator
   runs the 32/64/128 GB machines, capture real render wall-times to
   refine the policy constants (a tuning-refinement follow-up).

## 7. Commit plan

Branch per phase off main (`feat/aep-1-engine`, `-2-enumerate`, …); each
a code+tests commit plus a docs commit; PR → squash-merge → exe rebuild →
STATE. Phase 1 first, fully. The `sysinfo` dep decision is ratified
before Phase 1 code.

## 8. Evidence

**Phase 1 — the crate foundation (2026-07-25, `feat/aep-1-engine`):**
`crates/engine-ae` (`dpl-engine-ae`) created — `locate`, `Caps::probe`,
`RenderTuning::for_caps` + the profile policy, `rifx_is_aep`, the
aerender arg-vector builder, the stdout parser, `prefs_file` +
`welcome_screen_blocking` + `enable_automation` (AE-closed guard +
`.dpl-bak` backup, ending-preserving flip). Dep: `sysinfo 0.33.1`
(operator-ratified, `system` feature only).
1. `cargo test -p dpl-engine-ae` → **10 passed** — the tuning matrix
   (MFR OFF @15.3 GB/16, ON @128 GB/16, the 64 GB/32 Balanced-vs-
   MaxThroughput flip, clamp invariants across 8→512 GB × 4→64 cores),
   arg-vector quoting (`Best Settings` survives as one token), the
   stdout grammar, the RIFX gate, and the Welcome-pref flip (CRLF +
   backup preserved, idempotent, refuses while AE runs).
2. `cargo green` ✔ (below).
3. REAL-machine probe (`--example probe`, kept as a per-rig diagnostic):
   `locate` → AE 2026 aerender/AfterFX paths; `Caps::probe` → 15.34 GB /
   16 logical (matches the OS); Balanced tuning = mem 74 / cache 44 /
   MFR OFF (matches the worked example); `prefs_file` → the real 26.3
   Prefs.txt; `welcome_screen_blocking` → **true** (independently
   confirms the Home-screen block). No AE process was launched.

## Provenance (deep-plan)

- Opus general-purpose agent filled the AE-execution gap (aerender
  headlessness, the Welcome-screen block, OM/RS templates, stdout
  grammar, the quoting gotcha). Fable spot-checked the Welcome pref
  (line 1417) and the mp4/Lossless template names against the real
  config files; ran `aerender -help`, the RAM/core probe, and `ffmpeg
  -encoders` directly. §3 design, the tuning policy, and all decisions
  are Fable's.
