# GitHub repo hardening — dependency graph, Dependabot, nightlies

## 1. Context

DevPULSE is a personal private tool, but the GitHub project still needs
proper supply-chain visibility and a downloadable rolling build. Without
that: no Insights dependency graph, no Dependabot PRs, no Release assets
to pull when the laptop is offline. This pass wires the free private-repo
surface (graph + alerts + version updates) and a self-hosted nightly
Release the operator can grab with one script.

## 2. Current state (ground-truthed 2026-07-27)

- CI: [`.github/workflows/green.yml`](../../.github/workflows/green.yml)
  (self-hosted `dpl-win` + `dpl-linux`) and
  [`.github/workflows/eguipilotai.yml`](../../.github/workflows/eguipilotai.yml).
  No Releases, no Dependabot config, `delete_branch_on_merge` was false.
- Branch protection / required checks remain Pro-gated — local pre-push
  hook stays the hard gate (STATE CI+process).
- Private tool depot (`dptools-deps` / `stage-tool-deps`) stays
  operator-staged; nightlies are **exe-only**.

## 3. Target design

1. **Dependency graph + alerts** — enable vulnerability alerts + automated
   security fixes; Cargo.lock already feeds the graph (verified: 61
   manifests via GraphQL).
2. **Dependabot version updates** — [`.github/dependabot.yml`](../../.github/dependabot.yml)
   for `cargo` + `github-actions` (weekly, grouped patch/minor for Cargo).
3. **Rolling nightly Release** — [`.github/workflows/nightly.yml`](../../.github/workflows/nightly.yml)
   builds on both self-hosted runners, publishes/replaces tag `nightly`
   with `devpulselabs-windows-x64.zip` + `devpulselabs-linux-aarch64.zip`.
   Soft-fail per lane if a runner is offline.
4. **Grab scripts** — [`scripts/pull-nightly.sh`](../../scripts/pull-nightly.sh)
   / [`scripts/pull-nightly.ps1`](../../scripts/pull-nightly.ps1).
5. **Repo hygiene** — `delete_branch_on_merge=true`.

## 4. Changes (execution order)

1. API: vulnerability alerts, automated security fixes, delete-branch-on-merge.
2. Commit `dependabot.yml`, `nightly.yml`, pull scripts, this plan.
3. PR → green-linux → squash-merge.
4. `workflow_dispatch` nightly once; prove Release + pull script.
5. Tip STATE.

## 5. Boundaries

- Repo stays private.
- No bundling of private provider/browser binaries in nightlies.
- No CodeQL / secret-scanning product claims if Advanced Security is paid-gated.
- No branch-protection required checks (Pro).
- No auto-fetch of upstream tools into `dptools-deps`.

## 6. Verification

- GraphQL: `dependencyGraphManifests.totalCount > 0`,
  `hasVulnerabilityAlertsEnabled == true`.
- Dependabot config present under `.github/dependabot.yml`.
- Nightly `workflow_dispatch` publishes tag `nightly` with ≥1 zip.
- `./scripts/pull-nightly.sh` downloads the linux aarch64 asset.

## 7. Commit plan

Branch `feat/github-repo-hardening`. One commit for workflows/scripts/plan;
PR → gate → merge → tip STATE.

## 8. Evidence

_(filled at close)_
