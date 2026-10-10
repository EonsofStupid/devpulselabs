# One AI kernel — merging DevPULSE's chat layer and automaton, completely

> **SUPERSEDED 2026-10-10 — do not execute.** Operator: *"it should properly
> build chat from scratch inside the fucking app like i asked from the start
> and not just blindly copy that one but properly look at my requirements for
> it and get it actually planned and implemented."* Chat (and the AI layer it
> sits on) is built natively in this repo: `docs/plans/chat-native-program.md`.
> automaton (now `EonsofStupid/chatmaton`) and Shippin are NOT dependencies,
> sources, or sidecars (`docs/plans/external-reach-inventory.md` rows 1–3).
> Phase 2+ never ran here; nothing in this repo depends on `mato-*`. The
> text below is kept unedited as history.

Program (multi-pass), not a single change. Cross-repo:
`EonsofStupid/automaton` (the kernel) ⇄ `EonsofStupid/devpulselabs` (a consumer).

The operator, 2026-08-11: *"we want to merge the two as a complete right."*

## 1. Context

Two codebases grew the **same design** for multi-provider AI independently:

- **automaton** — a deliberate, multi-product KERNEL: abstract layer as SSOT,
  five CLI providers as adapters, a control-plane/entitlement contract, a
  signal journal, WezTerm/Zellij + panel + API as equal surfaces. Rust
  workspace (`mato-{acp,providers,kernel,cli,daemon}`) already namespaced
  `ProjectDirs::from("ai","DevPulse","Automaton")`.
- **DevPULSE `dpl-chat-*`** — a working chat layer with a live provider
  execution seam (`ProviderRuntime`, Codex app-server + Grok ACP spokes),
  capability qualification, a history store, and an egui peer-window surface.

They are the same pattern with **incompatible types** (§2.3). "Merge
completely" means: ONE canonical vocabulary + ONE execution seam + ONE event
contract, authored once in the kernel, consumed by DevPULSE and every future
product — not two registries that drift.

**This subsumes the never-executed `ai-chat-social-separation.md`.** That plan
would have renamed `dpl-chat-*` → `dpl-ai-*` internally; instead the AI layer
becomes the EXTERNAL kernel. One move, not two.

## 2. Current state (ground-truthed 2026-08-11)

### 2.1 automaton is a clean, dependable Rust workspace

`Cargo.toml`: members `mato-acp · mato-providers · mato-kernel · mato-cli ·
mato-daemon`, path deps, edition 2021, standard stack (tokio, serde, clap,
uuid, tracing). Git-dependable from a private repo (server1 + estate have
auth). The Node `engine/` is a SEPARATE prototype — NOT in the cargo
workspace — where all five providers' `start()` was proven fast. The Rust
side is the durable half.

**What each Rust crate is** (measured by `Command::new` count + headers):
- `mato-providers` — the STATIC registry: `capability`, `identity`, `roster`,
  `readiness`, `detect` (25 KB), `preflight`. Spawns nothing.
- `mato-kernel` — "single writer for provider roster + signal journal"
  (`SignalBus`, roster.json, signals.jsonl, tracing). Spawns nothing.
- `mato-acp` — the ONE Rust execution path (ACP/JSON-RPC, 3 spawns).
- `mato-cli` / `mato-daemon` — thin surfaces (daemon `main.rs` is 1.8 KB).

### 2.2 DevPULSE is still `dpl-chat-*`, and its execution is MORE built

`crates/chat/`: `core · runtime · store · providers/{claude,codex,cursor,
gemini,grok} · accounts · browser-auth · profiles · interactions · imagine ·
window/windows/renderers · context · extensions`. No `crates/ai` — the
separation never ran.

DevPULSE's `ProviderRuntime` seam has **two live Rust execution spokes**
(Codex app-server, Grok ACP) — automaton's Rust execution is ACP-only. So
DevPULSE brings the stronger EXECUTION half; automaton brings the stronger
REGISTRY/SIGNAL/multi-product half. **Complementary, not redundant.**

### 2.3 The incompatibility that makes this a real merge, not a swap

**Two `Capability` taxonomies, neither a superset:**

| automaton (15) — permission/feature | DevPULSE (22) — agent session |
|---|---|
| TextInput, ImageInput, Streaming, ToolUse, StructuredOutput, Resume, UsageReporting, ShellRead, ShellWrite, FilesystemRead, FilesystemWrite, Mcp, WebAccess, ProviderState, SessionHistory | ReadSessions, PageHistory, LiveTail, StartSession, ResumeSession, ForkSession, StreamTurns, InterruptTurn, SteerTurn, Approvals, Tools, Diffs, Plans, Attachments, Images, Video, Models, Modes, RateLimits, SubscriptionAuth, DeviceCodeAuth, Headless |

Overlap ≈ 5 concepts; each has ~10–17 the other cannot express.

**`ProviderId` differs at the root:** automaton is a CLOSED enum
(`Chatgpt|Grok|Claude|Gemini|Cursor|Mock`); DevPULSE is an OPEN
`ProviderId(&'static str)` newtype — which is *why* Codex/Grok were added
without touching an enum. The merge must not lose that extensibility.

## 3. Target — the unified kernel

### 3.1 Structure (Decision A — see §4)

**Recommended:** automaton is THE kernel repo. The unified crates live there;
DevPULSE depends via a pinned git dependency and DELETES the `dpl-chat-*`
registry/runtime it replaces. New products depend the same way. One authority.

### 3.2 The canonical capability model — automaton's LEVELED profile + the union vocabulary

Grounding (2026-08-11) refined this: automaton does not carry a flat set, it
carries `CapabilityProfile` — a `BTreeMap<Capability, CapabilityLevel>` where
`CapabilityLevel ∈ {Native, Wrapped, Degraded, Unsupported, Unknown}`, plus a
`for_provider` DECLARED baseline and `supports(cap, allow_degraded)`. This is
strictly richer than DevPULSE's flat qualified `Vec<Capability>`, which is
just the Native+Wrapped projection.

**So the canonical model is automaton's `CapabilityProfile`**, holding the
union vocabulary below. And the two philosophies reconcile cleanly:
- automaton's `for_provider` = the **DECLARED** baseline (what a provider
  should do — drives UI *before* connect);
- DevPULSE's `qualified_capabilities()` = the **EARNED** actual (what the live
  runtime advertised at connect — drives `can_send`).
Kernel keeps both: declared profile, narrowed by earned qualification.

The union `Capability` (dedup the ~5 true synonyms: `Streaming`→`StreamTurns`,
`ToolUse`→`Tools`, `Resume`→`ResumeSession`, `TextInput` implied):

```
enum Capability {
  // ── SESSION (agent-loop shape — from DevPULSE) ──
  StartSession, ResumeSession, ForkSession, StreamTurns, InterruptTurn,
  SteerTurn, ReadSessions, PageHistory, LiveTail, SessionHistory,
  // ── INTERACTION (what a turn can surface — from DevPULSE) ──
  Approvals, Tools, Diffs, Plans, Attachments, StructuredOutput,
  // ── MEDIA (from both) ──
  ImageInput, Images, Video,
  // ── ACCESS / PERMISSION (from automaton) ──
  ShellRead, ShellWrite, FilesystemRead, FilesystemWrite, Mcp, WebAccess,
  // ── PROVIDER META (from both) ──
  Models, Modes, RateLimits, UsageReporting, ProviderState,
  SubscriptionAuth, DeviceCodeAuth, Headless,
}
```

Dedup the true synonyms once (`Streaming`→`StreamTurns`, `ToolUse`→`Tools`,
`Resume`→`ResumeSession`, `TextInput` is implied). This enum is authored in
the kernel; both taxonomies map onto it losslessly. `qualified_capabilities`
(DevPULSE) becomes the kernel's one derivation function.

### 3.3 `ProviderId` — open registry, canonical constants (Decision B)

Keep DevPULSE's open string-backed id (extensible), add automaton's serde
`snake_case` wire form, and publish canonical consts (`CHATGPT`, `GROK`,
`CLAUDE`, `GEMINI`, `CURSOR`, `MOCK`) so callers use the const, not a literal.
Note: today both repos cover the SAME five providers — the open form is a
hedge for the sixth, and it preserves automaton's `parse`/`as_str`/`all`
ergonomics via a `known()` slice.

### 3.3b The canonical IDENTITY / entitlement model — adopt automaton's decomposition

DevPULSE conflates everything in one `ActivationState`
(NotInstalled/SignedOut/Verifying/Ready/…). automaton splits it into
orthogonal axes that the kernel should own, because they answer different
questions:
- `AccessClass` — ApiKey | SubscriptionOauth | ConsumerApp (how it pays);
- `ActivationKind` — OfficialCli | DeveloperApi | BusinessApi | ConsumerApp
  (**the "subscription ≠ API" distinction — the automaton charter's core**);
- `AuthState` — Green | AuthNeeded | Expired;
- `Health` — Ok | Degraded | Missing | Error;
- `ProviderCatalogEntry` / `PROVIDER_CATALOG` — the static catalog
  (`primary_binary`, `typical_access`).

DevPULSE's `ActivationState` becomes a PROJECTION of `(AuthState, Health)` for
the egui surface. This is what makes "subscription and API usage base
capabilities" (the repo's stated purpose) a first-class kernel fact, not a
string.

### 3.4 Execution — DevPULSE's seam is the model (Decision C)

The kernel's execution layer is DevPULSE's `ProviderRuntime` (Codex + Grok
live), with `mato-acp` folded in as one transport. The Node engine becomes a
reference until Rust execution covers all five (Codex ✓, Grok ✓ via ACP,
Claude via ACP, Gemini + Cursor to build). One event contract (§3.5) means
the Node engine can even remain a daemon-surface transitionally.

### 3.5 One event contract

automaton's abstract event stream (`session.started`,
`message.assistant.delta`, `tool.started`, …) and DevPULSE's `RuntimeEvent`
(`Connected`, `SessionUpdate(AssistantText/Tool)`, `TurnFinished`, …) are the
same events named differently. Unify into the kernel's `RuntimeEvent`; the
JSON-lines form IS automaton's stream.

## 4. Load-bearing decisions (operator must ratify before Phase 1)

- **A — where the kernel lives.** (1) automaton is authority, DevPULSE depends
  via git *(recommended)*; (2) extract a third shared crate both depend on;
  (3) monorepo both. Recommendation (1): least churn, automaton is already
  designed for it.
- **B — `ProviderId` shape.** Open registry + canonical consts *(recommended,
  §3.3)* vs automaton's closed enum. Recommendation: open — losing
  extensibility would undo the seam work.
- **C — execution language.** Rust seam as the one executor *(recommended)*
  vs keep the Node engine as a daemon DevPULSE subprocesses. Recommendation:
  Rust — DevPULSE is pure-Rust and already has the two hardest spokes.

## 5. Phases (each ends green; none starts before the prior lands)

1. **Canonical vocabulary in the kernel** — author §3.2 `Capability`, §3.3
   `ProviderId`, unified `RuntimeEvent` in `mato-providers`; map automaton's
   existing 15 + DevPULSE's 22 onto it with round-trip tests. No consumer
   changes yet.
2. **DevPULSE consumes the kernel registry** — replace `dpl-chat-core`'s
   `Capability`/`ProviderId`/roster with the kernel crate; delete the
   duplicates; `qualified_capabilities` moves to the kernel. This is the
   proof-of-worth pass and the biggest single diff.
3. **Execution converges** — `ProviderRuntime` + `mato-acp` become the kernel
   seam; DevPULSE's Codex/Grok spokes re-home onto it.
4. **Signal journal + entitlement** — DevPULSE emits to `mato-kernel`'s signal
   journal; the control-plane contract is wired (mockable now, real on
   Shippin M1–M3).
5. **Retire the duplicates** — Node engine demoted to reference; `dpl-chat-*`
   surface crates (`window/windows/renderers`) stay in DevPULSE as the egui
   consumer; everything below them is the kernel.

## 6. Boundaries

🚫 Never: lose `ProviderId` extensibility (§3.3); force DevPULSE's 22 into
automaton's 15 (§2.3 — the lossy path this plan exists to avoid); bulk-rename
live `wardenclyffe.*`/`dpl-chat-*` identifiers before the phase that owns each;
break the double-green law on either repo.

⚠️ Ask first: any control-plane/billing wiring; making DevPULSE take a Node
runtime dependency; publishing the kernel to a public registry.

Out of scope: the WezTerm/Zellij terminal surface (automaton keeps it; DevPULSE
keeps egui); Shippin control-plane implementation.

## 7. Verification

Each phase: `cargo green` on BOTH repos (build DevPULSE against the pinned
kernel commit on server1 — 88c, sccache-warm). Round-trip tests that every
old automaton capability and every old DevPULSE capability maps to exactly one
canonical variant and back. DevPULSE's chat drills unchanged
(`chat-provider-tab`, `providers-popup`, `platform-sees-providers`,
`chat-codex-persisted-ready`). The live proof stays the Codex turn:
`ASSISTANT ANSWER` in the app, now through the merged kernel.

## 8. Evidence

Filled per phase. §2.3 is the honest core: the two are the same PATTERN and
NOT type-compatible, so this is a deliberate reconciliation to one vocabulary
— authored once, consumed everywhere — not a drop-in. Recorded before any
code so a future reader knows the merge was a design decision, not a rename.
