# Journal — 2026-07-30: C2s Triggered live proof (operator + calculator)

## Done so far

Live **Triggered** on this box (`DISPLAY=:1`, X11) is closed — operator press
on the coached control, not only `--proof-xtest`.

### What the streams proved

| Claim | Stream evidence |
|---|---|
| Locate name `7` | AT-SPI `Role::Button` + name; WINDOW `64×44@16,293` |
| Compose | X11 origin + `_GTK_FRAME_EXTENTS` → screen `64×44@219,462` |
| SCREEN is a lie (GTK4) | `GetExtents(SCREEN)` → `(0,0,…)` — never trust |
| Identity | `GetAccessibleAtPoint(WINDOW, center)` → `stream_hit="7"` |
| Actuation | XTEST at composed center → display Text `"7"`; same for `5` |
| Observe | Sync-grab Button1 + `ReplayPointer` (never eats the click) |
| Live operator | ButtonPress `256,480` **inside** hotspot → `Triggered` exit 0 (~4s) |

Paint now sits on the **calculator client** (WINDOW space = AT-SPI space).
Caption plate prefers **above** the ring.

### Landmines closed this pass (method, not folklore)

1. **Display Text was not empty** — Rust reader used the wrong walk /
   `GetText` form. Fixed: `Role::Text` + `GetText(0, -1)`.
2. **QueryPointer alone missed short real presses** — Sync-grab +
   ReplayPointer is the observe path; XTEST still green.
3. **Any display change ≠ Triggered** — coaching `7` while pressing `5`
   must timeout. Display-watch only accepts the coached digit.
4. **Caption-below looked like the target** — plate sat on keypad row
   `4/5/6`. Operator clicked the cyan plate on `5`. Caption now prefers
   above. Compose was never “hardcoded to 5”.
5. **Industry parallel** — [CUA #1931](https://github.com/trycua/cua/pull/1931)
   (2026-06): GTK4 AT-SPI SCREEN-as-WINDOW; correct via X11 origin delta.
   Same physics we measured; we compose from WINDOW + frame extents.

Plan evidence: `docs/plans/coach-c2s-foreign-target-spike.md` §9–§10.
Binary: `foreign-target-probe` in `crates/desktop-spike`.

Ops NEXT remains **C2a** coach-overlay ADR (no daemon until ADR).

## Exact next step

Author **C2a** ADR from these findings: process boundary for
`dpl-coach-overlay`, feature-gate / no-IPC law, paint in WINDOW space on
the target client, Sync-grab observe + semantic invoke, stream identity
gate before paint. Seed: plan §3.4 / §5.

## Landmines still open

- Windows / Wayland unproven for overlay + foreign-target.
- Multi-monitor / fractional scale beyond this single 1920×1080 X11 seat.
- Tier B/C (Blender socket, SolidWorks in-proc, vision) not started —
  named in §3.4 only.
- Live proof depends on the operator being on the **physical** `:1`
  desktop (Cursor UI clicks do not reach this X server).

## Verify commands

```bash
cargo test -p dpl-desktop-spike --lib
foreign-target-probe locate --name 7
foreign-target-probe invoke --name 7
foreign-target-probe spotlight --name 7 --proof-xtest --debug-pointer
# live (physical desktop): click the RING on 7, not the caption
foreign-target-probe spotlight --name 7 --seconds 60 --message "Press 7" --debug-pointer
```

---

## How the really smart teams roadmap this (bleeding-edge, methodical)

This is not “try AT-SPI then maybe screenshots.” Elite desktop-agent /
a11y teams (Playwright’s locator doctrine, CUA’s Linux driver, OS
automation groups at Microsoft/Apple accessibility) share a spine:

### 1. Tier ladder before features

```
Tier A — semantic tree (AT-SPI / UIA / AccessKit)
Tier B — first-party app API (Blender bpy, SolidWorks COM, …)
Tier C — bounded vision (SigLIP / SOM) only when A+B are dark
```

Never start at C. Vision is a **fallback with a refuse path**, not the
coach’s primary eyes. Today’s calculator proof is pure Tier A + X11.

### 2. Coordinate algebra is a typed module with unit tests

One formula, measured constants, refuse on unresolvable origin — never
paint at `(0,0)`. Our `compose_screen_rect` + §2 reconciliation test is
that pattern. CUA’s `gtk4_screen_offset` is the same idea under another
name.

### 3. Identity before paint; postcondition before “success”

- **Before paint:** `GetAccessibleAtPoint` (or equivalent) must name-match.
- **Before Triggered:** hotspot press **or** state change that matches the
  coached control — never “any pixel changed” / “any text changed”.
- **Actuation ladder:** semantic `doAction` → synthetic pointer → portal /
  libei — prove independence of observe vs invoke.

### 4. Adversary matrix every spike

| Attack | Expected |
|---|---|
| Wrong digit | timeout |
| Caption chrome clickable | must not be the hotspot |
| Unattended XTEST | Triggered |
| Dead bus / missing window | refuse by name |
| GTK4 SCREEN extents | ignored |

One green happy path is not a pass.

### 5. Roadmap as named gates, not a feature list

From this repo’s own C2s §3.4 (keep order unless redirected):

| Gate | Deliverable | Unlocks |
|---|---|---|
| **C2s** | Spike: locate / compose / paint / observe / invoke | **CLOSED** (live) |
| **C2a** | ADR: overlay process boundary + laws | permission to write daemon |
| **C2b** | `dpl-coach-overlay` self-test | foreign coach in-process shape |
| **C3** | Foreign → same `SemanticNode` / Locator | one agent surface |
| **C4** | Actuation ladder (semantic → XTEST → libei) | reliable do |
| **C5** | `AppAdapter` + first Tier B pack | Blender-class apps |
| **C6** | Capability packs + workflow induction | library of lessons |
| **C7** | Windows / SolidWorks lane | cross-OS |
| **C8** | Bounded vision fallback | dark trees only |
| **C9** | Lesson product | operator-facing teach |

### 6. Lab hygiene that makes bleeding-edge survivable

- **One spike binary**, not product wiring (`desktop-spike` stays orphan).
- **Linux-gate** heavy deps so Windows green stays honest.
- **Journal + plan §evidence** with commands a stranger can re-run.
- **Physical seat vs IDE seat** called out — remote IDE clicks ≠ X11.
- **Read industry diffs** (CUA, at-spi2-core, GTK frame-extents MRs)
  before inventing a third coordinate system.

### 7. What we deliberately do *not* do yet

SigLIP / screenshot gates for calculator coach. Kernel/eBPF. Generic
command runners. Credential paths. Daemon code before C2a ADR.

That restraint is the method.
