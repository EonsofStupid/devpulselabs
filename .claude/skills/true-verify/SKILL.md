---
name: true-verify
description: True test-and-verification pass on the REAL exe — screenshot every touched route, compare against committed baselines so the UI never regresses, and enforce that slide content is built from registered blocks/templates surfaced in the Library.
---

# true-verify — the real-artifact gate

**Argument (optional):** the routes/surfaces touched this pass (e.g.
`library/components workbench/slides`). No argument → verify every registered route.

## Mission

Law 2: the exe proves itself, or the claim doesn't exist. This skill runs after
`/egui-build` (or any code pass) and produces evidence, not assertions: test output,
fresh screenshots from the real binary, a baseline comparison, and a structural check
that slide content is **blocks and templates in the registry**, not monolith surfaces.
A pass that ends without evidence has not passed.

## Protocol

### 1. Logic gate

- `cargo test --workspace` — expect all green; paste the summary line. A single
  failure stops the pass: diagnose and fix (or bounce back to `/egui-build`).

### 2. Real artifact

- Kill strays first: `taskkill //F //IM devpulselabs.exe` (Bash; `/F /IM` in
  PowerShell) — a running instance locks the exe and the link error gets masked.
- `cargo build`, capture the real exit code, and confirm the exe mtime advanced
  (`target\debug\devpulselabs.exe`) — the mtime check is what catches a masked link
  failure.

### 3. Screenshot sweep — the regression floor

- For each route under test:
  `target\debug\devpulselabs.exe --screenshot proof-<hub>-<sub>.png --route <hub>/<sub>`
  (exit 0 = captured; nonzero = the harness itself is reporting a failure — treat as
  a red test).
- **Read every PNG** with the Read tool. Check for: □ glyph boxes (font coverage),
  blank/black regions (paint order), clipped or overflowing content, chrome seams
  (top/left/right/footer plates + frame_overlay present), and that the surface shows
  what the pass claims it shows.
- **Baseline compare.** Committed baselines live in `docs/proof/` (scratch shots
  `proof-*.png` are gitignored). Compare the fresh shot against the baseline for the
  same route — dimensions and visual content. A visible regression fails the pass.
  When a change is an *intended* visual change, copy the new shot to `docs/proof/`
  and say so in the commit message ("baseline updated: <route> — <why>"). Never
  update a baseline silently, and never update one to make a regression pass.
- If a route under test has no baseline yet, this pass creates it (commit the PNG to
  `docs/proof/`).

### 4. Gallery gate + blocks/templates law

- **Gallery gate:** everything generated or registered must render on a surface.
  Ramps → the Library pool wall (all `HUE_COUNT` hues, all 13 steps). New
  widgets/components → a Library gallery entry. If it exists and nothing renders it,
  that is a FAIL — the "18 generated, 3 used" recurrence this gate exists to kill.
- **Slides are compositions, not monoliths.** Every slide must be built from
  **registered blocks and templates**: reusable units implementing the registry
  contract (spec + render), enumerable by the Library, colored from the pool, sized by
  `Fluid::of`. Grep the slide code for the violations: hardcoded hex/rgb, one-off
  giant `fn ui` surfaces that duplicate paint logic instead of composing blocks,
  content that exists in code but appears in no gallery/library listing. Each
  violation is a finding — fix small ones in-pass, plan large ones via
  `/port-capture`.
- Confirm the Library's counts are honest: whatever the census says exists
  (hues × steps, template count, block count) must match what the wall actually
  renders.

### 5. Report

End with the evidence table: test summary line, per-route shot → verdict
(clean / regression / baseline-updated), gallery-gate findings, blocks/templates
findings. Pass = all green with evidence attached. Anything else = the list of reds
and who owns them (`/egui-build` for execution bugs, `/port-capture` for missing
capability). Never report "looks right" — report what the artifact showed.
