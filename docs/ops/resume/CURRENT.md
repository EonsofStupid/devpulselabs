# Resume pack — CURRENT

**Updated:** 2026-08-04 (Theme API hard-cut; OS + S0; rebased on origin)
**Lane:** providers
**Dirty:** false

## READ THESE FIRST — in this order

1. `docs/design/coldlight-design-os.md` — ColdLight Design OS (seeds · entities · presets · Themes).
2. `docs/design/theme-vocabulary-law.md` — S0 Theme face · FramePatch metal · Looks forbidden.
3. `docs/plans/coldlight-naming-and-the-pattern-library.md` — S1 engineering rename + pattern catalog law.
4. `docs/plans/chat-surface-map.md` — what chat is today.
5. `docs/plans/providers-carousel-and-the-matrix.md` / `docs/plans/c1-providers-carousel.md` — next surface work.
6. `docs/plans/pulse-program.md` — PU1–PU5.

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

## Done and on PR #192 (`feat/p1-provider-availability`)

P1 registry · V1 control + ink-bleed preset · V2 roster popup · V3 modal with
live tenure · A1 presence probes · A2 vendor sign-in · Cursor + Antigravity
providers · Enter-sends in the composer · **C1** carousel/chips on this branch.

**Theme vocabulary (2026-08-04):** ColdLight Design OS + S0 Theme law; N1
FramePatch metal; `with_theme` / `apply_theme_preset` / `theme_enter` /
`theme_preset_tray`; Looks forbidden. Docs + code rebased on origin tip.

Live on this machine: codex/claude/grok **signed in**, cursor/gemini **not
installed**.

## Next

1. **Push + double-green** PR #192 — both hosted lanes SUCCESS, then merge
   only on operator go. Watch for fabric conflict with #169 if still open.
2. After land: rebuild release exe; tip STATE.
3. PU1 — the `.pulse` container + verifier. **Signing is the real
   guarantee; encryption is obfuscation** — `pulse-program.md` §2.1 says why,
   and that section must not be softened.

## Known-wrong / placeholders

- `crates/app/src/pulse.rs` is an UNSIGNED JSON placeholder, superseded by
  PU2. It carries the researched provider CLI facts in the meantime.
- Cursor's probe needles are DOCUMENTED, not measured — `cursor-agent` is not
  installed here. Antigravity (`agy`) exposes no non-interactive auth status
  at all, so it can prove installed and not signed-in.
- The patch matrix has 4 patches, each now declaring its `lineage` (N1). The
  five ancestrals exist as PROVENANCE only — no patch has yet been DERIVED
  from a lineage, which is the point of having them.
- `tokens/motion.json` has 19 effects and **no terminal 2D burst**.

## Key files

- `crates/chat/window/src/lib.rs` — 6031 loc, the whole painted surface
- `crates/shell/src/providers.rs` — the control, popup, modal
- `crates/app/src/presence.rs` · `crates/app/src/pulse.rs`
- `crates/coldlight/tokens/{motion,icons,presets,frame-patches,hues}.json`

## Parked lanes

- **coach-ledger** — C2f Operator Import UI, plan drafted, awaiting
  `approve`; branch `feat/c2f-coach-import-ui` (#187).
- **substrate** (operator, 2026-08-02, unplanned): signals cleaner, DuckDB +
  Turso real, chat on moka / Dragonfly.
