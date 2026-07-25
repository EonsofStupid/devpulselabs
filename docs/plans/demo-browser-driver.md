# DEMO-B — the headless-browser driver (`dpl-engine-browser`)

Pass plan per the plan-before-code law. Operator commission (2026-07-25):
*"build the headless chrome inside devpulselabs please, it will be
needed."* This is the browser driver the roadmap's Program DEMO external
tier demanded be "planned properly, not smuggled" — planned here, with
its ADR at `docs/design/adr-013-browser-driver.md`.

## §1 Context

Program DEMO's external tier demonstrates OTHER tools (first target:
the Twilio console walkthrough, `docs/plans/demos/
demo-001-twilio-notification-keys.md`) and captures them into guides.
That needs a browser the app/agent can drive deterministically and
headlessly: navigate, evaluate, click, type, screenshot. The interim
Claude-in-Chrome path drives the OPERATOR's browser; this crate gives
devpulselabs its OWN driver — no extension, no operator session,
CI-runnable.

## §2 Current state (ground-truthed this session)

- No browser code exists anywhere in the workspace.
- `Cargo.toml:3-55` — explicit `members` list (no glob); new crates
  must be added by hand. Working tree note: the uncommitted
  `feat/lottie-1-spike` edits to `Cargo.toml`/`Cargo.lock` (member
  `crates/lottie` + `rasterlottie` + `dpl-lottie` lines) are parked to
  scratchpad and restored after this pass lands (§7).
- `crates/app/src/main.rs:895-958` — the managed-binary law (W6):
  resolve ONE absolute path from an ordered candidate list
  (bundled → PATH → winget), inject via `set_*_path`, loud warn naming
  every checked location on miss. This crate mirrors it.
- `crates/engine-media/src/lib.rs:29-42,280-290` — engine-crate house
  style: sync/blocking, `OnceLock<PathBuf>` injection seam, args built
  as `Vec<String>` (never shell strings), pure arg-builders with unit
  tests asserting verbatim args.
- `crates/engine-ae/src/lib.rs:1-16` — the subprocess-orchestrator
  precedent (aerender): locate install, argument VECTOR, parse output,
  everything unit-provable without the real binary; live proof gated
  separately.
- `crates/composer/tests/import_drill.rs` — the ignored-drill pattern
  for tests needing real machine state.
- `.cargo/config.toml` — `cargo green` = xtask (fmt-check +
  clippy -D warnings + tests + release build + exe-mtime assert).
- Workspace deps (`Cargo.toml:72-104`): serde/serde_json present; NO
  websocket client, NO base64. tokio is sync-feature-only — the async
  runtime is NOT available to engine crates.

## §3 Target design

New crate `crates/engine-browser` (`dpl-engine-browser`): headless
Chromium orchestration, browser as SUBPROCESS (ADR-013, the
ffmpeg/aerender pattern), driven over the Chrome DevTools Protocol on a
localhost WebSocket. Sync/blocking like every engine crate.

- **Resolve, never download**: candidates in order — ① injected path
  (`set_browser_path`, the app-shell seam) ② `DPL_BROWSER` env ③
  `bin/chrome[.exe]` beside the exe (portable bundle) ④ standard
  Chrome installs ⑤ **Edge** (`msedge.exe`, chromium — always present
  on Windows, the zero-install fallback) ⑥ PATH names (incl. Linux
  `google-chrome`/`chromium`/`chromium-browser` — the Wayland mini-PC
  lane is a product requirement). Miss → error naming every location.
- **Launch**: `--headless=new --remote-debugging-port=0
  --user-data-dir=<fresh temp profile> --no-first-run
  --no-default-browser-check --disable-sync --disable-extensions
  --window-size=W,H about:blank`. Port 0 → Chrome writes
  `DevToolsActivePort` (line 1 port, line 2 ws path) in the profile
  dir; we poll for it (10 s deadline), never guess ports.
- **CDP client**: `tungstenite` (pure Rust, `default-features=false` +
  `handshake`) over our own `TcpStream` (so read timeouts stay ours);
  JSON via serde_json. One command = id-matched request/response;
  events drain into a `VecDeque` en route. No threads, no async.
- **Surface (this pass)**: `Browser::launch(config)` →
  `new_page` (Target.createTarget + attachToTarget flatten +
  Page/Runtime.enable) · `navigate` (Page.navigate + wait
  Page.loadEventFired) · `evaluate` (Runtime.evaluate returnByValue) ·
  `screenshot_png` (Page.captureScreenshot → hand-rolled base64 decode
  ~25 lines, no new dep) · `click`/`insert_text` (Input domain) ·
  `set_viewport` (Emulation) · `close` (Browser.close, kill fallback,
  best-effort profile cleanup; `Drop` kills).
- **CLI** `src/bin/browser_cli.rs` (the `capture_cli` pattern):
  `browser_cli shot <url> <out.png> [--window WxH]` — manual proof +
  immediate demo-capture utility.
- Scale story: DG2's "pilot drives while recorders capture" composes
  this crate + engine-capture/engine-media; screencast-to-clip is the
  next pass on this seam, additive only.

## §4 Changes, in execution order

1. **Park the lottie edits**: copy dirty `Cargo.toml`+`Cargo.lock` to
   scratchpad; `git checkout -- Cargo.toml Cargo.lock`;
   `git switch -c feat/demo-browser-driver` (HEAD == main == 39ae7a4,
   verified).
2. **Edit `Cargo.toml`**: member `"crates/engine-browser"` after
   `"crates/engine-ae"`; workspace dep `tungstenite = { version =
   "0.27", default-features = false, features = ["handshake"] }` (with
   ratification comment; version adjusted to what resolves, recorded
   here) after `sysinfo`; `dpl-engine-browser = { path =
   "crates/engine-browser" }` after `dpl-engine-ae`.
3. **Create `crates/engine-browser/Cargo.toml`**: workspace
   version/edition/publish/lints; deps serde, serde_json, tungstenite.
4. **Create `crates/engine-browser/src/lib.rs`**: doc-comment carrying
   ADR-013; `resolve` module (candidates + first-exists +
   `set_browser_path` seam), pure `launch_args`, `DevToolsActivePort`
   parser, base64 decoder, CDP client, `Browser`/`Page` API per §3.
   Unit tests: args verbatim, port-file parse (happy + malformed),
   base64 round-trips, candidate ordering (env override first),
   error text names every candidate.
5. **Create `crates/engine-browser/src/bin/browser_cli.rs`** per §3.
6. **Create `crates/engine-browser/tests/browser_drill.rs`** —
   `#[ignore]` live drill: resolve (fail loudly if none), launch
   800×600, navigate `data:text/html,<h1 id=t>ok</h1>`, evaluate
   `document.getElementById('t').textContent` == "ok", screenshot →
   PNG magic + >1 KB, close; child gone.
7. **Docs**: this file; `docs/design/adr-013-browser-driver.md`;
   `docs/plans/demo-guide-pipeline.md` gains the DG-B row;
   `docs/STATE.md` tip updated.

## §5 Boundaries

- 🚫 `/content` — untouched, as ever.
- 🚫 `crates/lottie` + its Cargo lines — another pass's uncommitted
  work; parked and restored byte-identical, never committed here.
- 🚫 No `#[allow]`, no async runtime in the crate, no browser
  download-on-demand (resolve only), no TLS/`native-tls` features.
- ⚠️ ask first: wiring `init_browser()` into `app/main.rs` (deferred —
  nothing consumes it yet); screencast/replay capture (next pass).
- Out of scope: DG2 composition, pilot integration, Twilio capture
  itself.
- **AMENDED same-day (operator: "properly all included portable"):**
  DG-B.1 folded INTO this pass — `BrowserConfig` gains
  `profile: Option<PathBuf>` (persistent, never deleted) +
  `headless: bool`; `browser_cli login <dir>` is the visible sign-in
  ceremony; `shot --profile <dir>` rides the saved session. ADR-013 §5
  amended to match. No extension dependency anywhere in the flow.

## §6 Verification

1. `cargo green` exits 0 (fmt + clippy -D warnings + full tests +
   release build; release exe mtime moves — the xtask asserts it).
2. `cargo test -p dpl-engine-browser` — unit suite green, drill
   listed ignored.
3. `cargo test -p dpl-engine-browser -- --ignored --nocapture` — the
   LIVE proof on this machine (Chrome and Edge both installed): drill
   passes end-to-end, output shows resolved browser path + screenshot
   byte count.
4. `target\release\browser_cli.exe shot "https://example.com"
   <scratchpad>\example.png --window 1366x768` exits 0; the PNG opens
   and shows the page (attached to the PR).
5. CI: green.yml passes on `dpl-win` (runner up; drill stays ignored
   in CI — no browser assumption in the gate).

## §7 Commit plan

- Branch `feat/demo-browser-driver`; one commit: `Cargo.toml`,
  `Cargo.lock`, `crates/engine-browser/**`, `docs/plans/
  demo-browser-driver.md`, `docs/design/adr-013-browser-driver.md`,
  `docs/plans/demo-guide-pipeline.md`, `docs/plans/demos/
  demo-001-twilio-notification-keys.md`, `docs/STATE.md`.
  Message: `DEMO-B — dpl-engine-browser: headless Chromium over CDP
  (ADR-013)`.
- Push (pre-push hook runs the gate), `gh pr create`, merge after CI,
  pull main, rebuild release exe, verify mtime.
- Restore: `git switch feat/lottie-1-spike`, `git merge main` (ff),
  re-apply the three lottie Cargo lines from the scratchpad backup;
  `Cargo.lock` regenerates on next cargo run.

## §8 Evidence

- `Cargo.toml:3-55,72-104` (members list, dep roster) — read in full.
- `crates/app/src/main.rs:895-958` (`init_ffmpeg`, managed-binary law).
- `crates/engine-media/src/lib.rs` — read in full (injection seam,
  arg-vector style, blocking contract, test idiom).
- `crates/engine-ae/src/lib.rs:1-100` (subprocess orchestrator
  precedent).
- `.cargo/config.toml` (the `green` alias).
- `git rev-parse HEAD main` → both `39ae7a4…` (branch has no commits;
  lottie work is uncommitted only).
