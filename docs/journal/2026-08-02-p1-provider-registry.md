# 2026-08-02 — P1: activated providers become platform capabilities

Plan: `docs/plans/chat-providers-become-platform.md`.
ADR: `docs/design/adr-017-provider-capability-registry.md`.
Programs: `docs/plans/chat-operations-console.md` (link P1) and
`docs/plans/providers-presence-and-the-brain.md` (which this supplies).
Mock: `docs/design/mocks/providers.html`.

## What the operator asked for

> "chat needs to be built so its properly yes its own window but the
> providers that get activated in the chat become available in the platform"

## Done

**The finding that shaped the pass.** The platform could see that
*something happened* to a provider and could not see what any provider can
**do**. `ChatRuntime` is a private field of `ChatWindow` — a SURFACE — and a
workspace grep for `ActivationState` / `Capability::` outside `crates/chat/`
returned only the chat host itself. Zero platform surfaces could see a
provider at all.

The provider CHANNEL cannot answer this, by construction:

- `ProviderEvent` is fourteen variants of ONE activation attempt's progress.
  H1's providers activate by having transcripts on disk, so no attempt runs
  and no `Ready` is ever published. A history-activated provider never
  appears on the channel.
- A broadcast has no past. `subscribe_providers()` hands back a
  `broadcast::Receiver`, so a surface that subscribes late hears nothing
  about what activated before it existed.

**So THE ROSTER is a fold**, the same shape as THE JOBBOARD that already
sits beside it in `crates/fabric/src/lib.rs`: an `Arc<Mutex<Vec<..>>>` every
`Fabric` clone shares, replaced wholesale by the single publisher that always
knows the whole truth, readable with no subscription and no history. No new
mechanism was invented, and **no new dependency edge** — `dpl-fabric →
dpl-chat-core` is the ratified named exception in `architecture.md:65`, so
`Capability` crosses on an edge that already existed and the `structure`
audit had nothing to fire on.

**A capability SET, not `can_send`.** Workflows route a beat to whoever can
take it, A/B runs name every provider that can take the SAME beat, and
capture needs a stable subject. `can()` derives the booleans; storing the
reading instead of the fact is what ADR-017 §4 rejected.

**Three projections, one fact.** In-memory fold → `Tree::Agents` under
`providers/roster` (envelope `{written_at_ms, providers}`) → the seam's
`/state` under `registry.providers`, in stable capability slugs.

## Two things the code decided

1. **`ProviderId` is `&'static str`** (`chat/core/src/lib.rs:47`, via
   `static_id!`). A stored roster cannot be deserialized back into one
   without leaking, so restore RESOLVES stored names against the ids this
   build registers and drops the rest — the doctor law, applied to the
   estate. Stored capability slugs resolve the same way.
2. **`Capability` had no stable wire name**, only a human `label()` free to
   be reworded. `slug()` / `from_slug()` / `ALL` joined it in
   `dpl-chat-core` rather than becoming a second stringify table in the app
   crate.

**Publishing is change-detected.** Activation is mutated from fourteen
`set_provider_activation` sites plus the provider-event drain; any
enumeration of "activation-affecting" intents is one new call site away from
being silently wrong. `republish_roster` compares the projection and
publishes only on a difference.

## Proof

`tests/eguipilotai/platform-sees-providers.toml`, fixture `chat` (live
runtime), asserts `root/shell.status.providers` — the ROOT viewport, not
`chat.primary` — is visible and names Claude. All five steps green under
`devpulse-cloud`. Shot: `docs/proof/platform-sees-providers.png`, MAIN at
1440×900, status bar reading **`AI · Claude Code`** — a provider the platform
knows about only because there are transcripts on this disk.

Fifteen unit pins across `dpl-fabric`, `dpl-chat-core`, `dpl-seam`,
`dpl-shell` and the app. `cargo green` ✔.

## Open

- `chat-local-histories`, `chat-content-spine`, `chat-provider-tab` and
  `shell-navigation` fail under Xvfb on the headless Linux box, and fail
  identically on `2459cdf` without this pass's code (baseline exe rebuilt and
  re-run to confirm). They need a lane with a real display.
- The status-bar text chip is the smallest honest consumer, and a
  placeholder: V1 of the Providers program replaces it with the control the
  mock draws — brain + "Providers" + the `ai` circle, bleeding over 5.6–6.3s.
