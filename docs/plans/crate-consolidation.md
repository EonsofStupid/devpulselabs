# Crate consolidation — the pre-release pass

> "i am in pre-release so crates need to get merged and this needs to be
> properly captured and implemented" — the operator, 2026-08-03

**73 crates · 110,155 loc.** The chat tree alone is **20 crates · 19,292 loc**.

## 0. Read this first, because it is what makes the pass safe

`docs/design/architecture.md` §1 defines a **layer stack** — DOCUMENT →
ENGINE → SUPPORT → SURFACE → HUB → SHELL → APP, dependencies pointing down
and never up or sideways. §2 pins nine **ceilings** with stated reasons, and
§3 records the standing exceptions.

**Those boundaries are the value, and most of them are not negotiable.** A
crate is not a merge candidate because it is small — `dpl-proc` is 41 lines
with **12 consumers** and a `∅` ceiling, because "the quiet-spawn law is
dependency-free". Merging it would delete the law, not the crate.

So this pass does not ask "which crates are small". It asks **which crate
boundaries carry no information.**

## 1. The criterion

A crate merges into `Y` only when ALL of these hold:

1. **Same layer.** An engine never merges into a surface; the egui-free census
   (`xtask/src/main.rs`, `EGUI_FREE`) is a hard line, and merging across it
   silently links egui into a crate that swore it wouldn't.
2. **One real consumer, and it is `Y`.** A crate used by two consumers is a
   shared contract; that is what a crate is for.
3. **No ceiling with a stated reason.** The nine in §2 of the architecture
   are decisions, not accidents.
4. **The merge adds no dependency `Y`'s layer forbids.**

Anything failing one of these stays. Every merge amends
`docs/design/architecture.md` and the `xtask` audit tables **in the same
commit** — that rule already exists and this pass obeys it.

## 2. DELETE — verified dead, not merged

Two crates appear **only in their own `Cargo.toml`**. No `use`, no
dependency edge, no test. Checked by scanning every manifest and every `.rs`
in the workspace.

| crate | loc | status |
|---|---:|---|
| `dpl-chat-renderers` | 419 | zero consumers, zero imports |
| `dpl-chat-extensions` | 254 | zero consumers, zero imports |

**673 lines compiled on every build for nothing.** They were also written up
in `docs/plans/chat-surface-map.md` as live surfaces, which was wrong — that
document is corrected by this pass.

Pre-release law: **deleted, not deprecated.** If the Extensions concept is
wanted later it is re-authored against the surface as it stands then, not
resurrected from a crate nothing ever called.

## 3. MERGE — the chat tree

### 3.1 `dpl-chat-context` (153) → `dpl-chat-core`

One consumer: `dpl-chat-window`. Deps `dpl-chat-core` + `dpl-hub`. Both it and
core are egui-free. It is route-context *state* — the same kind of thing core
already holds, split out for no boundary reason.

Cost: `dpl-chat-core` gains `dpl-hub`. `dpl-fabric` already carries a
`dpl-hub` edge as a recorded exception for the same reason (facts carry
`Route`), so the precedent and its rationale exist.

### 3.2 The five vendor adapters → `dpl-chat-providers`, one crate, five modules

| crate | loc | consumers |
|---|---:|---|
| `dpl-chat-cursor` | 90 | app |
| `dpl-chat-gemini` | 87 | app, testkit |
| `dpl-chat-claude` | 395 | app, testkit |
| `dpl-chat-codex` | 1561 | app, testkit |
| `dpl-chat-grok` | 3440 | app, testkit |

**5,573 loc across five manifests, all in one layer, all consumed by the same
two crates.** They share `dpl-chat-core`; four also share `dpl-chat-store`.
Adding a provider today costs a directory, a manifest, a workspace member, a
registry line and a ceiling review. It should cost a module and a registry
line.

This is the merge that matters: it removes four manifests and makes "add a
provider" a one-file change — which the pulse program (`PU2`) and the
operator's "the API all stays" both depend on being cheap.

**Not merged into core.** The adapters spawn processes and do IO; core is the
pure domain layer. The layer line holds.

### 3.3 `dpl-chat-testkit` (499) → dev-dependency only

`crates/chat/testkit` says "production never depends on it", but it sits in
`devpulselabs`' **normal** dependencies. It stays its own crate — fixtures
shared by several test suites are a real contract — but the edge moves to
`[dev-dependencies]` so the claim in its own header becomes true and the
release binary stops carrying it.

*Verify before doing:* if `app` uses it outside `#[cfg(test)]`, that is a
separate defect and this becomes its own pass.

### 3.4 Deliberately NOT merged

| crate | loc | why it stays |
|---|---:|---|
| `dpl-chat-core` | 1034 | 19 consumers — the domain contract |
| `dpl-chat-store` | 1111 | 6 consumers |
| `dpl-chat-runtime` | 770 | 4 consumers |
| `dpl-chat-interactions` | 396 | 4 consumers |
| `dpl-chat-social` | 419 | 4 consumers |
| `dpl-chat-profiles` | 556 | the API population; F6 grows it |
| `dpl-chat-accounts` | 401 | Turso boundary; credentials law lives in its header |
| `dpl-chat-browser-auth` | 387 | the system-browser handoff is a security boundary |
| `dpl-chat-imagine` | 390 | ADR-011; its own key/consent policy |
| `dpl-chat-windows` | 624 | viewport lifecycle, distinct from painting |
| `dpl-chat-window` | 6306 | the surface. **See §5** |

**Net for the chat tree: 20 → 14 crates**, 673 loc deleted, four manifests
removed.

## 4. Outside chat — candidates needing one more pass

Named, not scheduled. Each needs its consumer verified the way §2 was.

| crate | loc | consumers | note |
|---|---:|---|---|
| `dpl-vector` | 293 | 1 | `∅` deps, single consumer — likely merges |
| `dpl-backdrop` | 297 | 1 | SUPPORT layer, single consumer |
| `egui-pilot-ai-lab` | 449 | 1 | harness-side |

**Explicitly safe from this pass**, with the reason already written down:
`dpl-proc` (∅, 12 consumers), `dpl-intent` (∅, ceiling), `dpl-svg` (∅,
ADR-015 build-dep), `dpl-estate`, `dpl-comp`, `dpl-doc`, `dpl-workflows`,
`dpl-landing`, `dpl-tools-deps` — all ceilinged; `dpl-lottie-render` — the ONE
FFI leaf (ADR-009) with the `unsafe_code` allowlist; every `crates/hubs/*` —
a hub is one registry line by doctrine, and `dpl-hub-home` being 63 lines is
the pattern working, not a fault.

## 5. The one that is too big, not too small

`dpl-chat-window` is **6,306 lines in one file** — larger than every other
chat crate combined was before this pass. Consolidation that only merges
small crates and leaves this untouched has improved nothing an operator can
feel.

It is not a crate split — the surfaces belong together. It is a **module
split** inside the crate: `rail`, `conversation`, `composer`, `overlays`,
`providers`. Same crate, same public API, no manifest change, no ceiling
amendment.

Scheduled **after** F1, because F1 rewrites the rail and splitting a file
under an in-flight rewrite means merging the same code twice.

## 6. Order

| link | what | risk |
|---|---|---|
| **X1** | Delete `chat-renderers` + `chat-extensions`. Correct `chat-surface-map.md`. | none — nothing references them |
| **X2** | `chat-context` → `chat-core`; amend the architecture + audit tables | low |
| **X3** | The five adapters → `dpl-chat-providers` | **highest value, highest churn** — one crate, five modules, registry unchanged |
| **X4** | `chat-testkit` to `[dev-dependencies]` | verify first |
| **X5** | Verify and merge the §4 candidates | low |
| **X6** | Split `chat/window/src/lib.rs` into modules — **after F1** | mechanical |

Each link is one commit, `cargo green`, and amends the doctrine tables in the
same commit. X1 first because deleting dead code before moving live code
means less to move.

## 7. What this pass will NOT do

- **No new crates.** Consolidation that mints a crate has failed.
- **No layer changes.** If a merge needs the stack rearranged, the merge is
  wrong.
- **No ceiling removed silently.** Every one that changes is amended in
  `architecture.md` §2 and `xtask` in the same commit, with the reason.
