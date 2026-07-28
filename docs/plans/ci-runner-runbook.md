# CI-runner runbook — cattle, not pets (hardening pass after #122)

## 1 · Context

PR #122 landed the runner migration: both green lanes live on the
dedicated server's guests (Windows VM 119 `devpulse-runner-win-01`,
Linux LXC 121 `devpulse-runner-lnx-01`, plus the ARM64 clyffy-lab
runner). It works and is proven — but the estate is a PET: the Windows
VM was provisioned imperatively across two sessions, the staged
`toolchain.ps1` had a real defect (no exit-code checks; wrote its
done-marker unconditionally — cost an hour of false "done"), and if VM
119 dies the rebuild lives only in session memory. Operator directive
(2026-07-28): "let's go ahead and plan and get the code implemented" —
commit the senior-grade fixes flagged in the audit: an idempotent
bootstrap for each lane, a runbook doc, RAM headroom for the shared VM,
and known-good snapshots.

## 2 · Current state (all ground-truthed live, this session)

- VM 119 (16 GB): git 2.55.0 (machine-wide), rustup + 1.92.0 default
  profile in the SYSTEM profile, VS Build Tools MSVC 14.44.35207 + SDK
  10.0.26100, LLVM 18.1.8 at `C:\Program Files\LLVM`, runner v2.336.0
  at `C:\actions-runner` as SYSTEM service
  `actions.runner.EonsofStupid-devpulselabs.devpulse-runner-win-01`
  (recovery + delayed auto-start), LocalMachine ExecutionPolicy
  RemoteSigned, machine PATH includes
  `C:\Windows\System32\config\systemprofile\.cargo\bin`. A SECOND
  runner (the other lane's `platform_devpulse`) coexists in a separate
  dir/service. `C:\bootstrap` holds the old installers + the buggy
  toolchain.ps1.
- LXC 121: full apt dep set (the server1 list) + runner; proven green.
- green.yml: `workflow_dispatch` + pull_request, lanes on
  [self-hosted, dpl-win] / [self-hosted, dpl-linux].
- CLAUDE.md "CI runners" section documents the topology (rewritten in
  #122); no runbook scripts exist anywhere in the repo. `docs/runbook/`
  does not exist.
- Proxmox host = server1 (`qm`/`pct` as root); no snapshots of 119/121
  taken for this purpose yet.

## 3 · Target design

`docs/runbook/` — committed, versioned, executable provisioning:

- `win-runner-bootstrap.ps1` — blank Windows x64 → registered dpl-win
  runner. IDEMPOTENT (every step checks "already satisfied" first),
  every install checks its REAL exit code (0/3010 for VS), fails loud
  with the failing step named, never writes a done-marker it didn't
  earn. Parameters: `-RepoUrl`, `-Token` (registration token, minted
  fresh by the caller), `-RunnerName`, `-Labels`. Versions PINNED to
  the proven set (git 2.55.0.3, rust 1.92.0, LLVM 18.1.8, VS
  VCTools latest-channel) — bumping a pin is a reviewed diff.
- `linux-runner-bootstrap.sh` — blank Debian/Ubuntu guest → registered
  dpl-linux runner: the full apt dep set (the known-good server1 list),
  rustup 1.92.0, runner install as the systemd service via
  `svc.sh install`. Same idempotency + pinning rules.
- `ci-runners.md` — the runbook: topology map, rebuild-from-blank
  procedure (mint token → run script → verify roster), snapshot policy
  (known-good after every toolchain change), the security rule
  (self-hosted PR runners mean this repo must NEVER go public without
  rework), heartbeat check (`gh api .../actions/runners` — any
  offline runner is an incident), and the operational gotchas paid for
  this session (SYSTEM temp = `C:\Windows\SystemTemp`, stale-listener
  → restart service, one-runner-per-guest preferred).
- CLAUDE.md: the CI-runners section gains ONE pointer line to the
  runbook (no duplication — CLAUDE.md stays the summary).

Infra actions (not code, executed after merge):
- `qm set 119 --memory 32768` + VM restart (two runners share it;
  16 GB is the LNK1318 zone) → verify runner auto-recovers online.
- `qm snapshot 119 known-good-2026-07-28` (disk-only) + `pct snapshot
  121 known-good-2026-07-28` — the baseline the runbook references.
- One `workflow_dispatch` green run as the post-restart proof.

Scale story: the Nth runner (mac lane, more Linux capacity) = mint a
token, run the matching bootstrap, add the label — no archaeology.

## 4 · Changes, in execution order

1. CREATE `docs/runbook/win-runner-bootstrap.ps1` (full contents in
   this pass's implementation — sections: params; helpers `Step`/`Ok`
   with exit-code enforcement; ExecutionPolicy; git (skip if
   `git --version` ok); rustup→1.92.0 default profile (skip if
   `cargo --version` under systemprofile ok); VS Build Tools VCTools
   (skip if `...\VC\Tools\MSVC\14.*` exists; accept exit 0 and 3010;
   logs pointer to SystemTemp); LLVM 18.1.8 `/S` (skip if
   `C:\Program Files\LLVM\bin\libclang.dll`); machine PATH append
   (idempotent contains-check); runner download latest → C:\actions-
   runner (skip if config.cmd present); `config.cmd --unattended
   --replace ... --runasservice --windowslogonaccount "NT
   AUTHORITY\SYSTEM"`; final verify: service RUNNING + `cargo`,
   `git`, `libclang.dll` all resolvable — script exits nonzero naming
   the first unsatisfied check).
2. CREATE `docs/runbook/linux-runner-bootstrap.sh` (apt set verbatim
   from the proven install; rustup 1.92.0; runner + `./svc.sh install
   && ./svc.sh start`; same verify-or-die tail).
3. CREATE `docs/runbook/ci-runners.md` (content per §3).
4. EDIT `CLAUDE.md` CI-runners section: append pointer line
   "Rebuild-from-blank + snapshot/heartbeat policy:
   `docs/runbook/ci-runners.md` (scripts beside it)."
5. Commit on `infra/runner-runbook` (worktree lane), push via server1
   (gate), PR, merge. Docs-only → CI won't run; the pre-push gate is
   the enforcement, per law.
6. POST-MERGE infra: RAM bump + restart 119 → verify service
   auto-recovery + roster online; snapshots of 119 and 121;
   `gh workflow run green.yml` dispatch → both lanes green = the
   final proof the runbook's world matches reality.

## 5 · Boundaries

- 🚫 The other lane's `platform_devpulse` runner (dir, service, repo)
  — never touched. 🚫 THE checkout's working tree (parked on
  `feat/chat-assist-mode` by the other session). 🚫 `/content`.
  🚫 No secrets in the repo — the scripts take tokens as parameters,
  never embed them; server1's gh token stays out of scope.
- ⚠️ Ask first: changing runner service accounts (SYSTEM → dedicated
  user), touching the ARM64 runner, un-pinning any toolchain version.
- Out of scope: offline alerting automation (documented as a manual
  heartbeat in the runbook; a cron is a later ask-first), the laptop's
  leftover schtask (operator-owned, elevated).

## 6 · Verification

1. PowerShell parses the .ps1: `powershell -NoProfile -Command
   "$null = [ScriptBlock]::Create((Get-Content -Raw <script>))"` exits 0.
   `bash -n linux-runner-bootstrap.sh` exits 0.
2. IDEMPOTENCY PROOF on the real VM: run `win-runner-bootstrap.ps1`
   on VM 119 (already-provisioned) with a fresh token — every step
   reports SKIP/ok, service ends RUNNING, exit 0. This validates the
   script against the exact machine it documents. Same for LXC 121
   with the .sh.
3. Post-merge infra: `qm config 119 | grep memory` → 32768; runner
   online after restart with NO manual step; snapshots listed by
   `qm listsnapshot 119` / `pct listsnapshot 121`.
4. `gh workflow run green` → both jobs conclude success on the estate
   runners.

## 7 · Commit plan

Branch `infra/runner-runbook` (worktree lane, off `5aff410`). One
commit: "infra(runbook): CI runners become cattle — idempotent
bootstraps + runbook + snapshot/heartbeat policy". Push from server1
(hook gate), `gh pr create`, squash-merge. Post-merge infra actions,
then the STATE.md close-out rides the session-end update.

## 8 · Evidence

Every claim in §2 was established live earlier THIS session (the
provisioning, failures, and fixes are this conversation's own work):
runner rosters via `gh api`, VM state via `qm guest exec` probes,
job logs via `gh api .../jobs/<id>/logs`, LXC apt via `pct exec`.
green.yml + CLAUDE.md re-read at `5aff410` in the lane.
