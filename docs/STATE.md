# STATE — the living ledger

Updated every pass. This file is the first read of every session.

**Tip:** `5593c40` on `main` (2026-07-23). One checkout, worktree removed.
Operator exe rebuilt 2026-07-23 16:51.

## Proven working (engine layer)

- Programs L (IA/accordion rail), D (deck/present/presenter/scripts),
  X (index/gallery/keep), C (capture parity: stills C1, steps C2,
  video C3, ViziWiz C4) — each with plan doc §8 evidence + exe drills.
- OS-global hotkeys Ctrl+Shift+S/9/X (rdev); Steps Save card; video
  format/audio matrix; ink flatten (source never mutated).
- EguiPilotAI client seat works: doctor green, live pixvix snapshot.

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
   and the `dpl-style` skill gates every UI pass on it. ColdLight grew
   the primitives (`brutal_frame`/`clay_frame`/`tip`, `RADIUS_LG`).
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
   **Retrofit sweep still owed**: shell footer launchers + rail (tips),
   hub/subroute cards → clay, session player controls, workbench walls,
   Save-card text fields' labels. Every UI pass MUST invoke `dpl-style`.
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
