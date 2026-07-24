# STATE — the living ledger

Updated every pass. This file is the first read of every session.

**Tip:** on `main` (2026-07-24). This session landed, via PR: the coldlight
trait system + viziwiz rename + main's #15/#17 native peer-window chat + the
three-product plan (#18); W-CAP palette claymorphism (#19); and the chat
consolidation (this branch). Exe rebuilt 2026-07-24.

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
   FIX (ratified 2026-07-24 "go", IN FLIGHT on PR #16 branch
   `design/coldlight-traits`): `docs/plans/pass0-coldlight-treatments.md`.
   LANDED: (1) doctrine rewritten to the composable trait system;
   (2) `tokens/traits.json` (neubrutalist/claymorphic base + cyber/retro
   stack) + build.rs generator (validated); (3) the pure compositor
   `compose(tone, base, stack, state) -> ResolvedStyle` (additive
   concatenate / override last-wins / state overlay; tests); (4) the
   paint API `cl::treated`/`brutal`/`clay` — `brutal_frame`/`clay_frame`
   RETIRED, all 9 callers migrated; the treatments RENDER (proof:
   docs/proof/traits-chat-{1100x700,1600x1000}.png — selected Codex chip
   is a bright neubrutalist button, lit-from-behind, dark text, over
   claymorphic panels). REMAINING in this program: tints painting +
   cyber/retro anim (needs motion.json + painter), the generated
   manifest the Workbench editor reads, the experience-inventory doc,
   then the `slide-catalog-pass` loop (port all ~27 slides, each an
   organized seed under Workbench: category × stackable tags). Skills:
   `coldlight-system` + `slide-catalog-pass`. Memory: `port-fidelity-standard`.

## CI + process (live as of #11/#12)

- Self-hosted runner `dpl-win-laptop` runs the green gate on every
  code PR (first run PASSED, 18m cold); pre-push hook enforces locally;
  PR → squash-merge flow; docs-only changes skip CI (paths-ignore).
- Staged: the Ubuntu/Wayland `dpl-linux` runner lane (commands in
  CLAUDE.md) — BOTH deploy targets (this laptop + the Wayland mini-PC)
  build on every PR once registered. Reboot-persistence for the Windows
  runner needs one elevated schtasks command (CLAUDE.md).
- Linux verification TODAY: WSL Ubuntu-26.04 has Rust 1.92 installed;
  ONE operator sudo apt command (CLAUDE.md "INTERIM Linux lane")
  unlocks `cargo check` inside Linux — cross-checking from Windows is
  blocked at khronos-egl's build script, so the Linux gate must run in
  Linux. The workspace's Linux code paths (pulse audio, Lavapipe
  profiles) have NOT compiled since the port — first Linux check may
  surface real findings.
3. Content root resolves from CWD (`./content`) — double-clicked exe reads
   `target/release/content`. Program P's data_root law not yet ported.
4. Stale EguiPilotAI scenarios/fixtures (pre-Program-L flyout/Library refs).

## Next

1. Footer fix + multi-size proofs (in flight, branch + PR).
2. Experience inventory doc (agent-authored from old app; operator ratifies).
3. Then the roadmap order: T (#109), G1 (#116), W5 reviewer, A1, P, RS1.

## Process facts

- Work lands via PR. The operator's exe rebuild + this file's update are
  part of DONE. Pushes can be slow — background them; commit is what counts.
- Old app reference: `C:\localdev\devpulse labs` (READ-ONLY).
