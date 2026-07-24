# R1w — comp documents over the seam: chat writes and RENDERS compositions

## 1. Context

R1v landed the signal seam (navigate/observe). This pass delivers the
ledger's "agent tools" intent (R1f): chat GENERATES into the editor — it can
list, read, WRITE (doctor-validated) and RENDER comp documents over HTTP.
Operator requirement: "chat will be able to generate, and work with this as
well… ability to have chat create videos… stitching videos." After this
pass an agent can author a `comp.json` from library clips and get a stitched
`render.mp4` back — the full R1 §5 loop over HTTP.

## 2. Current state (read in-session)

- `crates/seam/src/lib.rs` (R1v) — `SeamState{fabric, registry, last_route}`,
  `router()` with /state /command /signals; `handle_command` speaks
  navigate/viziwiz-summon; 5 tests (tower oneshot).
- `crates/comp/src/lib.rs` — `Composition` is serde end-to-end;
  `load(dir)->Result<Composition,CompError>`, `save(dir,&comp)` (crash-safe
  swap), `validate(&comp)->Vec<Issue{path,message}>` (shape doctor; media
  existence is a SOFT R1c concern).
- `crates/engine-media` — `StitchClip{path,in_pt,out_pt}`,
  `render_stitch(&clips,w,h,fps,&out)->Result<(),String>` (ffmpeg concat,
  ADR-009 subprocess).
- `crates/engine-capture` — `content_root()` honors the `CONTENT_ROOT` env
  var (→ the live drill runs against a scratchpad root).
- `crates/composer` — `fresh_comp_dir()` mints the first free
  `inbox/comp-N`; `stitch_clips()` maps Artifact layers, skips Elements.
- `crates/app/src/main.rs` — `dpl_seam::spawn(fabric.clone(), registry,
  port)` at the registry site.

## 3. Target design

The seam gains the comp-document surface, all under the same laws: the
DOCTOR gates every write (invalid comp → 422 with issues NAMED, nothing
written), rel dirs are JAILED inside the content root (no `..`, no absolute
paths — the index dump-jail's spirit), renders run OFF-thread with a polled
status. `SeamState` gains `root: PathBuf` (injected at spawn; tests inject a
temp dir) and `renders: Mutex<HashMap<String, RenderStatus>>`.

Endpoints:
- `GET /comps` — every `inbox/*/comp.json` under the root:
  `[{dir, clips, seconds} | {dir, error}]` (unreadable named, never hidden).
- `GET /comp?dir=inbox/comp-1` — the document + its live doctor report:
  `{dir, comp, issues}`.
- `POST /comp` `{dir?, comp}` — parse → `validate` → issues? `422
  {ok:false, issues}` (NOTHING written) : `save` → `200 {ok:true, dir}`
  (absent dir mints the first free `inbox/comp-N`).
- `POST /command {"cmd":"render","dir":…}` — load + validate (422 on
  issues/no playable clips/already running) → off-thread
  `render_stitch` → `202 {ok:true, out:"<dir>/render.mp4"}`.
- `GET /render?dir=…` — `{state: idle|running|done|failed, out?, error?}`.

## 4. Changes, in execution order

1. `crates/seam/Cargo.toml` — deps += `dpl-comp`, `dpl-engine-media`.
2. `crates/seam/src/lib.rs` — `jail(root, rel)`; `fresh_comp_dir(root)`;
   `RenderStatus`; `SeamState` grows `root` + `renders`;
   `seam_state(fabric, registry, root)`; the four routes + handlers;
   `spawn(fabric, registry, root, port)`; render worker thread
   (`dpl-seam-render`). Tests: jail refusals; invalid comp → 422 + nothing
   written; valid POST mints `inbox/comp-1` and GET round-trips; /comps
   lists it; element-only render → 422.
3. `crates/app/src/main.rs` — pass `dpl_engine_capture::content_root()`
   into `spawn`.

## 5. Boundaries

- 🚫 The operator's real `/content` — the live drill runs the exe with
  `CONTENT_ROOT` pointing at a scratchpad tree.
- 🚫 No source mutation ever — renders write NEW files (`render.mp4`).
- 🚫 No delete/dump endpoints (destructive ops stay behind the two-tap UI).
- ⚠️ Ask first: exposing the slide/episode document domain over the seam.
- Out of scope: gen backends, matting, live editor reload on external write
  (the composer reads its comp at open; a comp reload signal is a later rider).

## 6. Verification

1. `cargo test -p dpl-seam` → all green (5 old + 5 new).
2. `cargo green` via the pre-push hook → exit 0.
3. LIVE DRILL (scratchpad `CONTENT_ROOT`, sequential, then kill, no orphans):
   ffmpeg mints two lavfi clips in `<root>/inbox`; boot the real exe with
   the env; `POST /comp` (two Artifact layers, trims) → 200 + dir;
   `POST render` → 202; poll `GET /render` → done; ffprobe
   `render.mp4` ≈ the comp duration; a `POST /comp` with a zero-length
   layer → 422 naming the layer, file untouched.

## 7. Commit plan

Branch `feat/r1w-comp-endpoints` stacked on `feat/r1v-agent-seam`. One
commit: `feat(seam): R1w — comp documents over HTTP (list · read · doctor-
gated write · ffmpeg stitch render)`. Push + PR base r1v.

## 8. Evidence

- `cargo test -p dpl-seam` → 9/9 green (jail refusals by name; invalid comp
  → 422 + NOTHING written; valid POST mints `inbox/comp-1` + GET round-trip
  + /comps census; element-only render refused; plus the R1v five).
- LIVE DRILL (scratchpad `CONTENT_ROOT`, real exe, 2026-07-24, killed clean):
  - ffmpeg minted two deliberately mismatched clips: 640x480@30 3s +
    800x600@24 2s.
  - First POST /comp attempt used the wrong Source shape → the DOCTOR
    refused it: `422 "comp does not parse: missing field kind"` — the
    refusal named exactly what an agent needs to self-correct (and this
    drill self-corrected from it, which IS the design working).
  - `POST /comp` (tagged `{"kind":"artifact","rel":…}`) →
    `{"ok":true,"dir":"inbox/comp-1"}`.
  - `POST /command render` → `{"ok":true,"out":"inbox/comp-1/render.mp4"}`;
    poll `GET /render` → `state:"done"`.
  - `GET /comps` → `[{"dir":"inbox/comp-1","clips":2,"seconds":5.0}]`.
  - Zero-length layer POST → HTTP 422.
  - **ffprobe: `duration=5.000000`** — the two mismatched clips normalized
    and stitched to one 1280x720@30 file of exactly the comp's duration.
    CHAT CAN NOW WRITE A COMPOSITION AND GET A RENDERED VIDEO OVER HTTP.
- Deviation: none from §4; gate runs in the pre-push hook.
