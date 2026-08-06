# Separating AI from chat (and making room for social)

Branch: `refactor/ai-chat-social` off `main`.
Amends: `docs/design/architecture.md` §1–§4 + the xtask audit, same commit.

## 1. Context

The operator, 2026-08-04:

> "there is no clear separation in the code the UI can stay like this but i
> need chat to be separated from AI since it also powers the platform and
> chat as one and we have social to extend too later"

That is exactly right, and the code already agrees — the FOLDERS do not.

Everything AI lives under `crates/chat/` with a `dpl-chat-` prefix, which
says *chat owns AI*. The truth is the reverse: **AI is a platform capability,
and chat is one consumer of it.** Social will be the second.

This has been quietly distorting decisions all session:

- ADR-017 argued "provider capability is a PLATFORM fact" and then put the
  registry in crates named `chat-*`. The ADR was right; the packaging
  contradicted it.
- The `ProviderRuntime` seam — the thing the PLATFORM talks to providers
  through — landed in `dpl-chat-runtime`.
- `dpl-chat-core` holds `ProviderId` and `Capability`: platform vocabulary
  wearing a chat name.

**This pass is a RENAME, not a rewrite.** §2 proves that.

## 2. Current state

Ground-truthed in-session 2026-08-04.

### 2.1 The separation already exists in the dependency graph

Checked every AI crate against every chat-surface crate:
**zero AI→surface dependencies.** The direction is already correct.

### 2.2 The egui line falls exactly on the intended boundary

| bucket | crates | LOC | crates declaring egui |
|---|---|---|---|
| **AI** — core · runtime · accounts · browser-auth · profiles · store · imagine · providers/{claude,codex,cursor,gemini,grok} | 12 | ~11,700 | **0** |
| **chat surface** — window · windows · renderers · context · interactions · extensions · testkit | 7 | ~8,400 | 4 |
| **social** — social | 1 | 419 | 0 |

**The AI layer is already egui-free and already standalone.** Nothing has to
be engineered to make that true; it is true today, in the wrong folder.

### 2.3 Only TWO crates outside `crates/chat/` consume it

- `crates/app` — 17 of them.
- `crates/fabric` — `dpl-chat-core` only (the ratified exception,
  `architecture.md:65`).

`crates/shell` consumes none. So the blast radius outside the tree is two
manifests.

### 2.4 What the gate already knows

`crates/xtask/src/main.rs:698` carries the **egui-free census** and lists 13
chat crates. The surface crates are correctly absent.

**Gap found while reading it: `dpl-chat-cursor` is NOT in the census.** It
was added by a parallel session and never registered, so nothing stops it
growing an egui dependency. This pass adds it — a real defect surfaced by
the audit, not by the rename.

### 2.5 The one crate that is genuinely mixed

`dpl-chat-core` (1034 LOC) holds BOTH:

- AI vocabulary — `ProviderId`, `Capability`, `ProviderManifest`,
  `ActivationState`, `ChatEvent`, `SessionSummary`, `PresenceSpec`;
- the `ContentView` types D1 added — `ContentEpisode`, `ContentSpine`,
  `ContentChapter`, `ContentSlide` — which are chat's content seam and have
  nothing to do with providers.

Everything else is cleanly one side or the other.

## 3. Target design

```
crates/ai/       the platform's AI capability  — no egui, ever
crates/chat/     one consumer: the conversational surface
crates/social/   the second consumer, when it is extended
```

### 3.1 The rename map

| today | becomes |
|---|---|
| `chat/core` | `ai/core` → `dpl-ai-core` |
| `chat/runtime` | `ai/runtime` → `dpl-ai-runtime` *(carries the `ProviderRuntime` seam)* |
| `chat/accounts` · `browser-auth` · `profiles` · `store` · `imagine` | `ai/*` → `dpl-ai-*` |
| `chat/providers/{claude,codex,cursor,gemini,grok}` | `ai/providers/*` → `dpl-ai-{name}` |
| `chat/{window,windows,renderers,context,interactions,extensions,testkit}` | unchanged folder, stays `dpl-chat-*` |
| `chat/social` | `social/core` → `dpl-social-core` |

`dpl-ai-` and `dpl-social-` follow §4's naming law (`dpl-` + folder), and
grouped folders are already mandated past ~3 crates.

### 3.2 `core` splits, minimally

- `dpl-ai-core` keeps the provider/conversation vocabulary.
- The `ContentView` family moves to `dpl-chat-context` (153 LOC, already
  chat's route/content state, already a `dpl-chat-window` dependency).

This is an extraction of ~5 plain-data types, not a split down the middle.

### 3.3 What does NOT change

- **No logic moves.** Not one function body. Every diff hunk is a name, a
  path, or an import.
- **The UI stays as it is** — the operator said so explicitly. `window` at
  6032 LOC is untouched beyond its imports.
- **No new crate, and none removed.** 20 in, 20 out.
- Provider resolution, the seam, the registry, presence — all unchanged in
  behaviour.

### 3.4 Why this is the right moment

It is a pure rename **only while the direction is clean** (§2.1). The first
AI crate that reaches up into a surface crate turns this into a real
refactor. It is clean today.

## 4. Changes, in execution order

Each step compiles before the next. The compiler finds every miss — that is
the safety property this pass is built on.

1. `git mv` the 12 AI crates to `crates/ai/**`; `chat/social` →
   `crates/social/core`. Paths only.
2. Rename in each moved `Cargo.toml`: `dpl-chat-x` → `dpl-ai-x` /
   `dpl-social-core`.
3. Root `Cargo.toml` — member paths + the `[workspace.dependencies]` names.
4. Every `dpl_chat_*` → `dpl_ai_*` import across the workspace.
5. `crates/app` + `crates/fabric` manifests (§2.3).
6. Extract the `ContentView` family from `ai/core` into `chat/context`
   (§3.2).
7. `crates/xtask/src/main.rs` — rename the 13 census entries **and add
   `dpl-ai-cursor`** (§2.4).
8. `docs/design/architecture.md` — §1 layer stack, the `chat/*` sentence,
   §2 ceilings, §3 the `dpl-fabric → dpl-ai-core` exception, §4 naming.
9. `docs/design/adr-017-*.md` — a note that the registry now lives in a
   crate whose NAME matches what the ADR always claimed.

## 5. Boundaries

🚫 **Never touch**

- Any function body. If a diff hunk is not a name/path/import, it does not
  belong in this pass.
- `crates/chat/window`'s UI. Operator: *"the UI can stay like this."*
- Behaviour of resolution, activation, presence, the seam, or the registry.
- The `crates/hubs/*` and flat-crate restructure — **the operator is driving
  that**, and colliding on the same files would be the worst possible
  outcome. This pass touches ONLY `crates/chat/**` plus two manifests.

⚠️ **Ask first**

- Splitting anything beyond `ContentView` out of `core`.
- Any new crate. There are none in this plan and there should be none.

**Out of scope:** `dpl-proc`'s absorption · the `chat-renderers` duplication
· Codex transport (#210) · the passthrough resolver.

## 6. Verification

1. `cargo clean -p` every renamed crate, THEN
   `cargo clippy --workspace --all-targets -- -D warnings`. Cold, because a
   warm gate has hidden a rename's fallout twice this session.
2. `cargo green` → `green ✔`. The `structure` phase is the real test: the
   egui-free census must pass with the NEW names, which proves the AI layer
   is still egui-free after the move.
3. **The diff is the proof.** `git diff --stat` must show no line count
   change beyond imports, and `git log --follow` must survive on the moved
   files — `git mv` preserves history and a copy-delete does not.
4. Full drill sweep, all of which must be UNCHANGED:
   `chat-local-histories` · `chat-content-spine` · `chat-provider-tab` ·
   `providers-popup` · `providers-modal` · `platform-sees-providers` ·
   `chat-assist-toggle`.
5. Proof shot of Chat at 1300×800, identical to the pre-rename shot. A
   rename that changes a pixel is not a rename.

## 7. Commit plan

Branch `refactor/ai-chat-social`. ONE commit — a rename split across several
is unreviewable and unrevertable.

```
refactor: AI is a platform layer, not a chat detail
```

`gh pr list` FIRST, and **land or close #210 before starting** — it edits
`chat/providers/codex`, which this pass renames out from under it.

## 8. Evidence

Filled during execution. §6.3 is the honest measure: if the diff contains a
changed line of logic, the pass exceeded its mandate and must be split.

**Recorded before any code:** the reason this is cheap is §2.1 — zero
AI→surface dependencies. If that check had failed, this plan would have been
an argument for a real refactor instead of a rename, and the estimate would
not have been days.
