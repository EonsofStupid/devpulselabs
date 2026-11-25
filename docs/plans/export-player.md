# Export a player — the plan (EX)

**Status: STUB — ruled direction, to be grounded and planned in full when
its turn comes** (after LS4c). Ruled 2026-10-24 (R288).

The operator's words (R288, 2026-10-24):

> when i export we create this binary that I can run that responds to
> clicks/taps/ and some slides may require drag to point and release to
> connect or to slide away and reveal things like that so i can keep the
> audience engaged

## What is already ruled

- **The shape: a folder** ("A folder: player + lesson"). It holds the player
  exe (with the Lottie helper beside it, ADR-009), the lesson or episode,
  its slides at their pinned versions, scripts, looks, theme, and its
  videos and pictures. Double-click the exe and it opens straight into
  **Run** mode (`docs/plans/run-a-lesson.md`). Copy the folder anywhere.
- **Linked videos are copied in** ("Copied into the export"), so it runs on
  any machine with nothing plugged in and can never break mid-show.
- **The order:** run a lesson → portable storage → LS4b → LS4c (the
  gestures) → **EX** → LS4d. The first export is already interactive.

## What it builds on

- **Run mode (RL1):** tap anywhere to advance, swipe, the presenter
  driving, nothing editable, pinned versions. The exported player is
  Run mode with no Build mode at all.
- **Portable storage (ST):** the export is a library the player opens in
  place, with `roots.toml` set to imported and the marker present. The
  same layout, so no second format.
- **LS4c:** drag to point and release to connect (`drag_match`), slide away
  to reveal, push-to-trigger sub-events — as node trees and recipes that
  the renderer runs live. The export only carries them.

## Questions for the full plan (later)

- Does the player carry the whole app locked to Run, or a slimmer
  run-only build (smaller, but a second binary to keep green)?
- Does the export include the presenter's script screen (two windows), or
  only the audience slides?
- Is the export for Windows only, or Linux too (the lanes build both)?
- Does it record anything (taps, answers to a poll) for the operator to
  see afterwards?
