# W5 PROPOSAL — the real .guide: steps stills + session replay, standalone & phone-loadable

Status: AUTHORED for a FRESH SESSION to execute (operator, 2026-07-25:
"plan this out i will start a fresh session"). This is a proposal-grade
plan: the inventory below was verified in-session on 2026-07-25, but the
executing session MUST re-read §2's files IN FULL before coding
(plan-code-change law — never code from a summary).

## 1. Context — what the operator asked for

"Do we have the tools now for a proper session replay where it will
work with .guide with the stills captured by steps recorder?" Combined
with the ratified Guide-tab vision (2026-07-24): "standalone double
click and easy to load on the phone while you go through it on your
computer," ink available live while presenting.

ANSWER (verified): YES on ingredients, NO on the container — the stills
pipeline, the session replay, and the export flow all exist; what's
missing is the .guide artifact itself. The ratified NATIVE Guide Player
(winit+wgpu+libmpv, .guide ZIP + portable exe) is its own unbuilt
product — and a phone can't run an .exe anyway. Therefore W5 v1 =
**a self-contained `guide.html`**: double-click on ANY device, steps
walkthrough with embedded stills, optional session-video replay with
event-tick chapters. Its manifest becomes the .guide ZIP payload when
the native player product lands — nothing throwaway.

## 2. Verified inventory (re-read these IN FULL before executing)

- `crates/engine-steps/src/lib.rs` — the Steps recorder (C2):
  two-frame do/done model (250ms settle, change-detected dedup, MARKED
  moments always kept), lossy-WebP stills (`STEP_QUALITY = 82`),
  `StepRec { order, file, result_file: Option<..>, … }` (READ THE FULL
  STRUCT — it has more fields than were verified this session, likely
  click position/kind), `Session { dir, steps }`, save files EXACTLY
  once: rename-uniquify + **`guide.json`** in the guide folder; forced
  metadata `CaptureMeta { library, show, episode, name, tags }` (kebab
  law, `_v1` suffix). Also `pub mod edit` (unread — check what step
  editing already exists).
- Sessions (S1/S2, read 2026-07-24 in `crates/hubs/wiredfront/src/
  lib.rs`): a session folder = `session.mp4` + `events.jsonl` (per-line
  `{ms, type, …}` → typed tick labels) + `session.jpg` poster; the
  in-app player has the event-tick seek rail (`session_player_ui`) —
  the interaction model to mirror in HTML.
- Index: guides are kind "guide" (thumb = `step-001.webp`), sessions
  kind "session"; `Index::keep` files artifacts into shows.
- Composer Guide tab (W2): the 1-2-3-4 export flow (name seeds, render,
  keep chips) — the natural mount for "Export guide".
- Gallery/steps viewing: NO in-app step-by-step guide VIEWER was found
  this session — verify (grep "guide" surfaces) before claiming one.

## 3. Target design

1. **NEW crate `dpl-guide`** (no UI deps): the .guide CONTENT MODEL +
   the HTML renderer.
   - `Manifest { title, meta (show/episode/tags), steps: Vec<Step
     {order, image, result_image?, caption?, …}>, session: Option<
     SessionRef {video, events: Vec<Tick{ms,label}>}> }` — built FROM
     the real `guide.json` + optionally a session folder. Serde
     round-trip; validator (every referenced file exists) — the doctor
     pattern.
   - `write_html(manifest, assets, out) -> Result<PathBuf>`: ONE
     self-contained `guide.html` — stills embedded as base64 WebP data
     URIs (a 20-step guide ≈ a few MB — fine), prev/next buttons +
     keyboard arrows + touch swipe, step counter, captions, do/done
     frame toggle where `result_file` exists; when a session is
     attached, the video is referenced as a SIBLING file (`session.mp4`
     next to the html — embedding video would bloat absurdly; state it
     in the html when missing) with event ticks as clickable chapters.
     No external requests, no webview, no JS deps — hand-rolled
     minimal script (the mock-file discipline).
2. **Guide tab grows a real step 0**: pick WHAT ships — a steps guide
   (list kind "guide" artifacts) and/or a session (kind "session")
   and/or the comp render (existing). Export writes
   `inbox/<name>/guide.html` (+ copies `session.mp4` beside it when
   included), indexes it (write-through), and the existing keep chips
   file it into a show. Double-click proof: the html opens in a
   browser, works offline, loads on a phone (copy/AirDrop/Drive —
   operator's transport, not ours).
3. **Harness**: `--export-guide <guide-rel> [<session-rel>]` → runs the
   exact Guide-tab path headlessly, prints the output path, exits —
   the real-exe drill.

## 4. Changes, in execution order (crate-level; the executor details them)

1. CREATE `crates/guide/` (`dpl-guide`): manifest + from-guide.json
   loader + from-session loader + validator + `write_html` + tests
   (round-trip, missing-file validator, html contains N data URIs and
   no `http`/`https` references).
2. `crates/composer` Guide tab: the picker (guides/sessions lists from
   the index) + Export action calling dpl-guide + index write-through +
   status/keep wiring.
3. `crates/app/src/main.rs`: `--export-guide` seam (headless like
   `--clone-seed`).
4. CLAUDE.md harness list + STATE.md.

## 5. Boundaries

🚫 The NATIVE .guide player (winit+wgpu+libmpv product) is NOT this
pass — the html states nothing about being "the" player; the manifest
is the forward contract. 🚫 No webview, no external assets, no new
crate deps beyond workspace serde/base64 (CHECK: is a base64 encoder
already in the workspace? if not, hand-roll the 20-line encoder rather
than adding a dep — ask-first otherwise). 🚫 Sources never mutated
(export files NEW artifacts). 🚫 `/content` law. ⚠️ Ask-first: caption
EDITING UI (does `engine-steps::edit` already provide it? read it
first — don't rebuild what exists).

## 6. Verification

1. `cargo test -p dpl-guide` (+ composer) green.
2. `cargo green` ✔ via hook.
3. Real drill: record a real 3+ step guide via ViziWiz Steps (or use
   an existing inbox guide), `--export-guide` it WITH a real session →
   the html opens from a double-click with stills, swipe/arrows,
   do/done toggles, video chapters; airplane-mode load proves
   self-containment; a PHONE load is the operator's acceptance test.
4. Proof shots: the Guide tab's picker; the exported html in a browser
   at phone-ish width (--window is app-side; browser shot is manual).

## 7. Commit plan

Branch `feat/w5-guide-export` off main. Commits: (1) dpl-guide crate +
tests; (2) Guide-tab picker + export + harness seam. PR → main →
exe rebuild → STATE.

## 8. Evidence

(filled at execution by the fresh session)

## Open questions for the operator (answer before or during execution)

1. Ship the session VIDEO alongside by default, or stills-only default
   with video as an opt-in checkbox? (Video = big files on a phone.)
2. Captions: type them at export time in the Guide tab, or edit steps
   beforehand (if `engine-steps::edit` already does this, it wins)?
3. Name the format: keep the folder `inbox/<name>/guide.html` +
   sidecars for now, or mint the `.guide` ZIP extension already (the
   native player will consume the same manifest either way)?
