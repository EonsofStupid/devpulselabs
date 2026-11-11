# Guided workflow delivery and completion automation

Status: authorized implementation, 2026-11-02. The operator requests researched
implementation, automatic merging/closing on completion, conflict reporting,
and an executable they can test. This continues GF0/GF1a/GF1b and the remaining
Coach correctness findings; it does not authorize bypassing qualification gates.

## Grounded delivery contract

GitHub currently has auto-merge disabled. Main requires up-to-date branches,
green-windows and green-linux, resolved conversations, and enforces protection
for administrators. Preserve these protections. PR #308 is a conflicted draft
with unqualified provider adapters; dependency PRs and that draft are not silently
promoted by this cut.

Use GitHub's native auto-merge, enabled per explicitly opted-in, same-repository,
non-draft PR targeting main. Successful merge closes the PR; closing keywords
close its delivery issue only on merge. Do not auto-close failing/unmerged work.
Repository auto-merge setting may be enabled under this instruction.

A trusted-default-branch GitHub Actions monitor polls opted-in PRs on PR metadata
events, completed green runs, main updates and a schedule. It never checks out or
executes PR code with write credentials. It can request a normal branch update
with an expected head SHA when behind, and enable native auto-merge; it cannot
force-push, bypass checks or resolve semantic conflicts by guessing. Conflict or
failed-check issues are deduplicated by PR identity and closed when that blocker
clears. Summaries expose pending, blocked and merged states. Actions are pinned;
permissions are limited to the GitHub metadata/branch operations required.

The monitor is a durable GitHub inbox, not a claim that GitHub can wake an inactive
assistant conversation. During active delivery the assistant polls it and CI;
on resume the session loop checks tracked PRs/blocker issues. No Slack/email or
outside service is introduced. Production app/UI remain pure Rust; CI-only
PowerShell follows the repository's existing CI practice.

## Implementation and integration

1. Implement metadata-only monitoring with offline fixtures for draft/fork/base
   exclusion, conflicts, failed/pending/successful checks, stale-head updates,
   repeated alerts and closure. Document the opt-in and follow-up loop.
2. Ground F3-F6 against current canonical Coach architecture before editing.
   Restore the devtools-only foreign overlay/actuation boundary, preserve full
   locator constraints and truthful visibility/window/outcome semantics. Record
   the exact bounded Coach plan after source grounding.
3. Preserve all existing local GF0/GF1a/GF1b work, review and run appropriate
   tests plus the repository's required gate. Commit explicit product/docs paths
   only, excluding generated cache and unrelated parent/operator work.
4. Publish a tracked delivery issue and PR, opt it into native protected
   auto-merge, monitor both hosted platforms, resolve actual conflicts and fix
   failures without weakening checks. Verify merged SHA and issue closure.
5. Build/qualify a native test candidate with a recorded SHA and clear manual
   test steps. Preserve the operator's library and settings. Distinguish native
   startup/pixel evidence, unit tests and unqualified live capture/foreign apps.

## Research basis

- [GitHub native auto-merge](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/automatically-merging-a-pull-request): waits for required reviews/checks.
- [Repository auto-merge setting](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-auto-merge-for-pull-requests-in-your-repository).
- [Secure pull_request_target use](https://docs.github.com/en/actions/reference/security/securely-using-pull_request_target): privileged automation must not execute untrusted PR code.

## Done and limits

Delivery is not declared complete merely because a branch is pushed or auto-merge
is queued. Record PR/issue URLs, required checks, actual merge result and candidate
artifact identity. Report any remaining blocker plainly. Standards adoption,
redaction, full recorded watch/follow/execute and performance qualification remain
separate gates from these correctness/delivery improvements.
