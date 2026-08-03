# Chat as the platform's co-working space — the foundation

The operator's correction of 2026-08-03, after C1 landed. It changes what
Chat IS, so it is written down before anything else is built on top.

> "The chat is essentially a cowork space to have with the platform. Once any
> of these providers subscriptions become active in the sidebar and become
> active for the entire platform. AI is available in the whole platform not
> only chat."

> "When a subscription becomes active their button becomes available in the
> sidebar so you can select and see all of your sessions … I would simply be
> able to click on another and it would extend down in that sidebar all of the
> sessions had overtime it shows general chats and then per project."

> "the API all stays — its the same concept as the subscriptions but for any
> active API keys."

> "I need to know patterns and configurations that are common now for
> general/project AI storage and configurations for ensuring i have clean
> paths to ensure context is only available when i want it to be for that
> project or if a general discussion."

## 1. What is ALREADY true — with proof

Four things the operator asked for are further along than they look.

| the ask | what exists | where |
|---|---|---|
| activation crosses to the platform | P1's roster is published on the fabric and read by the shell status bar AND serialized onto the agent seam | `shell/src/lib.rs:143,165,217` · `seam/src/lib.rs:497` (`roster_json`) |
| the accordion rail | **built, shipped, and ratified** for the hub rail — `open_house`, per-house unfold factor, clip-reveal from the top so rows below are pushed down, ONE open at a time | `shell/src/lib.rs:16,338` · `docs/plans/l1a-accordion-rail.md` ✅ |
| per-project sessions | every indexed transcript already carries its `workspace`, and the store can already filter by it | `chat/store/src/lib.rs:57` (`workspace: String`), `:228` (`list(workspace_filter)`) |
| API keys never at rest | the precedent is set and documented — read from the environment at call time, "never stored, never logged, never part of an error" | `chat/imagine/src/lib.rs:28` · `chat/accounts/src/lib.rs:1-5` |

**The accordion is the biggest one.** The operator described it here as if it
were new; it is the same interaction they already specified for the hub rail
in L1, and it is already implemented once. This is the third time a pattern
built for one surface needs generalizing — see §6.

## 2. What is NOT true — the gaps, named

### 2.1 The platform can ask, but never does

`Fabric::providers_that_can(capability)` exists (`fabric/src/lib.rs:795`) and
has **zero production callers** — its only two callers are the fabric's own
tests at `:1319` and `:1323`.

So the platform *sees* providers and cannot *use* them. "AI is available in
the whole platform" is the gap between a published fact and an offered
capability, and it is exactly one link wide.

### 2.2 The chat rail still has the megamenu the hub rail already fixed

`focused_provider_tab()` REPLACES the rail's contents: `provider_tab_header`
plus that one provider's sessions, with the roster gone
(`chat/window/src/lib.rs`, the `ai_sidebar` match). Getting back needs a
`‹ All providers` row.

That is precisely what L1 rejected:

> "The rail is wrong — it behaves like a megamenu. Wanted: … its subroutes
> appear and EXTEND DOWN, pushing the others down."

And there is a SECOND navigation for the same thing: the top strip mints a
tab per activated provider (`PROVIDER_TAB_ID`, `SelectProviderTab`,
`connected_tabs`). Two competing ways to scope to one provider, neither of
which is the accordion.

### 2.3 Sessions have a project, and nothing groups by it

`workspace` is rendered as a subtitle string in two places
(`lib.rs:2438`, `:2625`) and grouped by in none. The session groups are
`PINNED` / `ACTIVE` / `RECENT` / `RECENT CHATS` — lifecycle, not project.

"general chats and then per project" is a grouping function over data that is
**already indexed**. No new IO, no new store, no migration.

### 2.4 Chat has no cache at all

`chat/store` re-reads and re-decodes transcripts on every call — there is no
`Cache` anywhere in it. moka is in the tree but used **only** by
`dpl-estate`, and pinned there by a dependency ceiling
(`xtask/src/main.rs:671`).

## 3. moka — an honest correction

**moka is not the Rust Redis, and that is a good thing here.**

- **Redis / Dragonfly** are out-of-process, networked key-value *servers*.
  Their reason to exist is sharing state between processes and machines.
- **moka** is an in-process concurrent cache for Rust, modelled on Java's
  Caffeine: bounded size, TTL/TTI expiry, concurrent reads without a global
  lock.

DevPULSE is a desktop application. There is no second process to share cache
with, so running a server would add a daemon, a socket, a port, a startup
ordering problem and a new failure mode — in exchange for nothing.

**So moka is the right choice, for a different reason than "it is the Rust
Dragonfly":** the chat store currently has *no* caching layer, and moka is
the bounded, expiring, concurrent one that the estate already proves in this
codebase. Adopting it is a ceiling amendment, made deliberately.

Where the other two land, for the record: `chat/accounts` is **Turso**
already (activation evidence, non-secret). **DuckDB** is not in the chat tree
at all and belongs to analytics over transcripts, not to serving them.

## 4. The context-scoping model — what the industry actually converged on

Researched against primary sources rather than recalled. The dominant pattern
(Claude Code's memory and MCP configuration, which Cursor/Copilot mirror) is
**three independent axes**. Most homegrown designs collapse them into one and
that is the mistake to avoid.

### Axis 1 — SCOPE: whose context is it

| scope | location | shared with |
|---|---|---|
| managed / org | `/etc/claude-code/CLAUDE.md` | everyone on the machine, cannot be excluded |
| user | `~/.claude/CLAUDE.md` | just you, every project |
| project | `./CLAUDE.md` or `./.claude/CLAUDE.md` | the team, via version control |
| local | `./CLAUDE.local.md` | just you, this project, gitignored |

**Two different merge semantics, and this is the subtle part.** *Instructions*
CONCATENATE — every discovered file is added, ordered filesystem-root down to
the working directory, and a nearer file is read last rather than replacing a
farther one. *Configuration* (MCP servers) OVERRIDES by precedence — local ▸
project ▸ user — and "the entire server entry from that source is used;
fields are not merged across scopes."

Prose accumulates. Wiring resolves. A design that picks one rule for both
will be wrong for half its content.

### Axis 2 — TRIGGER: when does it load

**This is the operator's actual question.** Scope says *whose*; trigger says
*when*, and only the trigger axis can make context conditional.

- **always** — no condition, in every session
- **path-scoped** — `.claude/rules/*.md` with YAML frontmatter
  `paths: ["src/api/**/*.ts"]`, loaded **only when a matching file is
  touched**
- **on demand** — subdirectory files load when that directory is read; topic
  files are read only when needed; skills load when invoked

The index is deliberately bounded: `MEMORY.md` loads its first 200 lines or
25 KB, whichever comes first, and detail is pushed into topic files that are
not loaded at startup. **A context store with no size ceiling on its entry
point is a context store that silently stops loading.**

### Axis 3 — TRUST: who could have put it there

The rule worth stealing outright: **an import inside a project file whose
path resolves OUTSIDE the working directory raises an approval dialog**,
because a teammate can commit one. Imports in *user*-scope files load
silently — you wrote those yourself.

The same shape governs MCP: project-scoped servers from `.mcp.json` are
`⏸ Pending approval` until reviewed, and rejections are recorded.

**And secrets are never in the config.** `.mcp.json` carries `${VAR}`
expansion so a team can share the wiring while the value stays in the
environment. That is exactly DevPULSE's existing `XAI_API_KEY` rule —
already right, and now confirmed as the industry answer rather than a local
habit.

### 4.1 What this means for DevPULSE, concretely

| the operator's words | the mechanism |
|---|---|
| "for that project" | Axis 1 project scope, keyed by the session's `workspace` |
| "or if a general discussion" | a session with NO project — sees user scope ONLY, never a project's |
| "context is only available when i want it to be" | Axis 2. Scope alone cannot do this; conditional loading is a separate axis and must be built as one |
| "clean paths" | project identity derived from the **git repository root**, so worktrees and subdirectories of one repo share one context — never from the CWD string |
| API keys | never stored. The profile carries a `${VAR}` reference; the value is read at call time |

**The law that falls out of this:** a general session must never be able to
read a project's context, and a project session must never read another
project's. That is a containment boundary, not a filter — it belongs in the
type that addresses context, so it cannot be forgotten at a call site.

## 5. What must be DELETED

The operator was explicit that code gets deleted rather than shimmed.

1. **`SelectProviderTab` / `PROVIDER_TAB_ID` / `provider_tab` /
   `focused_provider_tab` / `provider_tab_header`** — the rail-replacing
   mode. It is the megamenu, and the accordion replaces it whole.
2. **`connected_tabs` in the top strip** — the per-provider tab strip is the
   second competing navigation for "scope to one provider". With the
   accordion in the rail it is redundant; the strip goes back to
   `AI · Content · Social`.
3. **`‹ All providers`** — an escape hatch that only exists because the rail
   was replaced. Nothing to escape from once rows unfold in place.

No aliases, no deprecation, no compatibility variant. Their pilot ids die
with them, and the scenarios that assert them are rewritten, not skipped.

## 6. The pattern-library law, third invocation

The accordion is built once in `crates/shell/src/lib.rs`. The chat rail needs
the same interaction. Per
`docs/plans/coldlight-naming-and-the-pattern-library.md` §2 it is extracted to
`cl::patterns::accordion` and BOTH rails call it — the same move already made
for `bleed_control`, which the shell and the chat rail now share.

Three surfaces have now needed the same primitive. That is the signal the law
was written for.

**Note:** the accompanying `trait`→`FramePatch` rename is WITHDRAWN — main
already carries THE MATRIX and traits are being replaced, not renamed. See
`docs/plans/coldlight-naming-and-the-pattern-library.md` §1.

## 7. The program

| link | what | why here | deletes? |
|---|---|---|---|
| **F1** | **The accordion, extracted and adopted.** `cl::patterns::accordion` out of the shell rail; the chat provider rows become sections that unfold in place. Deletes everything in §5. | The operator's described interaction, and the deletion cannot wait — every link after this would build on the megamenu. | **yes** |
| **F2** | **The project axis.** Sessions group as **General** and then per project, from `IndexedSession.workspace`. Project identity resolves from the git repository root, never the CWD string. | The data is already indexed; this is a grouping function and the shape the whole context model keys off. | no |
| **F3** | **The context boundary.** A `ContextScope` that a session carries — `General` or `Project(id)` — and which is the ONLY way to address stored context. General cannot reach a project; a project cannot reach another. | §4.1's law. It lands before any context is written, because a boundary retrofitted onto existing call sites is a boundary with holes. | no |
| **F4** | **Platform AI.** The first real consumer of `providers_that_can` outside chat — a hub asks the roster for a capability and gets an answer. | §2.1. Until one exists, "AI in the whole platform" is a published fact nobody reads. | no |
| **F5** | **The chat cache.** moka in `chat/store`, bounded and TTL'd, with the ceiling amended deliberately. | §3. Lands after F2/F3 so the cache key includes the scope and cannot leak across it. | no |
| **F6** | **API keys as first-class.** An active API profile behaves exactly like an active subscription: it appears in the rail, unfolds its sessions. Keys stay `${VAR}` references. | The operator's "the API all stays — same concept as the subscriptions". | no |

**F1 first, and it deletes.** Every other link paints into the rail F1
defines; building them against the megamenu means building them twice.

## 8. What this changes about C1

C1's rail is right in shape and incomplete in behaviour. Its provider rows
select but do not unfold. That is an extension, not a rewrite — the row
becomes the accordion's section header.

C1's own §6 debt is unchanged and still owed: the selection burst, the
sporadic chip hover, and registering the four patterns into the slides
element census so the Library wall shows them.

## 9. Open, and deliberately not decided here

Whether project context lives as **files on disk** (the industry answer, and
inspectable) or **rows in Turso** (queryable, and already in the tree) is
F3's decision, made against the code as it stands then. The scoping model
above holds either way — which is why it is settled first.
