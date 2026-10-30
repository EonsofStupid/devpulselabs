# Public repository setup — 2026-10-30

## 1. Authorization and result

The operator authorized public visibility and complete setup in R293. Publish
EonsofStupid/devpulselabs and finish the associated GitHub configuration.
Use isolated chore/public-repository-setup from main bfc8e33. PC1b draft308 and
ST5b309 remain independent; neither is merged by this operation.

## 2. Current evidence

The repository is private, current credentials have repository admin access,
and two repository-level self-hosted runners (26 Linux,27 Windows) are online
and idle. green.yml uses them for pull_request. The canonical ci-runners.md
requires deregistration before any public transition. Nightly and EguiPilotAI
already target hosted runners. The license already anticipates public viewing
with all rights reserved; publication does not authorize relicensing.

## 3. Remote transition

Temporarily disable Actions dispatch, recheck the roster and active runs,
deregister only this repository's idle runner registrations, and verify zero
runners before setting visibility to public. Preserve guest services/files and
all artifacts/history. Read back public visibility. Keep read-only default
workflow permissions and no workflow PR approval rights. Require approval for
all external fork contributors and enable the applicable public-repo secret
scanning/push protection and dependency alerts. Restore Actions for hosted CI.

## 4. Workflow implementation

Move green Windows/Linux to GitHub-hosted images, explicit read-only workflow
permissions, verified system dependencies and a hosted cache. Preserve cargo
green and its artifact publication requirement. Remove estate-specific cache
deletion logic from hosted execution. Pin actions to their exact upstream
commit SHAs. Review nightly/EguiPilotAI public execution and permissions; avoid
changing their release policy or triggering a release as part of this task.
Run green for every PR, including documentation-only changes, so mandatory
status checks are always emitted and cannot strand a PR behind path filters.

## 5. Repository policy

Update README, CLAUDE and runner runbook to current public/hosted state.
Preserve the existing license and contribution policy. Document a private
vulnerability-reporting route. Configure main to require PRs, resolved threads
and successful green-windows/green-linux checks; forbid force pushes and branch
deletion. Do not require a second reviewer for a single-maintainer repository.
Verify public clone/read access and applicable security settings through APIs.

## 6. Verification and delivery

Validate workflow syntax and expressions, review the diff, run the required
local gate/pre-push hook, and open a separate PR. The hosted workflow must pass
both platforms before merge. Then verify main's run, public visibility, zero
self-hosted runners and protected-main requirements. Preserve artifacts if any
quota failure remains; do not bypass upload or weaken required checks.

## 7. Recovery

If a hosted prerequisite fails, fix the workflow in its PR. Keep public access
as authorized and self-hosted runners deregistered. Do not silently revert to
private or reconnect estate machines. Changes to publication/relicensing or
artifact deletion are outside this cut.

## 8. Evidence

API baseline: private, admin=true, two idle repo runners, Actions enabled;
default workflow permissions read and PR-review approval false. Latest feature
CI failed Windows artifact upload from storage quota after cargo green passed.
Current evidence will be recorded in the PR and docs/status.md on delivery.
