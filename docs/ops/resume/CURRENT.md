# Resume pack — CURRENT

**Updated:** 2026-08-03
**Lane:** providers
**Dirty:** false

## READ THESE FOUR FIRST — in this order

1. `docs/plans/coldlight-naming-and-the-pattern-library.md` — **two laws that
   gate everything else.** No invented term where egui has one (`trait` →
   `FramePatch`, and the axes onto `egui::Frame`), and every pattern owes the
   slide catalog a generic item in the SAME pass. N1/N2 run BEFORE the
   providers links, or the new items get authored in the old vocabulary and
   renamed twice.
2. `docs/plans/chat-surface-map.md` — **what the chat ACTUALLY is today.**
   Crates, the 14 painted surfaces, the 24 `HostIntent` variants, the 51
   pilot ids, and a real/not-real table. Written because a pass was built
   against the wrong mock; do not skip it.
3. `docs/plans/providers-carousel-and-the-matrix.md` — **what changes next**,
   in the operator's words plus the platform terms for them.
4. `docs/plans/pulse-program.md` — how updates get pushed from the base of
   operations. PU1–PU5.

## The chat mock EXISTS now — and it is grounded

`tests/eguipilotai/chat-surface-capture.toml` + the X11 grab recipe in
`docs/plans/chat-surface-map.md` §7 produce a real frame of the running chat.
The pilot harness **cannot** screenshot it — `chat.primary` is a deferred
viewport and eframe 0.35 never delivers a queued screenshot for one — so the
capture goes through `Xvfb` + `ffmpeg -f x11grab` against the devtools exe.

What that capture proves, and why the change is not a preference:
**six provider rows fill the rail, "More providers…" is clipped mid-word, and
`PINNED` / `ACTIVE` / `RECENT CHATS` do not fit at all.**

`docs/design/mocks/providers.html` is the REGISTRY mock and is NOT the chat.

## Goal

The Providers carousel (rework of V2), against
`docs/plans/providers-carousel-and-the-matrix.md`:

- The rail's provider SECTION collapses to the `(ai) 🧠 Providers` control.
  **The rail itself stays.**
- **Registration stops being what puts a row in the rail — ACTIVATION does.**
  The chips stand before their groups have any entries; activating a provider
  is what makes its row appear underneath.
- **Two triggers, one overlay:** `＋` in the composer's input section, and the
  rail cross-link. Today there is exactly one, and it is the clipped
  "More providers…" row.
- Clicking either opens a full-viewport overlay with TWO sections:
  **subscriptions** as a horizontal carousel (few, large), and **API** as a
  grid of **4 rows minimum** that scrolls (many — volume is the reason).
- Tiles elevate on hover/focus and fire a **selection burst** on commit.
- The rail groups both populations under **elevated chips** —
  `subscriptions` and `API · usage-based`. The qualifier is operationally
  load-bearing: it says which group costs money per call.
- Chip hover is a **sporadic** multi-hue effect — NOT the slow `ink-bleed`.
  Same authored pool, different temperament.

## Done and on PR #192 (`feat/p1-provider-availability`, green)

P1 registry · V1 control + ink-bleed preset · V2 roster popup · V3 modal with
live tenure · A1 presence probes · A2 vendor sign-in · Cursor + Antigravity
providers · Enter-sends in the composer.

Live on this machine: codex/claude/grok **signed in**, cursor/gemini **not
installed**.

## Next

1. Land PR #192, or rebase it — #169 also edits `crates/fabric/src/lib.rs`.
2. Mock the carousel overlay, then plan it. Mock-gate first, no code.
3. PU1 — the `.pulse` container + verifier. **Signing is the real
   guarantee; encryption is obfuscation** — `pulse-program.md` §2.1 says why,
   and that section must not be softened.

## Known-wrong / placeholders

- `crates/app/src/pulse.rs` is an UNSIGNED JSON placeholder, superseded by
  PU2. It carries the researched provider CLI facts in the meantime.
- Cursor's probe needles are DOCUMENTED, not measured — `cursor-agent` is not
  installed here. Antigravity (`agy`) exposes no non-interactive auth status
  at all, so it can prove installed and not signed-in.
- The trait matrix has 4 traits and **no seed lineages**. The operator's five
  ancestrals — Bauhaus, Art Deco, Swiss, Aero, Brutalist — are not in it yet.
- `tokens/motion.json` has 19 effects and **no terminal 2D burst**.

## Key files

- `crates/chat/window/src/lib.rs` — 6031 loc, the whole painted surface
- `crates/shell/src/providers.rs` — the control, popup, modal
- `crates/app/src/presence.rs` · `crates/app/src/pulse.rs`
- `crates/coldlight/tokens/{motion,icons,presets,traits,hues}.json`

## Parked lanes

- **coach-ledger** — C2f Operator Import UI, plan drafted, awaiting
  `approve`; branch `feat/c2f-coach-import-ui` (#187).
- **substrate** (operator, 2026-08-02, unplanned): signals cleaner, DuckDB +
  Turso real, chat on moka / Dragonfly.
