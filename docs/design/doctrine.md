# Design doctrine — the visual language, finally written down

This is the contract the first app carried in feel but never in words. It
gates every UI pass. The ONLY implementations are ColdLight's primitives —
surfaces never hand-roll these treatments.

## The two treatments

**BRUTAL** (`cl::brutal_frame(tone)` / the chip painters that mirror it) —
for everything ACTIONABLE: chips, buttons, toggles, anything that does
something when tapped.
- Thick 2px tone edge · HARD offset shadow `[3,3]`, zero blur (the shadow
  is a shape, not a glow) · flat `OVERLAY` fill · `RADIUS_SM`.
- Active state: tone wash fill + tone stroke. Idle: hairline stroke.
- Feel target: punchy, honest, unmistakably tappable — a layman knows it's
  a button without being told.

**CLAY** (`cl::clay_frame()`) — for everything PASSIVE: content wells,
panels, cards that hold things, empty states.
- Generous `RADIUS_LG` · deep soft scatter (`[0,10] / 32 / 4`) · hairline
  edge · `RAISED` fill.
- Feel target: soft, extruded, calm — content sits IN it, chrome sits ON it.

**Never mixed on one element.** If it does something → brutal. If it holds
something → clay. If it's ambiguous, it's probably two elements.

## The laws

1. **Tooltip law**: every interactive control ships `cl::tip(resp, "…")` —
   plain-language, layman-first ("Snap the whole screen", not "Full
   capture"). A control without a tip fails review.
2. **Tokens only**: ColdLight surfaces/ink/tones/radii — never raw hex,
   never ad-hoc paddings where a space token exists.
3. **Any-shape rendering**: every surface must render correctly at any
   window shape (`--window <w>x<h>` proofs; the fit guard owns the OS
   window). Fixed dimensions require a stated reason.
4. **Motion respects PilotClock** (EguiPilotAI law) — no uncontrolled
   animation on new surfaces.
5. **Proof includes feel**: a UI pass's §8 evidence includes at least one
   screenshot the operator could mistake for the old app's polish level —
   that is the acceptance bar.

## Rollout

Applied first to the shared chip painters (gallery/viziwiz `filter_chip`).
The retrofit sweep (pixvix palette chips, shell footer launchers, hub
cards → clay, rail tooltips) is tracked in docs/STATE.md and runs surface
by surface — every UI pass after this doc lands MUST invoke the
`dpl-style` skill and leave touched surfaces doctrine-clean.
