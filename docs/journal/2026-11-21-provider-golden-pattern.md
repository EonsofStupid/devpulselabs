# R302 — provider contract planning

## Done so far

Documentation-only pass requested by the operator. Added
`docs/plans/chat-provider-golden-pattern.md` and linked it from the doc map,
native chat program, requirements and status. Application changes from R301
remain uncommitted and were neither reverted nor extended in this pass.

Grounded by reading core/registry/runtime/context types, account/profile stores,
provider modules, host dispatch, roots, Social project identity, R292/R301,
provider-runtime ADR and ADR-017. The existing neutral `ProviderRuntime` is the
foundation; Grok still has a separate host path despite an existing seam adapter.
Claude/Gemini/Cursor do not have equivalent qualified live implementations.

Critical correction: R292 explicitly separates the working project from the
executable/media library. Recent `PlatformContext::current()` uses the build
folder/library and its filename as project metadata, and `for_platform()` uses
one userdata conversation archive. This is not proof of platform project
ownership. The earlier live Codex response and green gate tested narrower facts.
Status now records this limitation explicitly.

The plan specifies minimum identity/discovery/auth/capability/project/session/
model/options/role/turn/usage/limits/tool/failure capture, provenance and absence
states, existing file owners, provider admission sheets and shared tests.
It preserves all archive data and forbids creating another chat project authority.

## Exact next step

P0 in the plan: establish the actual platform working-project owner/API from
selection through stable identity to workspace/archive association, including
the existing R292 history. Demonstrate two different projects sharing one
installation/library in the ownership map before changing consumer bindings.
If that owner is missing in this checkout, record the missing implementation
and propose its owner explicitly. Do not repeat the directory-name inference.

## Landmines

- The prior uncommitted code is still present; do not confuse plan with repair.
- Chat archive ProjectId, Social ProjectId and EguiPilot ProjectId are distinct.
- `RuntimeCapabilities` currently grants start/stream/interrupt on connection;
  the new plan requires individual evidence before offering capabilities.
- Provider observations in `app/presence.rs` still have a per-user/PATH fallback;
  reconcile passive inventory versus the approved production resolver boundary.
- Exact Meta Muse product/client identity is unresolved; it does not block
  specifying the common contract or investigating platform ownership.
- Older journals and proof screenshots remain preserved. Do not delete them.

## Verification

This pass uses source inspection and documentation-link/diff checks. No tests,
build, UI action, new provider probe, archive migration or project-data edit is
claimed. The implementation gate belongs to the subsequent code cut.
