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
- Online, but a PR's Windows job sits queued while the runner is busy: that
  is the one Windows slot taken by another run (main's run after each
  merge, a dependabot PR, or the `platform_devpulse` neighbour on VM 119).
  `gh run list` shows which.

## The build cache (since 2026-10-24)

Each lane keeps its cargo build folder **outside the checkout**.
`actions/checkout`'s `git clean -ffdx` used to delete `target/` before every
run, so every Windows run rebuilt all ~600 crates twice and took 45–58
minutes. A warm run rebuilds only what changed.

**Where it lives** (`green.yml`'s first step sets `CARGO_TARGET_DIR`):
- **on the runner's data disk** when its `.env` sets `DPL_CI_CACHE_ROOT`:
  `<root>\devpulselabs`, e.g. `D:\ci\cargo\devpulselabs` on VM 119 once
  `-DataDisk` has run;
- otherwise **beside the checkout**, at `<runner workspace>\cargo-target`
  (`C:\actions\runner-devpulselabs\_work\devpulselabs\cargo-target`,
  `/opt/actions-runner/_work/devpulselabs/cargo-target`).

The guard and its limits:
- **The guard:** each run prints the cache size and its disk's free space.
  It wipes the cache, so that run builds cold, when the cache passes 50 GB
  or the disk has under 15 GB free.
- **Wipe it by hand** (e.g. after a toolchain bump, or if a run looks
  haunted): delete the cache folder on the guest. The next run builds cold
  and refills it.
- **Never point two runner installs at one cache.** A cache belongs to one
  runner directory, like the install itself. A shared cache across runners
  is sccache tier 2 (`sccache-shared-build-cache.md`), not a shared folder.

## Scaling a runner (the data disk)

A runner guest has two disks:
- **the OS disk**, which is never resized;
- **a data disk** that holds the build cache and nothing else.

The data volume is the only partition on its disk, so growing it is always
one host command and one online resize — no recovery partition in the way.
That is why the cache never lives on C:.

**Give a runner its data disk** (once; it works online, with no reboot):

```sh
# on the Proxmox host, as root
qm config 119 | grep -E '^(scsi|virtio|sata|ide)[0-9]+:'   # the bus in use
pvesm status                                               # a storage with room
qm set 119 --scsi1 <storage>:150                           # a new 150 GB disk
qm guest exec 119 --timeout 600 -- powershell -ExecutionPolicy Bypass -File \
  C:/actions/runner-devpulselabs/_work/devpulselabs/devpulselabs/docs/runbook/win-runner-bootstrap.ps1 -DataDisk
```

`-DataDisk` does the guest side:
- formats only a blank (RAW) disk, as NTFS labelled `ci-data`;
- writes `DPL_CI_CACHE_ROOT=<letter>:\ci\cargo` into this runner's `.env`;
- waits for any running job to finish, then removes the old cache on C:
  and restarts the service.

The next run builds cold onto the data disk; every run after it is warm.

**Grow it** later:

```sh
qm resize 119 scsi1 +100G
```

Then run `-DataDisk` again: it finds the `ci-data` volume and grows it
into the new space.

**Check it:** `win-runner-bootstrap.ps1 -VerifyOnly` reports the data volume
and its free space, and fails if the volume exists but `.env` does not
point at it.

**Add a runner** (more capacity, more slots):
1. Clone the runner VM, or rebuild from blank (§Rebuild).
2. Attach its own data disk.
3. Run the bootstrap with a fresh token, then `-DataDisk`.

The workflow names no drive and no machine, so a new runner needs only its
own `.env`. Each runner's cache is its own: one cold run, then warm.

**Linux (LXC 121)** is the same pattern:
1. Mount a volume: `pct set 121 -mp0 <storage>:100,mp=/ci-data`.
2. Add `DPL_CI_CACHE_ROOT=/ci-data/cargo` to `/opt/actions-runner/.env`.
3. Restart with `./svc.sh stop && ./svc.sh start` between jobs.

Until then its cache stays beside its checkout (65 GB free, 2026-10-24).

## Security rule (non-negotiable) — SETTLED 2026-07-30, PRECONDITION RESTORED 2026-08-07

**2026-08-07: the repo returned to PRIVATE** (operator ruling — personal
tool, no open-source exposure) and both estate runners were re-registered
from this runbook's recipe, exactly as §"kept because" below promised. The
lanes moved back to `dpl-win`/`dpl-linux` in `green.yml` the same day
(hosted minutes are metered on private repos — the first private-repo
hosted run burned 67 billable Windows-lane minutes on a docs PR's main
push). The rule's precondition — private repo, every PR author is the
operator — holds again. **If the repo ever goes public again: deregister
both runners FIRST, same day, before anything else.**

Re-registration lesson (2026-08-07, the 2026-07-28 hollow-directory failure
RECURRED): LXC 121's rebuild registered on GitHub fine, but the guest still
carried the pre-deregistration systemd unit pointing at the OLD root
(`/home/runner/runner-devpulselabs`), and the bootstrap's
`./svc.sh install || true` swallowed the "already exists" failure — so the
service started the stale install, the listener died with "Not configured",
and the roster showed the runner registered-but-offline. Cure: from the NEW
root, `./svc.sh uninstall` then `install` + `start`, and move the stale
directory aside (`*.stale-<date>`). OPEN hardening: the bootstrap should
detect a unit whose ExecStart is not `$RUNNER_HOME` and refuse loudly
instead of `|| true`-ing past it.

**The rule below was hit for real on 2026-07-30, and the resolution then
was: the estate runners are gone.** The repo went public on 2026-07-30
while both self-hosted runners were still wired to `pull_request`. Both
were deregistered within minutes and `green.yml` moved to GitHub-hosted
images (`windows-latest`, `ubuntu-latest`), which is the rework this rule
always demanded. Hosted minutes are free and unmetered for public repos.

The rule stands for any future self-hosted runner:

> These runners execute PR code with real privileges (the Windows one ran as
> SYSTEM). That is acceptable ONLY while this repo is private and every PR
> author is the operator. **Never wire a self-hosted runner to
> `pull_request` on a public repo** — fork PRs would run arbitrary code on the
> estate. Re-registering one requires either making the repo private again, or
> label-gating the workflow to same-repo PRs plus a dedicated low-privilege
> account.

Everything in this runbook about registering, bootstrapping and rebuilding
was proven again on 2026-08-07: the repo returned to private and both
runners were stood back up from these exact recipes (Windows re-registered
in place; Linux rebuilt from blank into `/opt/actions-runner`).

## Paid-for gotchas (2026-07-28, each from a real failing run)

- PowerShell `Restricted` policy silently kills every runner step
  ("running scripts is disabled") — the bootstrap sets RemoteSigned.
- rustup installs per-user: a SYSTEM service needs SYSTEM's profile
  cargo on the MACHINE PATH (and a service restart to see it).
- The Linux twin (2026-08-07, first proof run): the runner snapshots PATH
  into `/opt/actions-runner/.path` AT CONFIG TIME. `config.sh` run via
  `sudo -u runner` from root's non-login shell captures a PATH without
  `/home/runner/.cargo/bin` → every job dies `cargo: command not found`.
  Cure: prepend it to `.path`, restart the service. The bootstrap should
  eventually write `.path` itself after config.sh.
- VS Build Tools does NOT ship `libclang.dll` — bindgen needs the
  standalone LLVM install; the pin matches the dev laptop (18.1.8).
- khronos-egl needs the full apt dep set on Linux — a runner with only
  rustup dies at `pkg-config egl` in the first clippy.
- The 16 GB LNK1318/PDB cures live in `green.yml` env (`debug=0`,
  `jobs=8`) — they stay unless the Windows guest gets more RAM AND a
  green run proves they're unneeded.
- **One linker setting for every phase** (2026-10-24, #293). cargo
  fingerprints the target linker. When the test phase overrode
  `.cargo/config.toml`'s `rust-lld.exe` with the full sysroot path, clippy
  and tests rebuilt each other's build scripts and proc-macros (and the ~470
  crates above them) on every run. CI's dev `debug=0` makes the two phases'
  units share file names, so they overwrite each other. A warm run
  rebuilds nothing it already has only while every phase passes cargo the
  same settings — check with `CARGO_LOG=cargo::core::compiler::fingerprint=info`
  (`ConfigSettingsChanged` = a setting differs between invocations).
- **The artifact storage quota** (2026-10-29, `docs/plans/ci-exe-upload.md`).
  `upload the windows exe` failed with "Artifact storage quota has been hit"
  while the gate itself was green, and the merge law made every code PR red.
  - Private repos share 500 MB, billed by average use over the billing
    month. Deleting artifacts stops future use but does not undo the month's
    count, so a block mid-month lasts until the reset.
  - The usage report needs the `user` auth scope, which is not added.
  - The policy since then: upload from main only, kept 1 day. Don't add
    `continue-on-error` to the upload; it would hide a real failure.
