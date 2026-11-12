# A1 — the AI boundary, made real (pass plan)

**Status:** approved 2026-11-11 ("Approve, start A1a"; Cursor: "Keep it for now"). Ruled in R301: "Run A1 next".
Parent: [`chat-native-program.md`](chat-native-program.md) §4.3 and §4.4. This file
is deleted when A1 lands; its evidence goes to the PRs and `status.md`.

## 1. Context
The chat plan puts providers and subscriptions under `crates/ai/*` (§4.3.1, the
operator's ruling of 2026-10-11). The code never moved: everything still lives in
`crates/chat/*`, and CW1 was built in `crates/chat/providers/codex`. Plan and code
disagree, which R301 forbids ("ONE FUCKING SOURCE"). A1 makes the code match the
plan with **behaviour unchanged**: no regression, no shims, no aliases, no barrel
re-exports, nothing kept as "legacy".

## 2. Goal / non-goals
**Goal.** After A1, `crates/ai/*` owns every provider and subscription concern, and
`crates/chat/*` owns only chat. The structure audit enforces the line.

**Non-goals.**
- No new feature, no UI change. Every screen is pixel-identical before and after.
- The August branch's pieces (carousel, + menu, modes, `model/list`, connectors,
  brainstorm) land after A1, in their own passes.
- Retiring `chat/providers/cursor` (R33) changes what the Providers control shows,
  so it is NOT in A1. It moves like the others, and its retirement is its own ruling.

## 3. Ground truth (read 2026-11-11 on `main` 1807459)
Reverse dependencies, from every `Cargo.toml`:

| Crate today | Used by | A1 destination |
|---|---|---|
| `dpl-chat-accounts` | app | `crates/ai/accounts`, `dpl-ai-accounts` |
| `dpl-chat-profiles` | app, chat/window | `crates/ai/profiles`, `dpl-ai-profiles` |
| `dpl-chat-browser-auth` | app | `crates/ai/browser-auth`, `dpl-ai-browser-auth` |
| `dpl-chat-imagine` | app | `crates/ai/imagine`, `dpl-ai-imagine` |
| `dpl-chat-{codex,claude,grok,gemini,cursor}` | app, chat/testkit, xtask (codex) | `crates/ai/providers/<v>`, `dpl-ai-<v>` |
| `dpl-chat-core` | 16 chat crates, app, **fabric** | split: provider vocabulary to `dpl-ai-core`, chat events and slash commands stay |
| `dpl-chat-runtime` | app, codex, grok, testkit, window | split: the `seam.rs` port to `dpl-ai-core::adapter`, the registry to `dpl-ai-host::registry`, chat state to `dpl-chat-store` |
| `dpl-chat-store` | app, 4 providers, window | split: vendor history decoders and usage dialects into each provider; chat's own conversations stay |
| `app/src/{presence,ai_usage,ai_backfill}.rs` (491, 296, 531 lines) | app | `dpl-ai-host` |

Shortcuts found: `crates/fabric/src/lib.rs:12` re-exports `Capability` and
`ProviderId` from chat-core (`pub use`), which A1 removes. Callers import from
`dpl-ai-core`.

`architecture.md` names chat crates in 6 places and the xtask audit in 16. Both
change in the same commit as the code (structure law).

## 4. Design: three PRs, each behaviour-unchanged and gated
**A1a. Move the AI crates.** accounts, profiles, browser-auth, imagine and the five
providers move to `crates/ai/*` and are renamed `dpl-ai-*`. Every caller's `use`
paths change in the same commit. No code inside them changes beyond paths.

**A1b. Split the mixed crates.** Author `dpl-ai-core` (§4.3.2 modules, holding only
what exists today: provider, capability, presence, activation, session, usage,
history, approval, adapter) and `dpl-ai-host` (registry). Move the AI half of
`chat/core`, `chat/runtime` and `chat/store` there, delete the moved code from its
old home, and drop fabric's barrel. Apply the §4.3.5 rename map to what moves.
Three-tier `Event`, `Participant` and `census` types are authored only where code
already needs them. Anything else waits for the pass that uses it.

**A1c. Move the app's AI files.** presence, usage and backfill move from the app into
`dpl-ai-host`. The app keeps wiring only. Grok's own host path joins the shared
registry, as §4.3.5 rules.

Naming: the §0.6 rows for `adapter`, `census`, `activation` and `participant` are
added to `design-system.md` before the code that uses them. "seam" is not used
under `crates/ai/`.

## 5. Wiring
The app is the only crate that wires AI to chat. After A1, no `crates/chat/*` crate
depends on a provider crate, and no `crates/ai/*` crate depends on a chat surface.
The structure audit gains that rule in A1b, when it can hold: today `chat/window` still uses `dpl-ai-profiles` and `chat/testkit` uses the providers.

## 6. Proof (each PR)
- Full `cargo green` locally and on both hosted lanes.
- The chat drills unchanged: `--chat-open`, `--chat-authored`, `--chat-proof`, and
  the `chat_native_proof` binary.
- Real-exe screenshots of home, chat and the Providers control at 1440x900 and
  1100x700 against the previous build, pixel-identical apart from the build stamp
  and animated backdrop.
- A grep proves: no `dpl_chat_` path to a moved crate remains, no `pub use` barrel or
  `as` alias was added, and no vendor name remains under `crates/chat/`.
- The exe is rebuilt in `D:\projects\devpulselabs\target\release`.

## 7. Risks
- **Size.** A1b touches the 8,329-line chat window's imports. It is still a move,
  but review cost is high, so it is its own PR.
- **The fabric edge.** `dpl-fabric` names provider ids in facts. Its exception moves
  from chat-core to ai-core in `architecture.md`, unchanged in meaning.
- **Hidden behaviour in moved state.** If a split would change behaviour, A1 stops
  and reports instead of guessing.

## 8. Evidence
Filled in per PR.
