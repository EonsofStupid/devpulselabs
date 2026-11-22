# Reconcile before new work — 2026-11-22

Authorized by the operator: merge existing changes before starting anything
new and prevent work on an obsolete checkout. Local HEAD was 1807459; freshly
fetched origin/main is 67fe63f, eleven merged commits ahead (#321–#331).

## Delivery

1. Preserve every tracked/untracked draft and proof file in a local recovery
   commit before switching. Keep the snapshot available throughout integration.
   Do not touch library, provider credentials, existing archives or other repos.
2. Fast-forward local main to fetched origin/main. Reconcile the draft on a
   branch based on that main. Current AI ownership, catalog, trust grants,
   ColdLight chat UI and project/archive associations take precedence over the
   obsolete implementation. Account for each draft change explicitly.
3. Carry useful role/session popup, long-draft and provider-failure fixes into
   the current owners. Reuse current discovery; do not resurrect app/presence
   or chat-owned provider types. Exclude the known-invalid build/library
   project substitution and single-archive fallback. Retain original evidence
   as historical evidence, with limits made explicit. Reconcile requirement
   IDs and the provider contract with current main rather than overwriting it.
4. Add a baseline gate to existing developer tooling and session instructions:
   fetch the actual remote main; refuse stale/diverged working history or a
   failed fetch; expose exact head/base hashes; require integration before new
   work. Check again before delivery. Existing work is preserved, never reset.
   Session hooks must surface stale-baseline failures before normal planning.
5. Clean touched packages, run focused regressions and the full local gate,
   rebuild the real app, inspect native popup/long-draft proof. Freshly verify
   remote main immediately before creating/merging the PR. If it advances,
   reconcile and rerun affected checks before merge.
6. Push one reconciled milestone PR and merge under current R304 policy after
   the full local gate. Read main's hosted checks and fix failures. Fetch and
   fast-forward this checkout to the merged main; verify a clean working tree,
   matching local/remote HEAD and no unmerged milestone PR before finishing.

## Scope

This is integration and process repair, not a new provider implementation or
project-management rewrite. Existing Claude/Grok gaps remain explicitly recorded.
The prior green runs prove the old draft only; they do not qualify this merge.
No content is discarded: rejected implementation choices remain in the recovery
snapshot, and the final disposition explains why they are not production code.
