# ROADMAP — closing the census, with EguiPilotAI as the standing gate

Derived from docs/plans/census.md (2026-07-22, ratified missing-list order).
Every pass below runs plan → build → green → **pilot-verified** → commit.
Screenshot-only proofs are the fallback, not the standard, from here on.

## THE PILOT LAW (applies to every pass in this roadmap)

EguiPilotAI grows with the app — boring, consistent, aligned with how the
respected tools (Playwright / WebDriver BiDi / Maestro) shaped the same
problems:

1. **Stable IDs first** (Playwright's `data-testid` doctrine): every new
   interactive surface registers `PilotId`s at build time, named
   `devpulse.<domain>.<element>` (e.g. `devpulse.deck.next`,
   `devpulse.gallery.filter.guide`). Text/geometry are discovery locators,
   never identity — law 2 of the contract.
2. **One scenario file per user-facing flow** in `tests/eguipilotai/`
   (durable TOML, like `shell-navigation.toml`): the pass isn't done until
   its golden path drives through REAL input frames and asserts on the
   pruned semantic snapshot.
3. **Deterministic motion or no capture** (law 5): every new animation
   registers with `PilotClock` or a motion adapter the moment it's authored
   — never retrofitted.
4. **Renderer-keyed immutable baselines** for every new wall/surface; the
   Lavapipe lane is the routine gate, hardware lane for release.
5. **Toolkit extensions ride the passes, never fork them.** When a pass
   needs a capability the pilot lacks, extend the toolkit in the same
   boring shapes it already has (typed protocol command + runner method +
   CLI verb + scenario step), with conformance tests. Known upcoming
   extensions, tied to the passes that need them:
   - waiter/assertion vocabulary (`wait_for` semantic predicates with
     bounded timeouts — Playwright's auto-wait, made explicit) → D2
   - synthetic pointer gestures (drag path, swipe, pinch) → D2/T3/W4
   - multi-window/session targets (palette overlay, presenter viewport)
     → C-passes/D4
   - fixture parameterization (seeded content roots per scenario) → X1
   - trace viewer polish in Test Lab (step timeline ↔ captures) → ongoing

## Program D — Deck/Present runtime (census #1, the show machine)

- **D1 Episode documents.** `dpl-doc` grows the EPISODE tier: an episode
  folder owns `episode.json` (show meta, chapter list, ordered slide-folder
  refs) over the existing slide documents; validator extends (dup ids,
  dangling chapters, chapter-lock/interaction-budget/band as authored
  doctrine data, not hardcode); seed episode under `seeds/`. Pilot: doc
  round-trip units + an episodes wall scenario.
- **D2 DeckHost.** Reveal-step grammar (advance/retreat, step0=setup,
  roll-over, retreat-to-last — port the beat math verbatim); the full-bleed
  deck surface: tap/keyboard/edge-zone nav, progress rail + chapter ticks,
  toolbar, navigator overlay, chrome-hide + home-dot + edge-reveal strip
  (touch law: tap-min, no hover-only). Pilot: `deck-golden-path.toml`
  drives an entire seed episode start→end asserting position after every
  beat; needs the waiter vocabulary + gesture extension.
- **D3 Ink layer.** Stream-time annotation (pressure width, ink token
  color, per-slide clear, per-slide opt-out). Pilot: pointer-stroke flow +
  PilotClock-settled capture baseline.
- **D4 Presenter + sync.** Second viewport teleprompter (beat list, active
  script, up-next, timer) synced over the fabric (typed event, not
  BroadcastChannel). LANES (operator, 2026-07-23): EguiPilotAI
  extensions (multi-window targets included) are the CODEX seat's work —
  this seat authors the feature and declares the pilot deferral until
  the extension lands; never extends the toolkit itself.
- **D5 Scripts.** MDX-lite format kept byte-compatible (YAML frontmatter +
  inert body), parsed in dpl-doc; beats-parity validation; teleprompter
  consumes. Pilot: units + presenter scenario extension.

## Program X — Artifact index + library (census #2)

- **X1 Index engine.** `dpl-index` crate: embedded SQLite via TURSO (the
  pure-Rust engine — operator-ratified 2026-07-22, same engine the old
  repo proved), 15-field record, sidecar law, upsert write-through from
  pixvix save paths, reindex-from-disk truth w/ soft-clear + empty-scan
  guard, rekey, additive migrations. Pilot: fixture-parameterized seeded
  data-root scenarios; index-cli for headless drills.
- **X2 Inbox review + Gallery.** dump (inbox-only, companions, guide
  recursive) with the jail suite; wiredFRONT Gallery = ContentLibrary
  wall (filters, thumbs, honest fallbacks, rescan). Pilot: gallery
  scenario (filter taps, row assertions) + review flow. (KEEP moved to
  X3 — same filing-into-content doctrine as associate.)
- **X3 Keep + associate/import.** CONTENT TREE RATIFIED (operator,
  2026-07-23): `content/<show-name>/<episode-name>/` — the show name IS
  the stream category (e.g. `content/devpulse-news/<episode>`); episode
  folders carry the SAME doctor-enforced contract as seeds (episode.json
  + look.json + slide folders) plus a reserved episode-level `assets/`
  for kept artifacts. Seeds stay pristine templates; "new show" copies a
  seed into content; episode listings union both roots. EPISODES ARE NOT
  GUIDES (operator's emphasis): the Steps/guide system files through the
  inbox and stays separate — a guide only enters an episode via an
  explicit associate/T4 action, never by filing. Keep = move inbox
  artifact + companions into `<ep>/assets/` + rekey; associate/import =
  copy-never-move; frame-to-slide seam for W-track. ✅ DONE (plan
  docs/plans/x3-keep-associate.md §8 records the execution): dpl-doc
  content tier (list_shows/list_content_episodes/all_episodes/
  clone_episode, EPISODE_RESERVED += assets), dpl-index keep/associate
  (jail-bound source, episode-must-exist, companions, never-clobber,
  sidecar rewrite + rekey+upsert), Gallery Keep flow, union pickers in
  Present + Workbench + editor, "copy to my shows" chip w/ the three
  ratified show suggestions; drills: index-cli keep/associate, app
  `--clone-seed`; the first real show exists:
  content/devpulse-news/how-ai-works.

## Program W — Authoring (continues #104/#105)

- **W3 Presets** (ledger #104) — 6th token source + wall. Pilot: presets
  wall baseline.
- **W4 Tap-to-edit editor** (ledger #105) — the layman-touch editor over
  seed documents; write-back + undo. Pilot: the census's marquee scenario —
  tap title → font tray → pick Orbitron → assert the colocated slide.json
  diff is exactly one ref (the pilot asserts on disk state via a fixture
  hook, not just pixels).
- **W5 Guide reviewer + frame-to-slide.** do→done cards, click-highlight
  ring, reorder/delete/caption editing over guide.json; import a frame as
  a still slide. Depends X1/X2.

## Program C — Capture parity (census #4)

- **C1 Stills parity:** freeform polygon (overlay + engine), window-mode
  stills + picker, capture delay, re-snip, clipboard auto-copy, arrow-key
  nudge ceremony. Pilot: overlay flows via the multi-window extension;
  engine units for polygon masking.
- **C2 Steps parity:** rdev OS-global hooks (Ctrl+Shift+S/9/X), global
  click hook + window-following + click coords + do→done fingerprint
  dedup, seed-name from window title, forced-metadata Save card, guide
  EDIT rewrite. Pilot: in-app portions scenario-driven; OS-global portions
  engine-unit + drill (the pilot never fakes OS hooks).
- **C3 Video parity:** format matrix (mp4/webm-vp9/av1), audio
  (loopback/mic + device list), 720p default + max-cap, REC timer +
  poster. Pilot: recorder lifecycle scenario w/ suppressed-motion capture.
- **C4 ViziWiz ink editor** (wiredFRONT-side, never mixed with Workbench):
  ink tones/width/undo, flatten → inbox via forced metadata. Pilot: stroke
  + flatten flow, baseline on the composite.

## Program T — Templates & interaction (census #5)

- **T1** still (+pinch-zoom), video, video-loop, bullets templates (as
  document block kinds + templates; media block already carries the
  slots). Pilot: gallery-gate baselines per template.
- **T2** hotspots, diagram; replay template waits on Program S.
- **T3** minigame doctrine as data + drag-match keyboard path +
  reduced-motion support. DOCTRINE RATIFIED (operator, 2026-07-23):
  authored data, NEVER hardcoded — one `doctrine.json` shape also
  carries the deferred per-slide flags (chrome-hidden, ink opt-out,
  chapter-lock, budgets). Pilot: keyboard-only drag-match scenario (the
  old repo's Pass Q, now machine-verified every run).
- **T4 guide-slide template** (operator's spec, 2026-07-22): a FULL
  recorded guide becomes ONE slide — its internal reveal steps ARE the
  recorded steps, ridden by the D2 beat grammar (advance/retreat walks
  the do→done frames with the click-highlight ring; one guide = one
  slide in a show, stepped through live on the 75"). A new
  `Block::Guide` kind referencing the captured guide through the folder
  contract (frames live under the slide's reserved `assets/`); frames
  render through the one renderer; editable like any slide (the W5
  reviewer owns reorder/delete/caption). Depends: C2 (capture), D2
  (beat grammar), X1 (guides indexed). Pilot: a fixture guide steps
  end-to-end via the deck scenario.

## Program G — Guide export (net-new; operator's spec 2026-07-22)

The full guide pipeline this program completes: **C2 capture → W5
review/edit in Workbench → T4 guide-slide in a show → G1 standalone
export**. The export is the operator's shareable artifact: *"a self
contained guide i can later share on my website... a vlc like
experience... stand alone easy to download and browser and be optimized
while styling engaging."*

- **G1 self-contained guide export:** one download — a folder (or
  single-file HTML) bundling the guide's WebP frames + guide.json + a
  small dependency-free player: VLC-like transport (play/pause/
  step-forward/back, scrub, keyboard + touch), progress rail with step
  ticks, do→done toggle, click-highlight ring, captions from guide
  metadata. Styled FROM ColdLight — the mood/ink/font vocabulary
  generated out as CSS from the same token sources (never hand-authored
  twice; a build-time emitter, same pattern as the registries). Browser-
  optimized: WebP frames, lazy-load + preload-next, zero network
  dependencies — works from file:// and any static host. Pilot: export
  a fixture guide → engine-unit asserts the bundle is CLOSED (no
  external refs) + a headless-browser drill steps it end-to-end.
- **G2 export polish:** poster/OG share card, optional autoplay demo
  mode, size-budget report at export time. After G1 proves out.

Operator (2026-07-23): the portable guide export is a SEPARATE,
later, independent bonus feature — never entangled with the show
doctrine or the episode tree.

## Program A — Agent console (census #7)

- **A1** port C1: session sidebar (Claude+Codex JSONL on disk), transcript
  normalizer (never drops a line), event-renderer registry, tabs, quota
  telemetry, jailed reader. Pilot: fixture transcript dir scenario.
- **A2+** C2 driving/Council/mesh — from the old plan docs, own ADR first.

## Program S — Sessions, rethought (census #8)

- **S-ADR — RATIFIED (operator, 2026-07-23: "yes we have to go
  native")**: the video-first native session vehicle — screen-record the
  app itself via the existing recorder + a navigation event track from
  the fabric → a seekable session artifact. Full plan:
  docs/plans/s-adr-sessions.md. ✅ S1 capture (51bc34b) + S2 playback
  done; S3 export remains.

## Program L — the IA doctrine rebuild (operator, 2026-07-23) — PRIORITY

The operator's IA correction OUTRANKS the roadmap order ("we can
resume the roadmap whenever but this is more important"). Full capture
+ all rulings: docs/plans/l1-library-doctrine.md. Ledger #118. In
one breath: the ACCORDION rail (one house open, subroutes unfold
downward); a SHOWS rail entry (EffingAI · DevPulse News · Eons of
Stupidity — each page = Episodes · Gallery · Social over
content/<show>); Workbench = the BUILD house whose LIBRARY absorbs
the top-level Library hub's walls plus seeds (class folders:
static/split/animation/interactive) — the hub dissolves, pre-release
no-shims; wiredFRONT = the PREP house (captures reviewed + played in
place in the Gallery; custom slides minted here file as Library
INGREDIENTS); Home stays the streaming-prop backdrop; Workbench later
grows PDF carousels + execute-and-post social automation (YouTube, X,
TikTok, IG/FB, potentially Twitch + the operator's own community
platform — adapter registry leaves room for first-party). Passes:
L1a accordion rail → L1b seed class folders → L1c Library rebuild →
L1d Shows rail → L1e Gallery plays sessions in place.

## Program DEMO — agent-driven demonstrations (operator, 2026-07-23)

The operator's convergence insight, decided net-new (ledger #117):
"record demonstration using eguiPilot" — the route-aware chat gains a
UI mode that COMPOSES the tools already built: an EguiPilotAI scenario
DRIVES the app while the session recorder (S1) and/or the Steps
recorder CAPTURE it; the artifact is a guide or a session, web-playable
by construction (the G pattern). Say "record the steps for enabling
X", and the demonstration records itself.

- **In-app tier:** pilot scenario + recorder — every piece exists
  after S1/S2; the composition waits on F3 (typed chat tools) and the
  Codex seat's pilot extensions (their lane, unchanged).
- **External tier:** demonstrations of OTHER tools (e.g. a browser on
  Lovable): spin up a browser, drive it, and let Steps record the
  screen — the Steps recorder is already app-agnostic (OS-wide clicks).
  The browser DRIVER needs its own ADR (CDP, install-on-demand
  tooling — "whatever tools we need to install to make this
  streamlined", operator's words) — planned properly, not smuggled.
- **Standing word restated (operator, 2026-07-23):** absorbing After
  Effects capabilities is NOT to be gated — this is a private
  quality-of-life tool; licensing is a non-constraint (also recorded
  at R1 and in the vision doc).

## Program P — Portable/kiosk (census #9)

- **P1** data_root law + runtime_status self-report; **P2**
  package-portable + portable drill (as a pilot scenario suite run against
  the packaged folder); **P3** --kiosk + work-area clamp.

## Program H — Shell polish debt (census #10)

- NavDock-equivalent tool surface, persisted-state law (validated persist
  vs session-only), remaining fx ingredients (scanlines/noise/duotone/
  spot/conic as authored tokens), shortcut parity ([ ] rails, \ fullbleed).
  Folded into D1 (design session) where visual.

## Sequencing

W3 → W4 (in flight, small) run first; then **D1–D2** (the show machine
core) with X1 in parallel (index is UI-independent); then X2/W5, D3–D5,
C-passes, T (T4 after C2+D2+X1 land), **G1** (after T4 — the pipeline's
payoff), A1, P, H. S-ADR whenever the operator wants the session
conversation. Every pass lands its pilot scenarios in the same commit —
coverage and features never diverge.
