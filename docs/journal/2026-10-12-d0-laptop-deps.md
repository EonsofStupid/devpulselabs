# 2026-10-12 — D0 laptop deps: D0.1a landed, D0.1b in flight

Milestone `d0-laptop-deps` (plan: `docs/plans/d0-laptop-deps.md`).
Checkout: **`D:\projects\devpulselabs`** (THE checkout — operator ruling
2026-10-10; never work in `D:\localdev\devpulselabs`).

## Done so far

- **#256** (`3ae1dd3`) C0 docs: the native chat program, the external-reach
  inventory, true paths; the automaton-merge plan and journal SUPERSEDED.
- **#257** (`cd90864`) C0 close-out: inventory rows 55–59, "the one
  checkout", the approved D0 plan + CHAT+AI plan, D0.0 evidence
  (`docs/proof/d0-0-boot-1600x900.png`).
- **#258** (`496a318`) D0.1a honest media: ffmpeg 8 dshow device parsing,
  stderr tails, 0-frames-is-an-error, verified output, frame-deadline pacing
  (`Recorded { frames, seconds }.drift_seconds()`), `--video-audio`.
  Real exe: mp4/vp9/av1 149 frames drift 0.000; mic 60 s 1798 frames drift
  0.001 with an AAC stream.
- **#259** (`f720942`) the chat-native-program amendment (vendor coverage
  §3.2a, naming defects §3.2b, canonical AI layout §4.3.1–4.3.6).
- `target\release\devpulselabs.exe` rebuilt from `496a318` (Oct 12 00:18)
  and verified.

## D0.1b — approved ("go" 2026-10-12), uncommitted on `d0/1b-depot-only`

Done in the working tree:
- `composer/src/import.rs`: pdftoppm injected-depot-only, `PDFTOPPM_MISSING`
  honest error, `.output()` + last stderr line; app warning text updated.
- `lottie/src/lib.rs`: `helper_path() -> Result` (override → beside exe →
  Err naming the path), orphan fix on a bad READY header.
- `coach-runtime/src/overlay.rs` (new): `overlay_path()` shared by the coach
  hub and the coach daemon (both private `overlay_bin()` copies deleted).
- `engine-browser`: `candidates()`/`find_on_path`/system-install search
  removed; `resolve()` = override (must exist) → `dpl_tools_deps::
  resolve_current(CHROMIUM)` → honest Err. New dep `dpl-tools-deps`.
  The corrupted "Every place we theme" comment went with `candidates()`.
- `desktop-spike/.../passthrough_probe_linux.rs:340` "theme" → "look".
- `cargo fmt --all` clean.

## Exact next step

1. Clippy `-D warnings` on the touched crates, then the tests:
   `cargo test -p dpl-composer -p dpl-lottie -p dpl-coach-runtime
   -p dpl-engine-browser -p dpl-hub-coach`.
2. Release build (`cargo build --release -p devpulselabs -p
   dpl-lottie-render`), then the D0.1b real-exe proofs (plan §D0.1b proof):
   Lottie render via the depot-staged helper, PDF import via the depot
   pdftoppm (scratch root, generated one-page PDF), and a scratch copy of
   the exe dir without `dpl-lottie-render.exe` showing the honest error.
3. D0.1b evidence into the plan §8; commit; push (gate runs as the hook —
   background it and wait); PR; `gh pr checks <n> --watch` for BOTH lanes;
   merge; rebuild the exe here, check mtime; STATE.md.
4. Then D0.1c: design shown to the operator BEFORE code.

## Landmines

- The Bash tool resets cwd to `d:\localdev\devpulselabs` after every call:
  always `cd /d/projects/devpulselabs &&` first.
- `icacls` through Git Bash is mangled by MSYS: use PowerShell.
- Run `cargo fmt --all -- --check` before every push (D0.1a's first push
  failed fmt).
- Docs-only PRs skip CI (paths-ignore): `gh workflow run green.yml --ref
  <branch>`.
- ONE Windows estate runner: lanes queue; Windows takes ~40 min.
- The depot lives in `target/release/dptools-deps`: `cargo clean` deletes
  it — re-stage with `cargo xtask stage-tool-deps target/release`.
- Untracked `out/` and `.vscode/` in THE checkout are not this pass's — do
  not commit them.
- `D:\localdev\devpulselabs` holds the unused clone plus local branches
  `wip/c1-paused` and `park/...`: never push them.
- Other projects' `xtask run-dev` processes (Rust 1.92) are not ours.

## Verify commands

```
git -C D:/projects/devpulselabs status --short
git -C D:/projects/devpulselabs log --oneline -5
gh pr list
cargo fmt --all -- --check
cargo test -p dpl-composer --lib import::
cargo test -p dpl-lottie --lib
cargo test -p dpl-coach-runtime --lib overlay
cargo test -p dpl-engine-browser --lib
```
