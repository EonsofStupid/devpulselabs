# Resume pack — CURRENT

**Updated:** 2026-08-02
**Lane:** providers
**Dirty:** false

## Goal

V3 — the provider modal: a card promoted into a summary with the live-ticking
"how long they have been in business" figure. V2's cards are the thing it
grows from.

The mock for the whole program is `docs/design/mocks/providers.html`. V2 is
the slice below its control.

## Done

- **V2 — the roster popup**, `green ✔`. Cards carry live presence from the
  registry; not-installed and signed-out stay distinct. Cursor is registered
  as a provider, so it appears as a real row.
- **V1 — the Providers control + the ink-bleed preset**, `green ✔`. `dwell`
  tier + `dwell` ease + `ink-bleed` in `tokens/motion.json`, `brain` in
  `tokens/icons.json`, and a new authored `bleed` preset kind whose hue pool
  is build-checked against `hues.json`. Proof shots
  `docs/proof/providers-control-{cold,full}.png`.
- **P1 — the provider capability registry**, on
  `feat/p1-provider-availability`, `green ✔`. `ProviderCapability` folded
  onto the fabric as THE ROSTER, persisted to `Tree::Agents`, projected onto
  the seam under `registry.providers`, read by a first consumer outside the
  chat crates. Live proof `platform-sees-providers` passes; shot at
  `docs/proof/platform-sees-providers.png`.

## Next

1. Open the PR (P1+V1 committed, not pushed) and land it double-green.
2. Author the V3 plan against the mock — the tooltip that becomes a modal.

## Key files

- `docs/design/mocks/providers.html`
- `docs/plans/providers-presence-and-the-brain.md`
- `docs/plans/chat-providers-become-platform.md`
- `crates/fabric/src/lib.rs` (THE ROSTER) · `crates/shell/src/status.rs`
- `crates/shell/src/providers.rs` (the control)
- `crates/coldlight/tokens/{motion,icons,presets}.json`

## Parked lanes

- **coach-ledger** — C2f Operator Import UI. Plan
  `docs/plans/coach-c2f-operator-import-ui.md` drafted, awaiting `approve`;
  branch `feat/c2f-coach-import-ui` (#187, conflicting). Notepad on Windows
  after. CX-harvest / Pass-2 parked.
