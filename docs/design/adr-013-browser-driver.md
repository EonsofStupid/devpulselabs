# ADR-013 — The headless-browser driver (browser as subprocess, CDP)

| Field | Value |
|-------|-------|
| **Status** | ACCEPTED (operator commission, 2026-07-25: "build the headless chrome inside devpulselabs please, it will be needed") |
| **Owner crate** | `crates/engine-browser` (`dpl-engine-browser`) |
| **Demanded by** | Program DEMO external tier (roadmap: "The browser DRIVER needs its own ADR … planned properly, not smuggled") |

## Decision

1. **The browser is a SUBPROCESS, never a crate dep** — the ADR-009
   pattern ffmpeg and aerender already follow. No embedded webview, no
   CEF binding, no `headless_chrome`-style crate owning the child.
2. **Resolve, never download.** The driver locates an installed
   chromium-family binary from an ordered candidate list (injected
   path → `DPL_BROWSER` env → `bin/` beside the exe → standard Chrome
   installs → **Edge on Windows** (always present — the zero-install
   fallback) → PATH names incl. the Linux set). A miss is a loud error
   naming every checked location. Same law as managed ffmpeg (W6).
3. **Protocol: CDP over a localhost WebSocket.** Launch with
   `--headless=new --remote-debugging-port=0` and a **fresh temp
   profile**; read the port from the `DevToolsActivePort` file (never
   guess ports, never race a fixed port). We speak the DevTools
   protocol directly — flat-session Target attach, Page/Runtime/Input/
   Emulation domains.
4. **Sync, threadless, ours.** The client is blocking (engine-crate
   contract: callers run it off the UI thread), id-matched
   request/response with events drained to a queue. WebSocket framing
   via `tungstenite` (pure Rust, `default-features = false` — no TLS:
   the socket never leaves localhost). JSON via serde_json. No tokio,
   no threads.
5. **Ephemeral profiles by default; persistent DEMO profiles opt-in
   (DG-B.1, amended same-day — operator: the flow must be "properly
   all included, portable").** Default launches get a fresh temp
   `user-data-dir`, removed on close. `BrowserConfig.profile:
   Some(dir)` names a persistent profile the APP owns — the operator
   signs demo accounts in ONCE through the visible-window ceremony
   (`browser_cli login <dir>`), and captures ride the saved session.
   Persistent profiles are NEVER deleted by the driver and are never
   the operator's real browser profile, cookies, or credential
   stores. Credentials are typed by the operator only — tooling never
   retrieves or enters them. Authenticated demo captures happen in
   demo accounts (G-6 redaction law, docs/plans/demo-guide-pipeline.md).

## Rejected

- **Embedding (CEF/webview)** — a C-dep monster against the pure-Rust
  law, and a second render stack inside an egui app.
- **`chromiumoxide`/`headless_chrome` crates** — async-runtime or
  ownership models that fight the sync engine contract; we own both
  ends of a localhost socket (the same reasoning that rejected
  protobuf, ADR-WF2-001).
- **Download-on-demand browser** — supply-chain surface + gigabytes,
  for a machine class (Windows) that ships Edge anyway.
- **Driving the operator's Chrome via extension** — stays available
  interactively (Claude-in-Chrome), but the product driver must be
  headless, extensionless, and CI-runnable.

## Consequences

- Demo captures of external tools run unattended (DG2 composes this
  with the recorders; the pilot drives the app tier, this drives the
  web tier).
- Screencast → replay-clip capture is an additive follow-up on the
  same seam (frames → engine-media, which already owns encode).
- The gate never assumes a browser: live coverage is an `#[ignore]`
  drill, unit coverage is pure (args, parsers, framing).
