# External-reach inventory — everything this repo reaches outside itself for

**Status:** RECOMMENDATIONS ADOPTED, each revisitable — 2026-10-10 (operator,
asked for answers: *"what the fuck whatever fine"*). Rows are acted on only
inside the pass that owns them; rows whose recommendation is "Decide" stay
open until a pass needs them, then come back as a named question.
Authored 2026-10-10.
**Rule (operator, 2026-10-10):** *"every fucking thing you are reaching outside
of this repo for better be identified and I will confirm if wrong like chat"*.

Nothing below is relied on for new work until its row is marked **CONFIRMED**.
A row marked **REJECTED** becomes a removal item. Every row is ground-truthed
against `main` (`14d083d`) plus the uncommitted tree on 2026-10-10; "branch"
means the unmerged `origin/feat/providers-carousel`.

Answer by number: e.g. "confirm 11, 14; reject 1, 2, 3; 4 = D:\localdev\devpulse_alpha".

---

## A · Other repos / projects (dependencies or borrowed designs)

| # | Reach | Evidence | Today | Recommendation |
|---|---|---|---|---|
| 1 | **chatmaton** (formerly `automaton`, `mato-*` crates) as the AI "kernel" DevPULSE would depend on via git | `docs/plans/unified-ai-kernel-automaton-merge.md` (Decisions A/B/C), `docs/journal/2026-08-11-unified-ai-kernel.md` | Docs only — no Cargo dep, no code | **Reject.** Mark both docs SUPERSEDED by the native chat plan. |
| 2 | **Shippin** control plane / entitlement contract; Shippin `NAMING-AND-SSOT` "seat" definition | merge plan §5.4, §6; branch `crates/seat/src/lib.rs:10,19,77` | Docs on main; code on branch | **Reject.** |
| 3 | Branch `feat/providers-carousel` (23 commits): `crates/seat`, `crates/ai-core`, kernel-boundary gate, chat-as-coworkspace, brainstorm-per-seat | `git log origin/main..origin/feat/providers-carousel` | Unmerged | **Do not merge.** Anything worth keeping is re-planned under the native chat plan, by name, with your sign-off. |
| 4 | The **old app** as read-only reference and source of law (ADR-009/010, signalkind) | `CLAUDE.md:33`, `AGENTS.md:60,81`, `.claude/skills/port-capture`, `slide-catalog-pass`, ~15 plans | Docs say `C:\localdev\devpulse labs` / `D:/localdev/devpulse labs` — neither exists. **Found by exact name at `D:\localdev\review-archive\devpulse labs`** (full TS/TanStack/Tauri source + `.git`, HEAD `98babf4`; skills wrongly call it "build output only"). Other look-alikes never named in docs: `devpulse_alpha`, `DEVPULSE-ALPHA-2026`, `platform_devpulse` | **Confirm** the review-archive path; fix CLAUDE.md, AGENTS.md and the skills. |
| 5 | **Clyffy** as an in-app persona: pinned "house session" chat row, provider id `"clyffy"` | `crates/app/src/chat.rs:164,1642-1664,2359-2435`; `crates/chat/window/src/lib.rs:163,701-712,2469-2524`; `crates/chat/profiles/src/lib.rs:17,181`; `providers-presence-and-the-brain.md:222-269` | In app; inert ("sends arrive with the next slice") | **Decide:** is Clyffy part of DevPULSE chat? |
| 6 | "DevPulse platform" capture-share spec: forks `EonsofStupid/{str0m,scap,cortex}`, Qortex, Council, Turso control layer | `docs/design/wiredfront-capture-share-spec.md:1-40,104-111,238-247` | DRAFT, "do not implement" | **Decide** (likely park). |
| 7 | A2UI / AG-UI message vocabulary (Google, CopilotKit) | `docs/plans/operator-coach-a2ui.md:19-25`; `brainstorm-and-clyffy-drive-capture.md:38-64` | Vocabulary only | **Decide.** |
| 8 | ComfyUI node-graph concepts | `docs/plans/wiredfront-workflows-program.md:158-169`; `adr-010-workbench-assembly.md` | Program | Confirm (recorded as your directive). |
| 9 | Vendored INTO the repo: ColdLight tokens (from the prior repo), Lucide 1.27.0, 13 OFL fonts, Phosphor | `crates/coldlight/build.rs:4-6`; `assets/lucide/`, `assets/fonts/` | In-repo, embedded | Confirm (no runtime reach). |
| 10 | Vendored Codex app-server JSON schema (`codex-cli 0.146.0-alpha.3.1`) | `crates/chat/providers/codex/schema/` | Test-only | Confirm. Note the version gate rejects 0.146 (see 11). |

## B · External programs the app launches

| # | Reach | Evidence | How it is found today | Recommendation |
|---|---|---|---|---|
| 11 | **codex** CLI | runtime/activation `codex/src/activation.rs:538-634`; presence `app/src/presence.rs:47` | Runtime: depot (SHA-verified). **Presence + sign-in: bare name on PATH, full env.** Version gate `>=0.145,<0.146` rejects 0.146 | Confirm as a subprocess; one resolution (depot) for every use. |
| 12 | **grok** CLI (ACP `agent stdio`) | `grok/src/lib.rs:568-731`; `grok/src/runtime.rs:98-139`; presence | Runtime: depot. Presence: PATH | Same as 11. |
| 13 | **grok agent → any program** it names (ACP `terminal/create`) | `grok/src/terminal.rs:74-90` | Agent-chosen command from PATH; env cleared to an allow-list, plus agent-supplied vars; cwd jailed | **Decide:** allow, gate behind approval, or disable. |
| 14 | **claude**, **cursor-agent**, **gemini** CLIs | specs in `providers/{claude,cursor,gemini}/src/lib.rs`; `presence.rs:47,165` | PATH only (presence + sign-in); no runtime | **Decide** which providers chat must support (feeds the chat plan). |
| 15 | **ffmpeg** | `engine-capture`, `engine-media`, `video`; injected `app/src/main.rs:1571-1592` | Depot; never PATH | Confirm. |
| 16 | **pdftoppm** (Poppler) | `composer/src/import.rs:91-99` | Depot, **falls back to PATH** | Confirm; remove the PATH fallback. |
| 17 | **dpl-lottie-render** (our own helper, ThorVG) | `lottie/src/lib.rs:56-68` | env `DPL_LOTTIE_RENDER` → beside exe → **PATH** | Confirm; drop PATH. |
| 18 | **dpl-coach-overlay** (our own helper) | `hubs/coach/src/lib.rs:72-90` | env → beside exe → **PATH** | Confirm; drop PATH. |
| 19 | **Chrome / Edge / Chromium** | `engine-browser/src/lib.rs:52-120` | System installs, ProgramFiles, PATH, `/snap` — **never the depot's chromium**. Not linked into the app (browser-cli + tests) | **Decide:** depot-only, or system browser allowed. |
| 20 | **After Effects** `aerender` / `AfterFX` | `engine-ae/src/lib.rs:145-193` | Located in ProgramFiles; never spawned; not in app | Decide (park?). |
| 21 | Linux coach: `xdotool`, `kill`, `.desktop` `Exec=` apps, hardcoded `gnome-calculator` fallback | `coach-runtime/src/linux_interact.rs:29,158`; `linux_lifecycle.rs:28-30,65,278-280` | PATH; linked into the app on Linux | Decide. |
| 22 | **Placebo spawns** (uncommitted 10-09 work): bare `ffmpeg` / `dpl-lottie-render` with no args on every "activate"; `cmd /C exit 0` fallback | `crates/proc/src/process_manager.rs:121,193,239-246` | **DONE in C0:** parked on the local-only branch `park/codex-subprocess-lifecycle-2026-10-09` (`d063e87`); `main` never had it | Rejected. |

## C · Vendor-owned and user locations read or written

| # | Reach | Evidence | Today | Recommendation |
|---|---|---|---|---|
| 23 | Read `~/.codex/sessions` and `~/.claude/projects` at boot (history catalog) | `app/src/main.rs:1486-1530` | Read-only, jailed, size-limited | Decide (feeds chat plan). |
| 24 | `--ai-backfill` reads the same via `HOME` only | `app/src/main.rs:32-38` | **Finds nothing on Windows** (catalog uses `USERPROFILE`) | Fix or remove. |
| 25 | Provider CLI homes reached through env passthrough (`CODEX_HOME`, `GROK_HOME`, `APPDATA`, `USERPROFILE`, …) | `codex/src/activation.rs:636-670`; `grok/src/lib.rs:695-731` | Auth stays in the vendor's profile | Confirm (the app never holds the login). |
| 26 | `eguipilotai install-agent` **writes** into `~/.codex`, `~/.claude`, `~/.cursor` | `egui-pilot-ai-cli/src/main.rs:360-398` | CLI only | Decide. |
| 27 | `engine-ae` rewrites Adobe prefs in `%APPDATA%` (with backup) | `engine-ae/src/lib.rs:319-395` | Not in app | Decide (park?). |
| 28 | Composer Browse roots: every drive letter, `~/Videos`, `~/Downloads`, …; reference-mode imports create **symlinks** to outside files | `composer/src/browse.rs:225-300`; `composer/src/import.rs:185-231` | In app | Confirm or restrict. |
| 29 | Content/asset root search walks **up to 4 parent dirs, then CWD** | `engine-capture/src/lib.rs:196-229`; `video/src/lib.rs:324-365` | Can resolve outside the portable folder | **Fix:** beside-exe or explicit flag only. |
| 30 | Provider working directory and file-jail root = **process CWD** | `app/src/main.rs:990` | Varies with how the app is launched | Fix in the chat plan (explicit project root). |
| 31 | OS temp dir used (`dpl-comp-render-*`, `dpl-viziwiz-freeze.png`) despite the `.userdata/temp` rule | `composer/src/lib.rs:5782`; `viziwiz/src/editor.rs:50` | In app | Fix. |

## D · Network

| # | Reach | Evidence | Today | Recommendation |
|---|---|---|---|---|
| 32 | `https://api.x.ai/v1/images/generations` and `/v1/chat/completions` with `XAI_API_KEY` (imagine + brainstorm suggestions) | `chat/imagine/src/lib.rs:17-29,267-385`; ADR-011 | In app (ureq + rustls) — contradicts `README.md:25` "never holds an API key" | **Decide.** |
| 33 | Agent seam HTTP server **127.0.0.1:7341, on by default, no auth**, POST `/command`, `/comp` | `seam/src/lib.rs:504-509,993`; `app/src/main.rs:396-407` | In app | **Decide:** default off / token / keep. |
| 34 | Coach daemon 127.0.0.1:7843 | `coach-daemon/src/main.rs:150-231` | Separate binary | Decide. |
| 35 | System default browser opened for device login (`accounts.x.ai`) and setup docs | `chat/browser-auth/src/lib.rs:16,69`; `app/src/chat.rs:1475-1482` | In app | Confirm. |

## E · OS integration inside the app

| # | Reach | Evidence | Recommendation |
|---|---|---|---|
| 36 | **System-wide input hook** (`rdev::listen`) installed on the first ViziWiz frame, plus a synthetic Ctrl+Shift+9 (`rdev::simulate`) | `engine-steps/src/lib.rs:529-600`; `viziwiz/src/lib.rs:1063-1078`; `app/src/main.rs:1800,1816` | Decide. |
| 37 | Screen capture (xcap/DXGI) and audio devices (ffmpeg dshow/pulse) | `engine-capture/src/lib.rs` | Confirm (core feature). |
| 38 | AT-SPI / X11 (Linux coach crates linked into the app) | coach-inventory/fabric/runtime | Decide. |

## F · Heavy engines running in-process (vs. "deps as subprocesses")

| # | Engine | Pulled by | Recommendation |
|---|---|---|---|
| 39 | turso 0.7 (SQLite-compatible + tantivy + mimalloc; **sync-engine crates in the graph**) | `dpl-index`, `chat/accounts`, `chat/profiles` | Decide. |
| 40 | fjall (estate), parquet/arrow (ledger) | `dpl-estate`, `dpl-ledger` | Decide. |
| 41 | rav1e (AVIF), libwebp (C) | `engine-media`, `engine-capture` | Decide (ffmpeg could own encoding). |
| 42 | agent-client-protocol + async stack (~99 crates) | `dpl-chat-grok` only | Decided by the chat plan. |
| 43 | ureq + rustls + ring | `dpl-chat-imagine` only | Follows 32. |

## G · Build, CI, depot, infrastructure

| # | Reach | Evidence | Recommendation |
|---|---|---|---|
| 44 | Estate runners: Proxmox VM 119 `dpl-win`, LXC 121 `dpl-linux` on **server1** (`ssh.clyffy.ai`) | `.github/workflows/green.yml:64,118`; `docs/runbook/ci-runners.md` | Confirm (both online 2026-10-10). |
| 45 | **Nightly runs on GitHub-HOSTED runners** (`windows-latest`, `ubuntu-latest`, `ubuntu-24.04-arm`) — metered on a private repo | `.github/workflows/nightly.yml:29,97,184` | Decide (move to estate or keep). |
| 46 | Tool depot source: `C:\localdev\dptools-deps` and `sources.local.toml` | `dptools-deps/README.md:20`; `sources.example.toml` (`C:/private-tools/...`) | **RESOLVED in C0:** the depot is at `D:\projects\dptools-deps` (row 56). Staged into `target/release/dptools-deps`; the exe logs `verified embedded ffmpeg/pdftoppm resolved`. |
| 47 | sccache `D:\sccache` (absent) and `/root/.cache/sccache` on server1 | `docs/runbook/sccache-shared-build-cache.md` | Decide. |
| 48 | **Repo visibility contradiction**: CLAUDE.md says PRIVATE (2026-08-07); README, product-vision, nightly.yml, release-contract §5.4 say PUBLIC | `CLAUDE.md:80-91`; `README.md:148-170`; `release-contract.md` §5.4 | Confirm PRIVATE; fix the docs. |
| 49 | EguiPilot CI Dockerfile pins Rust 1.92.0; repo pins 1.97.1 | `ci/`; `rust-toolchain.toml` | Fix. |

## H · Agent tooling and stale paths

| # | Reach | Evidence | Recommendation |
|---|---|---|---|
| 50 | **SessionStart hook injects an outside transcript**: `.claude/ledger/last-session.txt` points at a **GitHub Copilot** session file under `C:\Users\jessa\.copilot\session-state\…` | `.claude/settings.json`; `.claude/ledger/session-start.sh:34-49` | **Decide** — this shapes every new session. |
| 51 | Codex sandbox repair notes, probe file, and a second plan ledger `.plan/` (untracked) | `docs/plans/2026-10-08-codex-sandbox-repair.md`; `docs/proof/codex-sandbox-probe-*.txt`; `.plan/` | Reject (not product); remove with 22. |
| 52 | Agent definitions/skills naming external models and missing skills (`clyffy-plan`, `clyffy-verify` not present) | `.claude/agents/*.md`; `.cursor/agents/*.md`; `.claude/skills/session-os/SKILL.md:93` | Decide. |
| 53 | **Stale absolute paths**: the checkout is `D:\localdev\devpulselabs`, but docs say `C:\localdev\devpulselabs`; sibling worktrees `C:\localdev\{dpl-main,devpulselabs-fx,dpl-lane-lucide,automaton}`; `/home/hades/…`, `/home/wardenop/…`, `/root/dev/devpulselabs` | `CLAUDE.md:11`; `AGENTS.md:81`; ~12 plans; journals | Fix CLAUDE.md/AGENTS.md now; historical journals stay as history. |
| 54 | Tests that assert on machine-local vendor history (`~/.claude/projects`, `~/.codex/sessions`) | `tests/eguipilotai/chat-local-histories.toml:13`; `platform-sees-providers.toml:9` | Decide (feeds chat plan). |

## I · Found during C0 (2026-10-10, drive-wide search)

| # | Reach | Evidence | Recommendation |
|---|---|---|---|
| 55 | **A second clone of this repo at `D:\projects\devpulselabs`.** Same origin, on `main` at `14d083d`. Its exe was built **Oct 2**, newer than this clone's Sep 27 exe before C0. It has its own staged depot and a `sources.local.toml` still pointing at the dead `C:/localdev/dptools-deps`. CLAUDE.md says "There is no other worktree"; that is false. | `D:\projects\devpulselabs\target\release\devpulselabs.exe`; `…\dptools-deps\sources.local.toml` | **RESOLVED 2026-10-10 — operator: `D:\projects\devpulselabs` IS THE checkout.** It holds the operator's library (`content/`, 62 files / 150 MB). The `D:\localdev\devpulselabs` clone (empty library) is not used; C0's first PR was authored there and is corrected here. This clone's `sources.local.toml` now points at `D:/projects/dptools-deps`. |
| 56 | **The operator's Central Supply Depot is at `D:\projects\dptools-deps`** (`artifacts/windows-x86_64/`: chromium 150.0.7871.129, codex 0.145.0-alpha.18, ffmpeg 8.1.2, gemini 0.54.4, grok 0.2.112, pdftoppm 26.02.0). It moved from `C:\localdev\dptools-deps`. | this clone's `dptools-deps/sources.local.toml` (gitignored) now points here; staged into `target/release/dptools-deps` and SHA-verified | Confirm. Resolves row 46. |
| 57 | gemini 0.54.4 in the depot is a **Node.js bundle** (`bundle/*.js`). Running it means a Node runtime, which is not a DevPULSE tool id. | `D:\projects\dptools-deps\artifacts\windows-x86_64\gemini\0.54.4\` | Decide in chat pass C5 (bundle Node in the depot, or not). |
| 58 | Another copy of the old app at `D:\projects\devpulse labs` (origin `EonsofStupid/vibe-flow`, HEAD `98babf4`, the same commit as the review-archive copy in row 4). | `git -C "D:\projects\devpulse labs" log -1` | Decide which copy is the read-only reference (C0 points docs at the review-archive copy). |
| 59 | Your OpenAI Codex desktop app runs its own `codex app-server` daemons (0.162.1, `~/.codex/packages/…`). These are separate from DevPULSE's depot codex. "ChatGPT is open" (R21) means *that* app is signed in, not necessarily the depot CLI. | process list 2026-10-10 | Informs C2's plain-words sign-in states. |

---

## Log

- 2026-10-10 — authored from three read-only sweeps (code, docs/CI, chat code)
  plus in-session spot checks of rows 11, 13, 30, 33.
