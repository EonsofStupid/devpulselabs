# Pass C0 — clean ground for the native chat program

Program: `docs/plans/chat-native-program.md` (§4.7, pass C0). Branch
`chat/c0-clean-ground`. Authored 2026-10-10.

## 1. Context

Before chat can be built natively, the ground has to be honest. Three
problems block that today:
- The working tree carried uncommitted 10-09 work that fails the gate and
  fakes process supervision.
- The docs still point every session at a chatmaton/automaton merge and at
  paths that do not exist on this machine.
- No tool depot exists here, so nothing depot-resolved (providers, ffmpeg)
  can run.

The operator approved proceeding on the recommendations (*"whatever fine"*,
2026-10-10).

## 2. Goal / non-goals

**Goal:** a clean `main` that:
- carries the external-reach inventory and the chat program;
- marks the merge superseded;
- states true paths;
- passes `cargo green` on Windows;
- has the release exe rebuilt from it.

Separately, the depot is staged on this machine for every payload that
exists here.

**Non-goals:**
- No product code.
- No fix to the Codex version gate (owned by C2).
- No action on any inventory row outside C0's scope.
- No push of the parked branch.

## 3. Ground truth (2026-10-10)

- **The 10-09 work** is now parked on local branch
  `park/codex-subprocess-lifecycle-2026-10-09` (`d063e87`): never pushed,
  never merged, nothing lost. After that, `main` was clean except for the two
  new docs.
- **Paths:**
  - This machine has no `C:\localdev`.
  - The checkout is `D:\localdev\devpulselabs`.
  - The old app's full source is `D:\localdev\review-archive\devpulse labs`
    (`src/domains/*` present, `.git` at `98babf4`).
- **Depot:** `dptools-deps/sources.local.toml` is absent, and so is any
  staged `dptools-deps/`.
  - **codex** is found at
    `C:\Users\jessa\AppData\Local\Programs\OpenAI\Codex\bin\codex.exe`, but
    it is `codex-cli 0.160.1` and ships beside `codex-code-mode-host.exe`.
  - **Chrome** exists only as a system install.
  - ffmpeg, pdftoppm and grok are being searched for across both drives (§8).
- **Staging tool:** `cargo xtask stage-tool-deps <root>` refuses a root that
  already has `dptools-deps/`. It selects entries for the current target and
  copies a file, or a tree plus an entrypoint. It hashes the copy, writes
  `manifest.json` and re-opens the result through the production resolver.

## 4. Design

1. **Docs:**
   - Land `external-reach-inventory.md` and `chat-native-program.md`.
   - Add SUPERSEDED headers to `unified-ai-kernel-automaton-merge.md` and
     `journal/2026-08-11-unified-ai-kernel.md`, keeping the 08-11 operator
     quote.
   - Add a superseded-claims entry at the top of `STATE.md`.
2. **True paths:**
   - `CLAUDE.md` (checkout, old app).
   - `AGENTS.md` (old app; the chat canonical pointer moves to the program
     and the inventory).
   - Skills `port-capture` (path and the stale "build output only" note) and
     `slide-catalog-pass`.
3. **Depot (machine-local, gitignored):**
   - Write `dptools-deps/sources.local.toml` from the binaries actually found.
   - Stage into `target/release` (beside the exe the operator runs).
   - A payload not on this machine is **not invented**; it is reported to the
     operator, and the app keeps degrading honestly for it.
4. **Ledger:** add a `docs/ops/resume/CURRENT.md` session block and a
   STATE.md entry.

## 5. Wiring

None. Docs and agent instructions only, plus one gitignored machine-local
file. No crate, edge or manifest changes.

## 6. Proof

- `cargo green` on the branch (fmt, clippy `-D warnings`, tests, structure,
  release, smoke).
- The PR's `green-windows` and `green-linux` checks are both COMPLETED
  SUCCESS before merge.
- `target\release\devpulselabs.exe` rebuilt from merged `main`, with its
  mtime checked.
- The staged depot re-opens through the production resolver (the xtask's own
  post-stage check). A launch logs `verified … resolved` for each staged
  payload.

## 7. Risks

- **Missing payloads.** If ffmpeg, pdftoppm or grok are not on this machine,
  the depot is partial. That is reported, not papered over (no download
  without operator say-so, inventory row 46).
- **Codex's sibling binary.** Codex 0.160.1 may need
  `codex-code-mode-host.exe`. Staging codex as a single file could break
  features that spawn the sibling. If so, stage the `bin/` directory as a
  tree with entrypoint `codex.exe`.
- **The Codex version gate** (0.145.x only) will report the staged 0.160.1
  as unsupported until C2. That is expected and stated.

## 8. Evidence

- **Park commit** `d063e87` (local only). After it, `main`'s tree was clean
  apart from this pass's docs.
- **Local gate.** `cargo green` on `chat/c0-clean-ground` (`91f0725`) passed
  end to end: fmt, audits, clippy `-D warnings`, workspace tests, release,
  and smoke (real pixels). Output: `green ✔ exe:
  D:\localdev\devpulselabs\target\release\devpulselabs.exe`.
- **Exe.** `target\release\devpulselabs.exe` moved from **Sep 27 14:27** to
  **Oct 10 12:34**. `dpl-lottie-render.exe` was unchanged; its sources had
  not changed, so there was nothing to relink.
- **Gate hook gap.** This clone had `core.hooksPath` empty, so the pre-push
  gate never ran from it. It is now `.githooks`.
- **Depot search.** A search of both drives found the operator's depot at
  `D:\projects\dptools-deps\artifacts\windows-x86_64` (inventory row 56).
  `sources.local.toml` was written against it; all five paths exist and each
  binary reports its expected version (`codex-cli 0.145.0-alpha.18`, `grok
  0.2.112`, `ffmpeg 8.1.2-full_build`).
- **Depot staging.** `cargo xtask stage-tool-deps target/release` staged
  codex, grok, chromium (tree), ffmpeg and pdftoppm (tree), writing
  `manifest.json` (schema 2, SHA-256 per payload). The run ended with
  "DevPULSE tool depot verified".
- **Real-exe proof.** The release exe ran with `--window 1600x900
  --screenshot … --no-seam`:
  - It logs `verified embedded ffmpeg resolved …\dptools-deps\media\ffmpeg\8.1.2`
    and `verified embedded pdftoppm resolved`.
  - The UI paints with the footer visible at that size, stamped `91f0725`.
  - Codex and Grok report "no prior activation evidence"; signing in is C2.
- **Orphan check.** No `devpulselabs.exe`, depot `codex.exe` or `grok.exe`
  was left running. The four running `codex.exe` processes are the
  operator's OpenAI Codex desktop app (0.162.1 and its own install), not
  DevPULSE (inventory row 59).
- **Found and recorded** (rows 55, 58): a second clone at
  `D:\projects\devpulselabs`, and a second old-app copy at
  `D:\projects\devpulse labs`.
- **CORRECTION (operator ruling, same day): `D:\projects\devpulselabs` IS THE
  checkout.** It holds the operator's library (62 files / 150 MB). This
  pass's §3 "the checkout is `D:\localdev\devpulselabs`", and the
  depot/exe evidence above, describe the unused clone. The real checkout was
  fast-forwarded to `3ae1dd3`, and its `sources.local.toml` repointed to
  `D:/projects/dptools-deps` (all five sources verified). Its staged depot
  is the same five payloads. The exe rebuild and real-exe proof for THIS
  checkout come with this PR.
- **The operator paused chat behind D0** (deps on this laptop first:
  camera, ffmpeg, files).
- **CI.** The `green` workflow ignores docs-only PRs, so #256 had no
  automatic checks. A `workflow_dispatch` on the branch (run `38079544559`)
  supplied both lanes on head `91f0725`: `green-linux` and `green-windows`
  both COMPLETED SUCCESS. #256 was squash-merged as `3ae1dd3`. This
  close-out (evidence, inventory rows 55–59, CLAUDE.md depot and
  second-clone note, STATE, CURRENT) lands as its own PR on the same
  double-green rule.
