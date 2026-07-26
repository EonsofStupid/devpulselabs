# macOS parity — the bounded checklist (Win+Linux now, Mac later)

Status: STRUCTURED (2026-07-26). Decision (operator): full cross-platform
support IS required. Ship **Windows + Linux fully now** (both gated in CI:
`green-windows` + `green-linux`), keep the **discipline** so Mac stays cheap,
and do the actual macOS implementation **later on real Mac hardware** (the
operator's MacBook Air, or a GitHub-hosted macOS runner) — it cannot be
compiled or verified from the current Linux/aarch64 box.

## Why Mac is moderate, not a rewrite

The app already runs on cross-platform foundations that support macOS:
- **egui/eframe** — the entire UI.
- **`xcap`** — screen/window capture (Win/Mac/Linux).
- **`rdev`** — the global input hook (Win/Mac/Linux).
- **ffmpeg** — a subprocess, never a crate dep (ADR-009) — OS-agnostic.

So most of the app is already portable. The gaps are a small set of explicit
Windows-only edges that need a macOS arm.

## The gap checklist (the "later" pass = implement these)

1. **`crates/engine-capture` — the two `#[cfg(target_os = "windows")]` blocks**
   (`src/lib.rs:1115`, `:1173`). Audit each: does `xcap`'s macOS path already
   cover it, or is a `#[cfg(target_os = "macos")]` arm needed? These are the
   most likely real work (window enumeration / DPI / capture specifics).
2. **`crates/proc` — quiet spawn** (`#[cfg(windows)]` +
   `std::os::windows::process::CommandExt`, the CREATE_NO_WINDOW flag).
   Confirm/round out the non-Windows arm (Mac has no console-window concept —
   likely a no-op, but must compile).
3. **`crates/engine-steps` — the `rdev` global hook.** `rdev` supports macOS
   but requires **Accessibility permission** (a first-run OS prompt) and has
   Mac-specific event quirks; needs handling + testing on a Mac.
4. **The no-activate REC pill** (Windows `WS_EX_NOACTIVATE`, always-on-top
   non-focus-stealing). macOS equivalent: an `NSPanel` with the
   `nonactivatingPanel` style. This is the sharpest UI-FFI edge.

## Discipline NOW (cheap; prevents a painful retrofit)

- **Every `#[cfg(windows)]` gets a sibling `#[cfg(target_os = "macos")]` (or
  `#[cfg(unix)]`) arm** — even a stubbed, honest "unsupported on macOS yet".
  Reviewers reject a bare Windows-only gate. (The Linux gate already caught
  two Windows-first bugs this way — Mac would catch more.)
- W9 and the current feature work are naturally cross-platform (pure paths +
  serde + the shared crates) — no new Windows-isms.

## Verification (the "later" trigger)

Needs a Mac: the operator's MacBook Air, or a GitHub-hosted `macos-latest`
runner. When one exists: add a `devpulse-mac` eguiPilot profile + a
`green-mac` job in `green.yml` — then macOS is gated exactly like Win/Linux
and this checklist is enforced, not aspirational.

## Mac-instance bootstrap (for a Claude Code instance on the MacBook Air)

The operator runs a SEPARATE instance on the Mac to do the macOS
implementation in parallel; this repo is the shared foundation. On the Mac:

1. `git clone https://github.com/EonsofStupid/devpulselabs && cd devpulselabs`
   then `git config core.hooksPath .githooks` (the gate hook).
2. Read THIS file — it is the work-list. Start with `cargo build` and let the
   compiler surface the real gaps (most of the app is already portable — it
   rides egui/xcap/rdev/ffmpeg, all macOS-capable, and the non-Windows paths
   are already Linux-verified in CI, so they should mostly compile on macOS).
3. Register the Mac as a self-hosted runner labeled `dpl-mac` (or use a
   hosted `macos-latest`), then UNCOMMENT the `green-mac` job in
   `.github/workflows/green.yml` and the `[profiles.devpulse-mac]` block in
   `.eguipilotai.toml` (both staged there, ready).
4. Work the gap checklist (§ above), gating each fix through `cargo green` on
   the Mac. Branch → PR → `green-mac` → merge, same discipline as Win/Linux.

This instance (Linux/aarch64) stays on Windows+Linux + features; it will NOT
touch the macOS FFI bodies (it can't verify them). Division: Win/Linux + the
portable structure here; macOS-only implementations there.
