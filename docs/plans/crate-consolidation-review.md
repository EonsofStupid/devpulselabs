# The crate census — what exists, what earns its keep, what gets folded

Review, not a rename. The operator, 2026-08-06:

> "first lets just focus on reviewing existing crates and getting them
> properly consolidated and built"

Measured in-session from `cargo metadata --no-deps` + per-crate LOC +
workspace reverse-dependency counts. Every number below traces to that
census, never to intent.

## 1. The shape

**73 crates · 114,956 LOC.**

| area | crates | LOC | note |
|---|---|---|---|
| core/platform | 35 | 64,265 | includes the app (8,878) and composer (10,062) |
| chat | 20 | 20,546 | `window` alone is 6,032 — 29% of chat |
| pilot (egui-pilot-ai-*) | 7 | 12,624 | the semantic harness, a side rail |
| hubs | 6 | 10,482 | one crate per hub, by law |
| coach | 5 | 7,039 | added by a parallel lane |

The count is not the problem. **Nine of the 20 chat crates are under 420
LOC**, and the question for each is not "is it small" but "does its boundary
buy anything".

## 2. Findings

### 2.1 The transitive-egui leak — and its ACTUAL root

> **CORRECTED after reading the source.** This section first proposed
> splitting `dpl-hub`. That treats a symptom. The root is one type in the
> wrong crate, and `architecture.md` §3 already names the fix. Recorded
> rather than silently rewritten, because the wrong version was the
> recommendation for an hour.

189 lines, **15 consumers**, and it declares `egui` + `dpl-coldlight`. Inside:

- **~75 lines of pure routing data** — `HubId`, `Route`, `SubId`, `HubSpec`.
  This is what almost every consumer imports.
- **~110 lines of painting** — the `Hub` trait (`fn ui(&mut self, ui: &mut
  egui::Ui, ...)`) and a `surface()` hub-floor renderer full of
  `cl::clay()`, `cl::gradient_tint`, `RichText`.

**The chain, measured:** `dpl-chat-context` / `-extensions` / `-windows`
import **only** `Route`, `HubId`, `SubId` from `dpl-hub` — never `HubSpec`,
never the `Hub` trait, never `placeholder`. But `dpl-hub` declares `egui`
(for the `Hub` trait and the `placeholder` module) **and** `dpl-coldlight`
(because `HubSpec.tone: Tone`). Either edge is enough to link egui.

`dpl-coldlight` is the deeper one: it also drags egui onto `dpl-doc` and
`dpl-seam`, both egui-free by intent. `architecture.md` §3 already records
this and already ratifies the cure — *"Pulling the token IDS into a paint-free
crate would remove this edge — a ratified later pass, not a drive-by."*

**So the fix is the token-id extraction, not a `dpl-hub` split.** Splitting
`dpl-hub` would free three chat crates and leave `dpl-doc` and `dpl-seam`
exactly as they are; extracting `Tone` and its siblings into a paint-free
crate fixes all five at the root and needs no new boundary invented for it.

`placeholder` is separately self-retiring: its own doc says *"P0-ONLY… Delete
this module hub-by-hub as real surfaces arrive — never extend it."* Two users
remain (`hubs/wiredfront:1469`, `hubs/workbench:414`), so it goes when they
get real surfaces — no action here.

For the record, the original framing:

| crate | LOC | wants | gets |
|---|---|---|---|
| `dpl-chat-context` | 153 | `Route` | egui, transitively |
| `dpl-chat-extensions` | 254 | `Route` | egui, transitively |
| `dpl-chat-windows` | 624 | `Route` | egui, transitively |

`dpl-chat-extensions`'s own header says *"This crate intentionally exposes no
egui callback"*. True about callbacks, false about linkage.

**And the gate cannot see it.** The `structure` audit's census checks
**direct** deps only ("never link egui/eframe *directly*"), so a transitive
pull is invisible to it — and `dpl-chat-context`, `-extensions` and `-windows`
are not in the census list at all.

### 2.2 Four crates are missing from the egui-free census

`dpl-chat-cursor` · `dpl-chat-context` · `dpl-chat-extensions` ·
`dpl-chat-windows` are egui-free by direct deps but unregistered
(`crates/xtask/src/main.rs:678`), so nothing stops any of them growing a UI
dependency tomorrow. `dpl-chat-cursor` was noted as a gap once already and
is still unregistered.

### 2.3 `architecture.md` misstates one crate

§1 says *"window/windows/renderers are surfaces"*. `dpl-chat-windows`
declares **no egui** — its deps are `dpl-chat-core`, `dpl-chat-interactions`,
`dpl-hub`. It is viewport/registry logic, not paint. Only **two** chat crates
link egui directly: `window` (6,032) and `renderers` (419).

### 2.4 Two dead crates are one unbuilt feature, not two mistakes

Neither has a single reference outside itself:

- `dpl-chat-extensions` (254) — `ExtensionRegistry`, `ExtensionManifest`,
  `ChatSurface` (with a `ConversationRenderer` placement slot).
- `dpl-chat-renderers` (419, egui) — `EventRendererRegistry`,
  `EventRendererRegistration`, `RenderEvent = fn(&mut egui::Ui,
  &ChatEventContent, Tone)`.

Same architecture in both (`Id` / `Registration` / `Error` / `Registry`), and
`ChatSurface::ConversationRenderer` is exactly the slot the renderer registry
fills. **They are the two halves of one declarative extension system.**

Meanwhile `dpl-chat-window` hard-codes **24 `ChatEventContent::` match arms**
and renders every event itself. The registries are not stale ideas — they are
the design the window bypassed.

### 2.5 `dpl-engine-ae` is staged, not stale

684 LOC, zero consumers, and its header says why: *"Phase 1 is everything that
needs no running AE. Enumeration (Phase 2), rendering (Phase 3), and the
ffmpeg export selector (Phase 4) build on this."* Ground-truthed against
aerender 26.3x87 on 2026-07-25.

**Verdict: keep, untouched.** A staged phase with a written successor is not
dead code; deleting it would repeat the loss the operator has already called
out twice.

### 2.6 The other zero-consumer crates are binaries, and correctly so

`xtask` · `egui-pilot-ai-cli` · `dpl-lottie-render` (ADR-009 subprocess
helper) · `dpl-coach-daemon` · `dpl-coach-overlay` (devtools-only, ratified)
· `dpl-desktop-spike` (named orphan) · `devpulselabs` (the app). Nothing
depends on a binary. **No action.**

### 2.7 `dpl-proc` — 41 lines, 12 consumers, and a real reason

The operator flagged this: *"if its 41 lines we need to get this properly into
the crate it belongs."*

It is one function (`quiet`, the CREATE_NO_WINDOW law) with an **∅-dependency
ceiling** (`architecture.md` §2), which is *why* twelve crates can lean on it
— any crate at any layer may depend on a crate with no deps. Folding it into
an engine crate would give every one of those twelve that engine's dep tree.

**It is not the wrong size; it is the wrong SCOPE.** A 41-line crate named
"proc" that owns exactly one of the several OS-process laws this repo needs.
The unbuilt sibling is already known: `Command::new("codex")` cannot find
`codex.cmd`, because Rust appends `.exe` and ignores `PATHEXT` — the
passthrough resolver has nowhere to live, and A1's presence probe reports
`NotInstalled` for a Codex that is installed (witnessed in this session's
logs).

**Verdict: grow it into the OS-primitives leaf it should have been** —
quiet-spawn *and* PATHEXT-aware executable resolution — keeping ∅ deps. That
is consolidation by giving a crate its whole job, not by deleting it.

### 2.8 Provider crates are wildly asymmetric, and that is honest

`grok` 3,767 · `codex` 2,441 · `claude` 395 · `cursor` 90 · `gemini` 87.

The two stubs are placeholders holding the shape (descriptor + manifest +
capability targets). They are not consolidation candidates — they are the
seam's next two spokes. **No action.**

## 3. What consolidation actually means here

Ranked by value, not by size:

1. **Extract the token IDs from `dpl-coldlight`** into a paint-free crate
   (§2.1) — the already-ratified pass. Fixes `dpl-doc`, `dpl-seam`, `dpl-hub`
   and, through `dpl-hub`, the three chat crates, at the root. **Prerequisite
   for item 2**: censusing those crates as "egui-free" before this lands would
   write a half-truth into the gate.
2. **Widen the gate** to catch what §2.1 hid: census the four missing crates
   (§2.2) and check TRANSITIVE egui, not just direct.
3. **Correct `architecture.md`** on `dpl-chat-windows` (§2.3), same commit as
   the audit table, per the change protocol.
4. **Grow `dpl-proc`** into the OS leaf with the PATHEXT resolver (§2.7),
   which also unblocks the passthrough work.
5. **Wire the extension system** (§2.4) — `window` renders through the
   registry instead of 24 hard-coded arms. This is the largest and the only
   one that changes behaviour, so it is LAST and its own pass.

Nothing here is a deletion. Items 1–4 are structural and behaviour-neutral;
item 5 is a feature that was already written and never connected.

## 4. Boundaries

🚫 **Not in this review's scope**

- The `kernel` / `engine` / `interfaces` re-tiering. The operator asked to
  consolidate first; naming a tier before the crates are right would name the
  wrong things.
- `dpl-engine-ae` (§2.5) and the provider stubs (§2.8) — measured, keep.
- Deleting anything. Standing operator law: *"why delete why not work and get
  this built correctly."*
- `dpl-chat-window`'s 6,032 lines. Splitting the surface is a real question
  and a separate one; item 5 is what would shrink it honestly.

## 5. Verification for each item

- `cargo green` → `green ✔` after each, with `cargo clean -p` on touched
  crates first (a warm gate has hidden fallout twice this session).
- Item 1: the three freed crates must appear in the egui-free census and pass.
- Item 2: the widened audit must FAIL on a deliberately reintroduced
  transitive egui edge before it is trusted.
- Item 5: the chat drills — `chat-local-histories`, `chat-content-spine`,
  `chat-provider-tab`, `chat-codex-persisted-ready` — unchanged.

## 6. The honest summary

**73 crates is not the defect.** One crate doing two jobs (§2.1), a gate that
checks direct deps only (§2.2), a law file that is wrong about one crate
(§2.3), a feature built and never wired (§2.4), and a leaf that owns one law
when it should own two (§2.7) — those are the defects. The count is a symptom
people reach for because it is easy to see.
