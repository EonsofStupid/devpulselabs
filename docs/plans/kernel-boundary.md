# The kernel boundary — AI as a platform capability, not a chat feature

> "all AI related code/crates will be separated and associated to the kernel
> boundary. The editor, the platforms kernel will be the engine level and then
> interfaces" — the operator, 2026-08-04

Three shapes were confirmed with the operator before any code moved: the
**editing engine** is kernel-side while the editor UI is an interface; the AI
contract gets its **own kernel-side crate**; and the **boundary lands before
the absorption pass**, not after.

## 1. The proof that the arrow points the wrong way

`crates/fabric/src/lib.rs:12`:

```rust
pub use dpl_chat_core::{Capability, ProviderId};
```

**The platform's signal bus re-exports its AI vocabulary from a chat crate.**
Every consumer of the fabric — the shell, the seam, every hub — reaches AI by
way of `dpl-chat-core`.

That single line is the whole problem stated in code:

- **AI is a dependency OF the platform core**, rather than a capability
  attached AT its boundary.
- It is why `Fabric::providers_that_can()` has **zero production callers**
  (`fabric/src/lib.rs:795`; its only two callers are the fabric's own tests).
  A hub that wants to ask "who can stream turns?" must first depend on the
  chat tree, so no hub asks.
- It is why "AI is available in the whole platform" is aspiration and not
  structure.

## 2. The three tiers

```
INTERFACES      chat/window · hubs/* · shell · editor UI · composer · deck
                     ↕  intents out, views in — never types down
════════════════ KERNEL BOUNDARY ═══════════════════════════════
AI SUBSYSTEM    the contract  ·  the registry  ·  the adapters
                presence  ·  auth  ·  the roster       ← attaches HERE
════════════════ KERNEL (engine) ═══════════════════════════════
                fabric (signals) · estate (Ring-0) · intent (gate)
                hub (routes) · doc + comp (document truth)
                the editing engine (mutate · undo · validate, egui-free)
```

**The rule that makes it a boundary and not a diagram:** an interface may
depend on the kernel and on the AI subsystem. **Neither the kernel nor the AI
subsystem may depend on an interface**, and the kernel may not depend on the
AI subsystem — AI attaches *to* the boundary, it does not sit under it.

That is checkable, so the `xtask` structure audit gets a rule for it (K3).

## 3. K1 — `dpl-ai-core`, the contract

A new egui-free crate holding what the platform needs in order to know an AI
provider exists and what it can do. Everything below moves out of
`crates/chat/core/src/lib.rs`.

| moves | why it is not chat |
|---|---|
| `ProviderId` (`static_id!`, :47) | the identity every layer names |
| `ProviderAccent` (:235) | provider identity, used by the shell status bar today |
| `ProviderDescriptor` (:244) | label, glyph, tenure — identity, not conversation |
| `Capability` (:262) | **the question a hub asks.** The whole point |
| `PresenceSpec` (:392) · `PresenceVerdict` (:413) | is the vendor installed and signed in — an OS fact |
| `ProviderManifest` (:451) | registration |
| `DeviceAuthorizationPrompt` (:460) · `ActivationState` (:467) | the auth lifecycle |
| `AccountSummary` (:560) · `RecoveryAction`/`Kind` (:566,:575) | the account, not the chat |
| `GuidedValue<T>` + `Source`/`Confidence`/`Sensitivity` (:121–:150) | **see §3.1** |

And from `crates/fabric/src/lib.rs`: `ProviderPresence` and
`ProviderCapability` — the roster row the fabric folds. The fabric keeps the
FOLD (that is signal machinery); the SHAPE moves to the contract.

### 3.1 `GuidedValue` is a kernel primitive, and it is the important one

```rust
pub enum GuidedValueSource      { Provider, Route, Session, Operator, Ai }
pub enum GuidedValueConfidence  { Authoritative, Suggested(u8) }
pub enum GuidedValueSensitivity { Public, Private, TransientSecret }
```

Provenance, trust, and **whether a value may cross a persistence boundary**.
Nothing about it is chat-specific, and two standing laws are already exactly
this type:

- *"keys are never at rest"* — `chat/imagine`'s `XAI_API_KEY` read at call
  time — is `Sensitivity::TransientSecret`.
- *"context is only available when I want it to be"*
  (`chat-as-coworkspace.md` §4) needs provenance on every seeded value, which
  is `Source` + `Confidence`.

Today an interface can only honour those by remembering to. Kernel-side, the
type carries it.

### 3.2 What STAYS in `dpl-chat-core`

`ChatDomain` · `ChatSessionId` · `ChatEventId` · `ProviderSessionId` ·
`TurnId` · `SessionState` · `SessionGroup` · `SessionSummary` ·
`ChatEventContent` · `ChatEventKind` · `EventPresentation` · `ChatEvent` ·
`BoundedNativePayload` · `TokenUsage` · `MAX_NATIVE_PAYLOAD_BYTES`.

Conversation. A hub asking for a provider should never have to learn what a
turn is.

### 3.3 Deliberately deferred

The `Content*` types (`ContentEpisode`, `ContentSpine`, `ContentView`, :57–:112)
sit in `chat/core` and are document territory — `dpl-doc`'s, arguably. Moving
them is a separate decision with its own consumers, and bundling it into K1
would make the boundary pass about two things. Recorded, not done.

## 4. The links

| link | what | risk |
|---|---|---|
| **K1** | Extract `dpl-ai-core`. Move the §3 types; re-point every `dpl-chat-*` import. | mechanical, wide |
| **K2** | `dpl-fabric` depends on `dpl-ai-core`, not `dpl-chat-core`. The roster shape moves; the fold stays. Removes the recorded exception in `architecture.md` §3. | small, high value |
| **K3** | `architecture.md` gains the boundary; the `xtask` audit gains the rule that enforces it — kernel and AI crates may not depend on an interface. | the rule is what stops it rotting |
| **K4** | The editing engine: lift document mutation out of `dpl-editor`/`dpl-composer` into an egui-free kernel crate. | needs its own read-first pass |
| **K5** | The first cross-hub AI consumer — a hub calls `providers_that_can` and gets an answer. Proves the boundary carries load. | the point of all of it |

Then, and only then, the absorption pass `X1`–`X6`
(`crate-consolidation.md`) — into the shape the kernel defines, so the five
adapters are folded once rather than folded and then re-pointed.

## 5. What this pass will NOT do

- **No behaviour change.** K1–K3 are a move and a re-point. The app paints
  exactly what it painted.
- **Nothing deleted.** Same law as the absorption pass: every line lands
  somewhere it belongs.
- **No new interface.** K5 is the first consumer, and it is one call.
