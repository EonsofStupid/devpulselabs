# C2b — `dpl-coach-overlay` daemon (X11 self-test first)

**Status:** PASS pass-1 self-test 2026-07-30 · landed #158  
**ADR:** [`coach-overlay-adr.md`](coach-overlay-adr.md) (RATIFIED)  
**Evidence base:** C2s spike + live Triggered journal  
**Journal:** [`../journal/2026-07-30-c2b-coach-overlay.md`](../journal/2026-07-30-c2b-coach-overlay.md)  
**Product NEXT (not this plan):** [`coach-operator-experience.md`](coach-operator-experience.md)
(C2c mock → inventory → fabric mint). **Pass-2 stdio is PARKED** (Advanced).

## 1. Goal

Land the first shippable `dpl-coach-overlay` binary that proves the ADR
gates on this box: locate → stream identity → WINDOW-space ColdLight paint
→ Sync-grab observe → Triggered, without wiring into production `app`.

## 2. Current state

- Physics proven in `foreign-target-probe` (desktop-spike) + live Triggered.
- ADR ratified; production IPC still forbidden.
- `crates/coach-overlay` ships on main (#158); `--self-test` green on this X11 box.
- Pass-2 stdio coach loop **parked** — product work is Operator Coach UX
  (C2c+), not more daemon pipe before the picker exists.

## 3. Design

Workspace member `crates/coach-overlay` (`dpl-coach-overlay`):

| Piece | Role |
|---|---|
| `lib.rs` | Compose / press-edge / refuse types (unit-tested) |
| `bin/dpl-coach-overlay` | CLI: `--self-test` (and later stdio coach loop) |
| Linux backend | AT-SPI + x11rb; cfg-gated |
| Windows stub | exit 2, no atspi/x11rb compile |

`--self-test [--name 7] [--app gnome-calculator]`:

1. Ensure app on AT-SPI bus  
2. Locate + `GetAccessibleAtPoint` name-match (else refuse)  
3. Overlay on calculator client; paint WINDOW hotspot; caption above  
4. Sync-grab observe + digit-matched display watch  
5. XTEST press at screen hotspot center  
6. Exit 0 on Triggered, 1 on timeout, 2 on refuse/platform  

Never a dep of `crates/app`. Structure: member + architecture §3 note.

## 4. Changes (order)

1. Ratify ADR + AGENTS.md tip (this pass start).  
2. Add crate + workspace member + architecture standing exception.  
3. Port geom tests + Linux self-test from spike findings.  
4. `cargo test -p dpl-coach-overlay` + live `--self-test`.  
5. Journal §evidence; tip STATE / ops.  
6. Pass 2 if needed: stdio `CoachCommand` loop (not required for self-test gate).

## 5. Boundaries

- No production wiring, no capability bit flip in protocol yet.  
- No Tier B/C, no Wayland GNOME claim.  
- Spike binary remains; overlay does not delete it.  
- Forbidden set = ADR §5.

## 6. Verification

- Unit: compose + press-edge matrix.  
- Reality: `target/release/dpl-coach-overlay --self-test` → Triggered on calculator.  
- Adversary: `--name bogon` refuses; Windows stub exits 2; `cargo check` workspace.

## 7. Commit plan

Branch `feat/c2b-coach-overlay` → PR with plan + crate + evidence. Pass 2
stdio coach loop is a follow-up PR if not in the first land.

## 8. Evidence

**2026-07-30 (this box, `DISPLAY=:1`, `gnome-calculator`):**

| Check | Result |
|---|---|
| `cargo test -p dpl-coach-overlay --lib` | 4/4 pass |
| `cargo clippy -p dpl-coach-overlay --all-targets -- -D warnings` | clean |
| `cargo xtask structure` | 66 members hygienic; orphan allow `coach-overlay` |
| `--self-test --name bogon` | exit 2 refuse |
| `--self-test --name 7` | exit 0 Triggered; ButtonPress `251,484` inside `64×44@219,462`; `stream_hit="7"` |

Journal: `docs/journal/2026-07-30-c2b-coach-overlay.md`.

**After #158:** do **not** treat Pass-2 stdio as NEXT. Product spine:
[`coach-operator-experience.md`](coach-operator-experience.md) (C2c+).
Pass-2 remains a valid Advanced follow-up under ADR launch gates when the
operator asks for it.
