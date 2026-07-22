# ROADMAP — closing the census, with EguiPilotAI as the standing gate

Derived from docs/plans/census.md (2026-07-22, ratified missing-list order).
Every pass below runs plan → build → green → **pilot-verified** → commit.
Screenshot-only proofs are the fallback, not the standard, from here on.

## THE PILOT LAW (applies to every pass in this roadmap)

The reusable extension contract is canonical in `docs/plans/eguipilotai.md`;
the copyable DevPULSE feature path is
`docs/eguipilotai-devpulse-authoring.md`; current Playwright parity and gaps
are tracked in `docs/eguipilotai-playwright-review.md`; detailed protocol,
accessibility, runner, evidence, and DevPULSE census work is indexed in
`docs/research/eguipilotai/README.md`. Implementation follows the gated slices
in `docs/plans/eguipilotai-v1-delivery.md`.

EguiPilotAI grows with the app — boring, consistent, aligned with the mature
contracts in Playwright, WebDriver BiDi/Actions, AccessKit, and egui while
remaining native:

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

Runner parallelism does not begin until protocol response/event routing,
stateful input cleanup, and actual AccessKit parity are complete. Feature
passes may add owner semantics and scenarios against the serial runner, but
they must not invent temporary per-feature harnesses around those foundation
gates.

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
  BroadcastChannel). Pilot: multi-window target extension; dual-surface
  position-mirroring scenario.
- **D5 Scripts.** MDX-lite format kept byte-compatible (YAML frontmatter +
  inert body), parsed in dpl-doc; beats-parity validation; teleprompter
  consumes. Pilot: units + presenter scenario extension.

## Program X — Artifact index + library (census #2)

- **X1 Index engine.** `dpl-index` crate: embedded SQLite (rusqlite — the
  boring choice), 15-field record, sidecar law, upsert write-through from
  pixvix save paths, reindex-from-disk truth w/ soft-clear + empty-scan
  guard, rekey, additive migrations. Pilot: fixture-parameterized seeded
  data-root scenarios; index-cli for headless drills.
- **X2 Inbox review + Gallery.** keep (into EXISTING episode, companions,
  rekey) / dump (inbox-only) with the jail suite; wiredFRONT Gallery =
  ContentLibrary wall (filters, thumbs, honest fallbacks, rescan). Pilot:
  gallery scenario (filter taps, row assertions) + review flow.
- **X3 Associate/import.** copy-never-move into episode generated assets;
  frame-to-slide seam for W-track.

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
  reduced-motion support. Pilot: keyboard-only drag-match scenario (the
  old repo's Pass Q, now machine-verified every run).

## Program A — Agent console (census #7)

- **A1** port C1: session sidebar (Claude+Codex JSONL on disk), transcript
  normalizer (never drops a line), event-renderer registry, tabs, quota
  telemetry, jailed reader. Pilot: fixture transcript dir scenario.
- **A2+** C2 driving/Council/mesh — from the old plan docs, own ADR first.

## Program S — Sessions, rethought (census #8)

- **S-ADR**: rrweb was webview tech; ratify the video-first session
  vehicle (screen-record the app itself via the existing recorder +
  navigation event track from the fabric → seekable session artifact).
  No code before the ADR is ratified with the operator.

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
C-passes, T, A1, P, H. S-ADR whenever the operator wants the session
conversation. Every pass lands its pilot scenarios in the same commit —
coverage and features never diverge.
