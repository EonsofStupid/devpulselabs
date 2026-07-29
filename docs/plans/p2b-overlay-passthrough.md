# Pass — p2b overlay mouse-passthrough proof

Status: CLOSED 2026-07-29 — PASS (control pair). Extends
`docs/plans/lesson-overlay-spike.md` §7/§8 (transparency was proven;
passthrough was deferred; now §8.1).

## 1. Intent

Decide empirically: with `with_mouse_passthrough(true)` on a transparent
always-on-top overlay, does a synthetic click reach a witness window
*under* the overlay?

C2b cannot start without this answer. Probe only — no product chrome,
no `dpl-coach-overlay` daemon.

## 2. In / out / blast radius

**In:** `crates/desktop-spike` only — new `passthrough-probe` binary +
plan §evidence; ops SESSION / remaining-work / STATE tip / journal.

**Out:** coach-overlay ADR, AT-SPI, app crate changes, Windows/Wayland
re-runs (record as unproven).

**Blast radius:** orphan spike crate; may add `x11rb` as a spike-only
dep for XTEST (already transitive via eframe). Never a dep of `app`.

## 3. Design

One binary, three roles (same pattern as overlay-probe self-control):

| Role | Job |
|---|---|
| `witness` | Opaque always-on-top? No — **below** overlay: normal window with a large hit target; on primary click writes one line to `--hit-file` and exits |
| `overlay` | Transparent + optional passthrough, same geometry, always-on-top, self-terminating |
| (default) orchestrator | Spawns witness → overlay → XTEST click at center → asserts hit-file |

**Control pair (conclusive):**

1. **passthrough ON** → expect hit-file written (PASS if click reached witness)
2. **passthrough OFF** (`--no-passthrough`) → expect hit-file empty (click eaten by overlay)

If both pass, `mouse_passthrough` works. If (1) fails, it does not. If (2)
also hits, the witness is wrong (geometry/stacking).

Click synthesizer: XTEST via `x11rb` (already in the lockfile through
eframe). No apt packages.

## 4. Verification

1. `cargo build -p dpl-desktop-spike --bin passthrough-probe`
2. Run orchestrator twice (passthrough + control) on this X11 box
3. Record stdout + verdict into lesson-overlay-spike.md §8.1
4. Ops NEXT → p4

## 5. Risks

- Headless CI has no DISPLAY — probe is operator/local evidence, not a
  `cargo test` gate (same as overlay-probe).
- Compositor stacking quirks — mitigated by fixed geometry + control pair.
- Always-on-top overlay hazard — both roles honour `--seconds` and exit.

## 6. Done when

§8.1 documents PASS or FAIL with the control pair; remaining-work marks
p2b CLOSED either way (a negative result is still the deliverable).

## 7. Evidence

Live on this box 2026-07-29 (`DISPLAY=:1`, NVIDIA GB10 / Vulkan):

| Run | Flags | Expect | Result |
|---|---|---|---|
| ON | transparent + passthrough | HIT | PASS |
| OFF | `--no-passthrough` (forces opaque overlay) | no HIT | PASS |

Hollow transparent + no-passthrough was *not* a valid negative control
here (click still reached the witness). Documented in spike §8.1.

Ops NEXT after land → **p4** (rail accordion degrade).
