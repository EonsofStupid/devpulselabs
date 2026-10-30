# Pass — CI uploads the Windows exe from main only, kept one day

**Ruled:** 2026-10-30. The operator chose "Upload only from main, 1 day".
**Touches:** `.github/workflows/green.yml` and `docs/runbook/ci-runners.md`.
No code changes.

## 1. Context — why this pass exists

Every code PR's `green-windows` lane went red on 2026-10-29 at its last step,
`upload the windows exe`:

> Failed to CreateArtifact: Artifact storage quota has been hit. Unable to
> upload any new artifacts.

- **The gate itself was green.** `cargo green` passed on both OSes on ST5b
  (#309, head `683b380`).
- **The merge law counts a red lane as red**, so nothing could merge.
- **Deleting didn't unblock it.** With the operator's approval, 19 old
  uploads were deleted, leaving 3 (79 MB). Twelve attempts over the
  following 8 hours still hit the quota.
  - GitHub bills private-repo storage by average use over the billing
    month.
  - Deleting stops future use but doesn't undo October's count, so the block
    most likely lasts until the month resets (likely 2026-11-01 UTC).
  - The usage report needs the `user` auth scope, which is not added.

## 2. Current state (ground-truthed in-session, `bfc8e33`)

- **The upload step has no `if:`.** It runs after every successful Windows
  gate: every PR push, every push to main, and every dispatch.
- **Uploads are kept 3 days** (`retention-days: 3`); each is about 27 MB.
- **At about 7 runs a day**, roughly 570 MB stays stored. That is more than
  the account's shared 500 MB for private repos, and it is what filled the
  quota.
- **Nothing downloads a PR's exe.**
  - The release exe is rebuilt locally from main after every merge (session
    law).
  - The artifact exists so that "the exe for the current main" has an
    answer (#201).
- **`eguipilotai.yml` also uploads, on `workflow_dispatch` only.** It is not
  part of this.

## 3. Design

- **The upload step gets `if: github.ref == 'refs/heads/main'`.** That
  covers pushes to main and a dispatch run on main; PR runs (`refs/pull/…`)
  skip it.
  - An `if:` without a status function still implies `success()`, so an
    artifact exists only for a commit that passed the gate on this OS. That
    property from #201 is kept.
- **`retention-days: 1`.** At about 3–5 merges a day that keeps roughly
  80–135 MB stored, well under the quota.
- **The header and the `push:` comments say PR runs no longer upload.**
- **The runbook's "Paid-for gotchas" gets the quota entry**: how it fails,
  why deleting doesn't help mid-month, and the policy.

## 4. Scope

- **In:** the step's `if:` and retention, the workflow comments, and one
  runbook bullet.
- **Out:**
  - a storage spending limit (the operator's billing setting);
  - the `eguipilotai.yml` upload;
  - any `continue-on-error` on the upload. That would hide a real failure
    and pass a check by side-stepping it.

## 5. Work plan

1. Edit `green.yml`: add the `if:`, set `retention-days: 1`, and update the
   comments.
2. Add the runbook bullet.
3. Open the PR. Its `green-windows` run must show the upload step
   **skipped** and the job **success**. Merge only when both lanes have
   passed.
4. Rebase ST5b (#309) on main and push. Its fresh run must be double-green,
   then merge.

## 6. Risks

- **Main's own upload stays red until the quota resets** (likely Nov 1). The
  merge commit's push run on main will fail at its upload step. That doesn't
  block a PR, but main shows red until then.
  - Recorded in the PR and in status. Re-run main's latest push run after the
    reset.
- **One day is short.** After a day with no merge, there is no downloadable
  main exe until the next merge. The local rebuild is unaffected. A
  `workflow_dispatch` on main makes a fresh one.
- **Storage is shared across all of the account's private repos.** Uploads
  from another private repo can still fill it. Those repos are outside this
  pass.

## 7. Verification / acceptance

- **This PR's `green-windows` run:** `upload the windows exe` is `skipped`
  and the job is `success`. `green-linux` is `success`.
- **ST5b's run after the rebase:** both lanes are `success` on its new head
  sha.
- **After the reset:** a push run on main uploads
  `devpulselabs-windows-<sha>` with an expiry one day out.

## 8. Evidence

- PR run: pending.
