# Resume pack — CURRENT

**Updated:** 2026-08-02
**Lane:** providers
**Dirty:** false

## Goal

V1 — the Providers control: the `dwell` duration tier, the two-colour
ink-bleed effect, the seeded hue pool, the brain icon, and the control itself
(brain + "Providers" + the `ai` circle at the P's cap-height).

The mock for the whole program is `docs/design/mocks/providers.html`. V1 is
the slice that makes its control real.

## Done

- **P1 — the provider capability registry**, on
  `feat/p1-provider-availability`, `green ✔`. `ProviderCapability` folded
  onto the fabric as THE ROSTER, persisted to `Tree::Agents`, projected onto
  the seam under `registry.providers`, read by a first consumer outside the
  chat crates. Live proof `platform-sees-providers` passes; shot at
  `docs/proof/platform-sees-providers.png`.

## Next

1. Open the P1 PR (branch is committed, not pushed) and land it double-green.
2. Author the V1 plan against `docs/design/mocks/providers.html` — tokens
   first (`tokens/motion.json` `dwell` tier + `ink-bleed`, `tokens/icons.json`
   brain), generated, never hand-written into the widget.
3. V1 replaces the placeholder `AI · …` text chip in the status bar.

## Key files

- `docs/design/mocks/providers.html`
- `docs/plans/providers-presence-and-the-brain.md`
- `docs/plans/chat-providers-become-platform.md`
- `crates/fabric/src/lib.rs` (THE ROSTER) · `crates/shell/src/status.rs`
- `crates/coldlight/tokens/motion.json` · `tokens/icons.json` (V1)

## Parked lanes

- **coach-ledger** — C2f Operator Import UI. Plan
  `docs/plans/coach-c2f-operator-import-ui.md` drafted, awaiting `approve`;
  branch `feat/c2f-coach-import-ui` (#187, conflicting). Notepad on Windows
  after. CX-harvest / Pass-2 parked.
