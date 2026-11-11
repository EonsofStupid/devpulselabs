# Protected guided-workflow delivery

The `delivery` workflow reconciles explicitly opted-in pull requests. Add
`delivery:auto-merge` to a same-repository, non-draft PR targeting `main` after
the work is ready for protected delivery. Include `Closes #<delivery issue>` in
the PR description when that issue should close on actual merge. The workflow
creates its labels if absent; it never opts other PRs into delivery.

GitHub native squash auto-merge remains subject to main's required
`green-windows` and `green-linux` checks, up-to-date branch requirement, resolved
conversations and other protections. It does not force-push or bypass checks.
The monitor rechecks scope/head immediately before requesting a mutation and
uses `--match-head-commit` for native auto-merge and `expected_head_sha` for an
ordinary branch update. Removing the label stops future monitor mutations;
**also disable auto-merge in GitHub if a native request is already pending.**

## Monitoring and blockers

The workflow runs after green is requested/completed, on main pushes, by manual
dispatch, and at minutes 7, 22, 37 and 52 each hour. Scheduled execution can be
delayed by GitHub. All runs serialize through one concurrency group.

A merge conflict, failed current-head required check, observed approval wait,
or missing required CI for 30 minutes without an active run creates one
`delivery:blocker` issue per PR. The managed body includes the observed head
and action needed. Repeated identical observations do not post repeated alerts;
recurrence reopens the same issue. Notes outside the marker are preserved.
When the blocker clears, its issue closes even if the PR is still waiting for
checks. A closed unmerged or no-longer-opted-in PR retires its blocker as
`not_planned`; that is not a completed delivery. The separate delivery issue
closes through its PR closing keyword on merge.

Only observations for the current head and required GitHub Actions check
contexts are used. GitHub remains the authoritative protection evaluator.
An Actions summary links each observed PR and reports main build/artifact
status. API failures fail the monitor and remain visible in Actions.

## CI approval and main executable

GitHub's current token rules make `GITHUB_TOKEN` PR opened/synchronize/reopened
events create approval-required workflow runs. An ordinary bot branch update
therefore does **not** promise automatically running green checks. Inspect the
PR approval banner and approve eligible trusted code with a writer account, or
perform a normal reviewed update from an authorized account. This monitor never
approves PR workflows, dispatches a PR ref, adds a PAT, or weakens approval policy.
See [GitHub token event rules](https://docs.github.com/en/actions/concepts/security/github_token)
and [workflow trigger documentation](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow).

Token-generated merges may suppress the usual push-triggered main build. When
the current main SHA is an opted-in merged PR's merge commit and no main green
run exists for it, the monitor explicitly dispatches `green.yml` on **main**.
It checks existing runs first and persists a per-SHA comment marker before
dispatch, preventing repeated requests while run indexing catches up. The
marker is an intent record, not evidence of a successful build. If dispatch
fails or the job stops between the marker and dispatch, inspect Actions and
manually dispatch green on main; it does not loop on an uncertain request.
If main advances between the final check and GitHub's dispatch resolution,
GitHub runs the then-current trusted main; the next reconciliation reports the
actual SHA. See [workflow dispatch API](https://docs.github.com/en/rest/actions/workflows#create-a-workflow-dispatch-event).

The summary distinguishes building, failed, artifact available, and artifact
unavailable. It links the green run, whose Windows artifact is named
`devpulselabs-windows-<main SHA>`. The existing artifact retention is one day;
expired/missing artifacts require a deliberate green rerun, not automatic
continuous rebuilds. A passing CI artifact is a test candidate, not evidence
of live OS recording, foreign-app automation or visual qualification.

## Trust boundary and operational follow-up

`delivery.yml` always checks out trusted `refs/heads/main` using a pinned action,
without persisted credentials. It does not use `pull_request_target`, check out
PR code, execute PR-provided commands, or download PR artifacts. The privileged
job performs metadata/API operations only; its fixtures also come from main.
Permissions are checks read for required-check observations,
contents/pull-requests/issues write for protected merge/update and reporting,
and actions write solely for the main dispatch. Authentication
uses the ephemeral workflow token. No new service or product runtime endpoint
is introduced.

GitHub issues and Actions are the durable notification surface. They cannot
wake an inactive assistant conversation. During active delivery, poll the PR,
required checks, `delivery:blocker` issues, and latest delivery summary. On
resume inspect these before declaring completion. Do not promote draft #308
or unrelated unlabelled dependency PRs as a side effect.

Run the offline fixtures locally with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .github/scripts/delivery.tests.ps1
```

They need no token, network access or Pester module. Hosted CI runs the same
fixtures with `pwsh`. Actual GitHub merge/approval, repository protection and
artifact availability remain integration checks after publication.
