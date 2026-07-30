# CI runners — the runbook

The green gate's self-hosted runners are CATTLE: each one is rebuilt
from a script in this directory, verified by that same script, and
backed by a known-good hypervisor snapshot. If rebuilding a runner ever
requires archaeology, this runbook has failed — fix it in the same PR.

## Topology (since 2026-07-28, PR #122)

| Runner | Guest | Labels | Install root | Bootstrapped by |
|---|---|---|---|---|
| `devpulse-runner-win-01` | Proxmox VM 119 (win x64) | `dpl-win` | `C:\actions\runner-devpulselabs` | `win-runner-bootstrap.ps1` |
| `devpulse-runner-lnx-01` | Proxmox LXC 121 (x64) | `dpl-linux`, `rust` | systemd unit is the truth | `linux-runner-bootstrap.sh` |
| `dpl-linux-clyffy-lab` | ARM64 guest | `dpl-linux` | systemd unit is the truth | `linux-runner-bootstrap.sh` |

Host: the dedicated server (server1 / `ssh.clyffy.ai`, Proxmox; `qm` /
`pct` as root). The dev laptop is NOT a runner. Guests hosting more
than one repo's runner use PER-REPO roots — `C:\actions\runner-<repo>`
on VM 119 (the `platform_devpulse` neighbor lives beside it; separate
directory + service, separate lifecycles, never touch each other's).
Two runner installs must NEVER share or hand off a directory — a
half-moved install left a service pointing at a hollow directory once
(2026-07-28), and the lane only failed when the VM next restarted.

## Rebuild from blank

1. Create/restore the guest (or `qm rollback 119 <snapshot>` /
   `pct rollback 121 <snapshot>` to the last known-good).
2. Mint a fresh registration token (expires in ~1 h, single-purpose):
   `gh api -X POST repos/EonsofStupid/devpulselabs/actions/runners/registration-token --jq .token`
3. Run the matching bootstrap script with `--token` / `-Token`.
   Both scripts are idempotent (safe on an already-provisioned guest)
   and end with verify-or-die.
4. Confirm the roster:
   `gh api repos/EonsofStupid/devpulselabs/actions/runners`
   then prove the lane: `gh workflow run green` → both jobs succeed.

Windows guest access without credentials: from the Proxmox host,
`qm guest exec 119 -- <cmd>` (guest agent runs as SYSTEM). Quote
Windows paths — bare backslashes die in the ssh/sh layers; for
anything with embedded quotes, base64 a `.cmd` across and decode with
`certutil`. SYSTEM's installer logs live in `C:\Windows\SystemTemp`,
not `C:\Windows\Temp`.

## Snapshot policy

After ANY toolchain change that turns the lane green (new MSVC, LLVM,
rust pin, runner major), take a fresh known-good snapshot and name it
by date: `qm snapshot 119 known-good-YYYY-MM-DD` /
`pct snapshot 121 known-good-YYYY-MM-DD`. Keep the last two; delete
older ones. The snapshot is the rollback path when a toolchain bump
breaks the lane.

## Heartbeat

A runner that is registered but offline fails silently — PRs just
queue forever. Check when a job sits queued > ~2 min:

- Roster: `gh api repos/EonsofStupid/devpulselabs/actions/runners`
  (any `"status": "offline"` is an incident).
- On-guest audit, no token needed:
  `win-runner-bootstrap.ps1 -VerifyOnly` /
  `linux-runner-bootstrap.sh --verify-only`.
- Online but jobs stay queued → the listener session predates a
  re-registration: restart the runner service
  (`Restart-Service actions.runner.*` / `./svc.sh stop && ./svc.sh start`).

## Security rule (non-negotiable) — SETTLED 2026-07-30

**The rule below was hit for real, and the resolution is: the estate runners
are gone.** The repo went public on 2026-07-30 while both self-hosted runners
were still wired to `pull_request`. Both were deregistered within minutes and
`green.yml` moved to GitHub-hosted images (`windows-latest`, `ubuntu-latest`),
which is the rework this rule always demanded. Hosted minutes are free and
unmetered for public repos.

The rule stands for any future self-hosted runner:

> These runners execute PR code with real privileges (the Windows one ran as
> SYSTEM). That is acceptable ONLY while this repo is private and every PR
> author is the operator. **Never wire a self-hosted runner to
> `pull_request` on a public repo** — fork PRs would run arbitrary code on the
> estate. Re-registering one requires either making the repo private again, or
> label-gating the workflow to same-repo PRs plus a dedicated low-privilege
> account.

Everything below about registering, bootstrapping and rebuilding the estate
runners is kept because it is the recipe for standing them back up if the repo
ever returns to private — not because anything is registered today.

## Paid-for gotchas (2026-07-28, each from a real failing run)

- PowerShell `Restricted` policy silently kills every runner step
  ("running scripts is disabled") — the bootstrap sets RemoteSigned.
- rustup installs per-user: a SYSTEM service needs SYSTEM's profile
  cargo on the MACHINE PATH (and a service restart to see it).
- VS Build Tools does NOT ship `libclang.dll` — bindgen needs the
  standalone LLVM install; the pin matches the dev laptop (18.1.8).
- khronos-egl needs the full apt dep set on Linux — a runner with only
  rustup dies at `pkg-config egl` in the first clippy.
- The 16 GB LNK1318/PDB cures live in `green.yml` env (`debug=0`,
  `jobs=8`) — they stay unless the Windows guest gets more RAM AND a
  green run proves they're unneeded.
